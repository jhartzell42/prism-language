//! Painting a scene: a `Gui.Scene`'s `draw` list, in the program's coordinates (origin at
//! the top left, y down, logical pixels), onto a tiny-skia pixmap of physical pixels.
//!
//! The draw commands are the window library's (`examples/chess/window.prism`):
//!
//! - **`Draw.Rect(rect, paint)`**: a filled rectangle.
//! - **`Draw.Text(at, text)`**: one line in ink, the top of the line at `at`.
//! - **`Draw.Image(rect, src)`**: a PNG file, scaled to the rectangle. `src` is a path
//!   relative to the working directory. A file that does not load is reported once on
//!   standard error and draws nothing.
//! - **`Draw.Clip(rect, draws)`**: `draws`, cut to the rectangle (within any clip
//!   around it).
//! - **`Draw.Group(turn, draws)`**: `draws` turned clockwise on the screen by
//!   `turn.angle` degrees about the pivot `(turn.x, turn.y)`, then scaled about it by
//!   `turn.scale`. This is the transform `prism_rtl::world::scene`'s clickable search applies,
//!   so a turned card is drawn where a test clicks it.
//! - **`Draw.Clickable(rect, path)`**: ignored. Hit testing is the program's (D9).
//!
//! Anything else in a draw list is ignored.

use std::collections::HashMap;
use std::rc::Rc;

use prism_rtl::Pod;
use tiny_skia::{Color, FillRule, FilterQuality, Mask, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Transform};

use crate::font::{Font, FONT_PX};

/// The window's background where nothing is drawn: `Paper`.
pub const BACKGROUND: (u8, u8, u8) = (255, 255, 255);

/// Paints draw lists. It keeps every image and glyph it has loaded, so a scene that
/// changes one square does not reread sixty-four files.
pub struct Renderer {
    font: Rc<Font>,
    /// Each `src` as loaded, or `None` for a file that failed (reported once).
    images: HashMap<String, Option<Pixmap>>,
    /// Each character at each pixel size, as an ink-coloured pixmap and its offset from
    /// the pen: `None` for a character with no visible pixels (a space).
    glyphs: HashMap<(char, u32), Option<Glyph>>,
}

struct Glyph {
    pixmap: Pixmap,
    /// The bitmap's left edge from the pen, in pixels.
    left: f32,
    /// The bitmap's top edge below the baseline (negative: above it), in pixels.
    top: f32,
}

impl Renderer {
    pub fn new(font: Rc<Font>) -> Renderer {
        Renderer { font, images: HashMap::new(), glyphs: HashMap::new() }
    }

    /// Clear `pixmap` to the background and paint each draw list on it, in order.
    /// `scale` is the window's scale factor: physical pixels per logical pixel.
    pub fn render(&mut self, pixmap: &mut Pixmap, scale: f32, draw_lists: &[Pod]) {
        let (r, g, b) = BACKGROUND;
        pixmap.fill(Color::from_rgba8(r, g, b, 255));
        let t = Transform::from_scale(scale, scale);
        for list in draw_lists {
            self.draw_all(pixmap, scale, list, t, None);
        }
    }

    /// Every command of a draw list (an array of `Draw.*` elements).
    fn draw_all(&mut self, pixmap: &mut Pixmap, scale: f32, list: &Pod, t: Transform, clip: Option<&Mask>) {
        if let Pod::Array(items) = list.structure() {
            for d in items.iter() {
                self.draw(pixmap, scale, d, t, clip);
            }
        }
    }

