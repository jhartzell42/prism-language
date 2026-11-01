//! What a widget block's statements do, beyond `prism_rt`'s `Build` methods: a
//! statement's value bound (an element emitted, a blueprint built), the rows of a plain
//! `for` gathered, a `match` or `if` over a handle lifted as one node, `never`, and a
//! `given:` block giving up.

use std::rc::Rc;

use prism_rt::{Build, Closure, Element, FieldKind, Value};

use crate::func::{self, eval_fault, Bx};
use crate::ops::R;
use crate::values::is_handle;

/// The build of a widget block, which is always there.
pub fn build<'r, 'x, 'a>(b: &'r mut Bx<'x, 'a>) -> R<&'r mut Build<'a>> {
    b.as_deref_mut().ok_or_else(|| eval_fault("this stands in a widget block, and there is no widget building here"))
}

/// What a statement does with the value on its right: an element is emitted, a
/// blueprint is built (transparently, Semantics ch. 8), anything else is the value.
pub fn bind_build(b: &mut Bx, v: Value) -> R<Value> {
    match v {
        Value::Element(e) => {
            check_identity(&e)?;
            build(b)?.emit((*e).clone())
        }
        Value::Widget(bp) => build(b)?.call(&bp, &[]),
        other => Ok(other),
    }
}

/// A dynamic where an identity field goes is refused, naming `rebuild` (Semantics ch. 8,
/// "Fields: what each sigil promises"; Type System ch. 4, "`@` never lifts").
pub fn check_identity(e: &Element) -> R<()> {
    let Some(decl) = &e.decl else { return Ok(()) };
    for (n, v) in &e.fields {
        if decl.field_kind(n) == Some(FieldKind::Identity) && !v.is_plain() {
            return Err(eval_fault(format!(
                "a dynamic in the identity field `{n}` of {}: a new {n} is a new place; for a new element per value, write\n      rebuild $x:\n          {}({n}=_, ..)",
                e.tag, e.tag
            )));
        }
    }
    Ok(())
}

/// What `never` is outside a build: the arm of a lifted `match` that fires nothing.
pub const NEVER: &str = "prism.never";

pub fn is_never_marker(v: &Value) -> bool {
    matches!(v, Value::Choice(c) if &*c.ctor == NEVER)
}

/// `never`: the event that never fires while building; the marker in a node's function.
pub fn never(b: &mut Bx) -> Value {
    match b.as_deref_mut() {
        Some(b) => b.never(),
        None => Value::ctor(NEVER, vec![]),
    }
}

/// `$ v`: a constant dynamic of a plain value while building.
pub fn constant(b: &mut Bx, v: Value) -> Value {
    match (v, b.as_deref_mut()) {
        (v @ Value::Dynamic(_), _) => v,
        (v, Some(b)) if !matches!(v, Value::Event(_)) => b.constant(v),
        (v, _) => v,
    }
}

