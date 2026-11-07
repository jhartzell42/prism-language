//! A window with no robot: the program of tests/common opens one `Gui.Window` with a red
//! rectangle in it. When the loop settles, the hook checks that the rectangle's pixel in
//! the presented frame is red, then presses there through `Session::input`, as a click
//! would. The program hears the `Press` and tears both places down; the World ends the
//! run with 0. worlds/canvas/cpp's macos_smoke in Rust. It needs a window system: on
//! Linux with no `DISPLAY` it says so and passes (xvfb_smoke covers Linux).
//!
//! Written by Claude on the claude-self-hosting branch; subject to change.

mod common;

use std::sync::atomic::Ordering;

use common::{CENTRE, HEARD, RED, SmokeProgram};
use prism_world_canvas_rtl::world::gui;
use prism_world_canvas_rtl::{Hooks, Session, run_hooked};
use prism_world_cli::UnwrapWorld;

fn main() {
    if cfg!(target_os = "linux") && std::env::var_os("DISPLAY").is_none() {
        println!("hooks_smoke: skipped, no DISPLAY");
        return;
    }
    let painted = std::rc::Rc::new(std::cell::Cell::new(false));
    let seen = painted.clone();
    let mut pressed = false;
    let hooks = Hooks {
        settled: Some(Box::new(move |s: &mut dyn Session| {
            if pressed {
                return;
            }
            let Some(window) = s.shared().windows.keys().next().cloned() else { return };
            let Some(px) = s.pixel(&window, CENTRE.0 as f64, CENTRE.1 as f64) else { return };
            if px != RED {
                eprintln!("hooks_smoke: the rectangle's pixel is {px:06x}, not {RED:06x}");
            }
            seen.set(px == RED);
            pressed = true;
            s.input(&window, gui::press(CENTRE.0 as f64, CENTRE.1 as f64));
        })),
    };
    let code = run_hooked::<SmokeProgram>(|w| Box::new(UnwrapWorld::new(w)), hooks);
    assert_eq!(code, 0, "the run's exit code");
    assert!(painted.get(), "the scene's red reached the presented frame");
    assert!(HEARD.load(Ordering::SeqCst), "the press reached the program");
    println!("hooks_smoke: a window opened, painted red, heard a press, closed; exit 0");
}
