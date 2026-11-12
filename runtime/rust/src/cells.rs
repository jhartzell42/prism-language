//! The node primitives emitted code calls: the few generic pieces of state a template
//! would otherwise emit identically in every program (claude/backend.md, "Node
//! primitives").
//!
//! Every primitive is interior-mutable and borrows only inside its own methods, so an
//! emitted node is a plain struct of these behind an `Rc`, its methods take `&self`, and
//! no borrow is ever held across a firing (Compiler ch. 4, "The ownership layer": a
//! firing may deliver back into the node that fired).
//!
//! - [`Hold`]: a hold without a commit pass, two values each stamped with the tick that
//!   wrote it (Graph ch. 4, "Holds without a commit pass").
//! - [`Stamped`]: scratch between heights and a per-tick kept value; a stale stamp is an
//!   empty field, and nothing clears it (Compiler ch. 3, CXa).
//! - [`Cache`]: a computed behavior's cache with its stale stamp (Graph ch. 4,
//!   "Invalidation by stamped marks").
//! - [`Port`]: an output event port across nodes: subscriptions with constant-time
//!   handles, a lien count, and the tick's kept value for a subscriber that catches up.
//! - [`Event`], [`Dynamic`]: the `!a` and `$a` values that cross the node interface.
//! - [`Defer`]: `@Defer` (decisions.md D6).
//! - [`Job`], [`Turn`]: the step machine of an `unbounded` function, which `@Run` cycles.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::cx::{Cx, Node};

/// A tick number. Tick 0 is the build.
pub type Tick = u64;

// ---------------------------------------------------------------------------------------
// Hold
// ---------------------------------------------------------------------------------------

/// `$x <- init then !e`'s storage: the newer and the older value, each with the tick that
/// wrote it. A read during tick `n` answers the newer value stamped before `n`; from the
/// end of propagation on, the one stamped at or before `n`. Nothing installs anything at
/// the end of a tick.
pub struct Hold<T> {
    slots: RefCell<Slots<T>>,
}

/// The newer value and its stamp, and the older one once there has been a write.
type Slots<T> = (T, Tick, Option<(T, Tick)>);

impl<T: Clone> Hold<T> {
    /// A hold's initial value, stamped with the tick that built it.
    pub fn new(init: T, cx: &Cx) -> Hold<T> {
        Hold { slots: RefCell::new((init, cx.tick(), None)) }
    }
    /// The value at the context's read point: before the tick during propagation, after
    /// it from the end of propagation on.
    pub fn get(&self, cx: &Cx) -> T {
        let s = self.slots.borrow();
        match &s.2 {
            Some((older, _)) if s.1 >= cx.read_point() => older.clone(),
            _ => s.0.clone(),
        }
    }
    /// The tick's one write (G1, Semantics R3): the older slot takes the value, stamped
    /// now. A second write in a tick is a node woken twice for the same inputs: a fault of
    /// the tick, worded as the C++ runtime words it (prismc/README.md, TW).
    pub fn set(&self, v: T, cx: &Cx) {
        let mut s = self.slots.borrow_mut();
        if s.1 == cx.tick() && s.2.is_some() {
            cx.fail_later(crate::pod::Fault::new("Runtime.TwoWrites", format!("a hold written twice in tick {}", cx.tick())));
            return;
        }
        let (old, stamp) = (std::mem::replace(&mut s.0, v), s.1);
        s.1 = cx.tick();
        s.2 = Some((old, stamp));
    }
    /// The newer value, whatever the tick: what a dead widget's hold is from then on.
    pub fn newest(&self) -> T {
        self.slots.borrow().0.clone()
    }
}

// ---------------------------------------------------------------------------------------
// Stamped
// ---------------------------------------------------------------------------------------

/// A value and the tick that wrote it: a merge's input record, a kept firing, a fork
/// point. It reads as empty in every other tick.
pub struct Stamped<T> {
    cell: RefCell<Option<(T, Tick)>>,
}

impl<T: Clone> Default for Stamped<T> {
    fn default() -> Stamped<T> {
        Stamped { cell: RefCell::new(None) }
    }
}

impl<T: Clone> Stamped<T> {
    pub fn new() -> Stamped<T> {
        Stamped::default()
    }
    pub fn set(&self, v: T, cx: &Cx) {
        *self.cell.borrow_mut() = Some((v, cx.tick()));
    }
    /// The value written this tick, if one was.
    pub fn get(&self, cx: &Cx) -> Option<T> {
        self.at(cx.tick())
    }
    fn at(&self, tick: Tick) -> Option<T> {
        self.cell.borrow().as_ref().filter(|(_, t)| *t == tick).map(|(v, _)| v.clone())
    }
    /// The value written this tick, consumed: a combinator decides each input once.
    pub fn take(&self, cx: &Cx) -> Option<T> {
        let v = self.get(cx);
        if v.is_some() {
            *self.cell.borrow_mut() = None;
        }
        v
    }
    /// Whether it was written this tick: an event input that fired.
    pub fn fired(&self, cx: &Cx) -> bool {
        self.cell.borrow().as_ref().is_some_and(|(_, t)| *t == cx.tick())
    }
}

// ---------------------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------------------

