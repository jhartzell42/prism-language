//! Widgets: construction through [`Build`], the builders (`rebuild`, keyed `for`, host
//! documents), and leaving the tree.
//!
//! An interpreter (or generated code) builds a widget by running a [`Blueprint`]'s
//! function against a [`Build`], which is the widget block's statements as methods:
//! `emit` for an element statement, `hold`/`hold_on` for `then`, `rebuild`, `for_each`,
//! `switch`, `flatten`, `started`, `defer`, `run`, `host_document`, and the pure
//! combinators (`map`, `sample`, `lift`, `merge`, ...) that lifting turns expressions
//! into. The rules each one keeps are cited on it.
//!
//! Construction happens only at the end of a tick, after propagation, from the values
//! after the tick (R7; Semantics ch. 7, "Construction is not prompt"), so every read a
//! `Build` makes is as of the current tick, and nothing it makes fires in it.

use std::rc::Rc;

use im_rc::{OrdMap, Vector};

use crate::document::{level, PlaceData, Row};
use crate::graph::{Cache2, Cell2, Kind};
use crate::tick::{Inputs, State};
use crate::value::{
    Blueprint, Closure, Element, Fault, NodeId, OutputKind, Path, Segment, Tick, Value, WidgetId,
};

/// Which document a widget's elements go to: the root document the World reads, or the
/// child document of a host (Semantics ch. 9, `@HostDocument`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scope {
    Root,
    /// The child document of the host node.
    Hosted(NodeId),
}

/// What a `rebuild`'s block or a `for`'s rows end on, which decides what the builder
/// hands back (Semantics ch. 4, the tables under "What a `rebuild` hands back" and
/// `for`). Compiler zero has no checker (D3), so the interpreter says which, from the
/// binder's sigil.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResultKind {
    /// `unit`: the builder stands bare and hands back `unit` (doubt UN).
    Unit,
    /// `!a`: a `rebuild` gives `$!a`; a `for` gives `!(K ==> a)`.
    Event,
    /// `$a`: a `rebuild` gives `$$a`; a `for` gives `$(K ==> a)`.
    Dynamic,
    /// A plain `a`: a `rebuild` gives `$a`; a `for` over a dynamic gives `$(K ==> a)`.
    Plain,
}

/// A widget instance.
#[derive(Clone)]
pub(crate) struct Widget {
    pub parent: Option<WidgetId>,
    /// Every node its construction made: its holds, bindings, places and combinators.
    pub nodes: Vec<NodeId>,
    /// Generations, rows, a host's child root and supplier instances.
    pub children: Vec<WidgetId>,
    /// Its places in the root document.
    pub places: Vec<NodeId>,
    /// Its places in host documents: the host and the child path.
    pub hosted: Vec<(NodeId, Path)>,
    pub live: bool,
}

/// The widget arena.
#[derive(Clone, Default)]
pub(crate) struct Widgets {
    cells: Vector<Option<Widget>>,
    free: Vector<u32>,
}

impl Widgets {
    pub fn get(&self, w: WidgetId) -> &Widget {
        self.cells[w.0 as usize].as_ref().expect("a freed widget was used")
    }
    pub fn get_mut(&mut self, w: WidgetId) -> &mut Widget {
        self.cells.get_mut(w.0 as usize).and_then(|c| c.as_mut()).expect("a freed widget was used")
    }
    pub fn is_live(&self, w: WidgetId) -> bool {
        self.cells.get(w.0 as usize).is_some_and(|c| c.as_ref().is_some_and(|w| w.live))
    }
    fn alloc(&mut self, w: Widget) -> WidgetId {
        match self.free.pop_back() {
            Some(i) => {
                self.cells.set(i as usize, Some(w));
                WidgetId(i)
            }
            None => {
                self.cells.push_back(Some(w));
                WidgetId(self.cells.len() as u32 - 1)
            }
        }
    }
    pub fn free(&mut self, w: WidgetId) {
        self.cells.set(w.0 as usize, None);
        self.free.push_back(w.0);
    }
    pub fn live_cells(&self) -> usize {
        self.cells.len() - self.free.len()
    }
    pub fn ids(&self) -> impl Iterator<Item = WidgetId> + '_ {
        self.cells.iter().enumerate().filter(|(_, c)| c.is_some()).map(|(i, _)| WidgetId(i as u32))
    }
}

// ---------------------------------------------------------------------------------------
// Builders
// ---------------------------------------------------------------------------------------

/// A `rebuild $x:` (Semantics ch. 4): builds its block for the current value of `$x`,
/// and anew whenever `$x` changes, at a new generation `#g`.
#[derive(Clone)]
pub(crate) struct RebuildData {
    pub owner: WidgetId,
    pub src: NodeId,
    /// The block, a [`Value::Widget`].
    pub bp: Value,
    pub path: Path,
    pub scope: Scope,
    pub row: Row,
    pub gens: u32,
    /// The current generation and the value it was built for.
    pub current: Option<(WidgetId, Value)>,
    /// The dynamic of the current generation's result (round 46 RD).
    pub result: NodeId,
}

impl RebuildData {
    pub fn values(&self) -> Vec<&Value> {
        let mut v = vec![&self.bp];
        if let Some((_, x)) = &self.current {
            v.push(x);
        }
        v
    }
}

/// What a `for` hands back, by what its rows end on.
#[derive(Clone)]
pub(crate) enum ForResult {
    Unit,
    /// `!(K ==> T)`: the consolidation.
    Events(NodeId),
    /// `$(K ==> T)` over rows ending on dynamics: the rows map and its gathering.
    Dynamics { rows: NodeId, gathered: NodeId },
    /// `$(K ==> T)` over rows ending on plain values.
    Plain { rows: NodeId },
}

/// A keyed `for` over a dynamic map or array (Semantics ch. 4, `for`): one row per key,
/// kept as long as the key is present.
#[derive(Clone)]
pub(crate) struct ForData {
    pub owner: WidgetId,
    pub src: NodeId,
    pub bp: Value,
    pub path: Path,
    pub scope: Scope,
    pub row: Row,
    /// Each row's widget and result, by key.
    pub rows: OrdMap<Value, (WidgetId, Value)>,
    pub result: ForResult,
}

