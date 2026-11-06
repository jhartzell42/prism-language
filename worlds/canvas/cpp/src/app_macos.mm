// The window side on macOS: a Cocoa loop that owns time here, app.cpp's Xlib loop over
// AppKit. It builds the program when the loop starts, ticks on each window input and
// whenever the program is `pending`, makes the `NSWindow`s match the document's
// `Gui.Window` places after every tick, and repaints every window when a patch moved a
// window or a scene. The rounds are app.cpp's:
//
// - each round takes every AppKit event waiting, then ticks for at most a frame's time
//   (`FRAME_BUDGET`), then reconciles the windows, then paints;
// - a World presents only settled states: painting waits until the owed ticks have run
//   (Semantics ch. 9, "A World presents settled states");
// - while ticks are owed or input is queued the loop goes round again at once, and
//   otherwise it sleeps in `nextEventMatchingMask:untilDate:distantFuture`.
//
// Painting: cairo paints into a buffer of the window's size in physical pixels (the
// backing scale factor, 2 on a Retina screen), again only where the scene changed
// (damage.hpp), which becomes a `CGImage` set as the view's layer contents, the whole
// frame (a layer's contents are one image); `[CATransaction flush]` then hands the frame
// to the window server. That flush is what "on the screen" means here, as `XSync` is on
// X11. The window's colour space is sRGB, the image's, so the frame goes as it is.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include "prism_world_canvas/big_stack.hpp"
#include "prism_world_canvas/canvas.hpp"

#import <Cocoa/Cocoa.h>
#import <QuartzCore/QuartzCore.h>

#include <chrono>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <map>

namespace prism_world_canvas {
namespace {
class App;
struct OsWindow;
}  // namespace
}  // namespace prism_world_canvas

/// The view of one window: flipped, so its coordinates are the program's (origin top
/// left, y down), and layer-backed, its layer showing the last frame.
@interface PrismCanvasView : NSView
@property(nonatomic) prism_world_canvas::App* app;
@property(nonatomic) prism_world_canvas::OsWindow* place;
@end

/// The window's delegate: a close ends the run (code 0), a resize or a move to a screen
/// of another scale remakes the image.
@interface PrismCanvasDelegate : NSObject <NSWindowDelegate>
@property(nonatomic) prism_world_canvas::App* app;
@property(nonatomic) prism_world_canvas::OsWindow* place;
@end

namespace prism_world_canvas {

namespace {

using prism_rtl::TickIn;
using prism_rtl::TickOut;
using Clock = std::chrono::steady_clock;

/// How long one round of the loop may spend ticking before it lets the windows repaint
/// and take input.
constexpr auto FRAME_BUDGET = std::chrono::milliseconds(16);

/// One `NSWindow`, for one `Gui.Window` place.
struct OsWindow {
    NSWindow* window = nil;
    PrismCanvasView* view = nil;
    PrismCanvasDelegate* delegate = nil;
    Path path;
    /// The place's id: scenes naming it draw here.
    Pod id;
    std::string title;
    /// The last size the program was told, logical pixels.
    double w = 0, h = 0;
    /// Physical pixels per logical pixel, the window's backing scale factor.
    double scale = 1;
    /// The window's size in physical pixels.
    int pw = 0, ph = 0;
    bool mapped = false;
    /// Asked to paint: a resize, a scale change, or a patch that moved a scene.
    bool damaged = true;
    /// The pixels painted into, cairo's RGB24 (0x00RRGGBB, native order), `pw` by `ph`:
    /// the frame, painted again where the scene changed (damage.hpp).
    std::vector<std::uint32_t> pixels;
    Painter painter;
};

NSString* ns(const std::string& s) { return [NSString stringWithUTF8String:s.c_str()]; }

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
    std::optional<FrameSize> frame_size(const Path& window) const override;

