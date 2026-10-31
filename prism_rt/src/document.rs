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
use crate::value::{Element, Fault, FieldKind, NodeId, OutputKind, Path, Value};

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
    /// What the place answers its widget with: `unit`, an event, a dynamic, a plain
    /// value, or a bundle's tuple of those.
    pub output: Value,
    /// The tag's output kind, which says how to read `output`.
    pub kind: OutputKind,
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

    /// The output's components with their kinds: one for a plain kind, one per component
    /// for a bundle.
    fn components(&self) -> Vec<(OutputKind, Value)> {
        match (&self.kind, &self.output) {
            (OutputKind::Bundle(ks), Value::Tuple(items)) => ks.iter().cloned().zip(items.iter().cloned()).collect(),
            (k, v) => vec![(k.clone(), v.clone())],
        }
    }

    /// The input nodes an answer by path fires, in component order.
    pub fn events(&self) -> Vec<NodeId> {
        self.components()
            .into_iter()
            .filter_map(|(k, v)| match (k, v) {
                (OutputKind::Event, Value::Event(e)) => Some(e),
                _ => None,
            })
            .collect()
    }

    /// The external cells a push writes, in component order.
    pub fn dynamics(&self) -> Vec<NodeId> {
        self.components()
            .into_iter()
            .filter_map(|(k, v)| match (k, v) {
                (OutputKind::Dynamic, Value::Dynamic(d)) => Some(d),
                _ => None,
            })
            .collect()
    }

    /// What this place's first-value query was answered with, as it stands now: the
    /// plain and dynamic components, the value alone for one, a tuple for several (the
    /// sharing rule hands it to a newborn holding the same fact).
    pub(crate) fn first_parts(&self, graph: &mut Graph) -> Result<Value, Fault> {
        let mut parts = Vec::new();
        for (k, v) in self.components() {
            match (k, v) {
                (OutputKind::Dynamic, Value::Dynamic(d)) => parts.push(graph.read_now(d)?),
                (OutputKind::Plain, v) => parts.push(v),
                _ => {}
            }
        }
        Ok(if parts.len() == 1 { parts.pop().unwrap() } else { Value::tuple(parts) })
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
    let mut until = crate::graph::FOREVER;
    Ok(Value::Element(Rc::new(project_fields(graph, element, asof, &mut until)?)))
}

/// A value's `Level` as of `asof`, and the last tick it is known valid for: every element
/// inside it projected, at any depth (a container's `children: $(Path ==> Elem)` prints
/// its children as elements, Conformance ch. 1, "Checks"), the values of the live fields
/// read, the request fields dropped. Values with no element inside are returned as they
/// are.
pub(crate) fn project(graph: &mut Graph, v: &Value, asof: crate::value::Tick) -> Result<(Value, crate::value::Tick), Fault> {
    let mut until = crate::graph::FOREVER;
    let out = project_into(graph, v, asof, &mut until)?;
    Ok((out, until))
}

fn project_fields(graph: &mut Graph, element: &Element, asof: crate::value::Tick, until: &mut crate::value::Tick) -> Result<Element, Fault> {
    let mut fields = Vec::with_capacity(element.fields.len());
    for (name, v) in &element.fields {
        let kind = element.decl.as_ref().and_then(|d| d.field_kind(name)).unwrap_or(FieldKind::Identity);
        match kind {
            FieldKind::Request => {}
            FieldKind::Live => {
                let now = match v {
                    Value::Dynamic(d) => {
                        let (x, u) = graph.read(*d, asof)?;
                        *until = (*until).min(u);
                        x
                    }
                    plain => plain.clone(),
                };
                fields.push((name.clone(), project_into(graph, &now, asof, until)?));
            }
            FieldKind::Identity => fields.push((name.clone(), project_into(graph, v, asof, until)?)),
        }
    }
    Ok(Element { tag: element.tag.clone(), fields, decl: None })
}

