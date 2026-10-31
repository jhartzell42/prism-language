//! Expressions (Semantics ch. 3, "Statements and expressions"; Type System ch. 4).
//!
//! An expression never binds. While a blueprint builds, an expression over dynamics or
//! events makes the nodes the lifting table names (`interp::Interp::apply`); in a node's
//! function at propagation time, and in a `check` line, the same expression runs over
//! plain values.
//!
//! The time words:
//!
//! - **`x on !e`** is the sample, `Build::tag`: a dynamic `x` is read as the tick opened
//!   (the tag law); a plain `x` is fired with the event. An event on the left is the
//!   fourth row of the table and refused.
//! - **`!a <| !b`** is the leftmost merge, `Build::merge`.
//! - **`then`** belongs to the hold statement (`block`).
//!
//! The free sigils (Semantics ch. 3, "How a block ends"): `@ v` is a blueprint whose
//! result is `v`, evaluated when it builds; `$ v` a constant dynamic; `? v` and `~ v` the
//! optional's and the fault's injections, lifted like any plain function.
//!
//! **Picks** this module makes where a checker would decide (README.md, Picks):
//!
//! - **ML Map literals lift.** A map literal with a `$` or `!` among its keys or values is
//!   lifted over them, as a function of its entries: `{$next: Added} on !add` (Semantics
//!   ch. 4's `@labels`) samples the map. Tuples and arrays take their components whole
//!   (`[[!a, !b]]` is an array of two events, Type System ch. 4), and so do constructors'
//!   type-parameter fields; a constructor's concrete field lifts (`Add "item {$next}"`
//!   is a `$Change`), as Type System ch. 4 says a function's signature does.
//! - **LM A lifted `match` or `if`** over a dynamic or an event with plain arms is one
//!   node: a map over the scrutinee and every handle its arms read, or a sample when one
//!   of them is an event (doubt EL's first ruling). An arm that is `never` fires nothing.
//!   Arms that read two or more events (EL's gates) are refused with a diagnostic.

use std::rc::Rc;

use prism_rt::{ResultKind, Value};
use prism_syntax::ast::*;

use crate::interp::{BlockKind, Bx, Code, Interp, Loc, OpKind, Scopes};
use crate::load::Global;
use crate::ops::R;
use crate::pattern::parse_int;

/// A dynamic or an event: a value the lifting table peels.
pub fn is_handle(v: &Value) -> bool {
    matches!(v, Value::Dynamic(_) | Value::Event(_))
}

/// What `never` is outside a build: the arm of a lifted `match` that fires nothing.
const NEVER: &str = "prism_zero.never";

pub fn is_never_marker(v: &Value) -> bool {
    matches!(v, Value::Choice(c) if &*c.ctor == NEVER)
}

/// The names that decisions.md puts out of this compiler, by name (prism_syntax API.md,
/// pick OS), with the feature each belongs to.
const OUT_BY_NAME: &[(&str, &str)] = &[
    ("!Align", "`!Align`"),
    ("@Collect", "POD hosting (`@Collect`, `@View`, `@Edit`, `@Translate`)"),
    ("@View", "POD hosting (`@Collect`, `@View`, `@Edit`, `@Translate`)"),
    ("@Edit", "POD hosting (`@Collect`, `@View`, `@Edit`, `@Translate`)"),
    ("@Translate", "POD hosting (`@Collect`, `@View`, `@Edit`, `@Translate`)"),
    ("Over", "named lenses `&Home` and `Over`"),
];