    /// One command, under the transform `t` (logical to physical) and an optional clip.
    fn draw(&mut self, pixmap: &mut Pixmap, scale: f32, e: &Pod, t: Transform, clip: Option<&Mask>) {
        let Some(tag) = e.tag() else { return };
        match tag {
            "Draw.Rect" => {
                let Some(rect) = e.field("rect").and_then(rect_of) else { return };
                let mut paint = Paint::default();
                let (r, g, b) = e.field("paint").map(paint_rgb).unwrap_or(GREY_UNKNOWN);
                paint.set_color_rgba8(r, g, b, 255);
                pixmap.fill_rect(rect, &paint, t, clip);
            }
            "Draw.Text" => {
                let (Some(at), Some(text)) = (e.field("at"), e.field("text").and_then(Pod::as_text)) else { return };
                let text = text.to_string();
                self.text(pixmap, scale, number(at.field("x")), number(at.field("y")), &text, t, clip);
            }
            "Draw.Image" => {
                let (Some(rect), Some(src)) = (e.field("rect").and_then(rect_of), e.field("src").and_then(Pod::as_text)) else {
                    return;
                };
                let src = src.to_string();
                let Some(image) = self.image(&src) else { return };
                let fit = t
                    .pre_translate(rect.x(), rect.y())
                    .pre_scale(rect.width() / image.width() as f32, rect.height() / image.height() as f32);
                let paint = PixmapPaint { quality: FilterQuality::Bicubic, ..PixmapPaint::default() };
                pixmap.draw_pixmap(0, 0, image.as_ref(), &paint, fit, clip);
            }
            "Draw.Clip" => {
                let (Some(rect), Some(draws)) = (e.field("rect").and_then(rect_of), e.field("draws")) else { return };
                let path = PathBuilder::from_rect(rect);
                let mask = match clip {
                    Some(outer) => {
                        let mut m = outer.clone();
                        m.intersect_path(&path, FillRule::Winding, true, t);
                        m
                    }
                    None => {
                        let Some(mut m) = Mask::new(pixmap.width(), pixmap.height()) else { return };
                        m.fill_path(&path, FillRule::Winding, true, t);
                        m
                    }
                };
                self.draw_all(pixmap, scale, draws, t, Some(&mask));
            }
            "Draw.Group" => {
                let (Some(turn), Some(draws)) = (e.field("turn"), e.field("draws")) else { return };
                let (x, y) = (number(turn.field("x")) as f32, number(turn.field("y")) as f32);
                let angle = number(turn.field("angle")) as f32;
                let s = turn.field("scale").map(|v| number(Some(v))).unwrap_or(1.0) as f32;
                let turned = t.pre_translate(x, y).pre_rotate(angle).pre_scale(s, s).pre_translate(-x, -y);
                self.draw_all(pixmap, scale, draws, turned, clip);
            }
            _ => {} // Draw.Clickable, and anything this World does not draw
        }
    }

    /// One line of ink with the top of its line at `(x, y)`. Glyphs are rasterized at the
    /// window's pixel size and drawn through a transform that undoes the scale, so text is
    /// sharp on a Retina screen and still turns with its group.
    #[allow(clippy::too_many_arguments)]
    fn text(&mut self, pixmap: &mut Pixmap, scale: f32, x: f64, y: f64, text: &str, t: Transform, clip: Option<&Mask>) {
        let px = FONT_PX * scale;
        let baseline = self.font.ascent(scale);
        let origin = t.pre_translate(x as f32, y as f32).pre_scale(1.0 / scale, 1.0 / scale);
        let mut pen = 0.0f32;
        for c in text.chars() {
            let advance = self.font.advance(c, px);
            if let Some(g) = self.glyph(c, px) {
                let at = origin.pre_translate((pen + g.left).round(), (baseline + g.top).round());
                pixmap.draw_pixmap(0, 0, g.pixmap.as_ref(), &PixmapPaint::default(), at, clip);
            }
            pen += advance;
        }
    }

