//! The dynamic value representation every program on this branch uses.
//!
//! Type System ch. 2 ("Plain values, and how they print"; "Records, choices, bundles";
//! "Collections"; "Numbers") gives the values; Semantics Appendix B gives their cost
//! promises, which the representations here are chosen to keep:
//!
//! - `Nat` and `Int` are arbitrary precision (round 47 BI), as [`BigInt`]s.
//! - `Text` is a rope ([`ropey::Rope`]): `Len` constant, `Take`/`Drop` logarithmic.
//! - Maps are ordered and persistent ([`im_rc::OrdMap`]): `Insert`, `Remove`, `?Get`
//!   logarithmic, iteration in key order.
//! - Arrays are persistent RRB vectors ([`im_rc::Vector`]): `Append` logarithmic, `Push`
//!   amortised constant.
//!
//! Values are immutable and reference counted. A value never points at a node of the
//! graph directly: an event, a dynamic or a widget is a *handle*, a [`NodeId`] or a
//! [`Blueprint`], so the only cycles in memory are cycles of node ids inside the graph's
//! arena, which the collector (`gc`) sweeps (decisions.md D5).
//!
//! A value of one type is only ever compared with a value of the same type in a checked
//! program, so the total order below ([`Ord`]) only has to be a total order; across
//! variants it is by variant, and the three integer types compare numerically with each
//! other so that a test's literal `1` matches a program's `Nat`, `Int` or `U8`.

use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::fmt;
use std::rc::Rc;

use im_rc::{OrdMap, Vector};
use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};

use crate::widget::Build;

/// A tick number. Tick 0 is the build (Semantics ch. 7, "The tick as a pure function").
/// `-1` is "before the build", the opening of tick 0.
pub type Tick = i64;

/// A node of the graph: an event, a dynamic, a place or a builder (Graph ch. 2).
/// Opaque to programs; an index into the graph's arena.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NodeId(pub(crate) u32);

/// A widget instance on (or recently on) the tree (Graph ch. 2, "A graph and its
/// documents").
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct WidgetId(pub(crate) u32);

/// A failure that escapes a tick. Not a Prism `~` value (those are [`Value::Fallible`]):
/// a `Fault` makes the tick fail, and a failed tick never happened (Semantics ch. 9, "A
/// tick that exhausts a World's patience never happened"; ch. 10).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fault {
    /// What failed, in the reporter's terms, e.g. `"Prelude.Unmet"` or `"Runtime.Type"`.
    pub kind: String,
    /// The human form.
    pub message: String,
}

impl Fault {
    pub fn new(kind: impl Into<String>, message: impl Into<String>) -> Fault {
        Fault { kind: kind.into(), message: message.into() }
    }
    /// A fault in the runtime's use: a value of the wrong kind where the runtime needed
    /// another. Under a checked program these never happen.
    pub fn runtime(message: impl Into<String>) -> Fault {
        Fault::new("Runtime.Misuse", message)
    }
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl std::error::Error for Fault {}

// ---------------------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------------------

/// `Text`: a rope of Unicode scalar values (Semantics Appendix B, Text: `Len` constant,
/// `Take` and `Drop` logarithmic "by a rope"). Cloning is constant time.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Text(pub ropey::Rope);

impl Text {
    pub fn new(s: &str) -> Text {
        Text(ropey::Rope::from_str(s))
    }
    /// `Len`: the number of characters. Constant.
    pub fn len(&self) -> usize {
        self.0.len_chars()
    }
    pub fn is_empty(&self) -> bool {
        self.0.len_chars() == 0
    }
    /// `Take text, n`: the first `n` characters. Logarithmic: a clone shares the rope's
    /// nodes, and the removal copies one path.
    pub fn take(&self, n: usize) -> Text {
        let mut r = self.0.clone();
        let n = n.min(r.len_chars());
        r.remove(n..);
        Text(r)
    }
    /// `Drop text, n`: everything after the first `n` characters. Logarithmic.
    pub fn drop_front(&self, n: usize) -> Text {
        let mut r = self.0.clone();
        let n = n.min(r.len_chars());
        r.remove(..n);
        Text(r)
    }
    /// Text concatenation, as interpolation does it. Logarithmic in the left operand's
    /// rope, linear in the right one's length.
    pub fn concat(&self, other: &Text) -> Text {
        let mut r = self.0.clone();
        for chunk in other.0.chunks() {
            let end = r.len_chars();
            r.insert(end, chunk);
        }
        Text(r)
    }
    pub fn char_at(&self, i: usize) -> Option<char> {
        if i < self.0.len_chars() { Some(self.0.char(i)) } else { None }
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for chunk in self.0.chunks() {
            f.write_str(chunk)?;
        }
        Ok(())
    }
}

impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_string())
    }
}

