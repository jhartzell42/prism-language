//! The document: places and paths, elements' two projections, the patch, requests in
//! path order, the ready state behind the first-value query, emit rows, and the scene
//! convention `Draw.Clickable` (decisions.md D9).
//!
//! Semantics ch. 8 states the rules this module keeps:
//!
//! - **A place is its path.** A place's identity is its path and nothing else, through any
//!   number of changes to its live fields ("A place is its path; a fact is its value").
//! - **A response is addressed by path; a state is keyed by the element's value.** An
//!   answer reaches the one place at its path; a push reaches every place whose element
//!   equals the fact's.
//! - **The patch is the document after `n` against the document after `n - 1`**: `Added`,
//!   `Changed` (live fields moved, same place) and `Removed`, never two entries for one
//!   place in one tick ("The patch").
//! - **Elements cross as two projections:** `Level`, plain fields and the current values
//!   of live ones, for the patch and pushes; `Edge`, the event components that fired, for
//!   requests and answers.
//! - **A new place's first value** comes from a first-value query asked while it is built,
//!   answered from the ready state as the tick opened with the tick's pushes on top ("A
//!   new place's first value"; Semantics ch. 9, "Queries are sync, from a ready state").

use std::collections::BTreeSet;
use std::fmt;
use std::rc::Rc;

use im_rc::OrdMap;

use crate::graph::Graph;
use crate::value::{Element, Fault, FieldKind, NodeId, Path, Value};

/// A document: each place's `Level` by path. The World's view of the element tree, a
/// dynamic `$(Path ==> Elem r)` read after each tick (ch. 8, "The element tree is a
/// dynamic").
pub type Document = OrdMap<Path, Value>;

/// A place in the root document, as the graph keeps it.
#[derive(Clone)]
pub(crate) struct PlaceData {
    pub path: Path,
    /// The element as emitted: live fields hold dynamics, request fields events.
    pub element: Rc<Element>,
    /// What the place answers its widget with: `unit`, an event, a dynamic or a plain
    /// value.
    pub output: Value,
    /// The request fields, by name, with their events.
    pub requests: Vec<(Rc<str>, NodeId)>,
    /// The dynamics of the live fields: the patch pass hears their marks.
    pub live: Vec<NodeId>,
    pub request_fields: usize,
}

impl PlaceData {
    pub fn values(&self) -> Vec<&Value> {
        let mut v: Vec<&Value> = self.element.fields.iter().map(|(_, v)| v).collect();
        v.push(&self.output);
        v
    }
}

/// A request that fired: the `Edge` of one request field of one place.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub path: Path,
    /// The place's qualified tag, `Fs.File`.
    pub tag: Rc<str>,
    pub field: Rc<str>,
    /// Whether the tag has more than one request field, so the transcript names it.
    pub several: bool,
    pub value: Value,
}

impl fmt::Display for Request {
    /// The transcript's `requested:` line: `path: value`, or `path: field=value` for a
    /// tag with several request fields (a spelling the book leaves open).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.several {
            write!(f, "{}: {}={}", self.path, self.field, self.value)
        } else {
            write!(f, "{}: {}", self.path, self.value)
        }
    }
}

/// The row a scope emits: which families its elements may come from (Semantics ch. 9,
/// "The sandbox law, as a theorem"). The checker enforces rows before tick 0; the runtime
/// keeps the third step of the law at the forwarding arm too, since compiler zero has no
/// checker (decisions.md D3): an element emitted into a scope whose row excludes its
/// family is a fault.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Row {
    /// `emit r` with `r` open: anything.
    #[default]
    Any,
    /// `emit Ui.* | Fs.*`: these families only.
    Families(BTreeSet<String>),
}

impl Row {
    pub fn of(families: &[&str]) -> Row {
        Row::Families(families.iter().map(|f| f.to_string()).collect())
    }
    pub fn allows(&self, family: &str) -> bool {
        match self {
            Row::Any => true,
            Row::Families(fs) => fs.contains(family),
        }
    }
}

impl fmt::Display for Row {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Row::Any => f.write_str("r"),
            Row::Families(fs) => {
                let parts: Vec<String> = fs.iter().map(|s| format!("{s}.*")).collect();
                f.write_str(&parts.join(" | "))
            }
        }
    }
}

/// The World's ready state: what it holds as the tick opens, plus any pure computation
/// over it (round 47 RS). The one question asked in the middle of a tick, a new place's
/// first value, is asked of it, and it never does work to answer (Semantics ch. 9).
pub trait ReadyState {
    /// The first value of a place whose element's `Level` is `element`, or `None` if the
    /// World has no answer for it.
    fn first_value(&self, element: &Value) -> Option<Value>;
}

/// A ready state with nothing in it.
pub struct NoReadyState;

impl ReadyState for NoReadyState {
    fn first_value(&self, _: &Value) -> Option<Value> {
        None
    }
}

/// A ready state written out as entries keyed by element value, as the test language's
/// `with` entries are.
#[derive(Clone, Default)]
pub struct ReadyMap(pub OrdMap<Value, Value>);

