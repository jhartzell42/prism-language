//! The prelude's two tags, answered by whatever World a program runs under (Semantics
//! ch. 10, "Exceptions: a widget giving up"):
//!
//! - **`Prelude.Unwrap(value=!~a)`** answers each `~ x` with `x`, in the next tick. The
//!   first `Error` poisons the innermost subtree with a path of its own that holds the
//!   place: its longest path prefix ending in a `rebuild` generation or a `for` key, or
//!   the whole program at the root. Nothing there is answered again, and no request
//!   from there is performed.
//! - **`Prelude.Exceptions`** hears the report, an `Exception`, a tick later: the nearest
//!   `Prelude.Exceptions` place outside the poisoned subtree whose parent holds it.
//! - **With no supervisor the root is poisoned,** and the CLI World's policy is to print
//!   the exception on standard error and exit 1 (Semantics ch. 9, "The CLI World's
//!   policy"; doubt UH). The canvas World follows the same policy.
//!
//! [`Prelude`] is the policy alone, which the CLI World runs inline; [`UnwrapWorld`] puts
//! it around a World that answers no prelude tag itself, as the canvas World does.

use std::collections::BTreeSet;
use std::rc::Rc;

use prism_rtl::world::EXIT_FAILED;
use prism_rtl::{Document, Fault, Patch, Path, Pod, ReadyState, Request, Segment, TickIn, TickOut, World};

/// The `Prelude.Unwrap` policy's state.
#[derive(Default)]
pub struct Prelude {
    poisoned: Vec<Path>,
    /// The live `Prelude.Exceptions` places.
    exceptions: BTreeSet<Path>,
    answers: Vec<(Path, Pod)>,
    exit: Option<u8>,
}

/// The innermost subtree with a path of its own holding `p`.
pub fn subtree_of(p: &Path) -> Path {
    let end = p.segments().iter().rposition(|s| matches!(s, Segment::Gen(_) | Segment::Key(_))).map_or(0, |i| i + 1);
    Path(p.segments()[..end].to_vec())
}

fn under(p: &Path, q: &Path) -> bool {
    p.segments().starts_with(q.segments())
}

/// The place a request came from: the document as the tick opened, which still holds a
/// place torn down in the tick; else the tick's newborns.
pub fn element_of<'a>(doc: &'a Document, out: &'a TickOut, path: &Path) -> Option<&'a Pod> {
    doc.get(path).or_else(|| out.patch.added.iter().find(|(p, _)| p == path).map(|(_, e)| e))
}

impl Prelude {
    pub fn new() -> Prelude {
        Prelude::default()
    }

    /// Whether `p` is in a poisoned subtree: the World serves it nothing.
    pub fn is_poisoned(&self, p: &Path) -> bool {
        self.poisoned.iter().any(|q| under(p, q))
    }

    /// Keep the `Prelude.Exceptions` places up to date from a tick's patch. Call it before
    /// [`Prelude::unwrap`], so a supervisor built in the tick hears a report from it.
    pub fn track(&mut self, patch: &Patch) {
        for (p, _) in &patch.removed {
            self.exceptions.remove(p);
        }
        for (p, e) in &patch.added {
            if e.tag() == Some("Prelude.Exceptions") {
                self.exceptions.insert(p.clone());
            }
        }
    }

    /// A `Prelude.Unwrap` place at `path` fired `value`, a `~ x`.
    pub fn unwrap(&mut self, path: &Path, value: &Pod) {
        let error = match value.structure() {
            Pod::Ok(x) => return self.answers.push((path.clone(), (**x).clone())),
            Pod::Failed(e) => e,
            // Not a `~` value: a template's mistake, answered as it came.
            other => return self.answers.push((path.clone(), other.clone())),
        };
        let sub = subtree_of(path);
        let kind = error.field("kind").cloned().unwrap_or(Pod::name("Prelude.Unknown"));
        let message = error.field("message").cloned().unwrap_or(Pod::text(""));
        let exception = Pod::record(
            "Exception",
            vec![("kind", kind.clone()), ("message", message.clone()), ("spans", Pod::Array(vec![])), ("path", Pod::Some(Box::new(Pod::Path(sub.clone()))))],
        );
        // The supervisor: outside the subtree, and its parent holds the subtree.
        let supervisor = self
            .exceptions
            .iter()
            .filter(|p| !under(p, &sub) && under(&sub, &Path(p.segments()[..p.len().saturating_sub(1)].to_vec())))
            .max_by_key(|p| p.len())
            .cloned();
        self.poisoned.push(sub.clone());
        match supervisor {
            Some(s) if !sub.is_empty() => self.answers.push((s, exception)),
            _ => {
                let message = message.as_text().map(|t| t.to_string()).unwrap_or_default();
                eprintln!("prism: the program gave up: {kind}: {message}");
                self.exit.get_or_insert(EXIT_FAILED);
            }
        }
    }