/// A computed behavior's cache: the value, the read point it was computed at, and the
/// first read point from which it is stale. A mark at a write in tick `n` makes it stale
/// from after `n`, so a reader in the tick of the write reads the value the tag law calls
/// clean, and the reader after recomputes once. A value computed at one read point serves
/// only reads at or past it (and before the stale point): a read after the tick made
/// during propagation ([`Cx::after`]) caches the value after the tick, and a reader back
/// at the tick's own read point is not served it.
pub struct Cache<T> {
    value: RefCell<Option<(T, Tick)>>,
    stale_from: Cell<Tick>,
}

impl<T: Clone> Default for Cache<T> {
    fn default() -> Cache<T> {
        Cache { value: RefCell::new(None), stale_from: Cell::new(Tick::MAX) }
    }
}

impl<T: Clone> Cache<T> {
    pub fn new() -> Cache<T> {
        Cache::default()
    }
    /// The cached value at the read point, or `compute`'s, which is then cached. A read
    /// below the cached read point computes and keeps nothing: the cache stays the later
    /// value, and the marks that made it stale were already consumed by that read.
    pub fn get(&self, cx: &Cx, compute: impl FnOnce() -> T) -> T {
        let at = cx.read_point();
        if let Some((v, from)) = self.value.borrow().clone() {
            if from <= at && at < self.stale_from.get() {
                return v;
            }
            if at < from {
                return compute();
            }
        }
        let v = compute();
        if at >= self.stale_from.get() {
            self.stale_from.set(Tick::MAX);
        }
        *self.value.borrow_mut() = Some((v.clone(), at));
        v
    }
    /// An input was written this tick: stale from after it.
    pub fn mark(&self, cx: &Cx) {
        self.stale_from.set(self.stale_from.get().min(cx.tick() + 1));
    }
}

// ---------------------------------------------------------------------------------------
// Snapshot
// ---------------------------------------------------------------------------------------

/// A structural dynamic (Semantics ch. 7, "Structural dynamics"; round 3 ST): a host's child
/// document, a `for`'s gathered `$(K ==> T)`, anything whose value is made of constructed
/// structure. The structure after tick `n` cannot exist before `n`'s construction step has
/// run, so it is never computed during propagation: the node takes a snapshot S(n) once, at
/// the point of its construction step where the structure behind it has settled
/// ([`Snapshot::take`], [`Snapshot::take_compared`]).
///
/// The dynamic is lawful (R2/R3, ch. 7 "A dynamic is a sample and an update event") and runs
/// one tick behind the structure: its value after tick `m` is S(m-1). It changes at `n + 1`
/// when S(n) differs from S(n-1); the node then publishes it at the start of `n + 1`, among
/// the first firings of that owed tick ([`Snapshot::queue`], [`Snapshot::published`]), as a
/// write of a hold, so its update event fires at `n + 1` with S(n) while every sample at
/// `n + 1` reads S(n-1). [`Snapshot::get`] answers that value. Its value after the build that
/// made the node is the build's snapshot.
///
/// The structure itself, S(n) from the construction step of `n` on, is
/// [`Snapshot::current`]: what a host's own rows read (a host that forwards everything is
/// observably not there, so a newborn child place gets its row in the tick it is born). A
/// write behind the structure [`Snapshot::mark`]s the snapshot; the node's construction step
/// takes it when marked. A current read of a marked snapshot after the tick before it is
/// taken is a bug of the compiler, not a stale value: it panics with the reason.
pub struct Snapshot<T> {
    /// The newest snapshot and its tick, and the older one once there have been two.
    slots: RefCell<Option<Slots<T>>>,
    /// The tick a write behind the structure marked, until the snapshot of that tick is taken.
    marked: Cell<Option<Tick>>,
    /// The tick whose snapshot differs from the one before it (the dynamic moves in the tick
    /// after), as of the last take.
    moved: Cell<Option<Tick>>,
    /// The tick the node queued the publication in, once per tick.
    queued: Cell<Option<Tick>>,
    what: &'static str,
}

