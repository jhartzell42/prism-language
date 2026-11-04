// The window side: an Xlib loop that owns time here. It builds the program when the loop
// starts, ticks on each window input and whenever the program is `pending`, makes the X
// windows match the document's `Gui.Window` places after every tick, and repaints every
// window when a patch moved a window or a scene. worlds/canvas/rust/src/app.rs, with winit's
// event loop written out:
//
// - each round takes every X event waiting, then ticks for at most a frame's time
//   (`FRAME_BUDGET`), then reconciles the windows, then paints;
// - a World presents only settled states: while the program owes a tick (`pending`), the
//   scene may show one host deep a tick ahead of another, so painting waits until the owed
//   ticks have run (Semantics ch. 9, "A World presents settled states");
// - while ticks are owed or input is queued the loop goes round again at once, and
//   otherwise it sleeps on the X connection.
//
// The run ends when the program asks (a fault the World's policy ends on, or a poisoned
// root from the prelude's wrapper), when the user closes a window (code 0), or when the
// document holds no window and nothing can cause another tick (code 0, as `drive`).
//
// Painting goes through MIT-SHM: cairo paints straight into the shared image, and
// `XShmPutImage` then `XSync` puts it on the screen. A display without MIT-SHM (a remote
// one) is told so once on standard error and painted through `XPutImage`.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

// The library first: Xlib defines `Bool` and `Status` as macros.
#include "prism_world_canvas/big_stack.hpp"
#include "prism_world_canvas/canvas.hpp"

#include <X11/Xatom.h>
#include <X11/Xlib.h>
#include <X11/Xutil.h>
#include <X11/extensions/XShm.h>
#include <X11/keysym.h>
#include <poll.h>
#include <sys/ipc.h>
#include <sys/shm.h>

#include <chrono>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <map>


namespace prism_world_canvas {

namespace {

using prism_rtl::TickIn;
using prism_rtl::TickOut;
using Clock = std::chrono::steady_clock;

/// How long one round of the loop may spend ticking before it lets the windows repaint
/// and take input.
constexpr auto FRAME_BUDGET = std::chrono::milliseconds(16);

/// One X window, for one `Gui.Window` place.
struct OsWindow {
    ::Window xid = 0;
    /// The place's id: scenes naming it draw here.
    Pod id;
    std::string title;
    /// The last size the program was told, logical pixels.
    double w = 0, h = 0;
    /// The window's size in physical pixels, from the last `ConfigureNotify`.
    int pw = 0, ph = 0;
    bool mapped = false;
    /// Asked to paint: an expose, a resize, or a patch that moved a scene.
    bool damaged = true;
    /// The image painted into, the window's size; remade on a resize.
    XImage* image = nullptr;
    XShmSegmentInfo shm{};
    bool shm_attached = false;
    std::vector<std::uint32_t> heap;  // the pixels without MIT-SHM
};

class App final : public Session {
public:
    App(Builder build, std::unique_ptr<prism_rtl::World> world, prism_rtl::Rc<Shared> shared, Renderer renderer, Hooks hooks)
        : build_(std::move(build)), world_(std::move(world)), shared_(std::move(shared)), renderer_(std::move(renderer)),
          hooks_(std::move(hooks)) {}
    ~App() override;

    int run();

    const Shared& shared() const override { return *shared_; }
    void input(const Path& window, Pod value) override { shared_->push_input(window, std::move(value)); }
    void exit(std::uint8_t code) override {
        if (!exit_) exit_ = code;
    }
    std::optional<std::uint32_t> pixel(const Path& window, double x, double y) const override;

private:
    /// One tick, on the big stack: `tick_here` there.
    void tick_once(TickIn input) {
        big_.run([&] { tick_here(std::move(input)); });
    }
    void tick_here(TickIn input);
    bool run_ticks();
    void reconcile();
    bool open(const Path& path, const WindowPlace& place);
    void close(OsWindow& w);
    bool make_image(OsWindow& w);
    void drop_image(OsWindow& w);
    int paint();
    void event(XEvent& ev);
    OsWindow* window_of(::Window xid, Path* path);
    bool all_shown() const;

