//! The tick: `tick: ?State, inputs -> State, outputs, pending` (architecture.md, "The
//! tick"; Semantics ch. 7, "The tick as a pure function").
//!
//! `tick` on no state is the build, tick 0. Every later tick runs in Graph ch. 4's order
//! ("A tick, in order"):
//!
//! 1. **Inputs come in.** Answers reach their places by path, pushes reach every place
//!    whose element equals theirs, the started list fires, flattens that adopted last
//!    tick wake, and every armed `@Defer` fires: each pending job advances one cycle.
//! 2. **Heights run in order** (R4, R5).
//! 3. **Propagation ends**: the request list is complete and sorted into document order
//!    (R9); every read from here on is as of this tick.
//! 4. **The builders drop what left**, freeing its paths.
//! 5. **The build list**: newborns are built from the values after the tick (R7).
//! 6. **The swap list**: switches and flattens adopt their new inners.
//! 7. **The detached widgets leave**: holds become constants, own events `never` (R8).
//! 8. **The patch pass**: the document after the tick against the document before.
//! 9. **The outputs leave**, with `pending`: a newborn owes a tick, or a `@Defer` is
//!    armed (doubt PD: one flag).
//!
//! A tick that faults never happened (Semantics ch. 9, round 47 CP): `tick` hands back
//! the state it was given, and the caller drops or re-queues the inputs by its own policy.
//! The state is persistent (the arena is an `im_rc` vector), so keeping it costs nothing
//! and the same state may be ticked twice (Semantics ch. 7, "Same arguments, same
//! results").

use std::rc::Rc;

use im_rc::OrdMap;

use crate::document::{level, Document, NoReadyState, Patch, ReadyState, Request};
use crate::graph::{Graph, Kind};
use crate::value::{Blueprint, Fault, NodeId, Path, Tick, Value, WidgetId};
use crate::widget::{self, construct, Scope, Widgets};

/// A program: its root blueprint, `@main`.
#[derive(Clone)]
pub struct Program {
    pub name: String,
    pub root: Blueprint,
}

impl Program {
    pub fn new(name: &str, root: Blueprint) -> Program {
        Program { name: name.to_string(), root }
    }
}

/// What a World hands a tick (Semantics ch. 7, "The inputs").
#[derive(Clone)]
pub struct Inputs {
    /// Answers to places, by path: `0: click`. Inputs are addressed to places that
    /// existed as the tick opened; an answer for a place that left is dropped.
    pub answers: Vec<(Path, Value)>,
    /// Pushes: new values of facts, keyed by element `Level`: `Fs.Contents(path="a") =
    /// ~ Contents(text="b")`.
    pub pushes: Vec<(Value, Value)>,
    /// The ready state the first-value query is answered from.
    pub ready: Rc<dyn ReadyState>,
}

impl Inputs {
    /// No inputs: a bare `tick`, or a `compute`.
    pub fn none() -> Inputs {
        Inputs { answers: vec![], pushes: vec![], ready: Rc::new(NoReadyState) }
    }
    pub fn with_ready(ready: Rc<dyn ReadyState>) -> Inputs {
        Inputs { answers: vec![], pushes: vec![], ready }
    }
    pub fn answer(mut self, path: Path, v: Value) -> Inputs {
        self.answers.push((path, v));
        self
    }
    pub fn push(mut self, element: Value, v: Value) -> Inputs {
        self.pushes.push((element, v));
        self
    }
}

impl Default for Inputs {
    fn default() -> Inputs {
        Inputs::none()
    }
}

/// What the root's result says this tick, for the transcript's `result:` line.
#[derive(Clone, Debug, PartialEq)]
pub enum ResultReport {
    /// A plain result at tick 0, or a dynamic one at tick 0 and whenever it changes.
    Value(Value),
    /// An event result that fired: `fired: value`.
    Fired(Value),
}

/// What a tick hands back to its World (Semantics ch. 7, "The outputs").
#[derive(Clone, Debug, Default)]
pub struct Outputs {
    pub tick: Tick,
    pub patch: Patch,
    /// Requests that fired, in document order, torn-down places included.
    pub requests: Vec<Request>,
    /// The codes of `Process.Exit` requests among them, in document order: the CLI
    /// World's convenience (Semantics ch. 9, "The CLI World's policy").
    pub exits: Vec<u8>,
    pub result: Option<ResultReport>,
}

