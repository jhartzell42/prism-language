//! The graph: nodes with heights, subscriptions, demand, double-buffered holds, marks,
//! and propagation by height within a tick.
//!
//! This is Graph's algorithm (Graph ch. 4) minus what decisions.md D4 leaves out
//! (compaction, degradation, the memory layout, fusion, threads). The pieces, and the
//! rules each one implements:
//!
//! - **Nodes in an arena, addressed by [`NodeId`].** Edges are ids, never pointers or
//!   callbacks: a consumer's end of an edge (its kind's inputs) is the strong reference,
//!   and a producer's `subscribers` and `readers` lists are weak (Graph ch. 4,
//!   "Downstream and upstream"). The arena is a persistent vector, so a [`State`]
//!   (crate::tick) is cheap to keep and a failed tick can be thrown away whole.
//! - **Heights.** A node's height is above every input it depends on promptly, so one
//!   pass in height order visits each node once (R4, R5; Graph ch. 4, "Heights"). Heights
//!   are computed when a node is subscribed and only ever raised.
//! - **Demand.** An event node computes only while something subscribes to it, or while
//!   it is a *sink*: a hold, a machine, a `@Defer`, a flatten, or a place with requests
//!   (Semantics ch. 12, "Demand"; Graph ch. 4, "Demand"). A sink always subscribes to its
//!   writers while its widget lives (G4). Subscribing activates a node, which subscribes
//!   to its own inputs; the last unsubscription deactivates it. Liens and tags are not
//!   modelled separately: a value holding a handle keeps the node alive through the
//!   collector, and a sample is a read that needs no subscription.
//! - **Holds without a commit pass.** A hold is two values, each stamped with the tick
//!   that wrote it ([`Cell2`]); a read as of tick `t` takes the newer value stamped at or
//!   before `t`. Reads during propagation of tick `n` are as of `n - 1` (the tag law, R2);
//!   reads after propagation are as of `n` (Graph ch. 4, "Holds without a commit pass").
//! - **Computed dynamics by stamped marks.** A lifted dynamic caches up to two values,
//!   each valid over a range of ticks ([`Cache2`]). A write at `n` marks its readers,
//!   closing every cache range that reached `n`; a read recomputes only what a mark
//!   closed (Graph ch. 4, "Invalidation by stamped marks"). So a sample costs one read
//!   or the stale part of the tree below it.
//! - **The change law.** A lifted dynamic's update event fires at `n` exactly when its
//!   value after `n` differs from its value after `n - 1` (R3); so does a hold's.

use std::collections::BTreeSet;

use im_rc::{OrdMap, Vector};

use crate::document::{PlaceData, Request};
use crate::value::{Closure, Fault, NodeId, Tick, Value, WidgetId};
use crate::widget::{ForData, HostData, RebuildData};

/// "Valid for ever", the open end of a validity range.
pub(crate) const FOREVER: Tick = Tick::MAX;

// ---------------------------------------------------------------------------------------
// Double-buffered cells and stamped caches
// ---------------------------------------------------------------------------------------

/// A hold's two variables, each stamped with the tick that wrote it (Graph ch. 4,
/// "Holds without a commit pass"; Compiler ch. 5's `Hold<T>`). Between ticks the newer
/// value is the hold's value; the older one, the shadow, is read by nothing after the
/// tick of the write.
#[derive(Clone, Debug)]
pub(crate) struct Cell2 {
    cur: Value,
    at: Tick,
    prev: Option<(Value, Tick)>,
}

impl Cell2 {
    pub fn new(v: Value, at: Tick) -> Cell2 {
        Cell2 { cur: v, at, prev: None }
    }
    /// The value as of tick `asof` (after `asof`), and the last tick it is known valid
    /// for.
    pub fn read(&self, asof: Tick) -> (Value, Tick) {
        if asof >= self.at {
            return (self.cur.clone(), FOREVER);
        }
        match &self.prev {
            Some((p, _)) => (p.clone(), self.at - 1),
            // A read from before the cell was made: only a newborn's own construction
            // could ask, and it reads as of its birth tick, so this is never reached.
            None => (self.cur.clone(), FOREVER),
        }
    }
    /// Write at tick `t`. At most one write per tick (R5); a second write in one tick
    /// replaces the first, which only construction at the end of a tick does.
    pub fn write(&mut self, v: Value, t: Tick) {
        if t == self.at {
            self.cur = v;
        } else {
            let old = std::mem::replace(&mut self.cur, v);
            self.prev = Some((old, self.at));
            self.at = t;
        }
    }
    /// The newer value: what the hold holds between ticks, and its constant when its
    /// widget dies.
    pub fn latest(&self) -> &Value {
        &self.cur
    }
    pub fn values(&self) -> impl Iterator<Item = &Value> {
        std::iter::once(&self.cur).chain(self.prev.iter().map(|(v, _)| v))
    }
}