impl<T: Clone> Snapshot<T> {
    /// An empty snapshot; `what` names the dynamic in the panic message.
    pub fn new(what: &'static str) -> Snapshot<T> {
        Snapshot { slots: RefCell::new(None), marked: Cell::new(None), moved: Cell::new(None), queued: Cell::new(None), what }
    }
    /// A write behind the structure this tick: the construction step takes the snapshot.
    pub fn mark(&self, cx: &Cx) {
        self.marked.set(Some(cx.tick()));
    }
    /// Whether a snapshot was ever taken: a place's entry takes its first at its node's
    /// build, uncompared.
    pub fn taken(&self) -> bool {
        self.slots.borrow().is_some()
    }
    /// Whether a write marked the structure this tick and the snapshot is not taken yet.
    pub fn marked(&self, cx: &Cx) -> bool {
        self.marked.get() == Some(cx.tick())
    }
    /// The snapshot of this tick taken, the structure after the tick; a second take in the
    /// same tick replaces the first. Without equality every take after the node's first
    /// tick moves the dynamic. Answers whether it moved.
    pub fn take(&self, v: T, cx: &Cx) -> bool {
        self.put(v, cx, |_, _| true)
    }
    fn put(&self, v: T, cx: &Cx, differs: impl Fn(&T, &T) -> bool) -> bool {
        let mut s = self.slots.borrow_mut();
        let (next, moved) = match s.take() {
            // the node's first tick: the dynamic's value after it is this snapshot
            None => ((v, cx.tick(), None), false),
            Some((_, stamp, None)) if stamp == cx.tick() => ((v, stamp, None), false),
            // A take equal to the snapshot it is compared with keeps that snapshot, so its
            // sharing survives the tick and the next comparison of it is cheap: no program
            // tells two equal values apart (Semantics ch. 11, "sharing is invisible").
            Some((_, stamp, Some((older, ostamp)))) if stamp == cx.tick() => {
                let moved = differs(&older, &v);
                let v = if moved { v } else { older.clone() };
                ((v, stamp, Some((older, ostamp))), moved)
            }
            Some((newest, stamp, _)) => {
                let moved = differs(&newest, &v);
                let v = if moved { v } else { newest.clone() };
                ((v, cx.tick(), Some((newest, stamp))), moved)
            }
        };
        *s = Some(next);
        self.moved.set(if moved { Some(cx.tick()) } else { None });
        self.marked.set(None);
        moved
    }
    /// Whether the node queues the publication now: the snapshot taken this tick moved the
    /// dynamic, and nothing queued it yet this tick (one firing per tick).
    pub fn queue(&self, cx: &Cx) -> bool {
        let first = self.moved.get() == Some(cx.tick()) && self.queued.get() != Some(cx.tick());
        if first {
            self.queued.set(Some(cx.tick()));
        }
        first
    }
    /// At the start of tick `n + 1`: whether S(n) still differs from S(n-1), so the
    /// publication writes the dynamic.
    pub fn published(&self, cx: &Cx) -> bool {
        self.moved.get() == Some(cx.tick() - 1)
    }
    /// The dynamic at the read point: its value after the tick before the read point, the
    /// snapshot of the tick before that (S(m-1) after `m`); the build's snapshot until then.
    pub fn get(&self, cx: &Cx) -> T {
        let at = cx.read_point();
        let s = self.slots.borrow();
        match &*s {
            Some((newest, stamp, _)) if *stamp + 2 <= at => newest.clone(),
            Some((_, _, Some((older, ostamp)))) => {
                debug_assert!(*ostamp + 2 <= at, "{}: two snapshots newer than read point {at}", self.what);
                older.clone()
            }
            Some((first, _, None)) => first.clone(),
            None => panic!("{} read before its first snapshot was taken", self.what),
        }
    }
    /// The structure at the read point: the newest snapshot taken before it, S(n) from
    /// tick `n`'s construction step on, S(n-1) during `n`'s propagation.
    pub fn current(&self, cx: &Cx) -> T {
        let at = cx.read_point();
        let s = self.slots.borrow();
        match &*s {
            Some((newest, stamp, _)) if *stamp < at => {
                if at > cx.tick() && self.marked(cx) {
                    let how = if cx.propagating() { "under `Cx::after` during propagation" } else { "by a builder that runs before the structure it reads has settled" };
                    panic!("{} read after tick {} before its snapshot was taken: a structure read {how}", self.what, cx.tick());
                }
                newest.clone()
            }
            Some((_, _, Some((older, _)))) => older.clone(),
            Some((_, stamp, None)) => panic!("{} read at read point {at} before its first snapshot, taken in tick {stamp}", self.what),
            None => panic!("{} read before its first snapshot was taken", self.what),
        }
    }
}

impl<T: Clone + Eq> Snapshot<T> {
    /// The snapshot taken and compared with the one that stood before this tick: whether
    /// the dynamic moved (R3: its update event fires only when the value changed). When it
    /// did not, the snapshot that stood is kept and `v` dropped.
    pub fn take_compared(&self, v: T, cx: &Cx) -> bool {
        self.put(v, cx, |a, b| a != b)
    }
}

// ---------------------------------------------------------------------------------------
// Heights
// ---------------------------------------------------------------------------------------

/// A global height no program reaches: a combinator raised past it is raised by a prompt
/// cycle across nodes, which the checker refuses, so it is a bug of the compiler.
const HEIGHT_LIMIT: u32 = 1 << 20;

/// A node's combinators' global heights, and the ones asked to wake this tick. A
/// combinator starts at its distance inside the node (the emitted constant) and is put
/// above every source that reaches it from another node, as the source's height is told
/// at the wiring ([`Port::subscribe_above`], a height twin called by a parent, a row or a
/// sink). The wake runs the lowest combinator asked ([`Heights::next`]) and asks again for
/// the next ([`Heights::again`]), so each combinator decides once, after every input that
/// can make it fire has had its turn (Graph ch. 4, "Heights"; Semantics ch. 7: double
/// firings never).
pub struct Heights {
    at: RefCell<Vec<u32>>,
    asked: RefCell<std::collections::BTreeSet<(u32, usize)>>,
}