    /// The answers this policy owes the next tick.
    pub fn take_answers(&mut self) -> Vec<(Path, Pod)> {
        std::mem::take(&mut self.answers)
    }

    /// Drop from a tick's inputs the answers to poisoned places, and to places no longer
    /// in `doc`, the document as the tick opens: a late result goes nowhere (Semantics
    /// ch. 9).
    pub fn filter(&self, input: &mut TickIn, doc: &Document) {
        input.answers.retain(|(p, _)| !self.is_poisoned(p) && doc.get(p).is_some());
    }

    /// 1 once the root is poisoned.
    pub fn exit(&self) -> Option<u8> {
        self.exit
    }
}

/// A World with the prelude's tags answered around it: it keeps the document to know
/// each request's tag, answers `Prelude.Unwrap`, and hands the inner World the rest of
/// the requests, less those from poisoned subtrees.
pub struct UnwrapWorld<W: World> {
    pub inner: W,
    doc: Document,
    prelude: Prelude,
}

impl<W: World> UnwrapWorld<W> {
    pub fn new(inner: W) -> UnwrapWorld<W> {
        UnwrapWorld { inner, doc: Document::new(), prelude: Prelude::new() }
    }
}

impl<W: World> World for UnwrapWorld<W> {
    fn ready(&self) -> Rc<dyn ReadyState> {
        self.inner.ready()
    }

    fn perform(&mut self, out: &TickOut) {
        self.prelude.track(&out.patch);
        let mut theirs: Vec<Request> = Vec::new();
        for r in &out.requests {
            if self.prelude.is_poisoned(&r.path) {
                continue;
            }
            match element_of(&self.doc, out, &r.path).and_then(Pod::tag) {
                Some("Prelude.Unwrap") => self.prelude.unwrap(&r.path, &r.value),
                _ => theirs.push(r.clone()),
            }
        }
        self.doc.apply(&out.patch);
        if theirs.len() == out.requests.len() {
            self.inner.perform(out);
        } else {
            self.inner.perform(&TickOut { requests: theirs, ..out.clone() });
        }
    }

    fn next_inputs(&mut self) -> Option<TickIn> {
        let mine = self.prelude.take_answers();
        let mut input = match (self.inner.next_inputs(), mine.is_empty()) {
            (Some(i), _) => i,
            (None, true) => return None,
            (None, false) => TickIn::none(),
        };
        input.answers.extend(mine);
        self.prelude.filter(&mut input, &self.doc);
        Some(input)
    }

    fn exit_code(&self) -> Option<u8> {
        self.prelude.exit().or(self.inner.exit_code())
    }

    fn failed(&mut self, fault: &Fault) -> u8 {
        self.inner.failed(fault)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(segs: Vec<Segment>) -> Path {
        Path(segs)
    }

    #[test]
    fn the_subtree_is_the_last_generation_or_key() {
        let p = path(vec![Segment::Pos(3), Segment::Gen(0), Segment::Pos(2)]);
        assert_eq!(subtree_of(&p), path(vec![Segment::Pos(3), Segment::Gen(0)]));
        assert_eq!(subtree_of(&path(vec![Segment::Pos(1)])), Path::root());
    }

    #[test]
    fn a_supervisor_hears_the_report_and_the_subtree_goes_quiet() {
        let mut pre = Prelude::new();
        let sup = path(vec![Segment::Pos(0)]);
        let unwrap = path(vec![Segment::Pos(3), Segment::Gen(0), Segment::Pos(2)]);
        pre.track(&Patch { added: vec![(sup.clone(), Pod::element("Prelude.Exceptions", vec![]))], ..Patch::default() });
        pre.unwrap(&unwrap, &Pod::Ok(Box::new(Pod::int(5))));
        let failed = Pod::Failed(Box::new(prism_rtl::ToPod::to_pod(&prism_rtl::Error::new("Fs.NotFound", "gone"))));
        pre.unwrap(&unwrap, &failed);
        let answers = pre.take_answers();
        assert_eq!(answers[0], (unwrap.clone(), Pod::int(5)));
        assert_eq!(answers[1].0, sup);
        assert_eq!(answers[1].1.field("message"), Some(&Pod::text("gone")));
        assert!(pre.is_poisoned(&unwrap) && !pre.is_poisoned(&sup));
        assert_eq!(pre.exit(), None);
    }
}