impl Interp {
    /// Evaluate an expression.
    pub fn eval(&self, e: &'static Expr, sc: &mut Scopes, mut b: Bx) -> R<Value> {
        let loc = Loc { module: sc.module, span: e.span };
        match &e.kind {
            ExprKind::Name(n) => self.eval_name(n, sc, b, loc),
            ExprKind::Lit(l) => self.eval_lit(l, sc, b, loc),
            ExprKind::Unit(sigils) => Ok(self.marked(sigils, Value::Unit, b)),
            ExprKind::Nil(sigils) => Ok(self.marked(sigils, Value::nil(), b)),
            ExprKind::Bound(k) => {
                let name = match k {
                    BoundName::Key => "key",
                    BoundName::Ix => "ix",
                    BoundName::Jx => "jx",
                    BoundName::Kx => "kx",
                };
                self.lookup_local(sc, name, b, loc)?
                    .ok_or_else(|| self.fault(loc, format!("`{name}` is bound only inside a `for`")))
            }
            ExprKind::Tuple(items) => {
                let mut vs = Vec::new();
                for i in items {
                    vs.push(self.eval(i, sc, b.as_deref_mut())?);
                }
                Ok(Value::tuple(vs))
            }
            ExprKind::Array(items) => {
                let mut vs = Vec::new();
                for i in items {
                    vs.push(self.eval(i, sc, b.as_deref_mut())?);
                }
                Ok(Value::array(vs))
            }
            ExprKind::Map(entries) => {
                let mut vs = Vec::new();
                for (k, v) in entries {
                    vs.push(self.eval(k, sc, b.as_deref_mut())?);
                    vs.push(self.eval(v, sc, b.as_deref_mut())?);
                }
                if vs.iter().any(is_handle) {
                    // Pick ML: the literal lifts over its handles.
                    let id = self.code_at(e, 20, || Code::MapLit { keys: entries.len() });
                    let f = self.closure(id, "{map}", vec![]);
                    return self.apply(&f, vs, b, loc);
                }
                let mut m = im_rc::OrdMap::new();
                let mut it = vs.into_iter();
                while let (Some(k), Some(v)) = (it.next(), it.next()) {
                    m.insert(k, v);
                }
                Ok(Value::Map(m))
            }
            ExprKind::Group(inner) => self.eval(inner, sc, b),
            ExprKind::Call { callee, args, rest, .. } => {
                if *rest {
                    return Err(self.fault(loc, "`..` among a call's arguments belongs to a pattern"));
                }
                self.eval_call(callee, args, sc, b, loc)
            }
            ExprKind::Binary { op, lhs, rhs } => self.eval_binary(*op, lhs, rhs, sc, b, loc),
            ExprKind::Unary { op, operand } => {
                if let ExprKind::Lit(Literal { kind: LitKind::Int(s), marks, .. }) = &operand.kind {
                    if marks.is_empty() {
                        let n = parse_int(s).ok_or_else(|| self.fault(loc, format!("{s} is not an integer")))?;
                        match op {
                            UnOp::Neg => return Ok(Value::Int(-n)),
                            UnOp::Pos => return Ok(Value::Int(n)),
                            UnOp::Not => {}
                        }
                    }
                }
                let v = self.eval(operand, sc, b.as_deref_mut())?;
                match op {
                    UnOp::Pos => Ok(v),
                    UnOp::Neg => self.apply(&self.op_value(OpKind::Neg), vec![v], b, loc),
                    UnOp::Not => self.apply(&self.op_value(OpKind::Not), vec![v], b, loc),
                }
            }
            ExprKind::Suffixed { .. } => Err(self.out_of_scope(loc, "suffixed operators")),
            ExprKind::FreeSigil { sigil, operand } => match sigil {
                Sigil::At => {
                    let id = self.code_at(e, 21, || Code::FreeAt { expr: operand, module: sc.module });
                    let free = self.free_names(&**operand, walk_expr);
                    let cap = self.capture(sc, &free, vec![], b, loc)?;
                    Ok(self.blueprint(id, "@", cap))
                }
                Sigil::Dollar => {
                    let v = self.eval(operand, sc, b.as_deref_mut())?;
                    Ok(match (v, b) {
                        (v @ Value::Dynamic(_), _) => v,
                        (v, Some(b)) if !matches!(v, Value::Event(_)) => b.constant(v),
                        (v, _) => v,
                    })
                }
                Sigil::Question | Sigil::Tilde => {
                    let v = self.eval(operand, sc, b.as_deref_mut())?;
                    if is_handle(&v) {
                        let key = if *sigil == Sigil::Question { 22 } else { 23 };
                        let id = self.code_at(&(), key, || Code::Wrap(*sigil));
                        let f = self.closure(id, if key == 22 { "?" } else { "~" }, vec![]);
                        return self.apply(&f, vec![v], b, loc);
                    }
                    Ok(if *sigil == Sigil::Question { Value::some(v) } else { Value::ok(v) })
                }
                Sigil::Bang => Err(self.fault(loc, "a free `!` stands only before a type")),
                Sigil::Amp => Err(self.fault(loc, "a reference stands only in an `update` block")),
            },
            ExprKind::Field { recv, name, .. } => {
                let r = self.eval(recv, sc, b.as_deref_mut())?;
                self.field_of(e, r, &name.name, b, loc)
            }
            ExprKind::Index { recv, index } => {
                let r = self.eval(recv, sc, b.as_deref_mut())?;
                let k = self.eval(index, sc, b.as_deref_mut())?;
                if is_handle(&r) || is_handle(&k) {
                    let id = self.code_at(&(), 24, || Code::Index);
                    let f = self.closure(id, "[]", vec![]);
                    return self.apply(&f, vec![r, k], b, loc);
                }
                self.index(&r, &k, loc)
            }
            ExprKind::TopicField(name) => {
                let r = self.lookup_local(sc, "_", b.as_deref_mut(), loc)?.ok_or_else(|| self.fault(loc, "no topic `_` here"))?;
                self.field_of(e, r, &name.name, b, loc)
            }
            ExprKind::TopicIndex(index) => {
                let r = self.lookup_local(sc, "_", b.as_deref_mut(), loc)?.ok_or_else(|| self.fault(loc, "no topic `_` here"))?;
                let k = self.eval(index, sc, b.as_deref_mut())?;
                self.index(&r, &k, loc)
            }
            ExprKind::Method { recv, method, args } => {
                // `recv.Method a, b` is `Method recv, a, b` (prism_syntax pick ME).
                let r = self.eval(recv, sc, b.as_deref_mut())?;
                let f = self.eval_name(method, sc, b.as_deref_mut(), loc)?;
                let mut vs = vec![r];
                for a in args {
                    vs.push(self.eval(&a.value, sc, b.as_deref_mut())?);
                }
                self.apply(&f, vs, b, loc)
            }
            ExprKind::TypeArgs { target, .. } => self.eval(target, sc, b),
            ExprKind::Cast { expr, checked, .. } => {
                if *checked {
                    return Err(self.out_of_scope(loc, "non-empty arrays"));
                }
                self.eval(expr, sc, b)
            }
            ExprKind::NonEmpty(_) => Err(self.out_of_scope(loc, "non-empty arrays")),
            ExprKind::Absent => Err(self.out_of_scope(loc, "sparse tuples")),
            ExprKind::Rest => Err(self.fault(loc, "`..` belongs to a pattern")),
            ExprKind::Lambda { params, body } => {
                let module = sc.module;
                let id = self.code_at(e, 25, || {
                    let layers = match params {
                        Some(ps) => ps.iter().map(crate::interp::layer_of_pattern).collect(),
                        None => vec![crate::interp::Layer::Var],
                    };
                    Code::Lambda { params: params.as_deref(), body, module, layers }
                });
                let free = self.free_names(e, walk_expr);
                let cap = self.capture(sc, &free, vec![], b, loc)?;
                Ok(self.closure(id, "lambda", cap))
            }
            ExprKind::If { .. } | ExprKind::Match { .. } => self.eval_control(e, sc, b, false),
            ExprKind::Rebuild { scrutinee, body } => {
                let src = self.eval(scrutinee, sc, b.as_deref_mut())?;
                let kind = sc.bind_kind.take().unwrap_or(ResultKind::Plain);
                let module = sc.module;
                let id = self.code_at(body, 26, || Code::Block { block: body, kind: BlockKind::Rebuild, module, set_row: false });
                let free = self.free_names(body, walk_block);
                let cap = self.capture(sc, &free, vec![], b.as_deref_mut(), loc)?;
                let Value::Widget(bp) = self.blueprint(id, "rebuild", cap) else { unreachable!() };
                let b = b.ok_or_else(|| self.fault(loc, "a `rebuild` stands in a widget block"))?;
                b.rebuild(&src, kind, &bp).map_err(|f| self.wrap_fault(loc, f))
            }
            ExprKind::For { binders, source, body } => {
                let src = self.eval(source, sc, b.as_deref_mut())?;
                let dynamic = matches!(src, Value::Dynamic(_));
                let kind = sc.bind_kind.take().unwrap_or(ResultKind::Plain);
                let module = sc.module;
                let binders: Option<&'static ForBinders> = binders.as_ref();
                let id = self.code_at(body, if dynamic { 27 } else { 28 }, || Code::Block {
                    block: body,
                    kind: BlockKind::For(binders, dynamic),
                    module,
                    set_row: false,
                });
                let mut free: Vec<Rc<str>> = (*self.free_names(body, walk_block)).clone();
                free.push(Rc::from("ix"));
                free.push(Rc::from("jx"));
                let cap = self.capture(sc, &free, vec![], b.as_deref_mut(), loc)?;
                let Value::Widget(bp) = self.blueprint(id, "for", cap) else { unreachable!() };
                let b = b.ok_or_else(|| self.fault(loc, "a `for` over a collection stands in a widget block"))?;
                let r = b.for_each(&src, kind, &bp).map_err(|f| self.wrap_fault(loc, f))?;
                if dynamic {
                    return Ok(r);
                }
                gather_plain_rows(r, b)
            }
            ExprKind::Given(block) => self.run_given(block, sc, b),
            ExprKind::Update { target, body } => {
                let t = self.eval(target, sc, b.as_deref_mut())?;
                self.run_update(t, body, sc, b, loc)
            }
            ExprKind::WidgetBlock(block) => {
                let module = sc.module;
                let id = self.code_at(block, 29, || Code::Block { block, kind: BlockKind::WidgetExpr, module, set_row: false });
                let free = self.free_names(block, walk_block);
                let cap = self.capture(sc, &free, vec![], b, loc)?;
                Ok(self.blueprint(id, "@widget", cap))
            }
            ExprKind::Monad { .. } => Err(self.out_of_scope(loc, "`T monad:` blocks and `^Monad`")),
            ExprKind::Template(t) => self.eval_template(t, sc, b, loc),
            ExprKind::Any(inner) => self.eval(inner, sc, b),
            ExprKind::Lens(_) => Err(self.out_of_scope(loc, "named lenses `&Home` and `Over`")),
            ExprKind::Error => Err(self.fault(loc, "this expression did not parse")),
        }
    }

