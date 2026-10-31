//! Pattern matching (Type System ch. 2, "Patterns and exhaustiveness"; Semantics ch. 4).
//!
//! The forms: literals, names (a binder with its stack, or a constructor with no
//! arguments), `ignore`, `unit`, `nil`, constructors applied by commas, records and tags
//! by named fields with `..`, `? p` and `~ p`, tuples, arrays, groups, family patterns
//! `Ui.*`. Exhaustiveness is the checker's (D3); a `match` with no arm that matches is an
//! error when it runs.
//!
//! A name in a pattern is a constructor when it is uppercase (or a reserved lowercase
//! constructor, or a declared unit value such as `click`), else a binder. `Error(..)`
//! matches a failed `~T`; since compiler zero keeps one value for both, it matches an
//! `Error` record too.

use std::rc::Rc;

use num_bigint::BigInt;
use prism_rt::Value;
use prism_syntax::ast::*;

use crate::interp::{Interp, Loc};
use crate::load::ModId;
use crate::ops::{self, R};

/// Match `v` against `p`, pushing the bindings. Bindings are by the binder's full name,
/// sigils included (`$count`, `!e`): sigils are part of names (Type System ch. 1).
pub fn matches(interp: &Interp, module: ModId, p: &Pattern, v: &Value, binds: &mut Vec<(Rc<str>, Value)>) -> R<bool> {
    let loc = Loc { module, span: p.span };
    Ok(match &p.kind {
        PatternKind::Ignore => true,
        PatternKind::Unit => matches!(v, Value::Unit),
        PatternKind::Nil => matches!(v, Value::Opt(None)),
        PatternKind::Lit(l) => lit_matches(interp, loc, l, v, false)?,
        PatternKind::Neg(l) => lit_matches(interp, loc, l, v, true)?,
        PatternKind::Group(inner) => matches(interp, module, inner, v, binds)?,
        PatternKind::Name(n) => {
            if n.path.len() > 1 {
                // A qualified name: a fault kind or a tag with no fields, or a module's
                // constructor.
                let full = n.path.join(".");
                if interp.tag(&full).is_some() {
                    return Ok(match v {
                        Value::Choice(c) => &*c.ctor == full,
                        Value::Element(e) => &*e.tag == full,
                        _ => false,
                    });
                }
                return Ok(ctor_name_matches(n.base(), v));
            }
            let base = n.base();
            match base {
                "True" => return Ok(matches!(v, Value::Bool(true))),
                "False" => return Ok(matches!(v, Value::Bool(false))),
                _ => {}
            }
            // A constructor when the name resolves to one; an uppercase name that does
            // not is a binder for a function value (`fn Fold xs, z, F`), as a lowercase
            // one is for a plain value.
            if interp.is_ctor(module, base) || interp.tag(base).is_some() {
                ctor_name_matches(base, v)
            } else {
                binds.push((Rc::from(n.text()), v.clone()));
                true
            }
        }
        PatternKind::Ctor { name, args } => {
            let base = name.base();
            match v {
                Value::Choice(c) if &*c.ctor == base || (name.path.len() > 1 && *c.ctor == *name.path.join(".")) => {
                    if c.payload.len() != args.len() {
                        return Ok(false);
                    }
                    for (a, x) in args.iter().zip(c.payload.iter()) {
                        if !matches(interp, module, a, x, binds)? {
                            return Ok(false);
                        }
                    }
                    true
                }
                // A record taken apart positionally, `Vector x, y`.
                Value::Record(r) if &*r.name == base => {
                    if r.fields.len() != args.len() {
                        return Ok(false);
                    }
                    for (a, (_, x)) in args.iter().zip(r.fields.iter()) {
                        if !matches(interp, module, a, x, binds)? {
                            return Ok(false);
                        }
                    }
                    true
                }
                _ => false,
            }
        }
        PatternKind::Record { name, fields, rest } => {
            let full = name.path.join(".");
            let target: &[(Rc<str>, Value)];
            let holder;
            match v {
                Value::Record(r) if *r.name == *full || &*r.name == name.base() => {
                    holder = r.clone();
                    target = &holder.fields;
                }
                Value::Fallible(Err(e)) if full == "Error" => return matches(interp, module, p, e, binds),
                Value::Element(e) if *e.tag == *full => {
                    let el = e.clone();
                    return fields_match(interp, module, fields, *rest, &el.fields, binds);
                }
                // `Rect(w, h)`: a choice with its payload in parentheses.
                Value::Choice(c) if &*c.ctor == name.base() && fields.iter().all(|f| f.name.is_none()) => {
                    if c.payload.len() != fields.len() && !rest {
                        return Ok(false);
                    }
                    for (f, x) in fields.iter().zip(c.payload.iter()) {
                        if !matches(interp, module, &f.pattern, x, binds)? {
                            return Ok(false);
                        }
                    }
                    return Ok(true);
                }
                _ => return Ok(false),
            }
            fields_match(interp, module, fields, *rest, target, binds)?
        }
        PatternKind::Sigil { sigil, inner } => match (sigil, v) {
            (Sigil::Question, Value::Opt(Some(x))) => matches(interp, module, inner, x, binds)?,
            (Sigil::Tilde, Value::Fallible(Ok(x))) => matches(interp, module, inner, x, binds)?,
            _ => false,
        },
        PatternKind::Tuple(ps) => match v {
            Value::Tuple(vs) if vs.len() == ps.len() => {
                for (a, x) in ps.iter().zip(vs.iter()) {
                    if !matches(interp, module, a, x, binds)? {
                        return Ok(false);
                    }
                }
                true
            }
            _ => false,
        },
        PatternKind::Array(ps) => match v {
            Value::Array(vs) if vs.len() == ps.len() => {
                for (a, x) in ps.iter().zip(vs.iter()) {
                    if !matches(interp, module, a, x, binds)? {
                        return Ok(false);
                    }
                }
                true
            }
            _ => false,
        },
        PatternKind::Family(q) => {
            let f = q.path.join(".");
            match v {
                Value::Element(e) => e.family() == f,
                Value::Choice(c) => prism_rt::value::family_of(&c.ctor) == f,
                _ => false,
            }
        }
    })
}