// ---------------------------------------------------------------------------------------
// Paths (Semantics ch. 8, "Paths and generations")
// ---------------------------------------------------------------------------------------

/// One position in a path: an element statement's number, a `rebuild`'s generation `#g`,
/// or a `for`'s or a host document's key `[k]`.
///
/// The derived order is the book's path order within one depth: positions and
/// generations by number, keys by their value's order. Two kinds never meet at one depth
/// under one parent, since only a `rebuild` makes generations and only a `for` or a host
/// makes keys.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Segment {
    Pos(u32),
    Gen(u32),
    Key(Value),
}

/// A path: a chain of positions from the root of a document. Opaque to programs except
/// for equality and order (Semantics ch. 8). The derived order on the vector is the
/// book's path order: segment by segment, a path before every path it prefixes.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
pub struct Path(pub Vec<Segment>);

impl Path {
    pub fn root() -> Path {
        Path(Vec::new())
    }
    pub fn child(&self, seg: Segment) -> Path {
        let mut v = self.0.clone();
        v.push(seg);
        Path(v)
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&crate::pod::print_path(self))
    }
}

// ---------------------------------------------------------------------------------------
// Aggregates
// ---------------------------------------------------------------------------------------

/// A record: a name and its fields, in declaration order (Type System ch. 2, Records).
/// Positional construction is the interpreter's business; the runtime keeps the names,
/// since every record prints with its fields named.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Record {
    pub name: Rc<str>,
    pub fields: Vec<(Rc<str>, Value)>,
}

/// A choice value: its constructor and the constructor's arguments (Type System ch. 2,
/// Choices). A unit type's one value (`click`) is a choice with no payload and a
/// lowercase name; a fault kind (`Fs.NotFound`) is a choice with a qualified name.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Choice {
    pub ctor: Rc<str>,
    pub payload: Vec<Value>,
}

/// What a tag's field promises (Semantics ch. 8, "Fields: what each sigil promises").
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FieldKind {
    /// A plain field: identity. It never changes while the place lives.
    Identity,
    /// A `$` field: live. It changes in place, and the place stays the same place.
    Live,
    /// A `!` field: a request. Each firing asks the World for something.
    Request,
}

/// What a tag answers its place with, the output in `tag T of Element o`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum OutputKind {
    /// No output parameter: every element is answered with `unit` (ch. 8, "Elements with
    /// no index").
    Unit,
    /// `!o`: answers are firings, addressed by path.
    Event,
    /// `$o`: a first value from the first-value query, then pushes keyed by value.
    Dynamic,
    /// A plain `o`: answered once, by the first-value query, when the place is made.
    Plain,
    /// A bundle, `((WindowId, !WindowInput))`: a tuple whose components are each answered
    /// as that kind alone is (Semantics ch. 8, "Elements cross as two projections": a type
    /// with both inside becomes several channels, and the program never sees the split).
    /// The place's output is the tuple of the components' handles and values.
    ///
    /// How a World addresses the components (the books leave it open; each is this
    /// crate's pick, listed in API.md):
    ///
    /// - **The first-value query** answers the plain and dynamic components together: with
    ///   one such component, the World gives its value; with several, a tuple of them in
    ///   order.
    /// - **An answer by path** fires the event components: with one, the answer is its
    ///   value (`1: Press Point(..)`, doubt ED); with several, a tuple with `? v` or `nil`
    ///   per event component.
    /// - **A push** writes the dynamic components, by the same rule as the first value
    ///   over the dynamic components alone.
    ///
    /// A component is `Event`, `Dynamic` or `Plain`; a bundle does not nest.
    Bundle(Vec<OutputKind>),
}