impl ForData {
    pub fn result_nodes(&self) -> Vec<NodeId> {
        match &self.result {
            ForResult::Unit => vec![],
            ForResult::Events(e) => vec![*e],
            ForResult::Dynamics { rows, gathered } => vec![*rows, *gathered],
            ForResult::Plain { rows } => vec![*rows],
        }
    }
    pub fn values(&self) -> Vec<&Value> {
        let mut v = vec![&self.bp];
        for (k, (_, r)) in self.rows.iter() {
            v.push(k);
            v.push(r);
        }
        v
    }
}

/// A host document (Semantics ch. 9): the child's places in a document of their own, and
/// a supplier instance per child place, at `path/[p]`, living as long as the place.
#[derive(Clone)]
pub(crate) struct HostData {
    pub owner: WidgetId,
    pub path: Path,
    pub scope: Scope,
    pub row: Row,
    /// `fn Path, Element c o -> @o`, a [`Value::Widget`].
    pub supplier: Value,
    pub child: Option<WidgetId>,
    pub instances: OrdMap<Path, WidgetId>,
    /// The child's places as emitted, by path: a `Written` map whose elements' live
    /// fields hold dynamics.
    pub raw: NodeId,
    /// The child's document, `$(Path ==> Elem c)`: the `Level`s of `raw`'s elements, a
    /// `HostDoc` node.
    pub doc: NodeId,
}

impl HostData {
    pub fn values(&self) -> Vec<&Value> {
        vec![&self.supplier]
    }
}

// ---------------------------------------------------------------------------------------
// Build
// ---------------------------------------------------------------------------------------

/// The statements of one widget block, as methods, while the widget is built.
///
/// Positions: an element statement, a `rebuild`, a `for`, an `@started`, a host document
/// and a `@Run` each take the next number; everything else takes none, and a widget
/// function applied with [`Build::call`] is transparent (Semantics ch. 8, "Paths and
/// generations"; doubt RP for `@Run`).
pub struct Build<'a> {
    pub(crate) st: &'a mut State,
    pub(crate) inputs: &'a Inputs,
    widget: WidgetId,
    prefix: Path,
    next: u32,
    scope: Scope,
    row: Row,
}

/// Build a widget: allocate it under `parent`, then run `body` against its `Build`.
pub(crate) fn construct(
    st: &mut State,
    inputs: &Inputs,
    parent: Option<WidgetId>,
    prefix: Path,
    scope: Scope,
    row: Row,
    body: impl FnOnce(&mut Build) -> Result<Value, Fault>,
) -> Result<(WidgetId, Value), Fault> {
    let w = st.widgets.alloc(Widget {
        parent,
        nodes: Vec::new(),
        children: Vec::new(),
        places: Vec::new(),
        hosted: Vec::new(),
        live: true,
    });
    if let Some(p) = parent {
        st.widgets.get_mut(p).children.push(w);
    }
    let mut b = Build { st, inputs, widget: w, prefix, next: 0, scope, row };
    let result = body(&mut b)?;
    Ok((w, result))
}

impl<'a> Build<'a> {
    /// The tick this construction belongs to.
    pub fn tick(&self) -> Tick {
        self.st.graph.now
    }

    /// The path the next positioned statement will take.
    pub fn next_path(&self) -> Path {
        self.prefix.child(Segment::Pos(self.next))
    }

    fn take_position(&mut self) -> Path {
        let p = self.prefix.child(Segment::Pos(self.next));
        self.next += 1;
        p
    }

    fn own(&mut self, kind: Kind) -> NodeId {
        let id = self.st.graph.add(kind, Some(self.widget));
        self.st.widgets.get_mut(self.widget).nodes.push(id);
        id
    }

    /// The dynamic behind a value in a `$` position: a dynamic's node, or a constant for a
    /// plain value (`$ v`, the free sigil).
    pub fn dynamic_of(&mut self, v: &Value) -> NodeId {
        match v {
            Value::Dynamic(id) => *id,
            plain => self.own(Kind::Const { value: plain.clone() }),
        }
    }

    fn event_of(&self, v: &Value) -> Result<NodeId, Fault> {
        match v {
            Value::Event(id) => Ok(*id),
            other => Err(Fault::runtime(format!("an event was needed, and {other} is not one"))),
        }
    }

    /// `emit r` as a block's first statement: the row its elements must come from.
    pub fn set_emit(&mut self, row: Row) {
        self.row = row;
    }

    /// A widget function applied in this block: transparent, so its statements take
    /// positions in this block's sequence (Semantics ch. 8). Its own `emit` applies to
    /// its own statements only.
    pub fn call(&mut self, bp: &Blueprint, args: &[Value]) -> Result<Value, Fault> {
        let row = self.row.clone();
        let r = bp.build(self, args);
        self.row = row;
        r
    }

    // ----- elements -----

    /// The element statement, `!rang <- Door.Chime(..)` or a bare element: the element
    /// takes the next position and its output is returned (Semantics ch. 8). In a host's
    /// child document, the host's supplier answers it instead of the World (ch. 9).
    pub fn emit(&mut self, e: Element) -> Result<Value, Fault> {
        if !self.row.allows(e.family()) {
            return Err(Fault::new(
                "Prelude.Sandbox",
                format!("{} emitted from a scope that emits {}", e.tag, self.row),
            ));
        }
        let path = self.take_position();
        match self.scope {
            Scope::Root => self.place(path, e),
            Scope::Hosted(host) => self.hosted(host, path, e),
        }
    }

