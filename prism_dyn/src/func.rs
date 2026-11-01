//! Function values and application with the lifting table (Type System ch. 4, "Where the
//! table looks"; decisions.md D3 and D16).
//!
//! **A function value is a `prism_rt` closure with a meta.** Every function a program
//! makes, interpreted or compiled, is a [`prism_rt::Closure`] whose name ends in `#n`,
//! where `n` names its [`FnMeta`] in this thread's registry: the layers its parameters
//! declare, which of them are `Nat`, whether `?` and `~` are peeled at its concrete
//! parameters, and its body. The runtime calls such a closure as it calls any (`call`
//! runs [`invoke`] with no build); a program applies it through [`apply`], which reads
//! the meta and decides the lift. A closure with no meta (one `prism_rt` made) is called
//! as it is.
//!
//! **The lifting table at each application.** Each argument's runtime layer is compared
//! with its parameter's declared [`Layer`]:
//!
//! - the parameter takes it whole (a `$Text` parameter, a `!a`, a blueprint, a tag's
//!   field, a constructor's type-parameter field): ordinary application;
//! - otherwise a `$` or `!` argument is peeled and the row is chosen: dynamics and no
//!   event make a map over the dynamics (`Build::lift`, the change law); one event makes a
//!   sample (`Build::sample`, the tag law); two events are refused, naming both;
//! - under the time layer, a `?` or `~` argument at a concrete parameter is peeled at the
//!   plain layer, `nil` or the leftmost `Error` moving outward (L5), when the meta asks.
//!
//! A lifted application needs a widget to put its node in, so it is possible only while
//! a blueprint builds: [`Bx`] is `Some` then.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use num_traits::Signed;
use prism_rt::{Build, Closure, Fault, Value};

use crate::ops::R;

/// The build a statement runs in: present while a blueprint builds, absent in a node's
/// function at propagation time and in a `check` line.
pub type Bx<'x, 'a> = Option<&'x mut Build<'a>>;

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

/// The body of a function: `body(captured, args, build)`.
pub type Body = Rc<dyn Fn(&[Value], Vec<Value>, &mut Bx) -> R<Value>>;

/// What a function value's parameters declare, and what it does.
pub struct FnMeta {
    pub name: Rc<str>,
    pub layers: Vec<Layer>,
    pub nat: Vec<bool>,
    /// Peel `?` and `~` at concrete parameters, wrapping the result once (functions,
    /// builtins, methods, operators); constructors, lambdas and widget functions do not.
    pub peel: bool,
    /// A record's constructor: named arguments are put in its field order.
    pub ctor: Option<Rc<crate::tables::CtorMeta>>,
    /// The maker's own number for this code (compiler zero's code id).
    pub user: usize,
    pub body: Body,
}

thread_local! {
    static METAS: RefCell<Vec<Rc<FnMeta>>> = const { RefCell::new(Vec::new()) };
    static SITES: RefCell<HashMap<&'static str, usize>> = RefCell::new(HashMap::new());
}

/// Register a meta; closures over it are made by [`closure`].
pub fn register(meta: FnMeta) -> usize {
    METAS.with(|m| {
        let mut m = m.borrow_mut();
        m.push(Rc::new(meta));
        m.len() - 1
    })
}

/// The meta registered once for a site: compiled code names each of its function sites
/// by a key, and the meta is made the first time the site runs.
pub fn site(key: &'static str, make: impl FnOnce() -> FnMeta) -> usize {
    if let Some(id) = SITES.with(|s| s.borrow().get(key).copied()) {
        return id;
    }
    let id = register(make());
    SITES.with(|s| s.borrow_mut().insert(key, id));
    id
}

pub fn meta(id: usize) -> Rc<FnMeta> {
    METAS.with(|m| m.borrow()[id].clone())
}

/// The meta of a closure, if it has one.
pub fn meta_of(c: &Closure) -> Option<Rc<FnMeta>> {
    let (_, n) = c.name().rsplit_once('#')?;
    let id: usize = n.parse().ok()?;
    METAS.with(|m| m.borrow().get(id).cloned())
}

/// The meta of a function value, if it is a closure with one.
pub fn meta_of_value(f: &Value) -> Option<Rc<FnMeta>> {
    match f {
        Value::Closure(c) => meta_of(c),
        _ => None,
    }
}

/// A function value over meta `id`, shown as `display`.
pub fn closure(id: usize, display: &str, captured: Vec<Value>) -> Value {
    let m = meta(id);
    Value::Closure(Closure::new(&format!("{display}#{id}"), captured, move |cap, args| {
        invoke(&m, cap, args.to_vec(), &mut None)
    }))
}

// ----- application -----

/// Apply a function value to arguments, by the lifting table.
pub fn apply(b: &mut Bx, f: &Value, args: Vec<Value>) -> R<Value> {
    apply_with(b, f, args, None)
}

/// Apply, with layers overridden by explicit instantiation (`Name<<$Int>> x`, Type
/// System ch. 4: "`Twice<<$Int>> $x` asks for the other reading explicitly").
pub fn apply_with(b: &mut Bx, f: &Value, args: Vec<Value>, over: Option<&[Option<Layer>]>) -> R<Value> {
    let Value::Closure(c) = f else {
        return Err(match f {
            Value::Widget(_) => eval_fault("a blueprint is applied by binding it with `<-`, never as a function (`@` never lifts)"),
            other => eval_fault(format!("{} is not a function", crate::ops::show_plain(other))),
        });
    };
    let Some(m) = meta_of(c) else {
        // A closure from outside: called as it is.
        return c.call(&args);
    };
    let layer = |i: usize| -> Layer {
        if let Some(Some(l)) = over.and_then(|o| o.get(i)) {
            if i < m.layers.len() {
                return *l;
            }
        }
        m.layers.get(i).copied().unwrap_or(Layer::Var)
    };
    let mut events = Vec::new();
    let mut dyns = Vec::new();
    for (i, a) in args.iter().enumerate() {
        if !layer(i).lifts_time() {
            continue;
        }
        match a {
            Value::Event(_) => events.push(i),
            Value::Dynamic(_) => dyns.push(i),
            _ => {}
        }
    }
    if events.is_empty() && dyns.is_empty() {
        return invoke(&m, c.captured(), args, b);
    }
    lift(b, f.clone(), args, &events, &dyns)
}

