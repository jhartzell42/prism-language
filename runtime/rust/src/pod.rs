//! Plain data at the World's edge: [`Pod`], paths, and the `^Pod` traits.
//!
//! Inside a program every value is typed Rust. A World sees none of them: elements,
//! answers, pushes and requests cross as plain data (Semantics ch. 9, "External Worlds
//! see plain data"), and a [`Pod`] is that data. A program turns its values into `Pod`s
//! through [`ToPod`] and back through [`FromPod`], derived per type by the templates; the
//! library implements both for its own types. A `Pod` prints in the transcript's spelling
//! ([`crate::print`]), reads back ([`crate::read`]), and is totally ordered, so pushes and
//! the ready state key by element value.

use std::fmt;
use std::rc::Rc;

use crate::collections::{Array, Map, Text};
use crate::num::{Int, Nat};

/// A plain value as a World sees it.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Pod {
    Unit,
    Bool(bool),
    /// `Nat`, `Int` and the sized integers alike: a test's literal `1` matches any.
    Int(Int),
    Float(F64),
    Char(char),
    Text(Text),
    /// Nanoseconds.
    Duration(i128),
    /// `Name(f=v, ..)`, fields in declaration order.
    Record(Rc<str>, Vec<(Rc<str>, Pod)>),
    /// `Ctor a, b`; a unit type's value (`click`) and a fault kind (`Fs.NotFound`) are
    /// choices with no arguments.
    Choice(Rc<str>, Vec<Pod>),
    /// `Fam.Tag(f=v, ..)`: an element's `Level`, or an `Edge`'s value.
    Element(Rc<str>, Vec<(Rc<str>, Pod)>),
    Tuple(Vec<Pod>),
    Array(Vec<Pod>),
    /// Entries in ascending key order.
    Map(Vec<(Pod, Pod)>),
    Nil,
    Some(Box<Pod>),
    /// `~ x`.
    Ok(Box<Pod>),
    /// A failed `~T`: its `Error` record.
    Failed(Box<Pod>),
    Path(Path),
    /// A value whose type has its own `impl ^Show`: it prints as the impl's text and
    /// matches as its structure (transcripts print by `^Show`, Type System ch. 2).
    Shown(ShownText, Box<Pod>),
}

/// A `Pod::Shown`'s text: what it prints as. It takes no part in equality, order or
/// hashing, which go by the structure (a chess `Spot` orders by its rank and file, not
/// by its name `e2`).
#[derive(Clone, Debug)]
pub struct ShownText(pub Rc<str>);

impl PartialEq for ShownText {
    fn eq(&self, _: &ShownText) -> bool {
        true
    }
}
impl Eq for ShownText {}
impl PartialOrd for ShownText {
    fn partial_cmp(&self, o: &ShownText) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for ShownText {
    fn cmp(&self, _: &ShownText) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
    }
}
impl std::hash::Hash for ShownText {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}
impl From<&str> for ShownText {
    fn from(s: &str) -> ShownText {
        ShownText(s.into())
    }
}

/// An `f64` ordered by IEEE's total order, so `nan` can key a map.
#[derive(Clone, Copy, Debug)]
pub struct F64(pub f64);

impl PartialEq for F64 {
    fn eq(&self, other: &F64) -> bool {
        self.0.total_cmp(&other.0).is_eq()
    }
}
impl Eq for F64 {}
impl PartialOrd for F64 {
    fn partial_cmp(&self, other: &F64) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for F64 {
    fn cmp(&self, other: &F64) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}
impl std::hash::Hash for F64 {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.0.to_bits().hash(h)
    }
}

fn named(fields: Vec<(&str, Pod)>) -> Vec<(Rc<str>, Pod)> {
    fields.into_iter().map(|(n, v)| (n.into(), v)).collect()
}

