//! Operations on values that a program's expressions spell: field access, indexing,
//! construction, elements, the operators with their user `impl`s, trait dispatch, and
//! how a value enters a text. Each also exists as a function value, so that it lifts
//! like any other application (Type System ch. 4: "Field access is a plain function").

use std::rc::Rc;

use prism_rt::{Element, Value};

use crate::func::{self, apply, call_body, call_plain, eval_fault, Body, Bx, FnMeta, Layer};
use crate::ops::{self, Op, R};
use crate::tables::{self, CtorMeta};

// ----- fields and indexing -----

/// Field access, with `?` and `~` peeled at the plain layer (Type System ch. 4, "Field
/// access is a plain function"). A failed `~` passes through, except that an `Error`'s
/// own fields are read: a value of type `Error` and a failed `~T` are one value here.
pub fn field(v: &Value, name: &str) -> R<Value> {
    match v {
        Value::Record(r) => r
            .fields
            .iter()
            .find(|(n, _)| &**n == name)
            .map(|(_, v)| v.clone())
            .ok_or_else(|| eval_fault(format!("{} has no field {name}", r.name))),
        Value::Element(e) => e.field(name).cloned().ok_or_else(|| eval_fault(format!("{} has no field {name}", e.tag))),
        Value::Opt(None) => Ok(Value::nil()),
        Value::Opt(Some(x)) => Ok(Value::some(field(x, name)?)),
        Value::Fallible(Ok(x)) => Ok(Value::ok(field(x, name)?)),
        Value::Fallible(Err(e)) => match field(e, name) {
            Ok(f) => Ok(f),
            Err(_) => Ok(v.clone()),
        },
        other => Err(eval_fault(format!("{} has no field {name}", ops::show_plain(other)))),
    }
}

/// Indexing gives `?v` (Semantics ch. 10, "Indexing returns `?V`").
pub fn index(v: &Value, k: &Value) -> R<Value> {
    match v {
        Value::Map(m) => Ok(m.get(k).cloned().map(Value::some).unwrap_or(Value::nil())),
        Value::Array(a) => Ok(ops::index_of(k).and_then(|i| a.get(i).cloned()).map(Value::some).unwrap_or(Value::nil())),
        Value::Text(t) => Ok(ops::index_of(k).and_then(|i| t.char_at(i)).map(|c| Value::some(Value::Char(c))).unwrap_or(Value::nil())),
        Value::Record(r) if &*r.name == "Table" => crate::builtins::table_get(v, k),
        Value::Opt(None) => Ok(Value::nil()),
        Value::Opt(Some(x)) => index(x, k),
        other => Err(eval_fault(format!("{} cannot be indexed", ops::show_plain(other)))),
    }
}

/// A dynamic or an event: a value the lifting table peels.
pub fn is_handle(v: &Value) -> bool {
    matches!(v, Value::Dynamic(_) | Value::Event(_))
}

/// `r.name`, lifted when `r` is a handle.
pub fn get_field(b: &mut Bx, r: Value, name: &str) -> R<Value> {
    if is_handle(&r) {
        let f = field_fn(name);
        return apply(b, &f, vec![r]);
    }
    field(&r, name)
}

/// `r[k]`, lifted when either is a handle.
pub fn get_index(b: &mut Bx, r: Value, k: Value) -> R<Value> {
    if is_handle(&r) || is_handle(&k) {
        let f = func::native("prism.index", "[]", &[], &[], false, Rc::new(|_, a, _| index(&a[0], &a[1])));
        return apply(b, &f, vec![r, k]);
    }
    index(&r, &k)
}