/// A tag declaration: `tag Ui.Button of Element !Click: label: $Text`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TagDecl {
    /// The qualified name, `Family.Tag`.
    pub name: Rc<str>,
    pub fields: Vec<(Rc<str>, FieldKind)>,
    pub output: OutputKind,
}

impl TagDecl {
    pub fn new(name: &str, fields: &[(&str, FieldKind)], output: OutputKind) -> Rc<TagDecl> {
        Rc::new(TagDecl {
            name: name.into(),
            fields: fields.iter().map(|(n, k)| (Rc::from(*n), *k)).collect(),
            output,
        })
    }
    /// The family, the part before the last dot: `Ui` for `Ui.Button`.
    pub fn family(&self) -> &str {
        family_of(&self.name)
    }
    pub fn field_kind(&self, field: &str) -> Option<FieldKind> {
        self.fields.iter().find(|(n, _)| &**n == field).map(|(_, k)| *k)
    }
    /// How many request fields the tag has; the transcript names the field of a request
    /// only when there is more than one.
    pub fn request_fields(&self) -> usize {
        self.fields.iter().filter(|(_, k)| *k == FieldKind::Request).count()
    }
}

/// The family of a qualified tag name: everything before the last dot.
pub fn family_of(name: &str) -> &str {
    match name.rfind('.') {
        Some(i) => &name[..i],
        None => "",
    }
}

/// An element value: a tag and its fields (Semantics ch. 8, "Elements and the element
/// statement"). An element built by a program carries its [`TagDecl`], and its live and
/// request fields hold [`Value::Dynamic`] and [`Value::Event`] handles. An element read
/// back from a transcript, or a [`Level`](crate::document) of one, holds plain values only
/// and may have no declaration. Equality and order ignore the declaration.
#[derive(Clone, Debug)]
pub struct Element {
    pub tag: Rc<str>,
    pub fields: Vec<(Rc<str>, Value)>,
    pub decl: Option<Rc<TagDecl>>,
}

impl Element {
    /// An element of a declared tag. Fields are given by name in any order and stored in
    /// declaration order; a field the call leaves out is a fault, except a request field,
    /// which defaults to `never`.
    pub fn new(decl: &Rc<TagDecl>, fields: Vec<(&str, Value)>) -> Result<Element, Fault> {
        let mut out = Vec::with_capacity(decl.fields.len());
        for (name, kind) in &decl.fields {
            match fields.iter().find(|(n, _)| *n == &**name) {
                Some((_, v)) => out.push((name.clone(), v.clone())),
                None if *kind == FieldKind::Request => out.push((name.clone(), Value::Unit)),
                None => {
                    return Err(Fault::runtime(format!("{} needs its field {}", decl.name, name)));
                }
            }
        }
        for (n, _) in &fields {
            if decl.field_kind(n).is_none() {
                return Err(Fault::runtime(format!("{} has no field {}", decl.name, n)));
            }
        }
        Ok(Element { tag: decl.name.clone(), fields: out, decl: Some(decl.clone()) })
    }
    /// An element with plain fields and no declaration, as the Pod reader makes one.
    pub fn plain(tag: &str, fields: Vec<(&str, Value)>) -> Element {
        Element {
            tag: tag.into(),
            fields: fields.into_iter().map(|(n, v)| (Rc::from(n), v)).collect(),
            decl: None,
        }
    }
    pub fn family(&self) -> &str {
        family_of(&self.tag)
    }
    pub fn field(&self, name: &str) -> Option<&Value> {
        self.fields.iter().find(|(n, _)| &**n == name).map(|(_, v)| v)
    }
}