impl ReadyState for ReadyMap {
    fn first_value(&self, element: &Value) -> Option<Value> {
        self.0.get(element).cloned()
    }
}

/// What changed in the root document in one tick, each list in path order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Patch {
    /// `Added(path, element)`: a place appeared, with its `Level` after the tick.
    pub added: Vec<(Path, Value)>,
    /// `Changed(path, before, after)`: a place's live fields moved.
    pub changed: Vec<(Path, Value, Value)>,
    /// `Removed(path)`, with the element as it stood after the previous tick (doubt TD).
    pub removed: Vec<(Path, Value)>,
}

impl Patch {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.changed.is_empty() && self.removed.is_empty()
    }
}

/// An element's `Level` as of tick `asof`: its plain fields, the current values of its
/// live ones, and no events.
pub(crate) fn level(graph: &mut Graph, element: &Element, asof: crate::value::Tick) -> Result<Value, Fault> {
    let mut fields = Vec::new();
    for (name, v) in &element.fields {
        let kind = element.decl.as_ref().and_then(|d| d.field_kind(name)).unwrap_or(FieldKind::Identity);
        match kind {
            FieldKind::Request => {}
            FieldKind::Live => match v {
                Value::Dynamic(d) => fields.push((name.clone(), graph.read(*d, asof)?.0)),
                plain => fields.push((name.clone(), plain.clone())),
            },
            FieldKind::Identity => fields.push((name.clone(), v.clone())),
        }
    }
    Ok(Value::Element(Rc::new(Element { tag: element.tag.clone(), fields, decl: None })))
}

/// Every element inside a value, the value itself included: a check matches "at any
/// depth", by the GUI library's convention that a container holds its children in a
/// field (Conformance ch. 1, "Checks").
pub fn elements_within(v: &Value, out: &mut Vec<Value>) {
    match v {
        Value::Element(e) => {
            out.push(v.clone());
            for (_, f) in &e.fields {
                elements_within(f, out);
            }
        }
        Value::Record(r) => r.fields.iter().for_each(|(_, f)| elements_within(f, out)),
        Value::Tuple(t) => t.iter().for_each(|f| elements_within(f, out)),
        Value::Choice(c) => c.payload.iter().for_each(|f| elements_within(f, out)),
        Value::Map(m) => m.values().for_each(|f| elements_within(f, out)),
        Value::Array(a) => a.iter().for_each(|f| elements_within(f, out)),
        Value::Opt(Some(x)) => elements_within(x, out),
        Value::Fallible(Ok(x)) => elements_within(x, out),
        _ => {}
    }
}

/// `Draw.Clickable(rect, path)`: a scene item that says the region `rect` of a window is
/// the element at `path` (decisions.md D9). Hit testing stays in Prism; the scene carries
/// this so the Test World can turn "click the element at path" into a `Press` on the
/// window. The macOS World ignores it.
pub fn clickable(x: i64, y: i64, w: i64, h: i64, target: Path) -> Value {
    Value::element(Element::plain(
        "Draw.Clickable",
        vec![
            (
                "rect",
                Value::record(
                    "Rect",
                    vec![("x", Value::int(x)), ("y", Value::int(y)), ("w", Value::int(w)), ("h", Value::int(h))],
                ),
            ),
            ("path", Value::Path(target)),
        ],
    ))
}

/// Find, in a document, a `Draw.Clickable` whose path prints as `target`: the place that
/// drew it, and the centre of its rectangle.
pub fn find_clickable(doc: &Document, target: &str) -> Option<(Path, i64, i64)> {
    for (path, level) in doc.iter() {
        let mut found = Vec::new();
        elements_within(level, &mut found);
        for e in found {
            let Value::Element(e) = &e else { continue };
            if &*e.tag != "Draw.Clickable" {
                continue;
            }
            let (Some(Value::Path(p)), Some(rect)) = (e.field("path"), e.field("rect")) else { continue };
            if crate::pod::print_path(p) != target {
                continue;
            }
            let num = |n: &str| rect.field(n).and_then(Value::as_i128).unwrap_or(0) as i64;
            return Some((path.clone(), num("x") + num("w") / 2, num("y") + num("h") / 2));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Segment;

    #[test]
    fn rows_allow_their_families() {
        let r = Row::of(&["Ui", "Fs"]);
        assert!(r.allows("Ui") && !r.allows("Net"));
        assert!(Row::Any.allows("Net"));
        assert_eq!(r.to_string(), "Fs.* | Ui.*");
    }

    #[test]
    fn a_clickable_is_found_by_its_path() {
        let target = Path(vec![Segment::Pos(1), Segment::Gen(0), Segment::Pos(0)]);
        let scene = Value::array(vec![clickable(0, 0, 10, 20, target)]);
        let window = Value::element(Element::plain("Gui.Window", vec![("scene", scene)]));
        let mut doc = Document::new();
        doc.insert(Path(vec![Segment::Pos(0)]), window);
        assert_eq!(find_clickable(&doc, "1/#0/0"), Some((Path(vec![Segment::Pos(0)]), 5, 10)));
        assert_eq!(find_clickable(&doc, "1/#0/1"), None);
    }
}
