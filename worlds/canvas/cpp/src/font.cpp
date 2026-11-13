// The bundled DejaVu Sans through FreeType (font.hpp).
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include "prism_world_canvas/font.hpp"

#include <ft2build.h>
#include FT_FREETYPE_H
#include FT_ADVANCES_H

#include <cmath>
#include <map>
#include <stdexcept>

// The font file, built into the object by the assembler; the Makefile names it in
// PRISM_CANVAS_FONT (worlds/canvas/rust/fonts/DejaVuSans.ttf, the Rust World's own file).
#ifndef PRISM_CANVAS_FONT
#error "PRISM_CANVAS_FONT must name the font file to build in"
#endif
#ifdef __APPLE__
// Mach-O: read-only data in __TEXT,__const, and C names carry a leading underscore.
asm(".section __TEXT,__const\n"
    ".p2align 4\n"
    "_prism_canvas_font_data:\n"
    ".incbin \"" PRISM_CANVAS_FONT "\"\n"
    "_prism_canvas_font_end:\n"
    ".text\n");
#else
asm(".section .rodata\n"
    ".balign 16\n"
    ".type prism_canvas_font_data, @object\n"
    "prism_canvas_font_data:\n"
    ".incbin \"" PRISM_CANVAS_FONT "\"\n"
    "prism_canvas_font_end:\n"
    ".previous\n");
#endif
extern "C" const unsigned char prism_canvas_font_data[];
extern "C" const unsigned char prism_canvas_font_end[];

namespace prism_world_canvas {

struct Font::Face {
    FT_Library library = nullptr;
    FT_Face face = nullptr;
    float units_per_em = 0;
    /// The size the face is set to, so a run of one size sets it once.
    mutable float px = 0;
    /// Each character's advance in font units.
    mutable std::map<char32_t, float> advances;
    ~Face() {
        if (face) FT_Done_Face(face);
        if (library) FT_Done_FreeType(library);
    }
};

Font::Font() : face_(std::make_unique<Face>()) {
    auto bug = [](const char* what) { throw std::logic_error(std::string("prism canvas: the bundled font: ") + what); };
    if (FT_Init_FreeType(&face_->library)) bug("FreeType does not start");
    auto size = FT_Long(prism_canvas_font_end - prism_canvas_font_data);
    if (FT_New_Memory_Face(face_->library, prism_canvas_font_data, size, 0, &face_->face)) bug("FreeType refuses it");
    FT_Face f = face_->face;
    if (!FT_IS_SCALABLE(f) || f->units_per_EM == 0) bug("it has no outlines");
    face_->units_per_em = float(f->units_per_EM);
    // fontdue's line metrics: the hhea ascender and descender, scaled in float.
    float scale = FONT_PX / face_->units_per_em;
    ascent_ = float(f->ascender) * scale;
    float descent = float(f->descender) * scale;
    height_ = std::ceil(ascent_ - descent);
}

Font::~Font() = default;

prism_rtl::Rc<const Font> Font::bundled() { return prism_rtl::rc_placed<const Font>([](void* at) { return new (at) Font(); }); }

float Font::advance(char32_t c, float px) const {
    auto it = face_->advances.find(c);
    if (it == face_->advances.end()) {
        FT_UInt g = FT_Get_Char_Index(face_->face, FT_ULong(c));
        FT_Fixed units = 0;
        if (FT_Get_Advance(face_->face, g, FT_LOAD_NO_SCALE, &units)) units = 0;
        it = face_->advances.emplace(c, float(units)).first;
    }
    return (px / face_->units_per_em) * it->second;
}

std::pair<double, double> Font::extent(const std::u32string& text) const {
    float width = 0;
    for (char32_t c : text) width += advance(c, FONT_PX);
    return {double(std::ceil(width)), double(height_)};
}

Coverage Font::rasterize(char32_t c, float px) const {
    FT_Face f = face_->face;
    if (face_->px != px) {
        FT_Set_Char_Size(f, 0, FT_F26Dot6(std::lround(px * 64.0f)), 72, 72);
        face_->px = px;
    }
    Coverage out;
    FT_UInt g = FT_Get_Char_Index(f, FT_ULong(c));
    if (FT_Load_Glyph(f, g, FT_LOAD_NO_HINTING | FT_LOAD_RENDER)) return out;
    const FT_Bitmap& bm = f->glyph->bitmap;
    if (bm.pixel_mode != FT_PIXEL_MODE_GRAY) return out;
    out.width = int(bm.width);
    out.height = int(bm.rows);
    out.left = f->glyph->bitmap_left;
    out.top = -f->glyph->bitmap_top;
    out.bytes.resize(std::size_t(out.width) * std::size_t(out.height));
    for (int y = 0; y < out.height; ++y)
        for (int x = 0; x < out.width; ++x) out.bytes[std::size_t(y * out.width + x)] = bm.buffer[y * bm.pitch + x];
    return out;
}

}  // namespace prism_world_canvas
