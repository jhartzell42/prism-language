// Painting a scene: a `Gui.Scene`'s `draw` list, in the program's coordinates (origin at
// the top left, y down, logical pixels), onto a cairo image surface of physical pixels.
// The draw commands and what each does are worlds/canvas/rust/src/render.rs's header, which
// holds word for word: `Draw.Rect`, `Draw.Text`, `Draw.Image` (a PNG relative to the
// working directory, reported once when it does not load), `Draw.Clip`, `Draw.Group` (a
// clockwise turn about a pivot, then a scale), `Draw.Clickable` (ignored), and anything
// else ignored.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#pragma once

#include <cairo.h>

#include <cstdint>
#include <map>
#include <memory>
#include <optional>
#include <string>
#include <tuple>
#include <utility>
#include <vector>

#include "prism_rtl/prism_rtl.hpp"
#include "prism_world_canvas/font.hpp"

namespace prism_world_canvas {

using prism_rtl::Pod;

/// An RGB colour, 0 to 255 a channel.
using Rgb = std::tuple<std::uint8_t, std::uint8_t, std::uint8_t>;

/// The window's background where nothing is drawn: `Paper`.
inline constexpr Rgb BACKGROUND = {255, 255, 255};
/// The colour drawn for a paint this World does not know, as prism-reflex's Gloss World.
inline constexpr Rgb GREY_UNKNOWN = {204, 204, 204};

/// A `Paint` as a colour. The palette is prism-reflex's Gloss World's, with `Paper` and
/// `Ink` for its `White` and `Black`; `Rgb r, g, b` takes each channel from 0 to 1.
Rgb paint_rgb(const Pod& paint);

/// A `Rect(x, y, w, h)` record: none for a negative or non-finite one, as tiny-skia's
/// `Rect::from_xywh`.
struct RectF {
    double x, y, w, h;
};
std::optional<RectF> rect_of(const Pod& v);

/// A number field: a `Float`, or an integer; 0 when absent.
double number(const Pod* v);

/// A box of physical pixels, `x0..x1` by `y0..y1`, inside the frame and not empty.
struct IRect {
    std::uint32_t x0 = 0, y0 = 0, x1 = 0, y1 = 0;
    std::uint64_t area() const { return std::uint64_t(x1 - x0) * std::uint64_t(y1 - y0); }
    friend bool operator==(const IRect&, const IRect&) = default;
};

/// What one top-level command can touch: nothing (a `Draw.Clickable`, an empty rectangle,
/// a command off the frame), a box, or the whole frame (a coordinate or transform that is
/// not finite).
struct Bounds {
    enum class Kind { Empty, Box, Whole } kind = Kind::Empty;
    IRect box{};
    friend bool operator==(const Bounds&, const Bounds&) = default;
};

/// Paints draw lists. It keeps every image and glyph it has loaded, so a scene that
/// changes one square does not reread sixty-four files.
class Renderer {
public:
    explicit Renderer(prism_rtl::Rc<const Font> font) : font_(std::move(font)) {}
    ~Renderer();
    Renderer(Renderer&&) = default;
    Renderer(const Renderer&) = delete;

    /// Clear `target` to the background and paint each draw list on it, in order.
    /// `scale` is the window's scale factor: physical pixels per logical pixel.
    void render(cairo_surface_t* target, float scale, const std::vector<const Pod*>& draw_lists);

    /// Paint these top-level commands of draw lists onto `target`, in order, as `render`
    /// paints each (with no clear): the damage painter's repaint of a box (damage.hpp).
    void draw_some(cairo_surface_t* target, float scale, const std::vector<const Pod*>& commands);

    /// The physical pixels of a `w` by `h` frame that the top-level command `e` can touch
    /// at `scale`: the box of everything `draw` would place, through the same transforms
    /// and glyph positions, rounded out to whole pixels and one pixel wider for
    /// anti-aliasing (an image three, for its filter), cut to the frame.
    Bounds bounds(const Pod& e, float scale, int w, int h);

private:
    struct Glyph {
        /// An A8 surface, or null for a character with no visible pixels (a space).
        cairo_surface_t* mask = nullptr;
        float left = 0;
        float top = 0;
    };
    void draw_all(cairo_t* cr, float scale, const Pod& list);
    void draw(cairo_t* cr, float scale, const Pod& e);
    void text(cairo_t* cr, float scale, double x, double y, const prism_rtl::Text& text);
    const Glyph& glyph(char32_t c, float px);
    cairo_surface_t* image(const std::string& src);

    prism_rtl::Rc<const Font> font_;
    /// Each `src` as loaded, or null for a file that failed (reported once).
    std::map<std::string, cairo_surface_t*> images_;
    /// Each character at each pixel size.
    std::map<std::pair<char32_t, std::uint32_t>, Glyph> glyphs_;
};

}  // namespace prism_world_canvas
