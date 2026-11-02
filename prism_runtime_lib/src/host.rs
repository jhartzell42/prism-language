//! Hosting (Semantics ch. 9, "`@HostDocument` and its forms"; docs/backend.md,
//! "Hosting"): a host node holds its child and, per place of the child, the blueprint
//! its supplier answers that place with, built at `host/[p]` in the host's own scope.
//!
//! The boundary between a host and its child is plain data, as a World's is: the child's
//! places reach the host as `Pod` levels, an answer goes down by path through the
//! child's `Node::answer`, and a first value comes from the dynamic the place's blueprint
//! answers with. A supplier's output type is its own per place (rank 2), so a blueprint
//! answers with an [`Answering`], its output as plain data.
//!
//! **The scope rides on the path.** Every widget constructor takes its path, and a
//! hosted child's paths start with [`Segment::Host`]: `Cx::added`, `changed`, `removed`,
//! `request`, `first_value_at` and `listen_at` send what is under a host to the host, the
//! rest to the document. Rows and generations inside the child build at paths under the
//! child's, so they inherit the scope with no argument of their own.
//!
//! - **The transparent rule.** A forwarding blueprint ([`element_blueprint`],
//!   `@Element e`) re-emits the child's element at its own place `host/[p]/0` and hands
//!   every answer there back to the child's place, so a host that forwards everything is
//!   observably not there, under deeper paths outside.
//! - **The child is inlined into the host's owner**, the node of the host's block: its
//!   places emit under the host's child path, and the host answers one by calling the
//!   owner's `Node::answer` under that path.
//! - **The child's document** is a dynamic of each child place's `Level`, read before
//!   the tick during propagation and after it from then on, as a hold is
//!   ([`HostDoc`]); the owner is patched and its builders run when it moves.
//! - **The sandbox law** is the checker's (the remainder row at the forwarding arm,
//!   Semantics ch. 9); at run time nothing the child emits reaches the document except
//!   through a blueprint the supplier built.
//!
//! Written by Claude, 2 November 2026, on the claude-self-hosting branch. Subject to
//! change.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::{Rc, Weak};

use crate::cells::{Dynamic, Event, Handle, Live, Port, Tick};
use crate::collections::Map;
use crate::cx::{Cx, Done, Node};
use crate::pod::{Fault, FromPod, Path, Pod, Segment, ToPod};

/// A blueprint's output as plain data: how it answers the child's place.
#[derive(Clone)]
pub enum Answering {
    /// A plain or constant output: the place's first value, never updated.
    Constant(Pod),
    /// An event output: each firing is an answer to the place.
    Event(Event<Pod>),
    /// A dynamic output: its value after the tick is the first value, each update a new
    /// answer.
    Dynamic(Dynamic<Pod>),
}

impl Answering {
    /// `@ x` for a plain or constant `x`.
    pub fn constant<T: ToPod>(v: &T) -> Answering {
        Answering::Constant(v.to_pod())
    }
    /// `@ !e`: each firing as plain data.
    pub fn event<T: ToPod + Clone + 'static>(e: &Event<T>) -> Answering {
        let port = Rc::new(Port::new());
        let out = port.clone();
        e.port.subscribe(Rc::new(move |v: &T, cx: &mut Cx| out.fire(v.to_pod(), cx)));
        Answering::Event(Event { owner: e.owner.clone(), port })
    }
}

/// `@a` as a value (docs/backend.md, "Types"): built at a path, a node and its output.
pub type Blueprint = Rc<dyn Fn(Path, &mut Cx) -> Result<(Rc<dyn Node>, Answering), Fault>>;

/// A node with no places and nothing to do: what `@ x` builds.
struct Inert;
impl Node for Inert {}

/// `@ x`: no places, the output `x`.
pub fn pure_blueprint(out: Answering) -> Blueprint {
    Rc::new(move |_path, _cx| Ok((Rc::new(Inert) as Rc<dyn Node>, out.clone())))
}

