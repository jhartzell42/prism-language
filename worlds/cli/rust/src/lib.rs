//! The CLI World over `prism_rtl`'s protocol (claude/backend.md, "Hosting"; Semantics ch. 9,
//! "The CLI World's policy"). It answers the `Fs`, `Process` and `Time` families of
//! `prelude/`, the prelude's own `Prelude.Unwrap` and `Prelude.Exceptions`, and the
//! `Fs.List` this crate adds for the compiler (`README.md`).
//!
//! - **`Fs.Contents(path)`** answers `$~Contents` from the ready state. When this World
//!   writes a file it pushes the new contents to every `Fs.Contents` of that path.
//! - **`Fs.Output(path, text=$Text)`** answers `$WriteState`: `Writing` first. Whenever
//!   the place is added or its text changes the World writes the file and pushes
//!   `Written`, or `WriteFailed err`, keyed by the element, into the next tick.
//! - **`Fs.File(path, requests=!FileRequest)`** answers each `Read` or `Write text` in the
//!   next tick with `~ Contents(..)` or an `Error`.
//! - **`Fs.List(dir)`** answers `$[Text]`: the `.prism` and `.prismtest` paths under `dir`,
//!   at any depth, sorted. A write that changes a listing pushes the new one.
//! - **`Process.Args`** answers the arguments, plain, once.
//! - **`Process.Env(name)`** answers the variable's value, plain, once: `nil` when it is
//!   unset or not UTF-8.
//! - **`Process.Exit(code=!U8)`** ends the program with `code` once every request of its
//!   tick is performed, in document order; the first exit of a tick wins.
//! - **`Time.After(wakes=!Wake)`** answers each wake with the same `Wake`, no sooner than
//!   its `after`, in a later tick. `@Later` rides on it.
//! - **`Prelude.Unwrap`, `Prelude.Exceptions`**: [`prelude`].
//!
//! The exit codes (Semantics ch. 9 and ch. 10): the code `Process.Exit` asked for; 0 when
//! nothing can cause another tick; 1 for a poisoned root, a failed tick, or a run that
//! exhausts the World's patience ([`RUN_PATIENCE`] ticks), each reported on standard
//! error; 2 for a program that fails to check, which is `prismc`'s to report before tick 0
//! ([`EXIT_UNCHECKED`]).
//!
//! The ready state reads the file system when a place asks: the ready state materialised
//! lazily, not the "no I/O in a query" of Semantics ch. 9 to the letter.

pub mod fs;
pub mod prelude;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use prism_rtl::{Document, Error, FromPod, Path, Pod, Program, ReadyState, TickIn, TickOut, ToPod, World};

pub use fs::{FileSystem, MemFs, RealFs};
pub use prelude::{Prelude, UnwrapWorld};
pub use prism_rtl::world::{EXIT_FAILED, EXIT_UNCHECKED};

/// The families this World answers, for a root's row check.
pub const FAMILIES: &[&str] = &["Fs", "Process", "Time", "Prelude"];

/// The CLI World's patience under [`run`]: unlimited unless `PRISM_PATIENCE` sets a budget
/// (Semantics ch. 9: a budget is the World's and optional).
pub const RUN_PATIENCE: usize = usize::MAX;

/// Run a program under the CLI World over the real file system, rooted at the working
/// directory, with `args` as `Process.Args`: the exit code.
pub fn run<P: Program>(args: Vec<String>) -> u8 {
    let root = std::env::current_dir().unwrap_or_default();
    let mut world = CliWorld::new(RealFs { root }, args);
    prism_rtl::drive::<P>(&mut world, &[], std::env::var("PRISM_PATIENCE").ok().and_then(|p| p.parse().ok()).unwrap_or(RUN_PATIENCE))
}

fn contents(r: Result<String, Error>) -> Pod {
    r.map(|text| Pod::record("Contents", vec![("text", Pod::text(text.as_str()))])).to_pod()
}

fn listing(paths: Vec<String>) -> Pod {
    Pod::Array(paths.into_iter().map(|p| Pod::text(p.as_str())).collect())
}

