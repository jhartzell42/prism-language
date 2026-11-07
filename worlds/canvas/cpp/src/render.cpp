// Painting a scene with cairo (render.hpp).
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include "prism_world_canvas/render.hpp"

#include <algorithm>
#include <cmath>
#include <cstring>
#include <cstdio>

namespace prism_world_canvas {

namespace {

constexpr double PI = 3.14159265358979323846;

void set_rgb(cairo_t* cr, Rgb c) {
    auto [r, g, b] = c;
    cairo_set_source_rgb(cr, r / 255.0, g / 255.0, b / 255.0);
}

std::uint8_t channel(const Pod& v) {
    double x = std::clamp(number(&v), 0.0, 1.0);
    return std::uint8_t(std::lround(x * 255.0));
}

bool finite(double x) { return std::isfinite(x); }

}  // namespace

namespace {
/// The `rect` field of a command, when it is a rectangle `rect_of` takes.
bool rect_field(const Pod& e, RectF& out) {
    const Pod* f = e.field("rect");
    if (!f) return false;
    auto r = rect_of(*f);
    if (!r) return false;
    out = *r;
    return true;
}
}  // namespace

double number(const Pod* v) {
    if (!v) return 0.0;
    return v->as_f64().value_or(0.0);
}

std::optional<RectF> rect_of(const Pod& v) {
    // In float, as tiny-skia's rectangles are.
    auto f = [&](const char* name) { return double(float(number(v.field(name)))); };
    RectF r{f("x"), f("y"), f("w"), f("h")};
    if (!finite(r.x) || !finite(r.y) || !finite(r.w) || !finite(r.h) || r.w < 0 || r.h < 0) return std::nullopt;
    return r;
}

Rgb paint_rgb(const Pod& paint) {
    auto c = paint.structure().if_choice();
    if (!c) return GREY_UNKNOWN;
    std::string_view name = c->ctor;
    if (auto dot = name.rfind('.'); dot != std::string_view::npos) name = name.substr(dot + 1);
    if (name == "Paper") return {255, 255, 255};
    if (name == "Grey") return {179, 179, 179};
    if (name == "Blue") return {70, 110, 200};
    if (name == "Ink") return {0, 0, 0};
    if (name == "Yellow") return {240, 220, 90};
    if (name == "Red") return {200, 40, 40};
    if (name == "Green") return {60, 160, 70};
    if (name == "Felt") return {20, 100, 50};
    if (name == "Rgb") {
        std::vector<const Pod*> ch;
        const std::vector<Pod>* parts = nullptr;
        if (c->args.size() == 1) parts = c->args[0].structure().if_tuple();
        if (parts)
            for (const auto& p : *parts) ch.push_back(&p);
        else
            for (const auto& p : c->args) ch.push_back(&p);
        if (ch.size() == 3) return {channel(*ch[0]), channel(*ch[1]), channel(*ch[2])};
    }
    return GREY_UNKNOWN;
}

Renderer::~Renderer() {
    for (auto& [_, s] : images_)
        if (s) cairo_surface_destroy(s);
    for (auto& [_, g] : glyphs_)
        if (g.mask) cairo_surface_destroy(g.mask);
}

void Renderer::render(cairo_surface_t* target, float scale, const std::vector<const Pod*>& draw_lists) {
    cairo_t* cr = cairo_create(target);
    set_rgb(cr, BACKGROUND);
    cairo_paint(cr);
    cairo_scale(cr, scale, scale);
    for (const Pod* list : draw_lists) draw_all(cr, scale, *list);
    cairo_destroy(cr);
    cairo_surface_flush(target);
}

void Renderer::draw_some(cairo_surface_t* target, float scale, const std::vector<const Pod*>& commands) {
    cairo_t* cr = cairo_create(target);
    cairo_scale(cr, scale, scale);
    for (const Pod* e : commands) draw(cr, scale, *e);
    cairo_destroy(cr);
    cairo_surface_flush(target);
}

namespace {

using Extent = Renderer::Extent;
using Kind = Renderer::Extent::Kind;

/// The box around the rectangle `x0, y0, x1, y1` taken through `m`.
Extent extent_of(const cairo_matrix_t& m, double x0, double y0, double x1, double y1) {
    double xs[4] = {x0, x1, x0, x1}, ys[4] = {y0, y0, y1, y1};
    Extent e{Kind::Box, 0, 0, 0, 0};
    for (int i = 0; i < 4; ++i) {
        cairo_matrix_transform_point(&m, &xs[i], &ys[i]);
        if (!finite(xs[i]) || !finite(ys[i])) return Extent{Kind::Whole};
    }
    e.x0 = std::min({xs[0], xs[1], xs[2], xs[3]});
    e.x1 = std::max({xs[0], xs[1], xs[2], xs[3]});
    e.y0 = std::min({ys[0], ys[1], ys[2], ys[3]});
    e.y1 = std::max({ys[0], ys[1], ys[2], ys[3]});
    return e;
}

Extent unite(const Extent& a, const Extent& b) {
    if (a.kind == Kind::Whole || b.kind == Kind::Whole) return Extent{Kind::Whole};
    if (a.kind == Kind::Nothing) return b;
    if (b.kind == Kind::Nothing) return a;
    return Extent{Kind::Box, std::min(a.x0, b.x0), std::min(a.y0, b.y0), std::max(a.x1, b.x1), std::max(a.y1, b.y1)};
}

/// Within a clip; anything not finite is unbounded inside a clip too, as cairo stops
/// drawing at a non-finite transform.
Extent within(const Extent& a, const Extent& clip) {
    if (a.kind == Kind::Whole || clip.kind == Kind::Whole) return Extent{Kind::Whole};
    if (a.kind == Kind::Nothing || clip.kind == Kind::Nothing) return Extent{};
    Extent e{Kind::Box, std::max(a.x0, clip.x0), std::max(a.y0, clip.y0), std::min(a.x1, clip.x1), std::min(a.y1, clip.y1)};
    if (e.x1 < e.x0 || e.y1 < e.y0) return Extent{};
    return e;
}

/// How far past its box a scaled image's filter (CAIRO_FILTER_BEST) may paint, in
/// physical pixels, beyond the pixel every box is widened by.
constexpr double IMAGE_BLEED = 2;

}  // namespace

Bounds Renderer::bounds(const Pod& e, float scale, int w, int h) {
    cairo_matrix_t m;
    cairo_matrix_init_scale(&m, scale, scale);
    Extent x = extent(e, scale, m);
    if (x.kind == Kind::Nothing) return Bounds{};
    if (x.kind == Kind::Whole) return Bounds{Bounds::Kind::Whole, {}};
    // Rounded out and cut to the frame in double, so no cast sees a value out of range.
    double x0 = std::clamp(std::floor(x.x0) - 1, 0.0, double(w)), y0 = std::clamp(std::floor(x.y0) - 1, 0.0, double(h));
    double x1 = std::clamp(std::ceil(x.x1) + 1, 0.0, double(w)), y1 = std::clamp(std::ceil(x.y1) + 1, 0.0, double(h));
    if (x1 <= x0 || y1 <= y0) return Bounds{};
    return Bounds{Bounds::Kind::Box, IRect{std::uint32_t(x0), std::uint32_t(y0), std::uint32_t(x1), std::uint32_t(y1)}};
}

Renderer::Extent Renderer::extent_all(const Pod& list, float scale, const cairo_matrix_t& m) {
    Extent out;
    if (auto items = list.structure().if_array())
        for (const Pod& d : *items) out = unite(out, extent(d, scale, m));
    return out;
}

// `draw`, case for case, with the context's matrix as `m`.
Renderer::Extent Renderer::extent(const Pod& e, float scale, const cairo_matrix_t& m) {
    const std::string* tag = e.tag();
    if (!tag) return Extent{};
    if (*tag == "Draw.Rect") {
        RectF r{};
        if (!rect_field(e, r)) return Extent{};
        return extent_of(m, r.x, r.y, r.x + r.w, r.y + r.h);
    }
    if (*tag == "Draw.Text") {
        const Pod* at = e.field("at");
        const Pod* t = e.field("text");
        const prism_rtl::Text* txt = t ? t->as_text() : nullptr;
        if (!at || !txt) return Extent{};
        double x = number(at->field("x")), y = number(at->field("y"));
        if (!finite(float(x)) || !finite(float(y))) return Extent{Kind::Whole};
        float px = FONT_PX * scale;
        float baseline = font_->ascent(scale);
        cairo_matrix_t tm = m;
        cairo_matrix_translate(&tm, float(x), float(y));
        cairo_matrix_scale(&tm, 1.0 / scale, 1.0 / scale);
        float pen = 0;
        Extent out;
        for (char32_t c : txt->chars()) {
            float advance = font_->advance(c, px);
            const Glyph& g = glyph(c, px);
            if (g.mask) {
                double gx = std::round(pen + g.left), gy = std::round(baseline + g.top);
                out = unite(out, extent_of(tm, gx, gy, gx + cairo_image_surface_get_width(g.mask), gy + cairo_image_surface_get_height(g.mask)));
            }
            pen += advance;
        }
        return out;
    }
    if (*tag == "Draw.Image") {
        const Pod* s = e.field("src");
        RectF r{};
        bool has = rect_field(e, r);
        const prism_rtl::Text* src = s ? s->as_text() : nullptr;
        if (!has || !src) return Extent{};
        cairo_surface_t* img = image(src->to_string());
        if (!img || r.w == 0 || r.h == 0) return Extent{};
        int iw = cairo_image_surface_get_width(img), ih = cairo_image_surface_get_height(img);
        cairo_matrix_t im = m;
        cairo_matrix_translate(&im, r.x, r.y);
        cairo_matrix_scale(&im, r.w / iw, r.h / ih);
        Extent out = extent_of(im, 0, 0, iw, ih);
        if (out.kind == Kind::Box) out = Extent{Kind::Box, out.x0 - IMAGE_BLEED, out.y0 - IMAGE_BLEED, out.x1 + IMAGE_BLEED, out.y1 + IMAGE_BLEED};
        return out;
    }
    if (*tag == "Draw.Clip") {
        const Pod* draws = e.field("draws");
        RectF r{};
        if (!rect_field(e, r) || !draws) return Extent{};
        return within(extent_all(*draws, scale, m), extent_of(m, r.x, r.y, r.x + r.w, r.y + r.h));
    }
    if (*tag == "Draw.Group") {
        const Pod* turn = e.field("turn");
        const Pod* draws = e.field("draws");
        if (!turn || !draws) return Extent{};
        double x = float(number(turn->field("x"))), y = float(number(turn->field("y")));
        double angle = float(number(turn->field("angle")));
        const Pod* sf = turn->field("scale");
        double s = float(sf ? number(sf) : 1.0);
        if (!finite(s) || !finite(x) || !finite(y) || !finite(angle)) return Extent{Kind::Whole};
        if (s == 0) return Extent{};  // drawn as nothing
        cairo_matrix_t gm = m;
        cairo_matrix_translate(&gm, x, y);
        cairo_matrix_rotate(&gm, angle * PI / 180.0);
        cairo_matrix_scale(&gm, s, s);
        cairo_matrix_translate(&gm, -x, -y);
        return extent_all(*draws, scale, gm);
    }
    return Extent{};
}

void Renderer::draw_all(cairo_t* cr, float scale, const Pod& list) {
    if (auto items = list.structure().if_array())
        for (const Pod& d : *items) draw(cr, scale, d);
}

void Renderer::draw(cairo_t* cr, float scale, const Pod& e) {
    const std::string* tag = e.tag();
    if (!tag) return;
    if (*tag == "Draw.Rect") {
        RectF rr{};
        bool has = rect_field(e, rr);
        const RectF* r = has ? &rr : nullptr;
        if (!r) return;
        const Pod* p = e.field("paint");
        set_rgb(cr, p ? paint_rgb(*p) : GREY_UNKNOWN);
        cairo_rectangle(cr, r->x, r->y, r->w, r->h);
        cairo_fill(cr);
    } else if (*tag == "Draw.Text") {
        const Pod* at = e.field("at");
        const Pod* t = e.field("text");
        const prism_rtl::Text* txt = t ? t->as_text() : nullptr;
        if (!at || !txt) return;
        text(cr, scale, number(at->field("x")), number(at->field("y")), *txt);
    } else if (*tag == "Draw.Image") {
        const Pod* s = e.field("src");
        RectF rr{};
        bool has = rect_field(e, rr);
        const RectF* r = has ? &rr : nullptr;
        const prism_rtl::Text* src = s ? s->as_text() : nullptr;
        if (!r || !src) return;
        cairo_surface_t* img = image(src->to_string());
        if (!img || r->w == 0 || r->h == 0) return;
        int iw = cairo_image_surface_get_width(img), ih = cairo_image_surface_get_height(img);
        cairo_save(cr);
        cairo_translate(cr, r->x, r->y);
        cairo_scale(cr, r->w / iw, r->h / ih);
        cairo_set_source_surface(cr, img, 0, 0);
        cairo_pattern_set_filter(cairo_get_source(cr), CAIRO_FILTER_BEST);
        cairo_paint(cr);
        cairo_restore(cr);
    } else if (*tag == "Draw.Clip") {
        const Pod* draws = e.field("draws");
        RectF rr{};
        bool has = rect_field(e, rr);
        const RectF* r = has ? &rr : nullptr;
        if (!r || !draws) return;
        cairo_save(cr);
        cairo_rectangle(cr, r->x, r->y, r->w, r->h);
        cairo_clip(cr);
        draw_all(cr, scale, *draws);
        cairo_restore(cr);
    } else if (*tag == "Draw.Group") {
        const Pod* turn = e.field("turn");
        const Pod* draws = e.field("draws");
        if (!turn || !draws) return;
        double x = float(number(turn->field("x"))), y = float(number(turn->field("y")));
        double angle = float(number(turn->field("angle")));
        const Pod* sf = turn->field("scale");
        double s = float(sf ? number(sf) : 1.0);
        // A zero or non-finite scale draws nothing (cairo's matrix would not invert).
        if (s == 0 || !finite(s) || !finite(x) || !finite(y) || !finite(angle)) return;
        cairo_save(cr);
        cairo_translate(cr, x, y);
        cairo_rotate(cr, angle * PI / 180.0);
        cairo_scale(cr, s, s);
        cairo_translate(cr, -x, -y);
        draw_all(cr, scale, *draws);
        cairo_restore(cr);
    }
    // Draw.Clickable, and anything this World does not draw.
}

void Renderer::text(cairo_t* cr, float scale, double x, double y, const prism_rtl::Text& text) {
    float px = FONT_PX * scale;
    float baseline = font_->ascent(scale);
    cairo_save(cr);
    cairo_translate(cr, float(x), float(y));
    cairo_scale(cr, 1.0 / scale, 1.0 / scale);
    cairo_set_source_rgb(cr, 0, 0, 0);
    float pen = 0;
    for (char32_t c : text.chars()) {
        float advance = font_->advance(c, px);
        const Glyph& g = glyph(c, px);
        if (g.mask) cairo_mask_surface(cr, g.mask, std::round(pen + g.left), std::round(baseline + g.top));
        pen += advance;
    }
    cairo_restore(cr);
}

const Renderer::Glyph& Renderer::glyph(char32_t c, float px) {
    std::uint32_t bits;
    std::memcpy(&bits, &px, sizeof bits);
    auto key = std::make_pair(c, bits);
    auto it = glyphs_.find(key);
    if (it != glyphs_.end()) return it->second;
    Coverage cov = font_->rasterize(c, px);
    Glyph g;
    if (cov.width > 0 && cov.height > 0) {
        g.mask = cairo_image_surface_create(CAIRO_FORMAT_A8, cov.width, cov.height);
        unsigned char* data = cairo_image_surface_get_data(g.mask);
        int stride = cairo_image_surface_get_stride(g.mask);
        cairo_surface_flush(g.mask);
        for (int row = 0; row < cov.height; ++row)
            std::memcpy(data + row * stride, cov.bytes.data() + std::size_t(row * cov.width), std::size_t(cov.width));
        cairo_surface_mark_dirty(g.mask);
        g.left = float(cov.left);
        g.top = float(cov.top);
    }
    return glyphs_.emplace(key, g).first->second;
}

cairo_surface_t* Renderer::image(const std::string& src) {
    auto it = images_.find(src);
    if (it != images_.end()) return it->second;
    cairo_surface_t* s = cairo_image_surface_create_from_png(src.c_str());
    if (cairo_surface_status(s) != CAIRO_STATUS_SUCCESS) {
        std::fprintf(stderr, "prism canvas: Draw.Image: %s: %s\n", src.c_str(), cairo_status_to_string(cairo_surface_status(s)));
        cairo_surface_destroy(s);
        s = nullptr;
    }
    images_.emplace(src, s);
    return s;
}

}  // namespace prism_world_canvas