impl Heights {
    /// The combinators at their distances inside the node, in the node's order.
    pub fn new(distances: Vec<u32>) -> Heights {
        Heights { at: RefCell::new(distances), asked: RefCell::new(std::collections::BTreeSet::new()) }
    }
    /// The global height of the node's `combinator`.
    pub fn of(&self, combinator: usize) -> u32 {
        self.at.borrow()[combinator]
    }
    /// A source at global height `h` reaches `combinator`: it sits above it. Gives the
    /// combinator's new height when it moved, which its own twin then tells what it feeds.
    pub fn above(&self, combinator: usize, h: u32) -> Option<u32> {
        let mut at = self.at.borrow_mut();
        if at[combinator] > h {
            return None;
        }
        assert!(h < HEIGHT_LIMIT, "a prismc bug: a combinator's height does not settle (a prompt cycle across nodes)");
        at[combinator] = h + 1;
        Some(h + 1)
    }
    /// An input of `combinator` fired: its wake is asked at its height.
    pub fn ask(&self, combinator: usize, cx: &mut Cx, node: Rc<dyn crate::cx::Node>) {
        let h = self.of(combinator);
        self.asked.borrow_mut().insert((h, combinator));
        cx.wake(h, node);
    }
    /// The lowest combinator asked whose height has come (at or below the running height),
    /// for the node's wake to decide; none when every one left sits higher. Combinators of
    /// one height decide in one wake of the node, in the node's order.
    pub fn next(&self, cx: &Cx) -> Option<usize> {
        let running = cx.running_height()?;
        let mut asked = self.asked.borrow_mut();
        match asked.first() {
            Some(&(h, _)) if h <= running => asked.pop_first().map(|(_, combinator)| combinator),
            _ => None,
        }
    }
    /// After a wake decided the combinators whose height had come: a wake for the lowest
    /// still asked.
    pub fn again(&self, cx: &mut Cx, node: Rc<dyn crate::cx::Node>) {
        let lowest = self.asked.borrow().first().copied();
        if let Some((h, _)) = lowest {
            cx.wake(h, node);
        }
    }
}

// ---------------------------------------------------------------------------------------
// Ports and the values that cross the node interface
// ---------------------------------------------------------------------------------------

/// A subscription's handle, for unsubscribing in constant time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle(usize);

/// A subscriber: a typed closure holding its consumer weakly, since references held
/// downstream are weak (Graph ch. 4).
pub type Subscriber<T> = Rc<dyn Fn(&T, &mut Cx) -> crate::cx::Done>;

/// A subscriber's height twin: told the global height its source fires at, when it
/// subscribes and each time that height rises. It walks where the subscriber's firing
/// would go and puts every combinator it reaches above that height ([`Heights::above`]),
/// a port it would fire straight through at that height ([`Port::raise`]).
pub type Above = Rc<dyn Fn(u32)>;

/// An output event port across nodes. `height` is the global height the port fires at:
/// the height of the combinator whose wake fires it, or of whatever fires it straight
/// through an input; it is only ever raised (Graph ch. 4, "Heights": one too high is
/// harmless), and only while nothing propagates.
pub struct Port<T> {
    subs: RefCell<Vec<Option<Subscriber<T>>>>,
    above: RefCell<Vec<Option<Above>>>,
    free: RefCell<Vec<usize>>,
    liens: Cell<u32>,
    kept: Stamped<T>,
    height: Cell<u32>,
}

/// How much an output is wanted (Graph ch. 4, "Demand"). Tags are a behavior's, and the
/// node that owns the behavior counts them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Demand {
    Idle,
    Liened,
    Subscribed,
}

impl<T: Clone> Default for Port<T> {
    fn default() -> Port<T> {
        Port { subs: RefCell::new(Vec::new()), above: RefCell::new(Vec::new()), free: RefCell::new(Vec::new()), liens: Cell::new(0), kept: Stamped::new(), height: Cell::new(0) }
    }
}

impl<T: Clone> Port<T> {
    pub fn new() -> Port<T> {
        Port::default()
    }
    pub fn subscribe(&self, f: Subscriber<T>) -> Handle {
        let mut subs = self.subs.borrow_mut();
        match self.free.borrow_mut().pop() {
            Some(i) => {
                subs[i] = Some(f);
                Handle(i)
            }
            None => {
                subs.push(Some(f));
                Handle(subs.len() - 1)
            }
        }
    }
    pub fn unsubscribe(&self, h: Handle) {
        if self.subs.borrow_mut().get_mut(h.0).and_then(Option::take).is_some() {
            if let Some(twin) = self.above.borrow_mut().get_mut(h.0) {
                *twin = None;
            }
            self.free.borrow_mut().push(h.0);
        }
    }
    /// Subscribe with the subscriber's height twin: the twin is told the port's height at
    /// once, and again each time the port is raised, so whatever the subscriber feeds
    /// sits above the port however late it subscribes.
    pub fn subscribe_above(&self, f: Subscriber<T>, above: Above) -> Handle {
        let h = self.subscribe(f);
        {
            let mut twins = self.above.borrow_mut();
            if twins.len() <= h.0 {
                twins.resize_with(h.0 + 1, || None);
            }
            twins[h.0] = Some(above.clone());
        }
        above(self.height.get());
        h
    }
    /// The global height the port fires at.
    pub fn height(&self) -> u32 {
        self.height.get()
    }
    /// The port fires at global height `h`: when that is above what it was, every
    /// subscriber's twin is told. The list is copied first, so a twin may subscribe.
    pub fn raise(&self, h: u32) {
        if h <= self.height.get() {
            return;
        }
        self.height.set(h);
        let twins: Vec<Above> = self.above.borrow().iter().flatten().cloned().collect();
        for twin in twins {
            twin(h);
        }
    }
    pub fn lien(&self) {
        self.liens.set(self.liens.get() + 1);
    }
    pub fn release(&self) {
        self.liens.set(self.liens.get().saturating_sub(1));
    }
    pub fn demand(&self) -> Demand {
        if self.subs.borrow().iter().any(Option::is_some) {
            Demand::Subscribed
        } else if self.liens.get() > 0 {
            Demand::Liened
        } else {
            Demand::Idle
        }
    }
    /// Fire: keep the value for the rest of the tick, then deliver it to every
    /// subscriber. The list is copied first, so a subscriber may unsubscribe.
    pub fn fire(&self, v: T, cx: &mut Cx) -> crate::cx::Done {
        self.kept.set(v.clone(), cx);
        let subs: Vec<Subscriber<T>> = self.subs.borrow().iter().flatten().cloned().collect();
        subs.iter().try_for_each(|f| f(&v, cx))
    }
    /// What it fired this tick: a subscriber arriving at a height after the firing
    /// catches up from it. A newborn or a swap never does.
    pub fn kept(&self, cx: &Cx) -> Option<T> {
        self.kept.get(cx)
    }
}