impl Pod {
    pub fn text(s: impl Into<Text>) -> Pod {
        Pod::Text(s.into())
    }
    pub fn int(n: i64) -> Pod {
        Pod::Int(Int::from(n))
    }
    /// A choice with no arguments: `click`, `Read`, `Fs.NotFound`.
    pub fn name(ctor: &str) -> Pod {
        Pod::Choice(ctor.into(), Vec::new())
    }
    pub fn record(name: &str, fields: Vec<(&str, Pod)>) -> Pod {
        Pod::Record(name.into(), named(fields))
    }
    pub fn element(tag: &str, fields: Vec<(&str, Pod)>) -> Pod {
        Pod::Element(tag.into(), named(fields))
    }
    /// The value a `Shown` stands for; any other value is itself.
    pub fn structure(&self) -> &Pod {
        if let Pod::Shown(_, p) = self { p.structure() } else { self }
    }
    /// A record's or element's field.
    pub fn field(&self, name: &str) -> Option<&Pod> {
        let (Pod::Record(_, fs) | Pod::Element(_, fs)) = self.structure() else { return None };
        fs.iter().find(|(n, _)| &**n == name).map(|(_, v)| v)
    }
    /// An element's qualified tag.
    pub fn tag(&self) -> Option<&str> {
        if let Pod::Element(t, _) = self.structure() { Some(t) } else { None }
    }
    pub fn as_text(&self) -> Option<&Text> {
        if let Pod::Text(t) = self.structure() { Some(t) } else { None }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self.structure() {
            Pod::Float(f) => Some(f.0),
            Pod::Int(n) => n.to_i64().map(|n| n as f64),
            _ => None,
        }
    }
    /// Every element inside this value, itself included: a check matches at any depth, by
    /// the GUI library's convention that a container holds its children in a field.
    pub fn elements<'a>(&'a self, out: &mut Vec<&'a Pod>) {
        match self.structure() {
            Pod::Element(_, fs) => {
                out.push(self);
                fs.iter().for_each(|(_, v)| v.elements(out))
            }
            Pod::Record(_, fs) => fs.iter().for_each(|(_, v)| v.elements(out)),
            Pod::Choice(_, xs) | Pod::Tuple(xs) | Pod::Array(xs) => xs.iter().for_each(|v| v.elements(out)),
            Pod::Map(es) => es.iter().for_each(|(_, v)| v.elements(out)),
            Pod::Some(x) | Pod::Ok(x) => x.elements(out),
            _ => {}
        }
    }
}

/// One step of a path: an element statement's position, a `rebuild`'s generation `#g`,
/// or a `for`'s or host document's key `[k]` (Semantics ch. 8).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Segment {
    Pos(u32),
    Gen(u32),
    Key(Pod),
}

/// A place's path. The derived order is document order: a path before every path it
/// prefixes, and the book's order within one depth.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct Path(pub Vec<Segment>);

impl Path {
    pub fn root() -> Path {
        Path(Vec::new())
    }
    pub fn child(&self, seg: Segment) -> Path {
        Path(self.0.iter().cloned().chain([seg]).collect())
    }
    /// The child at an element statement's position.
    pub fn at(&self, pos: u32) -> Path {
        self.child(Segment::Pos(pos))
    }
    /// The path `rest` under this one: a hosted child's place, relative to the child,
    /// under the child's root.
    pub fn join(&self, rest: &Path) -> Path {
        Path(self.0.iter().chain(rest.0.iter()).cloned().collect())
    }
    pub fn segments(&self) -> &[Segment] {
        &self.0
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&crate::print::path(self))
    }
}

impl fmt::Display for Pod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&crate::print::show(self))
    }
}

/// A failure that escapes a tick, which then never happened (Semantics ch. 9, ch. 10).
/// A Prism `~` value is data instead: a `Result<T, Error>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fault {
    pub kind: String,
    pub message: String,
}

