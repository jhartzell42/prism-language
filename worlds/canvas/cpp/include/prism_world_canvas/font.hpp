// The canvas World's one font: DejaVu Sans, the file in worlds/canvas/rust/fonts (its license
// beside it), built into the library, at one size. It measures text for `Gui.Metrics` and
// draws `Draw.Text`, so what the program lays out is what the window shows.
//
// The measures are worlds/canvas/rust/src/font.rs's, number for number: one size, one line, no
// kerning or shaping; a text's width is the sum of its characters' advances at `FONT_PX`
// (the font's own advances scaled by px over units per em, in `float` as fontdue does),
// rounded up, and its height is the ascent less the descent from the `hhea` table, rounded
// up, fourteen. FreeType reads the file and rasterizes the glyphs, unhinted.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#pragma once

#include <cstdint>
#include <memory>
#include <string>
#include <utility>
#include <vector>

namespace prism_world_canvas {

/// The size text is measured and drawn at, in the program's units (logical pixels).
/// DejaVu Sans at 12 px has an ascent-to-descent height just under 14.
inline constexpr float FONT_PX = 12.0f;

/// One character's coverage at a pixel size.
struct Coverage {
    int width = 0;
    int height = 0;
    /// The bitmap's left edge from the pen, in pixels.
    int left = 0;
    /// The bitmap's top edge below the baseline (negative: above it), in pixels.
    int top = 0;
    /// One byte of coverage per pixel, rows top to bottom.
    std::vector<std::uint8_t> bytes;
};

/// The bundled font at `FONT_PX`.
class Font {
public:
    /// The bundled DejaVu Sans. It is part of the binary, so this cannot fail at run time;
    /// if FreeType refuses it anyway, that is a bug, and it panics.
    static std::shared_ptr<const Font> bundled();
    ~Font();
    Font(const Font&) = delete;
    Font& operator=(const Font&) = delete;

    /// The size of `text` on one line, in whole units: the sum of the advances, rounded
    /// up, and the line's height. This is the `Gui.Metrics` answer.
    std::pair<double, double> extent(const std::u32string& text) const;
    /// The distance from the top of a line to its baseline, at `FONT_PX` times `scale`.
    float ascent(float scale) const { return ascent_ * scale; }
    /// A character's advance at `px`, in pixels.
    float advance(char32_t c, float px) const;
    /// One character's coverage at `px`.
    Coverage rasterize(char32_t c, float px) const;

private:
    Font();
    struct Face;
    std::unique_ptr<Face> face_;
    float ascent_ = 0;
    float height_ = 0;
};

}  // namespace prism_world_canvas
