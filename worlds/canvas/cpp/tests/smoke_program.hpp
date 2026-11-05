// The smoke tests' program, hand-written in claude/backend.md's shape: one `Gui.Window`
// with a `Gui.Scene` that fills a red rectangle, and on the window's first `Press` both
// places leave. With no window left and nothing owed, the World ends the run with 0.
// xvfb_smoke.cpp clicks it through XTest, macos_smoke.cpp through `Hooks`.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#pragma once

#include <atomic>
#include <cstdio>

#include "prism_world_canvas/canvas.hpp"

namespace smoke {

using namespace prism_rtl;
using namespace prism_world_canvas;

/// Set when the window's first input reached the program.
inline std::atomic<bool> heard{false};

/// The rectangle's centre and colour: where a driver looks and clicks.
constexpr int CX = 60, CY = 35;
constexpr std::uint32_t RED = 0xc82828;

// @main:
//     !input <- Gui.Window(title="Smoke", size=Extent(w=200.0, h=100.0))      # 0
//     Gui.Scene(window=WindowId "Smoke", draw=[Draw.Rect(rect=.., paint=Red)]) # 1
// and on `!input`'s first firing both places leave.
struct Smoke : Node {
    Path path;
    bool open = true;
    explicit Smoke(Path p) : path(std::move(p)) {}
    Done answer(std::span<const Segment> at, const Pod& v, Cx& cx) override {
        if (at.size() != 1 || !at[0].is_pos(0) || !open)
            return unexpected(Fault::answer("no answering place at " + path_of(at).to_string()));
        auto c = v.structure().if_choice();
        if (!c) return unexpected(Fault::answer("expected WindowInput, got " + v.to_string()));
        std::fprintf(stderr, "smoke: the window sent %s\n", v.to_string().c_str());
        if (c->ctor != "Press") return {};  // a `Resized` before the click
        heard = true;
        open = false;
        cx.removed(path.at(0));
        cx.removed(path.at(1));
        return {};
    }
};

struct SmokeProgram : Program {
    Runtime rt;
    prism_rtl::Rc<Smoke> root;
    static Built<SmokeProgram> build(std::span<const Pod>, const ReadyState& ready) {
        auto p = std::make_unique<SmokeProgram>();
        Cx cx = p->rt.build(ready);
        p->root = prism_rtl::make_rc<Smoke>(Path::root());
        Pod window = Pod::element("Gui.Window", {{"title", Pod::text(Text("Smoke"))}, {"size", gui::extent(200, 100)}});
        auto id = cx.first_value(window);
        if (!id) return unexpected(Fault::answer("Gui.Window unanswered"));
        cx.added(p->root->path.at(0), window);
        Pod rect = Pod::element("Draw.Rect", {{"rect", gui::rect(10, 10, 100, 50)}, {"paint", Pod::name("Red")}});
        cx.added(p->root->path.at(1), Pod::element("Gui.Scene", {{"window", *id}, {"draw", Pod::array({rect})}}));
        auto out = p->rt.close(std::move(cx));
        if (!out) return unexpected(out.error());
        return std::pair<std::unique_ptr<SmokeProgram>, TickOut>(std::move(p), std::move(out).value());
    }
    Result<TickOut> tick(const TickIn& input, const ReadyState& ready) override { return rt.tick(root, input, ready); }
    void compact() override { rt.compact(); }
};

}  // namespace smoke