impl PartialEq for Element {
    fn eq(&self, other: &Element) -> bool {
        self.tag == other.tag && self.fields == other.fields
    }
}
impl Eq for Element {}
impl PartialOrd for Element {
    fn partial_cmp(&self, other: &Element) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Element {
    fn cmp(&self, other: &Element) -> Ordering {
        self.tag.cmp(&other.tag).then_with(|| self.fields.cmp(&other.fields))
    }
}

// ---------------------------------------------------------------------------------------
// Closures and blueprints
// ---------------------------------------------------------------------------------------

thread_local! {
    static NEXT_SERIAL: Cell<u64> = const { Cell::new(1) };
}

fn next_serial() -> u64 {
    NEXT_SERIAL.with(|c| {
        let n = c.get();
        c.set(n + 1);
        n
    })
}

/// The host function behind a closure: `f(captured, args)`.
pub type NativeFn = dyn Fn(&[Value], &[Value]) -> Result<Value, Fault>;

/// A function value: a host-supplied function plus the values it captured.
///
/// The captured values are explicit, not hidden in the Rust closure, so the collector
/// can trace them (a closure that captures a hold of another widget is how the zombie
/// loop of Graph ch. 4, "The one loop", forms). Closures compare by identity: two
/// closures are equal only if they are the same closure.
#[derive(Clone)]
pub struct Closure(Rc<ClosureCell>);

pub struct ClosureCell {
    pub serial: u64,
    pub name: Rc<str>,
    pub captured: Vec<Value>,
    pub f: Rc<NativeFn>,
}

impl Closure {
    pub fn new(
        name: &str,
        captured: Vec<Value>,
        f: impl Fn(&[Value], &[Value]) -> Result<Value, Fault> + 'static,
    ) -> Closure {
        crate::gc::debug::closure_born();
        Closure(Rc::new(ClosureCell {
            serial: next_serial(),
            name: name.into(),
            captured,
            f: Rc::new(f),
        }))
    }
    /// A closure with nothing captured.
    pub fn pure(name: &str, f: impl Fn(&[Value]) -> Result<Value, Fault> + 'static) -> Closure {
        Closure::new(name, Vec::new(), move |_, args| f(args))
    }
    pub fn call(&self, args: &[Value]) -> Result<Value, Fault> {
        (self.0.f)(&self.0.captured, args)
    }
    pub fn name(&self) -> &str {
        &self.0.name
    }
    pub fn captured(&self) -> &[Value] {
        &self.0.captured
    }
    pub fn serial(&self) -> u64 {
        self.0.serial
    }
}

impl Drop for ClosureCell {
    fn drop(&mut self) {
        crate::gc::debug::closure_died();
    }
}

/// The host function behind a blueprint: it builds a widget through a [`Build`] and
/// returns the widget's result. `f(build, captured, args)`.
pub type BuildFn = dyn Fn(&mut Build, &[Value], &[Value]) -> Result<Value, Fault>;

/// A blueprint, an `@a` value: a widget function applied to its arguments, not yet built
/// (Semantics ch. 4, "The widget block"). Building one is a statement on the left edge;
/// applying one with [`Build::call`] is transparent and adds no path segment.
#[derive(Clone)]
pub struct Blueprint(Rc<BlueprintCell>);

pub struct BlueprintCell {
    pub serial: u64,
    pub name: Rc<str>,
    pub captured: Vec<Value>,
    pub f: Rc<BuildFn>,
}

impl Blueprint {
    pub fn new(
        name: &str,
        captured: Vec<Value>,
        f: impl Fn(&mut Build, &[Value], &[Value]) -> Result<Value, Fault> + 'static,
    ) -> Blueprint {
        crate::gc::debug::closure_born();
        Blueprint(Rc::new(BlueprintCell {
            serial: next_serial(),
            name: name.into(),
            captured,
            f: Rc::new(f),
        }))
    }
    /// A blueprint with nothing captured.
    pub fn pure(
        name: &str,
        f: impl Fn(&mut Build, &[Value]) -> Result<Value, Fault> + 'static,
    ) -> Blueprint {
        Blueprint::new(name, Vec::new(), move |b, _, args| f(b, args))
    }
    pub fn build(&self, b: &mut Build, args: &[Value]) -> Result<Value, Fault> {
        (self.0.f)(b, &self.0.captured, args)
    }
    pub fn name(&self) -> &str {
        &self.0.name
    }
    pub fn captured(&self) -> &[Value] {
        &self.0.captured
    }
    pub fn serial(&self) -> u64 {
        self.0.serial
    }
}

impl Drop for BlueprintCell {
    fn drop(&mut self) {
        crate::gc::debug::closure_died();
    }
}

/// An `&` reference: a mutable cell valid inside one `&` block (Semantics ch. 5). The
/// borrow rule is the checker's; the runtime only provides the cell. References compare
/// by identity.
#[derive(Clone)]
pub struct Ref(Rc<RefCellBox>);

pub struct RefCellBox {
    pub serial: u64,
    pub cell: RefCell<Value>,
}

impl Ref {
    pub fn new(v: Value) -> Ref {
        Ref(Rc::new(RefCellBox { serial: next_serial(), cell: RefCell::new(v) }))
    }
    pub fn get(&self) -> Value {
        self.0.cell.borrow().clone()
    }
    pub fn set(&self, v: Value) {
        *self.0.cell.borrow_mut() = v;
    }
    pub fn serial(&self) -> u64 {
        self.0.serial
    }
}

// ---------------------------------------------------------------------------------------
// Value
// ---------------------------------------------------------------------------------------

/// A Prism value at run time.
///
/// Plain values (data, Type System ch. 2, V6) print and read back in the transcript's
/// spelling (`pod`). Handles (`Event`, `Dynamic`, `Widget`) and functions do not print as
/// data; a bundle, a plain aggregate with handles inside, is still a `Value`.
#[derive(Clone, Default)]
pub enum Value {
    #[default]
    Unit,
    Bool(bool),
    /// `Nat`: arbitrary precision, never negative.
    Nat(BigInt),
    /// `Int`: arbitrary precision.
    Int(BigInt),
    /// `U8`, the one sized integer on this branch (decisions.md, Features in and out).
    U8(u8),
    /// `Float`, IEEE double (decisions.md D14; Type System ch. 2, Numbers). Printed as the
    /// shortest decimal that reads back ([`float_spelling`]). As a value it is ordered by
    /// IEEE's total order, so it can key a map; `==` in a program is IEEE equality
    /// ([`float_eq`]), which an interpreter asks for by name.
    Float(f64),
    Char(char),
    Text(Text),
    /// A `Duration` in nanoseconds; only as data on this branch (no arithmetic), for
    /// `Time.After`'s `Wake(after=0s, ..)`.
    Duration(i128),
    Record(Rc<Record>),
    /// A tuple, `((a, b))`.
    Tuple(Rc<Vec<Value>>),
    Choice(Rc<Choice>),
    Map(OrdMap<Value, Value>),
    Array(Vector<Value>),
    /// `?T`: `nil` or `? x`.
    Opt(Option<Rc<Value>>),
    /// `~T`: `~ x` (Ok) or an `Error` record (Err) (Semantics ch. 10).
    Fallible(Result<Rc<Value>, Rc<Value>>),
    Closure(Closure),
    Element(Rc<Element>),
    Path(Path),
    /// A `!` handle: an event node.
    Event(NodeId),
    /// A `$` handle: a dynamic node, a sample plus an update event.
    Dynamic(NodeId),
    /// An `@` handle: a blueprint.
    Widget(Blueprint),
    /// An `&` reference.
    Ref(Ref),
}

impl Value {
    pub fn nat(n: u64) -> Value {
        Value::Nat(BigInt::from(n))
    }
    pub fn int(n: i64) -> Value {
        Value::Int(BigInt::from(n))
    }
    pub fn text(s: &str) -> Value {
        Value::Text(Text::new(s))
    }
    pub fn float(f: f64) -> Value {
        Value::Float(f)
    }
    pub fn some(v: Value) -> Value {
        Value::Opt(Some(Rc::new(v)))
    }
    pub fn nil() -> Value {
        Value::Opt(None)
    }
    pub fn ok(v: Value) -> Value {
        Value::Fallible(Ok(Rc::new(v)))
    }
    /// A failed `~T`: `Error(kind=..., message=...)`.
    pub fn error(kind: &str, message: &str) -> Value {
        Value::Fallible(Err(Rc::new(Value::record(
            "Error",
            vec![("kind", Value::ctor(kind, vec![])), ("message", Value::text(message))],
        ))))
    }
    pub fn record(name: &str, fields: Vec<(&str, Value)>) -> Value {
        Value::Record(Rc::new(Record {
            name: name.into(),
            fields: fields.into_iter().map(|(n, v)| (Rc::from(n), v)).collect(),
        }))
    }
    pub fn ctor(name: &str, payload: Vec<Value>) -> Value {
        Value::Choice(Rc::new(Choice { ctor: name.into(), payload }))
    }
    pub fn tuple(items: Vec<Value>) -> Value {
        Value::Tuple(Rc::new(items))
    }
    pub fn array(items: Vec<Value>) -> Value {
        Value::Array(items.into_iter().collect())
    }
    pub fn map(entries: Vec<(Value, Value)>) -> Value {
        Value::Map(entries.into_iter().collect())
    }
    pub fn element(e: Element) -> Value {
        Value::Element(Rc::new(e))
    }

