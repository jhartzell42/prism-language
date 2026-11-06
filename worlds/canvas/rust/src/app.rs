//! The window side: winit's event loop, which owns time here. It builds the program when
//! the loop starts, ticks on each window input and whenever the program is `pending`,
//! makes the operating system's windows match the document's `Gui.Window` places after
//! every tick, and repaints every window when a patch moved a window or a scene.
//!
//! The loop is `prism_rtl::drive` (runtime/rust/src/world/mod.rs) turned inside
//! out, because winit calls us rather than the other way round: the same build,
//! `perform`, exit check, compact round after every tick and `pending` rule, run from
//! `about_to_wait`, which winit
//! calls once the events of one wake-up are delivered. Ticks run for at most a frame's
//! time per wake-up, so a long `@Run` keeps the windows responsive; while ticks are
//! owed the loop polls, and otherwise it sleeps until the next event.
//!
//! The run ends when the program asks (a fault the World's policy ends on, or a poisoned
//! root from the prelude's wrapper), when the user closes a window (code 0), or when the
//! document holds no window and nothing can cause another tick (code 0, as `drive`).

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};

use prism_rtl::world::probe::{self, Phase};
use prism_rtl::{Path, Pod, Program, TickIn, TickOut, World};
use softbuffer::{Context, Surface};
use tiny_skia::Pixmap;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId as OsWindowId};

use crate::damage::{Damage, Painter};
use crate::render::Renderer;
use crate::world::{Shared, gui};
use crate::{Hooks, Session};

/// How long one wake-up of the loop may spend ticking before it lets the windows
/// repaint and take input.
const FRAME_BUDGET: Duration = Duration::from_millis(16);

/// One operating-system window, for one `Gui.Window` place.
struct OsWindow {
    window: Rc<Window>,
    surface: Surface<Rc<Window>, Rc<Window>>,
    /// The place's id: scenes naming it draw here.
    id: Pod,
    title: String,
    /// The last size the program was told, logical pixels.
    size: (f64, f64),
    /// The pointer, physical pixels, from the last `CursorMoved`.
    cursor: PhysicalPosition<f64>,
    /// Asked to redraw and not yet presented.
    damaged: bool,
    /// The frame, painted again where the scene changed (`damage.rs`), with the scale it
    /// was painted at; `Session::pixel` reads it once it has been presented.
    frame: Option<(Pixmap, f64)>,
    painter: Painter,
    /// The frame as the screen takes it, 0x00RRGGBB, updated where it was painted.
    xrgb: Vec<u32>,
    /// The frame has been presented since it was last painted.
    presented: bool,
}

/// The program, its World, and the windows.
pub struct App<P: Program> {
    /// Built when the loop starts.
    program: Option<P>,
    world: Box<dyn World>,
    shared: Rc<RefCell<Shared>>,
    renderer: Renderer,
    /// The last tick said another is owed.
    pending: bool,
    built: bool,
    /// Ever had a window: a program that never opens one is told why nothing showed.
    opened_any: bool,
    windows: BTreeMap<Path, OsWindow>,
    places: HashMap<OsWindowId, Path>,
    /// The code the run ends with, once something has ended it.
    pub exit: Option<u8>,
    hooks: Hooks,
}

impl<P: Program> App<P> {
    pub fn new(world: Box<dyn World>, shared: Rc<RefCell<Shared>>, renderer: Renderer, hooks: Hooks) -> App<P> {
        App {
            program: None,
            world,
            shared,
            renderer,
            pending: false,
            built: false,
            opened_any: false,
            windows: BTreeMap::new(),
            places: HashMap::new(),
            exit: None,
            hooks,
        }
    }

