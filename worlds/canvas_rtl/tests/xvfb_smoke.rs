//! A window on a real X server: a hand-written program (in docs/backend.md's shape) opens
//! one `Gui.Window` with a `Gui.Scene` that fills a red rectangle. A second thread, a
//! client of its own on the same display, waits until the rectangle's pixel on the root
//! window is red (no window manager under Xvfb, so the window sits at the origin), then
//! clicks it through XTest. The program hears the `Press` and tears both places down;
//! with no window left and nothing owed, the World ends the run with 0.
//!
//! With no `DISPLAY`, the test starts itself again under `xvfb-run -a`; with no `xvfb-run`
//! it says so and passes. The child has 30 seconds.

use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use prism_rtl::*;
use prism_world_canvas_rtl::world::gui;

/// Set when the window's first input reached the program.
static HEARD: AtomicBool = AtomicBool::new(false);
/// Set when the scene's red showed on the screen.
static PAINTED: AtomicBool = AtomicBool::new(false);

/// The rectangle the scene fills, in the window's (and, with no window manager, the
/// screen's) pixels: its centre is where the robot looks and clicks.
const CENTRE: (i32, i32) = (60, 35);

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
            let image = (x.XGetImage)(d, root, CENTRE.0, CENTRE.1, 1, 1, (x.XAllPlanes)(), xlib::ZPixmap);
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
            return Err(Fault::answer(format!("no answering place at {}", Path(at.to_vec()))));
        }
        let Pod::Choice(c, _) = v.structure() else { return Err(Fault::answer(format!("expected WindowInput, got {v}"))) };
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

struct SmokeProgram {
    rt: Runtime,
    root: Rc<Smoke>,
}

impl Program for SmokeProgram {
    fn build(_: &[Pod], ready: &dyn ReadyState) -> Result<(Self, TickOut), Fault> {
        let mut rt = Runtime::new();
        let mut cx = rt.build(ready);
        let root = Rc::new(Smoke { path: Path::root(), open: true.into() });
        let window = Pod::element("Gui.Window", vec![("title", Pod::text("Smoke")), ("size", gui::extent(200.0, 100.0))]);
        let id = cx.first_value(&window).ok_or(Fault::answer("Gui.Window unanswered"))?;
        cx.added(root.path.at(0), window);
        let rect = Pod::element("Draw.Rect", vec![("rect", gui::rect(10.0, 10.0, 100.0, 50.0)), ("paint", Pod::name("Red"))]);
        cx.added(root.path.at(1), Pod::element("Gui.Scene", vec![("window", id), ("draw", Pod::Array(vec![rect]))]));
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

fn main() {
    if std::env::var_os("DISPLAY").is_some() {
        std::thread::spawn(robot);
        let code = prism_world_canvas_rtl::run::<SmokeProgram>();
        assert_eq!(code, 0, "the run's exit code");
        assert!(PAINTED.load(Ordering::SeqCst), "the scene's red reached the screen");
        assert!(HEARD.load(Ordering::SeqCst), "the window's input reached the program");
        println!("xvfb_smoke: a window opened, painted, heard a click, closed; exit 0");
        return;
    }
    if std::env::var_os("PRISM_SMOKE_CHILD").is_some() {
        panic!("xvfb-run gave the child no DISPLAY");
    }
    let me = std::env::current_exe().unwrap();
    let child = std::process::Command::new("xvfb-run").arg("-a").arg(&me).env("PRISM_SMOKE_CHILD", "1").spawn();
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
            assert!(status.success(), "the child under xvfb-run failed: {status}");
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