    fn place(&mut self, path: Path, e: Element) -> Result<Value, Fault> {
        let decl = e.decl.clone().ok_or_else(|| Fault::runtime(format!("{} has no tag declaration", e.tag)))?;
        let mut requests = Vec::new();
        let mut live = Vec::new();
        for (name, v) in &e.fields {
            match decl.field_kind(name) {
                Some(crate::value::FieldKind::Request) => match v {
                    Value::Event(id) => requests.push((name.clone(), *id)),
                    Value::Unit => {}
                    other => return Err(Fault::runtime(format!("{}.{name} is a request field, given {other}", e.tag))),
                },
                Some(crate::value::FieldKind::Live) => {
                    if let Value::Dynamic(id) = v {
                        live.push(*id);
                    }
                }
                _ => {
                    if !v.is_plain() {
                        return Err(Fault::new(
                            "Runtime.Identity",
                            format!("{}.{name} is identity and takes no dynamic; write a rebuild", e.tag),
                        ));
                    }
                }
            }
        }
        let now = self.tick();
        let output = match &decl.output {
            OutputKind::Unit => Value::Unit,
            OutputKind::Event => Value::Event(self.own(Kind::Input)),
            OutputKind::Dynamic => {
                let lv = level(&mut self.st.graph, &e, now)?;
                let first = self.first_value(&lv)?;
                Value::Dynamic(self.own(Kind::External { cell: Cell2::new(first, now) }))
            }
            OutputKind::Plain => {
                let lv = level(&mut self.st.graph, &e, now)?;
                self.first_value(&lv)?
            }
            OutputKind::Bundle(parts) => {
                // The plain and dynamic components share one first-value query: the
                // value itself with one, a tuple of them with several (OutputKind::Bundle).
                let asked = parts.iter().filter(|k| matches!(k, OutputKind::Plain | OutputKind::Dynamic)).count();
                let firsts = if asked == 0 {
                    vec![]
                } else {
                    let lv = level(&mut self.st.graph, &e, now)?;
                    let first = self.first_value(&lv)?;
                    split_parts(first, asked, &e.tag)?
                };
                let mut firsts = firsts.into_iter();
                let mut items = Vec::with_capacity(parts.len());
                for k in parts {
                    items.push(match k {
                        OutputKind::Event => Value::Event(self.own(Kind::Input)),
                        OutputKind::Dynamic => {
                            let first = firsts.next().unwrap_or_default();
                            Value::Dynamic(self.own(Kind::External { cell: Cell2::new(first, now) }))
                        }
                        OutputKind::Plain => firsts.next().unwrap_or_default(),
                        OutputKind::Unit => Value::Unit,
                        OutputKind::Bundle(_) => {
                            return Err(Fault::runtime(format!("{}: a bundle output does not nest", e.tag)));
                        }
                    });
                }
                Value::tuple(items)
            }
        };
        let data = PlaceData {
            path: path.clone(),
            element: Rc::new(e),
            output: output.clone(),
            kind: decl.output.clone(),
            requests,
            live,
            request_fields: decl.request_fields(),
        };
        let place = self.own(Kind::Place(Box::new(data)));
        self.st.places.insert(path, place);
        self.st.widgets.get_mut(self.widget).places.push(place);
        self.st.built.push(place);
        Ok(output)
    }

    /// The first-value query (Semantics ch. 8, "A new place's first value"): the tick's
    /// pushes first, then an older place holding the same fact (the sharing rule, Graph
    /// ch. 4, External), then the World's ready state.
    fn first_value(&mut self, lv: &Value) -> Result<Value, Fault> {
        if let Some((_, v)) = self.inputs.pushes.iter().rev().find(|(e, _)| e == lv) {
            return Ok(v.clone());
        }
        let now = self.tick();
        let places: Vec<NodeId> = self.st.places.values().copied().collect();
        for p in places {
            let Kind::Place(data) = self.st.graph.kind(p) else { continue };
            let data = data.clone();
            if data.dynamics().is_empty() {
                continue;
            }
            if level(&mut self.st.graph, &data.element, now)? == *lv {
                return data.first_parts(&mut self.st.graph);
            }
        }
        self.inputs.ready.first_value(lv).ok_or_else(|| {
            Fault::new("World.Unanswered", format!("the ready state has no first value for {lv}"))
        })
    }

    fn hosted(&mut self, host: NodeId, path: Path, e: Element) -> Result<Value, Fault> {
        let Kind::Host(h) = self.st.graph.kind(host) else {
            return Err(Fault::runtime("a hosted scope whose host is gone"));
        };
        let (owner, key_path, scope, row, supplier) =
            (h.owner, h.path.child(Segment::Key(Value::Path(path.clone()))), h.scope, h.row.clone(), h.supplier.clone());
        let Value::Widget(supplier) = supplier else { unreachable!() };
        let element = Value::element(e);
        let args = [Value::Path(path.clone()), element.clone()];
        let (instance, out) =
            construct(self.st, self.inputs, Some(owner), key_path, scope, row, |b| supplier.build(b, &args))?;
        if let Kind::Host(h) = self.st.graph.kind_mut(host) {
            h.instances.insert(path.clone(), instance);
        }
        let (raw, doc) = match self.st.graph.kind(host) {
            Kind::Host(h) => (h.raw, h.doc),
            _ => unreachable!(),
        };
        let mut map = match self.st.graph.read_now(raw)? {
            Value::Map(m) => m,
            _ => OrdMap::new(),
        };
        map.insert(Value::Path(path.clone()), element);
        rewrite_host_doc(&mut self.st.graph, raw, doc, map);
        self.st.widgets.get_mut(self.widget).hosted.push((host, path));
        Ok(out)
    }

    // ----- state -----

    /// `@started`: fires once, in the tick after this widget is built (R7). Takes a
    /// position, though it makes no place (doubt PA).
    pub fn started(&mut self) -> Value {
        self.take_position();
        let id = self.own(Kind::Started);
        self.st.started_next.push(id);
        Value::Event(id)
    }

    /// A hold whose event is not bound yet: the binding group of a widget block lets a
    /// line read a hold defined below it (Semantics ch. 4, "Binding groups and cycles").
    /// Connect it with [`Build::hold_on`].
    pub fn hold(&mut self, init: Value) -> Value {
        let now = self.tick();
        Value::Dynamic(self.own(Kind::Hold { src: None, cell: Cell2::new(init, now) }))
    }

    /// `then !e`: the hold takes each firing of `!e` (R3).
    pub fn hold_on(&mut self, hold: &Value, event: &Value) -> Result<(), Fault> {
        let Value::Dynamic(h) = hold else { return Err(Fault::runtime("hold_on needs a hold")) };
        let e = self.event_of(event)?;
        self.st.graph.rewire(*h, |k| {
            if let Kind::Hold { src, .. } = k {
                *src = Some(e);
            }
        });
        Ok(())
    }

    /// `$x <- init then !e`.
    pub fn hold_then(&mut self, init: Value, event: &Value) -> Result<Value, Fault> {
        let h = self.hold(init);
        self.hold_on(&h, event)?;
        Ok(h)
    }

