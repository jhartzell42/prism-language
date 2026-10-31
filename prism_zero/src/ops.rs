//! Operations on plain values: arithmetic, comparison, equality, display.
//!
//! The rules are Type System ch. 2 ("Numbers") and ch. 3 ("The three built in"):
//!
//! - **`Nat` and `Int` are arbitrary precision** (round 47 BI); `U8` wraps (round 47 OV),
//!   and its checked operators `+?`, `-?`, `*?` give `nil` where the result does not fit.
//! - **`-` is `Int`'s, not `Nat`'s** (round 47 NS): `Nat - Nat` is refused, naming `-?`
//!   and `Int n`. **`-?` on `Nat` gives `?Nat`**, `nil` below zero.
//! - **`/` on the integers gives `?`**, `nil` for a zero divisor.
//! - **`^Ord` orders records field by field and choices by alternative, then payload**:
//!   by the constructor's place in its declaration, not by its name.
//!
//! Compiler zero has no checker (D3), so an integer literal is an `Int` (the default
//! where nothing decides, doubt LD) until a signature says `Nat`; then the interpreter
//! converts it at the call (see `interp`). Where a `Nat` meets an `Int` literal, the sum
//! or product of two non-negative numbers is a `Nat`: what a checker would have made of
//! the literal.

use std::cmp::Ordering;
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};
use prism_rt::{Fault, Value};
use prism_syntax::ast::BinOp;

use crate::load::Loaded;

pub type R<T> = Result<T, Fault>;

fn type_err(msg: impl Into<String>) -> Fault {
    Fault::new("prism_zero.Type", msg)
}

/// The integer inside an integer value.
pub fn int_of(v: &Value) -> Option<BigInt> {
    match v {
        Value::Nat(n) | Value::Int(n) => Some(n.clone()),
        Value::U8(n) => Some(BigInt::from(*n)),
        _ => None,
    }
}

/// The non-negative machine index inside an integer value, for indexing and counts.
pub fn index_of(v: &Value) -> Option<usize> {
    int_of(v).and_then(|n| n.to_usize())
}

fn wrap_u8(n: &BigInt) -> Value {
    let m: BigInt = ((n % 256) + 256) % 256;
    Value::U8(m.to_u8().unwrap_or(0))
}

/// The result type of `+` and `*` on two integers.
fn combine_int(a: &Value, b: &Value, r: BigInt) -> Value {
    match (a, b) {
        (Value::U8(_), _) | (_, Value::U8(_)) => wrap_u8(&r),
        (Value::Nat(_), Value::Nat(_)) => Value::Nat(r),
        (Value::Nat(_), Value::Int(_)) | (Value::Int(_), Value::Nat(_)) if !r.is_negative() => Value::Nat(r),
        _ => Value::Int(r),
    }
}

/// A binary operator on plain values. `on`, `<|`, `and` and `or` are the interpreter's,
/// since they are about time or short-circuit.
pub fn binop(l: &Loaded, op: BinOp, a: &Value, b: &Value) -> R<Value> {
    use BinOp::*;
    match op {
        Eq => Ok(Value::Bool(equal(a, b))),
        NotEq => Ok(Value::Bool(!equal(a, b))),
        Lt => Ok(Value::Bool(compare(l, a, b) == Ordering::Less)),
        Le => Ok(Value::Bool(compare(l, a, b) != Ordering::Greater)),
        Gt => Ok(Value::Bool(compare(l, a, b) == Ordering::Greater)),
        Ge => Ok(Value::Bool(compare(l, a, b) != Ordering::Less)),
        Add | Sub | Mul | Div | Mod | AddQ | SubQ | MulQ => arith(op, a, b),
        And | Or | On | Merge => Err(type_err(format!("`{}` is not an arithmetic operator", op.as_str()))),
    }
}