    /// A fault from `prism_rt`, placed at the statement that caused it.
    pub fn wrap_fault(&self, loc: Loc, f: prism_rt::Fault) -> prism_rt::Fault {
        if f.kind == crate::interp::CYCLE_BUDGET || f.message.contains(".prism:") {
            return f;
        }
        prism_rt::Fault::new(f.kind.clone(), format!("{}: {}", self.l.modules[loc.module].at(loc.span), f.message))
    }

    /// `$` on `unit` or `nil`: a constant dynamic; `@unit` a blueprint of `unit`.
    fn marked(&self, sigils: &[Sigil], v: Value, b: Bx) -> Value {
        match (sigils.first(), b) {
            (Some(Sigil::Dollar), Some(b)) => b.constant(v),
            (Some(Sigil::At), _) => {
                let id = self.code_at(&(), 30, || Code::WidgetBuiltin(Rc::from("@unit")));
                self.blueprint(id, "@unit", vec![Value::array(vec![]), Value::array(vec![])])
            }
            _ => v,
        }
    }

    fn field_of(&self, e: &'static Expr, r: Value, name: &str, b: Bx, loc: Loc) -> R<Value> {
        if is_handle(&r) {
            let id = self.code_at(e, 31, || Code::Field(Rc::from(name)));
            let f = self.closure(id, &format!(".{name}"), vec![]);
            return self.apply(&f, vec![r], b, loc);
        }
        self.field(&r, name, loc)
    }

    /// A name: a local, a global, a tag, a builtin reached by its qualified name.
    pub fn eval_name(&self, n: &'static SName, sc: &mut Scopes, mut b: Bx, loc: Loc) -> R<Value> {
        let text = n.text();
        if n.path.len() == 1 {
            match text.as_str() {
                "True" => return Ok(Value::Bool(true)),
                "False" => return Ok(Value::Bool(false)),
                "never" | "!never" => {
                    return Ok(match b {
                        Some(b) => b.never(),
                        None => Value::ctor(NEVER, vec![]),
                    });
                }
                _ => {}
            }
            if let Some(v) = self.lookup_local(sc, &text, b.as_deref_mut(), loc)? {
                return Ok(v);
            }
            if let Some((owner, g)) = self.resolve_bare(sc.module, &text) {
                return self.global(owner, &text, &g, b, loc);
            }
            if let Some((_, feature)) = OUT_BY_NAME.iter().find(|(n, _)| *n == text) {
                return Err(self.out_of_scope(loc, feature));
            }
            return Err(self.fault(loc, format!("unknown name {text}")));
        }
        let full = n.path.join(".");
        if self.tag(&full).is_some() {
            return self.tag_value(&full, loc);
        }
        if crate::builtins::is_qualified_builtin(&full) {
            let id = self.code_at(n, 32, || Code::Builtin { name: Rc::from(full.as_str()), sig: None, layers: vec![], nat: vec![] });
            return Ok(self.closure(id, &full, vec![]));
        }
        let module_name = n.path[..n.path.len() - 1].join(".");
        let base = format!("{}{}", prism_syntax::token::sigils_str(&n.sigils), n.base());
        let target = self.l.modules.iter().position(|m| m.name.as_deref() == Some(module_name.as_str()));
        match target {
            Some(m) => {
                let here = self.l.modules[sc.module].alias.unwrap_or(sc.module);
                if !self.l.modules[here].qualified.contains(&m) {
                    return Err(self.fault(loc, format!("{full}: module {module_name} is not used here; write `use {module_name}` (Semantics ch. 6)")));
                }
                match self.l.modules[m].decls.get(base.as_str()).cloned() {
                    Some(g) => self.global(m, &base, &g, b, loc),
                    None => Err(self.fault(loc, format!("module {module_name} declares no {base}"))),
                }
            }
            None => Err(self.fault(loc, format!("unknown name {full}: no tag, and no module {module_name} among the files"))),
        }
    }