    /// An event bound below the line that reads it; resolve it with
    /// [`Build::resolve_event`].
    pub fn forward_event(&mut self) -> Value {
        Value::Event(self.own(Kind::Pass { src: None }))
    }

    pub fn resolve_event(&mut self, forward: &Value, event: &Value) -> Result<(), Fault> {
        let f = self.event_of(forward)?;
        let e = self.event_of(event)?;
        self.st.graph.rewire(f, |k| {
            if let Kind::Pass { src } = k {
                *src = Some(e);
            }
        });
        Ok(())
    }

    // ----- pure combinators -----

    /// `$ v`: a constant dynamic.
    pub fn constant(&mut self, v: Value) -> Value {
        Value::Dynamic(self.own(Kind::Const { value: v }))
    }

    /// `never`.
    pub fn never(&mut self) -> Value {
        Value::Event(self.own(Kind::Never))
    }

    /// A function lifted over one event: `f(v)` at each firing.
    pub fn map(&mut self, event: &Value, f: Closure) -> Result<Value, Fault> {
        let src = self.event_of(event)?;
        Ok(Value::Event(self.own(Kind::Map { src, f })))
    }

    /// A function lifted over an event and dynamics, `$count + 1 on !inc`: at each firing,
    /// `f(v, d1, ..)` with the dynamics as the tick opened (the tag law).
    pub fn sample(&mut self, event: &Value, dyns: &[Value], f: Closure) -> Result<Value, Fault> {
        let src = self.event_of(event)?;
        let dyns = dyns.iter().map(|d| self.dynamic_of(d)).collect();
        Ok(Value::Event(self.own(Kind::Sample { src, dyns, f })))
    }

    /// `x on !e`: a value tagged at an event. A dynamic `x` is sampled as the tick opened.
    pub fn tag(&mut self, event: &Value, x: &Value) -> Result<Value, Fault> {
        match x {
            Value::Dynamic(_) => self.sample(event, std::slice::from_ref(x), Closure::pure("on", |a| Ok(a[1].clone()))),
            plain => {
                let v = plain.clone();
                self.map(event, Closure::new("on", vec![v], |c, _| Ok(c[0].clone())))
            }
        }
    }

    /// A function lifted over dynamics (the change law): a dynamic of `f(inputs)`.
    pub fn lift(&mut self, inputs: &[Value], f: Closure) -> Value {
        let inputs = inputs.iter().map(|d| self.dynamic_of(d)).collect();
        Value::Dynamic(self.own(Kind::Lift { inputs, f, cache: Cache2::default() }))
    }

    /// `a <| b <| ...`: the leftmost merge.
    pub fn merge(&mut self, events: &[Value]) -> Result<Value, Fault> {
        let srcs = events.iter().map(|e| self.event_of(e)).collect::<Result<_, _>>()?;
        Ok(Value::Event(self.own(Kind::Merge { srcs })))
    }

    /// Several events in one tick: when any fires, `f` gets `? v` or `nil` per input and
    /// fires the node with `r` when it returns `? r`. `!Union`, `!MergeAll`, `!Both`,
    /// `!Except` and `!Align` are each a `combine`.
    pub fn combine(&mut self, events: &[Value], f: Closure) -> Result<Value, Fault> {
        let srcs = events.iter().map(|e| self.event_of(e)).collect::<Result<_, _>>()?;
        Ok(Value::Event(self.own(Kind::Combine { srcs, f })))
    }

    /// `!Union: fn !(k ==> v), !(k ==> v) -> !(k ==> v)`: the left winning a shared key.
    pub fn union(&mut self, left: &Value, right: &Value) -> Result<Value, Fault> {
        self.combine(
            &[left.clone(), right.clone()],
            Closure::pure("!Union", |a| {
                let mut out = im_rc::OrdMap::new();
                for side in a.iter().rev() {
                    if let Value::Opt(Some(m)) = side {
                        if let Value::Map(m) = &**m {
                            out = m.clone().union(out);
                        }
                    }
                }
                Ok(Value::some(Value::Map(out)))
            }),
        )
    }

    /// `!Filter: fn !?a -> !a`.
    pub fn filter(&mut self, event: &Value) -> Result<Value, Fault> {
        let src = self.event_of(event)?;
        Ok(Value::Event(self.own(Kind::Filter { src })))
    }

    /// `!Present: fn !?a -> !unit`.
    pub fn present(&mut self, event: &Value) -> Result<Value, Fault> {
        let src = self.event_of(event)?;
        Ok(Value::Event(self.own(Kind::Present { src })))
    }

    /// `!Gate: fn $Bool, !a -> !a`.
    pub fn gate(&mut self, cond: &Value, event: &Value) -> Result<Value, Fault> {
        let cond = self.dynamic_of(cond);
        let src = self.event_of(event)?;
        Ok(Value::Event(self.own(Kind::Gate { cond, src })))
    }

    /// `!Updated: fn $a -> !a`.
    pub fn updated(&mut self, dynamic: &Value) -> Value {
        let d = self.dynamic_of(dynamic);
        Value::Event(self.own(Kind::Updated { dynamic: d }))
    }

    /// A dynamic's value after this tick, the value construction reads (R7).
    pub fn read(&mut self, v: &Value) -> Result<Value, Fault> {
        match v {
            Value::Dynamic(d) => self.st.graph.read_now(*d),
            plain => Ok(plain.clone()),
        }
    }

    // ----- widget bindings -----

    /// `!e <- @Switch $!d` (round 47 SW): fires with the inner its dynamic held as the
    /// tick opened, and adopts at the end of each tick whatever inner the dynamic holds
    /// after it.
    pub fn switch(&mut self, outer: &Value) -> Result<Value, Fault> {
        let outer = self.dynamic_of(outer);
        let inner = match self.st.graph.read_now(outer)? {
            Value::Event(e) => e,
            other => return Err(Fault::runtime(format!("@Switch needs a dynamic of events, holding {other}"))),
        };
        Ok(Value::Event(self.own(Kind::Switch { outer, inner })))
    }

    /// `$v <- @Flatten $$d` (round 47 SW): the inner its outer held as the tick opened;
    /// moves to a new inner one tick after the outer changes.
    pub fn flatten(&mut self, outer: &Value) -> Result<Value, Fault> {
        let outer = self.dynamic_of(outer);
        let inner_v = self.st.graph.read_now(outer)?;
        let inner = self.dynamic_of(&inner_v);
        let now = self.tick();
        let first = self.st.graph.read_now(inner)?;
        Ok(Value::Dynamic(self.own(Kind::Flatten { outer, inner, cell: Cell2::new(first, now), adopting: false })))
    }