impl Fault {
    pub fn new(kind: &str, message: impl Into<String>) -> Fault {
        Fault { kind: kind.into(), message: message.into() }
    }
    /// A World answered no place, or with data of the wrong type.
    pub fn answer(message: impl Into<String>) -> Fault {
        Fault::new("Runtime.Answer", message)
    }
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl std::error::Error for Fault {}

/// Prism's `Error`, a `~T`'s failure: a fault kind (`Fs.NotFound`) and a message.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Error {
    pub kind: Rc<str>,
    pub message: Text,
}

impl Error {
    pub fn new(kind: &str, message: impl Into<Text>) -> Error {
        Error { kind: kind.into(), message: message.into() }
    }
}

// ---- ^Pod -------------------------------------------------------------------------------

/// A value as plain data. Derived per type by the templates.
pub trait ToPod {
    fn to_pod(&self) -> Pod;
}

/// A value from plain data: an answer, a push, a first value, a root argument. Only a
/// World can hand a program data of the wrong type, so a mismatch is a [`Fault::answer`].
pub trait FromPod: Sized {
    fn from_pod(p: &Pod) -> Result<Self, Fault>;
}

/// What every derived `from_pod` gives on a mismatch.
pub fn mismatch<T>(want: &str, got: &Pod) -> Result<T, Fault> {
    Err(Fault::answer(format!("expected {want}, got {got}")))
}

/// `ToPod` and `FromPod` for a type that is one `Pod` variant, and for its whole value.
macro_rules! pod_leaf {
    ($t:ty, $want:literal, |$s:ident| $to:expr, $pat:pat $(if $guard:expr)? => $from:expr) => {
        impl ToPod for $t {
            fn to_pod(&self) -> Pod {
                let $s = self;
                $to
            }
        }
        impl FromPod for $t {
            fn from_pod(p: &Pod) -> Result<$t, Fault> {
                match p.structure() {
                    $pat $(if $guard)? => Ok($from),
                    other => mismatch($want, other),
                }
            }
        }
    };
}

pod_leaf!((), "unit", |_s| Pod::Unit, Pod::Unit => ());
pod_leaf!(bool, "a Bool", |s| Pod::Bool(*s), Pod::Bool(b) => *b);
pod_leaf!(Int, "an Int", |s| Pod::Int(s.clone()), Pod::Int(n) => n.clone());
pod_leaf!(Nat, "a Nat", |s| Pod::Int(s.as_int().clone()), Pod::Int(n) if !n.is_negative() => Nat::from_int(n.clone()).unwrap());
pod_leaf!(u8, "a U8", |s| Pod::int(i64::from(*s)), Pod::Int(n) if n.to_i64().is_some_and(|n| (0..256).contains(&n)) => n.to_i64().unwrap() as u8);
pod_leaf!(f64, "a Float", |s| Pod::Float(F64(*s)), Pod::Float(f) => f.0);
pod_leaf!(char, "a Char", |s| Pod::Char(*s), Pod::Char(c) => *c);
pod_leaf!(Text, "a Text", |s| Pod::Text(s.clone()), Pod::Text(t) => t.clone());
pod_leaf!(Path, "a path", |s| Pod::Path(s.clone()), Pod::Path(p) => p.clone());

impl ToPod for Pod {
    fn to_pod(&self) -> Pod {
        self.clone()
    }
}
impl FromPod for Pod {
    fn from_pod(p: &Pod) -> Result<Pod, Fault> {
        Ok(p.clone())
    }
}

impl<T: ToPod> ToPod for Option<T> {
    fn to_pod(&self) -> Pod {
        self.as_ref().map_or(Pod::Nil, |x| Pod::Some(Box::new(x.to_pod())))
    }
}
impl<T: FromPod> FromPod for Option<T> {
    fn from_pod(p: &Pod) -> Result<Option<T>, Fault> {
        match p.structure() {
            Pod::Nil => Ok(None),
            Pod::Some(x) => T::from_pod(x).map(Some),
            other => mismatch("an option", other),
        }
    }
}

