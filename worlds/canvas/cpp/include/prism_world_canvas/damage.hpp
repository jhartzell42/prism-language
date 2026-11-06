// Painting a window's scene again only where it changed: worlds/canvas/rust/src/damage.rs,
// whose header holds word for word, in C++ over cairo.
//
// A `Painter` keeps, for one window's frame, the top-level draw commands it last painted
// (every scene's draw list for the window, in document order, concatenated) and each one's
// bounds in physical pixels. A new settled scene's commands are compared with them: those
// both lists begin with and end with stay, and the rest are compared position by position;
// each position whose command differs (or exists in only one of the two) damages the old
// command's bounds and the new one's. The damaged boxes are merged (more than 32
// become the one box around them all), and each is cleared to the background in a scratch
// surface of the frame's size, every command meeting it painted there whole and unclipped,
// as a full repaint paints it, and the box alone copied into the frame. The whole frame is
// painted on the first frame, a new size or scale, after `forget`, and when a damaged
// command cannot be bounded.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#pragma once

#include <cairo.h>

#include <cstdint>
#include <optional>
#include <utility>
#include <vector>

#include "prism_world_canvas/render.hpp"

namespace prism_world_canvas {

/// What one `Painter::paint` repainted: the whole frame, or these boxes (possibly none).
struct Damage {
    bool full = false;
    std::vector<IRect> rects;
    friend bool operator==(const Damage&, const Damage&) = default;
};

/// One window's last painted commands.
class Painter {
public:
    Painter() = default;
    ~Painter();
    Painter(Painter&& o) noexcept;
    Painter& operator=(Painter&& o) noexcept;
    Painter(const Painter&) = delete;
    Painter& operator=(const Painter&) = delete;

    /// Forget what was painted: the next paint is of the whole frame.
    void forget();

    /// Paint `draw_lists` onto `target`, a CAIRO_FORMAT_RGB24 image surface holding this
    /// painter's last frame, at `scale`: only where the scene changed.
    Damage paint(Renderer& renderer, cairo_surface_t* target, float scale, const std::vector<const Pod*>& draw_lists);

private:
    struct Painted {
        int w = 0, h = 0;
        float scale = 0;
        std::vector<std::pair<Pod, Bounds>> drawn;
    };
    std::optional<Painted> last_;
    /// Where a damaged box is painted afresh before it is copied into the frame.
    cairo_surface_t* scratch_ = nullptr;
};

}  // namespace prism_world_canvas
