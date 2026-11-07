// A window whose scene changes, painted only where it changed: worlds/canvas/rust's
// tests/damage_smoke.rs in C++. The program here opens one `Gui.Window` and shows a table
// of rows in its `Gui.Scene`, and each `Press` moves the scene to the next of a list of
// steps (a select, a swap, an update, a remove, an append, a turned group and a clip over
// the rows, a clear, a new table). Each time the loop settles, the hook reads every pixel
// of the frame handed to the window system and checks it against a full repaint of the
// World's current scene by a fresh `Renderer`, then presses. After the last step both
// places leave and the run ends with 0. It needs a window system: on Linux the Makefile
// runs it under `xvfb-run -a` when no `DISPLAY` is set, and with none it fails, saying so.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include <unistd.h>

#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>

#include "prism_world_canvas/canvas.hpp"

using namespace prism_rtl;
using namespace prism_world_canvas;

namespace {

Pod el(std::string tag, Fields fields) { return Pod::element(std::move(tag), std::move(fields)); }
Pod rect(double x, double y, double w, double h, const char* paint) {
    return el("Draw.Rect", {{"rect", gui::rect(x, y, w, h)}, {"paint", Pod::name(paint)}});
}
Pod text(double x, double y, const std::string& t) { return el("Draw.Text", {{"at", gui::point(x, y)}, {"text", Pod::text(Text(t))}}); }

/// The rows' draw commands, as the window library draws a table.
Pod table(const std::vector<std::string>& labels, const std::vector<Pod>& extras) {
    std::vector<Pod> d = {rect(2, 2, 120, 26, "Grey"), text(8, 8, "Create rows")};
    for (std::size_t i = 0; i < labels.size(); ++i) {
        double y = 32 + 22.5 * double(i);
        d.push_back(text(4, y + 4, labels[i]));
        d.push_back(el("Draw.Clickable", {{"rect", gui::rect(0, y, 160, 22)}, {"path", Pod::unit()}}));
        d.push_back(rect(170.25, y + 1, 20.5, 20, "Grey"));
        d.push_back(text(176, y + 4, "x"));
    }
    for (const Pod& e : extras) d.push_back(e);
    return Pod::array(std::move(d));
}

/// The scenes the program steps through, one per press: damage_smoke.rs's.
std::vector<Pod> steps() {
    std::vector<std::string> labels;
    for (int i = 1; i <= 12; ++i) labels.push_back(std::to_string(i) + " pretty red table");
    std::vector<Pod> out = {table(labels, {})};
    labels[1] = "▶ " + labels[1];
    out.push_back(table(labels, {}));
    std::swap(labels[1], labels[8]);
    out.push_back(table(labels, {}));
    for (std::size_t i = 0; i < labels.size(); i += 3) labels[i] += " !!!";
    out.push_back(table(labels, {}));
    labels.erase(labels.begin() + 1);
    out.push_back(table(labels, {}));
    labels.push_back("13 big brown chair");
    out.push_back(table(labels, {}));
    Pod turn = Pod::record("Turn", {{"x", Pod::floating(100)}, {"y", Pod::floating(120)}, {"angle", Pod::floating(30)}, {"scale", Pod::floating(0.8)}});
    Pod group = el("Draw.Group", {{"turn", turn}, {"draws", Pod::array({rect(80, 110, 60, 9, "Green"), text(90, 120, "turned")})}});
    Pod clip = el("Draw.Clip", {{"rect", gui::rect(30.5, 60, 50, 30.25)}, {"draws", Pod::array({rect(0, 0, 200, 300, "Yellow")})}});
    out.push_back(table(labels, {group, clip}));
    out.push_back(table(labels, {clip, group}));
    out.push_back(table({}, {}));
    std::vector<std::string> fresh;
    for (int i = 1; i <= 12; ++i) fresh.push_back(std::to_string(i) + " small yellow chair");
    out.push_back(table(fresh, {}));
    return out;
}

struct Steps : Node {
    Path path;
    Pod window;
    std::size_t step = 0;
    std::vector<Pod> scenes = steps();
    explicit Steps(Path p) : path(std::move(p)) {}
    Pod scene(std::size_t i) const { return el("Gui.Scene", {{"window", window}, {"draw", scenes[i]}}); }
    Done answer(std::span<const Segment> at, const Pod& v, Cx& cx) override {
        if (at.size() != 1 || !at[0].is_pos(0) || step >= scenes.size())
            return unexpected(Fault::answer("no answering place at " + path_of(at).to_string()));
        auto c = v.structure().if_choice();
        if (!c) return unexpected(Fault::answer("expected WindowInput, got " + v.to_string()));
        if (c->ctor != "Press") return {};
        ++step;
        if (step < scenes.size()) {
            cx.changed(path.at(1), scene(step));
        } else {
            cx.removed(path.at(0));
            cx.removed(path.at(1));
        }
        return {};
    }
};

struct StepsProgram : Program {
    Runtime rt;
    prism_rtl::Rc<Steps> root;
    static Built<StepsProgram> build(std::span<const Pod>, const ReadyState& ready) {
        auto p = std::make_unique<StepsProgram>();
        Cx cx = p->rt.build(ready);
        p->root = prism_rtl::make_rc<Steps>(Path::root());
        Pod window = Pod::element("Gui.Window", {{"title", Pod::text(Text("Damage"))}, {"size", gui::extent(200, 320)}});
        auto id = cx.first_value(window);
        if (!id) return unexpected(Fault::answer("Gui.Window unanswered"));
        p->root->window = *id;
        cx.added(p->root->path.at(0), window);
        cx.added(p->root->path.at(1), p->root->scene(0));
        auto out = p->rt.close(std::move(cx));
        if (!out) return unexpected(out.error());
        return std::pair<std::unique_ptr<StepsProgram>, TickOut>(std::move(p), std::move(out).value());
    }
    Result<TickOut> tick(const TickIn& input, const ReadyState& ready) override { return rt.tick(root, input, ready); }
    void compact() override { rt.compact(); }
};

}  // namespace

