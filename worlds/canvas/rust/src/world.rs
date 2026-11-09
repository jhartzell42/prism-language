//! The canvas World as the tick loop sees it: a [`World`] over `prism_rtl`'s protocol that
//! reads each tick's patch into a table of windows and scenes, answers the first-value
//! query from its font, and queues the windows' raw input for the next ticks. Nothing
//! here touches a display, so it is tested without one; `app` is the half that opens
//! windows and paints.
//!
//! What it answers (the four tags of Pitch ch. 6, as `prelude/gui.prism` declares them):
//!
//! - **`Gui.Window(title, size)`**: the first value is `WindowId title`, the Test World's
//!   policy (examples/README.md, doubt WI), so a program sees the same id under both. One
//!   operating-system window per place, `size` logical pixels inside; a new `title`
//!   retitles it. Its `!WindowInput` stream is answered by path: `Press Point` for the left
//!   button, `Pressed Key` for the keys `Key` names, `Resized Extent` when the window's
//!   size changes.
//! - **`Gui.Scene(window, draw)`**: drawn into every window whose id is `window`, in
//!   document order, on every patch that changes a scene or a window.
//! - **`Gui.Metrics(text)`**: the first value is the text's extent in the bundled font
//!   (`font`), a pure computation over what the World holds, so a ready state.
//! - **`Gui.Focus(window, at)`**: accepted and ignored. The window library draws its own
//!   focus ring.
//!
//! Any other element that reaches this World is reported once on standard error and
//! otherwise ignored; `Prelude.*` belongs to the [`prism_world_cli::UnwrapWorld`] this
//! World runs inside, and is passed over in silence.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;

use prism_rtl::Float;
use prism_rtl::{Path, Pod, ReadyState, TickIn, TickOut, World};

use crate::font::Font;

/// The four tags this World answers.
pub const TAGS: &[&str] = &["Gui.Window", "Gui.Scene", "Gui.Metrics", "Gui.Focus"];

/// The `Gui` family's values as plain data (`prelude/gui.prism`).
pub mod gui {
    use super::*;

    fn float(x: f64) -> Pod {
        Pod::Float(Float(x))
    }
    /// `WindowId title`.
    pub fn window_id(title: &str) -> Pod {
        Pod::Choice("WindowId".into(), vec![Pod::text(title)])
    }
    pub fn point(x: f64, y: f64) -> Pod {
        Pod::record("Point", vec![("x", float(x)), ("y", float(y))])
    }
    pub fn extent(w: f64, h: f64) -> Pod {
        Pod::record("Extent", vec![("w", float(w)), ("h", float(h))])
    }
    pub fn rect(x: f64, y: f64, w: f64, h: f64) -> Pod {
        Pod::record("Rect", vec![("x", float(x)), ("y", float(y)), ("w", float(w)), ("h", float(h))])
    }
    /// `Press Point(x, y)`.
    pub fn press(x: f64, y: f64) -> Pod {
        Pod::Choice("Press".into(), vec![point(x, y)])
    }
    /// `Resized Extent(w, h)`.
    pub fn resized(w: f64, h: f64) -> Pod {
        Pod::Choice("Resized".into(), vec![extent(w, h)])
    }
    /// `Pressed key`.
    pub fn pressed(key: Pod) -> Pod {
        Pod::Choice("Pressed".into(), vec![key])
    }
}

/// A `Gui.Window` place as the World holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowPlace {
    /// The id its first value answered, `WindowId title`, fixed for the place's life.
    pub id: Pod,
    /// The current title.
    pub title: String,
    /// The first size, logical pixels.
    pub size: (f64, f64),
}

/// A `Gui.Scene` place: the window it draws into and its current draw list.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenePlace {
    pub window: Pod,
    pub draw: Pod,
}

/// What the World and the window side share: the document's windows and scenes, by
/// place path, and the input waiting for a tick.
#[derive(Default)]
pub struct Shared {
    pub windows: BTreeMap<Path, WindowPlace>,
    pub scenes: BTreeMap<Path, ScenePlace>,
    /// Window input, oldest first, each addressed to a `Gui.Window` place.
    pub queue: VecDeque<(Path, Pod)>,
    /// Set when a patch moved a window or a scene; the window side clears it when it has
    /// caught up.
    pub dirty: bool,
}

