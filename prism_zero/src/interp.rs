//! The interpreter's core: code objects, closures and blueprints over `prism_rt`, scopes,
//! globals, application with the lifting table, and unbounded functions.
//!
//! **Functions are `prism_rt` closures.** Every function value of a Prism program is a
//! [`prism_rt::Closure`] (and every blueprint a [`prism_rt::Blueprint`]), so the runtime
//! can call it from a node without knowing it came from an interpreter. Its behaviour is
//! a [`Code`] in the interpreter's table, named by the number after the last `#` in the
//! closure's name; its environment travels in the closure's `captured` values, which the
//! collector traces (decisions.md D5: "Captured values are explicit so the collector
//! traces them"). The layout of `captured` for code with an environment is
//! `[names, extra, value...]`: an array of the captured names as texts, an array of extra
//! values (a widget function's arguments), then one value per name.
//!
//! **The lifting table at each application** (Type System ch. 4, "Where the table
//! looks"; D3). At an application the interpreter compares each argument's runtime layer
//! with the parameter's declared one ([`Layer`]):
//!
//! - the parameter already takes it (a `$Text` parameter, a `!a`, a blueprint, a tag's
//!   field, a constructor's type-parameter field): ordinary application;
//! - otherwise a `$` or `!` argument is peeled and the row is chosen: dynamics and no
//!   event make a map over the dynamics (`Build::lift`, the change law); one event makes
//!   a sample (`Build::sample`, the tag law); two events are refused, naming both;
//! - under the time layer, a `?` or `~` argument at a concrete parameter is peeled at the
//!   plain layer, `nil` or the leftmost `Error` moving outward (L5).
//!
//! A lifted application needs a widget to put its node in, so it is only possible while
//! a blueprint builds; a lift inside a node's function (at propagation time) is the
//! program asking for a node mid-tick, which the books refuse.
//!
//! **Unbounded functions** (Semantics ch. 7, "An unbounded function is a bounded body
//! iterated"; D6) run in cycles under `@Run`: a cycle point is every call of an unbounded
//! function from an unbounded function. See [`Unbounded`] for how prism_zero counts them
//! without saving continuations.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

use num_traits::Signed;
use prism_rt::{Blueprint, Build, Closure, Element, Fault, Value};
use prism_syntax::Span;
use prism_syntax::ast::*;

use crate::load::{CtorInfo, Global, Loaded, MethodInfo, ModId};
use crate::ops::{self, R};

/// An index into the code table.
pub type CodeId = usize;

/// The build a statement runs in, if any: present while a blueprint builds, absent in a
/// node's function at propagation time and in a `check` line.
pub type Bx<'x, 'a> = Option<&'x mut Build<'a>>;

/// Where an error is reported: a module and a span in it.
#[derive(Clone, Copy, Debug)]
pub struct Loc {
    pub module: ModId,
    pub span: Span,
}

/// What a parameter takes, read off its declared type (or, with no signature, off the
/// sigils of its pattern).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layer {
    /// A sigiled parameter (`$Int`, `!a`, `@a`), a tag's field, a constructor's
    /// type-parameter field: the argument goes in whole, sigils and all.
    Whole,
    /// A bare type variable: lifted over time afresh at each application, its `?` kept.
    Var,
    /// A concrete plain type: lifted over time, and `?`/`~` peeled at the plain layer.
    Concrete,
    /// A `?T` or `~T` parameter: lifted over time, its optional taken whole.
    Opt,
}

impl Layer {
    pub fn lifts_time(self) -> bool {
        self != Layer::Whole
    }
}

/// The layer of a parameter type.
pub fn layer_of(t: &Type) -> Layer {
    match crate::load::outer_sigil(t) {
        Some(Sigil::Dollar | Sigil::Bang | Sigil::At | Sigil::Amp) => return Layer::Whole,
        Some(Sigil::Question | Sigil::Tilde) => return Layer::Opt,
        None => {}
    }
    match &t.kind {
        TypeKind::Name { name, args } if name.path.len() == 1 && !name.is_upper() && args.is_empty() => Layer::Var,
        TypeKind::Hole(_) => Layer::Var,
        TypeKind::Fn { .. } => Layer::Var,
        TypeKind::Group(t) => layer_of(t),
        _ => Layer::Concrete,
    }
}

/// The layer a parameter pattern says when there is no signature: its binder's sigil.
pub fn layer_of_pattern(p: &Pattern) -> Layer {
    match &p.kind {
        PatternKind::Name(n) => match n.sigils.first() {
            Some(Sigil::Dollar | Sigil::Bang | Sigil::At | Sigil::Amp) => Layer::Whole,
            Some(_) => Layer::Opt,
            None => Layer::Var,
        },
        PatternKind::Sigil { .. } => Layer::Opt,
        _ => Layer::Var,
    }
}

/// True when a declared type is `Nat`: an integer argument is converted to it at the
/// call, since literals are `Int` until something decides (doubt LD).
pub fn is_nat_type(t: &Type) -> bool {
    matches!(&t.kind, TypeKind::Name { name, args } if args.is_empty() && name.sigils.is_empty() && name.base() == "Nat")
}

/// Convert an `Int` that a `Nat` parameter receives.
pub fn to_nat(v: Value) -> Value {
    match v {
        Value::Int(n) if !n.is_negative() => Value::Nat(n),
        other => other,
    }
}

/// A function's code.
pub struct FnCode {
    pub name: Rc<str>,
    pub sig: Option<&'static Signature>,
    pub clauses: Vec<&'static FnClause>,
    pub module: ModId,
    /// The name's first sigil: `@` a widget function, `?`/`~` a `given:` body.
    pub mark: Option<Sigil>,
    pub unbounded: bool,
    pub layers: Vec<Layer>,
    pub nat: Vec<bool>,
    /// A local function: it sees its block's names through its environment, and itself
    /// by name.
    pub local: bool,
}

