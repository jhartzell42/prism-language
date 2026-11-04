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

use prism_rtl::{Path, Pod, Program, TickIn, TickOut, World};
use softbuffer::{Context, Surface};
use tiny_skia::Pixmap;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId as OsWindowId};

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
    /// The last frame presented, and its scale, for `Session::pixel`.
    last: Option<(Pixmap, f64)>,
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
                self.world.perform(&out);
            }
            Err(fault) => {
                let code = self.world.failed(&fault);
                self.exit.get_or_insert(code);
                return;
            }
        }
        if let Some(code) = self.world.exit_code() {
            self.exit.get_or_insert(code);
        }
        if let Some(p) = self.program.as_mut() {
            p.compact();
        }
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
                    last: None,
                })
            });
            match opened {
                Ok(w) => {
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

    /// Paint one window's scenes and show them.
    fn redraw(&mut self, path: &Path) {
        let Some(w) = self.windows.get_mut(path) else { return };
        let size = w.window.inner_size();
        let (Some(width), Some(height)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else { return };
        let Some(mut pixmap) = Pixmap::new(size.width, size.height) else { return };
        let draws = self.shared.borrow().draw_lists(&w.id);
        self.renderer.render(&mut pixmap, w.window.scale_factor() as f32, &draws);
        let shown = w.surface.resize(width, height).and_then(|()| {
            let mut buffer = w.surface.buffer_mut()?;
            // Every pixel is opaque (the background is), so premultiplied is plain RGB.
            for (dst, px) in buffer.iter_mut().zip(pixmap.pixels()) {
                *dst = (px.red() as u32) << 16 | (px.green() as u32) << 8 | px.blue() as u32;
            }
            buffer.present()
        });
        match shown {
            Ok(()) => {
                w.damaged = false;
                w.last = Some((pixmap, w.window.scale_factor()));
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
            WindowEvent::RedrawRequested => self.redraw(&path),
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
        let (pixmap, scale) = self.app.windows.get(window)?.last.as_ref()?;
        let (px, py) = ((x * scale).round(), (y * scale).round());
        if px < 0.0 || py < 0.0 {
            return None;
        }
        let p = pixmap.pixel(px as u32, py as u32)?;
        Some((p.red() as u32) << 16 | (p.green() as u32) << 8 | p.blue() as u32)
    }
}