/// The program's state between ticks: the widget tree with each widget's holds and
/// bindings, the started list, and the document as the World last saw it. Cloning is
/// constant time.
#[derive(Clone, Default)]
pub struct State {
    pub(crate) graph: Graph,
    pub(crate) widgets: Widgets,
    pub(crate) root: Option<WidgetId>,
    pub(crate) result: Value,
    pub(crate) last_result: Option<Value>,
    /// The root document after the last tick: each place's `Level`.
    pub(crate) document: Document,
    /// The root document's places, by path.
    pub(crate) places: OrdMap<Path, NodeId>,
    /// The started list: newborns' `@started`s, which fire next tick.
    pub(crate) started: Vec<NodeId>,
    /// Flattens that adopted at the end of the last tick.
    pub(crate) adoptions: Vec<NodeId>,
    /// `@Defer`s armed for the next tick.
    pub(crate) armed: Vec<NodeId>,
    // Lists of one tick's end, empty between ticks.
    pub(crate) started_next: Vec<NodeId>,
    pub(crate) adoptions_next: Vec<NodeId>,
    pub(crate) built: Vec<NodeId>,
    pub(crate) removed: Vec<Path>,
    pub(crate) leaving: Vec<WidgetId>,
}

impl State {
    /// The last tick this state ran.
    pub fn tick_number(&self) -> Tick {
        self.graph.now
    }
    /// The root document, as the World last saw it.
    pub fn document(&self) -> &Document {
        &self.document
    }
    /// The debug allocator's count: live cells in the node and widget arenas (D5).
    pub fn live_cells(&self) -> usize {
        self.graph.live_nodes() + self.widgets.live_cells()
    }
    /// The live node cells alone.
    pub fn live_nodes(&self) -> usize {
        self.graph.live_nodes()
    }
    /// Whether the program wants another tick (doubt PD): something on the started list
    /// has a subscriber, a flatten that adopted is read, or a `@Defer` is armed.
    pub fn pending(&self) -> bool {
        let g = &self.graph;
        self.started.iter().any(|s| g.exists(*s) && !g.node(*s).subscribers.is_empty())
            || self.adoptions.iter().any(|f| g.exists(*f) && (!g.node(*f).subscribers.is_empty() || !g.node(*f).readers.is_empty()))
            || !self.armed.is_empty()
    }
}

/// What `tick` hands back. On a failed tick, `state` is the state that was given and
/// `outputs` the fault.
pub struct Ticked {
    pub state: Option<State>,
    pub outputs: Result<Outputs, Fault>,
    pub pending: bool,
}

/// `tick: ?State, inputs -> State, outputs, pending`.
pub fn tick(program: &Program, state: Option<State>, inputs: &Inputs) -> Ticked {
    match state {
        None => match build(program, inputs) {
            Ok((st, out)) => {
                let pending = st.pending();
                Ticked { state: Some(st), outputs: Ok(out), pending }
            }
            Err(f) => Ticked { state: None, outputs: Err(f), pending: false },
        },
        Some(previous) => {
            let mut st = previous.clone();
            match run(&mut st, inputs) {
                Ok(out) => {
                    let pending = st.pending();
                    Ticked { state: Some(st), outputs: Ok(out), pending }
                }
                Err(f) => {
                    let pending = previous.pending();
                    Ticked { state: Some(previous), outputs: Err(f), pending }
                }
            }
        }
    }
}

/// Tick 0: build the root, from the values after the build; nothing fires.
fn build(program: &Program, inputs: &Inputs) -> Result<(State, Outputs), Fault> {
    let mut st = State::default();
    st.graph.now = 0;
    let root = program.root.clone();
    let (w, result) = construct(&mut st, inputs, None, Path::root(), Scope::Root, Default::default(), |b| {
        root.build(b, &[])
    })?;
    st.root = Some(w);
    if let Value::Event(id) | Value::Dynamic(id) = &result {
        st.graph.pin(*id);
    }
    st.result = result;
    // Construction is one simultaneous definition (doubt NB): marks it raised among
    // newborns change nothing that is not already built from values after the tick.
    st.graph.lists = Default::default();
    let out = finish(&mut st, Vec::new())?;
    Ok((st, out))
}