    /// `@Defer: fn !a -> @!a` (decisions.md D6): every firing fires again in the next
    /// tick, by the runtime, with no World involved; an armed `@Defer` sets `pending`.
    pub fn defer(&mut self, event: &Value) -> Result<Value, Fault> {
        let src = self.event_of(event)?;
        let out = self.own(Kind::DeferOut { armed: None, armed_for: -1 });
        self.own(Kind::DeferIn { src, out });
        Ok(Value::Event(out))
    }

    /// `@Machine: fn (fn s, a -> ((s, ?b))), s, !a -> @!b`: a hold of `s` stepped by each
    /// firing of `!a`, firing `b` when the step gives one.
    pub fn machine(&mut self, step: Closure, init: Value, event: &Value) -> Result<Value, Fault> {
        let src = self.event_of(event)?;
        let now = self.tick();
        Ok(Value::Event(self.own(Kind::Machine { src, step, state: Cell2::new(init, now) })))
    }

    /// `@Run f, x`, the library definition over `@Machine` and `@Defer` (Semantics ch. 7,
    /// "An unbounded function is a bounded body iterated"; D6): one cycle per tick, from
    /// the tick after the build, each cycle deferring the next until the machine
    /// finishes; the result is an event in the tick it finishes.
    ///
    /// `cycle` is the function's bounded body between two cycle points: given the saved
    /// continuation it returns `Continue next` or `Done result`. The interpreter makes it
    /// from the unbounded function; `start` is the first continuation. Takes a position,
    /// as `@started` does (doubt RP).
    pub fn run(&mut self, cycle: Closure, start: Value) -> Result<Value, Fault> {
        let started = self.started();
        let deferred = self.own(Kind::DeferOut { armed: None, armed_for: -1 });
        let kick = self.merge(&[started, Value::Event(deferred)])?;
        let step = Closure::new("@Run.step", vec![Value::Closure(cycle)], |cap, args| {
            let Value::Closure(cycle) = &cap[0] else { unreachable!() };
            let r = cycle.call(&args[..1])?;
            let Value::Choice(c) = &r else {
                return Err(Fault::runtime("a cycle returns Continue s or Done v"));
            };
            match (&*c.ctor, c.payload.as_slice()) {
                ("Continue", [s]) => Ok(Value::tuple(vec![s.clone(), Value::some(r.clone())])),
                ("Done", [_]) => Ok(Value::tuple(vec![Value::Unit, Value::some(r.clone())])),
                _ => Err(Fault::runtime("a cycle returns Continue s or Done v")),
            }
        });
        let machine = self.machine(step, start, &kick)?;
        let pick = |want: &'static str| {
            Closure::pure(want, move |a| match &a[0] {
                Value::Choice(c) if &*c.ctor == want => Ok(Value::some(c.payload[0].clone())),
                _ => Ok(Value::nil()),
            })
        };
        let cont = self.map(&machine, pick("Continue"))?;
        let cont = self.filter(&cont)?;
        self.own(Kind::DeferIn { src: self.event_of(&cont)?, out: deferred });
        let done = self.map(&machine, pick("Done"))?;
        self.filter(&done)
    }

    /// `@Later: fn !a -> @!a`: each firing answered one tick later through a zero-delay
    /// `Time.After`; a World implements time (Semantics Appendix B).
    pub fn later(&mut self, event: &Value) -> Result<Value, Fault> {
        let wake = Closure::pure("@Later.wake", |a| {
            Ok(Value::record("Wake", vec![("after", Value::Duration(0)), ("context", a[0].clone())]))
        });
        let wakes = self.map(event, wake)?;
        let woke = self.emit(Element::new(&crate::families::time::after(), vec![("wakes", wakes)])?)?;
        self.map(&woke, Closure::pure("@Later.context", |a| Ok(a[0].field("context").cloned().unwrap_or(Value::Unit))))
    }

    // ----- builders -----

    /// `rebuild $x:` (Semantics ch. 4; round 46 RD): builds `bp` with the current value of
    /// `$x` at `p/#0`, and anew at `p/#g` whenever `$x` changes, at the end of the tick it
    /// changes in. Hands back a dynamic of the current generation's result, written as
    /// the last act of construction (Reflex's `widgetHold`), or `unit`.
    pub fn rebuild(&mut self, src: &Value, kind: ResultKind, bp: &Blueprint) -> Result<Value, Fault> {
        let path = self.take_position();
        let src = self.dynamic_of(src);
        let now = self.tick();
        let result = self.own(Kind::Written { cell: Cell2::new(Value::Unit, now) });
        let data = RebuildData {
            owner: self.widget,
            src,
            bp: Value::Widget(bp.clone()),
            path,
            scope: self.scope,
            row: self.row.clone(),
            gens: 0,
            current: None,
            result,
        };
        let node = self.own(Kind::Rebuild(Box::new(data)));
        let v = self.st.graph.read_now(src)?;
        build_generation(self.st, self.inputs, node, v)?;
        Ok(if kind == ResultKind::Unit { Value::Unit } else { Value::Dynamic(result) })
    }

    /// `for k, $v <- $m:` (Semantics ch. 4): builds `bp` once per key at `p/[k]`, keeps a
    /// row as long as its key is present, and gathers the rows' results, keyed. `bp`
    /// takes the key, the row's value (a dynamic for a dynamic collection, plain for a
    /// plain one) and `ix`, the row's position in key order when it was built. A dynamic
    /// array is keyed by index.
    pub fn for_each(&mut self, src: &Value, kind: ResultKind, bp: &Blueprint) -> Result<Value, Fault> {
        let path = self.take_position();
        if !matches!(src, Value::Dynamic(_)) {
            return self.for_plain(path, src, kind, bp);
        }
        let src = self.dynamic_of(src);
        let now = self.tick();
        let result = match kind {
            ResultKind::Unit => ForResult::Unit,
            ResultKind::Event => ForResult::Events(self.own(Kind::MergeRows { rows: OrdMap::new() })),
            ResultKind::Dynamic => {
                let rows = self.own(Kind::Written { cell: Cell2::new(Value::Map(OrdMap::new()), now) });
                let gathered = self.own(Kind::ForDyn { rows, inputs: vec![], cache: Cache2::default() });
                ForResult::Dynamics { rows, gathered }
            }
            ResultKind::Plain => {
                ForResult::Plain { rows: self.own(Kind::Written { cell: Cell2::new(Value::Map(OrdMap::new()), now) }) }
            }
        };
        let handed_back = match &result {
            ForResult::Unit => Value::Unit,
            ForResult::Events(e) => Value::Event(*e),
            ForResult::Dynamics { gathered, .. } => Value::Dynamic(*gathered),
            ForResult::Plain { rows } => Value::Dynamic(*rows),
        };
        let data = ForData {
            owner: self.widget,
            src,
            bp: Value::Widget(bp.clone()),
            path,
            scope: self.scope,
            row: self.row.clone(),
            rows: OrdMap::new(),
            result,
        };
        let node = self.own(Kind::For(Box::new(data)));
        let m = self.st.graph.read_now(src)?;
        build_rows(self.st, self.inputs, node, &m, &keys_of(&m))?;
        Ok(handed_back)
    }

    /// A `for` over a plain collection: built once.
    fn for_plain(&mut self, path: Path, src: &Value, kind: ResultKind, bp: &Blueprint) -> Result<Value, Fault> {
        let mut out = OrdMap::new();
        for (ix, k) in keys_of(src).into_iter().enumerate() {
            let v = entry(src, &k).unwrap_or(Value::Unit);
            let prefix = path.child(Segment::Key(k.clone()));
            let args = [k.clone(), v, Value::nat(ix as u64)];
            let (_, r) = construct(self.st, self.inputs, Some(self.widget), prefix, self.scope, self.row.clone(), |b| {
                bp.build(b, &args)
            })?;
            out.insert(k, r);
        }
        Ok(match kind {
            ResultKind::Unit => Value::Unit,
            _ => Value::Map(out),
        })
    }

    // ----- hosting -----

    /// `@HostDocument @child, supplier` (Semantics ch. 9): builds the child with its
    /// places in a document of their own; each child place is answered by an instance of
    /// `supplier`, built in this scope at `p/[cp]` with the place's path and element, and
    /// living as long as the place. Returns `((result, $doc))`, the child's result and its
    /// document `$(Path ==> Elem c)`.
    pub fn host_document(&mut self, child: &Blueprint, args: &[Value], supplier: &Blueprint) -> Result<Value, Fault> {
        let path = self.take_position();
        let now = self.tick();
        let raw = self.own(Kind::Written { cell: Cell2::new(Value::Map(OrdMap::new()), now) });
        let doc = self.own(Kind::HostDoc { raw, inputs: vec![], cache: Cache2::default() });
        let data = HostData {
            owner: self.widget,
            path,
            scope: self.scope,
            row: self.row.clone(),
            supplier: Value::Widget(supplier.clone()),
            child: None,
            instances: OrdMap::new(),
            raw,
            doc,
        };
        let host = self.own(Kind::Host(Box::new(data)));
        let (cw, result) = construct(self.st, self.inputs, Some(self.widget), Path::root(), Scope::Hosted(host), Row::Any, |b| {
            child.build(b, args)
        })?;
        if let Kind::Host(h) = self.st.graph.kind_mut(host) {
            h.child = Some(cw);
        }
        Ok(Value::tuple(vec![result, Value::Dynamic(doc)]))
    }

    /// `@HostSome @child, ?Give`, derived from `@HostDocument` as Semantics ch. 9 derives
    /// it: `?Give p, e` gives `? @b` to answer a place, or `nil` to forward it with
    /// `@Element e`, the generic arm, written once.
    pub fn host_some(&mut self, child: &Blueprint, args: &[Value], give: &Closure) -> Result<Value, Fault> {
        let supplier = Blueprint::new("@HostSome.supplier", vec![Value::Closure(give.clone())], |b, cap, args| {
            let Value::Closure(give) = &cap[0] else { unreachable!() };
            match give.call(args)? {
                Value::Opt(Some(bp)) => match &*bp {
                    Value::Widget(bp) => b.call(bp, &[]),
                    _ => Err(Fault::runtime("?Give returns ? @blueprint or nil")),
                },
                Value::Opt(None) => match &args[1] {
                    Value::Element(e) => b.emit((**e).clone()),
                    _ => Err(Fault::runtime("a supplier's second argument is the element")),
                },
                _ => Err(Fault::runtime("?Give returns ? @blueprint or nil")),
            }
        });
        self.host_document(child, args, &supplier)
    }

    /// `@HostList @child, !answers, supplier`, derived through `@HostDocument`: each event
    /// place hears the answers sent down to its path, merged ahead of its supplier's.
    pub fn host_list(
        &mut self,
        child: &Blueprint,
        args: &[Value],
        answers: &Value,
        supplier: &Blueprint,
    ) -> Result<Value, Fault> {
        let wrapped = Blueprint::new(
            "@HostList.supplier",
            vec![answers.clone(), Value::Widget(supplier.clone())],
            |b, cap, args| {
                let Value::Widget(supplier) = &cap[1] else { unreachable!() };
                let out = b.call(supplier, args)?;
                if !matches!(out, Value::Event(_)) {
                    return Ok(out);
                }
                let pick = Closure::new("@HostList.pick", vec![args[0].clone()], |c, a| match &a[0] {
                    Value::Map(m) => Ok(m.get(&c[0]).cloned().map(Value::some).unwrap_or(Value::nil())),
                    _ => Ok(Value::nil()),
                });
                let mine = b.map(&cap[0], pick)?;
                let mine = b.filter(&mine)?;
                b.merge(&[mine, out])
            },
        );
        self.host_document(child, args, &wrapped)
    }
}