    fn eval_lit(&self, l: &'static Literal, sc: &mut Scopes, mut b: Bx, loc: Loc) -> R<Value> {
        let v = match &l.kind {
            LitKind::Int(s) => Value::Int(parse_int(s).ok_or_else(|| self.fault(loc, format!("{s} is not an integer")))?),
            LitKind::Float(s) => crate::ops::float(
                s.chars().filter(|c| *c != '_').collect::<String>().parse().map_err(|_| self.fault(loc, format!("{s} is not a number")))?,
            ),
            LitKind::Duration { digits, unit } => {
                let n: i128 = digits.chars().filter(|c| *c != '_').collect::<String>().parse().unwrap_or(0);
                let scale: i128 = match unit.as_str() {
                    "ns" => 1,
                    "us" => 1_000,
                    "ms" => 1_000_000,
                    "s" => 1_000_000_000,
                    "min" => 60_000_000_000,
                    "h" => 3_600_000_000_000,
                    _ => 1,
                };
                Value::Duration(n * scale)
            }
            LitKind::Char(c) => Value::Char(*c),
            LitKind::Text(segs) => {
                if segs.iter().all(|s| matches!(s, TextSeg::Plain(_))) {
                    let s: String = segs.iter().map(|s| if let TextSeg::Plain(p) = s { p.as_str() } else { "" }).collect();
                    Value::text(&s)
                } else {
                    // Interpolation: the text-building function applied to every
                    // placeholder (Type System ch. 4), so it lifts like any other.
                    let id = self.code_at(l, 33, || {
                        Code::Interp(Rc::new(
                            segs.iter()
                                .map(|s| match s {
                                    TextSeg::Plain(p) => Some(p.clone()),
                                    TextSeg::Placeholder(_) => None,
                                })
                                .collect(),
                        ))
                    });
                    let mut vals = Vec::new();
                    for s in segs {
                        if let TextSeg::Placeholder(e) = s {
                            vals.push(self.eval(e, sc, b.as_deref_mut())?);
                        }
                    }
                    let f = self.closure(id, "interpolate", vec![]);
                    self.apply(&f, vals, b.as_deref_mut(), loc)?
                }
            }
        };
        // `$"Go"`: the constant lift (ch. 2, "Marks on literals").
        if l.marks.first() == Some(&Sigil::Dollar) {
            if let (Some(b), false) = (b, is_handle(&v)) {
                return Ok(b.constant(v));
            }
        }
        Ok(v)
    }

    fn eval_call(&self, callee: &'static Expr, args: &'static [Arg], sc: &mut Scopes, mut b: Bx, loc: Loc) -> R<Value> {
        // An element: `Ui.Label(text=$"...")`. Its fields take their arguments whole, by
        // their sigils (Semantics ch. 8).
        if let ExprKind::Name(n) = &callee.kind {
            if n.path.len() > 1 {
                let full = n.path.join(".");
                if let Some(t) = self.tag(&full) {
                    if !t.fault {
                        let decl = t.decl.clone();
                        let mut fields: Vec<(String, Value)> = Vec::new();
                        for (i, a) in args.iter().enumerate() {
                            let v = self.eval(&a.value, sc, b.as_deref_mut())?;
                            let name = match &a.name {
                                Some(id) => id.name.clone(),
                                None => decl
                                    .fields
                                    .get(i)
                                    .map(|(n, _)| n.to_string())
                                    .ok_or_else(|| self.fault(loc, format!("{full} takes {} fields", decl.fields.len())))?,
                            };
                            fields.push((name, v));
                        }
                        let fs: Vec<(&str, Value)> = fields.iter().map(|(n, v)| (n.as_str(), v.clone())).collect();
                        return self.element(&full, fs, loc);
                    }
                }
            }
        }
        // Explicit instantiation overrides the lift (Type System ch. 4).
        let (target, over) = match &callee.kind {
            ExprKind::TypeArgs { target, types } => (&**target, Some(types)),
            _ => (callee, None),
        };
        let f = self.eval(target, sc, b.as_deref_mut())?;
        let mut named: Vec<(Option<&str>, Value)> = Vec::new();
        for a in args {
            let v = self.eval(&a.value, sc, b.as_deref_mut())?;
            named.push((a.name.as_ref().map(|n| n.name.as_str()), v));
        }
        let vals = if named.iter().any(|(n, _)| n.is_some()) {
            self.order_named(&f, named, loc)?
        } else {
            named.into_iter().map(|(_, v)| v).collect()
        };
        let overrides = match over {
            Some(types) => Some(self.instantiation(&f, types)),
            None => None,
        };
        self.apply_with(&f, vals, overrides.as_deref(), b, loc)
    }

    /// Named arguments of a record's constructor, put in field order.
    fn order_named(&self, f: &Value, named: Vec<(Option<&str>, Value)>, loc: Loc) -> R<Vec<Value>> {
        let info = match self.layers_of(f).map(|(c, _)| c) {
            Some(c) => match &*c {
                Code::Ctor(info, _) => info.clone(),
                _ => return Err(self.fault(loc, "named arguments are a record's or an element's")),
            },
            None => return Err(self.fault(loc, "named arguments are a record's or an element's")),
        };
        let mut out: Vec<Option<Value>> = vec![None; info.fields.len()];
        for (i, (n, v)) in named.into_iter().enumerate() {
            let slot = match n {
                Some(n) => info
                    .fields
                    .iter()
                    .position(|(f, _)| &**f == n)
                    .ok_or_else(|| self.fault(loc, format!("{} has no field {n}", info.name)))?,
                None => i,
            };
            if slot < out.len() {
                out[slot] = Some(v);
            }
        }
        out.into_iter()
            .enumerate()
            .map(|(i, v)| v.ok_or_else(|| self.fault(loc, format!("{} needs its field {}", info.name, info.fields[i].0))))
            .collect()
    }

