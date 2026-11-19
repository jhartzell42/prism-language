//! A window whose scene changes, painted only where it changed: the program here opens one
//! `Gui.Window` and shows a table of rows in its `Gui.Scene`, and each `Press` moves the
//! scene to the next of a list of steps (a select, a swap, an update, a remove, an append,
//! a turned group and a clip over the rows, a clear, a new table). Each time the loop
//! settles, the hook reads every pixel of the presented frame and checks it against a full
//! repaint of the World's current scene by a fresh `Renderer`, then presses. After the last
//! step both places leave and the run ends with 0.
//! It needs a window system, and on Linux with no `DISPLAY` it says so and passes.
//!
//! Written by Claude on the claude-self-hosting branch; subject to change.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use prism_rtl::Float;
use prism_rtl::*;
use prism_world_canvas_rtl::font::Font;
use prism_world_canvas_rtl::render::Renderer;
use prism_world_canvas_rtl::world::gui;
use prism_world_canvas_rtl::{Hooks, Session, run_hooked};
use prism_world_cli::UnwrapWorld;
use tiny_skia::Pixmap;

fn el(tag: &str, fields: Vec<(&str, Pod)>) -> Pod {
    Pod::element(tag, fields)
}
fn rect(x: f64, y: f64, w: f64, h: f64, paint: &str) -> Pod {
    el("Draw.Rect", vec![("rect", gui::rect(x, y, w, h)), ("paint", Pod::name(paint))])
}
fn text(x: f64, y: f64, t: &str) -> Pod {
    el("Draw.Text", vec![("at", gui::point(x, y)), ("text", Pod::text(t))])
}

/// The rows' draw commands, as the window library draws a table: a label, its
/// clickable, a remove button and its `x`.
fn table(labels: &[String], extras: &[Pod]) -> Pod {
    let mut d = vec![rect(2.0, 2.0, 120.0, 26.0, "Grey"), text(8.0, 8.0, "Create rows")];
    for (i, l) in labels.iter().enumerate() {
        let y = 32.0 + 22.5 * i as f64;
        d.push(text(4.0, y + 4.0, l));
        d.push(el("Draw.Clickable", vec![("rect", gui::rect(0.0, y, 160.0, 22.0)), ("path", Pod::Unit)]));
        d.push(rect(170.25, y + 1.0, 20.5, 20.0, "Grey"));
        d.push(text(176.0, y + 4.0, "x"));
    }
    d.extend(extras.iter().cloned());
    Pod::Array(d)
}

/// The scenes the program steps through, one per press.
fn steps() -> Vec<Pod> {
    let mut labels: Vec<String> = (1..=12).map(|i| format!("{i} pretty red table")).collect();
    let mut out = vec![table(&labels, &[])];
    labels[1] = format!("▶ {}", labels[1]);
    out.push(table(&labels, &[]));
    labels.swap(1, 8);
    out.push(table(&labels, &[]));
    for l in labels.iter_mut().step_by(3) {
        l.push_str(" !!!");
    }
    out.push(table(&labels, &[]));
    labels.remove(1);
    out.push(table(&labels, &[]));
    labels.push("13 big brown chair".into());
    out.push(table(&labels, &[]));
    let turn = Pod::record(
        "Turn",
        vec![("x", Pod::Float(Float(100.0))), ("y", Pod::Float(Float(120.0))), ("angle", Pod::Float(Float(30.0))), ("scale", Pod::Float(Float(0.8)))],
    );
    let group = el("Draw.Group", vec![("turn", turn), ("draws", Pod::Array(vec![rect(80.0, 110.0, 60.0, 9.0, "Green"), text(90.0, 120.0, "turned")]))]);
    let clip = el("Draw.Clip", vec![("rect", gui::rect(30.5, 60.0, 50.0, 30.25)), ("draws", Pod::Array(vec![rect(0.0, 0.0, 200.0, 300.0, "Yellow")]))]);
    out.push(table(&labels, &[group.clone(), clip.clone()]));
    out.push(table(&labels, &[clip, group]));
    out.push(table(&[], &[]));
    out.push(table(&(1..=12).map(|i| format!("{i} small yellow chair")).collect::<Vec<_>>(), &[]));
    out
}

const SIZE: (f64, f64) = (200.0, 320.0);