fn arith(op: BinOp, a: &Value, b: &Value) -> R<Value> {
    use BinOp::*;
    if as_float(a).is_some() || as_float(b).is_some() {
        return float_arith(op, a, b);
    }
    // Text joined by `+`: not in the books' operator table; refused with the spelling
    // they use.
    let (Some(x), Some(y)) = (int_of(a), int_of(b)) else {
        if let (Value::Duration(x), Value::Duration(y)) = (a, b) {
            let _ = (x, y);
            return Err(Fault::new(
                "prism_zero.OutOfScope",
                "`Duration` arithmetic is out of this compiler (decisions.md, Features in and out)",
            ));
        }
        return Err(type_err(format!("`{}` needs two numbers, and was given {} and {}", op.as_str(), show_plain(a), show_plain(b))));
    };
    let sized = matches!(a, Value::U8(_)) || matches!(b, Value::U8(_));
    Ok(match op {
        Add => combine_int(a, b, x + y),
        Mul => combine_int(a, b, x * y),
        Sub => {
            if sized {
                wrap_u8(&(x - y))
            } else if matches!((a, b), (Value::Nat(_), Value::Nat(_))) {
                return Err(type_err(
                    "`Nat - Nat` has no answer below zero: write `-?` for a ?Nat, or `Int n` to subtract integers (round 47 NS)",
                ));
            } else {
                Value::Int(x - y)
            }
        }
        Div | Mod => {
            if y.is_zero() {
                return Ok(if op == Div { Value::nil() } else { a.clone() });
            }
            let (q, r) = floor_div(&x, &y);
            let out = if op == Div { q } else { r };
            let v = match (a, b) {
                _ if sized => wrap_u8(&out),
                (Value::Int(_), _) | (_, Value::Int(_)) if out.is_negative() => Value::Int(out),
                (Value::Nat(_), _) | (_, Value::Nat(_)) => Value::Nat(out),
                _ => Value::Int(out),
            };
            // `/` gives `?` (Type System ch. 2); `mod` a plain number, and `x mod 0` is `x`
            // (Claude's pick: the books give `mod` no result type).
            if op == Div { Value::some(v) } else { v }
        }
        SubQ => {
            let both_whole = !x.is_negative() && !y.is_negative();
            let d = x - y;
            if sized {
                if d.is_negative() { Value::nil() } else { Value::some(wrap_u8(&d)) }
            } else if matches!(a, Value::Nat(_)) || both_whole {
                // Pick NQ: two non-negative integers subtract as `Nat`s. An `Int` that is
                // not negative is, in compiler zero, nearly always a literal a checker
                // would have typed `Nat` (`{"Emma": 0}` reaching `$count -? 1`).
                if d.is_negative() { Value::nil() } else { Value::some(Value::Nat(d)) }
            } else {
                Value::some(Value::Int(d))
            }
        }
        AddQ | MulQ => {
            let r = if op == AddQ { x + y } else { x * y };
            if sized {
                if r > BigInt::from(255) || r.is_negative() { Value::nil() } else { Value::some(wrap_u8(&r)) }
            } else {
                Value::some(combine_int(a, b, r))
            }
        }
        _ => unreachable!(),
    })
}

fn floor_div(x: &BigInt, y: &BigInt) -> (BigInt, BigInt) {
    let q = x / y;
    let r = x - &q * y;
    if !r.is_zero() && (r.is_negative() != y.is_negative()) { (q - 1, r + y) } else { (q, r) }
}

/// Prefix `-`.
pub fn negate(v: &Value) -> R<Value> {
    if let Some(f) = as_float(v) {
        return Ok(float(-f));
    }
    match v {
        Value::Nat(n) | Value::Int(n) => Ok(Value::Int(-n)),
        Value::U8(n) => Ok(wrap_u8(&-BigInt::from(*n))),
        other => Err(type_err(format!("`-` needs a number, and was given {}", show_plain(other)))),
    }
}

/// Structural equality (`^Eq`, derived): what `==` means for every type with no `impl`.
pub fn equal(a: &Value, b: &Value) -> bool {
    match (num_f64(a), num_f64(b)) {
        (Some(x), Some(y)) if as_float(a).is_some() || as_float(b).is_some() => prism_rt::value::float_eq(x, y),
        _ => a == b,
    }
}

/// `^Ord`, derived: numbers by value, text by code point, records field by field,
/// choices by their alternative's place in the declaration and then payload.
pub fn compare(l: &Loaded, a: &Value, b: &Value) -> Ordering {
    if as_float(a).is_some() || as_float(b).is_some() {
        if let (Some(x), Some(y)) = (num_f64(a), num_f64(b)) {
            // IEEE's total order (Type System ch. 2).
            return x.total_cmp(&y);
        }
    }
    match (a, b) {
        (Value::Choice(x), Value::Choice(y)) => {
            let ix = l.ctors.get(&x.ctor).map(|c| (c.type_name.clone(), c.index));
            let iy = l.ctors.get(&y.ctor).map(|c| (c.type_name.clone(), c.index));
            let head = match (ix, iy) {
                (Some((tx, i)), Some((ty, j))) if tx == ty => i.cmp(&j),
                _ => x.ctor.cmp(&y.ctor),
            };
            head.then_with(|| compare_lists(l, &x.payload, &y.payload))
        }
        (Value::Record(x), Value::Record(y)) if x.name == y.name => {
            for ((_, p), (_, q)) in x.fields.iter().zip(y.fields.iter()) {
                let c = compare(l, p, q);
                if c != Ordering::Equal {
                    return c;
                }
            }
            Ordering::Equal
        }
        (Value::Tuple(x), Value::Tuple(y)) => compare_lists(l, x, y),
        (Value::Array(x), Value::Array(y)) => {
            for (p, q) in x.iter().zip(y.iter()) {
                let c = compare(l, p, q);
                if c != Ordering::Equal {
                    return c;
                }
            }
            x.len().cmp(&y.len())
        }
        (Value::Opt(Some(x)), Value::Opt(Some(y))) => compare(l, x, y),
        (Value::Fallible(Ok(x)), Value::Fallible(Ok(y))) => compare(l, x, y),
        _ => a.cmp(b),
    }
}