    // From the view and the delegate.
    void pressed_at(OsWindow& w, NSPoint p) { shared_->push_input(w.path, gui::press(p.x, p.y)); }
    void key_down(OsWindow& w, NSEvent* e);
    void geometry_changed(OsWindow& w);

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
    int paint();
    bool all_shown() const;
    void handle(NSEvent* e);

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
    /// Each place's window; a `std::map` so a view's pointer to its place stays put.
    std::map<Path, OsWindow> windows_;
    std::optional<std::uint8_t> exit_;
    bool results_ = std::getenv("PRISM_RESULTS") != nullptr;
};

App::~App() {
    // A program's teardown recurses as deep as its values: on the big stack.
    big_.run([&] {
        program_.reset();
        world_.reset();
    });
    for (auto& [_, w] : windows_) close(w);
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
        prism_rtl::probe::enter(prism_rtl::probe::Phase::Loop);
        exit(world_->failed(result.error()));
        return;
    }
    const TickOut& out = *result;
    pending_ = out.pending;
    // `PRISM_RESULTS` set: each result the root gives, on standard error, for a driver.
    if (results_ && out.result && out.result->kind == prism_rtl::Outcome::Kind::Value)
        std::fprintf(stderr, "result: %s\n", out.result->pod.to_string().c_str());
    prism_rtl::probe::enter(prism_rtl::probe::Phase::Perform);
    world_->perform(out);
    if (auto code = world_->exit_code()) exit(*code);
    prism_rtl::probe::enter(prism_rtl::probe::Phase::Compact);
    if (program_) program_->compact();
    prism_rtl::probe::enter(prism_rtl::probe::Phase::Loop);
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
    if (!w.window) return;
    w.window.delegate = nil;
    [w.window orderOut:nil];
    [w.window close];
    w.window = nil;
    w.view = nil;
    w.delegate = nil;
    w.mapped = false;
}

void App::geometry_changed(OsWindow& w) {
    NSSize size = w.view.bounds.size;
    double scale = w.window.backingScaleFactor;
    int pw = int(std::lround(size.width * scale)), ph = int(std::lround(size.height * scale));
    if (pw != w.pw || ph != w.ph || scale != w.scale) {
        w.pw = pw;
        w.ph = ph;
        w.scale = scale;
        w.pixels.assign(std::size_t(std::max(pw, 0)) * std::size_t(std::max(ph, 0)), 0xffffff);
        w.painter.forget();
        w.view.layer.contentsScale = scale;
        w.damaged = true;
    }
    if (size.width != w.w || size.height != w.h) {
        w.w = size.width;
        w.h = size.height;
        shared_->push_input(w.path, gui::resized(w.w, w.h));
    }
}