struct Steps {
    path: Path,
    window: RefCell<Option<Pod>>,
    step: Cell<usize>,
    scenes: Vec<Pod>,
}

impl Steps {
    fn scene(&self, i: usize) -> Pod {
        el("Gui.Scene", vec![("window", self.window.borrow().clone().unwrap()), ("draw", self.scenes[i].clone())])
    }
}

impl Node for Steps {
    fn answer(self: Rc<Self>, at: &[Segment], v: &Pod, cx: &mut Cx) -> Done {
        if at != [Segment::Pos(0)] || self.step.get() >= self.scenes.len() {
            return Err(Fault::answer(format!("no answering place at {}", Path(at.to_vec()))));
        }
        let Pod::Choice(c, _) = v.structure() else { return Err(Fault::answer(format!("expected WindowInput, got {v}"))) };
        if &**c != "Press" {
            return Ok(());
        }
        let next = self.step.get() + 1;
        self.step.set(next);
        if next < self.scenes.len() {
            cx.changed(self.path.at(1), self.scene(next));
        } else {
            cx.removed(self.path.at(0));
            cx.removed(self.path.at(1));
        }
        Ok(())
    }
}

struct StepsProgram {
    rt: Runtime,
    root: Rc<Steps>,
}

impl Program for StepsProgram {
    fn build(_: &[Pod], ready: &dyn ReadyState) -> Result<(Self, TickOut), Fault> {
        let mut rt = Runtime::new();
        let mut cx = rt.build(ready);
        let root = Rc::new(Steps { path: Path::root(), window: RefCell::new(None), step: Cell::new(0), scenes: steps() });
        let window = Pod::element("Gui.Window", vec![("title", Pod::text("Damage")), ("size", gui::extent(SIZE.0, SIZE.1))]);
        let id = cx.first_value(&window).ok_or(Fault::answer("Gui.Window unanswered"))?;
        *root.window.borrow_mut() = Some(id);
        cx.added(root.path.at(0), window);
        cx.added(root.path.at(1), root.scene(0));
        let out = rt.close(cx)?;
        Ok((StepsProgram { rt, root }, out))
    }
    fn tick(&mut self, input: &TickIn, ready: &dyn ReadyState) -> Result<TickOut, Fault> {
        let root: Rc<dyn Node> = self.root.clone();
        self.rt.tick(&root, input, ready)
    }
    fn compact(&mut self) {
        self.rt.compact();
    }
}

fn main() {
    if cfg!(target_os = "linux") && std::env::var_os("DISPLAY").is_none() {
        println!("damage_smoke: skipped, no DISPLAY");
        return;
    }
    let checked = Rc::new(Cell::new(0usize));
    let failed = Rc::new(Cell::new(false));
    let (seen, bad) = (checked.clone(), failed.clone());
    let mut renderer = Renderer::new(Rc::new(Font::bundled()));
    let hooks = Hooks {
        settled: Some(Box::new(move |s: &mut dyn Session| {
            let Some(window) = s.shared().windows.keys().next().cloned() else { return };
            let Some((w, h, scale)) = s.frame_size(&window) else { return };
            let id = s.shared().windows[&window].id.clone();
            let draws = s.shared().draw_lists(&id);
            let mut want = Pixmap::new(w, h).unwrap();
            renderer.render(&mut want, scale as f32, &draws);
            let mismatch = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).find(|&(x, y)| {
                let p = want.pixel(x, y).unwrap();
                let want = (p.red() as u32) << 16 | (p.green() as u32) << 8 | p.blue() as u32;
                s.pixel(&window, x as f64 / scale, y as f64 / scale) != Some(want)
            });
            if let Some((x, y)) = mismatch {
                eprintln!("damage_smoke: step {}: the frame differs from a full repaint at ({x}, {y})", seen.get());
                bad.set(true);
                s.exit(1);
                return;
            }
            seen.set(seen.get() + 1);
            s.input(&window, gui::press(1.0, 1.0));
        })),
    };
    let code = run_hooked::<StepsProgram>(|w| Box::new(UnwrapWorld::new(w)), hooks);
    assert!(!failed.get(), "a frame differed from a full repaint");
    assert_eq!(code, 0, "the run's exit code");
    assert_eq!(checked.get(), steps().len(), "every step's frame was checked");
    println!("damage_smoke: {} scenes, each frame equal to a full repaint; exit 0", checked.get());
}