/// Ticks 1 and on.
fn run(st: &mut State, inputs: &Inputs) -> Result<Outputs, Fault> {
    let n = st.graph.now + 1;
    st.graph.now = n;
    st.graph.lists = Default::default();

    // 1. Inputs come in.
    let mut answers = inputs.answers.clone();
    answers.sort_by(|a, b| a.0.cmp(&b.0));
    for (path, v) in answers {
        if let Some(place) = st.places.get(&path).copied() {
            deliver(st, place, v)?;
        }
    }
    for (fact, v) in &inputs.pushes {
        push(st, fact, v)?;
    }
    for s in std::mem::take(&mut st.started) {
        if st.graph.exists(s) && matches!(st.graph.kind(s), Kind::Started) {
            st.graph.fire(s, Value::Unit);
        }
    }
    for f in std::mem::take(&mut st.adoptions) {
        if st.graph.exists(f) {
            if let Kind::Flatten { adopting, .. } = st.graph.kind_mut(f) {
                *adopting = true;
                st.graph.wake(f);
            }
        }
    }
    for d in std::mem::take(&mut st.armed) {
        if !st.graph.exists(d) {
            continue;
        }
        if let Kind::DeferOut { armed, armed_for } = st.graph.kind_mut(d) {
            if *armed_for == n {
                if let Some(v) = armed.take() {
                    st.graph.fire(d, v);
                }
            }
        }
    }

    // 2. Heights run in order.
    st.graph.propagate()?;

    // 3-9.
    let requests = std::mem::take(&mut st.graph.lists.requests);
    finish_tick(st, inputs, requests)
}

/// Deliver an answer to a place: an event output fires; a dynamic one is written.
fn deliver(st: &mut State, place: NodeId, v: Value) -> Result<(), Fault> {
    let Kind::Place(data) = st.graph.kind(place) else { return Ok(()) };
    match data.output.clone() {
        Value::Event(e) if matches!(st.graph.kind(e), Kind::Input) => {
            if st.graph.fired(e).is_none() {
                st.graph.fire(e, v);
            }
        }
        Value::Dynamic(d) if matches!(st.graph.kind(d), Kind::External { .. }) => write_external(st, d, v)?,
        _ => {}
    }
    Ok(())
}

/// A push reaches every place whose element equals the fact, as the tick opened
/// (Semantics ch. 9, "Pushes are keyed by value").
fn push(st: &mut State, fact: &Value, v: &Value) -> Result<(), Fault> {
    let before = st.graph.now - 1;
    for place in st.places.values().copied().collect::<Vec<_>>() {
        let Kind::Place(data) = st.graph.kind(place) else { continue };
        let (Value::Dynamic(d), element) = (data.output.clone(), data.element.clone()) else { continue };
        if level(&mut st.graph, &element, before)? == *fact {
            write_external(st, d, v.clone())?;
        }
    }
    Ok(())
}

fn write_external(st: &mut State, d: NodeId, v: Value) -> Result<(), Fault> {
    let before = st.graph.now - 1;
    if st.graph.fired(d).is_none() && st.graph.read(d, before)?.0 != v {
        st.graph.write_cell(d, v.clone());
        st.graph.fire(d, v);
    }
    Ok(())
}

/// Steps 3 to 9.
fn finish_tick(st: &mut State, inputs: &Inputs, mut requests: Vec<Request>) -> Result<Outputs, Fault> {
    // 3. Requests in document order (R9). The sort is stable, so one place's request
    // fields keep their order.
    requests.sort_by(|a, b| a.path.cmp(&b.path));

    // 4 and 5. The builders: drops in path order, outer before inner, then builds. A
    // builder marked by construction itself (a `for` over a `rebuild`'s result) is taken
    // in a further round.
    loop {
        let marked: Vec<NodeId> = std::mem::take(&mut st.graph.lists.builders).into_iter().collect();
        if marked.is_empty() {
            break;
        }
        let mut ordered: Vec<(Path, NodeId)> = marked
            .into_iter()
            .filter(|b| st.graph.exists(*b))
            .filter_map(|b| widget::builder_path(st, b).map(|(p, _)| (p, b)))
            .collect();
        ordered.sort();
        let mut plans = Vec::new();
        for (_, b) in &ordered {
            if owner_live(st, *b) {
                if let Some(plan) = widget::drop_phase(st, *b)? {
                    plans.push((*b, plan));
                }
            }
        }
        for (b, plan) in plans {
            if owner_live(st, b) {
                widget::build_phase(st, inputs, b, plan)?;
            }
        }
    }

    // 6. The swap list.
    let swaps: Vec<NodeId> = std::mem::take(&mut st.graph.lists.swaps).into_iter().collect();
    for s in swaps {
        if st.graph.exists(s) && owner_live(st, s) {
            widget::swap(st, s)?;
        }
    }

    // 7. The detached widgets leave.
    for w in std::mem::take(&mut st.leaving) {
        widget::leave(st, w);
    }

    finish(st, requests)
}