/// `Process.Env`'s answer: `? value`, or `nil` when the variable is unset or not UTF-8.
pub fn env_value(name: &str) -> Pod {
    match std::env::var(name) {
        Ok(v) => Pod::Some(Box::new(Pod::text(v.as_str()))),
        Err(_) => Pod::Nil,
    }
}

fn text_field(e: &Pod, name: &str) -> Option<String> {
    e.field(name).and_then(Pod::as_text).map(|t| t.to_string())
}

/// The ready state: the file system and the arguments, asked when a place is built.
struct CliReady<F: FileSystem> {
    fs: Rc<RefCell<F>>,
    args: Rc<Vec<String>>,
}

impl<F: FileSystem> ReadyState for CliReady<F> {
    fn first_value(&self, e: &Pod) -> Option<Pod> {
        match e.tag()? {
            "Fs.Contents" => Some(contents(self.fs.borrow().read(&text_field(e, "path")?))),
            "Fs.Output" => Some(Pod::name("Writing")),
            "Fs.List" => Some(listing(self.fs.borrow().list(&text_field(e, "dir")?))),
            "Process.Args" => Some(Pod::Array(self.args.iter().map(|a| Pod::text(a.as_str())).collect())),
            "Process.Env" => Some(env_value(&text_field(e, "name")?)),
            _ => None,
        }
    }
}

/// The CLI World.
pub struct CliWorld<F: FileSystem> {
    pub fs: Rc<RefCell<F>>,
    args: Rc<Vec<String>>,
    /// The document as the last tick left it.
    doc: Document,
    answers: Vec<(Path, Pod)>,
    pushes: Vec<(Pod, Pod)>,
    /// Wakes not yet due: when, the `Time.After` place, the `Wake`.
    timers: Vec<(Instant, Path, Pod)>,
    /// Each live `Fs.List` element and the listing it last heard.
    lists: BTreeMap<Pod, Pod>,
    prelude: Prelude,
    /// The last tick said another is owed: no waiting on a timer while compute is.
    pending: bool,
    exit: Option<u8>,
}

impl<F: FileSystem + 'static> CliWorld<F> {
    pub fn new(fs: F, args: Vec<String>) -> CliWorld<F> {
        CliWorld {
            fs: Rc::new(RefCell::new(fs)),
            args: Rc::new(args),
            doc: Document::new(),
            answers: Vec::new(),
            pushes: Vec::new(),
            timers: Vec::new(),
            lists: BTreeMap::new(),
            prelude: Prelude::new(),
            pending: false,
            exit: None,
        }
    }

    /// Write a file, and push its new contents to every `Fs.Contents` of its path.
    fn write(&mut self, path: &str, text: &str) -> Result<(), Error> {
        self.fs.borrow_mut().write(path, text)?;
        let watched = Pod::element("Fs.Contents", vec![("path", Pod::text(path))]);
        self.pushes.push((watched, contents(Ok(text.to_string()))));
        Ok(())
    }

    /// One request, in document order.
    fn request(&mut self, element: &Pod, path: &Path, value: &Pod) {
        let answer = match (element.tag(), value.structure()) {
            (Some("Prelude.Unwrap"), _) => return self.prelude.unwrap(path, value),
            (Some("Process.Exit"), _) => {
                if self.exit.is_none() {
                    self.exit = Some(u8::from_pod(value).unwrap_or(EXIT_FAILED));
                }
                return;
            }
            (Some("Time.After"), _) => {
                let after = match value.field("after").map(Pod::structure) {
                    Some(Pod::Duration(ns)) => Duration::from_nanos((*ns).clamp(0, u64::MAX as i128) as u64),
                    _ => Duration::ZERO,
                };
                return self.timers.push((Instant::now() + after, path.clone(), value.clone()));
            }
            (Some("Fs.File"), Pod::Choice(c, _)) if &**c == "Read" => {
                let Some(file) = text_field(element, "path") else { return };
                contents(self.fs.borrow().read(&file))
            }
            (Some("Fs.File"), Pod::Choice(c, args)) if &**c == "Write" => {
                let Some(file) = text_field(element, "path") else { return };
                let text = args.first().and_then(Pod::as_text).map(|t| t.to_string()).unwrap_or_default();
                contents(self.write(&file, &text).map(|()| text))
            }
            _ => return,
        };
        self.answers.push((path.clone(), answer));
    }

    /// An `Fs.Output` added or changed: write its text, and push what the write did.
    fn output(&mut self, element: &Pod) {
        let (Some(path), Some(text)) = (text_field(element, "path"), text_field(element, "text")) else { return };
        let state = match self.write(&path, &text) {
            Ok(()) => Pod::name("Written"),
            Err(e) => Pod::Choice("WriteFailed".into(), vec![e.to_pod()]),
        };
        self.pushes.push((element.clone(), state));
    }

    /// After a write, push each listing that moved.
    fn relist(&mut self) {
        let fs = self.fs.borrow();
        for (element, last) in self.lists.iter_mut() {
            let Some(dir) = text_field(element, "dir") else { continue };
            let now = listing(fs.list(&dir));
            if now != *last {
                *last = now.clone();
                self.pushes.push((element.clone(), now));
            }
        }
    }

    /// Move every wake that is due into the next tick's answers.
    fn due(&mut self) {
        let now = Instant::now();
        let (due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.timers).into_iter().partition(|(at, _, _)| *at <= now);
        self.timers = later;
        self.answers.extend(due.into_iter().map(|(_, p, v)| (p, v)));
    }
}