    /// The layers `F<<T, ..>>` fixes: a parameter whose declared type is a variable given
    /// a sigiled type takes its argument whole. Variables in order of first appearance
    /// (Type System ch. 2, Inference).
    fn instantiation(&self, f: &Value, types: &[Type]) -> Vec<Option<crate::interp::Layer>> {
        let sig = match self.layers_of(f).map(|(c, _)| c) {
            Some(c) => match &*c {
                Code::Fn(fc) => fc.sig,
                Code::Builtin { sig, .. } => *sig,
                _ => None,
            },
            None => None,
        };
        let Some(TypeKind::Fn { params, .. }) = sig.map(|s| &s.ty.kind) else { return vec![] };
        let mut order: Vec<String> = Vec::new();
        fn vars(t: &Type, out: &mut Vec<String>) {
            match &t.kind {
                TypeKind::Name { name, args } => {
                    if name.path.len() == 1 && !name.is_upper() && args.is_empty() && !out.contains(&name.base().to_string()) {
                        out.push(name.base().to_string());
                    }
                    args.iter().for_each(|a| vars(a, out));
                }
                TypeKind::Array { elem, .. } => vars(elem, out),
                TypeKind::Group(t) | TypeKind::Sigiled { inner: t, .. } => vars(t, out),
                TypeKind::Tuple(ts) => ts.iter().for_each(|t| vars(t, out)),
                TypeKind::Map { key, value } => {
                    vars(key, out);
                    vars(value, out)
                }
                _ => {}
            }
        }
        for p in params {
            vars(p, &mut order);
        }
        params
            .iter()
            .map(|p| match &p.kind {
                TypeKind::Name { name, args } if args.is_empty() && name.sigils.is_empty() => {
                    let i = order.iter().position(|v| v == name.base())?;
                    let t = types.get(i)?;
                    Some(crate::interp::layer_of(t))
                }
                _ => None,
            })
            .collect()
    }

    fn eval_binary(&self, op: BinOp, lhs: &'static Expr, rhs: &'static Expr, sc: &mut Scopes, mut b: Bx, loc: Loc) -> R<Value> {
        match op {
            BinOp::On => {
                let l = self.eval(lhs, sc, b.as_deref_mut())?;
                let r = self.eval(rhs, sc, b.as_deref_mut())?;
                match (&l, &r) {
                    (Value::Event(_), Value::Event(_)) => Err(self.fault(
                        loc,
                        "`!a on !b` is two events in one operation; to mean the last `!a` when `!b` fires, hold `!a` first (Type System ch. 4)",
                    )),
                    (_, Value::Event(_)) => {
                        let b = b.ok_or_else(|| self.fault(loc, "`on` makes a node, and there is no widget building here"))?;
                        b.tag(&r, &l)
                    }
                    (_, Value::Dynamic(_)) => Err(self.fault(loc, "`on` samples at an event; its right operand is a dynamic")),
                    // Inside a lifted function the event's value stands for the firing.
                    _ => Ok(l),
                }
            }
            BinOp::Merge => {
                let l = self.eval(lhs, sc, b.as_deref_mut())?;
                let r = self.eval(rhs, sc, b.as_deref_mut())?;
                if is_never_marker(&l) {
                    return Ok(r);
                }
                match (&l, &r) {
                    (Value::Event(_), Value::Event(_)) => {
                        let b = b.ok_or_else(|| self.fault(loc, "`<|` makes a node, and there is no widget building here"))?;
                        b.merge(&[l, r])
                    }
                    (Value::Event(_), _) | (_, Value::Event(_)) | (Value::Dynamic(_), _) | (_, Value::Dynamic(_)) => {
                        Err(self.fault(loc, "`<|` merges two events"))
                    }
                    _ => Ok(l),
                }
            }
            BinOp::And | BinOp::Or => {
                let l = self.eval(lhs, sc, b.as_deref_mut())?;
                if let Value::Bool(x) = l {
                    if (op == BinOp::And && !x) || (op == BinOp::Or && x) {
                        return Ok(Value::Bool(x));
                    }
                    let r = self.eval(rhs, sc, b.as_deref_mut())?;
                    if let Value::Bool(_) = r {
                        return Ok(r);
                    }
                    return self.apply(&self.op_value(OpKind::Bin(op)), vec![l, r], b, loc);
                }
                let r = self.eval(rhs, sc, b.as_deref_mut())?;
                self.apply(&self.op_value(OpKind::Bin(op)), vec![l, r], b, loc)
            }
            _ => {
                let l = self.eval(lhs, sc, b.as_deref_mut())?;
                let r = self.eval(rhs, sc, b.as_deref_mut())?;
                self.apply(&self.op_value(OpKind::Bin(op)), vec![l, r], b, loc)
            }
        }
    }

    /// A `match` or an `if`: over a plain scrutinee, the chosen arm, as an expression
    /// block or (in a widget block, `widget`) as a building arm whose statements take
    /// positions in the block's sequence (Semantics ch. 4, "A building `match` or `if`
    /// needs a plain scrutinee"); over a dynamic or an event, a lifted function of it.
    pub fn eval_control(&self, e: &'static Expr, sc: &mut Scopes, mut b: Bx, widget: bool) -> R<Value> {
        let loc = Loc { module: sc.module, span: e.span };
        let s = match &e.kind {
            ExprKind::Match { scrutinee, .. } => self.eval(scrutinee, sc, b.as_deref_mut())?,
            ExprKind::If { cond, .. } => match &**cond {
                Cond::Expr(c) => self.eval(c, sc, b.as_deref_mut())?,
                Cond::Bind { value, .. } => self.eval(value, sc, b.as_deref_mut())?,
            },
            _ => unreachable!(),
        };
        if is_handle(&s) {
            return self.lift_arms(e, s, sc, b, loc);
        }
        if widget {
            if let Some(b) = b {
                return match self.select_arm(e, &s, sc, None, loc)? {
                    Some((block, binds)) => {
                        sc.push();
                        for (n, v) in binds {
                            sc.bind(n, v);
                        }
                        let r = self.run_widget_block(block, sc, b);
                        sc.pop();
                        r
                    }
                    None => Ok(Value::Unit),
                };
            }
        }
        self.arms_with(e, s, sc, b)
    }

