//! Hosting (Semantics ch. 9, "`@HostDocument` and its forms"; claude/backend.md, "Hosting"):
//! what is the same in every program when a host document is lowered as a `for` over its
//! child's document.
//!
//! A host is Prism inside the same graph and the same tick. The compiler inlines the hosted
//! child into the node of the host's block (its owner), in a scope of its own. The child's
//! document is `$(Path ==> $Elem c)` (Jimmy, 6 November 2026): each child place is an
//! [`Entry`], a structural dynamic of its own kept in the node the place is in, and the
//! outer is a [`HostDoc`] the owner keeps as a snapshot, taken when a place came or went
//! (the program reads its dynamics; the host's own rows its entries, `host_k_doc_direct`).
//! The owner emits a dispatch from a child place's path and a typed answer into the child's
//! input method (`host_k_answer`), and holds a [`Host`] made of it, and a `for` over the
//! places whose rows are [`HostRow`]s: one per child place, built when the key appears in
//! the child's places after a tick and torn down at the end of the tick its key leaves (R7,
//! R8, the `for` machinery of the emitted code). Nothing here is plain data: the
//! child's elements are `E`, the program's `Elem` enum, and an answer is `A`, the program's
//! `Answer` enum (one variant per tag, carrying that tag's output type).
//!
//! A row calls the supplier with the place's path and its live element ([`Live`]: the
//! identity, the level over time read through the place's entry, and the entry) and builds the
//! blueprint it answers with at `host/[p]`, handing it a [`Sink`], the typed way back into
//! the child's place. The blueprint forms decide *when* an answer reaches the child:
//!
//! - **Forwarding** ([`element_blueprint`], `@Element e`) is re-emission: a new element of
//!   the host's own at `host/[p]/0` whose level follows `e`'s, and whose answers and
//!   pushes from the World go to the child's place in the same tick, as the tick's
//!   inputs. A host that forwards everything is observably not there. A host in a hosted
//!   scope re-emits into the enclosing host's document instead ([`Host::enclosing`]).
//! - **A computed answer** is delayed: an event output's firing ([`event_blueprint`]) or a
//!   dynamic output's later update ([`dynamic_blueprint`]) is queued with
//!   [`Cx::answer_later`] and delivered as an input of the next tick, which is owed
//!   (`pending`, like an armed `@Defer`). A dynamic output's first value is read in the
//!   birth tick, synchronously (as a World's first-value query is), and a plain output
//!   ([`plain_blueprint`]) is the first value once. A computed answer in flight to a place
//!   whose key left is dropped (`HostRow::left`). Nested hosts add one tick per computing
//!   layer and none per forwarding layer.
//!
//! **First values.** A child place with a first value asks its host at its build
//! ([`Host::first_value`]), before the owner's node exists: the supplier row for that place
//! is built early and kept for the builder, which finds it present. This is the one early
//! construction, and it is why a [`Host`] reaches its owner through a link filled in once
//! the node is made ([`Host::link`]).
//!
//! Written by Claude, 3 November 2026, on the claude-self-hosting branch. Subject to change.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use crate::cells::{Described, Dynamic, Element, Event, Handle, Hold, Live, Port, Snapshot, Stamped};
use crate::collections::Map;
use crate::cx::{Cx, Done, Node};
use crate::pod::{Fault, Path, Pod, Segment, ToPod};

// ---------------------------------------------------------------------------------------
// The document as `$(Path ==> $Elem c)`: one entry per place, the outer keyed by places
// ---------------------------------------------------------------------------------------

/// A level as a World reads it (claude/incremental-host-document.md, doubt WD): plain data,
/// with every dynamic it holds sampled at the read point, deep. An element whose children
/// are its host's document (`children: $(Path ==> $Elem)`) holds dynamics, so it has no
/// `ToPod`; its level goes to a World through this, and `level_ports` lists the update
/// events of the dynamics it holds, so a World place over it is sent again when any of them
/// moves. The program's `Elem` implements it (emitted beside it).
pub trait Level: Clone + 'static {
    fn level_pod(&self, cx: &Cx) -> Pod;
    fn level_ports(&self, cx: &Cx, out: &mut Vec<Rc<Port<()>>>) {
        let _ = (cx, out);
    }
}

impl Level for Pod {
    fn level_pod(&self, _: &Cx) -> Pod {
        self.clone()
    }
}

/// A map of dynamics as a World reads it: each entry sampled at the read point, deep.
pub fn dynamics_pod<K: ToPod + Ord + Clone, E: Level>(m: &Map<K, Dynamic<E>>, cx: &Cx) -> Pod {
    Pod::Map(m.iter().map(|(k, d)| (k.to_pod(), d.sample(cx).level_pod(cx))).collect())
}

/// The update events of a map of dynamics and of every dynamic their values hold.
pub fn dynamics_ports<K: Ord + Clone, E: Level>(m: &Map<K, Dynamic<E>>, cx: &Cx, out: &mut Vec<Rc<Port<()>>>) {
    for (_, d) in m.iter() {
        out.push(d.updates.clone());
        d.sample(cx).level_ports(cx, out);
    }
}

/// A World place whose level holds dynamics: subscribed to every one of them as it was last
/// sent, so a move of any nested entry patches the place again in the tick it moves (doubt
/// WD). Renewed each time the place is sent.
#[derive(Default)]
pub struct Watch {
    subs: RefCell<Vec<(Rc<Port<()>>, Handle)>>,
}

impl Watch {
    pub fn new() -> Watch {
        Watch::default()
    }
    /// The level sent as `node`'s place: its nested dynamics watched from now on.
    pub fn renew<E: Level>(&self, level: &E, node: Rc<dyn Node>, cx: &Cx) {
        self.drop_all();
        let mut ports = Vec::new();
        level.level_ports(cx, &mut ports);
        let weak = Rc::downgrade(&node);
        let subs = ports
            .into_iter()
            .map(|p| {
                let w = weak.clone();
                let h = p.subscribe(Rc::new(move |_: &(), cx: &mut Cx| {
                    if let Some(n) = w.upgrade() {
                        cx.dirty(n);
                    }
                    Ok(())
                }));
                (p, h)
            })
            .collect();
        *self.subs.borrow_mut() = subs;
    }
    /// The place left: nothing is watched.
    pub fn drop_all(&self) {
        for (p, h) in self.subs.borrow_mut().drain(..) {
            p.unsubscribe(h);
        }
    }
}

/// `@Flatten` over a dynamic the node holds as a value (a `for` row's entry `$$a`, a host
/// document's entry): the subscription to its current inner's update event, which tells the
/// node (`moved`, its emitted method) that the inner moved, so what reads the flatten reads
/// it again. Following another inner lets go of the last; dropped with its node.
#[derive(Default)]
pub struct Follow {
    sub: RefCell<Option<(Rc<Port<()>>, Handle)>>,
}

impl Follow {
    pub fn new() -> Follow {
        Follow::default()
    }
    /// From now on the inner `updates` tells `node` through `moved`.
    pub fn follow<N: 'static>(&self, updates: &Rc<Port<()>>, node: &Rc<N>, moved: fn(&Rc<N>, &mut Cx)) {
        self.drop_sub();
        let weak = Rc::downgrade(node);
        let h = updates.subscribe(Rc::new(move |_: &(), cx: &mut Cx| {
            if let Some(n) = weak.upgrade() {
                moved(&n, cx);
            }
            Ok(())
        }));
        *self.sub.borrow_mut() = Some((updates.clone(), h));
    }
    fn drop_sub(&self) {
        if let Some((p, h)) = self.sub.borrow_mut().take() {
            p.unsubscribe(h);
        }
    }
}

/// What a row that holds no block (a row shared by every call of a widget that calls
/// itself) tells in a hosted scope when a place of its may have moved: the node that
/// built it, through that node's own telling ([`moved_by`]).
pub type Moved = Rc<dyn Fn(&mut Cx)>;

/// `node`'s method `moved` as a [`Moved`]: the node held weakly, told while it lives.
pub fn moved_by<N: 'static>(node: &Rc<N>, moved: fn(&Rc<N>, &mut Cx)) -> Moved {
    let weak = Rc::downgrade(node);
    Rc::new(move |cx: &mut Cx| {
        if let Some(n) = weak.upgrade() {
            moved(&n, cx);
        }
    })
}

impl Drop for Follow {
    fn drop(&mut self) {
        self.drop_sub();
    }
}