/// `@Element e`: the child's element re-emitted at `path/0`, its level following the
/// child's, every answer there handed back as the blueprint's output (the transparent
/// rule). A place the World gives a first value answers as a dynamic, pushes included;
/// any other as an event.
pub fn element_blueprint<E: ToPod + Clone + 'static>(e: &Live<E>) -> Blueprint {
    let identity = e.identity.to_pod();
    let level = e.level.map(|v: &E| v.to_pod());
    Rc::new(move |path, cx| {
        let at = path.at(0);
        let first = cx.first_value_at(&at, &identity);
        let node = Rc::new(Forward {
            at: at.clone(),
            level: level.clone(),
            port: Rc::new(Port::new()),
            value: RefCell::new(first.clone()),
            updates: Rc::new(Port::new()),
            handle: Cell::new(None),
        });
        let weak = Rc::downgrade(&node);
        let h = level.updates.subscribe(Rc::new(move |_: &(), cx: &mut Cx| {
            if let Some(n) = weak.upgrade() {
                let now = n.level.sample(cx);
                cx.changed(n.at.clone(), now);
            }
            Ok(())
        }));
        node.handle.set(Some(h));
        cx.added(at.clone(), level.sample(cx));
        let out = match first {
            Some(_) => {
                cx.listen_at(&at, identity.clone(), &(node.clone() as Rc<dyn Node>));
                let me = node.clone();
                Answering::Dynamic(Dynamic {
                    owner: Some(node.clone()),
                    read: Rc::new(move |_| me.value.borrow().clone().unwrap_or(Pod::Unit)),
                    updates: node.updates.clone(),
                })
            }
            None => Answering::Event(Event { owner: Some(node.clone()), port: node.port.clone() }),
        };
        Ok((node as Rc<dyn Node>, out))
    })
}

/// A forwarding blueprint's node: one place, the child's element re-emitted.
struct Forward {
    at: Path,
    level: Dynamic<Pod>,
    port: Rc<Port<Pod>>,
    value: RefCell<Option<Pod>>,
    updates: Rc<Port<()>>,
    handle: Cell<Option<Handle>>,
}

impl Forward {
    fn answered(&self, v: &Pod, cx: &mut Cx) -> Done {
        let dynamic = self.value.borrow().is_some();
        if dynamic {
            *self.value.borrow_mut() = Some(v.clone());
            self.updates.fire((), cx)
        } else {
            self.port.fire(v.clone(), cx)
        }
    }
}

impl Node for Forward {
    fn answer(self: Rc<Self>, at: &[Segment], v: &Pod, cx: &mut Cx) -> Done {
        match at {
            [Segment::Pos(0)] => self.answered(v, cx),
            _ => Err(Fault::answer(format!("no place at {} under a forwarded element", Path(at.to_vec())))),
        }
    }
    fn push(self: Rc<Self>, _element: &Pod, v: &Pod, cx: &mut Cx) -> Done {
        self.answered(v, cx)
    }
    fn leave(self: Rc<Self>, cx: &mut Cx) -> Done {
        if let Some(h) = self.handle.take() {
            self.level.updates.unsubscribe(h);
        }
        cx.removed(self.at.clone());
        Ok(())
    }
}

/// The supplier at the plain-data boundary: a child place's path and live element, the
/// blueprint answering it, or `None` to forward it (`@HostSome`'s `nil`).
type PodSupplier = Rc<dyn Fn(&Path, &Live<Pod>, &mut Cx) -> Result<Option<Blueprint>, Fault>>;

/// One child place as the host holds it: its level over time, the blueprint's node and
/// output, and the subscription that answers the child.
struct Row {
    level: Rc<RefCell<Pod>>,
    moved: Rc<Port<()>>,
    node: Rc<dyn Node>,
    out: Answering,
    handle: Option<Handle>,
}