/// Build the lift node for an application: a sample at one event, a map over dynamics,
/// or the refusal of two events (Type System ch. 4, the rows).
pub fn lift(b: &mut Bx, f: Value, args: Vec<Value>, events: &[usize], dyns: &[usize]) -> R<Value> {
    if events.len() >= 2 {
        return Err(eval_fault(
            "two events in one operation: in a tick where only one fires there is no value; merge them with `<|` to keep one, or use `!MergeAll` to keep both (Type System ch. 4)",
        ));
    }
    let Some(b) = b.as_deref_mut() else {
        return Err(eval_fault(
            "this application lifts over a dynamic or an event, which makes a node, and there is no widget building here (a lift inside a node's function, or in a `check`)",
        ));
    };
    let mut slots: Vec<usize> = events.to_vec();
    slots.extend_from_slice(dyns);
    let mut plain = args.clone();
    for &s in &slots {
        plain[s] = Value::Unit;
    }
    let node_fn = Closure::new("lift", vec![f, Value::array(plain)], move |cap, vals| {
        let Value::Array(plain) = &cap[1] else { unreachable!() };
        let mut full: Vec<Value> = plain.iter().cloned().collect();
        for (k, &s) in slots.iter().enumerate() {
            full[s] = vals.get(k).cloned().unwrap_or(Value::Unit);
        }
        apply(&mut None, &cap[0], full)
    });
    let dyn_vals: Vec<Value> = dyns.iter().map(|&i| args[i].clone()).collect();
    match events.first() {
        Some(&e) => b.sample(&args[e], &dyn_vals, node_fn),
        None => Ok(b.lift(&dyn_vals, node_fn)),
    }
}

/// Run a function on arguments its layers take (no lift): peel, convert `Nat`s, run the
/// body, wrap.
pub fn invoke(m: &FnMeta, cap: &[Value], args: Vec<Value>, b: &mut Bx) -> R<Value> {
    if !m.peel {
        let args = convert_nat(&m.nat, args);
        return (m.body)(cap, args, b);
    }
    let (args, wrap) = match peel(&m.layers, args) {
        Peeled::Short(v) => return Ok(v),
        Peeled::Args(a, w) => (a, w),
    };
    let args = convert_nat(&m.nat, args);
    let r = (m.body)(cap, args, b)?;
    Ok(wrap.apply(r))
}

/// Run a function's body with no peel and no conversion: an operator's user `impl`.
pub fn call_body(f: &Value, args: Vec<Value>, b: &mut Bx) -> R<Value> {
    let Value::Closure(c) = f else { return Err(eval_fault("not a function")) };
    match meta_of(c) {
        Some(m) => (m.body)(c.captured(), args, b),
        None => c.call(&args),
    }
}

/// Run a function value with no lift, peeling as its meta says.
pub fn call_plain(f: &Value, args: Vec<Value>, b: &mut Bx) -> R<Value> {
    let Value::Closure(c) = f else { return Err(eval_fault("not a function")) };
    match meta_of(c) {
        Some(m) => invoke(&m, c.captured(), args, b),
        None => c.call(&args),
    }
}

/// What peeling left: a short-circuit value, or the arguments and how to wrap the result.
pub enum Peeled {
    Short(Value),
    Args(Vec<Value>, Wrap),
}

#[derive(Default)]
pub struct Wrap {
    nil: bool,
    opt: bool,
    ok: bool,
    err: Option<Value>,
}

impl Wrap {
    pub fn apply(&self, r: Value) -> Value {
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

/// Peel `?` and `~` at concrete parameters (Type System ch. 4, "`?` and `~` at the plain
/// layer"): `nil` or the leftmost `Error` short-circuits; otherwise the result is
/// wrapped once.
pub fn peel(layers: &[Layer], mut args: Vec<Value>) -> Peeled {
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

/// Convert an `Int` that a `Nat` parameter receives: literals are `Int` until something
/// decides (pick LT).
pub fn to_nat(v: Value) -> Value {
    match v {
        Value::Int(n) if !n.is_negative() => Value::Nat(n),
        other => other,
    }
}

pub fn convert_nat(nat: &[bool], args: Vec<Value>) -> Vec<Value> {
    if !nat.iter().any(|n| *n) {
        return args;
    }
    args.into_iter().enumerate().map(|(i, a)| if nat.get(i).copied().unwrap_or(false) { to_nat(a) } else { a }).collect()
}

/// An evaluation fault, with no place: the caller names the place when it has one.
pub fn eval_fault(msg: impl Into<String>) -> Fault {
    Fault::new("prism.Eval", msg)
}

/// A function value made by Rust code: `name`, its layers, peeling or not, and a body.
pub fn native(key: &'static str, name: &str, layers: &[Layer], nat: &[bool], peel: bool, body: Body) -> Value {
    let id = site(key, || FnMeta {
        name: Rc::from(name),
        layers: layers.to_vec(),
        nat: nat.to_vec(),
        peel,
        ctor: None,
        user: usize::MAX,
        body,
    });
    closure(id, name, vec![])
}