/// One cached value of a computed dynamic, valid as of every tick in `from..=until`.
#[derive(Clone, Debug)]
pub(crate) struct Slot {
    value: Value,
    from: Tick,
    until: Tick,
}

/// A computed dynamic's cache: two stamped slots, enough for the two read points of a
/// tick, before `n` and after `n`.
#[derive(Clone, Debug, Default)]
pub(crate) struct Cache2 {
    slots: [Option<Slot>; 2],
}

impl Cache2 {
    fn get(&self, asof: Tick) -> Option<(Value, Tick)> {
        self.slots
            .iter()
            .flatten()
            .find(|s| s.from <= asof && asof <= s.until)
            .map(|s| (s.value.clone(), s.until))
    }
    fn put(&mut self, value: Value, from: Tick, until: Tick) {
        let new = Some(Slot { value, from, until });
        let victim = match (&self.slots[0], &self.slots[1]) {
            (None, _) => 0,
            (_, None) => 1,
            (Some(a), Some(b)) => {
                // Never evict the open slot for a closed one; otherwise drop the older.
                let keep_a = until != FOREVER && a.until == FOREVER;
                let keep_b = until != FOREVER && b.until == FOREVER;
                if keep_a || (!keep_b && a.from > b.from) { 1 } else { 0 }
            }
        };
        self.slots[victim] = new;
    }
    /// A mark at `m`: the inputs' value as of `m` may differ from as of `m - 1`. Close
    /// every range that reached `m`. Returns whether any did.
    fn close(&mut self, m: Tick) -> bool {
        let mut changed = false;
        for slot in &mut self.slots {
            if let Some(s) = slot {
                if s.until >= m {
                    changed = true;
                    if s.from >= m {
                        *slot = None;
                    } else {
                        s.until = m - 1;
                    }
                }
            }
        }
        changed
    }
    /// The most recently computed value, whatever its range.
    fn latest(&self) -> Option<Value> {
        self.slots.iter().flatten().max_by_key(|s| s.from).map(|s| s.value.clone())
    }
    fn values(&self) -> impl Iterator<Item = &Value> {
        self.slots.iter().flatten().map(|s| &s.value)
    }
}

// ---------------------------------------------------------------------------------------
// Nodes
// ---------------------------------------------------------------------------------------

/// What a node is. Events first, then dynamics, then the structural nodes a widget makes
/// (places and builders), whose data lives in `document` and `widget`.
#[derive(Clone)]
pub(crate) enum Kind {
    // ----- events -----
    /// `never`: fires in no tick. A dead widget's own events become this (Semantics
    /// ch. 7, "Holds die with their widget").
    Never,
    /// An element's event answer: fired by the tick's answers, addressed by path.
    Input,
    /// `@started`: fired at the opening of the tick after its widget was built (R7).
    Started,
    /// A forward reference inside a widget block (a binding group, Semantics ch. 4), set
    /// once the bound event exists.
    Pass { src: Option<NodeId> },
    /// A lift over one event with no dynamic operand: `f(v)`.
    Map { src: NodeId, f: Closure },
    /// A lift over an event with dynamic operands, `$count + 1 on !inc`: the dynamics are
    /// sampled as the tick opened (the tag law, R2).
    Sample { src: NodeId, dyns: Vec<NodeId>, f: Closure },
    /// `!Filter: fn !?a -> !a`.
    Filter { src: NodeId },
    /// `!Present: fn !?a -> !unit` (doubt PZ: it drops the value).
    Present { src: NodeId },
    /// `<|`, the leftmost merge: the first input that fired wins (round 46 MO; D13).
    Merge { srcs: Vec<NodeId> },
    /// Several events combined by a function, in one tick: when any input fires, `f`
    /// gets `? v` or `nil` per input, and the node fires with what `f` gives if it gives
    /// `? r`. The prelude's `!Union`, `!MergeAll`, `!MergeWith`, `!Both`, `!Except` and
    /// `!Align` are each one of these.
    Combine { srcs: Vec<NodeId>, f: Closure },
    /// `!Gate: fn $Bool, !a -> !a`: the condition is sampled as the tick opened.
    Gate { cond: NodeId, src: NodeId },
    /// `!Updated: fn $a -> !a`: a dynamic's update event.
    Updated { dynamic: NodeId },
    /// A keyed `for`'s consolidation over rows that end on an event: `!(K ==> T)`,
    /// firing with exactly the rows that fired (Semantics ch. 4, `for`).
    MergeRows { rows: OrdMap<Value, NodeId> },
    /// `!e <- @Switch $!d` (round 47 SW): fires with the inner the dynamic held as the
    /// tick opened, and adopts the new inner at the end of the tick.
    Switch { outer: NodeId, inner: NodeId },
    /// `@Machine: fn (fn s, a -> ((s, ?b))), s, !a -> @!b`: a hold of `s`, firing `b`.
    Machine { src: NodeId, step: Closure, state: Cell2 },
    /// The input half of a `@Defer`: arms its output for the next tick.
    DeferIn { src: NodeId, out: NodeId },
    /// The output half of a `@Defer`: a source that fires the armed value at the opening
    /// of the tick it was armed for (decisions.md D6).
    DeferOut { armed: Option<Value>, armed_for: Tick },