impl Shared {
    /// Queue one window input. A resize directly after a resize of the same window
    /// replaces it, so dragging a window's corner costs one tick per frame, not per pixel.
    pub fn push_input(&mut self, window: Path, input: Pod) {
        if let Some((p, last)) = self.queue.back_mut()
            && is_resize(&input)
            && *p == window
            && is_resize(last)
        {
            *last = input;
            return;
        }
        self.queue.push_back((window, input));
    }

    /// The draw lists of every scene for the window `id`, in document order.
    pub fn draw_lists(&self, id: &Pod) -> Vec<Pod> {
        self.scenes.values().filter(|s| s.window == *id).map(|s| s.draw.clone()).collect()
    }
}

fn is_resize(v: &Pod) -> bool {
    matches!(v.structure(), Pod::Choice(c, _) if &**c == "Resized")
}

/// The ready state: `Gui.Window` answered with its id, `Gui.Metrics` from the font.
pub struct CanvasReady {
    font: Rc<Font>,
}

fn text_field(e: &Pod, name: &str) -> Option<String> {
    e.field(name).and_then(Pod::as_text).map(|t| t.to_string())
}

impl ReadyState for CanvasReady {
    fn first_value(&self, element: &Pod) -> Option<Pod> {
        match element.tag()? {
            "Gui.Window" => Some(gui::window_id(&text_field(element, "title")?)),
            "Gui.Metrics" => {
                let (w, h) = self.font.extent(&text_field(element, "text")?);
                Some(gui::extent(w, h))
            }
            _ => None,
        }
    }
}

/// The canvas World.
pub struct CanvasWorld {
    shared: Rc<RefCell<Shared>>,
    font: Rc<Font>,
    /// Tags already reported as not answered here.
    warned: BTreeSet<String>,
}

impl CanvasWorld {
    pub fn new(shared: Rc<RefCell<Shared>>, font: Rc<Font>) -> CanvasWorld {
        CanvasWorld { shared, font, warned: BTreeSet::new() }
    }

    /// Say once that an element of `tag` reached this World and was not answered.
    fn not_answered(&mut self, tag: &str) {
        if tag.starts_with("Prelude.") || !self.warned.insert(tag.to_string()) {
            return;
        }
        eprintln!(
            "prism canvas: this World does not answer {tag}; it answers {} (a Ui element needs a @Window around it)",
            TAGS.join(", ")
        );
    }

    /// A place added or changed: take what the window side needs from its element.
    fn place(&mut self, path: &Path, element: &Pod, added: bool) {
        let Some(tag) = element.tag() else { return };
        let mut shared = self.shared.borrow_mut();
        match tag {
            "Gui.Window" => {
                let title = text_field(element, "title").unwrap_or_default();
                match shared.windows.get_mut(path) {
                    // `size` is identity and the id was answered once: only the title moves.
                    Some(w) if !added => w.title = title,
                    _ => {
                        let size = element.field("size").map(|s| (number(s, "w"), number(s, "h"))).unwrap_or((640.0, 480.0));
                        let id = gui::window_id(&title);
                        shared.windows.insert(path.clone(), WindowPlace { id, title, size });
                    }
                }
                shared.dirty = true;
            }
            "Gui.Scene" => {
                let window = element.field("window").cloned().unwrap_or(Pod::Unit);
                let draw = element.field("draw").cloned().unwrap_or(Pod::Array(vec![]));
                shared.scenes.insert(path.clone(), ScenePlace { window, draw });
                shared.dirty = true;
            }
            "Gui.Metrics" | "Gui.Focus" => {}
            other => {
                let other = other.to_string();
                drop(shared);
                self.not_answered(&other);
            }
        }
    }
}

fn number(v: &Pod, field: &str) -> f64 {
    crate::render::number(v.field(field))
}

impl World for CanvasWorld {
    fn ready(&self) -> Rc<dyn ReadyState> {
        Rc::new(CanvasReady { font: self.font.clone() })
    }

    /// None of the four tags has a request field, and the prelude's requests are the
    /// wrapper's, so a request is never this World's. The patch: removals first, so a
    /// place rebuilt at a new path in the same tick is not drawn twice.
    fn perform(&mut self, out: &TickOut) {
        {
            let mut shared = self.shared.borrow_mut();
            for (path, _) in &out.patch.removed {
                let had_window = shared.windows.remove(path).is_some();
                let had_scene = shared.scenes.remove(path).is_some();
                shared.dirty |= had_window || had_scene;
                // Input queued for a window that left goes nowhere (a late result).
                if had_window {
                    shared.queue.retain(|(p, _)| p != path);
                }
            }
        }
        for (path, e) in &out.patch.added {
            self.place(path, e, true);
        }
        for (path, e) in &out.patch.changed {
            self.place(path, e, false);
        }
    }

