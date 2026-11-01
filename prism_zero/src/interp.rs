//! The interpreter's core: code objects, closures and blueprints over `prism_rt`, scopes,
//! globals, and the program's tables handed to `prism_dyn`.
//!
//! **Functions are `prism_dyn` function values.** Every function value of a Prism
//! program is a [`prism_rt::Closure`] with a `prism_dyn` meta (and every blueprint a
//! [`prism_rt::Blueprint`]), so the runtime can call it from a node without knowing it
//! came from an interpreter, and compiled code and the interpreter apply it the same way.
//! Its behaviour is a [`Code`] in the interpreter's table, the meta's `user` number; its
//! environment travels in the closure's `captured` values, which the collector traces
//! (decisions.md D5: "Captured values are explicit so the collector traces them"). The
//! layout of `captured` for code with an environment is `[names, extra, value...]`: an
//! array of the captured names as texts, an array of extra values (a widget function's
//! arguments), then one value per name.
//!
//! **The lifting table at each application** is `prism_dyn::func` (Type System ch. 4,
//! "Where the table looks"; D3, D16): the interpreter supplies each code's layers, read
//! off its declared types ([`layer_of`]), and the body; `prism_dyn` compares the
//! arguments' runtime layers, builds the lift node, peels `?` and `~`.
//!
//! **Unbounded functions** (Semantics ch. 7; D6) count cycle points through
//! `prism_dyn::unbounded`, without saving continuations.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

use prism_dyn::func::FnMeta;
use prism_dyn::values::MethodMeta;
use prism_rt::{Blueprint, Build, Fault, Value};
use prism_syntax::Span;
use prism_syntax::ast::*;

pub use prism_dyn::func::{to_nat, Layer};
pub use prism_dyn::unbounded::{CYCLE_BUDGET, UNBOUNDED};

use crate::load::{CtorInfo, Global, Loaded, MethodInfo, ModId};
use crate::ops::{self, R};

/// An index into the code table.
pub type CodeId = usize;

/// The build a statement runs in, if any: present while a blueprint builds, absent in a
/// node's function at propagation time and in a `check` line.
pub type Bx<'x, 'a> = prism_dyn::Bx<'x, 'a>;

