//! The canvas World's one font: DejaVu Sans, bundled in the crate (`fonts/`, with its
//! license beside it), at one size. It measures text for `Gui.Metrics` and draws
//! `Draw.Text`, so what the program lays out is what the window shows.
//!
//! One size, one line, no kerning or shaping: a text's width is the sum of its
//! characters' advances. The Test World's font is nine units a character and fourteen
//! high (`prism_rt::families::gui::test_font_extent`); this one is proportional, and at
//! [`FONT_PX`] its line is fourteen units high too, so a layout's rows come out the same
//! height under both Worlds and only widths differ.

/// The bundled font file.
const DEJAVU_SANS: &[u8] = include_bytes!("../fonts/DejaVuSans.ttf");

/// The size text is measured and drawn at, in the program's units (logical pixels).
/// DejaVu Sans at 12 px has an ascent-to-descent height just under 14.
pub const FONT_PX: f32 = 12.0;

/// The bundled font at [`FONT_PX`].
pub struct Font {
    face: fontdue::Font,
    ascent: f32,
    height: f32,
}

impl Font {
    /// The bundled DejaVu Sans. It is part of the binary, so this cannot fail at run time.
    pub fn bundled() -> Font {
        let face = fontdue::Font::from_bytes(DEJAVU_SANS, fontdue::FontSettings::default())
            .expect("the bundled DejaVu Sans parses");
        let line = face.horizontal_line_metrics(FONT_PX).expect("DejaVu Sans has horizontal metrics");
        Font { face, ascent: line.ascent, height: (line.ascent - line.descent).ceil() }
    }

    /// The size of `text` on one line, in whole units: the sum of the advances, rounded
    /// up, and the line's height. This is the `Gui.Metrics` answer.
    pub fn extent(&self, text: &str) -> (f64, f64) {
        let width: f32 = text.chars().map(|c| self.advance(c, FONT_PX)).sum();
        (width.ceil() as f64, self.height as f64)
    }

    /// The distance from the top of a line to its baseline, at [`FONT_PX`] times `scale`.
    pub fn ascent(&self, scale: f32) -> f32 {
        self.ascent * scale
    }

    /// A character's advance at `px`, in pixels.
    pub fn advance(&self, c: char, px: f32) -> f32 {
        self.face.metrics(c, px).advance_width
    }

    /// One character's coverage bitmap at `px`: its metrics and one byte of coverage per
    /// pixel, rows top to bottom.
    pub fn rasterize(&self, c: char, px: f32) -> (fontdue::Metrics, Vec<u8>) {
        self.face.rasterize(c, px)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_is_fourteen_high_like_the_test_worlds() {
        let f = Font::bundled();
        assert_eq!(f.extent("White's").1, 14.0);
        assert_eq!(f.extent("").0, 0.0);
    }

    #[test]
    fn widths_are_whole_and_grow_with_the_text() {
        let f = Font::bundled();
        let (one, _) = f.extent("turn");
        let (two, _) = f.extent("turn turn");
        assert_eq!(one, one.round());
        assert!(one > 0.0 && two > one * 2.0, "{one} {two}");
    }
}
