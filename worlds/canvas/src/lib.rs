//! The canvas World: the one foreign World chess runs under (decisions.md D9), a window
//! for each `Gui.Window` place, painted in software.
//!
//! *Written by Claude, 31 October 2026, on the `claude-self-hosting` branch, after
//! prism-reflex's Gloss World (`src/Prism/Gloss.hs`). Subject to change: implementers use
//! their discretion. See `README.md` beside this crate for what it answers, what it
//! ignores, the font, and the known limits.*
//!
//! A drawing World is four tags and no hit testing (Pitch ch. 6, "Speaking a subset"):
//! `Gui.Window`, `Gui.Scene`, `Gui.Metrics`, `Gui.Focus`. Buttons, layout, focus and
//! hit testing are the window library's, in Prism (`examples/chess/window.prism`).
//!
//! - [`world`]: the [`World`] the tick loop calls, with no display in it: the patch read
//!   into windows and scenes, the first-value query, the input queue.
//! - [`render`]: a draw list painted onto a pixmap (tiny-skia).
//! - [`font`]: the bundled DejaVu Sans, measuring and rasterizing (fontdue).
//! - `app`: winit's event loop, the windows and their surfaces (softbuffer).

pub mod font;
pub mod render;
pub mod world;

mod app;

use std::cell::RefCell;
use std::rc::Rc;

use prism_rt::world::{World, EXIT_FAILED};
use prism_rt::Program;

pub use world::{CanvasWorld, Shared, TAGS};

/// The families a program under this World may emit at its root: `Gui`.
pub const FAMILIES: &[&str] = &["Gui"];

/// Run `program` under the canvas World until it ends, and return its exit code. `wrap`
/// puts the World inside whatever answers the rest of the program's families (prism_zero
/// wraps it in its `Prelude.Unwrap` World); pass `|w| Box::new(w)` for none.
///
/// This must be called on the main thread (macOS requires it of the event loop).
pub fn run(program: Program, wrap: impl FnOnce(CanvasWorld) -> Box<dyn World>) -> u8 {
    let shared = Rc::new(RefCell::new(Shared::default()));
    let font = Rc::new(font::Font::bundled());
    let world = wrap(CanvasWorld::new(shared.clone(), font.clone()));
    let event_loop = match winit::event_loop::EventLoop::new() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("prism canvas: cannot start a window system event loop: {e}");
            return EXIT_FAILED;
        }
    };
    let mut app = app::App::new(program, world, shared, render::Renderer::new(font));
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("prism canvas: the event loop failed: {e}");
        return EXIT_FAILED;
    }
    app.exit.unwrap_or(0)
}