    /// One tick, the build when there is no program yet: `drive`'s loop body. A failed
    /// tick is the World's to decide, and the CLI policy it keeps ends the run.
    fn tick_once(&mut self, input: TickIn) {
        let ready = self.world.ready();
        let result: Result<TickOut, _> = match self.program.as_mut() {
            Some(p) => p.tick(&input, &*ready),
            None => P::build(&[], &*ready).map(|(p, out)| {
                self.program = Some(p);
                out
            }),
        };
        match result {
            Ok(out) => {
                self.pending = out.pending;
                // `PRISM_RESULTS` set: each result the root gives, on standard error, for a
                // driver to read (the xvfb run of chess checks its status this way).
                if let (Some(prism_rtl::Outcome::Value(v)), Ok(_)) = (&out.result, std::env::var("PRISM_RESULTS")) {
                    eprintln!("result: {v}");
                }
                probe::enter(Phase::Perform);
                self.world.perform(&out);
            }
            Err(fault) => {
                probe::enter(Phase::Loop);
                let code = self.world.failed(&fault);
                self.exit.get_or_insert(code);
                return;
            }
        }
        if let Some(code) = self.world.exit_code() {
            self.exit.get_or_insert(code);
        }
        if let Some(p) = self.program.as_mut() {
            probe::enter(Phase::Compact);
            p.compact();
        }
        probe::enter(Phase::Loop);
    }

    /// Tick while the World has input or a tick is owed, for at most a frame. True when
    /// there is more to do.
    fn run_ticks(&mut self) -> bool {
        let deadline = Instant::now() + FRAME_BUDGET;
        while self.exit.is_none() {
            let input = match self.world.next_inputs() {
                Some(i) => i,
                None if self.pending => TickIn::none(),
                None => return false,
            };
            self.tick_once(input);
            if Instant::now() >= deadline {
                return true;
            }
        }
        false
    }

    /// Make the operating system's windows match the document: open the new places'
    /// windows, close the ones whose place left, retitle, and repaint after a change.
    fn reconcile(&mut self, event_loop: &ActiveEventLoop) {
        let mut shared = self.shared.borrow_mut();
        let gone: Vec<Path> = self.windows.keys().filter(|p| !shared.windows.contains_key(*p)).cloned().collect();
        for p in gone {
            if let Some(w) = self.windows.remove(&p) {
                self.places.remove(&w.window.id());
            }
        }
        for (path, place) in &shared.windows {
            if let Some(w) = self.windows.get_mut(path) {
                if w.title != place.title {
                    w.window.set_title(&place.title);
                    w.title = place.title.clone();
                }
                continue;
            }
            let attributes = Window::default_attributes()
                .with_title(place.title.clone())
                .with_inner_size(LogicalSize::new(place.size.0, place.size.1));
            let opened = event_loop.create_window(attributes).map_err(|e| e.to_string()).and_then(|window| {
                let window = Rc::new(window);
                let context = Context::new(window.clone()).map_err(|e| e.to_string())?;
                let surface = Surface::new(&context, window.clone()).map_err(|e| e.to_string())?;
                Ok(OsWindow {
                    window,
                    surface,
                    id: place.id.clone(),
                    title: place.title.clone(),
                    size: place.size,
                    cursor: PhysicalPosition::new(0.0, 0.0),
                    damaged: true,
                    frame: None,
                    painter: Painter::new(),
                    xrgb: Vec::new(),
                    presented: false,
                })
            });
            match opened {
                Ok(w) => {
                    srgb_window(&w.window);
                    self.places.insert(w.window.id(), path.clone());
                    self.windows.insert(path.clone(), w);
                    self.opened_any = true;
                }
                Err(e) => {
                    eprintln!("prism canvas: cannot open the window {:?}: {e}", place.title);
                    self.exit.get_or_insert(prism_rtl::world::EXIT_FAILED);
                }
            }
        }
        // A World presents only settled states: while the program owes a tick
        // (`pending`), the scene may show one host deep a tick ahead of another, so the
        // redraw waits until the owed ticks have run (Semantics ch. 9, "A World presents
        // settled states").
        if shared.dirty && !self.pending {
            shared.dirty = false;
            for w in self.windows.values_mut() {
                w.damaged = true;
                w.window.request_redraw();
            }
        }
    }