bool App::open(const Path& path, const WindowPlace& place) {
    if (place.w <= 0 || place.h <= 0) {
        std::fprintf(stderr, "prism canvas: cannot open the window \"%s\": its size is %gx%g\n", place.title.c_str(), place.w, place.h);
        return false;
    }
    OsWindow& w = windows_.emplace(path, OsWindow{}).first->second;
    w.path = path;
    w.id = place.id;
    w.title = place.title;
    w.w = place.w;
    w.h = place.h;
    NSRect frame = NSMakeRect(0, 0, place.w, place.h);
    NSWindowStyleMask style = NSWindowStyleMaskTitled | NSWindowStyleMaskClosable | NSWindowStyleMaskResizable |
                              NSWindowStyleMaskMiniaturizable;
    w.window = [[NSWindow alloc] initWithContentRect:frame styleMask:style backing:NSBackingStoreBuffered defer:NO];
    if (!w.window) {
        std::fprintf(stderr, "prism canvas: cannot open the window \"%s\"\n", place.title.c_str());
        windows_.erase(path);
        return false;
    }
    w.window.releasedWhenClosed = NO;
    w.window.title = ns(place.title);
    // The frame's own colour space: with the screen's, Core Animation converts every pixel
    // of every frame on the CPU, in this process, before the frame reaches the window
    // server (about 125 M instructions a 900 by 800 frame on an M2 Pro's built-in screen).
    // The colours on the screen are the same: the window server matches sRGB to the screen.
    w.window.colorSpace = NSColorSpace.sRGBColorSpace;
    w.view = [[PrismCanvasView alloc] initWithFrame:frame];
    w.view.app = this;
    w.view.place = &w;
    w.view.wantsLayer = YES;
    w.view.layer.contentsGravity = kCAGravityTopLeft;
    w.view.layerContentsRedrawPolicy = NSViewLayerContentsRedrawNever;
    w.window.contentView = w.view;
    w.delegate = [PrismCanvasDelegate new];
    w.delegate.app = this;
    w.delegate.place = &w;
    w.window.delegate = w.delegate;
    [w.window cascadeTopLeftFromPoint:NSMakePoint(20 + 24 * double(windows_.size()), NSMaxY(NSScreen.mainScreen.visibleFrame) - 20)];
    [w.window makeKeyAndOrderFront:nil];
    [w.window makeFirstResponder:w.view];
    w.mapped = true;
    geometry_changed(w);
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
                w.window.title = ns(place.title);
    // The frame's own colour space: with the screen's, Core Animation converts every pixel
    // of every frame on the CPU, in this process, before the frame reaches the window
    // server (about 125 M instructions a 900 by 800 frame on an M2 Pro's built-in screen).
    // The colours on the screen are the same: the window server matches sRGB to the screen.
    w.window.colorSpace = NSColorSpace.sRGBColorSpace;
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
    CGColorSpaceRef space = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
    [CATransaction begin];
    [CATransaction setDisableActions:YES];
    for (auto& [path, w] : windows_) {
        if (!w.damaged || !w.mapped || w.pw <= 0 || w.ph <= 0) continue;
        w.damaged = false;
        int stride = w.pw * 4;
        auto* data = reinterpret_cast<unsigned char*>(w.pixels.data());
        cairo_surface_t* surface = cairo_image_surface_create_for_data(data, CAIRO_FORMAT_RGB24, w.pw, w.ph, stride);
        w.painter.paint(renderer_, surface, float(w.scale), shared_->draw_lists(w.id));
        cairo_surface_flush(surface);
        cairo_surface_destroy(surface);
        // RGB24 is 0x00RRGGBB in native (little-endian) order: Quartz's 32-bit little
        // with the first byte skipped. The bitmap context's image is a copy, so the
        // buffer is free to paint again at once.
        CGContextRef cg = CGBitmapContextCreate(data, std::size_t(w.pw), std::size_t(w.ph), 8, std::size_t(stride), space,
                                                CGBitmapInfo(kCGBitmapByteOrder32Little) | CGBitmapInfo(kCGImageAlphaNoneSkipFirst));
        if (!cg) {
            std::fprintf(stderr, "prism canvas: cannot make a Quartz image for the window \"%s\"\n", w.title.c_str());
            exit(prism_rtl::EXIT_FAILED);
            break;
        }
        CGImageRef image = CGBitmapContextCreateImage(cg);
        CGContextRelease(cg);
        w.view.layer.contents = (__bridge id)image;
        CGImageRelease(image);
        ++painted;
    }
    [CATransaction commit];
    // The frame is with the window server.
    if (painted) [CATransaction flush];
    CGColorSpaceRelease(space);
    return painted;
}

std::optional<std::uint32_t> App::pixel(const Path& window, double x, double y) const {
    auto it = windows_.find(window);
    if (it == windows_.end()) return std::nullopt;
    const OsWindow& w = it->second;
    long px = std::lround(x * w.scale), py = std::lround(y * w.scale);
    if (px < 0 || py < 0 || px >= w.pw || py >= w.ph) return std::nullopt;
    return w.pixels[std::size_t(py) * std::size_t(w.pw) + std::size_t(px)] & 0xffffff;
}

std::optional<Session::FrameSize> App::frame_size(const Path& window) const {
    auto it = windows_.find(window);
    if (it == windows_.end() || it->second.pw <= 0 || it->second.ph <= 0) return std::nullopt;
    return FrameSize{it->second.pw, it->second.ph, it->second.scale};
}