/// A host's child's document: each place's `Level`, the newest and the one before the
/// tick that last moved it.
struct Doc {
    now: BTreeMap<Path, Pod>,
    before: BTreeMap<Path, Pod>,
    stamp: Tick,
    written: bool,
}

/// A host node (`@HostDocument`, `@HostSome`): the child, its document, and a supplier
/// row per child place at `path/[p]`.
pub struct Host {
    id: u64,
    me: Weak<Host>,
    path: Path,
    supplier: PodSupplier,
    rows: RefCell<BTreeMap<Path, Row>>,
    doc: RefCell<Doc>,
    updates: Rc<Port<()>>,
    owner: RefCell<Option<Weak<dyn Node>>>,
}

thread_local! {
    static NEXT_HOST: Cell<u64> = const { Cell::new(0) };
}

/// The host a hosted child's path names, held weakly (the child is the host's).
#[derive(Clone)]
pub struct HostRef(u64, Weak<Host>);

impl HostRef {
    pub fn id(&self) -> u64 {
        self.0
    }
}
impl PartialEq for HostRef {
    fn eq(&self, o: &HostRef) -> bool {
        self.0 == o.0
    }
}
impl Eq for HostRef {}
impl PartialOrd for HostRef {
    fn partial_cmp(&self, o: &HostRef) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for HostRef {
    fn cmp(&self, o: &HostRef) -> std::cmp::Ordering {
        self.0.cmp(&o.0)
    }
}
impl Hash for HostRef {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.0.hash(h)
    }
}
impl fmt::Debug for HostRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Host({})", self.0)
    }
}

/// A path under a hosted child: its host, and the path relative to the child.
pub fn split(path: &Path) -> Option<(Rc<Host>, Path)> {
    match path.segments().first() {
        Some(Segment::Host(h)) => h.1.upgrade().map(|host| (host, Path(path.segments()[1..].to_vec()))),
        _ => None,
    }
}

impl Host {
    fn new(path: Path, supplier: PodSupplier) -> Rc<Host> {
        let id = NEXT_HOST.with(|n| {
            let id = n.get();
            n.set(id + 1);
            id
        });
        Rc::new_cyclic(|me| Host {
            id,
            me: me.clone(),
            path,
            supplier,
            rows: RefCell::new(BTreeMap::new()),
            doc: RefCell::new(Doc { now: BTreeMap::new(), before: BTreeMap::new(), stamp: 0, written: false }),
            updates: Rc::new(Port::new()),
            owner: RefCell::new(None),
        })
    }