/// `.name` as a function, for lifting.
pub fn field_fn(name: &str) -> Value {
    thread_local! {
        static FIELDS: std::cell::RefCell<std::collections::HashMap<String, usize>> = Default::default();
    }
    let id = match FIELDS.with(|f| f.borrow().get(name).copied()) {
        Some(id) => id,
        None => {
            let n: Rc<str> = Rc::from(name);
            let id = func::register(FnMeta {
                name: Rc::from(format!(".{name}")),
                layers: vec![],
                nat: vec![],
                peel: false,
                ctor: None,
                user: usize::MAX,
                body: Rc::new(move |_, a, _| field(&a[0], &n)),
            });
            FIELDS.with(|f| f.borrow_mut().insert(name.to_string(), id));
            id
        }
    };
    func::closure(id, &format!(".{name}"), vec![])
}

// ----- construction -----

/// The value of a constructor written as an expression: a record or choice with no
/// fields is its value; one with fields is its function.
pub fn ctor_value(name: &str) -> R<Value> {
    let c = tables::ctor(name).ok_or_else(|| eval_fault(format!("no constructor {name}")))?;
    if c.fields.is_empty() {
        return Ok(if c.record { Value::record(&c.name, vec![]) } else { Value::ctor(&c.name, vec![]) });
    }
    thread_local! {
        static CTORS: std::cell::RefCell<std::collections::HashMap<Rc<str>, usize>> = Default::default();
    }
    let id = match CTORS.with(|f| f.borrow().get(&c.name).copied()) {
        Some(id) => id,
        None => {
            let cc = c.clone();
            let id = func::register(FnMeta {
                name: c.name.clone(),
                layers: c.layers.clone(),
                nat: vec![],
                peel: false,
                ctor: Some(c.clone()),
                user: usize::MAX,
                body: Rc::new(move |_, a, _| construct(&cc, a)),
            });
            CTORS.with(|f| f.borrow_mut().insert(c.name.clone(), id));
            id
        }
    };
    Ok(func::closure(id, &c.name, vec![]))
}

/// A constructor's function value over its own declaration (two modules may declare
/// the same constructor name; the tables keep one of them).
pub fn ctor_fn(c: Rc<CtorMeta>) -> Value {
    if c.fields.is_empty() {
        return if c.record { Value::record(&c.name, vec![]) } else { Value::ctor(&c.name, vec![]) };
    }
    let cc = c.clone();
    let id = func::register(FnMeta {
        name: c.name.clone(),
        layers: c.layers.clone(),
        nat: vec![],
        peel: false,
        ctor: Some(c.clone()),
        user: usize::MAX,
        body: Rc::new(move |_, a, _| construct(&cc, a)),
    });
    func::closure(id, &c.name, vec![])
}