/// One place of a hosted child, as its host's document holds it (Jimmy, 6 November 2026:
/// the document is `$(Path ==> $Elem c)`): a structural dynamic of its own (Semantics ch. 7,
/// round 3 ST, doubt EP). Its node takes a snapshot of the place's level at its build and
/// again whenever the place may have moved (its node's patch pass, as a World place is
/// sent); the dynamic the document holds for the place samples the snapshot of the tick
/// before (S(m-1) after `m`), and its update event fires at the start of the tick after a
/// take that changed the level, which that tick owes. Its comparison is the level's own
/// equality, a dynamic it holds equal only to itself (doubt EQ). The rows that read the
/// place as it stands (a host's row forwarding it, at any depth of hosts) are dirtied by a
/// take that moved it, so a forwarded element re-emits in the same tick.
pub struct Entry<E> {
    snap: Snapshot<E>,
    published: Rc<Port<()>>,
    dynamic: RefCell<Option<Dynamic<E>>>,
    watchers: RefCell<Vec<std::rc::Weak<dyn Node>>>,
}

impl<E: Clone + Eq + 'static> Entry<E> {
    /// An entry with no snapshot yet; `what` names the place in a fault.
    pub fn new(what: &'static str) -> Rc<Entry<E>> {
        let entry = Rc::new(Entry { snap: Snapshot::new(what), published: Rc::new(Port::new()), dynamic: RefCell::new(None), watchers: RefCell::new(Vec::new()) });
        let weak = Rc::downgrade(&entry);
        let read = Rc::new(move |cx: &Cx| weak.upgrade().expect("a place's entry is read only while its place lives").snap.get(cx));
        *entry.dynamic.borrow_mut() = Some(Dynamic { owner: None, read, updates: entry.published.clone() });
        entry
    }
    /// The place's level now: the first snapshot at its node's build, then each time the
    /// place may have moved, compared with the one that stood. A take that moved the level
    /// dirties the rows reading it as it stands and queues the publication, once per tick.
    pub fn take(self: &Rc<Self>, level: E, cx: &mut Cx) {
        if !self.snap.taken() {
            self.snap.take(level, cx);
            return;
        }
        if !self.snap.take_compared(level, cx) {
            return;
        }
        let rows: Vec<Rc<dyn Node>> = {
            let mut ws = self.watchers.borrow_mut();
            ws.retain(|w| w.strong_count() > 0);
            ws.iter().filter_map(std::rc::Weak::upgrade).collect()
        };
        for row in rows {
            cx.dirty(row);
        }
        if self.snap.queue(cx) {
            let me = self.clone();
            cx.answer_later(Box::new(move |cx| me.publish(cx)));
        }
    }
    /// At the start of the tick after a take that moved it: the entry's update event fires.
    fn publish(&self, cx: &mut Cx) -> Done {
        if self.snap.published(cx) { self.published.fire((), cx) } else { Ok(()) }
    }
    /// The place's level as it stands: the snapshot taken this tick from the construction
    /// step on, the one before during propagation.
    pub fn current(&self, cx: &Cx) -> E {
        self.snap.current(cx)
    }
    /// The dynamic the document holds for this place, `$Elem`: the same one for as long as
    /// the place lives.
    pub fn dynamic(&self) -> Dynamic<E> {
        self.dynamic.borrow().clone().expect("an entry's dynamic is made with it")
    }
    /// A row that reads the place as it stands: dirtied when a take moves it.
    pub fn watch(&self, row: std::rc::Weak<dyn Node>) {
        self.watchers.borrow_mut().push(row);
    }
}

/// A host's child document as one structural dynamic of the owner (pick ST): the outer of
/// `$(Path ==> $Elem c)`. `doc` is what the program reads, each place's dynamic by its path
/// relative to the child; `places` the entries themselves, which the host's own rows read.
/// It is taken when a place of the child came or went, and two are equal when they hold the
/// same entries at the same paths, so a value change of a place never moves it.
pub struct HostDoc<E> {
    pub doc: Map<Path, Dynamic<E>>,
    pub places: Map<Path, Rc<Entry<E>>>,
}

impl<E> Clone for HostDoc<E> {
    fn clone(&self) -> HostDoc<E> {
        HostDoc { doc: self.doc.clone(), places: self.places.clone() }
    }
}

impl<E: Clone + Eq + 'static> HostDoc<E> {
    pub fn new() -> HostDoc<E> {
        HostDoc { doc: Map::new(), places: Map::new() }
    }
    pub fn insert(&mut self, path: Path, entry: Rc<Entry<E>>) {
        self.doc.insert(path.clone(), entry.dynamic());
        self.places.insert(path, entry);
    }
}

impl<E: Clone + Eq + 'static> Default for HostDoc<E> {
    fn default() -> HostDoc<E> {
        HostDoc::new()
    }
}

impl<E> PartialEq for HostDoc<E> {
    fn eq(&self, other: &HostDoc<E>) -> bool {
        self.places.len() == other.places.len() && self.places.iter().zip(other.places.iter()).all(|((k1, e1), (k2, e2))| k1 == k2 && Rc::ptr_eq(e1, e2))
    }
}

impl<E> Eq for HostDoc<E> {}

/// `$each <- @PerKey $m` (Semantics Appendix B; round 9 PK, Jimmy's): a dynamic map taken
/// apart by key, `$(k ==> v)` into `$(k ==> $v)`, the shape a host's document has. The
/// outer is a hold the node's readers read, written only when a key came or went; each
/// key's inner is a hold of its own with its own update event, the same dynamic for as long
/// as the key stays (keys are identity, ch. 4), written and fired only when a write of `$m`
/// gives the key a value unequal (`==`) to the one it holds (`@Distinct`, per key). A new
/// key's inner starts at its value and fires nothing in its birth tick (R7); a key that
/// leaves drops out of the outer and its inner keeps its last value for the row that
/// still reads it in the leaving tick (R8), and a key that comes back gets a new inner. A
/// write of `$m` costs one `==` of the maps (at once when the map is the one last taken,
/// sharing being invisible, ch. 11) and otherwise one `==` per kept key.
pub struct PerKey<K, V> {
    outer: RefCell<Option<Hold<Map<K, Dynamic<V>>>>>,
    inners: RefCell<Map<K, PerKeyInner<V>>>,
    last: RefCell<Map<K, V>>,
}

/// One key's inner: its hold and update event, and the dynamic over them the outer holds.
#[derive(Clone)]
struct PerKeyInner<V> {
    cell: Rc<(Hold<V>, Rc<Port<()>>)>,
    dynamic: Dynamic<V>,
}

impl<V: Clone + 'static> PerKeyInner<V> {
    fn new(v: V, cx: &Cx) -> PerKeyInner<V> {
        let cell = Rc::new((Hold::new(v, cx), Rc::new(Port::new())));
        let held = cell.clone();
        let dynamic = Dynamic { owner: None, read: Rc::new(move |cx: &Cx| held.0.get(cx)), updates: cell.1.clone() };
        PerKeyInner { cell, dynamic }
    }
}

impl<K: Ord + Clone + 'static, V: Clone + Eq + 'static> Default for PerKey<K, V> {
    fn default() -> PerKey<K, V> {
        PerKey { outer: RefCell::new(None), inners: RefCell::new(Map::new()), last: RefCell::new(Map::new()) }
    }
}