fn fields_match(
    interp: &Interp,
    module: ModId,
    ps: &[FieldPat],
    rest: bool,
    fs: &[(Rc<str>, Value)],
    binds: &mut Vec<(Rc<str>, Value)>,
) -> R<bool> {
    if !rest && ps.len() != fs.len() {
        return Ok(false);
    }
    for (i, fp) in ps.iter().enumerate() {
        let target = match &fp.name {
            Some(n) => fs.iter().find(|(m, _)| **m == *n.name).map(|(_, v)| v),
            None => fs.get(i).map(|(_, v)| v),
        };
        match target {
            Some(t) => {
                if !matches(interp, module, &fp.pattern, t, binds)? {
                    return Ok(false);
                }
            }
            None => return Ok(false),
        }
    }
    Ok(true)
}

fn ctor_name_matches(name: &str, v: &Value) -> bool {
    match v {
        Value::Choice(c) => &*c.ctor == name && c.payload.is_empty(),
        Value::Record(r) => &*r.name == name && r.fields.is_empty(),
        _ => false,
    }
}


fn lit_matches(interp: &Interp, loc: Loc, l: &Literal, v: &Value, neg: bool) -> R<bool> {
    Ok(match &l.kind {
        LitKind::Int(s) => {
            let n = parse_int(s).ok_or_else(|| interp.fault(loc, format!("{s} is not an integer")))?;
            let n = if neg { -n } else { n };
            ops::int_of(v) == Some(n)
        }
        LitKind::Char(c) => matches!(v, Value::Char(x) if x == c),
        LitKind::Text(segs) => {
            let mut s = String::new();
            for seg in segs {
                match seg {
                    TextSeg::Plain(p) => s.push_str(p),
                    TextSeg::Placeholder(_) => return Err(interp.fault(loc, "a pattern's text has no placeholders")),
                }
            }
            matches!(v, Value::Text(t) if t.to_string() == s)
        }
        LitKind::Float(s) => {
            let f: f64 = s.parse().map_err(|_| interp.fault(loc, format!("{s} is not a number")))?;
            let f = if neg { -f } else { f };
            ops::num_f64(v) == Some(f) && ops::as_float(v).is_some()
        }
        LitKind::Duration { .. } => false,
    })
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
