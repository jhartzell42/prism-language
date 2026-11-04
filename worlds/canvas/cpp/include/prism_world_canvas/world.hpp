// The canvas World as the tick loop sees it: a `World` over `prism_rtl`'s protocol that
// reads each tick's patch into a table of windows and scenes, answers the first-value
// query from its font, and queues the windows' raw input for the next ticks. Nothing here
// touches a display, so it is tested without one; app.cpp is the half that opens windows
// and paints. The C++ form of worlds/canvas/rust/src/world.rs, whose header says what each of
// the four tags is answered with; it holds word for word.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#pragma once

#include <deque>
#include <map>
#include <memory>
#include <set>
#include <string>
#include <utility>
#include <vector>

#include "prism_rtl/prism_rtl.hpp"
#include "prism_world_cli/cli.hpp"

namespace prism_world_canvas {

class Font;

using prism_rtl::Path;
using prism_rtl::Pod;

/// The four tags this World answers.
inline const std::vector<std::string> TAGS = {"Gui.Window", "Gui.Scene", "Gui.Metrics", "Gui.Focus"};
/// The families a program under this World may emit at its root: `Gui`, and the prelude's,
/// which the `UnwrapWorld` around it answers.
inline const std::vector<std::string> FAMILIES = {"Gui", "Prelude"};

/// The `Gui` family's values as plain data (`prelude/gui.prism`).
namespace gui {
Pod window_id(const std::string& title);
Pod point(double x, double y);
Pod extent(double w, double h);
Pod rect(double x, double y, double w, double h);
/// `Press Point(x, y)`.
Pod press(double x, double y);
/// `Resized Extent(w, h)`.
Pod resized(double w, double h);
/// `Pressed key`.
Pod pressed(Pod key);
}  // namespace gui

/// A `Gui.Window` place as the World holds it.
struct WindowPlace {
    /// The id its first value answered, `WindowId title`, fixed for the place's life.
    Pod id;
    /// The current title.
    std::string title;
    /// The first size, logical pixels.
    double w = 640.0;
    double h = 480.0;
    bool operator==(const WindowPlace&) const = default;
};

/// A `Gui.Scene` place: the window it draws into and its current draw list.
struct ScenePlace {
    Pod window;
    Pod draw;
};

/// What the World and the window side share: the document's windows and scenes, by place
/// path, and the input waiting for a tick.
struct Shared {
    std::map<Path, WindowPlace> windows;
    std::map<Path, ScenePlace> scenes;
    /// Window input, oldest first, each addressed to a `Gui.Window` place.
    std::deque<std::pair<Path, Pod>> queue;
    /// Set when a patch moved a window or a scene; the window side clears it when it has
    /// caught up.
    bool dirty = false;

    /// Queue one window input. A resize directly after a resize of the same window
    /// replaces it, so dragging a window's corner costs one tick per frame, not per pixel.
    void push_input(Path window, Pod input);
    /// The draw lists of every scene for the window `id`, in document order.
    std::vector<const Pod*> draw_lists(const Pod& id) const;
};

/// The ready state: `Gui.Window` answered with its id, `Gui.Metrics` from the font.
class CanvasReady : public prism_rtl::ReadyState {
public:
    explicit CanvasReady(std::shared_ptr<const Font> font) : font_(std::move(font)) {}
    std::optional<Pod> first_value(const Pod& element) const override;

private:
    std::shared_ptr<const Font> font_;
};

/// The canvas World.
class CanvasWorld : public prism_rtl::World {
public:
    CanvasWorld(std::shared_ptr<Shared> shared, std::shared_ptr<const Font> font);
    std::shared_ptr<const prism_rtl::ReadyState> ready() const override;
    /// None of the four tags has a request field, and the prelude's requests are the
    /// wrapper's, so a request is never this World's. The patch: removals first, so a
    /// place rebuilt at a new path in the same tick is not drawn twice.
    void perform(const prism_rtl::TickOut& out) override;
    /// One window input per tick, oldest first: two presses never land in one tick, as in
    /// prism-reflex's Gloss World ("one turn per input").
    std::optional<prism_rtl::TickIn> next_inputs() override;
    /// The canvas World never asks a program to exit; closing a window ends the run.
    std::optional<std::uint8_t> exit_code() const override { return std::nullopt; }

private:
    void not_answered(const std::string& tag);
    void place(const Path& path, const Pod& element, bool added);

    std::shared_ptr<Shared> shared_;
    std::shared_ptr<const Font> font_;
    std::shared_ptr<const CanvasReady> ready_;
    /// Tags already reported as not answered here.
    std::set<std::string> warned_;
};

/// The prelude's `Prelude.Unwrap` policy around a World that answers no prelude tag
/// itself, as this one: worlds/cli/rust's `UnwrapWorld`, over worlds/cli/cpp's `Prelude`.
class UnwrapWorld : public prism_rtl::World {
public:
    explicit UnwrapWorld(std::unique_ptr<prism_rtl::World> inner);
    std::shared_ptr<const prism_rtl::ReadyState> ready() const override { return inner_->ready(); }
    void perform(const prism_rtl::TickOut& out) override;
    std::optional<prism_rtl::TickIn> next_inputs() override;
    std::optional<std::uint8_t> exit_code() const override;
    std::uint8_t failed(const prism_rtl::Fault& fault) override { return inner_->failed(fault); }

private:
    std::unique_ptr<prism_rtl::World> inner_;
    prism_rtl::Document doc_;
    prism_world_cli::Prelude prelude_;
};

}  // namespace prism_world_canvas
