//! The canvas World over `prism_rtl`'s protocol: a window for each `Gui.Window` place,
//! painted in software, speaking the compiled programs' [`Program`] and [`World`].
//!
//! *Written by Claude, 1 November 2026, on the `claude-self-hosting` branch. Subject to
//! change: implementers use their discretion. `README.md` beside this crate says what
//! differs from `worlds/canvas/rust`; that crate's README says what the World answers, what it
//! ignores, the font, and the known limits, all of which hold here.*
//!
//! A drawing World is four tags and no hit testing (Pitch ch. 6, "Speaking a subset"):
//! `Gui.Window`, `Gui.Scene`, `Gui.Metrics`, `Gui.Focus`.
//!
//! - [`world`]: the [`World`] the tick loop calls, with no display in it.
//! - [`render`]: a draw list painted onto a pixmap (tiny-skia).
//! - [`damage`]: a window's frame painted again only where its scene changed.
//! - [`font`]: the bundled DejaVu Sans, `worlds/canvas/rust/src/font.rs` itself.
//! - `app`: winit's event loop, the windows and their surfaces (softbuffer).

pub mod damage;
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
    run_hooked::<P>(wrap, Hooks::default())
}

/// What a hook may do to a running World: queue window input as the window system would,
/// end the run, and read back what reached the screen. A benchmark or a test drives the
/// program through it (worlds/canvas/cpp's `Session`, the same five calls).
pub trait Session {
    /// The document's windows and scenes, as the World last read them.
    fn shared(&self) -> std::cell::Ref<'_, Shared>;
    /// Queue an input for the `Gui.Window` place at `window`, as a click or a key would.
    fn input(&mut self, window: &prism_rtl::Path, value: prism_rtl::Pod);
    /// End the run with `code` once the hook returns.
    fn exit(&mut self, code: u8);
    /// The last presented frame's colour, 0xRRGGBB, at logical `(x, y)` in the window for
    /// the place at `window`; none outside it or before its first frame.
    fn pixel(&self, window: &prism_rtl::Path, x: f64, y: f64) -> Option<u32>;
    /// The last presented frame's width and height in physical pixels, and its scale
    /// (physical pixels per logical pixel), for the window for the place at `window`; none
    /// before its first frame. A test reads every pixel of the frame through `pixel` at
    /// `(px / scale, py / scale)`.
    fn frame_size(&self, window: &prism_rtl::Path) -> Option<(u32, u32, f64)>;
}

/// Calls into a hook from the loop. Each may be empty.
#[derive(Default)]
pub struct Hooks {
    /// The loop has settled: no input is queued, no tick is owed, and every window has
    /// presented the document's scenes since they last changed. Called each time the loop
    /// would go to sleep.
    pub settled: Option<Box<dyn FnMut(&mut dyn Session)>>,
}

/// [`run_with`], with `hooks` called from the loop.
pub fn run_hooked<P: Program>(wrap: impl FnOnce(CanvasWorld) -> Box<dyn World>, hooks: Hooks) -> u8 {
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
    let mut app = app::App::<P>::new(world, shared, render::Renderer::new(font), hooks);
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("prism canvas: the event loop failed: {e}");
        return EXIT_FAILED;
    }
    app.exit.unwrap_or(0)
}