    /// The value of a unit type `type Click = click`: a payload-free choice.
    pub fn click() -> Value {
        Value::ctor("click", vec![])
    }

    pub fn is_unit(&self) -> bool {
        matches!(self, Value::Unit)
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
    /// Any integer as an `i128`, if it fits.
    pub fn as_i128(&self) -> Option<i128> {
        match self {
            Value::Nat(n) | Value::Int(n) => n.to_i128(),
            Value::U8(n) => Some(*n as i128),
            _ => None,
        }
    }
    /// A `Float`'s value.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }
    pub fn as_text(&self) -> Option<&Text> {
        match self {
            Value::Text(t) => Some(t),
            _ => None,
        }
    }
    pub fn as_element(&self) -> Option<&Element> {
        match self {
            Value::Element(e) => Some(e),
            _ => None,
        }
    }
    /// The field of a record or an element.
    pub fn field(&self, name: &str) -> Option<&Value> {
        match self {
            Value::Record(r) => r.fields.iter().find(|(n, _)| &**n == name).map(|(_, v)| v),
            Value::Element(e) => e.field(name),
            _ => None,
        }
    }
    /// True for a value with no handle, function or reference anywhere inside: data, in
    /// Type System ch. 2's sense, which prints.
    pub fn is_plain(&self) -> bool {
        match self {
            Value::Closure(_)
            | Value::Event(_)
            | Value::Dynamic(_)
            | Value::Widget(_)
            | Value::Ref(_) => false,
            Value::Record(r) => r.fields.iter().all(|(_, v)| v.is_plain()),
            Value::Tuple(t) => t.iter().all(Value::is_plain),
            Value::Choice(c) => c.payload.iter().all(Value::is_plain),
            Value::Map(m) => m.iter().all(|(k, v)| k.is_plain() && v.is_plain()),
            Value::Array(a) => a.iter().all(Value::is_plain),
            Value::Opt(Some(v)) => v.is_plain(),
            Value::Fallible(Ok(v)) | Value::Fallible(Err(v)) => v.is_plain(),
            Value::Element(e) => e.fields.iter().all(|(_, v)| v.is_plain()),
            _ => true,
        }
    }

    /// The variant's rank in the total order across variants.
    fn rank(&self) -> u8 {
        match self {
            Value::Unit => 0,
            Value::Bool(_) => 1,
            Value::Nat(_) | Value::Int(_) | Value::U8(_) => 2,
            Value::Float(_) => 20,
            Value::Char(_) => 3,
            Value::Text(_) => 4,
            Value::Duration(_) => 5,
            Value::Record(_) => 6,
            Value::Tuple(_) => 7,
            Value::Choice(_) => 8,
            Value::Map(_) => 9,
            Value::Array(_) => 10,
            Value::Opt(_) => 11,
            Value::Fallible(_) => 12,
            Value::Closure(_) => 13,
            Value::Element(_) => 14,
            Value::Path(_) => 15,
            Value::Event(_) => 16,
            Value::Dynamic(_) => 17,
            Value::Widget(_) => 18,
            Value::Ref(_) => 19,
        }
    }

    fn integer(&self) -> Option<BigInt> {
        match self {
            Value::Nat(n) | Value::Int(n) => Some(n.clone()),
            Value::U8(n) => Some(BigInt::from(*n)),
            _ => None,
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Value {}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Value) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Value) -> Ordering {
        use Value::*;
        let (ra, rb) = (self.rank(), other.rank());
        if ra != rb {
            return ra.cmp(&rb);
        }
        match (self, other) {
            (Unit, Unit) => Ordering::Equal,
            (Bool(a), Bool(b)) => a.cmp(b),
            (Float(a), Float(b)) => a.total_cmp(b),
            (Char(a), Char(b)) => a.cmp(b),
            (Text(a), Text(b)) => a.cmp(b),
            (Duration(a), Duration(b)) => a.cmp(b),
            (Record(a), Record(b)) => a.cmp(b),
            (Tuple(a), Tuple(b)) => a.cmp(b),
            (Choice(a), Choice(b)) => a.cmp(b),
            (Map(a), Map(b)) => a.cmp(b),
            (Array(a), Array(b)) => a.cmp(b),
            (Opt(a), Opt(b)) => a.cmp(b),
            (Fallible(a), Fallible(b)) => match (a, b) {
                // An `Error` sorts before a success, as `nil` sorts before `? x`.
                (Err(x), Err(y)) | (Ok(x), Ok(y)) => x.cmp(y),
                (Err(_), Ok(_)) => Ordering::Less,
                (Ok(_), Err(_)) => Ordering::Greater,
            },
            (Closure(a), Closure(b)) => a.serial().cmp(&b.serial()),
            (Element(a), Element(b)) => a.cmp(b),
            (Path(a), Path(b)) => a.cmp(b),
            (Event(a), Event(b)) => a.cmp(b),
            (Dynamic(a), Dynamic(b)) => a.cmp(b),
            (Widget(a), Widget(b)) => a.serial().cmp(&b.serial()),
            (Ref(a), Ref(b)) => a.serial().cmp(&b.serial()),
            _ => {
                // The integers: numerically, across Nat, Int and U8.
                let (a, b) = (self.integer(), other.integer());
                a.cmp(&b)
            }
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&crate::pod::print(self))
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&crate::pod::print(self))
    }
}

// ---------------------------------------------------------------------------------------
// Float (decisions.md D14)
// ---------------------------------------------------------------------------------------

/// The transcript's spelling of a float (Type System ch. 2, "Plain values, and how they
/// print"): the shortest decimal that reads back, always with a point, never with an
/// exponent; `nan`, `inf`, `-inf`. Rust's `Display` for `f64` is the shortest round-trip
/// decimal and never writes an exponent, so only the point is added.
pub fn float_spelling(f: f64) -> String {
    if f.is_nan() {
        return "nan".into();
    }
    if f.is_infinite() {
        return if f > 0.0 { "inf".into() } else { "-inf".into() };
    }
    let s = format!("{f}");
    if s.contains('.') { s } else { format!("{s}.0") }
}

/// `==` on `Float`: IEEE equality, so `nan != nan` and `0.0 == -0.0` (Type System ch. 2).
/// The value order ([`Ord`] on [`Value`]) is IEEE's total order instead, which `^Ord` is.
pub fn float_eq(a: f64, b: f64) -> bool {
    a == b
}

/// The arithmetic operators on `Float` (Type System ch. 2, Numbers; ch. 3, `Quotient`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FloatOp {
    Add,
    Sub,
    Mul,
    /// `/` on `Float` gives a `Float` (IEEE: `x / 0.0` is an infinity or `nan`, never a
    /// failure; Semantics ch. 10, "Total operations").
    Div,
    /// `mod`: the remainder with the divisor's sign, as `mod` floors on integers.
    Mod,
}

