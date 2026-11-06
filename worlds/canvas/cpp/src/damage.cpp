// Painting a window's scene again only where it changed (damage.hpp).
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include "prism_world_canvas/damage.hpp"

namespace prism_world_canvas {

Painter::~Painter() {
    if (scratch_) cairo_surface_destroy(scratch_);
}

Painter::Painter(Painter&& o) noexcept : last_(std::move(o.last_)), scratch_(std::exchange(o.scratch_, nullptr)) {}

Painter& Painter::operator=(Painter&& o) noexcept {
    if (this != &o) {
        if (scratch_) cairo_surface_destroy(scratch_);
        last_ = std::move(o.last_);
        scratch_ = std::exchange(o.scratch_, nullptr);
    }
    return *this;
}

void Painter::forget() { last_.reset(); }

Damage Painter::paint(Renderer& renderer, cairo_surface_t* target, float scale, const std::vector<const Pod*>& draw_lists) {
    renderer.render(target, scale, draw_lists);
    return Damage{true, {}};
}

}  // namespace prism_world_canvas