/// Where an error is reported: a module and a span in it.
#[derive(Clone, Copy, Debug)]
pub struct Loc {
    pub module: ModId,
    pub span: Span,
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

/// A code object: what a closure or blueprint of the program does. Constructors, tags,
/// operators, field access, indexing, interpolation and lifts are `prism_dyn`'s own.
pub enum Code {
    Fn(FnCode),
    Lambda { params: Option<&'static [Pattern]>, body: &'static LambdaBody, module: ModId, layers: Vec<Layer> },
    Builtin { name: Rc<str>, sig: Option<&'static Signature>, layers: Vec<Layer>, nat: Vec<bool> },
    Method(Rc<MethodMeta>, Vec<Layer>),
    /// A blueprint over a block, with its module and the row its module's `emit` sets.
    Block { block: &'static Block, kind: BlockKind, module: ModId, set_row: bool },
    /// A widget function applied: a blueprint of its arguments.
    WidgetFn(CodeId),
    /// A widget builtin applied: a blueprint of its arguments.
    WidgetBuiltin(Rc<str>),
    /// `@ v`: a blueprint whose result is `v`, evaluated when it builds.
    FreeAt { expr: &'static Expr, module: ModId },
    /// A `match` or `if` over a dynamic or an event with plain arms, as a lifted function
    /// of its scrutinee and the values of the names its arms read (Type System ch. 4,
    /// "`match` and `if` as lifting"; `prism_dyn::widget::lift_branch`).
    Arms { expr: &'static Expr, module: ModId, names: Rc<Vec<Rc<str>>> },
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

/// The interpreter.
pub struct Interp {
    pub l: Loaded,
    codes: RefCell<Vec<Rc<Code>>>,
    by_addr: RefCell<HashMap<(usize, u8), CodeId>>,
    globals: RefCell<HashMap<(ModId, Rc<str>), Value>>,
    in_progress: RefCell<HashSet<(ModId, Rc<str>)>>,
    free: RefCell<HashMap<usize, Rc<Vec<Rc<str>>>>>,
    metas: RefCell<HashMap<CodeId, usize>>,
    me: Weak<Interp>,
}

impl Interp {
    pub fn new(l: Loaded) -> Rc<Interp> {
        let interp = Rc::new_cyclic(|me| Interp {
            l,
            codes: RefCell::new(Vec::new()),
            by_addr: RefCell::new(HashMap::new()),
            globals: RefCell::new(HashMap::new()),
            in_progress: RefCell::new(HashSet::new()),
            free: RefCell::new(HashMap::new()),
            metas: RefCell::new(HashMap::new()),
            me: me.clone(),
        });
        interp.install();
        interp
    }

    /// Hand the program's tables to `prism_dyn`: constructors, tags, and every `impl`'s
    /// methods as function values.
    fn install(&self) {
        use prism_dyn::tables;
        tables::reset();
        for c in self.l.ctors.values() {
            tables::add_ctor(ctor_meta(c));
        }
        for t in self.l.tags.values() {
            tables::add_tag(t.decl.clone(), t.fault);
        }
        let mut infos: HashMap<(String, String), Rc<MethodInfo>> = HashMap::new();
        for m in &self.l.modules {
            for g in m.decls.values() {
                if let Global::Method(info) = g {
                    infos.insert((info.trait_name.to_string(), info.name.to_string()), info.clone());
                }
            }
        }
        for ((tr, ty), imp) in &self.l.impls {
            tables::add_impl(tr, ty);
            for (name, clauses) in &imp.methods {
                let info = infos.get(&(tr.clone(), name.to_string()));
                let sig = info.and_then(|i| i.sig);
                let (layers, nat) = if sig.is_some() { sig_layers(sig) } else { (vec![Layer::Var, Layer::Var], vec![]) };
                let id = self.add_code(Code::Fn(FnCode {
                    name: name.clone(),
                    sig,
                    clauses: clauses.clone(),
                    module: imp.module,
                    mark: None,
                    unbounded: false,
                    layers,
                    nat,
                    local: false,
                }));
                tables::add_impl_method(tr, ty, name, self.closure(id, name, vec![]));
            }
        }
    }

    pub fn weak(&self) -> Weak<Interp> {
        self.me.clone()
    }

    // ----- errors -----

    /// An error at a place in the program.
    pub fn fault(&self, loc: Loc, msg: impl Into<String>) -> Fault {
        if loc.module == NOWHERE {
            // No place known here: the application that reached it names its own.
            return Fault::new("prism_zero.Eval", msg.into());
        }
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

    /// The code of a function value the interpreter made.
    pub fn code_of_value(&self, f: &Value) -> Option<Rc<Code>> {
        let m = prism_dyn::func::meta_of_value(f)?;
        if m.user == usize::MAX {
            return None;
        }
        self.codes.borrow().get(m.user).cloned()
    }

    /// The `prism_dyn` meta of code `id`, registered on first use: the layers its
    /// parameters declare, and a body that runs the code.
    fn meta_id(&self, id: CodeId) -> usize {
        if let Some(m) = self.metas.borrow().get(&id) {
            return *m;
        }
        let code = self.code(id);
        let (layers, nat, peel, name): (Vec<Layer>, Vec<bool>, bool, Rc<str>) = match &*code {
            Code::Fn(fc) => {
                if fc.mark == Some(Sigil::At) {
                    (vec![Layer::Whole; fc.clauses.first().map(|c| c.params.len()).unwrap_or(0).max(fc.layers.len())], vec![], false, fc.name.clone())
                } else {
                    (fc.layers.clone(), fc.nat.clone(), true, fc.name.clone())
                }
            }
            Code::Lambda { layers, .. } => (layers.clone(), vec![], false, Rc::from("lambda")),
            Code::Builtin { name, layers, nat, .. } => {
                if name.starts_with('@') {
                    (vec![Layer::Whole; layers.len()], vec![], false, name.clone())
                } else {
                    (layers.clone(), nat.clone(), true, name.clone())
                }
            }
            Code::Method(m, layers) => (layers.clone(), vec![], true, m.name.clone()),
            _ => (vec![], vec![], false, Rc::from("code")),
        };
        let me = self.weak();
        let mid = prism_dyn::func::register(FnMeta {
            name,
            layers,
            nat,
            peel,
            ctor: None,
            user: id,
            body: Rc::new(move |cap, args, b| {
                let interp = me.upgrade().ok_or_else(|| Fault::runtime("the interpreter is gone"))?;
                interp.invoke(id, cap, args, b.as_deref_mut(), None)
            }),
        });
        self.metas.borrow_mut().insert(id, mid);
        mid
    }

    /// A function value over code `id`.
    pub fn closure(&self, id: CodeId, display: &str, captured: Vec<Value>) -> Value {
        prism_dyn::func::closure(self.meta_id(id), display, captured)
    }

    /// A blueprint over code `id`.
    pub fn blueprint(&self, id: CodeId, display: &str, captured: Vec<Value>) -> Value {
        let me = self.weak();
        Value::Widget(Blueprint::new(&format!("{display}#{id}"), captured, move |b, cap, args| {
            let interp = me.upgrade().ok_or_else(|| Fault::runtime("the interpreter is gone"))?;
            interp.build(id, b, cap, args)
        }))
    }

    /// The operator `op` as a function value, for lifting (`prism_dyn`'s).
    pub fn op_value(&self, op: OpKind) -> Value {
        match op {
            OpKind::Bin(b) => prism_dyn::values::op_value(ops::op_of(b)),
            OpKind::Neg => prism_dyn::values::unop_value(prism_dyn::values::UnOp::Neg),
            OpKind::Not => prism_dyn::values::unop_value(prism_dyn::values::UnOp::Not),
        }
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
            Global::Ctor(info) => prism_dyn::values::ctor_fn(Rc::new(ctor_meta(info))),
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
                let default = if info.default.is_empty() {
                    None
                } else {
                    let (dl, dn) = sig_layers(info.sig);
                    let id = self.add_code(Code::Fn(FnCode {
                        name: info.name.clone(),
                        sig: info.sig,
                        clauses: info.default.clone(),
                        module: info.module,
                        mark: None,
                        unbounded: false,
                        layers: dl,
                        nat: dn,
                        local: false,
                    }));
                    Some(self.closure(id, &info.name, vec![]))
                };
                let meta = MethodMeta {
                    trait_name: info.trait_name.clone(),
                    name: info.name.clone(),
                    dispatch: info.dispatch,
                    default,
                    prelude: info.module == self.l.prelude,
                };
                let id = self.add_code(Code::Method(Rc::new(meta), layers));
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
        prism_dyn::values::tag_value(name).map_err(|f| self.wrap_fault(loc, f))
    }

    // ----- application -----

    /// Apply a function value to arguments, by the lifting table.
    pub fn apply(&self, f: &Value, args: Vec<Value>, b: Bx, loc: Loc) -> R<Value> {
        self.apply_with(f, args, None, b, loc)
    }

    /// Apply, with layers overridden by explicit instantiation (`Name<<$Int>> x`, Type
    /// System ch. 4: "`Twice<<$Int>> $x` asks for the other reading explicitly").
    pub fn apply_with(&self, f: &Value, args: Vec<Value>, over: Option<&[Option<Layer>]>, mut b: Bx, loc: Loc) -> R<Value> {
        prism_dyn::apply_with(&mut b, f, args, over).map_err(|e| self.wrap_fault(loc, e))
    }

    /// Run code `id` on arguments its layers take (`prism_dyn` has lifted and peeled).
    pub fn invoke(&self, id: CodeId, cap: &[Value], args: Vec<Value>, b: Bx, loc: Option<Loc>) -> R<Value> {
        let code = self.code(id);
        let loc = loc.unwrap_or(Loc { module: NOWHERE, span: Span::default() });
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
                self.run_fn(fc, id, cap, args, b, loc)
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
            Code::Builtin { name, .. } => {
                if name.starts_with('@') {
                    let wid = self.code_keyed(format!("wb:{name}"), || Code::WidgetBuiltin(name.clone()));
                    return Ok(self.blueprint(wid, name, vec![Value::array(vec![]), Value::array(args)]));
                }
                crate::builtins::call(self, name, &args, b, loc)
            }
            Code::Method(m, _) => {
                let mut b = b;
                prism_dyn::values::dispatch(m, args, &mut b).map_err(|f| self.wrap_fault(loc, f))
            }
            Code::Arms { expr, module, names } => {
                let (mut frame, _) = Interp::uncapture(cap);
                let mut it = args.into_iter();
                let scrut = it.next().unwrap_or(Value::Unit);
                for (n, v) in names.iter().zip(it) {
                    frame.vars.insert(n.clone(), v);
                }
                let mut sc = Scopes { module: *module, frames: vec![frame], mark: None, bind_kind: None };
                self.arms_with(expr, scrut, &mut sc, None)
            }
            Code::Block { .. } | Code::WidgetFn(_) | Code::WidgetBuiltin(_) | Code::FreeAt { .. } => {
                Err(self.fault(loc, "a blueprint is not a function"))
            }
        }
    }

    /// Run a function's clauses: the first whose patterns match.
    fn run_fn(&self, fc: &FnCode, id: CodeId, cap: &[Value], args: Vec<Value>, b: Bx, loc: Loc) -> R<Value> {
        // A cycle point: a call of an unbounded function from an unbounded function.
        if fc.unbounded {
            return prism_dyn::unbounded::unbounded(|| self.run_clauses(fc, id, cap, args, b, loc));
        }
        self.run_clauses(fc, id, cap, args, b, loc)
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
        prism_dyn::widget::bind_build(&mut Some(b), v).map_err(|f| self.wrap_fault(loc, f))
    }

    /// Construct a record or a choice. A record's arguments come in field order.
    pub fn construct(&self, info: &CtorInfo, args: Vec<Value>, loc: Loc) -> R<Value> {
        prism_dyn::values::construct(&ctor_meta(info), args).map_err(|f| self.wrap_fault(loc, f))
    }

    /// Construct an element of a declared tag, fields by name (Semantics ch. 8).
    pub fn element(&self, tag: &str, fields: Vec<(&str, Value)>, loc: Loc) -> R<Value> {
        prism_dyn::values::element(tag, fields).map_err(|f| self.wrap_fault(loc, f))
    }

    pub fn has_impl(&self, trait_name: &str, ty: &str) -> bool {
        prism_dyn::tables::has_impl(trait_name, ty)
    }

    /// A binary operator: a user `impl` of the operator's trait method first (Type System
    /// ch. 3: "`==` is a method of `^Eq`"), then the built-in meaning.
    pub fn binop(&self, op: BinOp, a: &Value, b2: &Value, mut b: Bx, loc: Loc) -> R<Value> {
        prism_dyn::values::binop_plain(&mut b, ops::op_of(op), a, b2).map_err(|f| self.wrap_fault(loc, f))
    }

    /// How a placeholder's value enters a text: a text as it is, a character as itself,
    /// anything else by `Show` (Type System ch. 4, interpolation).
    pub fn display(&self, v: &Value, mut b: Bx, loc: Loc) -> R<String> {
        prism_dyn::values::display(v, &mut b).map_err(|f| self.wrap_fault(loc, f))
    }

    /// `Show`: the type's `impl ^Show` if it has one, else the transcript's spelling.
    pub fn show(&self, v: &Value, mut b: Bx, loc: Loc) -> R<String> {
        prism_dyn::values::show(v, &mut b).map_err(|f| self.wrap_fault(loc, f))
    }

    /// Field access, with `?` and `~` peeled at the plain layer (`prism_dyn::values::field`).
    pub fn field(&self, v: &Value, name: &str, loc: Loc) -> R<Value> {
        prism_dyn::values::field(v, name).map_err(|f| self.wrap_fault(loc, f))
    }

    /// Indexing: `?v` (Semantics ch. 10, "Indexing returns `?V`").
    pub fn index(&self, v: &Value, k: &Value, loc: Loc) -> R<Value> {
        prism_dyn::values::index(v, k).map_err(|f| self.wrap_fault(loc, f))
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

/// A constructor's `prism_dyn` entry: a type-parameter field takes its argument whole
/// (Type System ch. 4, "Where the table looks").
fn ctor_meta(c: &CtorInfo) -> prism_dyn::tables::CtorMeta {
    let layers = c
        .fields
        .iter()
        .map(|(_, t)| match layer_of(t) {
            Layer::Var if c.params.iter().any(|p| matches!(&t.kind, TypeKind::Name{name,..} if name.base()==p)) => Layer::Whole,
            l => l,
        })
        .collect();
    prism_dyn::tables::CtorMeta {
        name: c.name.clone(),
        type_name: c.type_name.clone(),
        record: c.record,
        fields: c.fields.iter().map(|(n, _)| n.clone()).collect(),
        nat: c.fields.iter().map(|(_, t)| is_nat_type(t)).collect(),
        layers,
        index: c.index,
    }
}

/// A signature's parameter layers and which parameters are `Nat`.
pub fn sig_layers(sig: Option<&Signature>) -> (Vec<Layer>, Vec<bool>) {
    match sig.map(|s| &s.ty.kind) {
        Some(TypeKind::Fn { params, .. }) => (params.iter().map(layer_of).collect(), params.iter().map(is_nat_type).collect()),
        _ => (vec![], vec![]),
    }
}

/// The module of a place no one knows: a fault there names no place, and the
/// application that reached it names its own.
pub const NOWHERE: ModId = usize::MAX;