    /// A character at a pixel size, rasterized once.
    fn glyph(&mut self, c: char, px: f32) -> Option<&Glyph> {
        let font = self.font.clone();
        self.glyphs
            .entry((c, px.to_bits()))
            .or_insert_with(|| {
                let (m, coverage) = font.rasterize(c, px);
                let mut pixmap = Pixmap::new(m.width as u32, m.height as u32)?;
                // Ink is black, so a premultiplied pixel is (0, 0, 0, coverage).
                for (dst, a) in pixmap.data_mut().chunks_exact_mut(4).zip(coverage) {
                    dst.copy_from_slice(&[0, 0, 0, a]);
                }
                Some(Glyph { pixmap, left: m.xmin as f32, top: -(m.ymin as f32) - m.height as f32 })
            })
            .as_ref()
    }

    /// A PNG, loaded once.
    fn image(&mut self, src: &str) -> Option<&Pixmap> {
        self.images
            .entry(src.to_string())
            .or_insert_with(|| match Pixmap::load_png(src) {
                Ok(p) => Some(p),
                Err(e) => {
                    eprintln!("prism canvas: Draw.Image: {src}: {e}");
                    None
                }
            })
            .as_ref()
    }
}

/// The colour drawn for a paint this World does not know, as prism-reflex's Gloss World.
const GREY_UNKNOWN: (u8, u8, u8) = (204, 204, 204);

/// A `Paint` as a colour. The palette is prism-reflex's Gloss World's, with `Paper` and
/// `Ink` for its `White` and `Black`; `Rgb r, g, b` takes each channel from 0 to 1.
pub fn paint_rgb(paint: &Pod) -> (u8, u8, u8) {
    let Pod::Choice(ctor, payload) = paint.structure() else { return GREY_UNKNOWN };
    let name = ctor.rsplit('.').next().unwrap_or("");
    match name {
        "Paper" => (255, 255, 255),
        "Grey" => (179, 179, 179),
        "Blue" => (70, 110, 200),
        "Ink" => (0, 0, 0),
        "Yellow" => (240, 220, 90),
        "Red" => (200, 40, 40),
        "Green" => (60, 160, 70),
        "Felt" => (20, 100, 50),
        "Rgb" => {
            let channels: Vec<&Pod> = match payload.as_slice() {
                [Pod::Tuple(parts)] => parts.iter().collect(),
                parts => parts.iter().collect(),
            };
            match channels.as_slice() {
                [r, g, b] => (channel(r), channel(g), channel(b)),
                _ => GREY_UNKNOWN,
            }
        }
        _ => GREY_UNKNOWN,
    }
}

fn channel(v: &Pod) -> u8 {
    (number(Some(v)).clamp(0.0, 1.0) * 255.0).round() as u8
}

/// A `Rect(x, y, w, h)` record as a tiny-skia rectangle; `None` for an empty one.
pub fn rect_of(v: &Pod) -> Option<Rect> {
    let f = |name| number(v.field(name)) as f32;
    Rect::from_xywh(f("x"), f("y"), f("w"), f("h"))
}