    // ----- dynamics -----
    /// A constant: what a hold becomes when its widget dies, or a plain value used where
    /// a dynamic goes (`$ v`, the free sigil).
    Const { value: Value },
    /// `$x <- init then !e`: written with each firing of its event.
    Hold { src: Option<NodeId>, cell: Cell2 },
    /// A place's dynamic answer: a first value, then pushes keyed by element value.
    External { cell: Cell2 },
    /// A dynamic written only at the end of a tick, by construction: a `rebuild`'s result
    /// (round 46 RD), a `for`'s rows, a host document's child document. Its update event
    /// is heard by state only (doubt RE, Claude's first ruling): nothing fires.
    Written { cell: Cell2 },
    /// A lift over dynamics (the change law): `f(inputs)`, recomputed when marked.
    Lift { inputs: Vec<NodeId>, f: Closure, cache: Cache2 },
    /// One row's value in a keyed `for` over a dynamic map or array: `$m[key]`. When
    /// the key has just left (the row runs to the end of its tick, R8), it keeps its
    /// last value.
    MapEntry { src: NodeId, key: Value, cache: Cache2 },
    /// A `for` whose rows end on dynamics, gathered: `$(K ==> T)`. Reads its rows map
    /// and each row's dynamic.
    ForDyn { rows: NodeId, inputs: Vec<NodeId>, cache: Cache2 },
    /// `$v <- @Flatten $$d`: a hold written by its current inner's update event; adopts a
    /// new inner at the end of a tick and moves to it in the next one (round 47 SW).
    Flatten { outer: NodeId, inner: NodeId, cell: Cell2, adopting: bool },

    // ----- structure -----
    /// A place: an element at a path in a document.
    Place(Box<PlaceData>),
    Rebuild(Box<RebuildData>),
    For(Box<ForData>),
    Host(Box<HostData>),
}

impl Kind {
    /// The inputs this node depends on promptly: the edges heights follow, and the ones
    /// it subscribes to while active.
    pub fn prompt_inputs(&self) -> Vec<NodeId> {
        match self {
            Kind::Pass { src: Some(s) } => vec![*s],
            Kind::Map { src, .. }
            | Kind::Sample { src, .. }
            | Kind::Filter { src }
            | Kind::Present { src }
            | Kind::Gate { src, .. }
            | Kind::Machine { src, .. }
            | Kind::DeferIn { src, .. }
            | Kind::MapEntry { src, .. } => vec![*src],
            Kind::Merge { srcs } | Kind::Combine { srcs, .. } => srcs.clone(),
            Kind::Updated { dynamic } => vec![*dynamic],
            Kind::MergeRows { rows } => rows.values().copied().collect(),
            Kind::Switch { inner, .. } => vec![*inner],
            Kind::Hold { src: Some(s), .. } => vec![*s],
            Kind::Lift { inputs, .. } => inputs.clone(),
            Kind::ForDyn { inputs, .. } => inputs.clone(),
            Kind::Flatten { inner, .. } => vec![*inner],
            Kind::Place(p) => p.requests.iter().map(|(_, e)| *e).collect(),
            _ => vec![],
        }
    }

    /// The dynamics this node reads and must hear marks from (its invalidation edges,
    /// Graph ch. 4).
    pub fn read_inputs(&self) -> Vec<NodeId> {
        match self {
            Kind::Lift { inputs, .. } => inputs.clone(),
            Kind::MapEntry { src, .. } => vec![*src],
            Kind::ForDyn { rows, inputs, .. } => {
                let mut v = vec![*rows];
                v.extend(inputs.iter().copied());
                v
            }
            Kind::Switch { outer, .. } | Kind::Flatten { outer, .. } => vec![*outer],
            Kind::Place(p) => p.live.clone(),
            Kind::Rebuild(r) => vec![r.src],
            Kind::For(f) => vec![f.src],
            _ => vec![],
        }
    }

    /// Every node id this node refers to, for the collector.
    pub fn references(&self) -> Vec<NodeId> {
        let mut v = self.prompt_inputs();
        v.extend(self.read_inputs());
        match self {
            Kind::Sample { dyns, .. } => v.extend(dyns.iter().copied()),
            Kind::Gate { cond, .. } => v.push(*cond),
            Kind::DeferIn { out, .. } => v.push(*out),
            Kind::ForDyn { rows, .. } => v.push(*rows),
            Kind::Rebuild(r) => v.push(r.result),
            Kind::For(f) => v.extend(f.result_nodes()),
            Kind::Host(h) => v.push(h.doc),
            _ => {}
        }
        v
    }