/// An `!a` value: an output port and the node that owns it, held strongly, since a
/// value's `!` part liens its producer.
pub struct Event<T> {
    pub owner: Option<Rc<dyn Node>>,
    pub port: Rc<Port<T>>,
}

impl<T> Clone for Event<T> {
    fn clone(&self) -> Event<T> {
        Event { owner: self.owner.clone(), port: self.port.clone() }
    }
}

/// Events compare by identity, the port they fire, as dynamics do (`Dynamic`'s `PartialEq`):
/// an element's level holds the events of its event fields (ruling EP), and a place whose
/// level holds the same events has not moved.
impl<T> PartialEq for Event<T> {
    fn eq(&self, other: &Event<T>) -> bool {
        Rc::ptr_eq(&self.port, &other.port)
    }
}

impl<T> Eq for Event<T> {}

impl<T: Clone + 'static> Event<T> {
    /// `never`: a port nothing fires.
    pub fn never() -> Event<T> {
        Event { owner: None, port: Rc::new(Port::new()) }
    }
    /// A node's own event as a value: the port its firing method fires.
    pub fn of(port: &Rc<Port<T>>) -> Event<T> {
        Event { owner: None, port: port.clone() }
    }
    /// The event `f` of this one, a pure transform at each firing (a lifted call).
    pub fn map<U: Clone + 'static>(&self, f: impl Fn(&T) -> U + 'static) -> Event<U> {
        let port = Rc::new(Port::new());
        let out = port.clone();
        let raised = port.clone();
        self.port.subscribe_above(Rc::new(move |v: &T, cx: &mut Cx| out.fire(f(v), cx)), Rc::new(move |h| raised.raise(h)));
        Event { owner: self.owner.clone(), port }
    }
}

impl<U: Clone + 'static> Event<Option<U>> {
    /// `!Filter`: the present values.
    pub fn filter_some(&self) -> Event<U> {
        let port = Rc::new(Port::new());
        let out = port.clone();
        let raised = port.clone();
        self.port.subscribe_above(Rc::new(move |v: &Option<U>, cx: &mut Cx| match v {
            Some(u) => out.fire(u.clone(), cx),
            None => Ok(()),
        }), Rc::new(move |h| raised.raise(h)));
        Event { owner: self.owner.clone(), port }
    }
}

/// `!MergeAll` of a list that is not a literal (Semantics Appendix B): the list is a plain
/// value, so its events are fixed at the build. Each event's port is heard into a stamped
/// input, and the node is told (`heard`, which asks its wake); the wake takes the values
/// fired this tick, in the list's order, as one firing.
pub struct MergeList<T: Clone> {
    inputs: RefCell<Rc<Vec<Stamped<T>>>>,
    subs: RefCell<Vec<(Rc<Port<T>>, Handle)>>,
}

impl<T: Clone + 'static> MergeList<T> {
    pub fn new() -> MergeList<T> {
        MergeList { inputs: RefCell::new(Rc::new(Vec::new())), subs: RefCell::new(Vec::new()) }
    }
    /// Hears every event of `events`, telling `node` through `heard` when one fires.
    pub fn listen<N: 'static>(&self, events: &crate::collections::Array<Event<T>>, node: &Rc<N>, heard: fn(&Rc<N>, &mut Cx)) {
        self.listen_above(events, node, heard, |_, _| ())
    }
    /// As [`MergeList::listen`], with the node's height twin: `above` is told each
    /// event's height, at once and when it rises, and puts the node's merge above it.
    pub fn listen_above<N: 'static>(&self, events: &crate::collections::Array<Event<T>>, node: &Rc<N>, heard: fn(&Rc<N>, &mut Cx), above: fn(&Rc<N>, u32)) {
        let inputs: Rc<Vec<Stamped<T>>> = Rc::new(events.iter().map(|_| Stamped::new()).collect());
        *self.inputs.borrow_mut() = inputs.clone();
        let mut subs = self.subs.borrow_mut();
        for (i, ev) in events.iter().enumerate() {
            let weak = Rc::downgrade(node);
            let ins = inputs.clone();
            let told = weak.clone();
            let h = ev.port.subscribe_above(Rc::new(move |v: &T, cx: &mut Cx| {
                ins[i].set(v.clone(), cx);
                if let Some(n) = weak.upgrade() {
                    heard(&n, cx);
                }
                Ok(())
            }), Rc::new(move |h| {
                if let Some(n) = told.upgrade() {
                    above(&n, h);
                }
            }));
            subs.push((ev.port.clone(), h));
        }
    }
    /// The values the list's events fired this tick, in the list's order; none fired, none.
    pub fn take(&self, cx: &Cx) -> Option<crate::collections::Array<T>> {
        let mut all = crate::collections::Array::new();
        for s in self.inputs.borrow().iter() {
            if let Some(x) = s.take(cx) {
                all.push_back(x);
            }
        }
        if all.len() == 0 { None } else { Some(all) }
    }
}