impl<F: FileSystem + 'static> World for CliWorld<F> {
    fn ready(&self) -> Rc<dyn ReadyState> {
        Rc::new(CliReady { fs: self.fs.clone(), args: self.args.clone() })
    }

    /// The requests first, in document order, torn-down places included; then the patch.
    fn perform(&mut self, out: &TickOut) {
        self.pending = out.pending;
        self.prelude.track(&out.patch);
        for r in &out.requests {
            if self.prelude.is_poisoned(&r.path) {
                continue;
            }
            let Some(element) = prelude::element_of(&self.doc, out, &r.path).cloned() else { continue };
            self.request(&element, &r.path, &r.value);
        }
        self.doc.apply(&out.patch);
        for (p, e) in &out.patch.removed {
            if e.tag() == Some("Fs.List") && !self.doc.iter().any(|(q, f)| q != p && f == e) {
                self.lists.remove(e);
            }
        }
        for (p, e) in &out.patch.added {
            if e.tag() == Some("Fs.List") && !self.prelude.is_poisoned(p) {
                let first = self.ready().first_value(e).unwrap_or(Pod::Array(vec![]));
                self.lists.entry(e.clone()).or_insert(first);
            }
        }
        let writes: Vec<Pod> = out
            .patch
            .added
            .iter()
            .chain(&out.patch.changed)
            .filter(|(p, e)| e.tag() == Some("Fs.Output") && !self.prelude.is_poisoned(p))
            .map(|(_, e)| e.clone())
            .collect();
        let wrote = !writes.is_empty() || !self.pushes.is_empty();
        writes.iter().for_each(|e| self.output(e));
        if wrote && !self.lists.is_empty() {
            self.relist();
        }
    }

    /// Whatever is waiting; with nothing waiting but a wake and no tick owed, sleep until
    /// the wake is due. Answers to places that left, or that a poisoning silenced, are
    /// dropped (Semantics ch. 9, "A late result is dropped").
    fn next_inputs(&mut self) -> Option<TickIn> {
        loop {
            self.due();
            self.answers.extend(self.prelude.take_answers());
            if self.answers.is_empty()
                && self.pushes.is_empty()
                && !self.pending
                && let Some(at) = self.timers.iter().map(|(at, _, _)| *at).min()
            {
                std::thread::sleep(at.saturating_duration_since(Instant::now()));
                self.due();
            }
            let mut input = TickIn { answers: std::mem::take(&mut self.answers), pushes: std::mem::take(&mut self.pushes) };
            self.prelude.filter(&mut input, &self.doc);
            if !input.answers.is_empty() || !input.pushes.is_empty() {
                return Some(input);
            }
            if self.timers.is_empty() || self.pending {
                return None;
            }
        }
    }

    fn exit_code(&self) -> Option<u8> {
        self.exit.or(self.prelude.exit())
    }
}
