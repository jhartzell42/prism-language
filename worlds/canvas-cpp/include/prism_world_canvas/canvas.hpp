// The canvas World over `prism_rtl`'s protocol, in C++: a window for each `Gui.Window`
// place, painted in software, speaking the compiled programs' `Program` and `World`. The
// C++ form of worlds/canvas (README.md beside this library says what differs).
//
//   prism_world_canvas::run<MyProgram>()           // inside the prelude's Prelude.Unwrap
//   prism_world_canvas::run_with<MyProgram>(wrap, hooks)
//
// A drawing World is four tags and no hit testing (Pitch ch. 6, "Speaking a subset"):
// `Gui.Window`, `Gui.Scene`, `Gui.Metrics`, `Gui.Focus`.
//
// - world.hpp: the `World` the tick loop calls, with no display in it.
// - render.hpp: a draw list painted onto a cairo image surface.
// - font.hpp: the bundled DejaVu Sans, through FreeType.
// - this file: the loop over Xlib, the windows and their MIT-SHM images (app.cpp).
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#pragma once

#include <cstdint>
#include <functional>
#include <memory>
#include <span>

#include "prism_world_canvas/font.hpp"
#include "prism_world_canvas/render.hpp"
#include "prism_world_canvas/world.hpp"

namespace prism_world_canvas {

/// What a hook may do to a running World: queue window input as the window system would,
/// and end the run. A benchmark or a test drives the program through it.
class Session {
public:
    virtual ~Session() = default;
    /// The document's windows and scenes, as the World last read them.
    virtual const Shared& shared() const = 0;
    /// Queue an input for the `Gui.Window` place at `window`, as a click or a key would.
    virtual void input(const Path& window, Pod value) = 0;
    /// End the run with `code` once the hook returns.
    virtual void exit(std::uint8_t code) = 0;
};

/// Calls into a hook from the loop. Each may be empty.
struct Hooks {
    /// The loop has settled: no input is queued, no tick is owed, and every window shows
    /// the document's scenes, on the X server (the frame after the last change is painted
    /// and synced). Called each time the loop would go to sleep.
    std::function<void(Session&)> settled;
    /// A frame went to the X server and was synced: `painted` windows this round.
    std::function<void(Session&, int painted)> presented;
};

/// The build: the program from no arguments, as a `Program`.
using Builder = std::function<prism_rtl::Result<std::pair<std::unique_ptr<prism_rtl::Program>, prism_rtl::TickOut>>(
    const prism_rtl::ReadyState&)>;
/// Puts the canvas World inside whatever answers the rest of the program's families.
using Wrap = std::function<std::unique_ptr<prism_rtl::World>(std::unique_ptr<CanvasWorld>)>;

/// Run a program under the canvas World until it ends: its exit code (app.cpp).
int run_program(Builder build, Wrap wrap, Hooks hooks);

/// The wrapper `run` uses: the prelude's `Prelude.Unwrap` policy.
inline std::unique_ptr<prism_rtl::World> unwrapped(std::unique_ptr<CanvasWorld> w) {
    return std::make_unique<UnwrapWorld>(std::move(w));
}

/// `run`, with `wrap` putting the World inside whatever answers the rest of the program's
/// families (`unwrapped` for the prelude's) and `hooks` called from the loop.
template <class P>
int run_with(Wrap wrap, Hooks hooks = {}) {
    Builder build = [](const prism_rtl::ReadyState& ready)
        -> prism_rtl::Result<std::pair<std::unique_ptr<prism_rtl::Program>, prism_rtl::TickOut>> {
        auto built = P::build(std::span<const Pod>(), ready);
        if (!built) return prism_rtl::unexpected(built.error());
        std::unique_ptr<prism_rtl::Program> p = std::move(built.value().first);
        return std::pair<std::unique_ptr<prism_rtl::Program>, prism_rtl::TickOut>(std::move(p),
                                                                                   std::move(built.value().second));
    };
    return run_program(std::move(build), std::move(wrap), std::move(hooks));
}

/// Run the program `P` under the canvas World, inside the prelude's `Prelude.Unwrap`
/// policy, until it ends: its exit code. It takes no arguments and answers no
/// `Process.Args`.
template <class P>
int run() {
    return run_with<P>(unwrapped);
}

}  // namespace prism_world_canvas