impl<K: Ord + Clone + 'static, V: Clone + Eq + 'static> PerKey<K, V> {
    pub fn new() -> PerKey<K, V> {
        PerKey::default()
    }
    /// At the build: an inner per key of the source, each at its value, and the outer over
    /// them. Once; a read before the build has done it.
    pub fn start(&self, source: Map<K, V>, cx: &Cx) {
        if self.outer.borrow().is_some() {
            return;
        }
        let inners: Map<K, PerKeyInner<V>> = source.iter().map(|(k, v)| (k.clone(), PerKeyInner::new(v.clone(), cx))).collect();
        let outer: Map<K, Dynamic<V>> = inners.iter().map(|(k, i)| (k.clone(), i.dynamic.clone())).collect();
        *self.outer.borrow_mut() = Some(Hold::new(outer, cx));
        *self.inners.borrow_mut() = inners;
        *self.last.borrow_mut() = source;
    }
    /// The outer at the read point; before the build, set up from `source` first.
    pub fn outer(&self, cx: &Cx, source: impl FnOnce() -> Map<K, V>) -> Map<K, Dynamic<V>> {
        if self.outer.borrow().is_none() {
            self.start(source(), cx);
        }
        self.outer.borrow().as_ref().expect("`@PerKey` is set up").get(cx)
    }
    /// `$m` written with `source` (its update event's value): each kept key whose value
    /// changed has its inner written and fired, the new keys get inners, and the outer is
    /// written when a key came or went. Answers whether the outer moved.
    pub fn written(&self, source: Map<K, V>, cx: &mut Cx) -> bool {
        if self.outer.borrow().is_none() {
            self.start(source, cx);
            return false;
        }
        if source == *self.last.borrow() {
            return false;
        }
        // one walk of both in key order: the kept keys compared, the new ones given inners
        let old = self.inners.borrow().clone();
        let mut fired: Vec<Rc<Port<()>>> = Vec::new();
        let mut born: Vec<(K, PerKeyInner<V>)> = Vec::new();
        let mut moved = false;
        let mut olds = old.iter().peekable();
        for (k, v) in source.iter() {
            // the keys that left before this one
            while olds.peek().is_some_and(|(ok, _)| *ok < k) {
                olds.next();
                moved = true;
            }
            match olds.peek() {
                Some((ok, inner)) if *ok == k => {
                    if inner.cell.0.newest() != *v {
                        inner.cell.0.set(v.clone(), cx);
                        fired.push(inner.cell.1.clone());
                    }
                    olds.next();
                }
                _ => {
                    born.push((k.clone(), PerKeyInner::new(v.clone(), cx)));
                    moved = true;
                }
            }
        }
        if olds.next().is_some() {
            moved = true;
        }
        // a key came or went: the inners by the source's keys, and the outer over them
        if moved {
            let mut born = born.into_iter().peekable();
            let next: Vec<(K, PerKeyInner<V>)> = source
                .keys()
                .map(|k| match old.get(k) {
                    Some(i) => (k.clone(), i.clone()),
                    None => born.next().expect("a new key has its inner"),
                })
                .collect();
            let outer: Map<K, Dynamic<V>> = next.iter().map(|(k, i)| (k.clone(), i.dynamic.clone())).collect();
            *self.inners.borrow_mut() = next.into_iter().collect();
            self.outer.borrow().as_ref().expect("`@PerKey` is set up").set(outer, cx);
        }
        *self.last.borrow_mut() = source;
        for p in fired {
            let done = p.fire((), cx);
            cx.keep(done);
        }
        moved
    }
}

/// The way back into a child's place: a typed answer, delivered now.
pub type Sink<A> = Rc<dyn Fn(&A, &mut Cx) -> Done>;

/// What a blueprint built for a child place gives back: its node (whose places are under
/// the row's path), the place's first value when the output has one, and whether the
/// blueprint forwards the element (then the child place's requests go out with it, at
/// the row's `0`, and an enclosing host's answers to that place come back to the child).
pub struct Supplied<E, A> {
    pub node: Rc<dyn Node>,
    pub first: Option<A>,
    pub forwards: bool,
    /// A place of the blueprint's own at the row's `0` in a hosted scope (an element value
    /// placed, [`described_blueprint`]): its entry, which the enclosing host's document
    /// holds.
    pub entry: Option<Rc<Entry<E>>>,
    /// How many places of its own the blueprint built from the base it was given (a
    /// widget's blueprint, [`widget_blueprint`]; ruling TC: places are counted as built).
    pub built: u32,
    /// How a widget's blueprint leaves: its row's places torn down, the row with them.
    /// The other forms leave by their node.
    pub leave: Option<Rc<dyn Fn(&mut Cx)>>,
    /// A widget's blueprint built in a hosted scope: its row's own places, entries of the
    /// enclosing host's document, by their paths relative to the path it was built at.
    pub document: Option<RowDocument<E>>,
    /// A widget's blueprint built in a hosted scope: an enclosing host's typed answer to
    /// a place of its row's, the position counted from the row's first.
    pub deliver: Option<RowDeliver<A>>,
}

/// A row's own places in a hosted scope: each place's entry by its path relative to the
/// path the row was built at.
pub type RowDocument<E> = Rc<dyn Fn(&Cx) -> Vec<(Path, Rc<Entry<E>>)>>;

/// A host's typed answer to a place of a row's own, the position counted from the row's
/// first.
pub type RowDeliver<A> = Rc<dyn Fn(&[Segment], &A, &mut Cx) -> Done>;

/// `@o` as a value (claude/backend.md, "Types"): built at a path with the sink that answers
/// the child's place, in the scope of the host whose row builds it, its own places
/// numbered from the base it is given (0 under a host's row; a block that applies the
/// value gives its next position as built).
pub type Blueprint<E, A> = Rc<dyn Fn(Path, Sink<A>, &Scope<E, A>, u32, &mut Cx) -> Result<Supplied<E, A>, Fault>>;

/// A host's supplier, in the `@HostDocument` form: a blueprint for every child place.
pub type Supplier<E, A> = Rc<dyn Fn(&Path, &Element<E>) -> Blueprint<E, A>>;

/// `@HostSome`'s supplier made `@HostDocument`'s (Semantics ch. 9, the derivation): `nil`
/// means "not mine, forward it".
pub fn forward_on_none<E: Level + Eq, A: AnswerOf<E>>(some: Rc<dyn Fn(&Path, &Element<E>) -> Option<Blueprint<E, A>>>) -> Supplier<E, A> {
    Rc::new(move |p: &Path, e: &Element<E>| match some(p, e) {
        Some(b) => b,
        None => element_blueprint(e),
    })
}

/// The program's answer type, which a World's answer to an element decodes into by the
/// element's tag (emitted beside `Elem`).
pub trait AnswerOf<E>: Clone + 'static {
    fn decode(element: &E, v: &Pod) -> Result<Self, Fault>;
    /// A World's first value for the element: an answer, but for a bundle's tag, whose first
    /// value is its plain parts (`Answer`'s first-value variant, pick HO).
    fn decode_first(element: &E, v: &Pod) -> Result<Self, Fault> {
        Self::decode(element, v)
    }
    /// An answer held whole (a bundle with a part that is no plain data, made outside a
    /// tag's arm), as the parts the element's tag takes: its state and its events, which a
    /// host's place is answered with as [`parts_blueprint`] answers. `None` for any other
    /// answer, and for an element whose tag has no such bundle.
    fn held_parts(_element: &E, _answer: &Self) -> Option<PartsAnswer<Self>> {
        None
    }
}

/// Where a node's places go (claude/backend.md, "Hosting"): the document, or an enclosing
/// host's document, which answers a first-value query itself.
pub enum Scope<E, A> {
    World,
    Hosted(Rc<dyn HostOf<E, A>>),
}

// ---------------------------------------------------------------------------------------
// The blueprint forms
// ---------------------------------------------------------------------------------------

/// A node with no places and nothing to do.
struct Inert;
impl Node for Inert {}

/// `@ x`: no places, the first value `x`, once.
pub fn plain_blueprint<E: 'static, A: Clone + 'static>(x: A) -> Blueprint<E, A> {
    Rc::new(move |_path, _sink, _scope, _base, _cx| Ok(Supplied { node: Rc::new(Inert), first: Some(x.clone()), forwards: false, entry: None, built: 0, leave: None, document: None, deliver: None }))
}

/// A blueprint whose answers are made again: its first value and each answer it gives the
/// sink of what applies it, through `walk` (a function a blueprint holds, made the
/// function its reader's type names).
pub fn answers_mapped<E: 'static, A: Clone + 'static>(blueprint: Blueprint<E, A>, walk: impl Fn(&A) -> A + 'static) -> Blueprint<E, A> {
    let walk = Rc::new(walk);
    Rc::new(move |path, sink: Sink<A>, scope: &Scope<E, A>, base, cx| {
        let walked = walk.clone();
        let mapped: Sink<A> = Rc::new(move |answer: &A, cx: &mut Cx| sink(&walked(answer), cx));
        let mut supplied = blueprint(path, mapped, scope, base, cx)?;
        supplied.first = supplied.first.as_ref().map(|first| walk(first));
        Ok(supplied)
    })
}

/// `@ never`: no places, no first value, no answer.
pub fn never_blueprint<E: 'static, A: 'static>() -> Blueprint<E, A> {
    Rc::new(move |_path, _sink, _scope, _base, _cx| Ok(Supplied { node: Rc::new(Inert), first: None, forwards: false, entry: None, built: 0, leave: None, document: None, deliver: None }))
}

/// A widget's row as its blueprint built it: the row's node, how it leaves, how many
/// places it built, and the widget's plain result (unit when it gives none).
pub struct BuiltWidget<E, A, R> {
    pub node: Rc<dyn Node>,
    pub leave: Rc<dyn Fn(&mut Cx)>,
    pub built: u32,
    pub result: R,
    /// The row's own places as entries of a host's document (none for a row whose places
    /// are the World's).
    pub document: RowDocument<E>,
    /// A host's typed answer to a place of the row's own.
    pub deliver: RowDeliver<A>,
}

