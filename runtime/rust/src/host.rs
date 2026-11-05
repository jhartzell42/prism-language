//! Hosting (Semantics ch. 9, "`@HostDocument` and its forms"; claude/backend.md, "Hosting"):
//! what is the same in every program when a host document is lowered as a `for` over its
//! child's document.
//!
//! A host is Prism inside the same graph and the same tick. The compiler inlines the hosted
//! child into the node of the host's block (its owner), in a scope of its own, and emits
//! for the host two typed methods on the owner: the child's document as a getter
//! (`host_k_doc_direct`, the structure itself, not the lagged `$doc` the program reads:
//! every child place by its path relative to the child, each a snapshot
//! `Elem`) and a dispatch from a child place's path and a typed answer into the child's
//! input method (`host_k_answer`). The owner holds a [`Host`] made of those two, and a
//! `for` over the document whose rows are [`HostRow`]s: one per child place, built when the
//! key appears in the child's document after a tick and torn down at the end of the tick its key leaves
//! (R7, R8, the `for` machinery of the emitted code). Nothing here is plain data: the
//! child's elements are `E`, the program's `Elem` enum, and an answer is `A`, the program's
//! `Answer` enum (one variant per tag, carrying that tag's output type).
//!
//! A row calls the supplier with the place's path and its live element ([`Live`]: the
//! identity, and the level over time read through the owner's document) and builds the
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

use crate::cells::{Dynamic, Event, Handle, Live, Port, Stamped};
use crate::collections::Map;
use crate::cx::{Cx, Done, Node};
use crate::pod::{Fault, Path, Pod, Segment, ToPod};

/// The way back into a child's place: a typed answer, delivered now.
pub type Sink<A> = Rc<dyn Fn(&A, &mut Cx) -> Done>;

/// What a blueprint built for a child place gives back: its node (whose places are under
/// the row's path), the place's first value when the output has one, and whether the
/// blueprint forwards the element (then the child place's requests go out with it, at
/// the row's `0`, and an enclosing host's answers to that place come back to the child).
pub struct Supplied<A> {
    pub node: Rc<dyn Node>,
    pub first: Option<A>,
    pub forwards: bool,
}

/// `@o` as a value (claude/backend.md, "Types"): built at a path with the sink that answers
/// the child's place, in the scope of the host whose row builds it.
pub type Blueprint<E, A> = Rc<dyn Fn(Path, Sink<A>, &Scope<E, A>, &mut Cx) -> Result<Supplied<A>, Fault>>;

/// A host's supplier, in the `@HostDocument` form: a blueprint for every child place.
pub type Supplier<E, A> = Rc<dyn Fn(&Path, &Live<E>) -> Blueprint<E, A>>;

/// `@HostSome`'s supplier made `@HostDocument`'s (Semantics ch. 9, the derivation): `nil`
/// means "not mine, forward it".
pub fn forward_on_none<E: ToPod + Clone + 'static, A: AnswerOf<E>>(some: Rc<dyn Fn(&Path, &Live<E>) -> Option<Blueprint<E, A>>>) -> Supplier<E, A> {
    Rc::new(move |p: &Path, e: &Live<E>| match some(p, e) {
        Some(b) => b,
        None => element_blueprint(e),
    })
}

/// The program's answer type, which a World's answer to an element decodes into by the
/// element's tag (emitted beside `Elem`).
pub trait AnswerOf<E>: Clone + 'static {
    fn decode(element: &E, v: &Pod) -> Result<Self, Fault>;
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
    Rc::new(move |_path, _sink, _scope, _cx| Ok(Supplied { node: Rc::new(Inert), first: Some(x.clone()), forwards: false }))
}

/// `@ never`: no places, no first value, no answer.
pub fn never_blueprint<E: 'static, A: 'static>() -> Blueprint<E, A> {
    Rc::new(move |_path, _sink, _scope, _cx| Ok(Supplied { node: Rc::new(Inert), first: None, forwards: false }))
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

/// A computed answer: delivered as an input of the next tick, which is owed.
fn later<A: Clone + 'static>(sink: &Sink<A>, v: A, cx: &mut Cx) -> Done {
    let sink = sink.clone();
    cx.answer_later(Box::new(move |cx| sink(&v, cx)));
    Ok(())
}