/// Construct a record or a choice; a record's arguments come in field order.
pub fn construct(info: &CtorMeta, args: Vec<Value>) -> R<Value> {
    if args.len() != info.fields.len() {
        return Err(eval_fault(format!("{} takes {} arguments and was given {}", info.name, info.fields.len(), args.len())));
    }
    let args: Vec<Value> = args.into_iter().zip(info.nat.iter()).map(|(a, n)| if *n { func::to_nat(a) } else { a }).collect();
    if info.record {
        let fields = info.fields.iter().map(|n| &**n).zip(args).collect();
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

/// Named arguments of a record's constructor, put in field order.
pub fn order_named(f: &Value, named: Vec<(Option<&str>, Value)>) -> R<Vec<Value>> {
    let Some(info) = func::meta_of_value(f).and_then(|m| m.ctor.clone()) else {
        return Err(eval_fault("named arguments are a record's or an element's"));
    };
    let mut out: Vec<Option<Value>> = vec![None; info.fields.len()];
    for (i, (n, v)) in named.into_iter().enumerate() {
        let slot = match n {
            Some(n) => info.fields.iter().position(|f| &**f == n).ok_or_else(|| eval_fault(format!("{} has no field {n}", info.name)))?,
            None => i,
        };
        if slot < out.len() {
            out[slot] = Some(v);
        }
    }
    out.into_iter()
        .enumerate()
        .map(|(i, v)| v.ok_or_else(|| eval_fault(format!("{} needs its field {}", info.name, info.fields[i]))))
        .collect()
}

/// Apply with named arguments (`Point(x=1, y=2)`): a record's constructor's.
pub fn apply_named(b: &mut Bx, f: &Value, named: Vec<(Option<&str>, Value)>) -> R<Value> {
    let vals = if named.iter().any(|(n, _)| n.is_some()) { order_named(f, named)? } else { named.into_iter().map(|(_, v)| v).collect() };
    apply(b, f, vals)
}

// ----- elements and tags -----

/// An element of a declared tag, fields by name (Semantics ch. 8). Its fields take their
/// arguments whole, by their sigils.
pub fn element(tag: &str, fields: Vec<(&str, Value)>) -> R<Value> {
    let t = tables::tag(tag).ok_or_else(|| eval_fault(format!("no tag {tag}")))?;
    if t.fault {
        return Err(eval_fault(format!("{tag} is a fault kind, not an element")));
    }
    Element::new(&t.decl, fields).map(Value::element).map_err(|f| eval_fault(f.message))
}

/// An element with its fields given positionally or by name.
pub fn element_args(tag: &str, args: Vec<(Option<&str>, Value)>) -> R<Value> {
    let t = tables::tag(tag).ok_or_else(|| eval_fault(format!("no tag {tag}")))?;
    let mut fields: Vec<(String, Value)> = Vec::new();
    for (i, (n, v)) in args.into_iter().enumerate() {
        let name = match n {
            Some(n) => n.to_string(),
            None => t
                .decl
                .fields
                .get(i)
                .map(|(n, _)| n.to_string())
                .ok_or_else(|| eval_fault(format!("{tag} takes {} fields", t.decl.fields.len())))?,
        };
        fields.push((name, v));
    }
    element(tag, fields.iter().map(|(n, v)| (n.as_str(), v.clone())).collect())
}

/// The value of a tag written as an expression: a fault kind is its own value; an
/// element tag with fields is its constructor; with none, its element.
pub fn tag_value(name: &str) -> R<Value> {
    let t = tables::tag(name).ok_or_else(|| eval_fault(format!("no tag {name}")))?;
    if t.fault {
        return Ok(Value::ctor(name, vec![]));
    }
    if t.decl.fields.is_empty() {
        return Ok(Value::element(Element::new(&t.decl, vec![])?));
    }
    thread_local! {
        static TAGS: std::cell::RefCell<std::collections::HashMap<String, usize>> = Default::default();
    }
    let id = match TAGS.with(|f| f.borrow().get(name).copied()) {
        Some(id) => id,
        None => {
            let n = name.to_string();
            let id = func::register(FnMeta {
                name: Rc::from(name),
                layers: vec![],
                nat: vec![],
                peel: false,
                ctor: None,
                user: usize::MAX,
                body: Rc::new(move |_, a, _| {
                    let t = tables::tag(&n).ok_or_else(|| eval_fault(format!("no tag {n}")))?;
                    let fields: Vec<(&str, Value)> = t.decl.fields.iter().map(|(n, _)| &**n).zip(a).collect();
                    element(&n, fields)
                }),
            });
            TAGS.with(|f| f.borrow_mut().insert(name.to_string(), id));
            id
        }
    };
    Ok(func::closure(id, name, vec![]))
}

// ----- operators -----

/// The trait whose method an operator is (Type System ch. 3: "`==` is a method of `^Eq`").
fn op_trait(op: Op) -> &'static str {
    match op {
        Op::Eq | Op::NotEq => "Eq",
        Op::Lt | Op::Le | Op::Gt | Op::Ge => "Ord",
        _ => "Num",
    }
}

/// A binary operator on plain values: a user `impl` of the operator's trait method
/// first, then the built-in meaning.
pub fn binop_plain(b: &mut Bx, op: Op, x: &Value, y: &Value) -> R<Value> {
    let tr = op_trait(op);
    if tables::trait_has_impls(tr) {
        let ty = ops::type_name(x);
        if let Some(f) = tables::impl_method(tr, &ty, op.as_str()) {
            return call_body(&f, vec![x.clone(), y.clone()], b);
        }
        if tr == "Ord" {
            if let Some(f) = tables::impl_method("Ord", &ty, "Compare") {
                let r = call_plain(&f, vec![x.clone(), y.clone()], b)?;
                let o = match &r {
                    Value::Choice(c) if &*c.ctor == "Less" => std::cmp::Ordering::Less,
                    Value::Choice(c) if &*c.ctor == "Greater" => std::cmp::Ordering::Greater,
                    _ => std::cmp::Ordering::Equal,
                };
                return Ok(Value::Bool(match op {
                    Op::Lt => o == std::cmp::Ordering::Less,
                    Op::Le => o != std::cmp::Ordering::Greater,
                    Op::Gt => o == std::cmp::Ordering::Greater,
                    _ => o != std::cmp::Ordering::Less,
                }));
            }
        }
    }
    ops::binop(op, x, y)
}

/// Prefix `-` and `not`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnOp {
    Neg,
    Not,
}