/// A widget with places as a blueprint value (Semantics ch. 8: a blueprint builds nothing
/// until it is applied): `build` makes the widget's row at the path, its places numbered
/// from the base, in the World's scope; `build_hosted` makes it in a hosted scope, its
/// places entries of the host's document, given the host and what tells the host's owner
/// that a place of the row's came or went ([`HostOf::moved`]); `answer` makes the widget's
/// plain result the place's first value (nothing for a widget that gives none).
pub fn widget_blueprint<E: 'static, A: 'static, R: 'static>(
    build: impl Fn(Path, u32, &mut Cx) -> Result<BuiltWidget<E, A, R>, Fault> + 'static,
    build_hosted: impl Fn(Path, u32, Rc<dyn HostOf<E, A>>, Moved, &mut Cx) -> Result<BuiltWidget<E, A, R>, Fault> + 'static,
    answer: impl Fn(R) -> Option<A> + 'static,
) -> Blueprint<E, A> {
    Rc::new(move |path, _sink, scope: &Scope<E, A>, base, cx| match scope {
        Scope::World => {
            let made = build(path, base, cx)?;
            Ok(Supplied { node: made.node, first: answer(made.result), forwards: false, entry: None, built: made.built, leave: Some(made.leave), document: None, deliver: None })
        }
        Scope::Hosted(host) => {
            let made = build_hosted(path, base, host.clone(), host_moved(host), cx)?;
            Ok(Supplied { node: made.node, first: answer(made.result), forwards: false, entry: None, built: made.built, leave: Some(made.leave), document: Some(made.document), deliver: Some(made.deliver) })
        }
    })
}

/// A widget that gives an event back as a blueprint value: as [`widget_blueprint`], the
/// row's result the port its event fires. Each firing is an answer to the place the
/// blueprint answers, made of the event's value by `answer` and given to the sink of
/// what applied the blueprint, in the tick of the firing. The subscription lives as long
/// as the row, whose port it is.
pub fn widget_event_blueprint<E: 'static, A: 'static, T: Clone + 'static>(
    build: impl Fn(Path, u32, &mut Cx) -> Result<BuiltWidget<E, A, Rc<Port<T>>>, Fault> + 'static,
    build_hosted: impl Fn(Path, u32, Rc<dyn HostOf<E, A>>, Moved, &mut Cx) -> Result<BuiltWidget<E, A, Rc<Port<T>>>, Fault> + 'static,
    answer: impl Fn(&T) -> A + 'static,
) -> Blueprint<E, A> {
    let answer = Rc::new(answer);
    Rc::new(move |path, sink: Sink<A>, scope: &Scope<E, A>, base, cx| {
        let (made, hosted) = match scope {
            Scope::World => (build(path, base, cx)?, false),
            Scope::Hosted(host) => (build_hosted(path, base, host.clone(), host_moved(host), cx)?, true),
        };
        let answered = answer.clone();
        let _ = made.result.subscribe(Rc::new(move |v: &T, cx: &mut Cx| sink(&answered(v), cx)));
        let (document, deliver) = if hosted { (Some(made.document), Some(made.deliver)) } else { (None, None) };
        Ok(Supplied { node: made.node, first: None, forwards: false, entry: None, built: made.built, leave: Some(made.leave), document, deliver })
    })
}

/// A widget that gives a dynamic back as a blueprint value under a tag's arm: as
/// [`widget_blueprint`], the row's result its live dynamic. Its value at the row's build
/// is the place's first value, made an answer by `answer`; each later move of it is a
/// push to the place, an input of the next tick, which is owed, sampled when it is
/// delivered, as [`dynamic_blueprint`]'s is (several marks in one tick are one push).
/// The subscription lives as long as the row, whose port it is.
pub fn widget_dynamic_blueprint<E: 'static, A: 'static, T: Clone + 'static>(
    build: impl Fn(Path, u32, &mut Cx) -> Result<BuiltWidget<E, A, Dynamic<T>>, Fault> + 'static,
    build_hosted: impl Fn(Path, u32, Rc<dyn HostOf<E, A>>, Moved, &mut Cx) -> Result<BuiltWidget<E, A, Dynamic<T>>, Fault> + 'static,
    answer: impl Fn(&T) -> A + 'static,
) -> Blueprint<E, A> {
    let answer = Rc::new(answer);
    Rc::new(move |path, sink: Sink<A>, scope: &Scope<E, A>, base, cx| {
        let (made, hosted) = match scope {
            Scope::World => (build(path, base, cx)?, false),
            Scope::Hosted(host) => (build_hosted(path, base, host.clone(), host_moved(host), cx)?, true),
        };
        let first = answer(&made.result.sample(cx));
        let (read, answered) = (made.result.clone(), answer.clone());
        let queued: Stamped<()> = Stamped::new();
        let _ = made.result.updates.subscribe(Rc::new(move |_: &(), cx: &mut Cx| {
            if queued.fired(cx) {
                return Ok(());
            }
            queued.set((), cx);
            let (read, sink, answered) = (read.clone(), sink.clone(), answered.clone());
            cx.answer_later(Box::new(move |cx| sink(&answered(&read.sample(cx)), cx)));
            Ok(())
        }));
        let (document, deliver) = if hosted { (Some(made.document), Some(made.deliver)) } else { (None, None) };
        Ok(Supplied { node: made.node, first: Some(first), forwards: false, entry: None, built: made.built, leave: Some(made.leave), document, deliver })
    })
}

/// A host's telling as a [`Moved`]: the host held weakly, told while it lives.
pub fn host_moved<E: 'static, A: 'static>(host: &Rc<dyn HostOf<E, A>>) -> Moved {
    let weak = Rc::downgrade(host);
    Rc::new(move |cx: &mut Cx| {
        if let Some(h) = weak.upgrade() {
            h.moved(cx);
        }
    })
}

/// The rows a blueprint value of a block's own widget built (Semantics ch. 8: a blueprint
/// builds as often as it is applied), held weakly: the block's write behind what their
/// places read marks each one that lives. A row that left is dropped from the list when
/// the next one is added.
pub struct WeakRows<N>(Rc<RefCell<Vec<Weak<N>>>>);

impl<N> Clone for WeakRows<N> {
    fn clone(&self) -> WeakRows<N> {
        WeakRows(self.0.clone())
    }
}

impl<N> Default for WeakRows<N> {
    fn default() -> WeakRows<N> {
        WeakRows(Rc::new(RefCell::new(Vec::new())))
    }
}

impl<N> WeakRows<N> {
    pub fn new() -> WeakRows<N> {
        WeakRows::default()
    }
    /// A row the blueprint built.
    pub fn add(&self, row: &Rc<N>) {
        let mut rows = self.0.borrow_mut();
        rows.retain(|w| w.strong_count() > 0);
        rows.push(Rc::downgrade(row));
    }
    /// The rows that live.
    pub fn live(&self) -> Vec<Rc<N>> {
        self.0.borrow().iter().filter_map(Weak::upgrade).collect()
    }
}

/// A subscription a blueprint's node holds, let go when the row leaves.
struct Subscribed<T> {
    port: Rc<Port<T>>,
    handle: Cell<Option<Handle>>,
}

impl<T: Clone + 'static> Node for Subscribed<T> {
    fn leave(self: Rc<Self>, _cx: &mut Cx) -> Done {
        if let Some(h) = self.handle.take() {
            self.port.unsubscribe(h);
        }
        Ok(())
    }
}

/// The subscriptions of a bundle answer ([`parts_blueprint`]): one per event part, and one
/// per port that says a `$` part moved, let go when the row leaves.
struct SubscribedParts<T> {
    events: Vec<(Rc<Port<T>>, Cell<Option<Handle>>)>,
    updates: Vec<(Rc<Port<()>>, Cell<Option<Handle>>)>,
}

impl<T: Clone + 'static> Node for SubscribedParts<T> {
    fn leave(self: Rc<Self>, _cx: &mut Cx) -> Done {
        for (port, handle) in &self.events {
            if let Some(h) = handle.take() {
                port.unsubscribe(h);
            }
        }
        for (port, handle) in &self.updates {
            if let Some(h) = handle.take() {
                port.unsubscribe(h);
            }
        }
        Ok(())
    }
}

/// A computed answer: delivered as an input of the next tick, which is owed.
fn later<A: Clone + 'static>(sink: &Sink<A>, v: A, cx: &mut Cx) -> Done {
    let sink = sink.clone();
    cx.answer_later(Box::new(move |cx| sink(&v, cx)));
    Ok(())
}