/// What a blueprint over a block does when it builds.
#[derive(Clone, Copy)]
pub enum BlockKind {
    /// `@counter:` at the top of a module, or `@name:` inside a block.
    Widget,
    /// `@widget:` in an expression.
    WidgetExpr,
    /// A `rebuild`'s block: its argument is `_`.
    Rebuild,
    /// A `for`'s block: its arguments are the key, the row's value and `ix`.
    For(Option<&'static ForBinders>, bool),
}

/// A code object: what a closure or blueprint of the program does.
pub enum Code {
    Fn(FnCode),
    Lambda { params: Option<&'static [Pattern]>, body: &'static LambdaBody, module: ModId, layers: Vec<Layer> },
    Builtin { name: Rc<str>, sig: Option<&'static Signature>, layers: Vec<Layer>, nat: Vec<bool> },
    Ctor(Rc<CtorInfo>, Vec<Layer>),
    Tag(Rc<str>),
    Method(Rc<MethodInfo>, Vec<Layer>),
    /// A blueprint over a block, with its module and the row its module's `emit` sets.
    Block { block: &'static Block, kind: BlockKind, module: ModId, set_row: bool },
    /// A widget function applied: a blueprint of its arguments.
    WidgetFn(CodeId),
    /// A widget builtin applied: a blueprint of its arguments.
    WidgetBuiltin(Rc<str>),
    /// `@ v`: a blueprint whose result is `v`, evaluated when it builds.
    FreeAt { expr: &'static Expr, module: ModId },
    /// An operator as a function, for lifting.
    Op(OpKind),
    /// A text literal's interpolation: the literal's plain parts, `None` where a
    /// placeholder's value goes.
    Interp(Rc<Vec<Option<String>>>),
    /// Field access, `.f`.
    Field(Rc<str>),
    /// Indexing, `m[k]`.
    Index,
    /// `? v` and `~ v` as functions.
    Wrap(Sigil),
    /// A lifted application's node function: `captured = [f, args]`, the lifted
    /// arguments in `slots`, in the order the node passes them.
    Lifted { slots: Rc<Vec<usize>> },
    /// A `match` or `if` over a dynamic or an event with plain arms, as a lifted function
    /// of its scrutinee and the handles its arms read (Type System ch. 4, "`match` and
    /// `if` as lifting"). `captured = [names, extra, values]`; the node passes the
    /// scrutinee's value and then the lifted names' values.
    Arms { expr: &'static Expr, module: ModId, lifted: Rc<Vec<Rc<str>>>, filter: bool },
    /// A map literal with handles inside, lifted (Claude's pick, see `eval`).
    MapLit { keys: usize },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OpKind {
    Bin(BinOp),
    Not,
    Neg,
}

/// One frame of local names.
pub struct Frame {
    pub vars: HashMap<Rc<str>, Value>,
    /// A binding group's definitions not yet evaluated (Semantics ch. 4, "Binding groups
    /// and cycles"); see `block`.
    pub group: Option<Rc<crate::block::Group>>,
}

impl Frame {
    pub fn new() -> Frame {
        Frame { vars: HashMap::new(), group: None }
    }
}

impl Default for Frame {
    fn default() -> Frame {
        Frame::new()
    }
}

/// The local scopes of an evaluation.
pub struct Scopes {
    pub module: ModId,
    pub frames: Vec<Frame>,
    /// The mark of the function whose `given:` block is running: `?` or `~`.
    pub mark: Option<Sigil>,
    /// What a `rebuild` or `for` on the right of the bind being evaluated hands back,
    /// read off the binder's sigils (prism_rt API.md, `ResultKind`).
    pub bind_kind: Option<prism_rt::ResultKind>,
}

impl Scopes {
    pub fn new(module: ModId) -> Scopes {
        Scopes { module, frames: vec![Frame::new()], mark: None, bind_kind: None }
    }
    pub fn bind(&mut self, name: Rc<str>, v: Value) {
        self.frames.last_mut().expect("a frame").vars.insert(name, v);
    }
    pub fn push(&mut self) {
        self.frames.push(Frame::new());
    }
    pub fn pop(&mut self) {
        self.frames.pop();
    }
}

/// The unbounded functions' bookkeeping, per thread (Semantics ch. 7, "Unbounded is a
/// function color"; D6).
///
/// prism_zero does not save continuations. A cycle point is every call of an unbounded
/// function from an unbounded function; the cycles of a job are the stretches between
/// them. Since the function is pure, its run is the same whatever cycle it is replayed
/// in, so `@Run` runs it once to the end, counting cycle points, and then reports `More`
/// for as many cycles as the canonical rule gives and `Done` with the result in the last.
/// The transcript (which tick the result fires in) is the canonical rule's; only the cost
/// is front-loaded. A function that runs past the cycle budget is stopped at the budget's
/// cycle point and resumed from its start, with twice the budget, in a later cycle, so a
/// job that never ends (`Normalize` on omega) keeps `pending` set and every tick ends.
#[derive(Default)]
pub struct Unbounded {
    pub depth: Cell<usize>,
    pub budget: Cell<Option<u64>>,
    pub points: Cell<u64>,
}

thread_local! {
    pub static UNBOUNDED: Unbounded = Unbounded::default();
}

/// The kind of the fault that unwinds a run past its cycle budget.
pub const CYCLE_BUDGET: &str = "prism_zero.CycleBudget";

/// The interpreter.
pub struct Interp {
    pub l: Loaded,
    codes: RefCell<Vec<Rc<Code>>>,
    by_addr: RefCell<HashMap<(usize, u8), CodeId>>,
    globals: RefCell<HashMap<(ModId, Rc<str>), Value>>,
    in_progress: RefCell<HashSet<(ModId, Rc<str>)>>,
    free: RefCell<HashMap<usize, Rc<Vec<Rc<str>>>>>,
    ops: RefCell<Vec<(OpKind, CodeId)>>,
    me: Weak<Interp>,
}

impl Interp {
    pub fn new(l: Loaded) -> Rc<Interp> {
        Rc::new_cyclic(|me| Interp {
            l,
            codes: RefCell::new(Vec::new()),
            by_addr: RefCell::new(HashMap::new()),
            globals: RefCell::new(HashMap::new()),
            in_progress: RefCell::new(HashSet::new()),
            free: RefCell::new(HashMap::new()),
            ops: RefCell::new(Vec::new()),
            me: me.clone(),
        })
    }

    pub fn weak(&self) -> Weak<Interp> {
        self.me.clone()
    }

    // ----- errors -----

    /// An error at a place in the program.
    pub fn fault(&self, loc: Loc, msg: impl Into<String>) -> Fault {
        Fault::new("prism_zero.Eval", format!("{}: {}", self.l.modules[loc.module].at(loc.span), msg.into()))
    }

    /// A feature decisions.md puts out of this compiler, reached by evaluation.
    pub fn out_of_scope(&self, loc: Loc, feature: &str) -> Fault {
        Fault::new(
            "prism_zero.OutOfScope",
            format!(
                "{}: out of scope ({feature}): decisions.md puts this feature out of this compiler",
                self.l.modules[loc.module].at(loc.span)
            ),
        )
    }

    // ----- the code table -----

    pub fn add_code(&self, c: Code) -> CodeId {
        let mut codes = self.codes.borrow_mut();
        codes.push(Rc::new(c));
        codes.len() - 1
    }

    pub fn code(&self, id: CodeId) -> Rc<Code> {
        self.codes.borrow()[id].clone()
    }

    /// A code object made once per AST node (keyed by its address and a tag).
    pub fn code_at<T>(&self, node: &T, tag: u8, make: impl FnOnce() -> Code) -> CodeId {
        let key = (node as *const T as usize, tag);
        if let Some(id) = self.by_addr.borrow().get(&key) {
            return *id;
        }
        let id = self.add_code(make());
        self.by_addr.borrow_mut().insert(key, id);
        id
    }

    /// The code of a closure or a blueprint, by the number in its name.
    pub fn code_id_of(name: &str) -> Option<CodeId> {
        name.rsplit_once('#').and_then(|(_, n)| n.parse().ok())
    }

    /// A function value over code `id`.
    pub fn closure(&self, id: CodeId, display: &str, captured: Vec<Value>) -> Value {
        let me = self.weak();
        Value::Closure(Closure::new(&format!("{display}#{id}"), captured, move |cap, args| {
            let interp = me.upgrade().ok_or_else(|| Fault::runtime("the interpreter is gone"))?;
            interp.invoke(id, cap, args.to_vec(), None, None)
        }))
    }

    /// A blueprint over code `id`.
    pub fn blueprint(&self, id: CodeId, display: &str, captured: Vec<Value>) -> Value {
        let me = self.weak();
        Value::Widget(Blueprint::new(&format!("{display}#{id}"), captured, move |b, cap, args| {
            let interp = me.upgrade().ok_or_else(|| Fault::runtime("the interpreter is gone"))?;
            interp.build(id, b, cap, args)
        }))
    }

    /// The operator `op` as a function value, for lifting.
    pub fn op_value(&self, op: OpKind) -> Value {
        let id = {
            let found = self.ops.borrow().iter().find(|(o, _)| *o == op).map(|(_, id)| *id);
            match found {
                Some(id) => id,
                None => {
                    let id = self.add_code(Code::Op(op));
                    self.ops.borrow_mut().push((op, id));
                    id
                }
            }
        };
        let name = match op {
            OpKind::Bin(b) => b.as_str(),
            OpKind::Not => "not",
            OpKind::Neg => "-",
        };
        self.closure(id, name, vec![])
    }

    // ----- environments -----

    /// The free names of a node, cached: every unqualified name written in it, with its
    /// sigils, and the names the language binds (`_`, `key`, `ix`, `jx`, `kx`). An
    /// over-approximation: a name the node binds itself is captured from outside too,
    /// which costs nothing, since a written name is never rebound (Semantics ch. 4).
    pub fn free_names<T>(&self, node: &'static T, walk: impl FnOnce(&'static T, &mut Vec<Rc<str>>)) -> Rc<Vec<Rc<str>>> {
        let key = node as *const T as usize;
        if let Some(f) = self.free.borrow().get(&key) {
            return f.clone();
        }
        let mut out = Vec::new();
        walk(node, &mut out);
        let mut seen = HashSet::new();
        out.retain(|n| seen.insert(n.clone()));
        let rc = Rc::new(out);
        self.free.borrow_mut().insert(key, rc.clone());
        rc
    }

    /// Capture the locals among `names`, forcing any not yet defined in their binding
    /// group, as `[names, extra, values...]`.
    pub fn capture(&self, sc: &mut Scopes, names: &[Rc<str>], extra: Vec<Value>, mut b: Bx, loc: Loc) -> R<Vec<Value>> {
        let mut ns = Vec::new();
        let mut vs = Vec::new();
        for n in names {
            if let Some(v) = self.lookup_local(sc, n, b.as_deref_mut(), loc)? {
                ns.push(Value::Text(prism_rt::Text::new(n)));
                vs.push(v);
            }
        }
        let mut out = vec![Value::array(ns), Value::array(extra)];
        out.extend(vs);
        Ok(out)
    }

    /// Unpack `[names, extra, values...]` into a frame and the extra values.
    pub fn uncapture(cap: &[Value]) -> (Frame, Vec<Value>) {
        let mut f = Frame::new();
        let mut extra = Vec::new();
        if let (Some(Value::Array(names)), Some(Value::Array(ex))) = (cap.first(), cap.get(1)) {
            for (n, v) in names.iter().zip(cap[2..].iter()) {
                if let Value::Text(t) = n {
                    f.vars.insert(Rc::from(t.to_string()), v.clone());
                }
            }
            extra = ex.iter().cloned().collect();
        }
        (f, extra)
    }

    /// Look a name up in the local scopes, forcing a binding group's definition.
    pub fn lookup_local(&self, sc: &mut Scopes, name: &str, b: Bx, loc: Loc) -> R<Option<Value>> {
        for k in (0..sc.frames.len()).rev() {
            if let Some(v) = sc.frames[k].vars.get(name) {
                return Ok(Some(v.clone()));
            }
            if let Some(g) = sc.frames[k].group.clone() {
                if g.defines(name) {
                    return self.force(sc, k, g, name, b, loc).map(Some);
                }
            }
        }
        Ok(None)
    }

    // ----- globals -----

    /// The declaration a bare name resolves to from a module: its own, then what its
    /// `use M.*` lines and the prelude bring in.
    pub fn resolve_bare(&self, module: ModId, name: &str) -> Option<(ModId, Global)> {
        let module = self.l.modules[module].alias.unwrap_or(module);
        let m = &self.l.modules[module];
        if let Some(g) = m.decls.get(name) {
            return Some((module, g.clone()));
        }
        if let Some(&owner) = m.scope.get(name) {
            return self.l.modules[owner].decls.get(name).map(|g| (owner, g.clone()));
        }
        None
    }

    /// The value of a global, memoized.
    pub fn global(&self, owner: ModId, name: &str, g: &Global, b: Bx, loc: Loc) -> R<Value> {
        let key = (owner, Rc::from(name));
        if let Some(v) = self.globals.borrow().get(&key) {
            return Ok(v.clone());
        }
        let v = match g {
            Global::Let(ld) => {
                if !self.in_progress.borrow_mut().insert(key.clone()) {
                    return Err(self.fault(loc, format!("{name} is defined in terms of itself: a cycle among plain values (Semantics ch. 4)")));
                }
                let mut sc = Scopes::new(owner);
                let r = self.eval(&ld.value, &mut sc, None);
                self.in_progress.borrow_mut().remove(&key);
                let v = r?;
                let mut binds = Vec::new();
                if !crate::pattern::matches(self, owner, &ld.pattern, &v, &mut binds)? {
                    return Err(self.fault(Loc { module: owner, span: ld.span }, "the `let` pattern does not match its value"));
                }
                // Every binder of the pattern is memoized at once.
                for (n, bv) in &binds {
                    self.globals.borrow_mut().insert((owner, n.clone()), bv.clone());
                }
                return binds
                    .into_iter()
                    .find(|(n, _)| &**n == name)
                    .map(|(_, v)| v)
                    .ok_or_else(|| self.fault(loc, format!("{name} is not bound by its `let`")));
            }
            Global::Fn { sig, clauses } => {
                let id = self.fn_code(owner, name, *sig, clauses.clone(), false);
                self.closure(id, name, vec![])
            }
            Global::Widget { decl, .. } => {
                let id = self.add_code(Code::Block { block: &decl.body, kind: BlockKind::Widget, module: owner, set_row: true });
                self.blueprint(id, name, vec![Value::array(vec![]), Value::array(vec![])])
            }
            Global::Ctor(info) => {
                if info.fields.is_empty() {
                    if info.record {
                        Value::record(&info.name, vec![])
                    } else {
                        Value::ctor(&info.name, vec![])
                    }
                } else {
                    let layers = info
                        .fields
                        .iter()
                        .map(|(_, t)| match layer_of(t) {
                            Layer::Var if info.params.iter().any(|p| matches!(&t.kind, TypeKind::Name{name,..} if name.base()==p)) => Layer::Whole,
                            l => l,
                        })
                        .collect();
                    let id = self.add_code(Code::Ctor(info.clone(), layers));
                    self.closure(id, &info.name, vec![])
                }
            }
            Global::Unit(n) => Value::ctor(n, vec![]),
            Global::Builtin { sig } => {
                if !matches!(sig.ty.kind, TypeKind::Fn { .. }) {
                    // A builtin value: `@started`.
                    if name.starts_with('@') {
                        let id = self.add_code(Code::WidgetBuiltin(Rc::from(name)));
                        self.blueprint(id, name, vec![Value::array(vec![]), Value::array(vec![])])
                    } else {
                        crate::builtins::call(self, name, &[], b, loc)?
                    }
                } else {
                    let (layers, nat) = sig_layers(Some(sig));
                    let id = self.add_code(Code::Builtin { name: Rc::from(name), sig: Some(sig), layers, nat });
                    self.closure(id, name, vec![])
                }
            }
            Global::Method(info) => {
                let (layers, _) = sig_layers(info.sig);
                let id = self.add_code(Code::Method(info.clone(), layers));
                self.closure(id, name, vec![])
            }
        };
        self.globals.borrow_mut().insert(key, v.clone());
        Ok(v)
    }

    /// The code of a function: global (memoized through [`Interp::global`]) or local.
    pub fn fn_code(&self, module: ModId, name: &str, sig: Option<&'static Signature>, clauses: Vec<&'static FnClause>, local: bool) -> CodeId {
        let (mut layers, nat) = sig_layers(sig);
        if sig.is_none() {
            if let Some(c) = clauses.first() {
                layers = c.params.iter().map(layer_of_pattern).collect();
            }
        }
        let mark = match clauses.first().map(|c| &c.name) {
            Some(DefName::Name(n)) => n.sigils.first().copied(),
            _ => None,
        };
        let unbounded = matches!(sig.map(|s| &s.ty.kind), Some(TypeKind::Fn { color: Color::Unbounded, .. }));
        self.add_code(Code::Fn(FnCode { name: Rc::from(name), sig, clauses, module, mark, unbounded, layers, nat, local }))
    }

    /// A tag by its qualified name.
    pub fn tag(&self, name: &str) -> Option<&crate::load::TagInfo> {
        self.l.tags.get(name)
    }

    /// The value of a tag written as an expression: a fault kind is its own value; an
    /// element tag with fields is its constructor; with none, its element.
    pub fn tag_value(&self, name: &str, loc: Loc) -> R<Value> {
        let t = self.tag(name).ok_or_else(|| self.fault(loc, format!("no tag {name}")))?;
        if t.fault {
            return Ok(Value::ctor(name, vec![]));
        }
        if t.decl.fields.is_empty() {
            return Ok(Value::element(Element::new(&t.decl, vec![])?));
        }
        let id = self.code_at(t.ast, 0, || Code::Tag(Rc::from(name)));
        Ok(self.closure(id, name, vec![]))
    }

    // ----- application -----

    /// The layers a function value's parameters declare.
    pub fn layers_of(&self, f: &Value) -> Option<(Rc<Code>, Vec<Layer>)> {
        let Value::Closure(c) = f else { return None };
        let id = Interp::code_id_of(c.name())?;
        let code = self.code(id);
        let layers = match &*code {
            Code::Fn(fc) => {
                if fc.mark == Some(Sigil::At) {
                    vec![Layer::Whole; fc.clauses.first().map(|c| c.params.len()).unwrap_or(0).max(fc.layers.len())]
                } else {
                    fc.layers.clone()
                }
            }
            Code::Lambda { layers, .. } => layers.clone(),
            Code::Builtin { name, layers, .. } => {
                if name.starts_with('@') {
                    vec![Layer::Whole; layers.len()]
                } else {
                    layers.clone()
                }
            }
            Code::Ctor(_, layers) | Code::Method(_, layers) => layers.clone(),
            Code::Tag(_) => vec![],
            Code::Op(OpKind::Bin(op)) => {
                use BinOp::*;
                match op {
                    Add | Sub | Mul | Div | Mod | AddQ | SubQ | MulQ => vec![Layer::Concrete, Layer::Concrete],
                    _ => vec![Layer::Var, Layer::Var],
                }
            }
            Code::Op(OpKind::Neg) => vec![Layer::Concrete],
            Code::Op(OpKind::Not) => vec![Layer::Var],
            Code::Field(_) | Code::Index | Code::Wrap(_) | Code::Interp(_) | Code::MapLit { .. } => vec![],
            _ => vec![],
        };
        Some((code, layers))
    }

    /// Apply a function value to arguments, by the lifting table.
    pub fn apply(&self, f: &Value, args: Vec<Value>, b: Bx, loc: Loc) -> R<Value> {
        self.apply_with(f, args, None, b, loc)
    }

    /// Apply, with layers overridden by explicit instantiation (`Name<<$Int>> x`, Type
    /// System ch. 4: "`Twice<<$Int>> $x` asks for the other reading explicitly").
    pub fn apply_with(&self, f: &Value, args: Vec<Value>, over: Option<&[Option<Layer>]>, b: Bx, loc: Loc) -> R<Value> {
        let Value::Closure(c) = f else {
            return Err(match f {
                Value::Widget(_) => self.fault(loc, "a blueprint is applied by binding it with `<-`, never as a function (`@` never lifts)"),
                other => self.fault(loc, format!("{} is not a function", ops::show_plain(other))),
            });
        };
        let Some((_, mut layers)) = self.layers_of(f) else {
            // A closure from outside the interpreter: called as it is.
            return c.call(&args);
        };
        if let Some(over) = over {
            for (i, o) in over.iter().enumerate() {
                if let (Some(o), Some(l)) = (o, layers.get_mut(i)) {
                    *l = *o;
                }
            }
        }
        let mut events = Vec::new();
        let mut dyns = Vec::new();
        for (i, a) in args.iter().enumerate() {
            let layer = layers.get(i).copied().unwrap_or(Layer::Var);
            if !layer.lifts_time() {
                continue;
            }
            match a {
                Value::Event(_) => events.push(i),
                Value::Dynamic(_) => dyns.push(i),
                _ => {}
            }
        }
        if events.is_empty() && dyns.is_empty() {
            let id = Interp::code_id_of(c.name()).unwrap();
            return self.invoke(id, c.captured(), args, b, Some(loc));
        }
        self.lift(f.clone(), args, &events, &dyns, b, loc)
    }

    /// Build the lift node for an application: a sample at one event, a map over
    /// dynamics, or the refusal of two events (Type System ch. 4, the rows).
    pub fn lift(&self, f: Value, args: Vec<Value>, events: &[usize], dyns: &[usize], b: Bx, loc: Loc) -> R<Value> {
        if events.len() >= 2 {
            return Err(self.fault(
                loc,
                "two events in one operation: in a tick where only one fires there is no value; merge them with `<|` to keep one, or use `!MergeAll` to keep both (Type System ch. 4)",
            ));
        }
        let Some(b) = b else {
            return Err(self.fault(
                loc,
                "this application lifts over a dynamic or an event, which makes a node, and there is no widget building here (a lift inside a node's function, or in a `check`)",
            ));
        };
        let mut slots: Vec<usize> = events.to_vec();
        slots.extend_from_slice(dyns);
        let id = self.add_code(Code::Lifted { slots: Rc::new(slots.clone()) });
        let mut plain_args = args.clone();
        for &s in &slots {
            plain_args[s] = Value::Unit;
        }
        let Value::Closure(node_fn) = self.closure(id, "lift", vec![f, Value::array(plain_args)]) else { unreachable!() };
        let dyn_vals: Vec<Value> = dyns.iter().map(|&i| args[i].clone()).collect();
        match events.first() {
            Some(&e) => b.sample(&args[e], &dyn_vals, node_fn),
            None => Ok(b.lift(&dyn_vals, node_fn)),
        }
    }

    /// Run code `id` on plain arguments (or arguments its layers take whole).
    pub fn invoke(&self, id: CodeId, cap: &[Value], args: Vec<Value>, mut b: Bx, loc: Option<Loc>) -> R<Value> {
        let code = self.code(id);
        let loc = loc.unwrap_or(Loc { module: self.l.prelude, span: Span::default() });
        match &*code {
            Code::Fn(fc) => {
                if fc.mark == Some(Sigil::At) {
                    // A widget function applied: a blueprint of its arguments, built where
                    // it is bound (Semantics ch. 4).
                    let wid = self.code_at(fc, 1, || Code::WidgetFn(id));
                    let mut c = vec![cap.first().cloned().unwrap_or(Value::array(vec![])), Value::array(args)];
                    c.extend(cap.iter().skip(2).cloned());
                    return Ok(self.blueprint(wid, &fc.name, c));
                }
                let (args, wrap) = match self.peel(&fc.layers, args) {
                    Peeled::Short(v) => return Ok(v),
                    Peeled::Args(a, w) => (a, w),
                };
                let args = convert_nat(&fc.nat, args);
                let r = self.run_fn(fc, id, cap, args, b, loc)?;
                Ok(wrap.apply(r))
            }
            Code::Lambda { params, body, module, .. } => {
                let (frame, _) = Interp::uncapture(cap);
                let mut sc = Scopes { module: *module, frames: vec![frame, Frame::new()], mark: None, bind_kind: None };
                match params {
                    None => sc.bind(Rc::from("_"), args.into_iter().next().unwrap_or(Value::Unit)),
                    Some(ps) => {
                        if ps.len() != args.len() {
                            return Err(self.fault(loc, format!("this function takes {} arguments and was given {}", ps.len(), args.len())));
                        }
                        let mut binds = Vec::new();
                        for (p, a) in ps.iter().zip(args.iter()) {
                            if !crate::pattern::matches(self, *module, p, a, &mut binds)? {
                                return Err(self.fault(Loc { module: *module, span: p.span }, format!("the lambda's parameter does not match {}", ops::show_plain(a))));
                            }
                        }
                        for (n, v) in binds {
                            sc.bind(n, v);
                        }
                    }
                }
                match body {
                    LambdaBody::Expr(e) => self.eval(e, &mut sc, b),
                    LambdaBody::Block(bl) => self.run_expr_block(bl, &mut sc, b),
                }
            }
            Code::Builtin { name, layers, nat, .. } => {
                if name.starts_with('@') {
                    let wid = self.code_keyed(format!("wb:{name}"), || Code::WidgetBuiltin(name.clone()));
                    return Ok(self.blueprint(wid, name, vec![Value::array(vec![]), Value::array(args)]));
                }
                let (args, wrap) = match self.peel(layers, args) {
                    Peeled::Short(v) => return Ok(v),
                    Peeled::Args(a, w) => (a, w),
                };
                let args = convert_nat(nat, args);
                let r = crate::builtins::call(self, name, &args, b, loc)?;
                Ok(wrap.apply(r))
            }
            Code::Ctor(info, _) => self.construct(info, args, loc),
            Code::Tag(name) => {
                let t = self.tag(name).unwrap();
                let fields: Vec<(&str, Value)> =
                    t.decl.fields.iter().map(|(n, _)| &**n).zip(args).collect();
                self.element(name, fields, loc)
            }
            Code::Method(info, layers) => {
                let (args, wrap) = match self.peel(layers, args) {
                    Peeled::Short(v) => return Ok(v),
                    Peeled::Args(a, w) => (a, w),
                };
                let r = self.dispatch(info, args, b, loc)?;
                Ok(wrap.apply(r))
            }
            Code::Op(op) => {
                let (args, wrap) = match self.peel(&self.layers_of_op(*op), args) {
                    Peeled::Short(v) => return Ok(v),
                    Peeled::Args(a, w) => (a, w),
                };
                let r = match op {
                    OpKind::Bin(op) => self.binop(*op, &args[0], &args[1], b.as_deref_mut(), loc)?,
                    OpKind::Neg => ops::negate(&args[0])?,
                    OpKind::Not => match &args[0] {
                        Value::Bool(x) => Value::Bool(!x),
                        other => return Err(self.fault(loc, format!("`not` needs a Bool, and was given {}", ops::show_plain(other)))),
                    },
                };
                Ok(wrap.apply(r))
            }
            Code::Interp(parts) => {
                let mut out = String::new();
                let mut vals = args.into_iter();
                for p in parts.iter() {
                    match p {
                        Some(s) => out.push_str(s),
                        None => {
                            let v = vals.next().unwrap_or(Value::Unit);
                            out.push_str(&self.display(&v, b.as_deref_mut(), loc)?);
                        }
                    }
                }
                Ok(Value::text(&out))
            }
            Code::Field(name) => self.field(&args[0], name, loc),
            Code::Index => self.index(&args[0], &args[1], loc),
            Code::Wrap(Sigil::Question) => Ok(Value::some(args.into_iter().next().unwrap_or(Value::Unit))),
            Code::Wrap(_) => Ok(Value::ok(args.into_iter().next().unwrap_or(Value::Unit))),
            Code::Lifted { slots } => {
                // `cap = [f, args]`; the node passes the lifted values in slot order.
                let f = cap[0].clone();
                let Value::Array(plain) = &cap[1] else { unreachable!() };
                let mut full: Vec<Value> = plain.iter().cloned().collect();
                for (k, &s) in slots.iter().enumerate() {
                    full[s] = args.get(k).cloned().unwrap_or(Value::Unit);
                }
                self.apply(&f, full, None, loc)
            }
            Code::Arms { expr, module, lifted, filter } => {
                let (mut frame, _) = Interp::uncapture(cap);
                let mut it = args.into_iter();
                let scrut = it.next().unwrap_or(Value::Unit);
                for (n, v) in lifted.iter().zip(it) {
                    frame.vars.insert(n.clone(), v);
                }
                let mut sc = Scopes { module: *module, frames: vec![frame], mark: None, bind_kind: None };
                let v = self.arms_with(expr, scrut, &mut sc, None)?;
                if *filter {
                    Ok(if crate::eval::is_never_marker(&v) { Value::nil() } else { Value::some(v) })
                } else {
                    Ok(v)
                }
            }
            Code::MapLit { keys } => {
                let mut m = im_rc::OrdMap::new();
                for i in 0..*keys {
                    m.insert(args[2 * i].clone(), args[2 * i + 1].clone());
                }
                Ok(Value::Map(m))
            }
            Code::Block { .. } | Code::WidgetFn(_) | Code::WidgetBuiltin(_) | Code::FreeAt { .. } => {
                Err(self.fault(loc, "a blueprint is not a function"))
            }
        }
    }

    fn layers_of_op(&self, op: OpKind) -> Vec<Layer> {
        use BinOp::*;
        match op {
            OpKind::Bin(Add | Sub | Mul | Div | Mod | AddQ | SubQ | MulQ) | OpKind::Neg => vec![Layer::Concrete, Layer::Concrete],
            _ => vec![Layer::Var, Layer::Var],
        }
    }

    /// Peel `?` and `~` at concrete parameters (Type System ch. 4, "`?` and `~` at the
    /// plain layer"): `nil` or the leftmost `Error` short-circuits; otherwise the result
    /// is wrapped once.
    fn peel(&self, layers: &[Layer], mut args: Vec<Value>) -> Peeled {
        let mut wrap = Wrap::default();
        for (i, a) in args.iter_mut().enumerate() {
            if layers.get(i).copied() != Some(Layer::Concrete) {
                continue;
            }
            match a.clone() {
                Value::Opt(None) => {
                    wrap.nil = true;
                }
                Value::Opt(Some(x)) => {
                    wrap.opt = true;
                    *a = (*x).clone();
                }
                Value::Fallible(Err(e)) => {
                    if wrap.err.is_none() {
                        wrap.err = Some(Value::Fallible(Err(e)));
                    }
                }
                Value::Fallible(Ok(x)) => {
                    wrap.ok = true;
                    *a = (*x).clone();
                }
                _ => {}
            }
        }
        if let Some(e) = wrap.err.take() {
            return Peeled::Short(e);
        }
        if wrap.nil {
            return Peeled::Short(Value::nil());
        }
        Peeled::Args(args, wrap)
    }

    /// Run a function's clauses: the first whose patterns match.
    fn run_fn(&self, fc: &FnCode, id: CodeId, cap: &[Value], args: Vec<Value>, b: Bx, loc: Loc) -> R<Value> {
        // A cycle point: a call of an unbounded function from an unbounded function.
        let unbounded = fc.unbounded;
        if unbounded {
            let point = UNBOUNDED.with(|u| {
                if u.depth.get() > 0 {
                    let p = u.points.get() + 1;
                    u.points.set(p);
                    matches!(u.budget.get(), Some(budget) if p > budget)
                } else {
                    false
                }
            });
            if point {
                return Err(Fault::new(CYCLE_BUDGET, "the cycle budget ran out"));
            }
            UNBOUNDED.with(|u| u.depth.set(u.depth.get() + 1));
        }
        let r = self.run_clauses(fc, id, cap, args, b, loc);
        if unbounded {
            UNBOUNDED.with(|u| u.depth.set(u.depth.get() - 1));
        }
        r
    }

    fn run_clauses(&self, fc: &FnCode, id: CodeId, cap: &[Value], args: Vec<Value>, b: Bx, loc: Loc) -> R<Value> {
        let (frame, _) = Interp::uncapture(cap);
        for clause in &fc.clauses {
            if clause.params.len() != args.len() {
                continue;
            }
            let mut binds = Vec::new();
            let mut ok = true;
            for (p, a) in clause.params.iter().zip(args.iter()) {
                if !crate::pattern::matches(self, fc.module, p, a, &mut binds)? {
                    ok = false;
                    break;
                }
            }
            if !ok {
                continue;
            }
            let mut f0 = Frame::new();
            f0.vars = frame.vars.clone();
            if fc.local {
                // A local function sees itself by name.
                f0.vars.insert(fc.name.clone(), self.closure(id, &fc.name, cap.to_vec()));
            }
            let mut params = Frame::new();
            for (n, v) in binds {
                params.vars.insert(n, v);
            }
            let mut sc = Scopes { module: fc.module, frames: vec![f0, params], mark: fc.mark, bind_kind: None };
            return match &clause.body {
                FnBody::Expr(e) => self.eval(e, &mut sc, b),
                FnBody::Block(bl) => match fc.mark {
                    Some(Sigil::Question | Sigil::Tilde) => self.run_given(bl, &mut sc, b),
                    _ => self.run_expr_block(bl, &mut sc, b),
                },
            };
        }
        let shown: Vec<String> = args.iter().map(ops::show_plain).collect();
        Err(self.fault(loc, format!("no clause of {} matches {}", fc.name, shown.join(", "))))
    }

    /// Build a blueprint over code `id`.
    pub fn build(&self, id: CodeId, b: &mut Build, cap: &[Value], args: &[Value]) -> R<Value> {
        let code = self.code(id);
        match &*code {
            Code::Block { block, kind, module, set_row } => {
                let (frame, _) = Interp::uncapture(cap);
                let mut sc = Scopes { module: *module, frames: vec![frame, Frame::new()], mark: None, bind_kind: None };
                if *set_row {
                    let row = self.l.modules[*module].row.clone();
                    if row != prism_rt::Row::Any {
                        b.set_emit(row);
                    }
                }
                match kind {
                    BlockKind::Widget | BlockKind::WidgetExpr => {}
                    BlockKind::Rebuild => sc.bind(Rc::from("_"), args.first().cloned().unwrap_or(Value::Unit)),
                    BlockKind::For(binders, dynamic) => {
                        self.bind_for_row(&mut sc, *binders, *dynamic, args, *module)?;
                    }
                }
                self.run_widget_block(block, &mut sc, b)
            }
            Code::WidgetFn(fid) => {
                let fcode = self.code(*fid);
                let Code::Fn(fc) = &*fcode else { unreachable!() };
                let (_, extra) = Interp::uncapture(cap);
                let mut env = vec![cap[0].clone(), Value::array(vec![])];
                env.extend(cap.iter().skip(2).cloned());
                let row = self.l.modules[fc.module].row.clone();
                if !fc.local && row != prism_rt::Row::Any {
                    b.set_emit(row);
                }
                let loc = Loc { module: fc.module, span: fc.clauses[0].span };
                self.build_widget_fn(fc, *fid, &env, extra, b, loc)
            }
            Code::WidgetBuiltin(name) => {
                let (_, extra) = Interp::uncapture(cap);
                let loc = Loc { module: self.l.prelude, span: Span::default() };
                crate::builtins::call(self, name, &extra, Some(b), loc)
            }
            Code::FreeAt { expr, module } => {
                let (frame, _) = Interp::uncapture(cap);
                let mut sc = Scopes { module: *module, frames: vec![frame], mark: None, bind_kind: None };
                self.eval(expr, &mut sc, Some(b))
            }
            _ => Err(Fault::runtime("this code is not a blueprint")),
        }
    }

    /// Build a widget function's clause for its arguments.
    fn build_widget_fn(&self, fc: &FnCode, id: CodeId, cap: &[Value], args: Vec<Value>, b: &mut Build, loc: Loc) -> R<Value> {
        let (frame, _) = Interp::uncapture(cap);
        for clause in &fc.clauses {
            if clause.params.len() != args.len() {
                continue;
            }
            let mut binds = Vec::new();
            let mut ok = true;
            for (p, a) in clause.params.iter().zip(args.iter()) {
                if !crate::pattern::matches(self, fc.module, p, a, &mut binds)? {
                    ok = false;
                    break;
                }
            }
            if !ok {
                continue;
            }
            let mut f0 = Frame::new();
            f0.vars = frame.vars.clone();
            if fc.local {
                f0.vars.insert(fc.name.clone(), self.closure(id, &fc.name, cap.to_vec()));
            }
            let mut params = Frame::new();
            for (n, v) in binds {
                params.vars.insert(n, v);
            }
            let mut sc = Scopes { module: fc.module, frames: vec![f0, params], mark: None, bind_kind: None };
            return match &clause.body {
                FnBody::Block(bl) => self.run_widget_block(bl, &mut sc, b),
                FnBody::Expr(e) => {
                    // A one-line widget body: built as the block's only statement.
                    let v = self.eval(e, &mut sc, Some(&mut *b))?;
                    self.bind_build(v, b, Loc { module: fc.module, span: e.span })
                }
            };
        }
        let shown: Vec<String> = args.iter().map(ops::show_plain).collect();
        Err(self.fault(loc, format!("no clause of {} matches {}", fc.name, shown.join(", "))))
    }

    /// What a statement does with the value on its right: an element is emitted, a
    /// blueprint is built (transparently, Semantics ch. 8), anything else is the value.
    pub fn bind_build(&self, v: Value, b: &mut Build, loc: Loc) -> R<Value> {
        match v {
            Value::Element(e) => {
                self.check_identity(&e, loc)?;
                b.emit((*e).clone())
            }
            Value::Widget(bp) => b.call(&bp, &[]),
            other => Ok(other),
        }
    }

    /// A dynamic where an identity field goes is refused, naming `rebuild` (Semantics
    /// ch. 8, "Fields: what each sigil promises"; Type System ch. 4, "`@` never lifts").
    fn check_identity(&self, e: &Element, loc: Loc) -> R<()> {
        let Some(decl) = &e.decl else { return Ok(()) };
        for (n, v) in &e.fields {
            if decl.field_kind(n) == Some(prism_rt::FieldKind::Identity) && !v.is_plain() {
                return Err(self.fault(
                    loc,
                    format!(
                        "a dynamic in the identity field `{n}` of {}: a new {n} is a new place; for a new element per value, write\n      rebuild $x:\n          {}({n}=_, ..)",
                        e.tag, e.tag
                    ),
                ));
            }
        }
        Ok(())
    }

    /// Construct a record or a choice. A record's arguments come in field order.
    pub fn construct(&self, info: &CtorInfo, args: Vec<Value>, loc: Loc) -> R<Value> {
        if args.len() != info.fields.len() {
            return Err(self.fault(loc, format!("{} takes {} arguments and was given {}", info.name, info.fields.len(), args.len())));
        }
        let args: Vec<Value> =
            args.into_iter().zip(info.fields.iter()).map(|(a, (_, t))| if is_nat_type(t) { to_nat(a) } else { a }).collect();
        if info.record {
            let fields = info.fields.iter().map(|(n, _)| &**n).zip(args).collect();
            let v = Value::record(&info.name, fields);
            // `Error(..)` is the failed `~T` (Semantics ch. 10), as the transcript reads it back.
            if &*info.name == "Error" {
                if let Value::Record(r) = v {
                    return Ok(Value::Fallible(Err(Rc::new(Value::Record(r)))));
                }
            }
            Ok(v)
        } else {
            Ok(Value::ctor(&info.name, args))
        }
    }

    /// Construct an element of a declared tag, fields by name (Semantics ch. 8).
    pub fn element(&self, tag: &str, fields: Vec<(&str, Value)>, loc: Loc) -> R<Value> {
        let t = self.tag(tag).ok_or_else(|| self.fault(loc, format!("no tag {tag}")))?;
        if t.fault {
            return Err(self.fault(loc, format!("{tag} is a fault kind, not an element")));
        }
        Element::new(&t.decl, fields).map(Value::element).map_err(|f| self.fault(loc, f.message))
    }

    /// A trait method, dispatched on the runtime type of its dispatch argument (Type
    /// System ch. 3): the type's `impl`, else the trait's default, else the structural
    /// answer of a derivable trait.
    fn dispatch(&self, info: &MethodInfo, args: Vec<Value>, b: Bx, loc: Loc) -> R<Value> {
        let recv = args.get(info.dispatch).cloned().unwrap_or(Value::Unit);
        let recv = match &recv {
            // An array or optional of the trait's type dispatches on its first element.
            Value::Array(a) if !a.is_empty() && !self.has_impl(&info.trait_name, "[]") => a[0].clone(),
            _ => recv,
        };
        let ty = ops::type_name(&self.l, &recv);
        if let Some(imp) = self.l.impls.get(&(info.trait_name.to_string(), ty.to_string())) {
            if let Some(clauses) = imp.methods.get(&info.name) {
                let id = self.code_at(&clauses[0].span, 3, || {
                    let (layers, nat) = sig_layers(info.sig);
                    Code::Fn(FnCode {
                        name: info.name.clone(),
                        sig: info.sig,
                        clauses: clauses.clone(),
                        module: imp.module,
                        mark: None,
                        unbounded: false,
                        layers,
                        nat,
                        local: false,
                    })
                });
                return self.invoke(id, &[], args, b, Some(loc));
            }
        }
        if !info.default.is_empty() {
            let id = self.code_at(&info.default[0].span, 4, || {
                let (layers, nat) = sig_layers(info.sig);
                Code::Fn(FnCode {
                    name: info.name.clone(),
                    sig: info.sig,
                    clauses: info.default.clone(),
                    module: info.module,
                    mark: None,
                    unbounded: false,
                    layers,
                    nat,
                    local: false,
                })
            });
            return self.invoke(id, &[], args, b, Some(loc));
        }
        if info.module == self.l.prelude {
            return crate::builtins::structural(self, &info.name, &args, loc);
        }
        Err(self.fault(loc, format!("^{} has no impl for {ty}, and {} has no default", info.trait_name, info.name)))
    }

    pub fn has_impl(&self, trait_name: &str, ty: &str) -> bool {
        self.l.impls.contains_key(&(trait_name.to_string(), ty.to_string()))
    }

    /// A binary operator: a user `impl` of the operator's trait method first (Type System
    /// ch. 3: "`==` is a method of `^Eq`"), then the built-in meaning.
    pub fn binop(&self, op: BinOp, a: &Value, b2: &Value, b: Bx, loc: Loc) -> R<Value> {
        let ty = ops::type_name(&self.l, a);
        let tr = match op {
            BinOp::Eq | BinOp::NotEq => "Eq",
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => "Ord",
            _ => "Num",
        };
        if let Some(imp) = self.l.impls.get(&(tr.to_string(), ty.to_string())) {
            if let Some(clauses) = imp.methods.get(op.as_str()) {
                let id = self.code_at(&clauses[0].span, 5, || {
                    Code::Fn(FnCode {
                        name: Rc::from(op.as_str()),
                        sig: None,
                        clauses: clauses.clone(),
                        module: imp.module,
                        mark: None,
                        unbounded: false,
                        layers: vec![Layer::Var, Layer::Var],
                        nat: vec![],
                        local: false,
                    })
                });
                return self.invoke(id, &[], vec![a.clone(), b2.clone()], b, Some(loc));
            }
            if tr == "Ord" {
                if let Some(clauses) = imp.methods.get("Compare") {
                    let _ = clauses;
                    let o = self.compare_by_trait(a, b2, b, loc)?;
                    return Ok(Value::Bool(match op {
                        BinOp::Lt => o == std::cmp::Ordering::Less,
                        BinOp::Le => o != std::cmp::Ordering::Greater,
                        BinOp::Gt => o == std::cmp::Ordering::Greater,
                        _ => o != std::cmp::Ordering::Less,
                    }));
                }
            }
        }
        ops::binop(&self.l, op, a, b2).map_err(|f| self.fault(loc, f.message))
    }

    fn compare_by_trait(&self, a: &Value, b2: &Value, b: Bx, loc: Loc) -> R<std::cmp::Ordering> {
        let Some((owner, g)) = self.resolve_bare(self.l.prelude, "Compare") else {
            return Ok(ops::compare(&self.l, a, b2));
        };
        let f = self.global(owner, "Compare", &g, None, loc)?;
        let r = self.apply(&f, vec![a.clone(), b2.clone()], b, loc)?;
        Ok(match &r {
            Value::Choice(c) if &*c.ctor == "Less" => std::cmp::Ordering::Less,
            Value::Choice(c) if &*c.ctor == "Greater" => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        })
    }

    /// How a placeholder's value enters a text: a text as it is, a character as itself,
    /// anything else by `Show` (Type System ch. 4, interpolation).
    pub fn display(&self, v: &Value, b: Bx, loc: Loc) -> R<String> {
        Ok(match v {
            Value::Text(t) => t.to_string(),
            Value::Char(c) => c.to_string(),
            Value::Nat(n) | Value::Int(n) => n.to_string(),
            Value::U8(n) => n.to_string(),
            Value::Bool(x) => if *x { "True".into() } else { "False".into() },
            other => self.show(other, b, loc)?,
        })
    }

    /// `Show`: the type's `impl ^Show` if it has one, else the transcript's spelling.
    pub fn show(&self, v: &Value, b: Bx, loc: Loc) -> R<String> {
        let ty = ops::type_name(&self.l, v);
        if self.has_impl("Show", &ty) {
            if let Some((owner, g)) = self.resolve_bare(self.l.prelude, "Show") {
                let f = self.global(owner, "Show", &g, None, loc)?;
                if let Value::Text(t) = self.apply(&f, vec![v.clone()], b, loc)? {
                    return Ok(t.to_string());
                }
            }
        }
        Ok(ops::show_plain(v))
    }

    /// Field access, with `?` and `~` peeled at the plain layer (Type System ch. 4,
    /// "Field access is a plain function"). A failed `~` passes through, except that an
    /// `Error`'s own fields are read: a value of type `Error` and a failed `~T` are one
    /// value in compiler zero.
    pub fn field(&self, v: &Value, name: &str, loc: Loc) -> R<Value> {
        match v {
            Value::Record(r) => r
                .fields
                .iter()
                .find(|(n, _)| &**n == name)
                .map(|(_, v)| v.clone())
                .ok_or_else(|| self.fault(loc, format!("{} has no field {name}", r.name))),
            Value::Element(e) => e.field(name).cloned().ok_or_else(|| self.fault(loc, format!("{} has no field {name}", e.tag))),
            Value::Opt(None) => Ok(Value::nil()),
            Value::Opt(Some(x)) => Ok(Value::some(self.field(x, name, loc)?)),
            Value::Fallible(Ok(x)) => Ok(Value::ok(self.field(x, name, loc)?)),
            Value::Fallible(Err(e)) => match self.field(e, name, loc) {
                Ok(f) => Ok(f),
                Err(_) => Ok(v.clone()),
            },
            other => Err(self.fault(loc, format!("{} has no field {name}", ops::show_plain(other)))),
        }
    }

    /// Indexing: `?v` (Semantics ch. 10, "Indexing returns `?V`").
    pub fn index(&self, v: &Value, k: &Value, loc: Loc) -> R<Value> {
        match v {
            Value::Map(m) => Ok(m.get(k).cloned().map(Value::some).unwrap_or(Value::nil())),
            Value::Array(a) => Ok(ops::index_of(k).and_then(|i| a.get(i).cloned()).map(Value::some).unwrap_or(Value::nil())),
            Value::Text(t) => Ok(ops::index_of(k).and_then(|i| t.char_at(i)).map(|c| Value::some(Value::Char(c))).unwrap_or(Value::nil())),
            Value::Record(r) if &*r.name == "Table" => crate::builtins::table_get(v, k),
            Value::Opt(None) => Ok(Value::nil()),
            Value::Opt(Some(x)) => self.index(x, k, loc),
            other => Err(self.fault(loc, format!("{} cannot be indexed", ops::show_plain(other)))),
        }
    }

    fn bind_for_row(&self, sc: &mut Scopes, binders: Option<&'static ForBinders>, dynamic: bool, args: &[Value], module: ModId) -> R<()> {
        let key = args.first().cloned().unwrap_or(Value::Unit);
        let value = args.get(1).cloned().unwrap_or(Value::Unit);
        let ix = args.get(2).cloned().unwrap_or(Value::nat(0));
        // `ix` is this loop's, `jx` the enclosing loop's, `kx` the one around that.
        let outer_ix = sc.frames[0].vars.get("ix").cloned();
        let outer_jx = sc.frames[0].vars.get("jx").cloned();
        if let Some(j) = outer_ix {
            sc.bind(Rc::from("jx"), j);
        }
        if let Some(k) = outer_jx {
            sc.bind(Rc::from("kx"), k);
        }
        sc.bind(Rc::from("ix"), ix);
        sc.bind(Rc::from("key"), key.clone());
        let mut binds = Vec::new();
        let loc = Loc { module, span: Span::default() };
        match binders {
            None => {
                sc.bind(Rc::from(if dynamic { "$_" } else { "_" }), value);
            }
            Some(fb) => match &fb.second {
                Some(second) => {
                    if !crate::pattern::matches(self, module, &fb.first, &key, &mut binds)?
                        || !crate::pattern::matches(self, module, second, &value, &mut binds)?
                    {
                        return Err(self.fault(loc, "a `for` binder does not match its row"));
                    }
                }
                None => {
                    if !crate::pattern::matches(self, module, &fb.first, &value, &mut binds)? {
                        return Err(self.fault(Loc { module, span: fb.first.span }, "a `for` binder does not match its row"));
                    }
                }
            },
        }
        for (n, v) in binds {
            sc.bind(n, v);
        }
        Ok(())
    }
}

/// What peeling left: a short-circuit value, or the arguments and how to wrap the result.
enum Peeled {
    Short(Value),
    Args(Vec<Value>, Wrap),
}

#[derive(Default)]
struct Wrap {
    nil: bool,
    opt: bool,
    ok: bool,
    err: Option<Value>,
}

impl Wrap {
    fn apply(&self, r: Value) -> Value {
        let mut r = r;
        if self.opt && !matches!(r, Value::Opt(_)) {
            r = Value::some(r);
        }
        if self.ok && !matches!(r, Value::Fallible(_)) {
            r = Value::ok(r);
        }
        r
    }
}

/// A signature's parameter layers and which parameters are `Nat`.
pub fn sig_layers(sig: Option<&Signature>) -> (Vec<Layer>, Vec<bool>) {
    match sig.map(|s| &s.ty.kind) {
        Some(TypeKind::Fn { params, .. }) => (params.iter().map(layer_of).collect(), params.iter().map(is_nat_type).collect()),
        _ => (vec![], vec![]),
    }
}

fn convert_nat(nat: &[bool], args: Vec<Value>) -> Vec<Value> {
    args.into_iter().enumerate().map(|(i, a)| if nat.get(i).copied().unwrap_or(false) { to_nat(a) } else { a }).collect()
}