fn owner_live(st: &State, node: NodeId) -> bool {
    st.graph.node(node).owner.is_some_and(|w| st.widgets.is_live(w))
}

/// Steps 8 and 9, shared with the build.
fn finish(st: &mut State, requests: Vec<Request>) -> Result<Outputs, Fault> {
    let n = st.graph.now;

    // The armed `@Defer`s, for the next tick.
    st.armed = st
        .graph
        .ids()
        .filter(|d| matches!(st.graph.kind(*d), Kind::DeferOut { armed: Some(_), armed_for } if *armed_for == n + 1))
        .collect();

    // 8. The patch pass.
    let patch = patch_pass(st)?;

    // 9. The outputs.
    let exits = requests
        .iter()
        .filter(|r| &*r.tag == "Process.Exit")
        .filter_map(|r| r.value.as_i128().and_then(|c| u8::try_from(c).ok()))
        .collect();
    let result = match st.result.clone() {
        Value::Unit => None,
        Value::Event(e) => st.graph.fired(e).map(ResultReport::Fired),
        Value::Dynamic(d) => {
            let v = st.graph.read_now(d)?;
            if st.last_result.as_ref() != Some(&v) {
                st.last_result = Some(v.clone());
                Some(ResultReport::Value(v))
            } else {
                None
            }
        }
        plain if n == 0 => Some(ResultReport::Value(plain)),
        _ => None,
    };
    st.started = std::mem::take(&mut st.started_next);
    st.adoptions = std::mem::take(&mut st.adoptions_next);
    st.graph.lists = Default::default();
    Ok(Outputs { tick: n, patch, requests, exits, result })
}

/// The patch pass (Graph ch. 4, "The patch pass and the request list"): `Removed` for
/// each place that left, with its element as it stood after the previous tick; `Added`
/// for each newborn place, with its `Level` after the tick; `Changed` for each place
/// whose live field was marked and whose `Level` moved.
fn patch_pass(st: &mut State) -> Result<Patch, Fault> {
    let n = st.graph.now;
    let mut patch = Patch::default();
    let mut removed = std::mem::take(&mut st.removed);
    removed.sort();
    for path in removed {
        if let Some(before) = st.document.remove(&path) {
            patch.removed.push((path, before));
        }
    }
    let built = std::mem::take(&mut st.built);
    for place in &built {
        if !st.graph.exists(*place) {
            continue;
        }
        let Kind::Place(data) = st.graph.kind(*place) else { continue };
        let (path, element) = (data.path.clone(), data.element.clone());
        let lv = level(&mut st.graph, &element, n)?;
        st.document.insert(path.clone(), lv.clone());
        patch.added.push((path, lv));
    }
    let dirty: Vec<NodeId> = std::mem::take(&mut st.graph.lists.dirty).into_iter().collect();
    for place in dirty {
        if built.contains(&place) || !st.graph.exists(place) {
            continue;
        }
        let Kind::Place(data) = st.graph.kind(place) else { continue };
        let (path, element) = (data.path.clone(), data.element.clone());
        let after = level(&mut st.graph, &element, n)?;
        if let Some(before) = st.document.get(&path).cloned() {
            if before != after {
                st.document.insert(path.clone(), after.clone());
                patch.changed.push((path, before, after));
            }
        }
    }
    patch.added.sort_by(|a, b| a.0.cmp(&b.0));
    patch.changed.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(patch)
}