    /// The arms of `e` for a plain scrutinee value, as expression blocks.
    pub fn arms_with(&self, e: &'static Expr, s: Value, sc: &mut Scopes, mut b: Bx) -> R<Value> {
        let loc = Loc { module: sc.module, span: e.span };
        match self.select_arm(e, &s, sc, b.as_deref_mut(), loc)? {
            Some((block, binds)) => {
                sc.push();
                for (n, v) in binds {
                    sc.bind(n, v);
                }
                let r = self.run_expr_block(block, sc, b);
                sc.pop();
                r
            }
            None => Ok(Value::Unit),
        }
    }

    /// The arm a plain scrutinee chooses, and its pattern's bindings. `None` is an `if`
    /// with no `else` whose condition failed.
    fn select_arm(&self, e: &'static Expr, s: &Value, sc: &mut Scopes, mut b: Bx, loc: Loc) -> R<Option<(&'static Block, Vec<(Rc<str>, Value)>)>> {
        match &e.kind {
            ExprKind::Match { arms, .. } => {
                for arm in arms {
                    let mut binds = Vec::new();
                    if let Some(p) = &arm.pattern {
                        if !crate::pattern::matches(self, sc.module, p, s, &mut binds)? {
                            continue;
                        }
                    }
                    if let Some(g) = &arm.guard {
                        sc.push();
                        for (n, v) in binds.clone() {
                            sc.bind(n, v);
                        }
                        let ok = self.eval(g, sc, b.as_deref_mut());
                        sc.pop();
                        match ok? {
                            Value::Bool(true) => {}
                            Value::Bool(false) => continue,
                            other => return Err(self.fault(loc, format!("a guard is a Bool, and this one is {}", crate::ops::show_plain(&other)))),
                        }
                    }
                    return Ok(Some((&arm.body, binds)));
                }
                Err(self.fault(loc, format!("no arm matches {}", crate::ops::show_plain(s))))
            }
            ExprKind::If { cond, then, else_ } => match &**cond {
                Cond::Expr(_) => match s {
                    Value::Bool(true) => Ok(Some((then, vec![]))),
                    Value::Bool(false) => Ok(else_.as_ref().map(|e| (e, vec![]))),
                    other => Err(self.fault(loc, format!("an `if` needs a Bool, and this one is {}", crate::ops::show_plain(other)))),
                },
                Cond::Bind { pattern, guard, .. } => {
                    let mut binds = Vec::new();
                    let mut ok = crate::pattern::matches(self, sc.module, pattern, s, &mut binds)?;
                    if ok {
                        if let Some(g) = guard {
                            sc.push();
                            for (n, v) in binds.clone() {
                                sc.bind(n, v);
                            }
                            let r = self.eval(g, sc, b.as_deref_mut());
                            sc.pop();
                            ok = matches!(r?, Value::Bool(true));
                        }
                    }
                    if ok { Ok(Some((then, binds))) } else { Ok(else_.as_ref().map(|e| (e, vec![]))) }
                }
            },
            _ => unreachable!(),
        }
    }

    /// Pick LM: a `match` or `if` over a handle, as one lifted node.
    fn lift_arms(&self, e: &'static Expr, s: Value, sc: &mut Scopes, mut b: Bx, loc: Loc) -> R<Value> {
        let free = self.free_names(e, walk_arms);
        let mut lifted_names: Vec<Rc<str>> = Vec::new();
        let mut lifted_vals = Vec::new();
        let mut plain_names: Vec<Rc<str>> = Vec::new();
        for n in free.iter() {
            if let Some(v) = self.lookup_local(sc, n, b.as_deref_mut(), loc)? {
                if is_handle(&v) {
                    lifted_names.push(n.clone());
                    lifted_vals.push(v);
                } else {
                    plain_names.push(n.clone());
                }
            }
        }
        let mut args = vec![s];
        args.extend(lifted_vals);
        let events: Vec<usize> = (0..args.len()).filter(|i| matches!(args[*i], Value::Event(_))).collect();
        let dyns: Vec<usize> = (0..args.len()).filter(|i| matches!(args[*i], Value::Dynamic(_))).collect();
        if events.len() >= 2 {
            return Err(self.fault(
                loc,
                "this `match` reads several events in its arms; compiler zero lifts a `match` over one event at most (doubt EL's gates are not built); merge the arms' events with `<|` outside the `match`",
            ));
        }
        let filter = !events.is_empty();
        let key = format!("arms:{}:{}", e as *const Expr as usize, lifted_names.join(","));
        let module = sc.module;
        let lifted = Rc::new(lifted_names);
        let id = self.code_keyed(key, || Code::Arms { expr: e, module, lifted: lifted.clone(), filter });
        let cap = self.capture(sc, &plain_names, vec![], b.as_deref_mut(), loc)?;
        let f = self.closure(id, "match", cap);
        let node = self.lift(f, args, &events, &dyns, b.as_deref_mut(), loc)?;
        if filter {
            let b = b.ok_or_else(|| self.fault(loc, "a lifted `match` makes a node, and there is no widget building here"))?;
            return b.filter(&node);
        }
        Ok(node)
    }

    /// Code templates (Templates ch. 2) as text: each run as written, each placeholder by
    /// display, a brace block's last line, a call's result. The POD form of a templated
    /// language (`templatetext`, `templateplaceholder`) is not built.
    fn eval_template(&self, t: &'static Template, sc: &mut Scopes, mut b: Bx, loc: Loc) -> R<Value> {
        let mut out = String::new();
        self.template_parts(&t.parts, sc, b.as_deref_mut(), loc, &mut out)?;
        Ok(Value::text(&out))
    }

