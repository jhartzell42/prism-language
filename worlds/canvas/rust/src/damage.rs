//! Painting a window's scene again only where it changed.
//!
//! A [`Painter`] keeps, for one window's frame, the top-level draw commands it last painted
//! (every scene's draw list for the window, in document order, concatenated) and each one's
//! bounds: the physical pixels it can touch. When a new settled scene arrives it compares
//! the new commands with the last ones position by position; each position whose command
//! differs (or exists in only one of the two) damages the bounds of the old command and of
//! the new. It then merges the damaged boxes, and for each box clears it to the background
//! and paints, clipped to it, every new command whose bounds meet it, in order.
//!
//! Why that equals a full repaint: outside the damage, every command that touches a pixel,
//! old or new, sits at a position whose command did not change, so the commands touching
//! that pixel are the same ones in the same order, and the pixel is what it was. Inside a
//! box the pixel is painted afresh from the background by every command that can touch it.
//! Bounds are taken from the very placement the renderer uses (the transform, each glyph's
//! rounded position), rounded outward to whole pixels and one pixel wider for
//! anti-aliasing, and each box is whole pixels, so its clip has no partial edge.
//!
//! The whole frame is painted (and [`Damage::Full`] returned) on the first frame, when the
//! frame's size or scale changes, after [`Painter::forget`], and when a damaged command
//! cannot be bounded (a non-finite coordinate or transform).

use prism_rtl::Pod;
use tiny_skia::Pixmap;

use crate::render::Renderer;

/// A box of physical pixels, `x0..x1` by `y0..y1`, inside the frame and not empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IRect {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

impl IRect {
    pub fn area(&self) -> u64 {
        u64::from(self.x1 - self.x0) * u64::from(self.y1 - self.y0)
    }
}

/// What one command can touch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bounds {
    /// Nothing: a `Draw.Clickable`, an empty rectangle, a command off the frame.
    Empty,
    Box(IRect),
    /// Not bounded: a non-finite coordinate or transform.
    Whole,
}

/// What one [`Painter::paint`] repainted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Damage {
    /// The whole frame.
    Full,
    /// These boxes, possibly none.
    Rects(Vec<IRect>),
}

/// One window's last painted commands.
#[derive(Default)]
pub struct Painter {}

impl Painter {
    pub fn new() -> Painter {
        Painter::default()
    }

    /// Forget what was painted: the next paint is of the whole frame.
    pub fn forget(&mut self) {}

