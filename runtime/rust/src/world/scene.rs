//! `Draw.Clickable(rect, path)` in a `Gui.Scene` (decisions.md D9): the region `rect` of
//! a window is the element at `path` of the program's own layout. Hit testing stays in
//! Prism; the scene names it so the Test World can turn `click element path` into a
//! `Press` on the window. The canvas World ignores the tag.

use crate::doc::Document;
use crate::pod::{Path, Pod};

/// A `Draw.Clickable` found in a scene: the `Gui.Scene` place that drew it, the window
/// the scene is for, and its rectangle's centre in the window's coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct Clickable {
    pub scene: Path,
    pub window: Pod,
    pub x: f64,
    pub y: f64,
}

/// Every `Draw.Clickable` whose path prints as `target` in the document's scenes. Inside
/// a `Draw.Group` a rectangle is in the group's frame, so the centre is taken out through
/// each enclosing turn, innermost first; a `Draw.Clip` moves nothing.
pub fn find_clickables(doc: &Document, target: &str) -> Vec<Clickable> {
    let mut out = Vec::new();
    for (path, level) in doc.iter() {
        if level.tag() != Some("Gui.Scene") {
            continue;
        }
        let window = level.field("window").cloned().unwrap_or(Pod::Unit);
        if let Some(draw) = level.field("draw") {
            walk(draw, target, &mut Vec::new(), &mut |x, y| {
                out.push(Clickable { scene: path.clone(), window: window.clone(), x, y })
            });
        }
    }
    out
}

fn num(p: Option<&Pod>) -> f64 {
    p.and_then(Pod::as_f64).unwrap_or(0.0)
}

fn walk(p: &Pod, target: &str, turns: &mut Vec<Pod>, hit: &mut dyn FnMut(f64, f64)) {
    match (p.structure(), p.tag()) {
        (Pod::Array(items), _) => items.iter().for_each(|d| walk(d, target, turns, hit)),
        (_, Some("Draw.Clickable")) => {
            let (Some(Pod::Path(at)), Some(r)) = (p.field("path").map(Pod::structure), p.field("rect")) else { return };
            if crate::print::path(at) != target {
                return;
            }
            let (mut x, mut y) = (num(r.field("x")) + num(r.field("w")) / 2.0, num(r.field("y")) + num(r.field("h")) / 2.0);
            for t in turns.iter().rev() {
                (x, y) = turned(t, x, y);
            }
            hit(x, y);
        }
        (_, Some("Draw.Group")) => {
            turns.push(p.field("turn").cloned().unwrap_or(Pod::Unit));
            if let Some(d) = p.field("draws") {
                walk(d, target, turns, hit);
            }
            turns.pop();
        }
        (_, Some("Draw.Clip")) => {
            if let Some(d) = p.field("draws") {
                walk(d, target, turns, hit);
            }
        }
        _ => {}
    }
}

/// A point in a group's frame, in its parent's: turned clockwise on the screen by
/// `angle` degrees about the pivot `(x, y)`, then scaled about it; the inverse of the
/// window library's `Untransform`.
fn turned(t: &Pod, px: f64, py: f64) -> (f64, f64) {
    let (cx, cy, rad) = (num(t.field("x")), num(t.field("y")), num(t.field("angle")).to_radians());
    let scale = t.field("scale").map_or(1.0, |s| num(Some(s)));
    let (ux, uy) = (px - cx, py - cy);
    (cx + scale * (ux * rad.cos() - uy * rad.sin()), cy + scale * (ux * rad.sin() + uy * rad.cos()))
}