impl<T: Clone + 'static> Default for MergeList<T> {
    fn default() -> MergeList<T> {
        MergeList::new()
    }
}

impl<T: Clone> Drop for MergeList<T> {
    fn drop(&mut self) {
        for (p, h) in self.subs.borrow_mut().drain(..) {
            p.unsubscribe(h);
        }
    }
}

/// A held `@Switch`'s subscription to its current inner event (prismc's `HeldSwitch`):
/// listening to another event lets go of the one before. The event is kept whole: its
/// owner (a `merge_all`'s node) lives while the switch listens.
pub struct Listen<T: Clone> {
    sub: RefCell<Option<(Event<T>, Handle)>>,
}

impl<T: Clone + 'static> Default for Listen<T> {
    fn default() -> Listen<T> {
        Listen { sub: RefCell::new(None) }
    }
}

impl<T: Clone + 'static> Listen<T> {
    /// From now on `ev` tells `node` through `heard`, and the event before tells nothing.
    pub fn listen<N: 'static>(&self, ev: &Event<T>, node: &Rc<N>, heard: fn(&Rc<N>, T, &mut Cx)) {
        self.drop_sub();
        let weak = Rc::downgrade(node);
        let h = ev.port.subscribe(Rc::new(move |v: &T, cx: &mut Cx| {
            if let Some(n) = weak.upgrade() {
                heard(&n, v.clone(), cx);
            }
            Ok(())
        }));
        *self.sub.borrow_mut() = Some((ev.clone(), h));
    }
    fn drop_sub(&self) {
        if let Some((ev, h)) = self.sub.borrow_mut().take() {
            ev.port.unsubscribe(h);
        }
    }
}

impl<T: Clone> Drop for Listen<T> {
    fn drop(&mut self) {
        if let Some((ev, h)) = self.sub.borrow_mut().take() {
            ev.port.unsubscribe(h);
        }
    }
}

/// `!MergeAll` of a list as an event value: a node of its own that hears the list's events
/// (`MergeList`) and, woken, fires the values of the tick in the list's order.
struct MergeNode<T: Clone + 'static> {
    list: MergeList<T>,
    out: Rc<Port<crate::collections::Array<T>>>,
    /// The merge, the node's one combinator.
    heights: Heights,
}

impl<T: Clone + 'static> Node for MergeNode<T> {
    fn wake(self: Rc<Self>, cx: &mut Cx) -> crate::cx::Done {
        if self.heights.next(cx).is_none() {
            return Ok(());
        }
        match self.list.take(cx) {
            Some(all) => self.out.fire(all, cx),
            None => Ok(()),
        }
    }
}

/// The event `!MergeAll events` (Semantics Appendix B): its node woken above the height of
/// every event of the list, so it fires once in a tick, after each of them had its turn;
/// its own port fires at that height. Owned by the event.
pub fn merge_all<T: Clone + 'static>(events: &crate::collections::Array<Event<T>>) -> Event<crate::collections::Array<T>> {
    let node = Rc::new(MergeNode { list: MergeList::new(), out: Rc::new(Port::new()), heights: Heights::new(vec![1]) });
    node.out.raise(node.heights.of(0));
    node.list.listen_above(
        events,
        &node,
        |n: &Rc<MergeNode<T>>, cx: &mut Cx| n.heights.ask(0, cx, n.clone()),
        |n: &Rc<MergeNode<T>>, h: u32| {
            if let Some(moved) = n.heights.above(0, h) {
                n.out.raise(moved);
            }
        },
    );
    let port = node.out.clone();
    Event { owner: Some(node), port }
}

/// A `$a` value: a read at the context's read point, its update event, and its owner.
/// The update event says only that the value moved; a reader reads it again, so a
/// projection of a dynamic shares its source's port.
pub struct Dynamic<T> {
    pub owner: Option<Rc<dyn Node>>,
    pub read: Rc<dyn Fn(&Cx) -> T>,
    pub updates: Rc<Port<()>>,
}

impl<T> Clone for Dynamic<T> {
    fn clone(&self) -> Dynamic<T> {
        Dynamic { owner: self.owner.clone(), read: self.read.clone(), updates: self.updates.clone() }
    }
}

