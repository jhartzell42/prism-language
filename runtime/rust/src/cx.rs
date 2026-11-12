//! The tick's skeleton: the [`Node`] interface emitted nodes implement, the context
//! [`Cx`] their methods take, and the [`Runtime`] that keeps what a tick leaves for the
//! next. Emitted code decides everything a node does; the runtime keeps the lists and
//! walks them in Graph's order (Graph ch. 4, "A tick, in order"):
//!
//! 1. last round's undone clean-ups; the started list and the armed `@Defer`s fire; the
//!    answers a host's supplier computed last tick and the publications of the structural
//!    dynamics whose snapshot moved last tick ([`Cx::answer_later`]), then the World's
//!    answers by path and its pushes by element;
//! 2. the wakes by height, a bucket per height, each node once per height;
//! 3. propagation ends, and every read from here on is after the tick;
//! 4. the build list (a builder drops what left, then builds); 5. the swap list; 6. the
//!    detached widgets leave; 7. the patch pass;
//! 8. the outputs: the patch diffed against the document last sent, the requests in
//!    document order, the result, and `pending`.
//!
//! Then the compact round, [`Runtime::compact`]: the clean-ups, then the zombie collector.

use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::rc::{Rc, Weak};

use crate::cells::Tick;
use crate::doc::{Document, Edit, Request};
use crate::gc::Collector;
use crate::pod::{Fault, Path, Pod, Segment};
use crate::world::probe::{self, Phase as Mark};
use crate::world::{Outcome, ReadyState, TickIn, TickOut};

/// What a node method returns: a fault escaping it fails the tick.
pub type Done = Result<(), Fault>;

/// A node: one widget instance's storage and the methods its block compiles to. Each
/// method defaults to nothing, so a node implements what its block needs; each takes
/// `self: Rc<Self>` so the node can put itself on a list.
#[allow(unused_variables)]
pub trait Node: 'static {
    /// A wake at the height the node asked for: its internal combinators decide, once.
    fn wake(self: Rc<Self>, cx: &mut Cx) -> Done { Ok(()) }
    /// `@started`, in the tick after the build.
    fn started(self: Rc<Self>, cx: &mut Cx) -> Done { Ok(()) }
    /// An `@Defer` this node armed last tick fires.
    fn deferred(self: Rc<Self>, cx: &mut Cx) -> Done { Ok(()) }
    /// An answer to the place at `at`, relative to this node: the path index is emitted
    /// dispatch, each node matching its own positions, generations and keys.
    fn answer(self: Rc<Self>, at: &[Segment], v: &Pod, cx: &mut Cx) -> Done {
        Err(Fault::answer(format!("no place at {}", Path(at.to_vec()))))
    }
    /// A push to a place this node registered with [`Cx::listen`].
    fn push(self: Rc<Self>, element: &Pod, v: &Pod, cx: &mut Cx) -> Done { Ok(()) }
    /// The build list: drop what left, then build from the values after the tick.
    fn build(self: Rc<Self>, cx: &mut Cx) -> Done { Ok(()) }
    /// The swap list: a switch or flatten takes its new inner.
    fn swap(self: Rc<Self>, cx: &mut Cx) -> Done { Ok(()) }
    /// The detached widgets' leaving: write the zombie or the dead state.
    fn leave(self: Rc<Self>, cx: &mut Cx) -> Done { Ok(()) }
    /// The patch pass: `Changed` for each place whose live fields a mark reached.
    fn patch(self: Rc<Self>, cx: &mut Cx) -> Done { Ok(()) }
    /// The compact round, after a death or a zombie write: unsubscribe what the new state
    /// no longer reads, release what dead fields held, split off live paths.
    fn clean_up(self: Rc<Self>) {}
    /// The nodes this node's fields hold strongly: what the zombie collector follows.
    fn edges(&self, visit: &mut dyn FnMut(&Rc<dyn Node>)) {}
    /// The collector found this node garbage: let go of every value held.
    fn release(&self) {}
}

/// A list of nodes, each at most once until the walk passes it: a node added again after
/// its turn is walked again (a builder marked by a row built in the same walk).
#[derive(Default)]
struct List {
    nodes: Vec<Rc<dyn Node>>,
    seen: HashMap<*const (), usize>,
    walked: usize,
}