    /// Paint one window's scenes where they changed, and show the frame. The whole frame
    /// is handed to softbuffer: its macOS buffer starts empty every time (`age` 0), and on
    /// X11 a redraw for an expose cannot be told from one for a new scene.
    fn redraw(&mut self, path: &Path) {
        let Some(w) = self.windows.get_mut(path) else { return };
        let size = w.window.inner_size();
        let (Some(width), Some(height)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else { return };
        let scale = w.window.scale_factor();
        if w.frame.as_ref().map(|(p, _)| (p.width(), p.height())) != Some((size.width, size.height)) {
            let Some(pixmap) = Pixmap::new(size.width, size.height) else { return };
            w.frame = Some((pixmap, scale));
            w.xrgb = vec![0; size.width as usize * size.height as usize];
        }
        let Some((pixmap, painted_at)) = w.frame.as_mut() else { return };
        *painted_at = scale;
        let draws = self.shared.borrow().draw_lists(&w.id);
        let damage = w.painter.paint(&mut self.renderer, pixmap, scale as f32, &draws);
        // Every pixel is opaque (the background is), so premultiplied is plain RGB.
        let xrgb = |px: &tiny_skia::PremultipliedColorU8| (px.red() as u32) << 16 | (px.green() as u32) << 8 | px.blue() as u32;
        let pixels = pixmap.pixels();
        match &damage {
            Damage::Full => {
                for (dst, px) in w.xrgb.iter_mut().zip(pixels) {
                    *dst = xrgb(px);
                }
            }
            Damage::Rects(boxes) => {
                for r in boxes {
                    for y in r.y0..r.y1 {
                        let row = (y * size.width + r.x0) as usize..(y * size.width + r.x1) as usize;
                        for (dst, px) in w.xrgb[row.clone()].iter_mut().zip(&pixels[row]) {
                            *dst = xrgb(px);
                        }
                    }
                }
            }
        }
        w.presented = false;
        let shown = w.surface.resize(width, height).and_then(|()| {
            let mut buffer = w.surface.buffer_mut()?;
            buffer.copy_from_slice(&w.xrgb);
            buffer.present()
        });
        match shown {
            Ok(()) => {
                w.damaged = false;
                w.presented = true;
            }
            Err(e) => eprintln!("prism canvas: cannot paint {:?}: {e}", w.title),
        }
    }

    /// One window event, as input for the window's place.
    fn input(&mut self, path: &Path, event: &WindowEvent) {
        let Some(w) = self.windows.get_mut(path) else { return };
        let scale = w.window.scale_factor();
        let mut shared = self.shared.borrow_mut();
        match event {
            WindowEvent::CursorMoved { position, .. } => w.cursor = *position,
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                let at = w.cursor.to_logical::<f64>(scale);
                shared.push_input(path.clone(), gui::press(at.x, at.y));
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                for key in keys_of(&event.logical_key) {
                    shared.push_input(path.clone(), gui::pressed(key));
                }
            }
            WindowEvent::Resized(physical) => {
                let logical = physical.to_logical::<f64>(scale);
                let size = (logical.width, logical.height);
                if size != w.size {
                    w.size = size;
                    shared.push_input(path.clone(), gui::resized(size.0, size.1));
                }
                w.damaged = true;
                w.window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                w.damaged = true;
                w.window.request_redraw();
            }
            _ => {}
        }
    }
}