impl<T: Clone + 'static> Dynamic<T> {
    /// `$ v`: a constant, whose update event never fires.
    pub fn constant(v: T) -> Dynamic<T> {
        Dynamic { owner: None, read: Rc::new(move |_| v.clone()), updates: Rc::new(Port::new()) }
    }
    /// A dynamic with no owner over a read and the port that says it moved: a host's
    /// dynamic answer to a child place (`@ $d`), whose read goes through the owner's getter.
    pub fn of(read: impl Fn(&Cx) -> T + 'static, updates: &Rc<Port<()>>) -> Dynamic<T> {
        Dynamic { owner: None, read: Rc::new(read), updates: updates.clone() }
    }
    pub fn sample(&self, cx: &Cx) -> T {
        (self.read)(cx)
    }
    /// The dynamic `f` of this one: the same owner and update event, the read mapped.
    pub fn map<U: 'static>(&self, f: impl Fn(&T) -> U + 'static) -> Dynamic<U> {
        let read = self.read.clone();
        Dynamic { owner: self.owner.clone(), read: Rc::new(move |cx| f(&read(cx))), updates: self.updates.clone() }
    }
    /// The dynamic `f` of this one and `other` (a lift over two dynamics in a function that
    /// takes them whole, Type System ch. 4): read from both; its update event fires, once a
    /// tick, when either moves.
    pub fn map2<U: Clone + 'static, V: 'static>(&self, other: &Dynamic<U>, f: impl Fn(&T, &U) -> V + 'static) -> Dynamic<V> {
        let (a, b) = (self.read.clone(), other.read.clone());
        let merged: Rc<Port<()>> = Rc::new(Port::new());
        for src in [&self.updates, &other.updates] {
            let out = merged.clone();
            src.subscribe(Rc::new(move |_: &(), cx: &mut Cx| if out.kept(cx).is_some() { Ok(()) } else { out.fire((), cx) }));
        }
        Dynamic { owner: None, read: Rc::new(move |cx| f(&a(cx), &b(cx))), updates: merged }
    }
}

/// Two dynamics are equal when they are the same dynamic: the same read and the same update
/// event, never by their values (a dynamic has no `^Eq` in Prism, Type System ch. 2, and no
/// program compares one). This is what the runtime's own comparisons see where a value
/// holds dynamics (a host document's entries, an element whose children are dynamics): a
/// place's entry is the same dynamic for as long as the place lives, so an element whose
/// children hold the same dynamics has not moved (claude/incremental-host-document.md, doubt
/// EQ).
impl<T> PartialEq for Dynamic<T> {
    fn eq(&self, other: &Dynamic<T>) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(&self.read), Rc::as_ptr(&other.read)) && Rc::ptr_eq(&self.updates, &other.updates)
    }
}

impl<T> Eq for Dynamic<T> {}

/// A dynamic is not plain data: a World reads a level that holds dynamics deep, each
/// sampled at the read point (`crate::host::Level`). Reaching this is a bug of the compiler,
/// never a value.
impl<T> crate::pod::ToPod for Dynamic<T> {
    fn to_pod(&self) -> crate::pod::Pod {
        panic!("a dynamic made plain data without a read point: a level that holds dynamics is read with `Level`")
    }
}

/// No World hands a program a dynamic.
impl<T> crate::pod::FromPod for Dynamic<T> {
    fn from_pod(p: &crate::pod::Pod) -> Result<Dynamic<T>, crate::pod::Fault> {
        crate::pod::mismatch("a dynamic, which is never plain data", p)
    }
}

/// A dynamic does not print (no `^Show`); reaching this is a bug of the compiler.
impl<T> crate::print::Show for Dynamic<T> {
    fn show(&self, _: &mut crate::print::Printer) {
        panic!("a dynamic shown: a dynamic has no `^Show`")
    }
}

/// A live element, `Element r o`: what a host's supplier receives for each of its child's
/// places. `identity` is the place's level at its build, whose tag and plain fields never
/// change while it lives; `level` is its level over time, which a `$` field reads; `entry`
/// is the place's entry in its host's document (a forwarded element's place in an
/// enclosing host's document is the same entry).
pub struct Live<E> {
    pub identity: E,
    pub level: Dynamic<E>,
    pub entry: Rc<crate::host::Entry<E>>,
    /// The place's requests as its host's row hears them: the event field's name when the
    /// tag has several, and the value fired.
    pub requests: Rc<Port<(Option<String>, crate::pod::Pod)>>,
}

impl<E: Clone> Clone for Live<E> {
    fn clone(&self) -> Live<E> {
        Live { identity: self.identity.clone(), level: self.level.clone(), entry: self.entry.clone(), requests: self.requests.clone() }
    }
}

impl<E> Live<E> {
    /// One event field of the live element as an `!T` (an element pattern's `!field=!r`,
    /// ruling EP): each request the place makes through `field` (`None`: the tag's only
    /// event field), decoded, in the tick the place fires it. A value that does not decode
    /// is a fault of that tick.
    pub fn requested<T: crate::pod::FromPod + Clone + 'static>(&self, field: Option<&'static str>) -> Event<T> {
        let port = Rc::new(Port::new());
        let out = port.clone();
        self.requests.subscribe(Rc::new(move |(f, v): &(Option<String>, crate::pod::Pod), cx: &mut Cx| {
            if f.as_deref() == field {
                out.fire(T::from_pod(v)?, cx)
            } else {
                Ok(())
            }
        }));
        Event { owner: None, port }
    }
}

/// An element applied as a plain value, before a statement or a host places it (Type System
/// ch. 2, "Names and sigils"): its tag's description record as the program's own emitted
/// type (`desc`, which a pattern takes back by its type, [`element_view`]), its level over
/// time, mapped from its `$` fields, and each event field's firings, the requests of the
/// place once it is placed. It reads nothing until it is placed.
pub struct Described<E> {
    pub desc: Rc<dyn std::any::Any>,
    pub level: Dynamic<E>,
    pub requests: Vec<(Option<String>, Event<crate::pod::Pod>)>,
}

impl<E> Clone for Described<E> {
    fn clone(&self) -> Described<E> {
        Described { desc: self.desc.clone(), level: self.level.clone(), requests: self.requests.clone() }
    }
}