int main() {
#ifndef __APPLE__
    if (!std::getenv("DISPLAY")) {
        std::fprintf(stderr, "damage_smoke: no DISPLAY; run it under xvfb-run -a (the Makefile does)\n");
        return 1;
    }
#endif
    alarm(60);  // a window that never settles ends the test
    std::size_t checked = 0;
    bool failed = false;
    Renderer renderer(Font::bundled());
    Hooks hooks;
    hooks.settled = [&](Session& s) {
        if (s.shared().windows.empty()) return;
        const Path& window = s.shared().windows.begin()->first;
        auto size = s.frame_size(window);
        if (!size) return;
        const Pod& id = s.shared().windows.at(window).id;
        cairo_surface_t* want = cairo_image_surface_create(CAIRO_FORMAT_RGB24, size->w, size->h);
        renderer.render(want, float(size->scale), s.shared().draw_lists(id));
        const unsigned char* data = cairo_image_surface_get_data(want);
        std::size_t stride = std::size_t(cairo_image_surface_get_stride(want));
        for (int y = 0; y < size->h && !failed; ++y)
            for (int x = 0; x < size->w; ++x) {
                std::uint32_t px;
                std::memcpy(&px, data + std::size_t(y) * stride + std::size_t(x) * 4, 4);
                auto got = s.pixel(window, x / size->scale, y / size->scale);
                if (got != (px & 0xffffff)) {
                    std::fprintf(stderr, "damage_smoke: step %zu: the frame differs from a full repaint at (%d, %d)\n", checked, x, y);
                    failed = true;
                    break;
                }
            }
        cairo_surface_destroy(want);
        if (failed) {
            s.exit(1);
            return;
        }
        ++checked;
        s.input(window, gui::press(1, 1));
    };
    int code = run_with<StepsProgram>(unwrapped, hooks);
    std::size_t want = steps().size();
    if (failed || code != 0 || checked != want) {
        std::fprintf(stderr, "damage_smoke: exit %d, %zu of %zu frames checked%s\n", code, checked, want, failed ? ", one differed" : "");
        return 1;
    }
    std::printf("damage_smoke: %zu scenes, each frame equal to a full repaint; exit 0\n", checked);
    return 0;
}