/// `@ !e`: each firing of `e` is an answer to the child's place, a tick late.
pub fn event_blueprint<E: 'static, A: Clone + 'static>(e: Event<A>) -> Blueprint<E, A> {
    Rc::new(move |_path, sink: Sink<A>, _scope, _base, _cx| {
        let node = Rc::new(Subscribed { port: e.port.clone(), handle: Cell::new(None) });
        let h = e.port.subscribe(Rc::new(move |v: &A, cx: &mut Cx| later(&sink, v.clone(), cx)));
        node.handle.set(Some(h));
        Ok(Supplied { node, first: None, forwards: false, entry: None, built: 0, leave: None, document: None, deliver: None })
    })
}

/// A bundle answer's parts (Type System ch. 2, "Bundles"): `@ ((a, $b, !c, !d))` under a
/// bundle tag's arm, any mix of plain, `$` and event parts. The plain and `$` parts are
/// the place's state, one dynamic of the tag's state case (a constant when no part
/// moves), with a port per `$` part that says the part moved; each event part is an event
/// of the tag's answer case.
pub struct PartsAnswer<A> {
    state: Option<Dynamic<A>>,
    moved: Vec<Rc<Port<()>>>,
    events: Vec<Event<A>>,
}

/// A bundle answer with a state: its plain and `$` parts.
pub fn parts_state<A>(state: Dynamic<A>) -> PartsAnswer<A> {
    PartsAnswer { state: Some(state), moved: Vec::new(), events: Vec::new() }
}

/// A bundle answer with no state: its first event part.
pub fn parts_event<A>(e: Event<A>) -> PartsAnswer<A> {
    PartsAnswer { state: None, moved: Vec::new(), events: vec![e] }
}

impl<A> PartsAnswer<A> {
    /// One more port that says the state moved (a second `$` part's).
    pub fn moved_by(mut self, port: &Rc<Port<()>>) -> PartsAnswer<A> {
        self.moved.push(port.clone());
        self
    }
    /// One more event part.
    pub fn event(mut self, e: Event<A>) -> PartsAnswer<A> {
        self.events.push(e);
        self
    }
}

/// `@ ((a, $b, !c))` for a bundle's tag (pick HO; round 3 DY): the state's value at the
/// row's build is the place's first value, read synchronously; each later move of a `$`
/// part is a push of the whole state to the place, and each firing of an event part an
/// answer, all inputs of the next tick, which is owed. As [`dynamic_blueprint`], several
/// marks of the state in one tick are one push, sampled when it is delivered. Two event
/// parts that fire in one tick are two answers of that next tick.
pub fn parts_blueprint<E: 'static, A: Clone + 'static>(parts: PartsAnswer<A>) -> Blueprint<E, A> {
    let parts = Rc::new(parts);
    Rc::new(move |_path, sink: Sink<A>, _scope, _base, cx| {
        let (first, node) = answer_parts(&parts, &sink, cx);
        Ok(Supplied { node: Rc::new(node), first, forwards: false, entry: None, built: 0, leave: None, document: None, deliver: None })
    })
}

/// A bundle answer given to a sink ([`parts_blueprint`], [`widget_parts_blueprint`]): the
/// state's value now, the place's first value, and the subscriptions that push each
/// later move of the state and answer each firing of an event part, a tick late.
fn answer_parts<A: Clone + 'static>(parts: &PartsAnswer<A>, sink: &Sink<A>, cx: &mut Cx) -> (Option<A>, SubscribedParts<A>) {
    let first = parts.state.as_ref().map(|d| d.sample(cx));
    let mut events = Vec::new();
    for e in &parts.events {
        let answers = sink.clone();
        let h = e.port.subscribe(Rc::new(move |v: &A, cx: &mut Cx| later(&answers, v.clone(), cx)));
        events.push((e.port.clone(), Cell::new(Some(h))));
    }
    let mut updates = Vec::new();
    if let Some(d) = &parts.state {
        let queued: Rc<Stamped<()>> = Rc::new(Stamped::new());
        let mut ports = vec![d.updates.clone()];
        for port in &parts.moved {
            if !ports.iter().any(|p| Rc::ptr_eq(p, port)) {
                ports.push(port.clone());
            }
        }
        for port in ports {
            let (read, sink, queued) = (d.clone(), sink.clone(), queued.clone());
            let h = port.subscribe(Rc::new(move |_: &(), cx: &mut Cx| {
                if queued.fired(cx) {
                    return Ok(());
                }
                queued.set((), cx);
                let (read, sink) = (read.clone(), sink.clone());
                cx.answer_later(Box::new(move |cx| sink(&read.sample(cx), cx)));
                Ok(())
            }));
            updates.push((port, Cell::new(Some(h))));
        }
    }
    (first, SubscribedParts { events, updates })
}

/// A widget that gives a bundle back as a blueprint value under a bundle tag's arm: as
/// [`widget_blueprint`], the row's result its one value, its events and dynamics whole.
/// `parts` makes the bundle answer of it (the tag's state of its plain and `$` parts,
/// the tag's answers of its event parts), which is answered as [`parts_blueprint`]
/// answers: the state at the row's build the place's first value, each later move a
/// push and each firing an answer, inputs of the next tick. The subscriptions live as
/// long as the row, whose ports they are.
pub fn widget_parts_blueprint<E: 'static, A: Clone + 'static, R: 'static>(
    build: impl Fn(Path, u32, &mut Cx) -> Result<BuiltWidget<E, A, R>, Fault> + 'static,
    build_hosted: impl Fn(Path, u32, Rc<dyn HostOf<E, A>>, Moved, &mut Cx) -> Result<BuiltWidget<E, A, R>, Fault> + 'static,
    parts: impl Fn(&R) -> PartsAnswer<A> + 'static,
) -> Blueprint<E, A> {
    Rc::new(move |path, sink: Sink<A>, scope: &Scope<E, A>, base, cx| {
        let (made, hosted) = match scope {
            Scope::World => (build(path, base, cx)?, false),
            Scope::Hosted(host) => (build_hosted(path, base, host.clone(), host_moved(host), cx)?, true),
        };
        let (first, _subscribed) = answer_parts(&parts(&made.result), &sink, cx);
        let (document, deliver) = if hosted { (Some(made.document), Some(made.deliver)) } else { (None, None) };
        Ok(Supplied { node: made.node, first, forwards: false, entry: None, built: made.built, leave: Some(made.leave), document, deliver })
    })
}

/// `@ $d`: the value after the birth tick is the place's first value, read now,
/// synchronously, as a World's first-value query is; each later change of `d` is a push to
/// the child's place as an input of the next tick, which is owed (Semantics ch. 9 as ruled,
/// round 3 DY). The port says only that `d` moved: the value is sampled when the push is
/// delivered, at the next tick's read point, so it is the value after the tick of the
/// change whatever height the change came at, and several marks in one tick are one push.
pub fn dynamic_blueprint<E: 'static, A: Clone + 'static>(d: Dynamic<A>) -> Blueprint<E, A> {
    Rc::new(move |_path, sink: Sink<A>, _scope, _base, cx| {
        let first = d.sample(cx);
        let node = Rc::new(Subscribed { port: d.updates.clone(), handle: Cell::new(None) });
        let read = d.clone();
        let queued: Stamped<()> = Stamped::new();
        let h = d.updates.subscribe(Rc::new(move |_: &(), cx: &mut Cx| {
            if queued.fired(cx) {
                return Ok(());
            }
            queued.set((), cx);
            let (read, sink) = (read.clone(), sink.clone());
            cx.answer_later(Box::new(move |cx| sink(&read.sample(cx), cx)));
            Ok(())
        }));
        node.handle.set(Some(h));
        Ok(Supplied { node, first: Some(first), forwards: false, entry: None, built: 0, leave: None, document: None, deliver: None })
    })
}

/// The owner a supplier's dynamic answer reads through (`@ $d`, round 3 DY): the supplier
/// is made in the owner's build, before its node, and reads the node's getter for `d` once
/// it is linked ([`Link::link`], beside [`Host::link`]). While the owner is being built, an
/// early row's first value reads the dynamic's value at the build instead (the emitted
/// read's other arm), since nothing fires at a build.
pub struct Link<P>(Rc<RefCell<Weak<P>>>);

impl<P> Clone for Link<P> {
    fn clone(&self) -> Link<P> {
        Link(self.0.clone())
    }
}

impl<P> Default for Link<P> {
    fn default() -> Link<P> {
        Link(Rc::new(RefCell::new(Weak::new())))
    }
}