    /// Where ticks run, and the program is torn down: first, so it goes last.
    BigStack big_;
    Builder build_;
    std::unique_ptr<prism_rtl::World> world_;
    prism_rtl::Rc<Shared> shared_;
    Renderer renderer_;
    Hooks hooks_;
    std::unique_ptr<prism_rtl::Program> program_;
    /// The last tick said another is owed.
    bool pending_ = false;
    /// Ever had a window: a program that never opens one is told why nothing showed.
    bool opened_any_ = false;
    std::map<Path, OsWindow> windows_;
    std::optional<std::uint8_t> exit_;
    bool results_ = std::getenv("PRISM_RESULTS") != nullptr;

    Display* dpy_ = nullptr;
    int screen_ = 0;
    Visual* visual_ = nullptr;
    int depth_ = 0;
    GC gc_ = nullptr;
    bool shm_ = false;
    Atom wm_delete_ = 0;
    Atom wm_protocols_ = 0;
    Atom net_wm_name_ = 0;
    Atom utf8_string_ = 0;
    /// The scale factor: physical pixels per logical pixel. X11 here has one, 1.
    float scale_ = 1.0f;
};

/// MIT-SHM's attach fails asynchronously, as an X error: caught here, not fatal.
bool shm_failed = false;
int shm_error_handler(Display*, XErrorEvent*) {
    shm_failed = true;
    return 0;
}

App::~App() {
    // A program's teardown recurses as deep as its values: on the big stack.
    big_.run([&] {
        program_.reset();
        world_.reset();
    });
    for (auto& [_, w] : windows_) close(w);
    if (dpy_) {
        if (gc_) XFreeGC(dpy_, gc_);
        XCloseDisplay(dpy_);
    }
}

void App::tick_here(TickIn input) {
    auto ready = world_->ready();
    prism_rtl::Result<TickOut> result = [&]() -> prism_rtl::Result<TickOut> {
        if (program_) return program_->tick(input, *ready);
        auto built = build_(*ready);
        if (!built) return prism_rtl::unexpected(built.error());
        program_ = std::move(built.value().first);
        return std::move(built.value().second);
    }();
    if (!result) {
        exit(world_->failed(result.error()));
        return;
    }
    const TickOut& out = *result;
    pending_ = out.pending;
    // `PRISM_RESULTS` set: each result the root gives, on standard error, for a driver to
    // read (the xvfb run of chess checks its status this way).
    if (results_ && out.result && out.result->kind == prism_rtl::Outcome::Kind::Value)
        std::fprintf(stderr, "result: %s\n", out.result->pod.to_string().c_str());
    world_->perform(out);
    if (auto code = world_->exit_code()) exit(*code);
    if (program_) program_->compact();
}

bool App::run_ticks() {
    auto deadline = Clock::now() + FRAME_BUDGET;
    while (!exit_) {
        std::optional<TickIn> input = world_->next_inputs();
        if (!input) {
            if (!pending_) return false;
            input = TickIn::none();
        }
        tick_once(std::move(*input));
        if (Clock::now() >= deadline) return true;
    }
    return false;
}

void App::close(OsWindow& w) {
    drop_image(w);
    if (w.xid) XDestroyWindow(dpy_, w.xid);
    w.xid = 0;
}

void App::drop_image(OsWindow& w) {
    if (!w.image) return;
    if (w.shm_attached) {
        XShmDetach(dpy_, &w.shm);
        XSync(dpy_, False);
        shmdt(w.shm.shmaddr);
        w.shm_attached = false;
    }
    w.image->data = nullptr;  // the pixels are the segment's or `heap`'s, not Xlib's
    XDestroyImage(w.image);
    w.image = nullptr;
    w.heap.clear();
}

bool App::make_image(OsWindow& w) {
    drop_image(w);
    if (w.pw <= 0 || w.ph <= 0) return true;
    if (shm_) {
        w.image = XShmCreateImage(dpy_, visual_, unsigned(depth_), ZPixmap, nullptr, &w.shm, unsigned(w.pw), unsigned(w.ph));
        if (!w.image) return false;
        w.shm.shmid = shmget(IPC_PRIVATE, std::size_t(w.image->bytes_per_line) * std::size_t(w.ph), IPC_CREAT | 0600);
        if (w.shm.shmid < 0) return false;
        w.shm.shmaddr = static_cast<char*>(shmat(w.shm.shmid, nullptr, 0));
        if (w.shm.shmaddr == reinterpret_cast<char*>(-1)) {
            shmctl(w.shm.shmid, IPC_RMID, nullptr);
            return false;
        }
        w.image->data = w.shm.shmaddr;
        w.shm.readOnly = False;
        shm_failed = false;
        auto old = XSetErrorHandler(shm_error_handler);
        XShmAttach(dpy_, &w.shm);
        XSync(dpy_, False);
        XSetErrorHandler(old);
        // Both sides are attached: the segment goes when the last detaches.
        shmctl(w.shm.shmid, IPC_RMID, nullptr);
        if (shm_failed) {
            std::fprintf(stderr, "prism canvas: the X server refused a shared image; painting through XPutImage\n");
            shm_ = false;
            shmdt(w.shm.shmaddr);
            w.image->data = nullptr;
            XDestroyImage(w.image);
            w.image = nullptr;
            return make_image(w);
        }
        w.shm_attached = true;
        return true;
    }
    w.heap.assign(std::size_t(w.pw) * std::size_t(w.ph), 0);
    w.image = XCreateImage(dpy_, visual_, unsigned(depth_), ZPixmap, 0, reinterpret_cast<char*>(w.heap.data()), unsigned(w.pw),
                           unsigned(w.ph), 32, w.pw * 4);
    return w.image != nullptr;
}

bool App::open(const Path& path, const WindowPlace& place) {
    OsWindow w;
    w.id = place.id;
    w.title = place.title;
    w.w = place.w;
    w.h = place.h;
    w.pw = int(std::lround(place.w * scale_));
    w.ph = int(std::lround(place.h * scale_));
    if (w.pw <= 0 || w.ph <= 0) {
        std::fprintf(stderr, "prism canvas: cannot open the window \"%s\": its size is %gx%g\n", place.title.c_str(), place.w, place.h);
        return false;
    }
    XSetWindowAttributes attrs{};
    attrs.background_pixel = WhitePixel(dpy_, screen_);
    attrs.event_mask = ExposureMask | ButtonPressMask | KeyPressMask | StructureNotifyMask;
    w.xid = XCreateWindow(dpy_, RootWindow(dpy_, screen_), 0, 0, unsigned(w.pw), unsigned(w.ph), 0, depth_, InputOutput, visual_,
                          CWBackPixel | CWEventMask, &attrs);
    if (!w.xid) {
        std::fprintf(stderr, "prism canvas: cannot open the window \"%s\"\n", place.title.c_str());
        return false;
    }
    XStoreName(dpy_, w.xid, place.title.c_str());
    XChangeProperty(dpy_, w.xid, net_wm_name_, utf8_string_, 8, PropModeReplace,
                    reinterpret_cast<const unsigned char*>(place.title.data()), int(place.title.size()));
    XSetWMProtocols(dpy_, w.xid, &wm_delete_, 1);
    // The image is made in place: a shared image points at its window's `shm`.
    OsWindow& placed = windows_.emplace(path, std::move(w)).first->second;
    if (!make_image(placed)) {
        std::fprintf(stderr, "prism canvas: cannot make an image for the window \"%s\"\n", place.title.c_str());
        close(placed);
        windows_.erase(path);
        return false;
    }
    XMapWindow(dpy_, placed.xid);
    opened_any_ = true;
    return true;
}

void App::reconcile() {
    Shared& shared = *shared_;
    for (auto it = windows_.begin(); it != windows_.end();) {
        if (shared.windows.count(it->first)) {
            ++it;
            continue;
        }
        close(it->second);
        it = windows_.erase(it);
    }
    for (const auto& [path, place] : shared.windows) {
        auto it = windows_.find(path);
        if (it != windows_.end()) {
            OsWindow& w = it->second;
            if (w.title != place.title) {
                XStoreName(dpy_, w.xid, place.title.c_str());
                XChangeProperty(dpy_, w.xid, net_wm_name_, utf8_string_, 8, PropModeReplace,
                                reinterpret_cast<const unsigned char*>(place.title.data()), int(place.title.size()));
                w.title = place.title;
            }
            continue;
        }
        if (!open(path, place)) exit(prism_rtl::EXIT_FAILED);
    }
    if (shared.dirty && !pending_) {
        shared.dirty = false;
        for (auto& [_, w] : windows_) w.damaged = true;
    }
}

int App::paint() {
    int painted = 0;
    for (auto& [path, w] : windows_) {
        if (!w.damaged || !w.mapped || !w.image) continue;
        w.damaged = false;
        auto* data = reinterpret_cast<unsigned char*>(w.image->data);
        cairo_surface_t* surface = cairo_image_surface_create_for_data(data, CAIRO_FORMAT_RGB24, w.pw, w.ph, w.image->bytes_per_line);
        renderer_.render(surface, scale_, shared_->draw_lists(w.id));
        cairo_surface_destroy(surface);
        if (w.shm_attached)
            XShmPutImage(dpy_, w.xid, gc_, w.image, 0, 0, 0, 0, unsigned(w.pw), unsigned(w.ph), False);
        else
            XPutImage(dpy_, w.xid, gc_, w.image, 0, 0, 0, 0, unsigned(w.pw), unsigned(w.ph));
        ++painted;
    }
    // The frame is on the server, and the shared image free to paint again.
    if (painted) XSync(dpy_, False);
    return painted;
}

std::optional<std::uint32_t> App::pixel(const Path& window, double x, double y) const {
    auto it = windows_.find(window);
    if (it == windows_.end() || !it->second.image) return std::nullopt;
    const OsWindow& w = it->second;
    long px = std::lround(x * scale_), py = std::lround(y * scale_);
    if (px < 0 || py < 0 || px >= w.pw || py >= w.ph) return std::nullopt;
    auto* row = reinterpret_cast<const std::uint32_t*>(w.image->data + std::ptrdiff_t(py) * w.image->bytes_per_line);
    return row[px] & 0xffffff;
}

OsWindow* App::window_of(::Window xid, Path* path) {
    for (auto& [p, w] : windows_)
        if (w.xid == xid) {
            if (path) *path = p;
            return &w;
        }
    return nullptr;
}

/// A key as the window library's `Key`: `Char c`, `Left`, `Right`, `Backspace`, `Tab`,
/// `Enter`; Delete is taken as Backspace and Space as `Char ' '`, as prism-reflex's Gloss
/// World does. Other keys are not reported. Characters are Latin-1, `XLookupString`'s.
std::vector<Pod> keys_of(XKeyEvent& ev) {
    char buf[32];
    KeySym sym = 0;
    int n = XLookupString(&ev, buf, sizeof buf, &sym, nullptr);
    auto named = [](const char* name) { return std::vector<Pod>{Pod::name(name)}; };
    switch (sym) {
        case XK_BackSpace:
        case XK_Delete:
        case XK_KP_Delete: return named("Backspace");
        case XK_Tab:
        case XK_ISO_Left_Tab: return named("Tab");
        case XK_Return:
        case XK_KP_Enter: return named("Enter");
        case XK_Left:
        case XK_KP_Left: return named("Left");
        case XK_Right:
        case XK_KP_Right: return named("Right");
        case XK_space: return {Pod::choice("Char", {Pod::character(U' ')})};
        default: break;
    }
    std::vector<Pod> out;
    for (int i = 0; i < n; ++i) {
        auto c = char32_t(static_cast<unsigned char>(buf[i]));
        if (c < 0x20 || c == 0x7f) continue;
        out.push_back(Pod::choice("Char", {Pod::character(c)}));
    }
    return out;
}

void App::event(XEvent& ev) {
    Path path;
    OsWindow* w = window_of(ev.xany.window, &path);
    if (!w) return;
    Shared& shared = *shared_;
    switch (ev.type) {
        case ClientMessage:
            if (Atom(ev.xclient.message_type) == wm_protocols_ && Atom(ev.xclient.data.l[0]) == wm_delete_) exit(0);
            break;
        case MapNotify:
            w->mapped = true;
            w->damaged = true;
            break;
        case UnmapNotify: w->mapped = false; break;
        case Expose: w->damaged = true; break;
        case ButtonPress:
            if (ev.xbutton.button == Button1)
                shared.push_input(path, gui::press(ev.xbutton.x / double(scale_), ev.xbutton.y / double(scale_)));
            break;
        case KeyPress:
            for (Pod& k : keys_of(ev.xkey)) shared.push_input(path, gui::pressed(std::move(k)));
            break;
        case ConfigureNotify: {
            int pw = ev.xconfigure.width, ph = ev.xconfigure.height;
            if (pw != w->pw || ph != w->ph) {
                w->pw = pw;
                w->ph = ph;
                if (!make_image(*w)) {
                    std::fprintf(stderr, "prism canvas: cannot make an image for the window \"%s\"\n", w->title.c_str());
                    exit(prism_rtl::EXIT_FAILED);
                }
                w->damaged = true;
            }
            double lw = pw / double(scale_), lh = ph / double(scale_);
            if (lw != w->w || lh != w->h) {
                w->w = lw;
                w->h = lh;
                shared.push_input(path, gui::resized(lw, lh));
            }
            break;
        }
        default: break;
    }
}

bool App::all_shown() const {
    for (const auto& [_, w] : windows_)
        if (!w.mapped || w.damaged) return false;
    return true;
}

int App::run() {
    dpy_ = XOpenDisplay(nullptr);
    if (!dpy_) {
        std::fprintf(stderr, "prism canvas: cannot open the X display %s\n", std::getenv("DISPLAY") ? std::getenv("DISPLAY") : "(DISPLAY is unset)");
        return prism_rtl::EXIT_FAILED;
    }
    screen_ = DefaultScreen(dpy_);
    visual_ = DefaultVisual(dpy_, screen_);
    depth_ = DefaultDepth(dpy_, screen_);
    if (visual_->c_class != TrueColor || depth_ != 24 || visual_->red_mask != 0xff0000 || visual_->green_mask != 0xff00 ||
        visual_->blue_mask != 0xff) {
        std::fprintf(stderr, "prism canvas: the screen's visual is not 24-bit TrueColor 0xRRGGBB, the only one this World paints\n");
        return prism_rtl::EXIT_FAILED;
    }
    gc_ = XCreateGC(dpy_, RootWindow(dpy_, screen_), 0, nullptr);
    shm_ = XShmQueryExtension(dpy_);
    if (!shm_) std::fprintf(stderr, "prism canvas: the X server has no MIT-SHM; painting through XPutImage\n");
    wm_protocols_ = XInternAtom(dpy_, "WM_PROTOCOLS", False);
    wm_delete_ = XInternAtom(dpy_, "WM_DELETE_WINDOW", False);
    net_wm_name_ = XInternAtom(dpy_, "_NET_WM_NAME", False);
    utf8_string_ = XInternAtom(dpy_, "UTF8_STRING", False);

    // The build is the first tick.
    tick_once(TickIn::none());
    reconcile();
    while (!exit_) {
        while (!exit_ && XPending(dpy_)) {
            XEvent ev;
            XNextEvent(dpy_, &ev);
            event(ev);
        }
        if (exit_) break;
        bool more = run_ticks();
        reconcile();
        if (exit_) break;
        if (!pending_) {
            int painted = paint();
            if (painted && hooks_.presented) hooks_.presented(*this, painted);
        }
        if (exit_) break;
        if (!more && windows_.empty()) {
            if (!opened_any_) std::fprintf(stderr, "prism canvas: the program opened no Gui.Window; put its widget in a @Window\n");
            exit(0);
            break;
        }
        if (more || pending_ || !shared_->queue.empty()) continue;
        if (all_shown() && hooks_.settled) {
            hooks_.settled(*this);
            if (exit_ || !shared_->queue.empty()) continue;
        }
        // Nothing owed: sleep until the X server has something.
        if (XPending(dpy_)) continue;
        pollfd fd{ConnectionNumber(dpy_), POLLIN, 0};
        poll(&fd, 1, -1);
    }
    return *exit_;
}

}  // namespace

int run_program(Builder build, Wrap wrap, Hooks hooks) {
    auto shared = prism_rtl::make_rc<Shared>();
    auto font = Font::bundled();
    auto world = wrap(std::make_unique<CanvasWorld>(shared, font));
    App app(std::move(build), std::move(world), shared, Renderer(font), std::move(hooks));
    return app.run();
}

}  // namespace prism_world_canvas