    fn template_parts(&self, parts: &'static [TemplatePart], sc: &mut Scopes, mut b: Bx, loc: Loc, out: &mut String) -> R<()> {
        for p in parts {
            match p {
                TemplatePart::Text(s) => out.push_str(s),
                TemplatePart::Placeholder { expr, column } => {
                    let v = self.eval(expr, sc, b.as_deref_mut())?;
                    let text = match &v {
                        Value::Array(items) => {
                            let mut s = String::new();
                            for i in items.iter() {
                                s.push_str(&self.display(i, b.as_deref_mut(), loc)?);
                            }
                            s
                        }
                        other => self.display(other, b.as_deref_mut(), loc)?,
                    };
                    match column {
                        // A placeholder alone on its line indents every line of its value.
                        Some(c) => {
                            let pad = " ".repeat(*c as usize);
                            let lines: Vec<&str> = text.split('\n').collect();
                            for (i, l) in lines.iter().enumerate() {
                                if i > 0 {
                                    out.push('\n');
                                }
                                if !l.is_empty() {
                                    out.push_str(&pad);
                                }
                                out.push_str(l);
                            }
                        }
                        None => out.push_str(&text),
                    }
                }
                TemplatePart::Block(block) => {
                    let v = self.run_expr_block(block, sc, b.as_deref_mut())?;
                    if !matches!(v, Value::Unit) {
                        out.push_str(&self.display(&v, b.as_deref_mut(), loc)?);
                    }
                }
                TemplatePart::Call { name, attrs, body, span } => {
                    let cloc = Loc { module: sc.module, span: *span };
                    let f = self.eval_name(name, sc, b.as_deref_mut(), cloc)?;
                    let mut args = Vec::new();
                    for a in attrs {
                        args.push(self.eval(&a.value, sc, b.as_deref_mut())?);
                    }
                    if let Some(body) = body {
                        let mut inner = String::new();
                        self.template_parts(body, sc, b.as_deref_mut(), cloc, &mut inner)?;
                        args.push(Value::text(&inner));
                    }
                    let v = self.apply(&f, args, b.as_deref_mut(), cloc)?;
                    out.push_str(&self.display(&v, b.as_deref_mut(), cloc)?);
                }
            }
        }
        Ok(())
    }

    /// A code object made once per key.
    pub fn code_keyed(&self, key: String, make: impl FnOnce() -> Code) -> crate::interp::CodeId {
        thread_local! {
            static KEYED: std::cell::RefCell<std::collections::HashMap<(usize, String), usize>> = Default::default();
        }
        let me = self as *const Interp as usize;
        if let Some(id) = KEYED.with(|k| k.borrow().get(&(me, key.clone())).copied()) {
            return id;
        }
        let id = self.add_code(make());
        KEYED.with(|k| k.borrow_mut().insert((me, key), id));
        id
    }

    /// The value a builtin global would have, for `Global` lookups from builtins.
    pub fn prelude_fn(&self, name: &str, loc: Loc) -> R<Value> {
        match self.resolve_bare(self.l.prelude, name) {
            Some((owner, g)) => self.global(owner, name, &g, None, loc),
            None => Err(self.fault(loc, format!("the prelude has no {name}"))),
        }
    }

    /// Whether a global is a constructor: for the resolver.
    pub fn is_ctor(&self, module: crate::load::ModId, name: &str) -> bool {
        matches!(self.resolve_bare(module, name), Some((_, Global::Ctor(_))) | Some((_, Global::Unit(_))))
    }
}

// ----- free names -------------------------------------------------------------------

fn name_of(n: &SName, out: &mut Vec<Rc<str>>) {
    if n.path.len() == 1 {
        out.push(Rc::from(n.text()));
    }
}