    /// One window input per tick, oldest first: two presses never land in one tick, as
    /// in prism-reflex's Gloss World ("one turn per input").
    fn next_inputs(&mut self) -> Option<TickIn> {
        let (path, input) = self.shared.borrow_mut().queue.pop_front()?;
        Some(TickIn::none().answer(path, input))
    }

    /// The canvas World never asks a program to exit; closing a window ends the run
    /// (`app`).
    fn exit_code(&self) -> Option<u8> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_rtl::{Patch, Segment};

    fn world() -> (CanvasWorld, Rc<RefCell<Shared>>) {
        let shared = Rc::new(RefCell::new(Shared::default()));
        (CanvasWorld::new(shared.clone(), Rc::new(Font::bundled())), shared)
    }

    fn outputs(patch: Patch) -> TickOut {
        TickOut { patch, ..TickOut::default() }
    }

    fn at(n: u32) -> Path {
        Path(vec![Segment::Pos(n)])
    }

    #[test]
    fn a_window_is_answered_with_its_title_and_metrics_from_the_font() {
        let (w, _) = world();
        let ready = w.ready();
        let window = Pod::element("Gui.Window", vec![("title", Pod::text("Chess")), ("size", gui::extent(512.0, 560.0))]);
        assert_eq!(ready.first_value(&window), Some(gui::window_id("Chess")));
        let metrics = Pod::element("Gui.Metrics", vec![("text", Pod::text("turn"))]);
        let (tw, th) = Font::bundled().extent("turn");
        assert_eq!(ready.first_value(&metrics), Some(gui::extent(tw, th)));
        assert_eq!(ready.first_value(&Pod::element("Gui.Focus", vec![])), None);
    }

    #[test]
    fn the_patch_keeps_windows_and_scenes_by_path() {
        let (mut w, shared) = world();
        let window = Pod::element("Gui.Window", vec![("title", Pod::text("Chess")), ("size", gui::extent(512.0, 560.0))]);
        let scene = Pod::element("Gui.Scene", vec![("window", gui::window_id("Chess")), ("draw", Pod::Array(vec![]))]);
        w.perform(&outputs(Patch { added: vec![(at(1), window), (at(3), scene)], changed: vec![], removed: vec![] }));
        {
            let s = shared.borrow();
            assert_eq!(s.windows[&at(1)], WindowPlace { id: gui::window_id("Chess"), title: "Chess".into(), size: (512.0, 560.0) });
            assert_eq!(s.draw_lists(&gui::window_id("Chess")).len(), 1);
            assert!(s.dirty);
        }
        // Retitling keeps the id the first value answered.
        let retitled = Pod::element("Gui.Window", vec![("title", Pod::text("Mate")), ("size", gui::extent(512.0, 560.0))]);
        w.perform(&outputs(Patch { added: vec![], changed: vec![(at(1), retitled.clone())], removed: vec![] }));
        assert_eq!(shared.borrow().windows[&at(1)].id, gui::window_id("Chess"));
        assert_eq!(shared.borrow().windows[&at(1)].title, "Mate");
        w.perform(&outputs(Patch { added: vec![], changed: vec![], removed: vec![(at(1), retitled), (at(3), Pod::Unit)] }));
        assert!(shared.borrow().windows.is_empty() && shared.borrow().scenes.is_empty());
    }

    #[test]
    fn inputs_go_one_per_tick_and_resizes_coalesce() {
        let (mut w, shared) = world();
        {
            let mut s = shared.borrow_mut();
            s.push_input(at(1), gui::resized(600.0, 600.0));
            s.push_input(at(1), gui::resized(700.0, 650.0));
            s.push_input(at(1), gui::press(10.0, 20.0));
            s.push_input(at(1), gui::press(30.0, 40.0));
        }
        let mut got = vec![];
        while let Some(i) = w.next_inputs() {
            assert_eq!(i.answers.len(), 1);
            got.push(i.answers[0].1.clone());
        }
        assert_eq!(got, vec![gui::resized(700.0, 650.0), gui::press(10.0, 20.0), gui::press(30.0, 40.0)]);
    }
}
