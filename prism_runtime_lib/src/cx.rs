//! The tick's skeleton: the [`Node`] interface emitted nodes implement, the context
//! [`Cx`] their methods take, and the [`Runtime`] that keeps what a tick leaves for the
//! next. Emitted code decides everything a node does; the runtime keeps the lists and
//! walks them in Graph's order (Graph ch. 4, "A tick, in order"):
//!
//! 1. last round's undone clean-ups; the started list and the armed `@Defer`s fire; the
//!    answers by path and the pushes by element;
//! 2. the wakes by height, a bucket per height, each node once per height;
//! 3. propagation ends, and every read from here on is after the tick;
//! 4. the build list (a builder drops what left, then builds); 5. the swap list; 6. the
//!    detached widgets leave; 7. the patch pass;
//! 8. the outputs: the patch diffed against the document last sent, the requests in
//!    document order, the result, and `pending`.
//!
//! Then the compact round, [`Runtime::compact`]: the clean-ups, then the zombie collector.

use std::collections::{BTreeMap, HashMap};
use std::rc::{Rc, Weak};

use crate::cells::Tick;
use crate::doc::{Document, Edit, Request};
use crate::gc::Collector;
use crate::pod::{Fault, Path, Pod, Segment};
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
/// its turn is walked again (a host's owner, marked by its child's patch after its own).
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
    result: Option<Outcome>,
    listens: Vec<(Pod, Weak<dyn Node>)>,
    zombies: Vec<Weak<dyn Node>>,
    cleanups: Vec<Rc<dyn Node>>,
    /// A fault raised where no `Done` goes back (a host building a supplier's blueprint
    /// for a place its child added): the tick fails at its close.
    fault: Option<Fault>,
}

/// One tick's context: the tick and its read point, the lists, the outputs so far.
pub struct Cx<'w> {
    tick: Tick,
    read_at: Tick,
    height: Option<u32>,
    ready: &'w dyn ReadyState,
    pushes: Vec<(Pod, Pod)>,
    g: Gathered,
}

impl Cx<'_> {
    /// The tick being run: what writes are stamped with.
    pub fn tick(&self) -> Tick {
        self.tick
    }
    /// The read point: the tick during propagation (a read before it), the tick plus one
    /// from the end of propagation on (a read after it).
    pub fn read_point(&self) -> Tick {
        self.read_at
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

    /// A place built, with its `Level` after the tick. A place under a host's child goes
    /// to the host, which builds its supplier's blueprint (the scope rides on the path:
    /// [`crate::host`]).
    pub fn added(&mut self, path: Path, level: Pod) {
        match crate::host::split(&path) {
            Some((host, at)) => {
                let done = host.added(at, level, self);
                self.keep(done);
            }
            None => self.g.edits.push(Edit::Added(path, level)),
        }
    }
    /// A place torn down.
    pub fn removed(&mut self, path: Path) {
        match crate::host::split(&path) {
            Some((host, at)) => host.removed(&at, self),
            None => self.g.edits.push(Edit::Removed(path)),
        }
    }
    /// A place whose live fields may have moved: the diff decides.
    pub fn changed(&mut self, path: Path, level: Pod) {
        match crate::host::split(&path) {
            Some((host, at)) => host.changed(&at, level, self),
            None => self.g.edits.push(Edit::Changed(path, level)),
        }
    }
    /// A request field fired, with its `Edge` value; `field` names it when the tag has
    /// several. A hosted child's request is the host's: it reaches the World only through
    /// a forwarding blueprint.
    pub fn request(&mut self, path: Path, field: Option<&str>, value: Pod) {
        match crate::host::split(&path) {
            Some((host, at)) => host.request(&at, field, value, self),
            None => self.g.requests.push(Request { path, field: field.map(Into::into), value }),
        }
    }
    /// A fault where no `Done` goes back: the tick fails at its close.
    pub fn keep(&mut self, done: Done) {
        if let (Err(f), None) = (done, &self.g.fault) {
            self.g.fault = Some(f);
        }
    }
    /// The root's result: a plain one at the build, a dynamic one when it changes, an
    /// event one when it fires.
    pub fn result(&mut self, outcome: Outcome) {
        self.g.result = Some(outcome)
    }

    /// A new place's first value: the ready state as the tick opened, with the tick's
    /// pushes on top (Semantics ch. 9, "Queries are sync, from a ready state").
    pub fn first_value(&self, element: &Pod) -> Option<Pod> {
        let pushed = self.pushes.iter().rev().find(|(e, _)| e == element).map(|(_, v)| v.clone());
        pushed.or_else(|| self.ready.first_value(element))
    }
    /// The first value of the place at `path`: the World's, or under a hosted child the
    /// host's, from the dynamic its supplier's blueprint answers the place with (Semantics
    /// ch. 9, "How a supplier answers").
    pub fn first_value_at(&mut self, path: &Path, element: &Pod) -> Option<Pod> {
        match crate::host::split(path) {
            Some((host, at)) => match host.first_value(at, element, self) {
                Ok(v) => v,
                Err(f) => {
                    self.keep(Err(f));
                    None
                }
            },
            None => self.first_value(element),
        }
    }
    /// Register a place for the pushes keyed by its element's value.
    pub fn listen(&mut self, element: Pod, node: &Rc<dyn Node>) {
        self.g.listens.push((element, Rc::downgrade(node)))
    }
    /// Register the place at `path` for pushes: a hosted child's place hears its host
    /// instead, which answers it by path.
    pub fn listen_at(&mut self, path: &Path, element: Pod, node: &Rc<dyn Node>) {
        if crate::host::split(path).is_none() {
            self.listen(element, node)
        }
    }
}