/// An operator as a function value, for lifting: arithmetic takes concrete numbers
/// (peeling `?` and `~`), the rest take their operands as they are.
pub fn op_value(op: Op) -> Value {
    thread_local! {
        static OPS: std::cell::RefCell<std::collections::HashMap<Op, usize>> = Default::default();
    }
    let id = match OPS.with(|f| f.borrow().get(&op).copied()) {
        Some(id) => id,
        None => {
            let layers = if op.is_arith() { vec![Layer::Concrete, Layer::Concrete] } else { vec![Layer::Var, Layer::Var] };
            let body: Body = match op {
                Op::And => Rc::new(|_, a, _| bool_op(Op::And, &a[0], &a[1])),
                Op::Or => Rc::new(|_, a, _| bool_op(Op::Or, &a[0], &a[1])),
                _ => Rc::new(move |_, a, b| binop_plain(b, op, &a[0], &a[1])),
            };
            let id = func::register(FnMeta { name: Rc::from(op.as_str()), layers, nat: vec![], peel: true, ctor: None, user: usize::MAX, body });
            OPS.with(|f| f.borrow_mut().insert(op, id));
            id
        }
    };
    func::closure(id, op.as_str(), vec![])
}

fn bool_op(op: Op, x: &Value, y: &Value) -> R<Value> {
    match (x, y) {
        (Value::Bool(a), Value::Bool(c)) => Ok(Value::Bool(if op == Op::And { *a && *c } else { *a || *c })),
        _ => Err(eval_fault(format!("`{}` needs two Bools", op.as_str()))),
    }
}

/// `-x` and `not x` as function values.
pub fn unop_value(op: UnOp) -> Value {
    match op {
        UnOp::Neg => func::native("prism.neg", "-", &[Layer::Concrete, Layer::Concrete], &[], true, Rc::new(|_, a, _| ops::negate(&a[0]))),
        UnOp::Not => func::native(
            "prism.not",
            "not",
            &[Layer::Var, Layer::Var],
            &[],
            true,
            Rc::new(|_, a, _| match &a[0] {
                Value::Bool(x) => Ok(Value::Bool(!x)),
                other => Err(eval_fault(format!("`not` needs a Bool, and was given {}", ops::show_plain(other)))),
            }),
        ),
    }
}

/// `x op y` with the lifting table: `on`, `<|` and the short circuits are the caller's.
pub fn binop(b: &mut Bx, op: Op, x: Value, y: Value) -> R<Value> {
    if !is_handle(&x) && !is_handle(&y) && !(op.is_arith() && (is_opt(&x) || is_opt(&y))) {
        return binop_plain(b, op, &x, &y);
    }
    apply(b, &op_value(op), vec![x, y])
}

fn is_opt(v: &Value) -> bool {
    matches!(v, Value::Opt(_) | Value::Fallible(_))
}