// ---------------------------------------------------------------------------------------
// Construction at the end of a tick, and leaving
// ---------------------------------------------------------------------------------------

/// Build a `rebuild`'s next generation for the value `v`, and write its result as the
/// last act of construction (round 46 RD).
fn build_generation(st: &mut State, inputs: &Inputs, node: NodeId, v: Value) -> Result<(), Fault> {
    let Kind::Rebuild(r) = st.graph.kind(node) else { unreachable!() };
    let (owner, path, scope, row, g, bp, result) =
        (r.owner, r.path.clone(), r.scope, r.row.clone(), r.gens, r.bp.clone(), r.result);
    let Value::Widget(bp) = bp else { unreachable!() };
    let prefix = path.child(Segment::Gen(g));
    let arg = [v.clone()];
    let (w, out) = construct(st, inputs, Some(owner), prefix, scope, row, |b| bp.build(b, &arg))?;
    if let Kind::Rebuild(r) = st.graph.kind_mut(node) {
        r.gens += 1;
        r.current = Some((w, v));
    }
    st.graph.write_cell(result, out);
    Ok(())
}

/// Write a host's map of child elements, and point its document at their live fields'
/// dynamics, so the document moves when any of them does.
fn rewrite_host_doc(graph: &mut crate::graph::Graph, raw: NodeId, doc: NodeId, map: OrdMap<Value, Value>) {
    let mut inputs: Vec<NodeId> = Vec::new();
    for e in map.values() {
        if let Value::Element(e) = e {
            for (name, v) in &e.fields {
                let live = e.decl.as_ref().and_then(|d| d.field_kind(name)) == Some(crate::value::FieldKind::Live);
                if let (true, Value::Dynamic(d)) = (live, v) {
                    if !inputs.contains(d) {
                        inputs.push(*d);
                    }
                }
            }
        }
    }
    graph.write_cell(raw, Value::Map(map));
    graph.rewire(doc, |k| {
        if let Kind::HostDoc { inputs: i, .. } = k {
            *i = inputs;
        }
    });
}

