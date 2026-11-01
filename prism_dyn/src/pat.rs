//! Pattern matching on values (Type System ch. 2, "Patterns and exhaustiveness";
//! Semantics ch. 4): one test per pattern form. A matcher (compiler zero's walk over its
//! tree, or the code `prismc` emits for a pattern) composes them; names were resolved
//! before (a constructor or a binder), so each test is about the value alone.
//!
//! `Error(..)` matches a failed `~T`; since one value stands for both, it matches an
//! `Error` record too.

use num_bigint::BigInt;
use prism_rt::Value;

use crate::ops;

/// `True`, `False`.
pub fn is_bool(v: &Value, b: bool) -> bool {
    matches!(v, Value::Bool(x) if *x == b)
}

pub fn is_unit(v: &Value) -> bool {
    matches!(v, Value::Unit)
}

pub fn is_nil(v: &Value) -> bool {
    matches!(v, Value::Opt(None))
}

/// A constructor with no arguments, by its name.
pub fn is_ctor(v: &Value, name: &str) -> bool {
    match v {
        Value::Choice(c) => &*c.ctor == name && c.payload.is_empty(),
        Value::Record(r) => &*r.name == name && r.fields.is_empty(),
        _ => false,
    }
}

/// A qualified name that is a tag: a fault kind's value, or an element of the tag.
pub fn is_tag(v: &Value, full: &str) -> bool {
    match v {
        Value::Choice(c) => &*c.ctor == full,
        Value::Element(e) => &*e.tag == full,
        _ => false,
    }
}

/// `Name p1, .., pn`: a choice's payload, or a record taken apart positionally
/// (`Vector x, y`). `full` is the qualified spelling when the pattern wrote one.
pub fn ctor(v: &Value, base: &str, full: Option<&str>, n: usize) -> Option<Vec<Value>> {
    match v {
        Value::Choice(c) if &*c.ctor == base || full.is_some_and(|f| *c.ctor == *f) => {
            if c.payload.len() != n {
                return None;
            }
            Some(c.payload.clone())
        }
        Value::Record(r) if &*r.name == base => {
            if r.fields.len() != n {
                return None;
            }
            Some(r.fields.iter().map(|(_, x)| x.clone()).collect())
        }
        _ => None,
    }
}

/// `Name(f=p, ..)`: a record's, an element's or (with every field unnamed) a choice's
/// parts for the field patterns given, in their order; `None` when the value is not
/// one or a named field is missing.
pub fn record(v: &Value, full: &str, base: &str, names: &[Option<&str>], rest: bool) -> Option<Vec<Value>> {
    match v {
        Value::Record(r) if *r.name == *full || &*r.name == base => fields(names, rest, &r.fields),
        Value::Fallible(Err(e)) if full == "Error" => record(e, full, base, names, rest),
        Value::Element(e) if *e.tag == *full => fields(names, rest, &e.fields),
        // `Rect(w, h)`: a choice with its payload in parentheses.
        Value::Choice(c) if &*c.ctor == base && names.iter().all(|n| n.is_none()) => {
            if c.payload.len() != names.len() && !rest {
                return None;
            }
            if c.payload.len() < names.len() {
                return None;
            }
            Some(c.payload[..names.len()].to_vec())
        }
        _ => None,
    }
}

fn fields(names: &[Option<&str>], rest: bool, fs: &[(std::rc::Rc<str>, Value)]) -> Option<Vec<Value>> {
    if !rest && names.len() != fs.len() {
        return None;
    }
    let mut out = Vec::with_capacity(names.len());
    for (i, n) in names.iter().enumerate() {
        let target = match n {
            Some(n) => fs.iter().find(|(m, _)| &**m == *n).map(|(_, v)| v),
            None => fs.get(i).map(|(_, v)| v),
        };
        out.push(target?.clone());
    }
    Some(out)
}

/// `? p`.
pub fn some(v: &Value) -> Option<Value> {
    match v {
        Value::Opt(Some(x)) => Some((**x).clone()),
        _ => None,
    }
}

/// `~ p`.
pub fn ok(v: &Value) -> Option<Value> {
    match v {
        Value::Fallible(Ok(x)) => Some((**x).clone()),
        _ => None,
    }
}

/// `((p1, .., pn))`.
pub fn tuple(v: &Value, n: usize) -> Option<Vec<Value>> {
    match v {
        Value::Tuple(vs) if vs.len() == n => Some((**vs).clone()),
        _ => None,
    }
}

/// `[[p1, .., pn]]`.
pub fn array(v: &Value, n: usize) -> Option<Vec<Value>> {
    match v {
        Value::Array(vs) if vs.len() == n => Some(vs.iter().cloned().collect()),
        _ => None,
    }
}

/// `Ui.*`: an element of the family, or a fault kind of it.
pub fn family(v: &Value, fam: &str) -> bool {
    match v {
        Value::Element(e) => e.family() == fam,
        Value::Choice(c) => prism_rt::value::family_of(&c.ctor) == fam,
        _ => false,
    }
}

/// An integer literal.
pub fn int(v: &Value, n: &BigInt) -> bool {
    ops::int_of(v).as_ref() == Some(n)
}

/// A small integer literal, as compiled code writes one.
pub fn small(v: &Value, n: i64) -> bool {
    ops::int_of(v) == Some(BigInt::from(n))
}

pub fn char(v: &Value, c: char) -> bool {
    matches!(v, Value::Char(x) if *x == c)
}

pub fn text(v: &Value, s: &str) -> bool {
    match v {
        Value::Text(t) => t.len() == s.chars().count() && t.to_string() == s,
        _ => false,
    }
}

pub fn float(v: &Value, f: f64) -> bool {
    ops::num_f64(v) == Some(f) && ops::as_float(v).is_some()
}

/// An integer literal as written: digits with `_`, `0x`, `0b`, `0o`.
pub fn parse_int(s: &str) -> Option<BigInt> {
    let s: String = s.chars().filter(|c| *c != '_').collect();
    let (digits, radix) = if let Some(h) = s.strip_prefix("0x") {
        (h.to_string(), 16)
    } else if let Some(b) = s.strip_prefix("0b") {
        (b.to_string(), 2)
    } else if let Some(o) = s.strip_prefix("0o") {
        (o.to_string(), 8)
    } else {
        (s, 10)
    };
    BigInt::parse_bytes(digits.as_bytes(), radix)
}