/// What a program keeps between ticks besides its root: the tick counter, the started
/// list, the armed `@Defer`s, the push index, the clean-ups, the document last sent, and
/// the collector's zombies.
#[derive(Default)]
pub struct Runtime {
    tick: Tick,
    started: Vec<Rc<dyn Node>>,
    armed: Vec<Rc<dyn Node>>,
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
        Cx { tick: 0, read_at: 1, height: None, ready, pushes: Vec::new(), g: Gathered::default() }
    }

    /// One tick as a World calls it: open, fire, deliver, close.
    pub fn tick(&mut self, root: &Rc<dyn Node>, input: &TickIn, ready: &dyn ReadyState) -> Result<TickOut, Fault> {
        self.cleanups.drain(..).for_each(Node::clean_up);
        let n = self.tick + 1;
        let mut cx = Cx { tick: n, read_at: n, height: None, ready, pushes: input.pushes.clone(), g: Gathered::default() };
        for node in std::mem::take(&mut self.started) {
            node.started(&mut cx)?;
        }
        for node in std::mem::take(&mut self.armed) {
            node.deferred(&mut cx)?;
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
        (cx.height, cx.read_at) = (None, cx.tick + 1);
        // A patch can mark builders and nodes again (a host's document moves when its
        // child's places patch): the walks run again until nothing new is listed.
        loop {
            List::walk(&mut cx, |cx| &mut cx.g.builds, |n, cx| n.build(cx))?;
            List::walk(&mut cx, |cx| &mut cx.g.swaps, |n, cx| n.swap(cx))?;
            List::walk(&mut cx, |cx| &mut cx.g.leaves, |n, cx| n.leave(cx))?;
            List::walk(&mut cx, |cx| &mut cx.g.patches, |n, cx| n.patch(cx))?;
            let g = &cx.g;
            if [&g.builds, &g.swaps, &g.leaves, &g.patches].iter().all(|l| l.walked >= l.nodes.len()) {
                break;
            }
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
        (self.started, self.armed, self.tick) = (g.started.nodes, g.armed.nodes, cx.tick);
        Ok(TickOut {
            tick: cx.tick,
            patch: self.document.diff(g.edits),
            requests: g.requests,
            result: g.result,
            pending: !self.started.is_empty() || !self.armed.is_empty(),
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
