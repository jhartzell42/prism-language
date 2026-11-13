//! The smoke tests' program, hand-written in claude/backend.md's shape: one `Gui.Window` with
//! a `Gui.Scene` that fills a red rectangle, and on the window's first `Press` both places
//! leave. With no window left and nothing owed, the World ends the run with 0.
//! xvfb_smoke.rs clicks it through XTest, hooks_smoke.rs through `Hooks`.
//!
//! Written by Claude on the claude-self-hosting branch; subject to change.

use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use prism_rtl::*;
use prism_world_canvas_rtl::world::gui;

/// Set when the window's first input reached the program.
pub static HEARD: AtomicBool = AtomicBool::new(false);

/// The rectangle's centre and colour: where a driver looks and clicks.
pub const CENTRE: (i32, i32) = (60, 35);
pub const RED: u32 = 0xc82828;

// @main:
//     !input <- Gui.Window(title="Smoke", size=Extent(w=200.0, h=100.0))      # 0
//     Gui.Scene(window=WindowId "Smoke", draw=[Draw.Rect(rect=.., paint=Red)]) # 1
// and on `!input`'s first firing both places leave.
struct Smoke {
    path: Path,
    open: std::cell::Cell<bool>,
}

impl Node for Smoke {
    fn answer(self: Rc<Self>, at: &[Segment], v: &Pod, cx: &mut Cx) -> Done {
        if at != [Segment::Pos(0)] || !self.open.get() {
            return Err(Fault::answer(format!(
                "no answering place at {}",
                Path(at.to_vec())
            )));
        }
        let Pod::Choice(c, _) = v.structure() else {
            return Err(Fault::answer(format!("expected WindowInput, got {v}")));
        };
        eprintln!("smoke: the window sent {v}");
        if &**c != "Press" {
            return Ok(()); // a `Resized` before the click
        }
        HEARD.store(true, Ordering::SeqCst);
        self.open.set(false);
        cx.removed(self.path.at(0));
        cx.removed(self.path.at(1));
        Ok(())
    }
}

pub struct SmokeProgram {
    rt: Runtime,
    root: Rc<Smoke>,
}

impl Program for SmokeProgram {
    fn build(_: &[Pod], ready: &dyn ReadyState) -> Result<(Self, TickOut), Fault> {
        let mut rt = Runtime::new();
        let mut cx = rt.build(ready);
        let root = Rc::new(Smoke {
            path: Path::root(),
            open: true.into(),
        });
        let window = Pod::element(
            "Gui.Window",
            vec![
                ("title", Pod::text("Smoke")),
                ("size", gui::extent(200.0, 100.0)),
            ],
        );
        let id = cx
            .first_value(&window)
            .ok_or(Fault::answer("Gui.Window unanswered"))?;
        cx.added(root.path.at(0), window);
        let rect = Pod::element(
            "Draw.Rect",
            vec![
                ("rect", gui::rect(10.0, 10.0, 100.0, 50.0)),
                ("paint", Pod::name("Red")),
            ],
        );
        cx.added(
            root.path.at(1),
            Pod::element(
                "Gui.Scene",
                vec![("window", id), ("draw", Pod::Array(vec![rect]))],
            ),
        );
        let out = rt.close(cx)?;
        Ok((SmokeProgram { rt, root }, out))
    }
    fn tick(&mut self, input: &TickIn, ready: &dyn ReadyState) -> Result<TickOut, Fault> {
        let root: Rc<dyn Node> = self.root.clone();
        self.rt.tick(&root, input, ready)
    }
    fn compact(&mut self) {
        self.rt.compact();
    }
}