    /// `@HostDocument @child, supplier`: a blueprint for every place.
    pub fn each<E: FromPod + Clone + 'static>(path: Path, supplier: Rc<dyn Fn(&Path, &Live<E>) -> Blueprint>) -> Rc<Host> {
        Host::some(path, Rc::new(move |p: &Path, e: &Live<E>| Some(supplier(p, e))))
    }

    /// `@HostSome @child, supplier`: `None` forwards the place.
    pub fn some<E: FromPod + Clone + 'static>(path: Path, supplier: Rc<dyn Fn(&Path, &Live<E>) -> Option<Blueprint>>) -> Rc<Host> {
        Host::new(
            path,
            Rc::new(move |p: &Path, e: &Live<Pod>, _cx: &mut Cx| {
                let identity = E::from_pod(&e.identity)?;
                let fallback = identity.clone();
                let level = e.level.map(move |v: &Pod| E::from_pod(v).unwrap_or_else(|_| fallback.clone()));
                let live = Live { identity, level };
                Ok(supplier(p, &live))
            }),
        )
    }

    /// The root path of the child: every path under it is this host's.
    pub fn child_path(&self) -> Path {
        Path(vec![Segment::Host(HostRef(self.id, self.me.clone()))])
    }
    /// This host's id, which its child's paths name.
    pub fn id(&self) -> u64 {
        self.id
    }
    /// The node the host and its child are inlined into: it reads the child's document
    /// (patched and rebuilt when it moves), and an answer to a child place goes to it
    /// under the child's path.
    pub fn set_owner(&self, owner: &Rc<dyn Node>) {
        *self.owner.borrow_mut() = Some(Rc::downgrade(owner));
    }

    /// The child's document at the context's read point: before the tick during
    /// propagation, after it from the end of propagation on.
    pub fn document(&self, cx: &Cx) -> BTreeMap<Path, Pod> {
        let d = self.doc.borrow();
        if d.written && d.stamp >= cx.read_point() { d.before.clone() } else { d.now.clone() }
    }

    fn write_doc(&self, cx: &mut Cx, f: impl FnOnce(&mut BTreeMap<Path, Pod>)) {
        {
            let mut d = self.doc.borrow_mut();
            if !d.written || d.stamp != cx.tick() {
                d.before = d.now.clone();
                d.stamp = cx.tick();
                d.written = true;
            }
            f(&mut d.now);
        }
        let owner = self.owner.borrow().as_ref().and_then(Weak::upgrade);
        if let Some(o) = owner {
            cx.dirty(o.clone());
            cx.build_later(o);
        }
        let done = self.updates.fire((), cx);
        cx.keep(done);
    }

    /// The row for child place `at`, built from the supplier's blueprint when it has none
    /// yet: when the place asks its first value, or when it is added.
    fn ensure_row(&self, at: &Path, level: &Pod, cx: &mut Cx) -> Done {
        if self.rows.borrow().contains_key(at) {
            return Ok(());
        }
        let cell = Rc::new(RefCell::new(level.clone()));
        let moved = Rc::new(Port::new());
        let read = cell.clone();
        let live = Live {
            identity: level.clone(),
            level: Dynamic { owner: None, read: Rc::new(move |_| read.borrow().clone()), updates: moved.clone() },
        };
        let bp = match (self.supplier)(at, &live, cx)? {
            Some(bp) => bp,
            None => element_blueprint(&live),
        };
        let (node, out) = bp(self.path.child(Segment::Key(Pod::Path(at.clone()))), cx)?;
        let handle = self.answer_from(at, &out);
        self.rows.borrow_mut().insert(at.clone(), Row { level: cell, moved, node, out, handle });
        Ok(())
    }

    /// The subscription that answers the child's place from the blueprint's output.
    fn answer_from(&self, at: &Path, out: &Answering) -> Option<Handle> {
        let me = self.me.clone();
        let at = at.clone();
        match out {
            Answering::Constant(_) => None,
            Answering::Event(e) => Some(e.port.subscribe(Rc::new(move |v: &Pod, cx: &mut Cx| match me.upgrade() {
                Some(h) => h.answer_child(&at, v, cx),
                None => Ok(()),
            }))),
            Answering::Dynamic(d) => {
                let read = d.clone();
                Some(d.updates.subscribe(Rc::new(move |_: &(), cx: &mut Cx| match me.upgrade() {
                    Some(h) => {
                        let v = read.sample(cx);
                        h.answer_child(&at, &v, cx)
                    }
                    None => Ok(()),
                })))
            }
        }
    }

    fn answer_child(&self, at: &Path, v: &Pod, cx: &mut Cx) -> Done {
        let owner = self.owner.borrow().as_ref().and_then(Weak::upgrade);
        match owner {
            Some(o) => {
                let mut segs = vec![Segment::Host(HostRef(self.id, self.me.clone()))];
                segs.extend(at.segments().iter().cloned());
                o.answer(&segs, v, cx)
            }
            None => Ok(()),
        }
    }

    /// A child place's first value: its blueprint's output after the tick, unless that
    /// is an event.
    pub fn first_value(&self, at: Path, level: &Pod, cx: &mut Cx) -> Result<Option<Pod>, Fault> {
        self.ensure_row(&at, level, cx)?;
        let out = self.rows.borrow().get(&at).map(|r| r.out.clone());
        Ok(match out {
            Some(Answering::Constant(v)) => Some(v),
            Some(Answering::Dynamic(d)) => Some(d.sample(cx)),
            _ => None,
        })
    }

    /// A child place built: into the document, and its blueprint built.
    pub fn added(&self, at: Path, level: Pod, cx: &mut Cx) -> Done {
        self.write_doc(cx, |d| {
            d.insert(at.clone(), level.clone());
        });
        self.ensure_row(&at, &level, cx)
    }

    /// A child place's level moved: the document, and the live element its blueprint
    /// reads.
    pub fn changed(&self, at: &Path, level: Pod, cx: &mut Cx) {
        if self.doc.borrow().now.get(at) == Some(&level) {
            return;
        }
        self.write_doc(cx, |d| {
            d.insert(at.clone(), level.clone());
        });
        let row = self.rows.borrow().get(at).map(|r| (r.level.clone(), r.moved.clone()));
        if let Some((cell, moved)) = row {
            *cell.borrow_mut() = level;
            let done = moved.fire((), cx);
            cx.keep(done);
        }
    }

    /// A child place torn down: its key leaves the document and its blueprint leaves
    /// with it, at the end of the same tick (teardown is seen at the host).
    pub fn removed(&self, at: &Path, cx: &mut Cx) {
        self.write_doc(cx, |d| {
            d.remove(at);
        });
        let row = self.rows.borrow_mut().remove(at);
        if let Some(r) = row {
            if let (Some(h), Answering::Event(e)) = (r.handle, &r.out) {
                e.port.unsubscribe(h);
            }
            if let (Some(h), Answering::Dynamic(d)) = (r.handle, &r.out) {
                d.updates.unsubscribe(h);
            }
            let done = r.node.leave(cx);
            cx.keep(done);
        }
    }

    /// A hosted child's request reaches no World: a host answers its child, and a
    /// request goes out only from an element the host's blueprint emits.
    pub fn request(&self, _at: &Path, _field: Option<&str>, _value: Pod, _cx: &mut Cx) {}
}