/// A bundle's first value split into its `n` plain and dynamic components: the value
/// itself for one, a tuple's items for several.
pub(crate) fn split_parts(v: Value, n: usize, tag: &str) -> Result<Vec<Value>, Fault> {
    if n == 1 {
        return Ok(vec![v]);
    }
    match v {
        Value::Tuple(items) if items.len() == n => Ok((*items).clone()),
        other => Err(Fault::runtime(format!("{tag}'s answer has {n} components, and {other} is not a tuple of {n}"))),
    }
}

/// The keys of a map, or the indices of an array.
fn keys_of(m: &Value) -> Vec<Value> {
    match m {
        Value::Map(m) => m.keys().cloned().collect(),
        Value::Array(a) => (0..a.len()).map(|i| Value::nat(i as u64)).collect(),
        _ => vec![],
    }
}

fn entry(m: &Value, k: &Value) -> Option<Value> {
    match m {
        Value::Map(m) => m.get(k).cloned(),
        Value::Array(a) => k.as_i128().and_then(|i| a.get(i as usize).cloned()),
        _ => None,
    }
}

/// Build the rows for `keys` of a `for`, then bring its gathered result up to date.
fn build_rows(st: &mut State, inputs: &Inputs, node: NodeId, m: &Value, keys: &[Value]) -> Result<(), Fault> {
    let Kind::For(f) = st.graph.kind(node) else { unreachable!() };
    let (owner, src, path, scope, row, bp) = (f.owner, f.src, f.path.clone(), f.scope, f.row.clone(), f.bp.clone());
    let Value::Widget(bp) = bp else { unreachable!() };
    let all = keys_of(m);
    for k in keys {
        let ix = all.iter().position(|x| x == k).unwrap_or(0);
        let prefix = path.child(Segment::Key(k.clone()));
        let key = k.clone();
        let (w, r) = construct(st, inputs, Some(owner), prefix, scope, row.clone(), |b| {
            let value = Value::Dynamic(b.own(Kind::MapEntry { src, key: key.clone(), cache: Cache2::default() }));
            bp.build(b, &[key.clone(), value, Value::nat(ix as u64)])
        })?;
        if let Kind::For(f) = st.graph.kind_mut(node) {
            f.rows.insert(k.clone(), (w, r));
        }
    }
    refresh_for_result(st, node)
}

/// After a `for`'s rows change at the end of a tick: the consolidation listens to the
/// current rows, and the rows map is written, as construction (heard by state only).
fn refresh_for_result(st: &mut State, node: NodeId) -> Result<(), Fault> {
    let Kind::For(f) = st.graph.kind(node) else { unreachable!() };
    let (result, rows) = (f.result.clone(), f.rows.clone());
    match result {
        ForResult::Unit => {}
        ForResult::Events(merge) => {
            let mut events = OrdMap::new();
            for (k, (_, r)) in rows.iter() {
                if let Value::Event(e) = r {
                    events.insert(k.clone(), *e);
                } else {
                    return Err(Fault::runtime(format!("a row of an event `for` ended on {r}")));
                }
            }
            st.graph.rewire(merge, |k| {
                if let Kind::MergeRows { rows } = k {
                    *rows = events;
                }
            });
        }
        ForResult::Dynamics { rows: cell, gathered } => {
            let map: OrdMap<Value, Value> = rows.iter().map(|(k, (_, r))| (k.clone(), r.clone())).collect();
            let inputs: Vec<NodeId> = map
                .values()
                .filter_map(|v| if let Value::Dynamic(d) = v { Some(*d) } else { None })
                .collect();
            st.graph.rewire(gathered, |k| {
                if let Kind::ForDyn { inputs: i, .. } = k {
                    *i = inputs;
                }
            });
            st.graph.write_cell(cell, Value::Map(map));
        }
        ForResult::Plain { rows: cell } => {
            let map: OrdMap<Value, Value> = rows.iter().map(|(k, (_, r))| (k.clone(), r.clone())).collect();
            st.graph.write_cell(cell, Value::Map(map));
        }
    }
    Ok(())
}

/// What a builder will build once the drops are done.
pub(crate) enum Plan {
    Generation(Value),
    Rows(Value, Vec<Value>),
}

/// The path that orders builders in the end-of-tick walk: outer before inner.
pub(crate) fn builder_path(st: &State, node: NodeId) -> Option<(Path, WidgetId)> {
    match st.graph.kind(node) {
        Kind::Rebuild(r) => Some((r.path.clone(), r.owner)),
        Kind::For(f) => Some((f.path.clone(), f.owner)),
        _ => None,
    }
}