impl<E> Described<E> {
    /// An event field of the description: its firings are the placed element's requests,
    /// `field` naming it when the tag has several.
    pub fn request(mut self, field: Option<&str>, firings: Event<crate::pod::Pod>) -> Described<E> {
        self.requests.push((field.map(Into::into), firings));
        self
    }
}

/// A description of a tag: `desc` its description record, `level` its level over time.
pub fn described<E, D: 'static>(desc: D, level: Dynamic<E>) -> Described<E> {
    Described { desc: Rc::new(desc), level, requests: Vec::new() }
}

/// An element value, `Element r o` (Semantics RULINGS round 12, EV: elements are ordinary
/// values): a live element, which a host's supplier receives, or a description, a tag
/// applied.
pub enum Element<E> {
    Live(Live<E>),
    Described(Described<E>),
}

impl<E: Clone> Clone for Element<E> {
    fn clone(&self) -> Element<E> {
        match self {
            Element::Live(l) => Element::Live(l.clone()),
            Element::Described(d) => Element::Described(d.clone()),
        }
    }
}

/// A description as an element value.
pub fn described_element<E>(d: Described<E>) -> Element<E> {
    Element::Described(d)
}

/// An element seen as the description `D` of one tag: a description of that tag as it is, a
/// live element as `of_live` gives it (the description of its level, when it is of the
/// tag), else `None`.
pub fn element_view<E, D: Clone + 'static>(e: &Element<E>, of_live: impl Fn(&Live<E>) -> Option<D>) -> Option<D> {
    match e {
        Element::Live(l) => of_live(l),
        Element::Described(d) => d.desc.downcast_ref::<D>().cloned(),
    }
}

/// Whether an element is of the tag whose description is `D` ([`element_view`]).
pub fn element_is<E, D: Clone + 'static>(e: &Element<E>, of_live: impl Fn(&Live<E>) -> Option<D>) -> bool {
    element_view(e, of_live).is_some()
}

// ---------------------------------------------------------------------------------------
// @Defer, and the step machines @Run cycles
// ---------------------------------------------------------------------------------------

/// `@Defer: fn !a -> @!a`: each firing fires again in the next tick, by the runtime, with
/// no World involved, and an armed `@Defer` sets `pending` (decisions.md D6).
///
/// A `Defer` is a field of the node that binds it. [`Defer::arm`] records the value and
/// puts the node on the armed list; in the next tick the runtime calls the node's
/// [`Node::deferred`] at the lowest height, where [`Defer::fired`] gives the value.
pub struct Defer<T> {
    armed: RefCell<Option<(T, Tick)>>,
}

impl<T: Clone> Default for Defer<T> {
    fn default() -> Defer<T> {
        Defer { armed: RefCell::new(None) }
    }
}

impl<T: Clone> Defer<T> {
    pub fn new() -> Defer<T> {
        Defer::default()
    }
    /// The argument fired: fire it again next tick.
    pub fn arm(&self, v: T, node: Rc<dyn Node>, cx: &mut Cx) {
        *self.armed.borrow_mut() = Some((v, cx.tick()));
        cx.arm(node);
    }
    /// `@Defer`'s firing this tick: what was armed in the tick before.
    pub fn fired(&self, cx: &Cx) -> Option<T> {
        self.armed.borrow().as_ref().filter(|(_, t)| t + 1 == cx.tick()).map(|(v, _)| v.clone())
    }
}

/// `@Later: fn !a -> @!a` (Semantics Appendix B): each firing answered one tick later,
/// through a zero-delay `Time.After`. The firings whose wake the World has not answered
/// yet, oldest first: [`LaterQueue::queue`] at each firing, as the wake is asked;
/// [`LaterQueue::answered`] at each answer of the place, which gives the oldest back.
pub struct LaterQueue<T> {
    pending: RefCell<std::collections::VecDeque<T>>,
}

impl<T> Default for LaterQueue<T> {
    fn default() -> LaterQueue<T> {
        LaterQueue { pending: RefCell::new(std::collections::VecDeque::new()) }
    }
}

impl<T> LaterQueue<T> {
    pub fn new() -> LaterQueue<T> {
        LaterQueue::default()
    }
    /// The argument fired: kept until its wake is answered.
    pub fn queue(&self, v: T) {
        self.pending.borrow_mut().push_back(v);
    }
    /// The place was answered: the oldest firing still kept. An answer with none kept is
    /// a World answering a wake nobody asked for.
    pub fn answered(&self) -> T {
        match self.pending.borrow_mut().pop_front() {
            Some(v) => v,
            None => panic!("an answer at the place of an `@Later` that asked for no wake"),
        }
    }
}

/// An `unbounded` function stopped at a cycle point: its continuation (Semantics ch. 7,
/// "An unbounded function is a bounded body iterated"). The compiler emits one type per
/// group of unbounded functions, whose frames are the cycle points of the canonical rule
/// (a call of an unbounded function from an unbounded function), and `Start F, x` is the
/// job stopped before its first cycle.
pub trait Job<B> {
    /// One cycle: run bounded code from here to the next cycle point, or to the result.
    fn cycle(&self) -> Turn<B>;
}

/// What a cycle ends in.
pub enum Turn<B> {
    /// Stopped at a cycle point; the next cycle resumes here.
    More(Rc<dyn Job<B>>),
    Finished(B),
}