/// `@ !e`: each firing of `e` is an answer to the child's place, a tick late.
pub fn event_blueprint<E: 'static, A: Clone + 'static>(e: Event<A>) -> Blueprint<E, A> {
    Rc::new(move |_path, sink: Sink<A>, _scope, _cx| {
        let node = Rc::new(Subscribed { port: e.port.clone(), handle: Cell::new(None) });
        let h = e.port.subscribe(Rc::new(move |v: &A, cx: &mut Cx| later(&sink, v.clone(), cx)));
        node.handle.set(Some(h));
        Ok(Supplied { node, first: None, forwards: false })
    })
}

/// `@ $d`: the value after the birth tick is the place's first value, read now,
/// synchronously, as a World's first-value query is; each later change of `d` is a push to
/// the child's place as an input of the next tick, which is owed (Semantics ch. 9 as ruled,
/// round 3 DY). The port says only that `d` moved: the value is sampled when the push is
/// delivered, at the next tick's read point, so it is the value after the tick of the
/// change whatever height the change came at, and several marks in one tick are one push.
pub fn dynamic_blueprint<E: 'static, A: Clone + 'static>(d: Dynamic<A>) -> Blueprint<E, A> {
    Rc::new(move |_path, sink: Sink<A>, _scope, cx| {
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
        Ok(Supplied { node, first: Some(first), forwards: false })
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
}

/// `@Element e`: the child's element re-emitted at `path/0`, its level following the
/// child's, every answer and push there handed to the child's place in the same tick (the
/// transparent rule). In the World's scope the place goes out with `cx.added` and its first
/// value is the World's; in a hosted scope nothing goes out (the enclosing host's document
/// reads the row), and the first value is the enclosing host's.
pub fn element_blueprint<E: ToPod + Clone + 'static, A: AnswerOf<E>>(e: &Live<E>) -> Blueprint<E, A> {
    let e = e.clone();
    Rc::new(move |path, sink: Sink<A>, scope: &Scope<E, A>, cx| {
        let at = path.at(0);
        // The level at the build is the identity: a row is built from the entry after the
        // tick, and an early row's entry cannot be read until its host's node is made.
        let element = e.identity.to_pod();
        let first = match scope {
            Scope::World => cx.first_value(&element).map(|v| A::decode(&e.identity, &v)).transpose()?,
            Scope::Hosted(host) => host.first_value(&at, &e.identity, cx)?,
        };
        let node = Rc::new(Forward { at: at.clone(), element: e.identity.clone(), level: e.level.clone(), sink, handle: Cell::new(None) });
        if let Scope::World = scope {
            let weak = Rc::downgrade(&node);
            let h = e.level.updates.subscribe(Rc::new(move |_: &(), cx: &mut Cx| {
                if let Some(n) = weak.upgrade() {
                    let now = n.level.sample(cx).to_pod();
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
        Ok(Supplied { node, first, forwards: true })
    })
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

impl<E: ToPod + Clone + 'static, A: AnswerOf<E>> Node for Forward<E, A> {
    fn answer(self: Rc<Self>, at: &[Segment], v: &Pod, cx: &mut Cx) -> Done {
        match at {
            [Segment::Pos(0)] => (self.sink)(&A::decode(&self.element, v)?, cx),
            _ => Err(Fault::answer(format!("no place at {} under a forwarded element", Path(at.to_vec())))),
        }
    }
    fn push(self: Rc<Self>, _element: &Pod, v: &Pod, cx: &mut Cx) -> Done {
        (self.sink)(&A::decode(&self.element, v)?, cx)
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

/// A host as its rows reach it: the hosted child's document at the read point, the typed
/// dispatch into the child's places, a child place's first value (the early construction),
/// the supplier, and the scope the rows' blueprints build in. Implemented by [`Host`],
/// which erases the owner's type.
pub trait HostOf<E, A> {
    fn document(&self, cx: &Cx) -> Map<Path, E>;
    fn deliver(&self, place: &Path, v: &A, cx: &mut Cx) -> Done;
    fn first_value(&self, place: &Path, element: &E, cx: &mut Cx) -> Result<Option<A>, Fault>;
    fn supplier(&self) -> &Supplier<E, A>;
    fn scope(&self) -> Scope<E, A>;
}

/// The rows of a host's `for p, e <- $doc`, which the owner keeps in a field.
pub type Rows<E, A> = Rc<RefCell<Map<Path, Rc<HostRow<E, A>>>>>;

/// A host of owner `P` (the node its line and its child are inlined into): its place, its
/// rows, its supplier, the enclosing host when the line stands in a hosted scope, and the
/// owner's two emitted methods, the child's document and the typed dispatch.
pub struct Host<E, A, P> {
    me: Weak<Host<E, A, P>>,
    at: Path,
    owner: RefCell<Weak<P>>,
    rows: Rows<E, A>,
    supplier: Supplier<E, A>,
    enclosing: Option<Rc<dyn HostOf<E, A>>>,
    document: fn(&P, &Cx) -> Map<Path, E>,
    deliver: fn(&Rc<P>, &Path, &A, &mut Cx) -> Done,
}

impl<E: ToPod + Clone + 'static, A: AnswerOf<E>, P: 'static> Host<E, A, P> {
    /// Made in the owner's build, before its node: `rows` is the field the `for` keeps,
    /// `enclosing` the host whose scope the line stands in, if any.
    pub fn new(at: Path, rows: Rows<E, A>, supplier: Supplier<E, A>, enclosing: Option<Rc<dyn HostOf<E, A>>>, document: fn(&P, &Cx) -> Map<Path, E>, deliver: fn(&Rc<P>, &Path, &A, &mut Cx) -> Done) -> Rc<Host<E, A, P>> {
        Rc::new_cyclic(|me| Host { me: me.clone(), at, owner: RefCell::new(Weak::new()), rows, supplier, enclosing, document, deliver })
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

    /// A child place's request: out with its forwarded element, or nowhere when the
    /// supplier answered the place (a host answers its child; a request reaches a World
    /// only from an element the host's blueprint emits).
    pub fn request(&self, place: &Path, field: Option<&str>, value: Pod, cx: &mut Cx) -> Done {
        let row = self.rows.borrow().get(place).cloned();
        match row {
            Some(row) => Ok(row.request(field, value, cx)),
            None => Err(Fault::answer(format!("a request from {place}, which has no row under the host at {}", self.at))),
        }
    }
}

impl<E: ToPod + Clone + 'static, A: AnswerOf<E>, P: 'static> HostOf<E, A> for Host<E, A, P> {
    fn document(&self, cx: &Cx) -> Map<Path, E> {
        (self.document)(&self.owner(), cx)
    }
    fn deliver(&self, place: &Path, v: &A, cx: &mut Cx) -> Done {
        (self.deliver)(&self.owner(), place, v, cx)
    }
    /// A child place's first value, asked at its build (Semantics ch. 9, "Queries are
    /// sync"): the supplier row for the place is built now, before the owner's node, and
    /// kept where the builder finds it. The one early construction.
    fn first_value(&self, place: &Path, element: &E, cx: &mut Cx) -> Result<Option<A>, Fault> {
        let row = HostRow::build(self.at.child(Segment::Key(place.to_pod())), place.clone(), element.clone(), self.me(), cx)?;
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
}

/// One row of a host's `for p, e <- $doc`: the supplier's blueprint for child place `key`,
/// built at `host/[key]`. Its entry is the projection `$doc[key]` (pick RE), read through
/// the host while the key is in the document; the row ends at the end of the tick its key
/// leaves (R8), its blueprint's node with it, and nothing reads its entry after.
pub struct HostRow<E, A> {
    path: Path,
    key: Path,
    /// Weak: the host keeps its rows, and a strong pointer back would be a count cycle
    /// that leaks a torn-down host (D5). The host outlives its rows.
    host: Weak<dyn HostOf<E, A>>,
    /// The key left: a computed answer in flight to the place is dropped.
    left: Cell<bool>,
    sink: Sink<A>,
    supplied: RefCell<Option<Supplied<A>>>,
    /// The entry moved: readers of the live element's level (a forwarded element) read it
    /// again.
    level_moved: Rc<Port<()>>,
}

impl<E: ToPod + Clone + 'static, A: Clone + 'static> HostRow<E, A> {
    /// Built after propagation, from the entry after the tick (R7): the supplier is asked
    /// for the place's blueprint, and the blueprint is built under the row's path, in the
    /// host's scope. The `for` machinery of the emitted code calls this with the host as
    /// the row's parent.
    pub fn build(path: Path, key: Path, value: E, host: Rc<dyn HostOf<E, A>>, cx: &mut Cx) -> Result<Rc<HostRow<E, A>>, Fault> {
        let node = Rc::new_cyclic(|me: &Weak<HostRow<E, A>>| {
            let row = me.clone();
            let sink: Sink<A> = Rc::new(move |v: &A, cx: &mut Cx| match row.upgrade() {
                // An answer to a row that left, or one that is gone, is dropped (Semantics
                // ch. 9, "A late result is dropped"): a computed answer in flight to a
                // place whose key left.
                Some(row) if !row.left.get() => row.host().deliver(&row.key, v, cx),
                _ => Ok(()),
            });
            HostRow { path, key, host: Rc::downgrade(&host), left: Cell::new(false), sink, supplied: RefCell::new(None), level_moved: Rc::new(Port::new()) }
        });
        let element = node.element(value);
        let blueprint = (host.supplier())(&node.key, &element);
        let supplied = blueprint(node.path.clone(), node.sink.clone(), &host.scope(), cx)?;
        *node.supplied.borrow_mut() = Some(supplied);
        Ok(node)
    }

    fn host(&self) -> Rc<dyn HostOf<E, A>> {
        self.host.upgrade().expect("a host outlives its rows")
    }

    /// The blueprint's first value: the place's, when its output has one.
    pub fn first(&self) -> Option<A> {
        self.supplied.borrow().as_ref().and_then(|s| s.first.clone())
    }

    /// The live element the supplier sees: the place's level at the build, and its level
    /// over time, the row's entry.
    fn element(self: &Rc<Self>, identity: E) -> Live<E> {
        let me = Rc::downgrade(self);
        let read = Rc::new(move |cx: &Cx| me.upgrade().expect("a live element outlives its host's row").entry(cx));
        Live { identity, level: Dynamic { owner: None, read, updates: self.level_moved.clone() } }
    }

    /// The row's entry, `$doc[key]`, at the read point. The document is the child's
    /// places as they stand, so a key that left is gone from it at once: the leaving
    /// unsubscribes every reader of the entry first (`leave_row`), and a read after that
    /// is a bug of the compiler, not a value.
    pub fn entry(&self, cx: &Cx) -> E {
        self.host().document(cx).get(&self.key).cloned().expect("a host row's entry is read only while its key is in the child's document")
    }

    fn forwards(&self) -> bool {
        self.supplied.borrow().as_ref().is_some_and(|s| s.forwards)
    }

    /// The child place's request: out with the forwarded element, or nowhere when the
    /// supplier answered the place.
    pub fn request(&self, field: Option<&str>, value: Pod, cx: &mut Cx) {
        if self.forwards() {
            cx.request(self.path.at(0), field, value);
        }
    }

    /// The row's own places, for an enclosing host's document: the forwarded element at
    /// `0`, when there is one.
    pub fn document(&self, cx: &Cx) -> Vec<(Path, E)> {
        if self.forwards() { vec![(Path::root().at(0), self.entry(cx))] } else { Vec::new() }
    }

    /// An enclosing host's typed answer to a place under the row: the forwarded element's,
    /// handed to the child's place now.
    pub fn deliver(&self, at: &[Segment], v: &A, cx: &mut Cx) -> Done {
        match at {
            [Segment::Pos(0)] if self.forwards() => (self.sink)(v, cx),
            _ => Err(Fault::answer(format!("no hosted place at {} under the host row at {}", Path(at.to_vec()), self.path))),
        }
    }

    /// The key left the child's document: the blueprint's node leaves with the row at the
    /// end of this tick (R8; a forwarded element is torn down in this tick's patch), and an
    /// answer in flight is dropped. The emitted prelude's `Row` calls this.
    pub fn leave_row(&self, cx: &mut Cx) {
        self.left.set(true);
        if let Some(s) = self.supplied.borrow_mut().take() {
            let done = s.node.leave(cx);
            cx.keep(done);
        }
    }
}

impl<E: ToPod + Clone + 'static, A: Clone + 'static> Node for HostRow<E, A> {
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
