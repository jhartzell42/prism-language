//! A window on a real X server: the program of tests/common opens one `Gui.Window` with a
//! red rectangle in it. A second thread, a
//! client of its own on the same display, waits until the rectangle's pixel on the root
//! window is red (no window manager under Xvfb, so the window sits at the origin), then
//! clicks it through XTest. The program hears the `Press` and tears both places down;
//! with no window left and nothing owed, the World ends the run with 0.
//!
//! With no `DISPLAY`, the test starts itself again under `xvfb-run -a`; with no `xvfb-run`
//! it says so and passes. The child has 30 seconds.

mod common;

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use common::{HEARD, SmokeProgram};

/// Set when the scene's red showed on the screen.
static PAINTED: AtomicBool = AtomicBool::new(false);

/// The robot: look for red at the centre, then click there.
#[cfg(target_os = "linux")]
fn robot() {
    use x11_dl::{xlib, xtest};
    let (Ok(x), Ok(t)) = (xlib::Xlib::open(), xtest::Xf86vmode::open()) else {
        eprintln!("smoke: no libX11 or libXtst to click with");
        return;
    };
    unsafe {
        let d = (x.XOpenDisplay)(std::ptr::null());
        if d.is_null() {
            eprintln!("smoke: the robot cannot open the display");
            return;
        }
        let root = (x.XDefaultRootWindow)(d);
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            let image = (x.XGetImage)(
                d,
                root,
                CENTRE.0,
                CENTRE.1,
                1,
                1,
                (x.XAllPlanes)(),
                xlib::ZPixmap,
            );
            if !image.is_null() {
                let px = (x.XGetPixel)(image, 0, 0);
                (x.XDestroyImage)(image);
                if (px >> 16 & 0xff, px >> 8 & 0xff, px & 0xff) == (200, 40, 40) {
                    PAINTED.store(true, Ordering::SeqCst);
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        (t.XTestFakeMotionEvent)(d, -1, CENTRE.0, CENTRE.1, 0);
        (t.XTestFakeButtonEvent)(d, 1, 1, 0);
        (t.XTestFakeButtonEvent)(d, 1, 0, 0);
        (x.XFlush)(d);
        std::thread::sleep(Duration::from_millis(200));
        (x.XCloseDisplay)(d);
    }
}

#[cfg(not(target_os = "linux"))]
fn robot() {}

fn main() {
    if std::env::var_os("DISPLAY").is_some() {
        std::thread::spawn(robot);
        let code = prism_world_canvas_rtl::run::<SmokeProgram>();
        assert_eq!(code, 0, "the run's exit code");
        assert!(
            PAINTED.load(Ordering::SeqCst),
            "the scene's red reached the screen"
        );
        assert!(
            HEARD.load(Ordering::SeqCst),
            "the window's input reached the program"
        );
        println!("xvfb_smoke: a window opened, painted, heard a click, closed; exit 0");
        return;
    }
    if std::env::var_os("PRISM_SMOKE_CHILD").is_some() {
        panic!("xvfb-run gave the child no DISPLAY");
    }
    let me = std::env::current_exe().unwrap();
    let child = std::process::Command::new("xvfb-run")
        .arg("-a")
        .arg(&me)
        .env("PRISM_SMOKE_CHILD", "1")
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            println!("xvfb_smoke: skipped, no display and no xvfb-run ({e})");
            return;
        }
    };
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(
                status.success(),
                "the child under xvfb-run failed: {status}"
            );
            println!("xvfb_smoke: passed under xvfb-run");
            return;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("the window never sent input within 30 s");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
