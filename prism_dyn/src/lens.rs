//! `&` blocks (Semantics ch. 5): the parts a lens path names, read and replaced.
//!
//! **An `&` block means the pure function it translates to** (round 47 AB): its state is
//! the updated value and every live local, and each statement replaces one part of one
//! local. The values are persistent (`im_rc` and `Rc`), and a part is replaced with
//! `Rc::make_mut`, so a part nobody else holds is edited in place and a shared part is
//! copied first: K5's "edited in place when unique, else copy", with K1's one meaning
//! either way. `Buffer a` is an array and `Table k v` a record of its keys in insertion
//! order and a map (K6). A local is the caller's: compiler zero keeps it in a frame,
//! compiled code in a Rust variable.

use std::rc::Rc;

use prism_rt::Value;

use crate::func::eval_fault;
use crate::ops::R;

/// One step of a lens path.
#[derive(Clone, Debug)]
pub enum Step {
    Field(Rc<str>),
    Index(Value),
}

/// The part of `v` at `steps`.
pub fn read(v: &Value, steps: &[Step]) -> R<Value> {
    let mut v = v.clone();
    for s in steps {
        v = match s {
            Step::Field(f) => crate::values::field(&v, f)?,
            Step::Index(k) => crate::values::index(&v, k)?,
        };
    }
    Ok(v)
}

/// The element a `for &s <- &p:` lends: the part, out of its `?`.
pub fn read_element(v: &Value, steps: &[Step]) -> R<Value> {
    Ok(match read(v, steps)? {
        Value::Opt(Some(x)) => (*x).clone(),
        other => other,
    })
}

/// Set the part at `steps` inside `whole`, in place where nobody else holds it. A part
/// keeps its type: an `Int` literal set into a `Nat` part is a `Nat`, as a checker would
/// have typed the literal (`&fuel <= 0`, then `&fuel -? 1`).
pub fn set(whole: Value, steps: &[Step], v: Value) -> R<Value> {
    let Some((first, rest)) = steps.split_first() else { return Ok(keep_type(&whole, v)) };
    match (whole, first) {
        (Value::Record(mut r), Step::Field(f)) => {
            let rec = Rc::make_mut(&mut r);
            let slot = rec.fields.iter_mut().find(|(n, _)| **n == **f).ok_or_else(|| eval_fault(format!("{} has no field {f}", rec.name)))?;
            let part = std::mem::take(&mut slot.1);
            slot.1 = set(part, rest, v)?;
            Ok(Value::Record(r))
        }
        (t @ Value::Record(_), Step::Index(k)) => {
            let cur = crate::builtins::table_get(&t, k)?;
            let new = set_index_part(cur, rest, v)?;
            crate::builtins::table_set(t, k.clone(), new)
        }
        (Value::Map(mut m), Step::Index(k)) => {
            let cur = m.get(k).cloned().map(Value::some).unwrap_or(Value::nil());
            match set_index_part(cur, rest, v)? {
                Value::Opt(Some(x)) => {
                    m.insert(k.clone(), (*x).clone());
                }
                Value::Opt(None) => {
                    m.remove(k);
                }
                other => {
                    m.insert(k.clone(), other);
                }
            }
            Ok(Value::Map(m))
        }
        (Value::Array(mut a), Step::Index(k)) => {
            let i = crate::ops::index_of(k)
                .filter(|i| *i < a.len())
                .ok_or_else(|| eval_fault(format!("index {} is past the end of an array of {}", crate::ops::show_plain(k), a.len())))?;
            let cur = Value::some(a[i].clone());
            match set_index_part(cur, rest, v)? {
                Value::Opt(Some(x)) => a.set(i, (*x).clone()),
                Value::Opt(None) => return Err(eval_fault("an array's element cannot be removed by setting `nil`")),
                other => a.set(i, other),
            };
            Ok(Value::Array(a))
        }
        (Value::Opt(Some(x)), _) => Ok(Value::some(set((*x).clone(), steps, v)?)),
        (other, _) => Err(eval_fault(format!("{} has no part here", crate::ops::show_plain(&other)))),
    }
}

/// Replace a local's part in place: `&x.f[k] <= v`.
pub fn set_in(local: &mut Value, steps: &[Step], v: Value) -> R<()> {
    let whole = std::mem::take(local);
    *local = set(whole, steps, v)?;
    Ok(())
}

/// An index path's part is `?V` (Semantics ch. 5): a deeper step goes through the `?`.
fn set_index_part(cur: Value, rest: &[Step], v: Value) -> R<Value> {
    if rest.is_empty() {
        return Ok(v);
    }
    match cur {
        Value::Opt(Some(x)) => Ok(Value::some(set((*x).clone(), rest, v)?)),
        _ => Err(eval_fault("a path through a missing key")),
    }
}

/// The keys and values of a collection, in order: an array's indices, a map's keys, a
/// table's keys in insertion order.
pub fn entries(v: &Value) -> Vec<(Value, Value)> {
    match v {
        Value::Array(a) => a.iter().enumerate().map(|(i, x)| (Value::nat(i as u64), x.clone())).collect(),
        Value::Map(m) => m.iter().map(|(k, x)| (k.clone(), x.clone())).collect(),
        Value::Record(r) if &*r.name == "Table" => crate::builtins::table_entries(v),
        _ => vec![],
    }
}

/// The value set into a part, with the part's integer type kept.
pub fn keep_type(old: &Value, v: Value) -> Value {
    match (old, &v) {
        (Value::Nat(_), Value::Int(_)) => crate::func::to_nat(v),
        (Value::Opt(Some(o)), Value::Opt(Some(x))) if matches!((&**o, &**x), (Value::Nat(_), Value::Int(_))) => {
            Value::some(crate::func::to_nat((**x).clone()))
        }
        _ => v,
    }
}