    /// Every value this node holds, for the collector.
    pub fn values(&self) -> Vec<&Value> {
        let mut out: Vec<&Value> = Vec::new();
        match self {
            Kind::Map { f, .. } | Kind::Sample { f, .. } | Kind::Lift { f, .. } | Kind::Combine { f, .. } => {
                out.extend(f.captured().iter());
            }
            Kind::Machine { step, state, .. } => {
                out.extend(step.captured().iter());
                out.extend(state.values());
            }
            Kind::DeferOut { armed: Some(v), .. } => out.push(v),
            Kind::Const { value } => out.push(value),
            Kind::Hold { cell, .. } | Kind::External { cell } | Kind::Written { cell } => {
                out.extend(cell.values());
            }
            Kind::Flatten { cell, .. } => out.extend(cell.values()),
            _ => {}
        }
        match self {
            Kind::Lift { cache, .. } | Kind::MapEntry { cache, .. } | Kind::ForDyn { cache, .. } => {
                out.extend(cache.values());
            }
            _ => {}
        }
        if let Kind::MapEntry { key, .. } = self {
            out.push(key);
        }
        match self {
            Kind::Place(p) => out.extend(p.values()),
            Kind::Rebuild(r) => out.extend(r.values()),
            Kind::For(f) => out.extend(f.values()),
            Kind::Host(h) => out.extend(h.values()),
            _ => {}
        }
        out
    }

    /// A sink always subscribes to its writers while it lives (G4).
    fn is_sink(&self) -> bool {
        matches!(
            self,
            Kind::Hold { .. } | Kind::Machine { .. } | Kind::DeferIn { .. } | Kind::Flatten { .. } | Kind::Place(_)
        )
    }
}

#[derive(Clone)]
pub(crate) struct Node {
    pub kind: Kind,
    pub height: u32,
    /// The widget whose construction made this node, while that widget lives.
    pub owner: Option<WidgetId>,
    /// Consumers subscribed to this node's firings (weak).
    pub subscribers: Vec<NodeId>,
    /// Consumers that read this dynamic and hear its marks (weak).
    pub readers: Vec<NodeId>,
    /// Demand from outside the graph: the root's result.
    pub pins: u32,
    /// Whether this node is subscribed to its prompt inputs.
    pub active: bool,
    pub fired_at: Tick,
    pub fired: Value,
    pub marked_at: Tick,
}

impl Node {
    fn demanded(&self) -> bool {
        self.pins > 0 || !self.subscribers.is_empty() || self.kind.is_sink()
    }
}

/// The lists a tick fills and empties (Graph ch. 4, "A tick, in order"). Empty between
/// ticks, so a kept state carries none of them.
#[derive(Clone, Default)]
pub(crate) struct TickLists {
    /// Nodes to evaluate, by height.
    pub queue: BTreeSet<(u32, NodeId)>,
    /// Builders whose dynamic was marked: candidates for the build list.
    pub builders: BTreeSet<NodeId>,
    /// Switches and flattens whose outer was marked: the swap list.
    pub swaps: BTreeSet<NodeId>,
    /// Places with a live field marked: candidates for `changed:`.
    pub dirty: BTreeSet<NodeId>,
    /// Requests that fired, in firing order; sorted into document order at the end of
    /// propagation.
    pub requests: Vec<Request>,
}

/// The graph of one program: an arena of nodes, the tick counter, and the tick's lists.
#[derive(Clone, Default)]
pub(crate) struct Graph {
    nodes: Vector<Option<Node>>,
    free: Vector<u32>,
    /// The tick being run, or the last tick run between ticks.
    pub now: Tick,
    pub lists: TickLists,
}

impl Graph {
    // ----- the arena -----