fn project_into(graph: &mut Graph, v: &Value, asof: crate::value::Tick, until: &mut crate::value::Tick) -> Result<Value, Fault> {
    if !holds_element(v) {
        return Ok(v.clone());
    }
    Ok(match v {
        Value::Element(e) => Value::Element(Rc::new(project_fields(graph, e, asof, until)?)),
        Value::Record(r) => {
            let mut fs = Vec::with_capacity(r.fields.len());
            for (n, x) in &r.fields {
                fs.push((n.clone(), project_into(graph, x, asof, until)?));
            }
            Value::Record(Rc::new(crate::value::Record { name: r.name.clone(), fields: fs }))
        }
        Value::Tuple(t) => {
            let mut xs = Vec::with_capacity(t.len());
            for x in t.iter() {
                xs.push(project_into(graph, x, asof, until)?);
            }
            Value::tuple(xs)
        }
        Value::Choice(c) => {
            let mut xs = Vec::with_capacity(c.payload.len());
            for x in &c.payload {
                xs.push(project_into(graph, x, asof, until)?);
            }
            Value::Choice(Rc::new(crate::value::Choice { ctor: c.ctor.clone(), payload: xs }))
        }
        Value::Map(m) => {
            let mut out = OrdMap::new();
            for (k, x) in m.iter() {
                out.insert(k.clone(), project_into(graph, x, asof, until)?);
            }
            Value::Map(out)
        }
        Value::Array(a) => {
            let mut out = im_rc::Vector::new();
            for x in a.iter() {
                out.push_back(project_into(graph, x, asof, until)?);
            }
            Value::Array(out)
        }
        Value::Opt(Some(x)) => Value::some(project_into(graph, x, asof, until)?),
        Value::Fallible(Ok(x)) => Value::ok(project_into(graph, x, asof, until)?),
        other => other.clone(),
    })
}

/// Whether a value has an element emitted by a program (one with a declaration, whose
/// live fields may hold dynamics) anywhere inside it. A `Level` has none, so projecting a
/// projected value copies nothing.
fn holds_element(v: &Value) -> bool {
    match v {
        Value::Element(e) => e.decl.is_some() || e.fields.iter().any(|(_, f)| holds_element(f)),
        Value::Record(r) => r.fields.iter().any(|(_, f)| holds_element(f)),
        Value::Tuple(t) => t.iter().any(holds_element),
        Value::Choice(c) => c.payload.iter().any(holds_element),
        Value::Map(m) => m.values().any(holds_element),
        Value::Array(a) => a.iter().any(holds_element),
        Value::Opt(Some(x)) | Value::Fallible(Ok(x)) => holds_element(x),
        _ => false,
    }
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
/// the element at `path` of the program's own layout (decisions.md D9). Hit testing stays
/// in Prism; the scene carries this so the Test World can turn "click the element at
/// path" into a `Press` on the window. The macOS World ignores it.
pub fn clickable(x: f64, y: f64, w: f64, h: f64, target: Path) -> Value {
    Value::element(Element::plain(
        "Draw.Clickable",
        vec![("rect", crate::families::gui::rect(x, y, w, h)), ("path", Value::Path(target))],
    ))
}

/// A `Draw.Clickable` found in a scene: the `Gui.Scene` place that drew it, the window
/// the scene is for, and the centre of its rectangle in the window's coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct Clickable {
    pub scene: Path,
    pub window: Value,
    pub x: f64,
    pub y: f64,
}

/// Every `Draw.Clickable` whose path prints as `target` (`pod::print_path`, a program's
/// `^Show` for keys included), in the document's `Gui.Scene`s. A clickable inside a
/// `Draw.Group` has its rectangle in the group's frame, so the centre is taken out through
/// each enclosing group's turn, innermost first (README doubt CT: a turned card is clicked
/// where it is drawn); a `Draw.Clip` changes nothing about where its contents are.
pub fn find_clickables(doc: &Document, target: &str) -> Vec<Clickable> {
    let mut out = Vec::new();
    for (path, level) in doc.iter() {
        let Value::Element(scene) = level else { continue };
        if &*scene.tag != "Gui.Scene" {
            continue;
        }
        let window = scene.field("window").cloned().unwrap_or(Value::Unit);
        if let Some(draw) = scene.field("draw") {
            walk_draws(draw, target, &mut Vec::new(), &mut |x, y| {
                out.push(Clickable { scene: path.clone(), window: window.clone(), x, y })
            });
        }
    }
    out
}