impl List {
    fn add(&mut self, n: Rc<dyn Node>) {
        let key = Rc::as_ptr(&n) as *const ();
        if self.seen.get(&key).is_some_and(|&at| at >= self.walked) {
            return;
        }
        self.seen.insert(key, self.nodes.len());
        self.nodes.push(n);
    }
    /// Walk a list, including what is added to it while walking.
    fn walk<'w>(cx: &mut Cx<'w>, pick: for<'a> fn(&'a mut Cx<'w>) -> &'a mut List, f: fn(Rc<dyn Node>, &mut Cx<'w>) -> Done) -> Done {
        let mut i = pick(cx).walked;
        while let Some(n) = pick(cx).nodes.get(i).cloned() {
            pick(cx).walked = i + 1;
            f(n, cx)?;
            i += 1;
        }
        Ok(())
    }
}

/// What a tick gathers as it runs.
#[derive(Default)]
struct Gathered {
    heights: Vec<List>,
    builds: List,
    swaps: List,
    leaves: List,
    patches: List,
    started: List,
    armed: List,
    edits: Vec<Edit>,
    requests: Vec<Request>,
    results: Vec<Outcome>,
    listens: Vec<(Pod, Weak<dyn Node>)>,
    zombies: Vec<Weak<dyn Node>>,
    cleanups: Vec<Rc<dyn Node>>,
    /// What this tick leaves for the start of the next, which is owed: the answers a host's
    /// supplier computed, each the typed delivery into the child's place (hosting,
    /// [`crate::host`]), and the publications of structural dynamics whose snapshot moved
    /// ([`crate::cells::Snapshot`]).
    later: Vec<Later>,
    /// A fault raised where no `Done` goes back (a row leaving inside a builder's walk):
    /// the tick fails at its close.
    fault: Option<Fault>,
}

/// A firing waiting for the start of the next tick: a closure over the node method that
/// fires (a computed answer into a child's place, a structural dynamic's publication).
pub type Later = Box<dyn FnOnce(&mut Cx) -> Done>;

/// Where a tick stands: propagating (its firings and wakes), or constructing (the walks
/// after propagation and the patch pass). The build, tick 0, constructs only.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    Propagating,
    Constructing,
}

/// One tick's context: the tick and its read point, the lists, the outputs so far.
pub struct Cx<'w> {
    tick: Tick,
    /// The read point: a cell, so [`Cx::after`] can move it past the tick through a shared
    /// reference and move it back.
    read_at: Cell<Tick>,
    phase: Cell<Phase>,
    height: Option<u32>,
    ready: &'w dyn ReadyState,
    pushes: Vec<(Pod, Pod)>,
    g: Gathered,
    /// A fault a cell raises through a shared reference (a hold written twice).
    late: std::cell::RefCell<Option<Fault>>,
}