/// A number field: a `Float`, or an integer.
pub fn number(v: Option<&Pod>) -> f64 {
    v.and_then(Pod::as_f64).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::gui;
    use prism_rtl::pod::F64;

    fn el(tag: &str, fields: Vec<(&str, Pod)>) -> Pod {
        Pod::element(tag, fields)
    }

    fn float(x: f64) -> Pod {
        Pod::Float(F64(x))
    }

    /// A red `Draw.Rect`.
    fn rect(x: f64, y: f64, w: f64, h: f64) -> Pod {
        el("Draw.Rect", vec![("rect", gui::rect(x, y, w, h)), ("paint", Pod::name("Red"))])
    }

    /// Paint one draw list at `scale` onto a 100 by 100 logical canvas.
    fn paint(scale: f32, draws: Vec<Pod>) -> Pixmap {
        let size = (100.0 * scale) as u32;
        let mut pixmap = Pixmap::new(size, size).unwrap();
        Renderer::new(Rc::new(Font::bundled())).render(&mut pixmap, scale, &[Pod::Array(draws)]);
        pixmap
    }

    fn rgb(p: &Pixmap, x: u32, y: u32) -> (u8, u8, u8) {
        let c = p.pixel(x, y).unwrap();
        (c.red(), c.green(), c.blue())
    }

    #[test]
    fn a_rect_is_painted_in_logical_pixels() {
        let p = paint(2.0, vec![rect(10.0, 10.0, 20.0, 20.0)]);
        assert_eq!(rgb(&p, 40, 40), (200, 40, 40)); // inside, at twice the scale
        assert_eq!(rgb(&p, 70, 70), BACKGROUND); // outside
    }

    #[test]
    fn a_clip_cuts_and_a_group_turns_about_its_pivot() {
        let clipped = el("Draw.Clip", vec![("rect", gui::rect(0.0, 0.0, 15.0, 100.0)), ("draws", Pod::Array(vec![rect(10.0, 10.0, 20.0, 20.0)]))]);
        let p = paint(1.0, vec![clipped]);
        assert_eq!(rgb(&p, 12, 20), (200, 40, 40));
        assert_eq!(rgb(&p, 20, 20), BACKGROUND);
        // A bar right of the pivot (50, 50), turned 90 degrees clockwise, hangs below it.
        let turn = Pod::record(
            "Turn",
            vec![("x", float(50.0)), ("y", float(50.0)), ("angle", float(90.0)), ("scale", float(1.0))],
        );
        let group = el("Draw.Group", vec![("turn", turn), ("draws", Pod::Array(vec![rect(60.0, 48.0, 30.0, 4.0)]))]);
        let p = paint(1.0, vec![group]);
        assert_eq!(rgb(&p, 50, 75), (200, 40, 40));
        assert_eq!(rgb(&p, 75, 50), BACKGROUND);
    }

    #[test]
    fn text_is_inked_below_its_point_and_clickables_draw_nothing() {
        let text = el("Draw.Text", vec![("at", gui::point(4.0, 4.0)), ("text", Pod::text("White's turn"))]);
        let click = el("Draw.Clickable", vec![("rect", gui::rect(0.0, 50.0, 100.0, 50.0)), ("path", Pod::Unit)]);
        let p = paint(1.0, vec![text, click]);
        let dark = |y0: u32, y1: u32| (0..100).any(|x| (y0..y1).any(|y| rgb(&p, x, y).0 < 128));
        assert!(dark(4, 18), "the line is inked");
        assert!(!dark(0, 4) && !dark(20, 100), "and nothing else is");
    }

    #[test]
    fn an_image_is_scaled_to_its_rect_and_a_missing_one_draws_nothing() {
        let src = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../examples/chess/pieces/kl.png");
        let king = el("Draw.Image", vec![("rect", gui::rect(0.0, 0.0, 100.0, 100.0)), ("src", Pod::text(src))]);
        let p = paint(1.0, vec![king]);
        assert!((0..100).any(|x| (0..100).any(|y| rgb(&p, x, y) != BACKGROUND)), "the king shows");
        let missing = el("Draw.Image", vec![("rect", gui::rect(0.0, 0.0, 100.0, 100.0)), ("src", Pod::text("no/such.png"))]);
        let p = paint(1.0, vec![missing]);
        assert!((0..100).all(|x| (0..100).all(|y| rgb(&p, x, y) == BACKGROUND)));
    }

    #[test]
    fn paints_follow_the_gloss_palette() {
        assert_eq!(paint_rgb(&Pod::name("Paper")), (255, 255, 255));
        assert_eq!(paint_rgb(&Pod::name("Ink")), (0, 0, 0));
        let rgb = Pod::Choice("Rgb".into(), vec![float(1.0), float(0.5), float(0.0)]);
        assert_eq!(paint_rgb(&rgb), (255, 128, 0));
        assert_eq!(paint_rgb(&Pod::name("Mauve")), GREY_UNKNOWN);
    }
}