/// One `Float` operation, IEEE double throughout.
pub fn float_op(op: FloatOp, x: f64, y: f64) -> f64 {
    match op {
        FloatOp::Add => x + y,
        FloatOp::Sub => x - y,
        FloatOp::Mul => x * y,
        FloatOp::Div => x / y,
        FloatOp::Mod => {
            let r = x % y;
            if r != 0.0 && (r < 0.0) != (y < 0.0) { r + y } else { r }
        }
    }
}

/// Integer helpers an interpreter needs for `Nat`'s `-?` (round 47 NS): `nil` below zero.
pub fn nat_checked_sub(a: &BigInt, b: &BigInt) -> Value {
    let d = a - b;
    if d.is_negative() { Value::nil() } else { Value::some(Value::Nat(d)) }
}

/// True for a `Nat` or `Int` zero, or a `U8` zero.
pub fn is_zero(v: &Value) -> bool {
    match v {
        Value::Nat(n) | Value::Int(n) => n.is_zero(),
        Value::U8(n) => *n == 0,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_compare_numerically_across_types() {
        assert_eq!(Value::nat(3), Value::int(3));
        assert_eq!(Value::U8(3), Value::nat(3));
        assert!(Value::int(-1) < Value::nat(0));
    }

    #[test]
    fn floats_print_shortest_with_a_point() {
        for (f, s) in [
            (280.0, "280.0"),
            (0.1, "0.1"),
            (-0.5, "-0.5"),
            (1e21, "1000000000000000000000.0"),
            (1e-7, "0.0000001"),
            (-0.0, "-0.0"),
            (f64::NAN, "nan"),
            (f64::INFINITY, "inf"),
            (f64::NEG_INFINITY, "-inf"),
            (0.1 + 0.2, "0.30000000000000004"),
        ] {
            assert_eq!(float_spelling(f), s);
        }
    }

    #[test]
    fn floats_order_totally_and_compare_by_ieee() {
        assert!(Value::float(-1.0) < Value::float(0.5));
        assert_eq!(Value::float(f64::NAN), Value::float(f64::NAN));
        assert!(!float_eq(f64::NAN, f64::NAN));
        assert!(float_eq(0.0, -0.0));
        assert_eq!(float_op(FloatOp::Mod, -1.0, 3.0), 2.0);
        assert_eq!(float_op(FloatOp::Div, 1.0, 0.0), f64::INFINITY);
    }

    #[test]
    fn text_take_and_drop_share_the_rope() {
        let t = Text::new("hello world");
        assert_eq!(t.take(5).to_string(), "hello");
        assert_eq!(t.drop_front(6).to_string(), "world");
        assert_eq!(t.len(), 11);
        assert_eq!(t.take(99).to_string(), "hello world");
    }

    #[test]
    fn text_concat() {
        let a = Text::new("ab");
        assert_eq!(a.concat(&Text::new("cd")).to_string(), "abcd");
    }

    #[test]
    fn maps_are_ordered_by_key() {
        let m = Value::map(vec![(Value::nat(2), Value::text("b")), (Value::nat(1), Value::text("a"))]);
        match &m {
            Value::Map(m) => {
                let keys: Vec<_> = m.keys().cloned().collect();
                assert_eq!(keys, vec![Value::nat(1), Value::nat(2)]);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn path_order_puts_a_prefix_first() {
        let a = Path(vec![Segment::Pos(1)]);
        let b = a.child(Segment::Gen(0));
        let c = Path(vec![Segment::Pos(2)]);
        assert!(a < b && b < c);
    }

    #[test]
    fn closures_compare_by_identity() {
        let f = Closure::pure("f", |_| Ok(Value::Unit));
        let g = Closure::pure("f", |_| Ok(Value::Unit));
        assert_eq!(Value::Closure(f.clone()), Value::Closure(f));
        assert_ne!(Value::Closure(g.clone()), Value::Closure(Closure::pure("f", |_| Ok(Value::Unit))));
    }

    #[test]
    fn element_new_checks_fields() {
        let d = TagDecl::new("Ui.Label", &[("text", FieldKind::Live)], OutputKind::Unit);
        assert!(Element::new(&d, vec![]).is_err());
        assert!(Element::new(&d, vec![("text", Value::text("x")), ("y", Value::Unit)]).is_err());
        let e = Element::new(&d, vec![("text", Value::text("x"))]).unwrap();
        assert_eq!(e.family(), "Ui");
    }
}