impl Cx<'_> {
    /// The tick being run: what writes are stamped with.
    pub fn tick(&self) -> Tick {
        self.tick
    }
    /// The read point: the tick during propagation (a read before it), the tick plus one
    /// from the end of propagation on (a read after it).
    pub fn read_point(&self) -> Tick {
        self.read_at.get()
    }
    /// Whether the tick is still propagating: its firings and wakes, before the construction
    /// step. A structure ([`crate::cells::Snapshot::current`]) has no value after the tick
    /// yet, so a read of one after the tick is a bug here and a value from the construction
    /// step on.
    pub fn propagating(&self) -> bool {
        self.phase.get() == Phase::Propagating
    }
    /// A read after the tick, made during propagation: `f` runs with the read point past
    /// the tick, so every hold it reads answers its value after this tick, and the read
    /// point is then put back. `!Updated $d` fires `$d`'s value after the tick in the tick
    /// of the change (Semantics R3); this is how its node computes it at its height.
    pub fn after<T>(&self, f: impl FnOnce(&Cx) -> T) -> T {
        let saved = self.read_at.replace(self.tick + 1);
        let v = f(self);
        self.read_at.set(saved);
        v
    }
    /// A read as the tick opened, made after propagation: `f` runs with the read point at
    /// the tick, so every hold it reads answers its value before this tick's writes, and the
    /// read point is then put back. A `for` row whose key left reads its entry's last value
    /// this way (Semantics R8: the row ends at the end of the tick its key leaves; the
    /// entry after that tick does not exist).
    pub fn before<T>(&self, f: impl FnOnce(&Cx) -> T) -> T {
        let saved = self.read_at.replace(self.tick);
        let v = f(self);
        self.read_at.set(saved);
        v
    }

    /// The height whose wakes run now; none outside the heights' run.
    pub fn running_height(&self) -> Option<u32> {
        self.height
    }
    /// Ask for a wake at `height`, once per height. A wake asked for at or below the
    /// running height runs at the next one (the clamp, Graph ch. 4).
    pub fn wake(&mut self, height: u32, node: Rc<dyn Node>) {
        let h = self.height.map_or(height, |run| height.max(run + 1)) as usize;
        if self.g.heights.len() <= h {
            self.g.heights.resize_with(h + 1, List::default);
        }
        self.g.heights[h].add(node);
    }
    /// A builder whose dynamic changed: it builds after propagation.
    pub fn build_later(&mut self, node: Rc<dyn Node>) {
        self.g.builds.add(node)
    }
    /// A switch or flatten whose outer was marked.
    pub fn swap_later(&mut self, node: Rc<dyn Node>) {
        self.g.swaps.add(node)
    }
    /// A widget detached this tick: it runs to the end of the tick, then leaves.
    pub fn leave_later(&mut self, node: Rc<dyn Node>) {
        self.g.leaves.add(node)
    }
    /// A node whose places' live fields a mark reached: the patch pass asks it.
    pub fn dirty(&mut self, node: Rc<dyn Node>) {
        self.g.patches.add(node)
    }
    /// A newborn that binds `@started`: it fires next tick, and that tick is owed.
    pub fn on_started(&mut self, node: Rc<dyn Node>) {
        self.g.started.add(node)
    }
    /// A node with an armed `@Defer` ([`crate::cells::Defer::arm`] calls this).
    pub fn arm(&mut self, node: Rc<dyn Node>) {
        self.g.armed.add(node)
    }
    /// A node that took its dead or zombie state: its clean-up runs in the compact
    /// round, and a zombie is watched by the collector until it dies.
    pub fn clean_up_later(&mut self, node: Rc<dyn Node>, zombie: bool) {
        if zombie {
            self.g.zombies.push(Rc::downgrade(&node));
        }
        self.g.cleanups.push(node);
    }

    /// A place built, with its level: the document takes it.
    pub fn added(&mut self, path: Path, level: Pod) {
        self.g.edits.push(Edit::Added(path, level))
    }
    /// A place torn down.
    pub fn removed(&mut self, path: Path) {
        self.g.edits.push(Edit::Removed(path))
    }
    /// A place whose live fields may have moved: the diff decides.
    pub fn changed(&mut self, path: Path, level: Pod) {
        self.g.edits.push(Edit::Changed(path, level))
    }
    /// A request field fired, with its `Edge` value; `field` names it when the tag has
    /// several.
    pub fn request(&mut self, path: Path, field: Option<&str>, value: Pod) {
        self.g.requests.push(Request { path, field: field.map(Into::into), value })
    }
    /// A firing for the start of the next tick, which is owed (like an `@Defer`): an answer
    /// a host's supplier computed (an event output's firing, a dynamic output's update),
    /// delivered to the child's place as an input of that tick (Semantics ch. 9 as ruled: a
    /// computed answer is a tick late); the publication of a structural dynamic whose
    /// snapshot moved this tick, written at n+1 as a hold is, so its update event fires at
    /// n+1 with the snapshot of n (ch. 7, round 3 ST).
    pub fn answer_later(&mut self, deliver: Later) {
        self.g.later.push(deliver)
    }
    /// A fault a cell raises where it holds only `&Cx`: the tick fails at its close.
    pub fn fail_later(&self, f: Fault) {
        self.late.borrow_mut().get_or_insert(f);
    }
    /// A fault where no `Done` goes back: the tick fails at its close.
    pub fn keep(&mut self, done: Done) {
        if let (Err(f), None) = (done, &self.g.fault) {
            self.g.fault = Some(f);
        }
    }
    /// The root's result: a plain one at the build, a dynamic one when it changes, an
    /// event one when it fires. A root that ends on a bundle tells each part by itself, so
    /// a tick holds the results in the order they were told.
    pub fn result(&mut self, outcome: Outcome) {
        self.g.results.push(outcome)
    }

    /// A new place's first value: the ready state as the tick opened, with the tick's
    /// pushes on top (Semantics ch. 9, "Queries are sync, from a ready state").
    pub fn first_value(&self, element: &Pod) -> Option<Pod> {
        let pushed = self.pushes.iter().rev().find(|(e, _)| e == element).map(|(_, v)| v.clone());
        pushed.or_else(|| self.ready.first_value(element))
    }
    /// Register a place for the pushes keyed by its element's value.
    pub fn listen(&mut self, element: Pod, node: &Rc<dyn Node>) {
        self.g.listens.push((element, Rc::downgrade(node)))
    }
}

/// What a program keeps between ticks besides its root: the tick counter, the started
/// list, the armed `@Defer`s, the computed answers due, the push index, the clean-ups, the
/// document last sent, and the collector's zombies.
#[derive(Default)]
pub struct Runtime {
    tick: Tick,
    started: Vec<Rc<dyn Node>>,
    armed: Vec<Rc<dyn Node>>,
    later: Vec<Later>,
    listeners: BTreeMap<Pod, Vec<Weak<dyn Node>>>,
    cleanups: Vec<Rc<dyn Node>>,
    document: Document,
    collector: Collector,
}

