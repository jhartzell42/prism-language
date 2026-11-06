// Painting a window's scene again only where it changed (damage.hpp; the why, and what it
// costs, are worlds/canvas/rust/src/damage.rs's header).
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include "prism_world_canvas/damage.hpp"

#include <algorithm>
#include <cstring>

namespace prism_world_canvas {

namespace {

/// More damaged boxes than this are painted as the one box around them all.
constexpr std::size_t MAX_BOXES = 32;

IRect unite(const IRect& a, const IRect& b) {
    return IRect{std::min(a.x0, b.x0), std::min(a.y0, b.y0), std::max(a.x1, b.x1), std::max(a.y1, b.y1)};
}

/// The boxes, with every two that overlap or touch made one, until none do; more than
/// `MAX_BOXES` become the one box around them all.
std::vector<IRect> merge(std::vector<IRect> boxes) {
    if (boxes.size() > MAX_BOXES) {
        IRect all = boxes[0];
        for (const IRect& b : boxes) all = unite(all, b);
        return {all};
    }
again:
    for (std::size_t i = 0; i < boxes.size(); ++i)
        for (std::size_t j = i + 1; j < boxes.size(); ++j) {
            const IRect &a = boxes[i], &b = boxes[j];
            if (a.x0 <= b.x1 && b.x0 <= a.x1 && a.y0 <= b.y1 && b.y0 <= a.y1) {
                boxes[i] = unite(a, b);
                boxes[j] = boxes.back();
                boxes.pop_back();
                goto again;
            }
        }
    return boxes;
}

bool meets(const Bounds& b, const IRect& r) {
    switch (b.kind) {
        case Bounds::Kind::Empty: return false;
        case Bounds::Kind::Whole: return true;
        case Bounds::Kind::Box: return b.box.x0 < r.x1 && r.x0 < b.box.x1 && b.box.y0 < r.y1 && r.y0 < b.box.y1;
    }
    return true;
}

}  // namespace

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
    int w = cairo_image_surface_get_width(target), h = cairo_image_surface_get_height(target);
    std::vector<const Pod*> commands;
    for (const Pod* list : draw_lists)
        if (auto items = list->structure().if_array())
            for (const Pod& d : *items) commands.push_back(&d);
    if (!last_ || last_->w != w || last_->h != h || last_->scale != scale) {
        renderer.render(target, scale, draw_lists);
        Painted p{w, h, scale, {}};
        p.drawn.reserve(commands.size());
        for (const Pod* c : commands) p.drawn.emplace_back(*c, renderer.bounds(*c, scale, w, h));
        last_ = std::move(p);
        return Damage{true, {}};
    }
    std::vector<IRect> boxes;
    bool whole = false;
    auto damage = [&](const Bounds& b) {
        if (b.kind == Bounds::Kind::Box) boxes.push_back(b.box);
        if (b.kind == Bounds::Kind::Whole) whole = true;
    };
    std::vector<std::pair<Pod, Bounds>> old = std::move(last_->drawn);
    std::vector<std::pair<Pod, Bounds>> drawn;
    drawn.reserve(commands.size());
    for (std::size_t i = 0; i < commands.size(); ++i) {
        if (i < old.size() && old[i].first == *commands[i]) {
            drawn.push_back(std::move(old[i]));
            continue;
        }
        if (i < old.size()) damage(old[i].second);
        Bounds b = renderer.bounds(*commands[i], scale, w, h);
        damage(b);
        drawn.emplace_back(*commands[i], b);
    }
    for (std::size_t i = commands.size(); i < old.size(); ++i) damage(old[i].second);
    last_->drawn = std::move(drawn);
    if (whole) {
        renderer.render(target, scale, draw_lists);
        return Damage{true, {}};
    }
    boxes = merge(std::move(boxes));
    if (boxes.empty()) return Damage{false, {}};
    if (scratch_ && (cairo_image_surface_get_width(scratch_) != w || cairo_image_surface_get_height(scratch_) != h)) {
        cairo_surface_destroy(scratch_);
        scratch_ = nullptr;
    }
    if (!scratch_) scratch_ = cairo_image_surface_create(CAIRO_FORMAT_RGB24, w, h);
    auto [br, bg, bb] = BACKGROUND;
    std::vector<const Pod*> meeting;
    for (const IRect& r : boxes) {
        // The box cleared to the background as `render` clears, through cairo, so even the
        // unused byte of each pixel is what a full repaint leaves.
        cairo_t* cr = cairo_create(scratch_);
        cairo_set_operator(cr, CAIRO_OPERATOR_SOURCE);
        cairo_set_source_rgb(cr, br / 255.0, bg / 255.0, bb / 255.0);
        cairo_rectangle(cr, r.x0, r.y0, r.x1 - r.x0, r.y1 - r.y0);
        cairo_fill(cr);
        cairo_destroy(cr);
        meeting.clear();
        for (const auto& [c, b] : last_->drawn)
            if (meets(b, r)) meeting.push_back(&c);
        renderer.draw_some(scratch_, scale, meeting);
        // The box alone into the frame: rows of `r`, both surfaces `w` by `h` RGB24, and
        // `r` inside both (`Renderer::bounds` cuts every box to the frame).
        cairo_surface_flush(target);
        const unsigned char* from = cairo_image_surface_get_data(scratch_);
        unsigned char* to = cairo_image_surface_get_data(target);
        std::size_t fs = std::size_t(cairo_image_surface_get_stride(scratch_)), ts = std::size_t(cairo_image_surface_get_stride(target));
        std::size_t bytes = std::size_t(r.x1 - r.x0) * 4;
        for (std::uint32_t y = r.y0; y < r.y1; ++y) std::memcpy(to + y * ts + std::size_t(r.x0) * 4, from + y * fs + std::size_t(r.x0) * 4, bytes);
        cairo_surface_mark_dirty_rectangle(target, int(r.x0), int(r.y0), int(r.x1 - r.x0), int(r.y1 - r.y0));
    }
    return Damage{false, std::move(boxes)};
}

}  // namespace prism_world_canvas