impl<P> Link<P> {
    pub fn new() -> Link<P> {
        Link::default()
    }
    pub fn link(&self, owner: &Rc<P>) {
        *self.0.borrow_mut() = Rc::downgrade(owner);
    }
    /// The owner, once linked and while it lives.
    pub fn owner(&self) -> Option<Rc<P>> {
        self.0.borrow().upgrade()
    }
    /// The owner, as `Weak::upgrade` gives it: a generation built before its block's node
    /// is made holds its block through a link (a static widget of several clauses whose
    /// result the block reads at its build), and reads it as a row reads its `parent`.
    pub fn upgrade(&self) -> Option<Rc<P>> {
        self.owner()
    }
}

/// `@Element e`: the child's element re-emitted at `path/0`, its level following the
/// child's, every answer and push there handed to the child's place in the same tick (the
/// transparent rule). In the World's scope the place goes out with `cx.added` and its first
/// value is the World's; in a hosted scope nothing goes out (the enclosing host's document
/// reads the row), and the first value is the enclosing host's.
///
/// `@Element d` of a description places it ([`described_blueprint`]).
pub fn element_blueprint<E: Level + Eq, A: AnswerOf<E>>(e: &Element<E>) -> Blueprint<E, A> {
    let e = match e {
        Element::Live(l) => l.clone(),
        Element::Described(d) => return described_blueprint(d),
    };
    Rc::new(move |path, sink: Sink<A>, scope: &Scope<E, A>, base, cx| {
        let at = path.at(base);
        // The level at the build is the identity: a row is built from the entry after the
        // tick, and an early row's entry cannot be read until its host's node is made.
        let element = e.identity.level_pod(cx);
        let first = match scope {
            Scope::World => cx.first_value(&element).map(|v| A::decode_first(&e.identity, &v)).transpose()?,
            // the forwarded element is the same place for the enclosing host: its entry
            Scope::Hosted(host) => host.first_value(&at, &e.identity, &e.entry, cx)?,
        };
        let node = Rc::new(Forward { at: at.clone(), element: e.identity.clone(), level: e.level.clone(), sink, handle: Cell::new(None) });
        if let Scope::World = scope {
            let weak = Rc::downgrade(&node);
            let h = e.level.updates.subscribe(Rc::new(move |_: &(), cx: &mut Cx| {
                if let Some(n) = weak.upgrade() {
                    let now = n.level.sample(cx).level_pod(cx);
                    cx.changed(n.at.clone(), now);
                }
                Ok(())
            }));
            node.handle.set(Some(h));
            cx.added(at, element.clone());
            if first.is_some() {
                cx.listen(element, &(node.clone() as Rc<dyn Node>));
            }
        }
        // in a hosted scope the forwarded element is the child's place itself: its entry
        let entry = match scope {
            Scope::World => None,
            Scope::Hosted(_) => Some(e.entry.clone()),
        };
        Ok(Supplied { node, first, forwards: true, entry, built: 1, leave: None, document: None, deliver: None })
    })
}

/// `@Element d` of a description (Type System ch. 2: an element applied is a plain value
/// until a statement or a host places it): a new element at `path/0`, as an inline element
/// statement there is. Its level is read at the build and sent (`cx.added`), and sent again
/// in the tick it moves; its first value is the World's, decoded by its tag; every answer
/// and push there is handed to the sink, the child's place; each firing of an event field
/// is the place's request. In a hosted scope the place is the row's own in the enclosing
/// host's document: an entry of its own (`Supplied::entry`), which takes its level at the
/// build and each time it moves; an enclosing host's answer there is handed to the sink,
/// and a request there goes to the enclosing host ([`HostOf::request_from`]).
pub fn described_blueprint<E: Level + Eq, A: AnswerOf<E>>(d: &Described<E>) -> Blueprint<E, A> {
    let d = d.clone();
    Rc::new(move |path, sink: Sink<A>, scope: &Scope<E, A>, base, cx| {
        let at = path.at(base);
        let identity = d.level.sample(cx);
        let node = Rc::new(Placed { at: at.clone(), element: identity.clone(), level: d.level.clone(), sink, handle: Cell::new(None), requests: RefCell::new(Vec::new()), entry: RefCell::new(None) });
        let weak = Rc::downgrade(&node);
        match scope {
            Scope::World => {
                let element = identity.level_pod(cx);
                let first = cx.first_value(&element).map(|v| A::decode_first(&identity, &v)).transpose()?;
                // the level's update event fires during propagation, where a read still gives
                // the value before the tick: the node is marked, and its patch reads the level
                let h = d.level.updates.subscribe(Rc::new(move |_: &(), cx: &mut Cx| {
                    if let Some(n) = weak.upgrade() {
                        cx.dirty(n);
                    }
                    Ok(())
                }));
                node.handle.set(Some(h));
                for (field, firings) in d.requests.iter() {
                    let (to, field) = (at.clone(), field.clone());
                    let h = firings.port.subscribe(Rc::new(move |v: &Pod, cx: &mut Cx| {
                        cx.request(to.clone(), field.as_deref(), v.clone());
                        Ok(())
                    }));
                    node.requests.borrow_mut().push((firings.port.clone(), h));
                }
                cx.added(at, element.clone());
                if first.is_some() {
                    cx.listen(element, &(node.clone() as Rc<dyn Node>));
                }
                Ok(Supplied { node, first, forwards: false, entry: None, built: 1, leave: None, document: None, deliver: None })
            }
            Scope::Hosted(host) => {
                // a place of the row's own in the enclosing host's document: an entry that
                // takes the level at the build and again each time it moves
                let entry = Entry::new("an element value's place");
                entry.take(identity.clone(), cx);
                let first = host.first_value(&at, &identity, &entry, cx)?;
                *node.entry.borrow_mut() = Some(entry.clone());
                let h = d.level.updates.subscribe(Rc::new(move |_: &(), cx: &mut Cx| {
                    if let Some(n) = weak.upgrade() {
                        let now = n.level.sample(cx);
                        if let Some(e) = n.entry.borrow().as_ref() {
                            e.take(now, cx);
                        }
                    }
                    Ok(())
                }));
                node.handle.set(Some(h));
                // each firing of an event field is the place's request, a request of the
                // enclosing host's child: its row for the place hears it and hands it on
                for (field, firings) in d.requests.iter() {
                    let (to, field, outer) = (at.clone(), field.clone(), host.clone());
                    let h = firings.port.subscribe(Rc::new(move |v: &Pod, cx: &mut Cx| outer.request_from(&to, field.as_deref(), v.clone(), cx)));
                    node.requests.borrow_mut().push((firings.port.clone(), h));
                }
                Ok(Supplied { node, first, forwards: false, entry: Some(entry), built: 1, leave: None, document: None, deliver: None })
            }
        }
    })
}

/// A placed description's node: one place, the description's element.
struct Placed<E, A> {
    at: Path,
    /// The level at the build: its tag decodes the place's answers.
    element: E,
    level: Dynamic<E>,
    sink: Sink<A>,
    /// The re-emission on the level's updates.
    handle: Cell<Option<Handle>>,
    /// The event fields' subscriptions, the place's requests.
    requests: RefCell<Vec<(Rc<Port<Pod>>, Handle)>>,
    /// In a hosted scope, the place's entry in the enclosing host's document.
    entry: RefCell<Option<Rc<Entry<E>>>>,
}

impl<E: Level + Eq, A: AnswerOf<E>> Node for Placed<E, A> {
    fn answer(self: Rc<Self>, at: &[Segment], v: &Pod, cx: &mut Cx) -> Done {
        match at {
            [Segment::Pos(0)] => (self.sink)(&A::decode(&self.element, v)?, cx),
            _ => Err(Fault::answer(format!("no place at {} under a placed element value", Path(at.to_vec())))),
        }
    }
    /// A push is to the element's `$` output: of a bundle's tag its `$` part, which the
    /// tag's first-value case carries (`decode_first`; any other tag's is its answer).
    fn push(self: Rc<Self>, _element: &Pod, v: &Pod, cx: &mut Cx) -> Done {
        (self.sink)(&A::decode_first(&self.element, v)?, cx)
    }
    /// The World's place after its level's update event fired: the level at the read
    /// point, sent again (the diff decides whether it moved).
    fn patch(self: Rc<Self>, cx: &mut Cx) -> Done {
        if self.entry.borrow().is_none() {
            let now = self.level.sample(cx).level_pod(cx);
            cx.changed(self.at.clone(), now);
        }
        Ok(())
    }
    fn leave(self: Rc<Self>, cx: &mut Cx) -> Done {
        if let Some(h) = self.handle.take() {
            self.level.updates.unsubscribe(h);
            if self.entry.borrow().is_none() {
                cx.removed(self.at.clone());
            }
        }
        for (port, h) in self.requests.borrow_mut().drain(..) {
            port.unsubscribe(h);
        }
        Ok(())
    }
}