fn compare_lists(l: &Loaded, x: &[Value], y: &[Value]) -> Ordering {
    for (p, q) in x.iter().zip(y.iter()) {
        let c = compare(l, p, q);
        if c != Ordering::Equal {
            return c;
        }
    }
    x.len().cmp(&y.len())
}

/// The transcript's spelling of a value (Type System ch. 2, "Plain values, and how they
/// print"): the structural `Show` that `derive ^Show` promises.
pub fn show_plain(v: &Value) -> String {
    prism_rt::pod::print(v)
}

/// The value of `Compare`: `Less`, `Equal` or `Greater`.
pub fn ordering_value(o: Ordering) -> Value {
    Value::ctor(
        match o {
            Ordering::Less => "Less",
            Ordering::Equal => "Equal",
            Ordering::Greater => "Greater",
        },
        vec![],
    )
}

/// The runtime type of a value, as an `impl` header names it: a record's or a choice's
/// declared type, or a primitive's name; `[]` for arrays, `==>` for maps.
pub fn type_name(l: &Loaded, v: &Value) -> Rc<str> {
    match v {
        Value::Unit => "unit".into(),
        Value::Bool(_) => "Bool".into(),
        Value::Nat(_) => "Nat".into(),
        Value::Int(_) => "Int".into(),
        Value::U8(_) => "U8".into(),
        Value::Char(_) => "Char".into(),
        Value::Text(_) => "Text".into(),
        Value::Duration(_) => "Duration".into(),
        v if as_float(v).is_some() => "Float".into(),
        Value::Record(r) => l.ctors.get(&r.name).map(|c| c.type_name.clone()).unwrap_or_else(|| r.name.clone()),
        Value::Choice(c) => l.ctors.get(&c.ctor).map(|c| c.type_name.clone()).unwrap_or_else(|| c.ctor.clone()),
        Value::Tuple(_) => "(())".into(),
        Value::Map(_) => "==>".into(),
        Value::Array(_) => "[]".into(),
        Value::Opt(_) => "?".into(),
        Value::Fallible(_) => "~".into(),
        Value::Element(e) => e.tag.clone(),
        Value::Path(_) => "Path".into(),
        _ => "<handle>".into(),
    }
}

// ---------------------------------------------------------------------------------------
// Float (decisions.md D14)
// ---------------------------------------------------------------------------------------

/// A `Float` value: prism_rt's `Value::Float`, which prints and reads back in Type System
/// ch. 2's spelling.
pub fn float(f: f64) -> Value {
    Value::Float(f)
}

/// The float inside a float value.
pub fn as_float(v: &Value) -> Option<f64> {
    v.as_float()
}

/// Any number as an `f64`: a literal `2` where a `Float` is wanted is a `Float` (a
/// literal is chosen by its context, Type System ch. 2).
pub fn num_f64(v: &Value) -> Option<f64> {
    as_float(v).or_else(|| int_of(v).and_then(|n| n.to_f64()))
}

fn float_arith(op: BinOp, a: &Value, b: &Value) -> R<Value> {
    use prism_rt::value::{float_op, FloatOp};
    use BinOp::*;
    let (Some(x), Some(y)) = (num_f64(a), num_f64(b)) else {
        return Err(type_err(format!("`{}` needs two numbers, and was given {} and {}", op.as_str(), show_plain(a), show_plain(b))));
    };
    let f = |o: FloatOp| float(float_op(o, x, y));
    Ok(match op {
        Add => f(FloatOp::Add),
        Sub => f(FloatOp::Sub),
        Mul => f(FloatOp::Mul),
        // IEEE division never fails (Semantics ch. 10, "Total operations").
        Div => f(FloatOp::Div),
        Mod => f(FloatOp::Mod),
        AddQ => Value::some(f(FloatOp::Add)),
        SubQ => Value::some(f(FloatOp::Sub)),
        MulQ => Value::some(f(FloatOp::Mul)),
        _ => unreachable!(),
    })
}