impl<T: ToPod> ToPod for Result<T, Error> {
    fn to_pod(&self) -> Pod {
        match self {
            Ok(x) => Pod::Ok(Box::new(x.to_pod())),
            Err(e) => Pod::Failed(Box::new(e.to_pod())),
        }
    }
}
impl<T: FromPod> FromPod for Result<T, Error> {
    fn from_pod(p: &Pod) -> Result<Result<T, Error>, Fault> {
        match p.structure() {
            Pod::Ok(x) => T::from_pod(x).map(Ok),
            Pod::Failed(e) => Error::from_pod(e).map(Err),
            other => mismatch("a ~ value", other),
        }
    }
}

impl ToPod for Error {
    fn to_pod(&self) -> Pod {
        Pod::record("Error", vec![("kind", Pod::name(&self.kind)), ("message", Pod::Text(self.message.clone()))])
    }
}
impl FromPod for Error {
    fn from_pod(p: &Pod) -> Result<Error, Fault> {
        match (p.field("kind").map(Pod::structure), p.field("message").and_then(Pod::as_text)) {
            (Some(Pod::Choice(kind, a)), Some(m)) if a.is_empty() => Ok(Error { kind: kind.clone(), message: m.clone() }),
            _ => mismatch("an Error", p),
        }
    }
}

impl<T: ToPod> ToPod for Rc<T> {
    fn to_pod(&self) -> Pod {
        (**self).to_pod()
    }
}
impl<T: FromPod> FromPod for Rc<T> {
    fn from_pod(p: &Pod) -> Result<Rc<T>, Fault> {
        T::from_pod(p).map(Rc::new)
    }
}

impl<T: ToPod + Clone> ToPod for Array<T> {
    fn to_pod(&self) -> Pod {
        Pod::Array(self.iter().map(ToPod::to_pod).collect())
    }
}
impl<T: FromPod + Clone> FromPod for Array<T> {
    fn from_pod(p: &Pod) -> Result<Array<T>, Fault> {
        let Pod::Array(xs) = p.structure() else { return mismatch("an array", p) };
        xs.iter().map(T::from_pod).collect()
    }
}

impl<K: ToPod + Ord + Clone, V: ToPod + Clone> ToPod for Map<K, V> {
    fn to_pod(&self) -> Pod {
        Pod::Map(self.iter().map(|(k, v)| (k.to_pod(), v.to_pod())).collect())
    }
}
impl<K: FromPod + Ord + Clone, V: FromPod + Clone> FromPod for Map<K, V> {
    fn from_pod(p: &Pod) -> Result<Map<K, V>, Fault> {
        let Pod::Map(es) = p.structure() else { return mismatch("a map", p) };
        es.iter().map(|(k, v)| Ok((K::from_pod(k)?, V::from_pod(v)?))).collect()
    }
}

macro_rules! pod_tuple {
    ($($t:ident $i:tt),+) => {
        impl<$($t: ToPod),+> ToPod for ($($t,)+) {
            fn to_pod(&self) -> Pod {
                Pod::Tuple(vec![$(self.$i.to_pod()),+])
            }
        }
        impl<$($t: FromPod),+> FromPod for ($($t,)+) {
            fn from_pod(p: &Pod) -> Result<Self, Fault> {
                match p.structure() {
                    Pod::Tuple(xs) if xs.len() == [$($i),+].len() => Ok(($($t::from_pod(&xs[$i])?,)+)),
                    other => mismatch("a tuple", other),
                }
            }
        }
    };
}

pod_tuple!(A 0, B 1);
pod_tuple!(A 0, B 1, C 2);
pod_tuple!(A 0, B 1, C 2, D 3);
pod_tuple!(A 0, B 1, C 2, D 3, E 4);
pod_tuple!(A 0, B 1, C 2, D 3, E 4, F 5);
