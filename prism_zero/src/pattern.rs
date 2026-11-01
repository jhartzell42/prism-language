//! Pattern matching (Type System ch. 2, "Patterns and exhaustiveness"; Semantics ch. 4).
//!
//! The forms: literals, names (a binder with its stack, or a constructor with no
//! arguments), `ignore`, `unit`, `nil`, constructors applied by commas, records and tags
//! by named fields with `..`, `? p` and `~ p`, tuples, arrays, groups, family patterns
//! `Ui.*`. Exhaustiveness is the checker's (D3); a `match` with no arm that matches is an
//! error when it runs.
//!
//! A name in a pattern is a constructor when it resolves to one (or to a declared unit
//! value such as `click`), else a binder. Each form's test on the value is
//! `prism_dyn::pat`'s, which compiled code shares; this module walks the tree, resolving
//! names in the module.

use std::rc::Rc;

use prism_dyn::pat;
use prism_rt::Value;
use prism_syntax::ast::*;

use crate::interp::{Interp, Loc};
use crate::load::ModId;
use crate::ops::R;

pub use prism_dyn::pat::parse_int;

/// Match `v` against `p`, pushing the bindings. Bindings are by the binder's full name,
/// sigils included (`$count`, `!e`): sigils are part of names (Type System ch. 1).
pub fn matches(interp: &Interp, module: ModId, p: &Pattern, v: &Value, binds: &mut Vec<(Rc<str>, Value)>) -> R<bool> {
    let loc = Loc { module, span: p.span };
    Ok(match &p.kind {
        PatternKind::Ignore => true,
        PatternKind::Unit => pat::is_unit(v),
        PatternKind::Nil => pat::is_nil(v),
        PatternKind::Lit(l) => lit_matches(interp, loc, l, v, false)?,
        PatternKind::Neg(l) => lit_matches(interp, loc, l, v, true)?,
        PatternKind::Group(inner) => matches(interp, module, inner, v, binds)?,
        PatternKind::Name(n) => {
            if n.path.len() > 1 {
                // A qualified name: a fault kind or a tag with no fields, or a module's
                // constructor.
                let full = n.path.join(".");
                if interp.tag(&full).is_some() {
                    return Ok(pat::is_tag(v, &full));
                }
                return Ok(pat::is_ctor(v, n.base()));
            }
            let base = n.base();
            match base {
                "True" => return Ok(pat::is_bool(v, true)),
                "False" => return Ok(pat::is_bool(v, false)),
                _ => {}
            }
            // A constructor when the name resolves to one; an uppercase name that does
            // not is a binder for a function value (`fn Fold xs, z, F`), as a lowercase
            // one is for a plain value.
            if interp.is_ctor(module, base) || interp.tag(base).is_some() {
                pat::is_ctor(v, base)
            } else {
                binds.push((Rc::from(n.text()), v.clone()));
                true
            }
        }
        PatternKind::Ctor { name, args } => {
            let full = if name.path.len() > 1 { Some(name.path.join(".")) } else { None };
            match pat::ctor(v, name.base(), full.as_deref(), args.len()) {
                Some(parts) => all(interp, module, args.iter(), &parts, binds)?,
                None => false,
            }
        }
        PatternKind::Record { name, fields, rest } => {
            let full = name.path.join(".");
            let names: Vec<Option<&str>> = fields.iter().map(|f| f.name.as_ref().map(|n| n.name.as_str())).collect();
            match pat::record(v, &full, name.base(), &names, *rest) {
                Some(parts) => all(interp, module, fields.iter().map(|f| &f.pattern), &parts, binds)?,
                None => false,
            }
        }
        PatternKind::Sigil { sigil, inner } => {
            let x = match sigil {
                Sigil::Question => pat::some(v),
                Sigil::Tilde => pat::ok(v),
                _ => None,
            };
            match x {
                Some(x) => matches(interp, module, inner, &x, binds)?,
                None => false,
            }
        }
        PatternKind::Tuple(ps) => match pat::tuple(v, ps.len()) {
            Some(parts) => all(interp, module, ps.iter(), &parts, binds)?,
            None => false,
        },
        PatternKind::Array(ps) => match pat::array(v, ps.len()) {
            Some(parts) => all(interp, module, ps.iter(), &parts, binds)?,
            None => false,
        },
        PatternKind::Family(q) => pat::family(v, &q.path.join(".")),
    })
}

/// Every sub-pattern against its part.
fn all<'p>(interp: &Interp, module: ModId, ps: impl Iterator<Item = &'p Pattern>, parts: &[Value], binds: &mut Vec<(Rc<str>, Value)>) -> R<bool> {
    for (p, x) in ps.zip(parts.iter()) {
        if !matches(interp, module, p, x, binds)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn lit_matches(interp: &Interp, loc: Loc, l: &Literal, v: &Value, neg: bool) -> R<bool> {
    Ok(match &l.kind {
        LitKind::Int(s) => {
            let n = parse_int(s).ok_or_else(|| interp.fault(loc, format!("{s} is not an integer")))?;
            pat::int(v, &if neg { -n } else { n })
        }
        LitKind::Char(c) => pat::char(v, *c),
        LitKind::Text(segs) => {
            let mut s = String::new();
            for seg in segs {
                match seg {
                    TextSeg::Plain(p) => s.push_str(p),
                    TextSeg::Placeholder(_) => return Err(interp.fault(loc, "a pattern's text has no placeholders")),
                }
            }
            pat::text(v, &s)
        }
        LitKind::Float(s) => {
            let f: f64 = s.parse().map_err(|_| interp.fault(loc, format!("{s} is not a number")))?;
            pat::float(v, if neg { -f } else { f })
        }
        LitKind::Duration { .. } => false,
    })
}