    pub fn node(&self, id: NodeId) -> &Node {
        self.nodes[id.0 as usize].as_ref().expect("a node the collector freed was used")
    }
    pub fn node_mut(&mut self, id: NodeId) -> &mut Node {
        self.nodes
            .get_mut(id.0 as usize)
            .and_then(|n| n.as_mut())
            .expect("a node the collector freed was used")
    }
    pub fn exists(&self, id: NodeId) -> bool {
        self.nodes.get(id.0 as usize).is_some_and(|n| n.is_some())
    }
    pub fn kind(&self, id: NodeId) -> &Kind {
        &self.node(id).kind
    }
    pub fn kind_mut(&mut self, id: NodeId) -> &mut Kind {
        &mut self.node_mut(id).kind
    }
    /// The number of live cells in the arena (the debug allocator's count, D5).
    pub fn live_nodes(&self) -> usize {
        self.nodes.len() - self.free.len()
    }
    pub fn ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.iter().enumerate().filter(|(_, n)| n.is_some()).map(|(i, _)| NodeId(i as u32))
    }
    /// Free a node's cell. The collector calls this only on nodes nothing reaches.
    pub fn free(&mut self, id: NodeId) {
        self.nodes.set(id.0 as usize, None);
        self.free.push_back(id.0);
    }

    /// Make a node. Its readers' edges are registered and, if it is a sink, it subscribes
    /// to its inputs at once.
    pub fn add(&mut self, kind: Kind, owner: Option<WidgetId>) -> NodeId {
        let reads = kind.read_inputs();
        let sink = kind.is_sink();
        let node = Node {
            kind,
            height: 0,
            owner,
            subscribers: Vec::new(),
            readers: Vec::new(),
            pins: 0,
            active: false,
            fired_at: -2,
            fired: Value::Unit,
            marked_at: -2,
        };
        let id = match self.free.pop_back() {
            Some(i) => {
                self.nodes.set(i as usize, Some(node));
                NodeId(i)
            }
            None => {
                self.nodes.push_back(Some(node));
                NodeId(self.nodes.len() as u32 - 1)
            }
        };
        for r in reads {
            self.add_reader(r, id);
        }
        if sink {
            self.activate(id);
        }
        id
    }

    // ----- demand and heights -----

    fn subscribe(&mut self, producer: NodeId, consumer: NodeId) {
        self.node_mut(producer).subscribers.push(consumer);
        let n = self.node(producer);
        if !n.active && n.demanded() {
            self.activate(producer);
        }
    }

    pub fn unsubscribe(&mut self, producer: NodeId, consumer: NodeId) {
        let n = self.node_mut(producer);
        if let Some(i) = n.subscribers.iter().position(|c| *c == consumer) {
            n.subscribers.swap_remove(i);
        }
        let n = self.node(producer);
        if n.active && !n.demanded() {
            self.deactivate(producer);
        }
    }

    /// Demand from outside the graph (the root's result, a World's interest).
    pub fn pin(&mut self, id: NodeId) {
        self.node_mut(id).pins += 1;
        if !self.node(id).active {
            self.activate(id);
        }
    }

    fn activate(&mut self, id: NodeId) {
        self.node_mut(id).active = true;
        let inputs = self.kind(id).prompt_inputs();
        let mut h = 0;
        for i in inputs {
            self.subscribe(i, id);
            h = h.max(self.node(i).height + 1);
        }
        self.raise(id, h);
    }

    fn deactivate(&mut self, id: NodeId) {
        self.node_mut(id).active = false;
        for i in self.kind(id).prompt_inputs() {
            self.unsubscribe(i, id);
        }
    }

    /// Raise a node's height to at least `h`, and its subscribers' above it. Heights are
    /// never lowered, since one too high is harmless (Graph ch. 4, "Heights").
    fn raise(&mut self, id: NodeId, h: u32) {
        if self.node(id).height >= h {
            return;
        }
        assert!(
            (h as usize) <= self.nodes.len() + 1,
            "a prompt cycle reached the graph; the checker refuses these (Semantics ch. 7, R4)"
        );
        self.node_mut(id).height = h;
        for s in self.node(id).subscribers.clone() {
            self.raise(s, h + 1);
        }
    }

    pub fn add_reader(&mut self, producer: NodeId, reader: NodeId) {
        self.node_mut(producer).readers.push(reader);
    }

    pub fn remove_reader(&mut self, producer: NodeId, reader: NodeId) {
        let n = self.node_mut(producer);
        if let Some(i) = n.readers.iter().position(|r| *r == reader) {
            n.readers.swap_remove(i);
        }
    }

    /// Change a node's inputs in place, keeping its subscriptions and readers' edges in
    /// step: new inputs are subscribed before old ones are let go, so demand never dips
    /// (Graph ch. 4, "Swap").
    pub fn rewire(&mut self, id: NodeId, change: impl FnOnce(&mut Kind)) {
        let (old_p, old_r) = (self.kind(id).prompt_inputs(), self.kind(id).read_inputs());
        change(self.kind_mut(id));
        let (new_p, new_r) = (self.kind(id).prompt_inputs(), self.kind(id).read_inputs());
        for r in new_r.iter().filter(|r| !old_r.contains(r)) {
            self.add_reader(*r, id);
        }
        for r in old_r.iter().filter(|r| !new_r.contains(r)) {
            self.remove_reader(*r, id);
        }
        if self.node(id).active {
            let mut h = 0;
            for p in new_p.iter().filter(|p| !old_p.contains(p)) {
                self.subscribe(*p, id);
            }
            for p in old_p.iter().filter(|p| !new_p.contains(p)) {
                self.unsubscribe(*p, id);
            }
            for p in &new_p {
                h = h.max(self.node(*p).height + 1);
            }
            self.raise(id, h);
        } else if self.node(id).demanded() {
            self.activate(id);
        }
    }

    /// Replace a node's kind for good, as a widget's leaving does (Semantics ch. 7, "Holds
    /// die with their widget"): its subscriptions and readers' edges are let go.
    pub fn retire(&mut self, id: NodeId, kind: Kind) {
        if self.node(id).active {
            self.deactivate(id);
        }
        for r in self.kind(id).read_inputs() {
            self.remove_reader(r, id);
        }
        let n = self.node_mut(id);
        n.kind = kind;
        n.owner = None;
        n.active = false;
        if n.pins > 0 {
            self.activate(id);
        }
    }

    // ----- marks -----

    /// A dynamic's value as of `m` may differ from its value as of `m - 1`: tell its
    /// readers. Caches close their ranges; places go on the dirty list, builders on the
    /// build list, switches and flattens on the swap list (Graph ch. 4, steps 2 and 5).
    pub fn mark(&mut self, id: NodeId, m: Tick) {
        for r in self.node(id).readers.clone() {
            if !self.exists(r) {
                continue;
            }
            let propagate = {
                let node = self.node_mut(r);
                let closed = match &mut node.kind {
                    Kind::Lift { cache, .. } | Kind::MapEntry { cache, .. } | Kind::ForDyn { cache, .. } => {
                        Some(cache.close(m))
                    }
                    _ => None,
                };
                match closed {
                    Some(changed) => {
                        let first = node.marked_at < m;
                        node.marked_at = m;
                        changed || first
                    }
                    None => false,
                }
            };
            match self.kind(r) {
                Kind::Place(_) => {
                    self.lists.dirty.insert(r);
                }
                Kind::Rebuild(_) | Kind::For(_) => {
                    self.lists.builders.insert(r);
                }
                Kind::Switch { outer, .. } | Kind::Flatten { outer, .. } if *outer == id => {
                    self.lists.swaps.insert(r);
                }
                _ => {}
            }
            if propagate {
                self.mark(r, m);
            }
        }
    }

    /// Write a hold-like cell at the current tick and mark its readers.
    pub fn write_cell(&mut self, id: NodeId, v: Value) {
        let now = self.now;
        match self.kind_mut(id) {
            Kind::Hold { cell, .. } | Kind::External { cell } | Kind::Written { cell } | Kind::Flatten { cell, .. } => {
                cell.write(v, now);
            }
            _ => panic!("write_cell on a node with no cell"),
        }
        self.mark(id, now);
    }

    // ----- reads -----

    /// A dynamic's value as of tick `asof`, and the last tick that value is known valid
    /// for. During propagation of tick `n` a sample reads as of `n - 1` (R2); from the
    /// end of propagation on, every read is as of `n`.
    pub fn read(&mut self, id: NodeId, asof: Tick) -> Result<(Value, Tick), Fault> {
        match &self.node(id).kind {
            Kind::Const { value } => Ok((value.clone(), FOREVER)),
            Kind::Hold { cell, .. } | Kind::External { cell } | Kind::Written { cell } | Kind::Flatten { cell, .. } => {
                Ok(cell.read(asof))
            }
            Kind::Lift { inputs, f, cache } => {
                if let Some(hit) = cache.get(asof) {
                    return Ok(hit);
                }
                let (inputs, f) = (inputs.clone(), f.clone());
                let mut vals = Vec::with_capacity(inputs.len());
                let mut until = FOREVER;
                for i in inputs {
                    let (v, u) = self.read(i, asof)?;
                    vals.push(v);
                    until = until.min(u);
                }
                let v = f.call(&vals)?;
                self.put_cache(id, v.clone(), asof, until);
                Ok((v, until))
            }
            Kind::MapEntry { src, key, cache } => {
                if let Some(hit) = cache.get(asof) {
                    return Ok(hit);
                }
                let (src, key, fallback) = (*src, key.clone(), cache.latest());
                let (m, until) = self.read(src, asof)?;
                let v = lookup(&m, &key).or(fallback).unwrap_or(Value::Unit);
                self.put_cache(id, v.clone(), asof, until);
                Ok((v, until))
            }
            Kind::ForDyn { rows, cache, .. } => {
                if let Some(hit) = cache.get(asof) {
                    return Ok(hit);
                }
                let rows = *rows;
                let (rm, mut until) = self.read(rows, asof)?;
                let mut out = OrdMap::new();
                if let Value::Map(rm) = rm {
                    for (k, v) in rm.iter() {
                        let v = match v {
                            Value::Dynamic(d) => {
                                let (v, u) = self.read(*d, asof)?;
                                until = until.min(u);
                                v
                            }
                            plain => plain.clone(),
                        };
                        out.insert(k.clone(), v);
                    }
                }
                let v = Value::Map(out);
                self.put_cache(id, v.clone(), asof, until);
                Ok((v, until))
            }
            _ => Err(Fault::runtime("a sample of something that is not a dynamic")),
        }
    }

    fn put_cache(&mut self, id: NodeId, v: Value, from: Tick, until: Tick) {
        if let Kind::Lift { cache, .. } | Kind::MapEntry { cache, .. } | Kind::ForDyn { cache, .. } = self.kind_mut(id)
        {
            cache.put(v, from, until);
        }
    }

    /// The value after the current tick: the read construction and the patch pass make.
    pub fn read_now(&mut self, id: NodeId) -> Result<Value, Fault> {
        let now = self.now;
        Ok(self.read(id, now)?.0)
    }

    // ----- firing -----

    /// This node's firing in the current tick, if it fired.
    pub fn fired(&self, id: NodeId) -> Option<Value> {
        let n = self.node(id);
        if n.fired_at == self.now { Some(n.fired.clone()) } else { None }
    }

    /// Record a firing and wake the subscribers at their heights.
    pub fn fire(&mut self, id: NodeId, v: Value) {
        let now = self.now;
        let n = self.node_mut(id);
        debug_assert!(n.fired_at != now, "an event fired twice in one tick (R5)");
        n.fired_at = now;
        n.fired = v;
        for s in self.node(id).subscribers.clone() {
            let h = self.node(s).height;
            self.lists.queue.insert((h, s));
        }
    }

    /// Wake a node at its own height (a flatten that adopted, at the next tick).
    pub fn wake(&mut self, id: NodeId) {
        let h = self.node(id).height;
        self.lists.queue.insert((h, id));
    }

    /// Run heights in order until nothing is left to wake (Graph ch. 4, step 2).
    pub fn propagate(&mut self) -> Result<(), Fault> {
        while let Some((_, id)) = self.lists.queue.pop_first() {
            if self.exists(id) {
                self.evaluate(id)?;
            }
        }
        Ok(())
    }

    /// Evaluate one node at its height: every input that can make it fire has had its
    /// turn.
    fn evaluate(&mut self, id: NodeId) -> Result<(), Fault> {
        let now = self.now;
        let before = now - 1;
        let kind = self.node(id).kind.clone();
        let firing: Option<Value> = match kind {
            Kind::Pass { src: Some(s) } => self.fired(s),
            Kind::Map { src, f } => match self.fired(src) {
                Some(v) => Some(f.call(&[v])?),
                None => None,
            },
            Kind::Sample { src, dyns, f } => match self.fired(src) {
                Some(v) => {
                    let mut args = vec![v];
                    for d in dyns {
                        args.push(self.read(d, before)?.0);
                    }
                    Some(f.call(&args)?)
                }
                None => None,
            },
            Kind::Filter { src } => match self.fired(src) {
                Some(Value::Opt(Some(x))) => Some((*x).clone()),
                _ => None,
            },
            Kind::Present { src } => match self.fired(src) {
                Some(Value::Opt(Some(_))) => Some(Value::Unit),
                _ => None,
            },
            Kind::Merge { srcs } => srcs.iter().find_map(|s| self.fired(*s)),
            Kind::Combine { srcs, f } => {
                let args: Vec<Value> =
                    srcs.iter().map(|s| self.fired(*s).map(Value::some).unwrap_or(Value::nil())).collect();
                if args.iter().all(|a| *a == Value::nil()) {
                    None
                } else {
                    match f.call(&args)? {
                        Value::Opt(Some(r)) => Some((*r).clone()),
                        Value::Opt(None) => None,
                        _ => return Err(Fault::runtime("a combining function returns ? r or nil")),
                    }
                }
            }
            Kind::Gate { cond, src } => match self.fired(src) {
                Some(v) if self.read(cond, before)?.0 == Value::Bool(true) => Some(v),
                _ => None,
            },
            Kind::Updated { dynamic } => self.fired(dynamic),
            Kind::MergeRows { rows } => {
                let fired: OrdMap<Value, Value> =
                    rows.iter().filter_map(|(k, e)| self.fired(*e).map(|v| (k.clone(), v))).collect();
                if fired.is_empty() { None } else { Some(Value::Map(fired)) }
            }
            Kind::Switch { inner, .. } => self.fired(inner),
            Kind::Machine { src, step, state } => match self.fired(src) {
                Some(a) => {
                    let s = state.read(before).0;
                    let r = step.call(&[s, a])?;
                    let Value::Tuple(pair) = r else {
                        return Err(Fault::runtime("a machine's step returns ((state, ?output))"));
                    };
                    if pair.len() != 2 {
                        return Err(Fault::runtime("a machine's step returns ((state, ?output))"));
                    }
                    if let Kind::Machine { state, .. } = self.kind_mut(id) {
                        state.write(pair[0].clone(), now);
                    }
                    match &pair[1] {
                        Value::Opt(Some(b)) => Some((**b).clone()),
                        Value::Opt(None) => None,
                        _ => return Err(Fault::runtime("a machine's output is a ?value")),
                    }
                }
                None => None,
            },
            Kind::DeferIn { src, out } => {
                if let Some(v) = self.fired(src) {
                    if let Kind::DeferOut { armed, armed_for } = self.kind_mut(out) {
                        *armed = Some(v);
                        *armed_for = now + 1;
                    }
                }
                None
            }
            Kind::Hold { src: Some(s), cell } => match self.fired(s) {
                Some(v) if v != cell.read(before).0 => {
                    self.write_cell(id, v.clone());
                    Some(v)
                }
                _ => None,
            },
            Kind::Lift { .. } | Kind::MapEntry { .. } | Kind::ForDyn { .. } => {
                let new = self.read(id, now)?.0;
                let old = self.read(id, before)?.0;
                if new != old { Some(new) } else { None }
            }
            Kind::Flatten { inner, adopting, cell, .. } => {
                let candidate = match self.fired(inner) {
                    Some(v) => Some(v),
                    None if adopting => Some(self.read(inner, before)?.0),
                    None => None,
                };
                if let Kind::Flatten { adopting, .. } = self.kind_mut(id) {
                    *adopting = false;
                }
                match candidate {
                    Some(v) if v != cell.read(before).0 => {
                        self.write_cell(id, v.clone());
                        Some(v)
                    }
                    _ => None,
                }
            }
            Kind::Place(p) => {
                for (field, ev) in &p.requests {
                    if let Some(v) = self.fired(*ev) {
                        self.lists.requests.push(Request {
                            path: p.path.clone(),
                            tag: p.element.tag.clone(),
                            field: field.clone(),
                            several: p.request_fields > 1,
                            value: v,
                        });
                    }
                }
                None
            }
            _ => None,
        };
        if let Some(v) = firing {
            self.fire(id, v);
        }
        Ok(())
    }
}