impl Runtime {
    pub fn new() -> Runtime {
        Runtime::default()
    }

    /// Tick 0: a build reads values only, all of them after the tick.
    pub fn build<'w>(&mut self, ready: &'w dyn ReadyState) -> Cx<'w> {
        Cx { tick: 0, read_at: Cell::new(1), phase: Cell::new(Phase::Constructing), height: None, ready, pushes: Vec::new(), g: Gathered::default(), late: Default::default() }
    }

    /// One tick as a World calls it: open, fire, deliver, close.
    pub fn tick(&mut self, root: &Rc<dyn Node>, input: &TickIn, ready: &dyn ReadyState) -> Result<TickOut, Fault> {
        probe::enter(Mark::Propagate);
        self.cleanups.drain(..).for_each(Node::clean_up);
        let n = self.tick + 1;
        let mut cx = Cx { tick: n, read_at: Cell::new(n), phase: Cell::new(Phase::Propagating), height: None, ready, pushes: input.pushes.clone(), g: Gathered::default(), late: Default::default() };
        for node in std::mem::take(&mut self.started) {
            node.started(&mut cx)?;
        }
        for node in std::mem::take(&mut self.armed) {
            node.deferred(&mut cx)?;
        }
        for deliver in std::mem::take(&mut self.later) {
            deliver(&mut cx)?;
        }
        for (path, v) in &input.answers {
            root.clone().answer(path.segments(), v, &mut cx)?;
        }
        for (element, v) in &input.pushes {
            let live: Vec<_> = self.listeners.get(element).into_iter().flatten().filter_map(Weak::upgrade).collect();
            for node in live {
                node.push(element, v, &mut cx)?;
            }
        }
        self.close(cx)
    }

    /// Run the heights, the walks after propagation and the patch pass; gather outputs.
    pub fn close(&mut self, mut cx: Cx) -> Result<TickOut, Fault> {
        let mut h = 0;
        while h < cx.g.heights.len() {
            cx.height = Some(h as u32);
            let mut i = 0;
            while let Some(node) = cx.g.heights[h].nodes.get(i).cloned() {
                node.wake(&mut cx)?;
                i += 1;
            }
            h += 1;
        }
        cx.height = None;
        cx.read_at.set(cx.tick + 1);
        cx.phase.set(Phase::Constructing);
        // A walk can list nodes for an earlier one (a builder's new row is patched; a row
        // built in the patch pass is built): the walks run again until nothing new is listed.
        loop {
            probe::enter(Mark::Build);
            List::walk(&mut cx, |cx| &mut cx.g.builds, |n, cx| n.build(cx))?;
            List::walk(&mut cx, |cx| &mut cx.g.swaps, |n, cx| n.swap(cx))?;
            List::walk(&mut cx, |cx| &mut cx.g.leaves, |n, cx| n.leave(cx))?;
            probe::enter(Mark::Patch);
            List::walk(&mut cx, |cx| &mut cx.g.patches, |n, cx| n.patch(cx))?;
            let g = &cx.g;
            if [&g.builds, &g.swaps, &g.leaves, &g.patches].iter().all(|l| l.walked >= l.nodes.len()) {
                break;
            }
        }

        if let Some(f) = cx.late.borrow_mut().take() {
            return Err(f);
        }
        let mut g = cx.g;
        if let Some(f) = g.fault.take() {
            return Err(f);
        }
        g.requests.sort_by(|a, b| a.path.cmp(&b.path));
        for (element, node) in g.listens {
            self.listeners.entry(element).or_default().push(node);
        }
        self.listeners.retain(|_, ns| {
            ns.retain(|n| n.strong_count() > 0);
            !ns.is_empty()
        });
        self.collector.watch(g.zombies.into_iter());
        self.cleanups.extend(g.cleanups);
        (self.started, self.armed, self.later, self.tick) = (g.started.nodes, g.armed.nodes, g.later, cx.tick);
        probe::enter(Mark::Diff);
        Ok(TickOut {
            tick: cx.tick,
            patch: self.document.diff(g.edits),
            requests: g.requests,
            results: g.results,
            pending: !self.started.is_empty() || !self.armed.is_empty() || !self.later.is_empty(),
        })
    }

    /// The compact round, between ticks: the clean-ups, then the zombie collector.
    /// Returns how many nodes the collector found garbage.
    pub fn compact(&mut self) -> usize {
        self.cleanups.drain(..).for_each(Node::clean_up);
        self.collector.collect()
    }

    /// The document as last sent.
    pub fn document(&self) -> &Document {
        &self.document
    }
}
