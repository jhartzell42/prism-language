//! The canvas World over `prism_rtl`'s protocol: a window for each `Gui.Window` place,
//! painted in software. It is `worlds/canvas` (which serves the interpreter, prism_zero)
//! speaking the compiled programs' [`Program`] and [`World`] instead of prism_rt's.
//!
//! *Written by Claude, 1 November 2026, on the `claude-self-hosting` branch. Subject to
//! change: implementers use their discretion. `README.md` beside this crate says what
//! differs from `worlds/canvas`; that crate's README says what the World answers, what it
//! ignores, the font, and the known limits, all of which hold here.*
//!
//! A drawing World is four tags and no hit testing (Pitch ch. 6, "Speaking a subset"):
//! `Gui.Window`, `Gui.Scene`, `Gui.Metrics`, `Gui.Focus`.
//!
//! - [`world`]: the [`World`] the tick loop calls, with no display in it.
//! - [`render`]: a draw list painted onto a pixmap (tiny-skia).
//! - [`font`]: the bundled DejaVu Sans, `worlds/canvas/src/font.rs` itself.
//! - `app`: winit's event loop, the windows and their surfaces (softbuffer).

#[path = "../../canvas/src/font.rs"]
pub mod font;
pub mod render;
pub mod world;

mod app;

use std::cell::RefCell;
use std::rc::Rc;

use prism_rtl::world::EXIT_FAILED;
use prism_rtl::{Program, World};
use prism_world_cli::UnwrapWorld;

pub use world::{CanvasWorld, Shared, TAGS};

/// The families a program under this World may emit at its root: `Gui`, and the
/// prelude's, which the [`UnwrapWorld`] around it answers.
pub const FAMILIES: &[&str] = &["Gui", "Prelude"];

/// Run the program `P` under the canvas World, inside the prelude's `Prelude.Unwrap`
/// policy, until it ends: its exit code. It takes no arguments and answers no
/// `Process.Args`.
///
/// This must be called on the main thread (macOS requires it of the event loop).
pub fn run<P: Program>() -> u8 {
    run_with::<P>(|w| Box::new(UnwrapWorld::new(w)))
}

/// [`run`], with `wrap` putting the World inside whatever answers the rest of the
/// program's families; `|w| Box::new(w)` for none.
pub fn run_with<P: Program>(wrap: impl FnOnce(CanvasWorld) -> Box<dyn World>) -> u8 {
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
    let mut app = app::App::<P>::new(world, shared, render::Renderer::new(font));
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("prism canvas: the event loop failed: {e}");
        return EXIT_FAILED;
    }
    app.exit.unwrap_or(0)
}