/// Every unqualified name an expression writes, and the language's bound names.
pub fn walk_expr(e: &Expr, out: &mut Vec<Rc<str>>) {
    match &e.kind {
        ExprKind::Name(n) => name_of(n, out),
        ExprKind::Lit(l) => walk_lit(l, out),
        ExprKind::Bound(k) => out.push(Rc::from(match k {
            BoundName::Key => "key",
            BoundName::Ix => "ix",
            BoundName::Jx => "jx",
            BoundName::Kx => "kx",
        })),
        ExprKind::Tuple(xs) | ExprKind::Array(xs) => xs.iter().for_each(|x| walk_expr(x, out)),
        ExprKind::Map(es) => es.iter().for_each(|(k, v)| {
            walk_expr(k, out);
            walk_expr(v, out)
        }),
        ExprKind::Group(x) | ExprKind::NonEmpty(x) | ExprKind::Any(x) => walk_expr(x, out),
        ExprKind::Call { callee, args, .. } => {
            walk_expr(callee, out);
            args.iter().for_each(|a| walk_expr(&a.value, out));
        }
        ExprKind::Binary { lhs, rhs, .. } | ExprKind::Suffixed { lhs, rhs, .. } => {
            walk_expr(lhs, out);
            walk_expr(rhs, out);
        }
        ExprKind::Unary { operand, .. } | ExprKind::FreeSigil { operand, .. } => walk_expr(operand, out),
        ExprKind::Field { recv, .. } => walk_expr(recv, out),
        ExprKind::Index { recv, index } => {
            walk_expr(recv, out);
            walk_expr(index, out);
        }
        ExprKind::TopicField(_) => out.push(Rc::from("_")),
        ExprKind::TopicIndex(i) => {
            out.push(Rc::from("_"));
            walk_expr(i, out);
        }
        ExprKind::Method { recv, method, args } => {
            walk_expr(recv, out);
            name_of(method, out);
            args.iter().for_each(|a| walk_expr(&a.value, out));
        }
        ExprKind::TypeArgs { target, .. } => walk_expr(target, out),
        ExprKind::Cast { expr, .. } => walk_expr(expr, out),
        ExprKind::Lambda { body, .. } => match &**body {
            LambdaBody::Expr(x) => walk_expr(x, out),
            LambdaBody::Block(b) => walk_block(b, out),
        },
        ExprKind::If { cond, then, else_ } => {
            walk_cond(cond, out);
            walk_block(then, out);
            if let Some(e) = else_ {
                walk_block(e, out);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            walk_expr(scrutinee, out);
            for a in arms {
                if let Some(g) = &a.guard {
                    walk_expr(g, out);
                }
                walk_block(&a.body, out);
            }
        }
        ExprKind::Rebuild { scrutinee, body } => {
            walk_expr(scrutinee, out);
            walk_block(body, out);
        }
        ExprKind::For { source, body, .. } => {
            walk_expr(source, out);
            walk_block(body, out);
        }
        ExprKind::Given(b) | ExprKind::WidgetBlock(b) => walk_block(b, out),
        ExprKind::Update { target, body } => {
            walk_expr(target, out);
            walk_block(body, out);
        }
        ExprKind::Monad { monad, body } => {
            walk_expr(monad, out);
            walk_block(body, out);
        }
        ExprKind::Template(t) => walk_template(&t.parts, out),
        ExprKind::Unit(_) | ExprKind::Nil(_) | ExprKind::Absent | ExprKind::Rest | ExprKind::Lens(_) | ExprKind::Error => {}
    }
}

/// The names a lifted `match`'s or `if`'s arms read, without its scrutinee.
fn walk_arms(e: &Expr, out: &mut Vec<Rc<str>>) {
    match &e.kind {
        ExprKind::Match { arms, .. } => {
            for a in arms {
                if let Some(g) = &a.guard {
                    walk_expr(g, out);
                }
                walk_block(&a.body, out);
            }
        }
        ExprKind::If { cond, then, else_ } => {
            if let Cond::Bind { guard: Some(g), .. } = &**cond {
                walk_expr(g, out);
            }
            walk_block(then, out);
            if let Some(e) = else_ {
                walk_block(e, out);
            }
        }
        _ => walk_expr(e, out),
    }
}

fn walk_template(parts: &[TemplatePart], out: &mut Vec<Rc<str>>) {
    for p in parts {
        match p {
            TemplatePart::Text(_) => {}
            TemplatePart::Placeholder { expr, .. } => walk_expr(expr, out),
            TemplatePart::Block(b) => walk_block(b, out),
            TemplatePart::Call { name, attrs, body, .. } => {
                name_of(name, out);
                attrs.iter().for_each(|a| walk_expr(&a.value, out));
                if let Some(b) = body {
                    walk_template(b, out);
                }
            }
        }
    }
}

fn walk_lit(l: &Literal, out: &mut Vec<Rc<str>>) {
    if let LitKind::Text(segs) = &l.kind {
        for s in segs {
            if let TextSeg::Placeholder(e) = s {
                walk_expr(e, out);
            }
        }
    }
}

fn walk_cond(c: &Cond, out: &mut Vec<Rc<str>>) {
    match c {
        Cond::Expr(e) => walk_expr(e, out),
        Cond::Bind { value, guard, .. } => {
            walk_expr(value, out);
            if let Some(g) = guard {
                walk_expr(g, out);
            }
        }
    }
}

/// Every unqualified name a block's statements write.
pub fn walk_block(b: &Block, out: &mut Vec<Rc<str>>) {
    for s in &b.stmts {
        walk_stmt(s, out);
    }
}

pub fn walk_stmt(s: &Stmt, out: &mut Vec<Rc<str>>) {
    match &s.kind {
        StmtKind::Emit(_) | StmtKind::Signature(_) => {}
        StmtKind::Bind { value, then, .. } => {
            walk_expr(value, out);
            if let Some(t) = then {
                walk_expr(t, out);
            }
        }
        StmtKind::Ignore(e) | StmtKind::Return(e) | StmtKind::Expr(e) => walk_expr(e, out),
        StmtKind::Let { value, .. } => walk_expr(value, out),
        StmtKind::Fn(c) => match &c.body {
            FnBody::Expr(e) => walk_expr(e, out),
            FnBody::Block(b) => walk_block(b, out),
        },
        StmtKind::Widget(w) => walk_block(&w.body, out),
        StmtKind::Set { target, value } | StmtKind::Accum { target, value, .. } => {
            walk_expr(target, out);
            walk_expr(value, out);
        }
        StmtKind::Require(c) | StmtKind::Forbid(c) => walk_expr(c, out),
        StmtKind::While { cond, body } => {
            walk_cond(cond, out);
            walk_block(body, out);
        }
    }
}

/// A `for` over a plain collection gathers its rows' results as the table of Semantics
/// ch. 4 says: rows ending on events give `!(K ==> T)`, firing with the rows that fired;
/// rows ending on dynamics give `$(K ==> T)`. `prism_rt`'s plain `for` hands back the map
/// of the rows' results as they are, so the interpreter gathers it.
fn gather_plain_rows(r: Value, b: &mut prism_rt::Build) -> R<Value> {
    let Value::Map(m) = &r else { return Ok(r) };
    if m.is_empty() {
        return Ok(r);
    }
    let keys: Vec<Value> = m.keys().cloned().collect();
    let vals: Vec<Value> = m.values().cloned().collect();
    if vals.iter().all(|v| matches!(v, Value::Event(_))) {
        let f = prism_rt::Closure::new("for.rows", vec![Value::array(keys)], |cap, fired| {
            let Value::Array(keys) = &cap[0] else { unreachable!() };
            let mut out = im_rc::OrdMap::new();
            for (k, v) in keys.iter().zip(fired) {
                if let Value::Opt(Some(x)) = v {
                    out.insert(k.clone(), (**x).clone());
                }
            }
            Ok(if out.is_empty() { Value::nil() } else { Value::some(Value::Map(out)) })
        });
        return b.combine(&vals, f);
    }
    if vals.iter().all(|v| matches!(v, Value::Dynamic(_))) {
        let f = prism_rt::Closure::new("for.rows", vec![Value::array(keys)], |cap, now| {
            let Value::Array(keys) = &cap[0] else { unreachable!() };
            Ok(Value::Map(keys.iter().cloned().zip(now.iter().cloned()).collect()))
        });
        return Ok(b.lift(&vals, f));
    }
    Ok(r)
}