/// Prefix `-x` and `not x` with the lifting table.
pub fn unop(b: &mut Bx, op: UnOp, x: Value) -> R<Value> {
    apply(b, &unop_value(op), vec![x])
}

/// `a and b`, `a or b`: a plain `Bool` on the left short-circuits; otherwise lifted.
/// The right operand is given as a thunk, run only when needed.
pub fn and_or(b: &mut Bx, op: Op, x: Value, y: impl FnOnce(&mut Bx) -> R<Value>) -> R<Value> {
    if let Value::Bool(v) = x {
        if (op == Op::And && !v) || (op == Op::Or && v) {
            return Ok(Value::Bool(v));
        }
        let r = y(b)?;
        if let Value::Bool(_) = r {
            return Ok(r);
        }
        return apply(b, &op_value(op), vec![x, r]);
    }
    let r = y(b)?;
    apply(b, &op_value(op), vec![x, r])
}

// ----- trait methods -----

/// A trait method's declaration, for dispatch on the runtime type of one argument.
pub struct MethodMeta {
    pub trait_name: Rc<str>,
    pub name: Rc<str>,
    /// Which argument's runtime type picks the impl.
    pub dispatch: usize,
    /// The trait's default body, if it gives one.
    pub default: Option<Value>,
    /// A prelude trait: with no impl and no default, the derivable answer.
    pub prelude: bool,
}

/// A trait method as a function value.
pub fn method_value(key: &'static str, m: MethodMeta, layers: &[Layer]) -> Value {
    let name = m.name.clone();
    let m = Rc::new(m);
    let id = func::site(key, || FnMeta {
        name: name.clone(),
        layers: layers.to_vec(),
        nat: vec![],
        peel: true,
        ctor: None,
        user: usize::MAX,
        body: Rc::new(move |_, a, b| dispatch(&m, a, b)),
    });
    func::closure(id, &name, vec![])
}

/// Dispatch a trait method on the runtime type of its dispatch argument (Type System
/// ch. 3): the type's `impl`, else the trait's default, else the structural answer of a
/// derivable trait.
pub fn dispatch(m: &MethodMeta, args: Vec<Value>, b: &mut Bx) -> R<Value> {
    let recv = args.get(m.dispatch).cloned().unwrap_or(Value::Unit);
    let recv = match &recv {
        // An array of the trait's type dispatches on its first element.
        Value::Array(a) if !a.is_empty() && !tables::has_impl(&m.trait_name, "[]") => a[0].clone(),
        _ => recv,
    };
    let ty = ops::type_name(&recv);
    if let Some(f) = tables::impl_method(&m.trait_name, &ty, &m.name) {
        return call_plain(&f, args, b);
    }
    if let Some(d) = &m.default {
        return call_plain(d, args, b);
    }
    if m.prelude {
        return crate::builtins::structural(&m.name, &args);
    }
    Err(eval_fault(format!("^{} has no impl for {ty}, and {} has no default", m.trait_name, m.name)))
}

// ----- text -----

/// How a placeholder's value enters a text: a text as it is, a character as itself,
/// anything else by `Show` (Type System ch. 4, interpolation).
pub fn display(v: &Value, b: &mut Bx) -> R<String> {
    Ok(match v {
        Value::Text(t) => t.to_string(),
        Value::Char(c) => c.to_string(),
        Value::Nat(n) | Value::Int(n) => n.to_string(),
        Value::U8(n) => n.to_string(),
        Value::Bool(x) => {
            if *x {
                "True".into()
            } else {
                "False".into()
            }
        }
        other => show(other, b)?,
    })
}

/// `Show`: the type's `impl ^Show` if it has one, else the transcript's spelling.
pub fn show(v: &Value, b: &mut Bx) -> R<String> {
    if tables::trait_has_impls("Show") {
        let ty = ops::type_name(v);
        if let Some(f) = tables::impl_method("Show", &ty, "Show") {
            if let Value::Text(t) = call_plain(&f, vec![v.clone()], b)? {
                return Ok(t.to_string());
            }
        }
    }
    Ok(ops::show_plain(v))
}