/// A forwarding blueprint's node: one place, the child's element re-emitted.
struct Forward<E, A> {
    at: Path,
    element: E,
    level: Dynamic<E>,
    sink: Sink<A>,
    /// The re-emission on the level's updates, in the World's scope.
    handle: Cell<Option<Handle>>,
}

impl<E: Level, A: AnswerOf<E>> Node for Forward<E, A> {
    fn answer(self: Rc<Self>, at: &[Segment], v: &Pod, cx: &mut Cx) -> Done {
        match at {
            [Segment::Pos(0)] => (self.sink)(&A::decode(&self.element, v)?, cx),
            _ => Err(Fault::answer(format!("no place at {} under a forwarded element", Path(at.to_vec())))),
        }
    }
    /// A push is to the element's `$` output: of a bundle's tag its `$` part, which the
    /// tag's first-value case carries (`decode_first`; any other tag's is its answer).
    fn push(self: Rc<Self>, _element: &Pod, v: &Pod, cx: &mut Cx) -> Done {
        (self.sink)(&A::decode_first(&self.element, v)?, cx)
    }
    fn leave(self: Rc<Self>, cx: &mut Cx) -> Done {
        if let Some(h) = self.handle.take() {
            self.level.updates.unsubscribe(h);
            cx.removed(self.at.clone());
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------------------
// The host and the rows of its `for`
// ---------------------------------------------------------------------------------------

/// A host as its rows reach it: the typed dispatch into the child's places, a child place's
/// first value (the early construction), the supplier, and the scope the rows' blueprints
/// build in. Implemented by [`Host`], which erases the owner's type. A row reads its place
/// through the place's entry, not through the host.
pub trait HostOf<E, A> {
    fn deliver(&self, place: &Path, v: &A, cx: &mut Cx) -> Done;
    fn first_value(&self, place: &Path, element: &E, entry: &Rc<Entry<E>>, cx: &mut Cx) -> Result<Option<A>, Fault>;
    fn supplier(&self) -> &Supplier<E, A>;
    fn scope(&self) -> Scope<E, A>;
    /// A request from the place `place` of this host's child, which a host under this one
    /// hands up (a forwarded element's, or a placed description's): [`Host::request`].
    fn request_from(&self, place: &Path, field: Option<&str>, value: Pod, cx: &mut Cx) -> Done;
    /// A place of this host's child came or went behind its document (a row a blueprint
    /// built there built or tore down a place of its own): the owner's `host_k_moved`.
    /// Nothing before the owner's node is made: its document's first snapshot is taken
    /// after.
    fn moved(&self, cx: &mut Cx);
}

/// The rows of a host's `for p, e <- $doc`, which the owner keeps in a field.
pub type Rows<E, A> = Rc<RefCell<Map<Path, Rc<HostRow<E, A>>>>>;

/// A host of owner `P` (the node its line and its child are inlined into): its place, its
/// rows, its supplier, the enclosing host when the line stands in a hosted scope, and the
/// owner's emitted typed dispatch.
pub struct Host<E, A, P> {
    me: Weak<Host<E, A, P>>,
    at: Path,
    owner: RefCell<Weak<P>>,
    rows: Rows<E, A>,
    supplier: Supplier<E, A>,
    enclosing: Option<Rc<dyn HostOf<E, A>>>,
    deliver: fn(&Rc<P>, &Path, &A, &mut Cx) -> Done,
    moved: fn(&Rc<P>, &mut Cx),
}

impl<E: Level + Eq, A: AnswerOf<E>, P: 'static> Host<E, A, P> {
    /// Made in the owner's build, before its node: `rows` is the field the `for` keeps,
    /// `enclosing` the host whose scope the line stands in, if any; `deliver` and `moved`
    /// the owner's emitted `host_k_answer` and `host_k_moved`.
    pub fn new(at: Path, rows: Rows<E, A>, supplier: Supplier<E, A>, enclosing: Option<Rc<dyn HostOf<E, A>>>, deliver: fn(&Rc<P>, &Path, &A, &mut Cx) -> Done, moved: fn(&Rc<P>, &mut Cx)) -> Rc<Host<E, A, P>> {
        Rc::new_cyclic(|me| Host { me: me.clone(), at, owner: RefCell::new(Weak::new()), rows, supplier, enclosing, deliver, moved })
    }

    /// The owner's node, once made: the rows read the document and answer through it.
    pub fn link(&self, owner: &Rc<P>) {
        *self.owner.borrow_mut() = Rc::downgrade(owner);
    }

    fn owner(&self) -> Rc<P> {
        self.owner.borrow().upgrade().expect("a host's node is linked at its build and outlives its rows")
    }

    fn me(&self) -> Rc<dyn HostOf<E, A>> {
        self.me.upgrade().expect("a host outlives its rows")
    }

    /// `@HostList`'s answers sent down by path (Semantics ch. 9, the list form; rulings
    /// round 12, AQ): each firing of `answers` hands the place at each of its keys that
    /// answer, typed by its tag, as a computed answer, the next tick's input, which is
    /// owed. A key with no place is dropped, as an answer to a place that left is.
    pub fn hear(&self, answers: &Event<Map<Path, A>>) {
        let me = self.me.clone();
        // the subscription holds the host weakly and lives as long as the event's port
        let _ = answers.port.subscribe(Rc::new(move |m: &Map<Path, A>, cx: &mut Cx| match me.upgrade() {
            Some(host) => host.send_down(m, cx),
            None => Ok(()),
        }));
    }

    fn send_down(&self, m: &Map<Path, A>, cx: &mut Cx) -> Done {
        for (p, v) in m.iter() {
            let row = self.rows.borrow().get(p).cloned();
            if let Some(row) = row {
                row.answer_later(v.clone(), cx);
            }
        }
        Ok(())
    }

    /// A child place's request: out with its forwarded element, or nowhere when the
    /// supplier answered the place (a host answers its child; a request reaches a World
    /// only from an element the host's blueprint emits).
    pub fn request(&self, place: &Path, field: Option<&str>, value: Pod, cx: &mut Cx) -> Done {
        let row = self.rows.borrow().get(place).cloned();
        match row {
            Some(row) => row.request(field, value, cx),
            None => Err(Fault::answer(format!("a request from {place}, which has no row under the host at {}", self.at))),
        }
    }
}

impl<E: Level + Eq, A: AnswerOf<E>, P: 'static> HostOf<E, A> for Host<E, A, P> {
    fn deliver(&self, place: &Path, v: &A, cx: &mut Cx) -> Done {
        (self.deliver)(&self.owner(), place, v, cx)
    }
    /// A child place's first value, asked at its build (Semantics ch. 9, "Queries are
    /// sync"): the supplier row for the place is built now, before the owner's node, and
    /// kept where the builder finds it. The one early construction.
    fn first_value(&self, place: &Path, element: &E, entry: &Rc<Entry<E>>, cx: &mut Cx) -> Result<Option<A>, Fault> {
        let row = HostRow::early(self.at.child(Segment::Key(place.to_pod())), place.clone(), element.clone(), entry.clone(), self.me(), cx)?;
        let first = row.first();
        self.rows.borrow_mut().insert(place.clone(), row);
        Ok(first)
    }
    fn supplier(&self) -> &Supplier<E, A> {
        &self.supplier
    }
    fn scope(&self) -> Scope<E, A> {
        match &self.enclosing {
            Some(h) => Scope::Hosted(h.clone()),
            None => Scope::World,
        }
    }
    fn request_from(&self, place: &Path, field: Option<&str>, value: Pod, cx: &mut Cx) -> Done {
        self.request(place, field, value, cx)
    }
    fn moved(&self, cx: &mut Cx) {
        let owner = self.owner.borrow().upgrade();
        if let Some(owner) = owner {
            (self.moved)(&owner, cx);
        }
    }
}

/// One row of a host's `for p, e <- $doc`: the supplier's blueprint for child place `key`,
/// built at `host/[key]`. Its entry is the place's own (pick RE), which it reads as it
/// stands; the row ends at the end of the tick its key leaves (R8), its blueprint's node
/// with it, and nothing reads its entry after.
pub struct HostRow<E, A> {
    path: Path,
    key: Path,
    place: Rc<Entry<E>>,
    /// Weak: the host keeps its rows, and a strong pointer back would be a count cycle
    /// that leaks a torn-down host (D5). The host outlives its rows.
    host: Weak<dyn HostOf<E, A>>,
    /// The key left: a computed answer in flight to the place is dropped.
    left: Cell<bool>,
    /// The subscriptions of a bundle answered held whole ([`AnswerOf::held_parts`]): they
    /// leave with the row.
    held: RefCell<Option<Rc<dyn Node>>>,
    sink: Sink<A>,
    supplied: RefCell<Option<Supplied<E, A>>>,
    /// The entry moved: readers of the live element's level (a forwarded element) read it
    /// again.
    level_moved: Rc<Port<()>>,
    /// The place's requests, which the live element's event fields hear (`Live::requested`).
    requests: Rc<Port<(Option<String>, Pod)>>,
}

impl<E: Clone + Eq + 'static, A: AnswerOf<E>> HostRow<E, A> {
    /// Built after propagation, from the entry after the tick (R7): the supplier is asked
    /// for the place's blueprint, and the blueprint is built under the row's path, in the
    /// host's scope. The `for` machinery of the emitted code calls this with the host as
    /// the row's parent; the place's entry has its first snapshot, its identity.
    pub fn build(path: Path, key: Path, value: Rc<Entry<E>>, host: Rc<dyn HostOf<E, A>>, cx: &mut Cx) -> Result<Rc<HostRow<E, A>>, Fault> {
        let identity = value.current(cx);
        HostRow::early(path, key, identity, value, host, cx)
    }

