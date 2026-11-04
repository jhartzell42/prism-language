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