/// `m[k]` for a map, or `xs[i]` for an array.
fn lookup(m: &Value, key: &Value) -> Option<Value> {
    match m {
        Value::Map(m) => m.get(key).cloned(),
        Value::Array(a) => key.as_i128().and_then(|i| usize::try_from(i).ok()).and_then(|i| a.get(i).cloned()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add1() -> Closure {
        Closure::pure("add1", |a| match &a[0] {
            Value::Nat(n) => Ok(Value::Nat(n + 1)),
            _ => Err(Fault::runtime("nat")),
        })
    }

    #[test]
    fn cell_reads_before_and_after_a_write() {
        let mut c = Cell2::new(Value::nat(0), 0);
        c.write(Value::nat(1), 3);
        assert_eq!(c.read(2), (Value::nat(0), 2));
        assert_eq!(c.read(3).0, Value::nat(1));
        assert_eq!(c.read(9).0, Value::nat(1));
    }

    #[test]
    fn cache_close_drops_and_shortens() {
        let mut c = Cache2::default();
        c.put(Value::nat(1), 0, FOREVER);
        assert!(c.close(4));
        assert_eq!(c.get(3), Some((Value::nat(1), 3)));
        assert_eq!(c.get(4), None);
        assert!(!c.close(4));
    }

    #[test]
    fn hold_write_marks_a_lift_and_the_tag_law_holds() {
        let mut g = Graph::default();
        let input = g.add(Kind::Input, None);
        let hold = g.add(Kind::Hold { src: Some(input), cell: Cell2::new(Value::nat(0), 0) }, None);
        let lift = g.add(Kind::Lift { inputs: vec![hold], f: add1(), cache: Cache2::default() }, None);
        assert_eq!(g.read(lift, 0).unwrap().0, Value::nat(1));
        g.now = 1;
        g.fire(input, Value::nat(5));
        g.propagate().unwrap();
        // During the tick a sample reads the opening; after it, the new value.
        assert_eq!(g.read(lift, 0).unwrap().0, Value::nat(1));
        assert_eq!(g.read(lift, 1).unwrap().0, Value::nat(6));
    }

    #[test]
    fn an_unsubscribed_map_never_runs() {
        use std::cell::Cell;
        use std::rc::Rc;
        let runs = Rc::new(Cell::new(0));
        let r = runs.clone();
        let mut g = Graph::default();
        let input = g.add(Kind::Input, None);
        let f = Closure::pure("count", move |a| {
            r.set(r.get() + 1);
            Ok(a[0].clone())
        });
        let _map = g.add(Kind::Map { src: input, f }, None);
        g.now = 1;
        g.fire(input, Value::Unit);
        g.propagate().unwrap();
        assert_eq!(runs.get(), 0, "nothing runs that nothing needs");
    }

    #[test]
    fn heights_order_a_diamond() {
        let mut g = Graph::default();
        let input = g.add(Kind::Input, None);
        let a = g.add(Kind::Map { src: input, f: add1() }, None);
        let b = g.add(Kind::Map { src: a, f: add1() }, None);
        let m = g.add(Kind::Merge { srcs: vec![b, a] }, None);
        let hold = g.add(Kind::Hold { src: Some(m), cell: Cell2::new(Value::nat(0), 0) }, None);
        assert!(g.node(hold).height > g.node(m).height);
        assert!(g.node(m).height > g.node(b).height);
        g.now = 1;
        g.fire(input, Value::nat(1));
        g.propagate().unwrap();
        // The leftmost operand, `b`, wins.
        assert_eq!(g.read(hold, 1).unwrap().0, Value::nat(3));
    }
}