fn number(v: Option<&Value>) -> f64 {
    match v {
        Some(Value::Float(f)) => *f,
        Some(other) => other.as_i128().map(|n| n as f64).unwrap_or(0.0),
        None => 0.0,
    }
}

fn walk_draws(v: &Value, target: &str, turns: &mut Vec<Value>, hit: &mut dyn FnMut(f64, f64)) {
    match v {
        Value::Array(items) => items.iter().for_each(|d| walk_draws(d, target, turns, hit)),
        Value::Element(e) => match &*e.tag {
            "Draw.Clickable" => {
                let (Some(Value::Path(p)), Some(rect)) = (e.field("path"), e.field("rect")) else { return };
                if crate::pod::print_path(p) != target {
                    return;
                }
                let mut x = number(rect.field("x")) + number(rect.field("w")) / 2.0;
                let mut y = number(rect.field("y")) + number(rect.field("h")) / 2.0;
                for t in turns.iter().rev() {
                    (x, y) = turned(t, x, y);
                }
                hit(x, y);
            }
            "Draw.Group" => {
                let Some(draws) = e.field("draws") else { return };
                turns.push(e.field("turn").cloned().unwrap_or(Value::Unit));
                walk_draws(draws, target, turns, hit);
                turns.pop();
            }
            "Draw.Clip" => {
                if let Some(draws) = e.field("draws") {
                    walk_draws(draws, target, turns, hit);
                }
            }
            _ => {}
        },
        _ => {}
    }
}

/// A point in a group's frame, in its parent's: turned clockwise on the screen by
/// `angle` degrees about the pivot `(x, y)`, then scaled about it; the inverse of the
/// window library's `Untransform`.
fn turned(t: &Value, px: f64, py: f64) -> (f64, f64) {
    let (cx, cy) = (number(t.field("x")), number(t.field("y")));
    let angle = number(t.field("angle"));
    let scale = t.field("scale").map(|s| number(Some(s))).unwrap_or(1.0);
    let rad = angle.to_radians();
    let (ux, uy) = (px - cx, py - cy);
    (cx + scale * (ux * rad.cos() - uy * rad.sin()), cy + scale * (ux * rad.sin() + uy * rad.cos()))
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
    fn a_clickable_is_found_by_its_path_through_a_turn() {
        let target = Path(vec![Segment::Pos(1), Segment::Gen(0), Segment::Pos(0)]);
        let flat = clickable(0.0, 0.0, 10.0, 20.0, target.clone());
        let turn = Value::record(
            "Turn",
            vec![("x", Value::float(0.0)), ("y", Value::float(0.0)), ("angle", Value::float(90.0)), ("scale", Value::float(2.0))],
        );
        let group = Value::element(Element::plain("Draw.Group", vec![("turn", turn), ("draws", Value::array(vec![flat.clone()]))]));
        let id = crate::families::gui::window_id("w");
        let scene = |d: Value| Value::element(Element::plain("Gui.Scene", vec![("window", id.clone()), ("draw", Value::array(vec![d]))]));
        let mut doc = Document::new();
        doc.insert(Path(vec![Segment::Pos(3)]), scene(flat));
        let found = find_clickables(&doc, "1/#0/0");
        assert_eq!(found, vec![Clickable { scene: Path(vec![Segment::Pos(3)]), window: id.clone(), x: 5.0, y: 10.0 }]);
        assert!(find_clickables(&doc, "1/#0/1").is_empty());
        doc.insert(Path(vec![Segment::Pos(3)]), scene(group));
        let c = &find_clickables(&doc, "1/#0/0")[0];
        assert!((c.x - -20.0).abs() < 1e-9 && (c.y - 10.0).abs() < 1e-9, "{c:?}");
    }
}