/// A `for` over a plain collection gathers its rows' results as the table of Semantics
/// ch. 4 says: rows ending on events give `!(K ==> T)`, firing with the rows that fired;
/// rows ending on dynamics give `$(K ==> T)`. `prism_rt`'s plain `for` hands back the map
/// of the rows' results as they are.
pub fn gather_plain_rows(r: Value, b: &mut Build) -> R<Value> {
    let Value::Map(m) = &r else { return Ok(r) };
    if m.is_empty() {
        return Ok(r);
    }
    let keys: Vec<Value> = m.keys().cloned().collect();
    let vals: Vec<Value> = m.values().cloned().collect();
    if vals.iter().all(|v| matches!(v, Value::Event(_))) {
        let f = Closure::new("for.rows", vec![Value::array(keys)], |cap, fired| {
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
        let f = Closure::new("for.rows", vec![Value::array(keys)], |cap, now| {
            let Value::Array(keys) = &cap[0] else { unreachable!() };
            Ok(Value::Map(keys.iter().cloned().zip(now.iter().cloned()).collect()))
        });
        return Ok(b.lift(&vals, f));
    }
    Ok(r)
}

/// A `match` or an `if` over a dynamic or an event with plain arms, as one lifted node
/// (pick LM): a map over the scrutinee and every handle among `env`, the values its arms
/// read, or a sample when one of them is an event, whose arms that are `never` fire
/// nothing. `arms` gets the scrutinee's value and then `env`'s, each handle replaced by
/// its value.
pub fn lift_branch(b: &mut Bx, scrutinee: Value, env: Vec<Value>, arms: Value) -> R<Value> {
    let mut args = vec![scrutinee];
    args.extend(env);
    let events: Vec<usize> = (0..args.len()).filter(|i| matches!(args[*i], Value::Event(_))).collect();
    let dyns: Vec<usize> = (0..args.len()).filter(|i| matches!(args[*i], Value::Dynamic(_))).collect();
    if events.len() >= 2 {
        return Err(eval_fault(
            "this `match` reads several events in its arms; a `match` lifts over one event at most (doubt EL's gates are not built); merge the arms' events with `<|` outside the `match`",
        ));
    }
    let filter = !events.is_empty();
    let f = if filter {
        Value::Closure(Closure::new("match.filtered", vec![arms], |cap, a| {
            let v = func::call_plain(&cap[0], a.to_vec(), &mut None)?;
            Ok(if is_never_marker(&v) { Value::nil() } else { Value::some(v) })
        }))
    } else {
        arms
    };
    let node = func::lift(b, f, args, &events, &dyns)?;
    if filter {
        return build(b)?.filter(&node);
    }
    Ok(node)
}

/// The arms of a `match` or `if`, as compiled code writes them: `arms(build, scrutinee,
/// env)`.
pub type Arms = fn(&mut Bx, Value, &[Value]) -> R<Value>;

/// A compiled `match` or `if`: over a plain scrutinee its arms run here; over a handle,
/// as one lifted node ([`lift_branch`]).
pub fn branch(b: &mut Bx, scrutinee: Value, env: &[Value], arms: Arms) -> R<Value> {
    if !is_handle(&scrutinee) {
        return arms(b, scrutinee, env);
    }
    let f = Value::Closure(Closure::new("match", vec![], move |_, a| {
        let mut it = a.iter().cloned();
        let s = it.next().unwrap_or(Value::Unit);
        let rest: Vec<Value> = it.collect();
        arms(&mut None, s, &rest)
    }));
    lift_branch(b, scrutinee, env.to_vec(), f)
}

/// A `given:` block giving up (Semantics ch. 4, `given:`; ch. 10): `nil` under a `?`
/// name; under a `~` name, `Prelude.Unmet` with the guard's source text as its message
/// (decisions.md D11).
pub fn give_up(fallible: bool, guard: &str) -> Value {
    if fallible {
        Value::Fallible(Err(Rc::new(Value::record(
            "Error",
            vec![("kind", Value::ctor("Prelude.Unmet", vec![])), ("message", Value::text(guard.trim()))],
        ))))
    } else {
        Value::nil()
    }
}

/// The answer a `given:` block's last line gives: `? v`, `~ v`, or a `?T` passing through.
pub fn given_answer(fallible: bool, v: Value) -> Value {
    match (&v, fallible) {
        (Value::Opt(_) | Value::Fallible(_), _) => v,
        (_, true) => Value::ok(v),
        _ => Value::some(v),
    }
}

/// A `given:` bind: the value to match, or the failed value the block gives up into.
pub fn given_bind(v: Value) -> Result<Value, Value> {
    match &v {
        Value::Opt(None) | Value::Fallible(Err(_)) => Err(v),
        Value::Opt(Some(x)) | Value::Fallible(Ok(x)) => Ok((**x).clone()),
        _ => Ok(v),
    }
}