    /// Built with the place's identity given: a first-value query at the place's build,
    /// before its entry has a snapshot (the early construction).
    pub fn early(path: Path, key: Path, identity: E, place: Rc<Entry<E>>, host: Rc<dyn HostOf<E, A>>, cx: &mut Cx) -> Result<Rc<HostRow<E, A>>, Fault> {
        let node = Rc::new_cyclic(|me: &Weak<HostRow<E, A>>| {
            let row = me.clone();
            let sink: Sink<A> = Rc::new(move |v: &A, cx: &mut Cx| match row.upgrade() {
                // An answer to a row that left, or one that is gone, is dropped (Semantics
                // ch. 9, "A late result is dropped"): a computed answer in flight to a
                // place whose key left.
                Some(row) if !row.left.get() => row.host().deliver(&row.key, v, cx),
                _ => Ok(()),
            });
            HostRow { path, key, place, host: Rc::downgrade(&host), left: Cell::new(false), held: RefCell::new(None), sink, supplied: RefCell::new(None), level_moved: Rc::new(Port::new()), requests: Rc::new(Port::new()) }
        });
        let weak: Weak<dyn Node> = Rc::downgrade(&(node.clone() as Rc<dyn Node>));
        node.place.watch(weak);
        let element = node.element(identity.clone());
        let blueprint = (host.supplier())(&node.key, &element);
        let mut supplied = blueprint(node.path.clone(), node.sink.clone(), &host.scope(), 0, cx)?;
        // a bundle made outside the tag's arm and held whole is answered part by part: its
        // state the place's first value, each firing of an event part an answer
        if let Some(parts) = supplied.first.as_ref().and_then(|first| A::held_parts(&identity, first)) {
            let (first, subscribed) = answer_parts(&parts, &node.sink, cx);
            supplied.first = first;
            *node.held.borrow_mut() = Some(Rc::new(subscribed));
        }
        *node.supplied.borrow_mut() = Some(supplied);
        Ok(node)
    }

    fn host(&self) -> Rc<dyn HostOf<E, A>> {
        self.host.upgrade().expect("a host outlives its rows")
    }

    /// An answer the host sends down to the row's place (`@HostList`): delivered as the
    /// next tick's input, and dropped if the key left by then.
    pub fn answer_later(&self, v: A, cx: &mut Cx) {
        let sink = self.sink.clone();
        cx.answer_later(Box::new(move |cx| sink(&v, cx)));
    }

    /// The blueprint's first value: the place's, when its output has one.
    pub fn first(&self) -> Option<A> {
        self.supplied.borrow().as_ref().and_then(|s| s.first.clone())
    }

    /// The live element the supplier sees: the place's level at the build, and its level
    /// over time, the row's entry.
    fn element(self: &Rc<Self>, identity: E) -> Element<E> {
        let me = Rc::downgrade(self);
        let read = Rc::new(move |cx: &Cx| me.upgrade().expect("a live element outlives its host's row").entry(cx));
        Element::Live(Live { identity, level: Dynamic { owner: None, read, updates: self.level_moved.clone() }, entry: self.place.clone(), requests: self.requests.clone() })
    }

    /// The row's entry at the read point: the place's level as it stands (its snapshot of
    /// this tick from the construction step on).
    pub fn entry(&self, cx: &Cx) -> E {
        self.place.current(cx)
    }

    fn forwards(&self) -> bool {
        self.supplied.borrow().as_ref().is_some_and(|s| s.forwards)
    }

    /// The child place's request: heard by the live element's event field (a supplier's
    /// pattern that bound it), and out with the forwarded element, or nowhere else when
    /// the supplier answered the place.
    pub fn request(&self, field: Option<&str>, value: Pod, cx: &mut Cx) -> Done {
        self.requests.fire((field.map(String::from), value.clone()), cx)?;
        if self.forwards() {
            // the forwarded element is the host's own at `0` under the row: out to the
            // World, or, under a host that is itself hosted, a place of the enclosing
            // host's child, whose row decides again (a request rises host by host, each
            // adding its key)
            match self.host().scope() {
                Scope::World => cx.request(self.path.at(0), field, value),
                Scope::Hosted(outer) => outer.request_from(&self.path.at(0), field, value, cx)?,
            }
        }
        Ok(())
    }

    /// The row's own places, for an enclosing host's document: the forwarded element at
    /// `0`, when there is one, which is the child's place itself (its entry); a placed
    /// element value's; or the places of a widget's row the blueprint built.
    pub fn entries(&self, cx: &Cx) -> Vec<(Path, Rc<Entry<E>>)> {
        if self.forwards() {
            return vec![(Path::root().at(0), self.place.clone())];
        }
        let document = self.supplied.borrow().as_ref().and_then(|s| s.document.clone());
        if let Some(document) = document {
            return document(cx);
        }
        match self.supplied.borrow().as_ref().and_then(|s| s.entry.clone()) {
            Some(entry) => vec![(Path::root().at(0), entry)],
            None => Vec::new(),
        }
    }

    /// An enclosing host's typed answer to a place under the row: the forwarded element's,
    /// handed to the child's place now; a place of a widget's row the blueprint built,
    /// handed to the row.
    pub fn deliver(&self, at: &[Segment], v: &A, cx: &mut Cx) -> Done {
        let deliver = self.supplied.borrow().as_ref().and_then(|s| s.deliver.clone());
        if let Some(deliver) = deliver {
            return deliver(at, v, cx);
        }
        match at {
            [Segment::Pos(0)] if self.forwards() || self.supplied.borrow().as_ref().is_some_and(|s| s.entry.is_some()) => (self.sink)(v, cx),
            _ => Err(Fault::answer(format!("no hosted place at {} under the host row at {}", Path(at.to_vec()), self.path))),
        }
    }

    /// The key left the child's document: the blueprint's node leaves with the row at the
    /// end of this tick (R8; a forwarded element is torn down in this tick's patch), and an
    /// answer in flight is dropped. The emitted prelude's `Row` calls this.
    pub fn leave_row(&self, cx: &mut Cx) {
        self.left.set(true);
        if let Some(held) = self.held.borrow_mut().take() {
            let done = held.leave(cx);
            cx.keep(done);
        }
        if let Some(s) = self.supplied.borrow_mut().take() {
            match &s.leave {
                Some(leave) => leave(cx),
                None => {
                    let done = s.node.leave(cx);
                    cx.keep(done);
                }
            }
        }
    }
}

impl<E: Clone + Eq + 'static, A: Clone + 'static> Node for HostRow<E, A> {
    /// A World's answer under the row: to the blueprint's node (a forwarded element).
    fn answer(self: Rc<Self>, at: &[Segment], v: &Pod, cx: &mut Cx) -> Done {
        let inner = self.supplied.borrow().as_ref().map(|s| s.node.clone());
        match inner {
            Some(node) => node.answer(at, v, cx),
            None => Err(Fault::answer(format!("no place at {} under the host row at {}", Path(at.to_vec()), self.path))),
        }
    }
    /// The entry may have moved (the host marked the row): the live element's readers
    /// read it again, and a forwarded element re-emits.
    fn patch(self: Rc<Self>, cx: &mut Cx) -> Done {
        self.level_moved.fire((), cx)
    }
}