impl Node for Host {
    /// An answer at `path/[p]/..`: to the blueprint built for child place `p`.
    fn answer(self: Rc<Self>, at: &[Segment], v: &Pod, cx: &mut Cx) -> Done {
        match at {
            [Segment::Key(Pod::Path(p)), rest @ ..] => {
                let node = self.rows.borrow().get(p).map(|r| r.node.clone());
                match node {
                    Some(n) => n.answer(rest, v, cx),
                    None => Err(Fault::answer(format!("no hosted place {p} under {}", self.path))),
                }
            }
            _ => Err(Fault::answer(format!("no place at {} under the host at {}", Path(at.to_vec()), self.path))),
        }
    }
}

/// A host's child document as its owner's block reads it, `$(Path ==> Elem c)`.
pub struct HostDoc<E> {
    host: Rc<Host>,
    _e: std::marker::PhantomData<E>,
}

impl<E: FromPod + Clone> HostDoc<E> {
    pub fn new(host: &Rc<Host>) -> HostDoc<E> {
        HostDoc { host: host.clone(), _e: std::marker::PhantomData }
    }
    /// The document at the read point, each level as the program's element.
    pub fn get(&self, cx: &Cx) -> Map<Path, E> {
        self.host.document(cx).iter().filter_map(|(p, l)| E::from_pod(l).ok().map(|e| (p.clone(), e))).collect()
    }
}

/// A constructor's result, fallible or not, as a `Result`: the host line builds its
/// child without knowing which.
pub trait Built<T> {
    fn built(self) -> Result<T, Fault>;
}
impl<T> Built<Rc<T>> for Rc<T> {
    fn built(self) -> Result<Rc<T>, Fault> {
        Ok(self)
    }
}
impl<T> Built<Rc<T>> for Result<Rc<T>, Fault> {
    fn built(self) -> Result<Rc<T>, Fault> {
        self
    }
}
pub fn built<T, B: Built<T>>(b: B) -> Result<T, Fault> {
    b.built()
}