/// Step 4 of Graph ch. 4's tick: a builder whose dynamic changed drops what left, freeing
/// its paths, and says what it will build.
pub(crate) fn drop_phase(st: &mut State, node: NodeId) -> Result<Option<Plan>, Fault> {
    match st.graph.kind(node).clone() {
        Kind::Rebuild(r) => {
            let v = st.graph.read_now(r.src)?;
            if r.current.as_ref().is_some_and(|(_, old)| *old == v) {
                return Ok(None);
            }
            if let Some((w, _)) = r.current {
                detach(st, w);
            }
            Ok(Some(Plan::Generation(v)))
        }
        Kind::For(f) => {
            let m = st.graph.read_now(f.src)?;
            let new_keys = keys_of(&m);
            let mut rows = f.rows.clone();
            let mut changed = false;
            for (k, (w, _)) in f.rows.iter() {
                if !new_keys.contains(k) {
                    detach(st, *w);
                    rows.remove(k);
                    changed = true;
                }
            }
            let added: Vec<Value> = new_keys.into_iter().filter(|k| !f.rows.contains_key(k)).collect();
            if let Kind::For(f) = st.graph.kind_mut(node) {
                f.rows = rows;
            }
            if added.is_empty() && !changed {
                return Ok(None);
            }
            Ok(Some(Plan::Rows(m, added)))
        }
        _ => Ok(None),
    }
}

/// Step 5: the build list. A build is whole, nested builders included.
pub(crate) fn build_phase(st: &mut State, inputs: &Inputs, node: NodeId, plan: Plan) -> Result<(), Fault> {
    match plan {
        Plan::Generation(v) => build_generation(st, inputs, node, v),
        Plan::Rows(m, added) => build_rows(st, inputs, node, &m, &added),
    }
}

/// Step 6: the swap list. A switch or flatten whose outer changed takes its new inner,
/// subscribing to it before letting go of the old, with no catch-up; it hears the new
/// inner from the next tick (round 47 SW; Graph ch. 4, `@Switch`, `@Flatten`).
pub(crate) fn swap(st: &mut State, node: NodeId) -> Result<(), Fault> {
    match st.graph.kind(node).clone() {
        Kind::Switch { outer, inner } => {
            let new = match st.graph.read_now(outer)? {
                Value::Event(e) => e,
                other => return Err(Fault::runtime(format!("@Switch's dynamic holds {other}, not an event"))),
            };
            if new != inner {
                st.graph.rewire(node, |k| {
                    if let Kind::Switch { inner, .. } = k {
                        *inner = new;
                    }
                });
            }
        }
        Kind::Flatten { outer, inner, .. } => {
            let new = match st.graph.read_now(outer)? {
                Value::Dynamic(d) => d,
                plain => {
                    let owner = st.graph.node(node).owner;
                    let id = st.graph.add(Kind::Const { value: plain }, owner);
                    if let Some(w) = owner {
                        st.widgets.get_mut(w).nodes.push(id);
                    }
                    id
                }
            };
            if new != inner {
                st.graph.rewire(node, |k| {
                    if let Kind::Flatten { inner, .. } = k {
                        *inner = new;
                    }
                });
                st.adoptions_next.push(node);
            }
        }
        _ => {}
    }
    Ok(())
}

/// Detach a widget and everything under it, at its builder's turn in the end-of-tick
/// walk: its places leave the document's index, so their paths are free for a newborn,
/// and it goes on the leaving list. It has already run to the end of the tick (R8).
pub(crate) fn detach(st: &mut State, w: WidgetId) {
    if !st.widgets.is_live(w) {
        return;
    }
    st.widgets.get_mut(w).live = false;
    st.leaving.push(w);
    let widget = st.widgets.get(w).clone();
    if let Some(p) = widget.parent {
        if st.widgets.is_live(p) {
            st.widgets.get_mut(p).children.retain(|c| *c != w);
        }
    }
    for place in &widget.places {
        if let Kind::Place(data) = st.graph.kind(*place) {
            let path = data.path.clone();
            if st.places.get(&path) == Some(place) {
                st.places.remove(&path);
            }
            st.removed.push(path);
        }
    }
    // Teardown is seen at the host, never by the widget (Semantics ch. 9): the place's
    // key leaves the child document and its supplier instance leaves with it.
    for (host, cp) in &widget.hosted {
        let instance = match st.graph.kind_mut(*host) {
            Kind::Host(h) => h.instances.remove(cp),
            _ => None,
        };
        if let Kind::Host(h) = st.graph.kind(*host) {
            let (raw, doc) = (h.raw, h.doc);
            if let Ok(Value::Map(mut m)) = st.graph.read_now(raw) {
                m.remove(&Value::Path(cp.clone()));
                rewrite_host_doc(&mut st.graph, raw, doc, m);
            }
        }
        if let Some(i) = instance {
            detach(st, i);
        }
    }
    for c in widget.children {
        detach(st, c);
    }
}

/// Step 7: a detached widget leaves. Each of its holds becomes a constant with its last
/// value, each current inner of its switches and flattens is dropped, and each event
/// from its own elements and holds becomes `never` (Semantics ch. 7, "Holds die with
/// their widget"). Its stateless combinators stay what they were, and run only while
/// something subscribes: the zombie (round 47 ZB).
pub(crate) fn leave(st: &mut State, w: WidgetId) {
    for id in st.widgets.get(w).nodes.clone() {
        if !st.graph.exists(id) {
            continue;
        }
        let replacement = match st.graph.kind(id) {
            Kind::Hold { cell, .. } | Kind::External { cell } | Kind::Written { cell } | Kind::Flatten { cell, .. } => {
                Some(Kind::Const { value: cell.latest().clone() })
            }
            Kind::Input
            | Kind::Started
            | Kind::Pass { .. }
            | Kind::Switch { .. }
            | Kind::Machine { .. }
            | Kind::DeferIn { .. }
            | Kind::DeferOut { .. }
            | Kind::Place(_)
            | Kind::Rebuild(_)
            | Kind::For(_)
            | Kind::Host(_) => Some(Kind::Never),
            _ => None,
        };
        match replacement {
            Some(k) => st.graph.retire(id, k),
            None => st.graph.node_mut(id).owner = None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_of_arrays_are_indices() {
        let a = Value::array(vec![Value::text("a"), Value::text("b")]);
        assert_eq!(keys_of(&a), vec![Value::nat(0), Value::nat(1)]);
        assert_eq!(entry(&a, &Value::nat(1)), Some(Value::text("b")));
    }
}