/// A key as the window library's `Key`: `Char c`, `Left`, `Right`, `Backspace`, `Tab`,
/// `Enter`; forward delete is taken as Backspace and Space as `Char ' '`, as app.cpp
/// does. Other keys are not reported. Characters are the event's, as Unicode.
void App::key_down(OsWindow& w, NSEvent* e) {
    auto named = [&](const char* name) { shared_->push_input(w.path, gui::pressed(Pod::name(name))); };
    switch (e.keyCode) {
        case 51:   // delete
        case 117:  // forward delete
            return named("Backspace");
        case 48: return named("Tab");
        case 36:   // return
        case 76:   // keypad enter
            return named("Enter");
        case 123: return named("Left");
        case 124: return named("Right");
        default: break;
    }
    NSString* chars = e.characters;
    NSUInteger n = chars.length;
    for (NSUInteger i = 0; i < n; ++i) {
        char32_t c = [chars characterAtIndex:i];
        if (c >= 0xd800 && c < 0xdc00 && i + 1 < n) {
            char32_t lo = [chars characterAtIndex:++i];
            c = 0x10000 + ((c - 0xd800) << 10) + (lo - 0xdc00);
        }
        if (c < 0x20 || c == 0x7f || (c >= 0xf700 && c <= 0xf8ff)) continue;  // controls; AppKit's function keys
        shared_->push_input(w.path, gui::pressed(Pod::choice("Char", {Pod::character(c)})));
    }
}

void App::handle(NSEvent* e) { [NSApp sendEvent:e]; }

bool App::all_shown() const {
    for (const auto& [_, w] : windows_)
        if (!w.mapped || w.damaged) return false;
    return true;
}

int App::run() {
    [NSApplication sharedApplication];
    [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
    [NSApp finishLaunching];
    [NSApp activateIgnoringOtherApps:YES];

    // The build is the first tick.
    tick_once(TickIn::none());
    reconcile();
    while (!exit_) {
        @autoreleasepool {
            while (!exit_) {
                NSEvent* e = [NSApp nextEventMatchingMask:NSEventMaskAny untilDate:[NSDate distantPast] inMode:NSDefaultRunLoopMode
                                                  dequeue:YES];
                if (!e) break;
                handle(e);
            }
            if (exit_) break;
            bool more = run_ticks();
            reconcile();
            if (exit_) break;
            if (!pending_) {
                prism_rtl::probe::enter(prism_rtl::probe::Phase::Paint);
                int painted = paint();
                prism_rtl::probe::enter(prism_rtl::probe::Phase::Loop);
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
            // Nothing owed: sleep until AppKit has an event.
            NSEvent* e = [NSApp nextEventMatchingMask:NSEventMaskAny untilDate:[NSDate distantFuture] inMode:NSDefaultRunLoopMode
                                              dequeue:YES];
            if (e) handle(e);
        }
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

@implementation PrismCanvasView
- (BOOL)isFlipped {
    return YES;
}
- (BOOL)acceptsFirstResponder {
    return YES;
}
- (BOOL)acceptsFirstMouse:(NSEvent*)event {
    return YES;
}
- (void)mouseDown:(NSEvent*)event {
    if (self.app) self.app->pressed_at(*self.place, [self convertPoint:event.locationInWindow fromView:nil]);
}
- (void)keyDown:(NSEvent*)event {
    if (self.app) self.app->key_down(*self.place, event);
}
@end

@implementation PrismCanvasDelegate
- (BOOL)windowShouldClose:(NSWindow*)sender {
    if (self.app) self.app->exit(0);
    return NO;
}
- (void)windowDidResize:(NSNotification*)note {
    if (self.app) self.app->geometry_changed(*self.place);
}
- (void)windowDidChangeBackingProperties:(NSNotification*)note {
    if (self.app) self.app->geometry_changed(*self.place);
}
@end