    /// Paint `draw_lists` onto `pixmap`, which holds this painter's last frame, at `scale`:
    /// only where the scene changed (the module's header).
    pub fn paint(&mut self, renderer: &mut Renderer, pixmap: &mut Pixmap, scale: f32, draw_lists: &[Pod]) -> Damage {
        renderer.render(pixmap, scale, draw_lists);
        Damage::Full
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::Font;
    use crate::world::gui;
    use prism_rtl::pod::F64;
    use std::rc::Rc;

    const W: f64 = 300.0;
    const H: f64 = 240.0;

    fn el(tag: &str, fields: Vec<(&str, Pod)>) -> Pod {
        Pod::element(tag, fields)
    }
    fn float(x: f64) -> Pod {
        Pod::Float(F64(x))
    }
    fn rect(x: f64, y: f64, w: f64, h: f64, paint: &str) -> Pod {
        el("Draw.Rect", vec![("rect", gui::rect(x, y, w, h)), ("paint", Pod::name(paint))])
    }
    fn text(x: f64, y: f64, t: &str) -> Pod {
        el("Draw.Text", vec![("at", gui::point(x, y)), ("text", Pod::text(t))])
    }

    /// A small xorshift, so the scenes are the same every run.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    /// A table as the window library draws one: a header of buttons, then rows of a
    /// label, a remove button and their clickables, some at fractional positions; then
    /// whatever `extras` hold, drawn over it.
    #[derive(Clone)]
    struct Table {
        labels: Vec<String>,
        top: f64,
        paint: &'static str,
        extras: Vec<Pod>,
    }

    impl Table {
        fn scene(&self) -> Vec<Pod> {
            let mut d = vec![rect(2.0, 2.0, 120.0, 26.0, "Grey"), text(8.0, 8.0, "Create 1,000 rows")];
            for (i, l) in self.labels.iter().enumerate() {
                let y = self.top + 22.5 * i as f64;
                if y > H + 8.0 {
                    break;
                }
                d.push(text(4.0, y + 4.0, l));
                d.push(el("Draw.Clickable", vec![("rect", gui::rect(0.0, y, 200.0, 22.0)), ("path", Pod::Unit)]));
                d.push(rect(220.25, y + 1.0, 20.5, 20.0, self.paint));
                d.push(text(226.0, y + 4.0, "x"));
            }
            d.extend(self.extras.iter().cloned());
            d
        }
    }

    fn word(rng: &mut Rng) -> String {
        const WORDS: &[&str] = &["pretty", "large", "big", "small", "tall", "red", "yellow", "table", "chair", "▶ brown"];
        WORDS[rng.below(WORDS.len() as u64) as usize].to_string()
    }

    fn extra(rng: &mut Rng) -> Pod {
        match rng.below(5) {
            0 => rect(rng.below(250) as f64 + 0.3, rng.below(200) as f64 + 0.7, 40.4, 13.2, "Blue"),
            1 => el(
                "Draw.Clip",
                vec![
                    ("rect", gui::rect(rng.below(200) as f64 + 0.5, rng.below(150) as f64, 50.0, 30.25)),
                    ("draws", Pod::Array(vec![rect(0.0, 0.0, 300.0, 240.0, "Yellow"), text(30.0, 30.0, "clipped text")])),
                ],
            ),
            2 => {
                let turn = Pod::record(
                    "Turn",
                    vec![
                        ("x", float(150.0)),
                        ("y", float(120.0)),
                        ("angle", float(rng.below(360) as f64 + 0.5)),
                        ("scale", float(0.75)),
                    ],
                );
                el("Draw.Group", vec![("turn", turn), ("draws", Pod::Array(vec![rect(140.0, 100.0, 60.0, 8.0, "Green"), text(150.0, 110.0, "turned")]))])
            }
            3 => {
                let src = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../examples/chess/pieces/kl.png");
                el("Draw.Image", vec![("rect", gui::rect(rng.below(250) as f64 + 0.25, rng.below(180) as f64, 37.5, 37.5)), ("src", Pod::text(src))])
            }
            _ => text(rng.below(250) as f64 + 0.5, rng.below(220) as f64 + 0.25, "over"),
        }
    }

    /// One change to the table, as a program's step might make it.
    fn change(t: &mut Table, rng: &mut Rng) {
        let n = t.labels.len();
        match rng.below(10) {
            0 if n > 0 => {
                let i = rng.below(n as u64) as usize;
                t.labels[i] = format!("▶ {}", t.labels[i]);
            }
            1 if n > 1 => {
                let (i, j) = (rng.below(n as u64) as usize, rng.below(n as u64) as usize);
                t.labels.swap(i, j);
            }
            2 if n > 0 => {
                t.labels.remove(rng.below(n as u64) as usize);
            }
            3 => t.labels.insert(rng.below(n as u64 + 1) as usize, word(rng)),
            4 => t.extras.push(extra(rng)),
            5 if !t.extras.is_empty() => {
                t.extras.remove(rng.below(t.extras.len() as u64) as usize);
            }
            6 if t.extras.len() > 1 => {
                let k = t.extras.len();
                t.extras.swap(rng.below(k as u64) as usize, rng.below(k as u64) as usize);
            }
            7 => t.top = if t.top == 40.0 { 40.5 } else { 40.0 },
            8 => t.paint = if t.paint == "Grey" { "Red" } else { "Grey" },
            9 if rng.below(4) == 0 => t.labels.clear(),
            _ => {}
        }
    }

    fn full(renderer: &mut Renderer, size: (u32, u32), scale: f32, draws: &[Pod]) -> Pixmap {
        let mut p = Pixmap::new(size.0, size.1).unwrap();
        renderer.render(&mut p, scale, draws);
        p
    }

    fn size_at(scale: f32) -> (u32, u32) {
        ((W as f32 * scale).round() as u32, (H as f32 * scale).round() as u32)
    }

    /// Damage painting equals a full repaint of the last scene, pixel for pixel, after
    /// every step of a long sequence of changes, at three scales.
    #[test]
    fn painting_the_damage_equals_a_full_repaint() {
        for scale in [1.0f32, 2.0, 1.5] {
            let mut rng = Rng(0x9e37_79b9_7f4a_7c15 ^ scale.to_bits() as u64);
            let mut renderer = Renderer::new(Rc::new(Font::bundled()));
            let mut reference = Renderer::new(Rc::new(Font::bundled()));
            let size = size_at(scale);
            let mut pixmap = Pixmap::new(size.0, size.1).unwrap();
            let mut painter = Painter::new();
            let mut t = Table { labels: (0..14).map(|_| word(&mut rng)).collect(), top: 40.0, paint: "Grey", extras: vec![] };
            let mut partial = 0;
            for step in 0..400 {
                // a second scene now and then, as a window with two `Gui.Scene`s has
                let lists = if step % 7 == 3 {
                    vec![Pod::Array(t.scene()), Pod::Array(vec![rect(10.0, 200.0, 30.0, 10.0, "Felt")])]
                } else {
                    vec![Pod::Array(t.scene())]
                };
                let damage = painter.paint(&mut renderer, &mut pixmap, scale, &lists);
                if matches!(damage, Damage::Rects(_)) {
                    partial += 1;
                }
                let want = full(&mut reference, size, scale, &lists);
                if pixmap.data() != want.data() {
                    let at = pixmap.data().iter().zip(want.data()).position(|(a, b)| a != b).unwrap() / 4;
                    panic!("scale {scale}, step {step}: the frame differs from a full repaint at ({}, {}) after {damage:?}", at as u32 % size.0, at as u32 / size.0);
                }
                change(&mut t, &mut rng);
            }
            assert!(partial > 300, "scale {scale}: only {partial} of 400 paints were partial");
        }
    }

    /// A small change damages a small part of the frame; the same scene damages none; the
    /// first frame and a new size repaint the whole.
    #[test]
    fn a_small_change_damages_a_small_box() {
        let scale = 2.0;
        let mut renderer = Renderer::new(Rc::new(Font::bundled()));
        let mut painter = Painter::new();
        let size = size_at(scale);
        let mut pixmap = Pixmap::new(size.0, size.1).unwrap();
        let mut rng = Rng(7);
        let mut t = Table { labels: (0..30).map(|_| word(&mut rng)).collect(), top: 40.0, paint: "Grey", extras: vec![] };
        assert_eq!(painter.paint(&mut renderer, &mut pixmap, scale, &[Pod::Array(t.scene())]), Damage::Full);
        assert_eq!(painter.paint(&mut renderer, &mut pixmap, scale, &[Pod::Array(t.scene())]), Damage::Rects(vec![]));
        t.labels[2] = format!("▶ {}", t.labels[2]);
        let Damage::Rects(boxes) = painter.paint(&mut renderer, &mut pixmap, scale, &[Pod::Array(t.scene())]) else {
            panic!("a select repainted the whole frame")
        };
        let area: u64 = boxes.iter().map(IRect::area).sum();
        assert!(!boxes.is_empty() && area * 50 < u64::from(size.0 * size.1), "a select damaged {boxes:?}");
        painter.forget();
        assert_eq!(painter.paint(&mut renderer, &mut pixmap, scale, &[Pod::Array(t.scene())]), Damage::Full);
        let mut bigger = Pixmap::new(size.0 + 10, size.1).unwrap();
        assert_eq!(painter.paint(&mut renderer, &mut bigger, scale, &[Pod::Array(t.scene())]), Damage::Full);
    }

    /// A command at a non-finite place cannot be bounded: the whole frame.
    #[test]
    fn an_unbounded_command_repaints_the_whole_frame() {
        let mut renderer = Renderer::new(Rc::new(Font::bundled()));
        let mut painter = Painter::new();
        let mut pixmap = Pixmap::new(100, 100).unwrap();
        painter.paint(&mut renderer, &mut pixmap, 1.0, &[Pod::Array(vec![rect(1.0, 1.0, 5.0, 5.0, "Red")])]);
        let turn = Pod::record("Turn", vec![("x", float(f64::NAN)), ("y", float(0.0)), ("angle", float(0.0)), ("scale", float(1.0))]);
        let group = el("Draw.Group", vec![("turn", turn), ("draws", Pod::Array(vec![rect(1.0, 1.0, 5.0, 5.0, "Red")]))]);
        let lists = [Pod::Array(vec![rect(1.0, 1.0, 5.0, 5.0, "Red"), group])];
        assert_eq!(painter.paint(&mut renderer, &mut pixmap, 1.0, &lists), Damage::Full);
    }
}