/// Give the window the frame's colour space, sRGB: Core Animation then hands the frame
/// to the window server as it is, where with the screen's colour space it converts every
/// pixel of every frame on the CPU, in this process, before the frame reaches the screen
/// (on an M2 Pro's built-in screen, about 125 M instructions a 900 by 800 frame). The
/// colours on the screen are the same: the window server matches sRGB to the screen.
#[cfg(target_os = "macos")]
fn srgb_window(window: &Window) {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let Ok(handle) = window.window_handle() else { return };
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else { return };
    let view: *mut AnyObject = appkit.ns_view.as_ptr().cast();
    // SAFETY: `view` is the live NSView winit made for this window, on the main thread;
    // `window`, `sRGBColorSpace` and `setColorSpace:` are AppKit's own methods.
    unsafe {
        let ns_window: *mut AnyObject = msg_send![view, window];
        let srgb: *mut AnyObject = msg_send![class!(NSColorSpace), sRGBColorSpace];
        if !ns_window.is_null() {
            let _: () = msg_send![ns_window, setColorSpace: srgb];
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn srgb_window(_: &Window) {}

/// A key as the window library's `Key`: `Char c`, `Left`, `Right`, `Backspace`, `Tab`,
/// `Enter`; Delete is taken as Backspace and Space as `Char ' '`, as prism-reflex's Gloss
/// World does. Other keys are not reported.
fn keys_of(key: &Key) -> Vec<Pod> {
    let named = |name: &str| vec![Pod::name(name)];
    match key {
        Key::Named(NamedKey::Backspace | NamedKey::Delete) => named("Backspace"),
        Key::Named(NamedKey::Tab) => named("Tab"),
        Key::Named(NamedKey::Enter) => named("Enter"),
        Key::Named(NamedKey::ArrowLeft) => named("Left"),
        Key::Named(NamedKey::ArrowRight) => named("Right"),
        Key::Named(NamedKey::Space) => vec![Pod::Choice("Char".into(), vec![Pod::Char(' ')])],
        Key::Character(s) => s.chars().map(|c| Pod::Choice("Char".into(), vec![Pod::Char(c)])).collect(),
        _ => vec![],
    }
}

impl<P: Program> ApplicationHandler for App<P> {
    /// The loop has started: build the program and open its windows.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.built {
            return;
        }
        self.built = true;
        self.tick_once(TickIn::none());
        self.reconcile(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: OsWindowId, event: WindowEvent) {
        let Some(path) = self.places.get(&id).cloned() else { return };
        match event {
            WindowEvent::CloseRequested => {
                self.exit.get_or_insert(0);
                event_loop.exit();
            }
            // A redraw the system asks for while ticks are owed waits for them too.
            WindowEvent::RedrawRequested if self.pending => self.shared.borrow_mut().dirty = true,
            WindowEvent::RedrawRequested => {
                probe::enter(Phase::Paint);
                self.redraw(&path);
                probe::enter(Phase::Loop);
            }
            other => self.input(&path, &other),
        }
    }

    /// The events of this wake-up are in: tick, then make the windows match.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if !self.built {
            return;
        }
        let more = self.run_ticks();
        self.reconcile(event_loop);
        if self.exit.is_none() && !more && self.windows.is_empty() {
            if !self.opened_any {
                eprintln!("prism canvas: the program opened no Gui.Window; put its widget in a @Window");
            }
            self.exit = Some(0);
        }
        if self.exit.is_some() {
            event_loop.exit();
            return;
        }
        let mut queued = !self.shared.borrow().queue.is_empty();
        if !more && !self.pending && !queued && self.windows.values().all(|w| !w.damaged) {
            if let Some(mut settled) = self.hooks.settled.take() {
                settled(&mut HookSession { app: self });
                self.hooks.settled = Some(settled);
                if self.exit.is_some() {
                    event_loop.exit();
                    return;
                }
                queued = !self.shared.borrow().queue.is_empty();
            }
        }
        event_loop.set_control_flow(if more || self.pending || queued { ControlFlow::Poll } else { ControlFlow::Wait });
    }
}

/// The running World as a hook sees it.
struct HookSession<'a, P: Program> {
    app: &'a mut App<P>,
}

impl<P: Program> Session for HookSession<'_, P> {
    fn shared(&self) -> std::cell::Ref<'_, Shared> {
        self.app.shared.borrow()
    }
    fn input(&mut self, window: &Path, value: Pod) {
        self.app.shared.borrow_mut().push_input(window.clone(), value);
    }
    fn exit(&mut self, code: u8) {
        self.app.exit.get_or_insert(code);
    }
    fn pixel(&self, window: &Path, x: f64, y: f64) -> Option<u32> {
        let w = self.app.windows.get(window)?;
        if !w.presented {
            return None;
        }
        let (pixmap, scale) = w.frame.as_ref()?;
        let (px, py) = ((x * scale).round(), (y * scale).round());
        if px < 0.0 || py < 0.0 {
            return None;
        }
        let (px, py) = (px as u32, py as u32);
        if px >= pixmap.width() || py >= pixmap.height() {
            return None;
        }
        // what was handed to the screen
        w.xrgb.get(py as usize * pixmap.width() as usize + px as usize).copied()
    }
    fn frame_size(&self, window: &Path) -> Option<(u32, u32, f64)> {
        let w = self.app.windows.get(window)?;
        let (pixmap, scale) = w.frame.as_ref().filter(|_| w.presented)?;
        Some((pixmap.width(), pixmap.height(), *scale))
    }
}