/// The `^Show` hook the transcript prints by (prism_rt's `pod::set_show_hook`).
pub fn show_hook() -> prism_rt::pod::ShowHook {
    Rc::new(|v: &Value| {
        let ty = ops::type_name(v);
        if !tables::has_impl("Show", &ty) {
            return None;
        }
        show(v, &mut None).ok()
    })
}

/// A text literal's interpolation: the literal's plain parts, `None` where a
/// placeholder's value goes. Lifted like any function over its placeholders.
pub fn interpolate(b: &mut Bx, parts: &[Option<&str>], vals: Vec<Value>) -> R<Value> {
    if vals.iter().any(is_handle) {
        let id = func::site("prism.interp", || FnMeta {
            name: Rc::from("interpolate"),
            layers: vec![],
            nat: vec![],
            peel: false,
            ctor: None,
            user: usize::MAX,
            body: Rc::new(|cap, a, b| {
                let Some(Value::Array(ps)) = cap.first() else { unreachable!() };
                let parts: Vec<Option<String>> =
                    ps.iter().map(|p| if let Value::Opt(Some(t)) = p { Some(t.as_text().map(|t| t.to_string()).unwrap_or_default()) } else { None }).collect();
                interpolate_plain(b, parts.into_iter(), a)
            }),
        });
        let ps = Value::array(parts.iter().map(|p| match p { Some(s) => Value::some(Value::text(s)), None => Value::nil() }).collect());
        let f = func::closure(id, "interpolate", vec![ps]);
        return apply(b, &f, vals);
    }
    interpolate_plain(b, parts.iter().map(|p| p.map(|s| s.to_string())), vals)
}

pub fn interpolate_plain(b: &mut Bx, parts: impl Iterator<Item = Option<String>>, vals: Vec<Value>) -> R<Value> {
    let mut out = String::new();
    let mut it = vals.into_iter();
    for p in parts {
        match p {
            Some(s) => out.push_str(&s),
            None => {
                let v = it.next().unwrap_or(Value::Unit);
                out.push_str(&display(&v, b)?);
            }
        }
    }
    Ok(Value::text(&out))
}

/// `? v` and `~ v`: lifted over a handle.
pub fn wrap_some(b: &mut Bx, v: Value) -> R<Value> {
    if is_handle(&v) {
        let f = func::native("prism.some", "?", &[], &[], false, Rc::new(|_, a, _| Ok(Value::some(a.into_iter().next().unwrap_or(Value::Unit)))));
        return apply(b, &f, vec![v]);
    }
    Ok(Value::some(v))
}

pub fn wrap_ok(b: &mut Bx, v: Value) -> R<Value> {
    if is_handle(&v) {
        let f = func::native("prism.ok", "~", &[], &[], false, Rc::new(|_, a, _| Ok(Value::ok(a.into_iter().next().unwrap_or(Value::Unit)))));
        return apply(b, &f, vec![v]);
    }
    Ok(Value::ok(v))
}

/// A map literal: lifted over a `$` or `!` among its entries (pick ML).
pub fn map_literal(b: &mut Bx, entries: Vec<Value>) -> R<Value> {
    if entries.iter().any(is_handle) {
        let n = entries.len() / 2;
        let f = func::native(
            "prism.maplit",
            "{map}",
            &[],
            &[],
            false,
            Rc::new(|_, a, _| {
                let mut m = im_rc::OrdMap::new();
                let mut it = a.into_iter();
                while let (Some(k), Some(v)) = (it.next(), it.next()) {
                    m.insert(k, v);
                }
                Ok(Value::Map(m))
            }),
        );
        let _ = n;
        return apply(b, &f, entries);
    }
    let mut m = im_rc::OrdMap::new();
    let mut it = entries.into_iter();
    while let (Some(k), Some(v)) = (it.next(), it.next()) {
        m.insert(k, v);
    }
    Ok(Value::Map(m))
}
