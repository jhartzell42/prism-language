//! The node primitives emitted code calls: the few generic pieces of state a template
//! would otherwise emit identically in every program (docs/backend.md, "Node
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
    /// the tick (and an assertion in a debug build).
    pub fn set(&self, v: T, cx: &Cx) {
        let mut s = self.slots.borrow_mut();
        if s.1 == cx.tick() && s.2.is_some() {
            debug_assert!(false, "a hold of {} written twice in tick {}", std::any::type_name::<T>(), cx.tick());
            cx.fail_later(crate::pod::Fault::new("Runtime.TwoWrites", format!("a hold of {} written twice in tick {}", std::any::type_name::<T>(), cx.tick())));
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

/// A structural dynamic's value (Semantics ch. 7, "Structural dynamics"; round 3 ST): a
/// host's child document, a `for`'s gathered `$(K ==> T)`, anything whose value is made of
/// constructed structure. Such a value after tick `n` cannot exist before `n`'s
/// construction step has run, so it is never computed during propagation: the node takes it
/// once, at the point of its construction step where the structure behind it has settled
/// ([`Snapshot::take`], [`Snapshot::take_compared`]), and every read answers a snapshot. A
/// read at read point `n` (a sample during propagation, or under [`Cx::before`]) answers the
/// snapshot of `n - 1`; a read after the tick answers the snapshot of `n`, taken or, when
/// nothing marked the structure this tick, the one that stands.
///
/// A write behind the structure [`Snapshot::mark`]s the snapshot; the node's construction
/// step takes it when marked. Reading a marked snapshot after the tick before it is taken is
/// a bug of the compiler, not a stale value: a read under [`Cx::after`] during propagation,
/// or a builder that runs before the structure it reads has settled. It panics with the
/// reason.
pub struct Snapshot<T> {
    /// The newest snapshot and its tick, and the older one once there have been two.
    slots: RefCell<Option<Slots<T>>>,
    /// The tick a write behind the structure marked, until the snapshot of that tick is taken.
    marked: Cell<Option<Tick>>,
    what: &'static str,
}

impl<T: Clone> Snapshot<T> {
    /// An empty snapshot; `what` names the dynamic in the panic message.
    pub fn new(what: &'static str) -> Snapshot<T> {
        Snapshot { slots: RefCell::new(None), marked: Cell::new(None), what }
    }
    /// A write behind the structure this tick: the construction step takes the snapshot.
    pub fn mark(&self, cx: &Cx) {
        self.marked.set(Some(cx.tick()));
    }
    /// Whether a write marked the structure this tick and the snapshot is not taken yet.
    pub fn marked(&self, cx: &Cx) -> bool {
        self.marked.get() == Some(cx.tick())
    }
    /// The snapshot of this tick, taken: the value after the tick, which every read after
    /// the tick answers from now on. A second take in the same tick replaces the first.
    pub fn take(&self, v: T, cx: &Cx) {
        let mut s = self.slots.borrow_mut();
        *s = Some(match s.take() {
            Some((_, stamp, older)) if stamp == cx.tick() => (v, stamp, older),
            Some((newest, stamp, _)) => (v, cx.tick(), Some((newest, stamp))),
            None => (v, cx.tick(), None),
        });
        self.marked.set(None);
    }
    /// The value at the read point: the newest snapshot taken before it.
    pub fn get(&self, cx: &Cx) -> T {
        let at = cx.read_point();
        let s = self.slots.borrow();
        match &*s {
            Some((newest, stamp, _)) if *stamp < at => {
                if at > cx.tick() && self.marked(cx) {
                    let how = if cx.propagating() { "under `Cx::after` during propagation" } else { "by a builder that runs before the structure it reads has settled" };
                    panic!("{} read after tick {} before its snapshot was taken: a structural dynamic read {how}", self.what, cx.tick());
                }
                newest.clone()
            }
            Some((_, _, Some((older, _)))) => older.clone(),
            Some((_, stamp, None)) => panic!("{} read at read point {at} before its first snapshot, taken in tick {stamp}", self.what),
            None => panic!("{} read before its first snapshot was taken", self.what),
        }
    }
}

impl<T: Clone + PartialEq> Snapshot<T> {
    /// The snapshot taken and compared with the one that stood: whether the value moved.
    /// The update event of a structural dynamic fires only when it did (R3 with one tick of
    /// lag; a dedupe where equality exists).
    pub fn take_compared(&self, v: T, cx: &Cx) -> bool {
        let moved = self.slots.borrow().as_ref().is_none_or(|(newest, _, _)| *newest != v);
        self.take(v, cx);
        moved
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

/// An output event port across nodes.
pub struct Port<T> {
    subs: RefCell<Vec<Option<Subscriber<T>>>>,
    free: RefCell<Vec<usize>>,
    liens: Cell<u32>,
    kept: Stamped<T>,
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
        Port { subs: RefCell::new(Vec::new()), free: RefCell::new(Vec::new()), liens: Cell::new(0), kept: Stamped::new() }
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
            self.free.borrow_mut().push(h.0);
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
        self.port.subscribe(Rc::new(move |v: &T, cx: &mut Cx| out.fire(f(v), cx)));
        Event { owner: self.owner.clone(), port }
    }
}

impl<U: Clone + 'static> Event<Option<U>> {
    /// `!Filter`: the present values.
    pub fn filter_some(&self) -> Event<U> {
        let port = Rc::new(Port::new());
        let out = port.clone();
        self.port.subscribe(Rc::new(move |v: &Option<U>, cx: &mut Cx| match v {
            Some(u) => out.fire(u.clone(), cx),
            None => Ok(()),
        }));
        Event { owner: self.owner.clone(), port }
    }
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
    pub fn sample(&self, cx: &Cx) -> T {
        (self.read)(cx)
    }
    /// The dynamic `f` of this one: the same owner and update event, the read mapped.
    pub fn map<U: 'static>(&self, f: impl Fn(&T) -> U + 'static) -> Dynamic<U> {
        let read = self.read.clone();
        Dynamic { owner: self.owner.clone(), read: Rc::new(move |cx| f(&read(cx))), updates: self.updates.clone() }
    }
}

/// A live element, `Element r o`: what a host's supplier receives for each of its child's
/// places. `identity` is the place's level at its build, whose tag and plain fields never
/// change while it lives; `level` is its level over time, which a `$` field reads.
pub struct Live<E> {
    pub identity: E,
    pub level: Dynamic<E>,
}

impl<E: Clone> Clone for Live<E> {
    fn clone(&self) -> Live<E> {
        Live { identity: self.identity.clone(), level: self.level.clone() }
    }
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
