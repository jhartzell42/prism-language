//! The prelude functions written in Rust: each is registered by the name its signature
//! has in `prelude/prelude.prism` (a signature with no clauses), and lifted by that
//! signature like any function. The cost promise of each is Semantics Appendix B's.
//!
//! Three groups:
//!
//! - **Plain functions** over arrays, maps, texts and numbers: called with plain
//!   arguments (the interpreter lifts them first).
//! - **Event combinators** (`!Filter`, `!Present`, `!Union`, `!MergeAll`, `!MergeWith`,
//!   `!Both`, `!Except`, `!Gate`, `!Updated`): pure nodes, so each needs the build it
//!   is applied in; constant per firing.
//! - **Widget bindings** (`@started`, `@Element`, the host documents, `@Switch`,
//!   `@Flatten`, `@Defer`, `@Later`, `@Machine`, `@Run`): applied, each is a blueprint of
//!   its arguments, and binding it calls the `prism_rt` `Build` method of the same name
//!   (prism_rt API.md). Each is set up once per widget instance.

use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};
use prism_rt::{Blueprint, Build, Closure, Element, Fault, Value};

use crate::interp::{Bx, Interp, Loc, CYCLE_BUDGET, UNBOUNDED};
use crate::ops::{self, R};

/// Builtins reached by a qualified name, with no signature line (Semantics ch. 5's
/// block-local types).
pub fn is_qualified_builtin(name: &str) -> bool {
    matches!(name, "Buffer.Numbered" | "Buffer.Empty" | "Table.Empty")
}

fn arg(args: &[Value], i: usize) -> &Value {
    const UNIT: &Value = &Value::Unit;
    args.get(i).unwrap_or(UNIT)
}

/// Call the builtin `name`.
pub fn call(interp: &Interp, name: &str, args: &[Value], b: Bx, loc: Loc) -> R<Value> {
    let bad = |what: &str| interp.fault(loc, format!("{name}: {what}"));
    let a0 = arg(args, 0);
    let a1 = arg(args, 1);
    Ok(match name {
        // ----- arrays, texts and maps (Appendix B) -----
        // Constant.
        "Len" => Value::nat(match a0 {
            Value::Array(a) => a.len(),
            Value::Text(t) => t.len(),
            Value::Map(m) => m.len(),
            Value::Record(r) if &*r.name == "Table" => table_entries(a0).len(),
            _ => return Err(bad("takes an array, a text or a map")),
        } as u64),
        // Logarithmic.
        "Append" => match (a0, a1) {
            (Value::Array(x), Value::Array(y)) => {
                let mut x = x.clone();
                x.append(y.clone());
                Value::Array(x)
            }
            _ => return Err(bad("joins two arrays")),
        },
        // Amortised constant.
        "Push" => match a0 {
            Value::Array(x) => {
                let mut x = x.clone();
                x.push_back(a1.clone());
                Value::Array(x)
            }
            _ => return Err(bad("pushes onto an array")),
        },
        // `n·t`, the closure's color taken through (D8: bounded in, bounded out).
        "Fold" => {
            let f = arg(args, 2);
            let three = interp.layers_of(f).map(|(_, l)| l.len() == 3).unwrap_or(false);
            let mut acc = a1.clone();
            match a0 {
                Value::Array(xs) => {
                    for x in xs.iter() {
                        acc = interp.apply(f, vec![acc, x.clone()], None, loc)?;
                    }
                }
                Value::Map(m) => {
                    for (k, v) in m.iter() {
                        let args = if three { vec![acc, k.clone(), v.clone()] } else { vec![acc, Value::tuple(vec![k.clone(), v.clone()])] };
                        acc = interp.apply(f, args, None, loc)?;
                    }
                }
                _ => return Err(bad("folds an array or a map")),
            }
            acc
        }
        // Linear in the total length.
        "Concat" => match a0 {
            Value::Array(xs) => {
                let mut out = prism_rt::Text::default();
                for x in xs.iter() {
                    match x {
                        Value::Text(t) => out = out.concat(t),
                        _ => return Err(bad("joins texts")),
                    }
                }
                Value::Text(out)
            }
            _ => return Err(bad("joins an array of texts")),
        },
        // Maps: logarithmic, keys ordered (Appendix B, Maps).
        "Insert" => match a0 {
            Value::Map(m) => Value::Map(m.update(a1.clone(), arg(args, 2).clone())),
            _ => return Err(bad("inserts into a map")),
        },
        "Remove" => match a0 {
            Value::Map(m) => Value::Map(m.without(a1)),
            _ => return Err(bad("removes from a map")),
        },
        "?Get" => interp.index(a0, a1, loc)?,
        "?First" => match a0 {
            Value::Map(m) => m.get_min().map(|(k, v)| Value::some(Value::tuple(vec![k.clone(), v.clone()]))).unwrap_or(Value::nil()),
            _ => return Err(bad("takes a map")),
        },
        "Keys" => match a0 {
            Value::Map(m) => Value::Array(m.keys().cloned().collect()),
            _ => return Err(bad("takes a map")),
        },
        "Values" => match a0 {
            Value::Map(m) => Value::Array(m.values().cloned().collect()),
            _ => return Err(bad("takes a map")),
        },
        "Size" => match a0 {
            Value::Map(m) => Value::nat(m.len() as u64),
            _ => return Err(bad("takes a map")),
        },
        // `n` log `n`, the later pair winning a shared key.
        "Map" => match a0 {
            Value::Array(xs) => {
                let mut m = im_rc::OrdMap::new();
                for x in xs.iter() {
                    match x {
                        Value::Tuple(t) if t.len() == 2 => {
                            m.insert(t[0].clone(), t[1].clone());
                        }
                        _ => return Err(bad("takes an array of pairs")),
                    }
                }
                Value::Map(m)
            }
            _ => return Err(bad("takes an array of pairs")),
        },
        // The left winning; `m` log `(n+m)`.
        "Union" => match (a0, a1) {
            (Value::Map(x), Value::Map(y)) => Value::Map(x.clone().union(y.clone())),
            _ => return Err(bad("joins two maps")),
        },
        // Logarithmic, by a rope.
        "Take" | "Drop" => {
            let n = ops::index_of(a1).ok_or_else(|| bad("takes a count"))?;
            match a0 {
                Value::Text(t) => Value::Text(if name == "Take" { t.take(n) } else { t.drop_front(n) }),
                Value::Array(a) => {
                    let n = n.min(a.len());
                    Value::Array(if name == "Take" { a.clone().slice(..n) } else { a.clone().slice(n..) })
                }
                _ => return Err(bad("cuts a text or an array")),
            }
        }
        "Upper" | "Lower" => match a0 {
            Value::Text(t) => Value::text(&if name == "Upper" { t.to_string().to_uppercase() } else { t.to_string().to_lowercase() }),
            _ => return Err(bad("takes a text")),
        },
        "Split" => match (a0, a1) {
            (Value::Text(t), Value::Text(sep)) => {
                let (t, sep) = (t.to_string(), sep.to_string());
                if sep.is_empty() {
                    Value::array(t.chars().map(|c| Value::text(&c.to_string())).collect())
                } else {
                    Value::array(t.split(sep.as_str()).map(Value::text).collect())
                }
            }
            _ => return Err(bad("splits a text at a text")),
        },
        // Constant (TD-V1).
        "?Step" => {
            let text = interp.field(a0, "text", loc)?;
            let at = ops::index_of(&interp.field(a0, "at", loc)?).unwrap_or(0);
            match text.as_text().and_then(|t| t.char_at(at)) {
                Some(c) => Value::some(Value::tuple(vec![
                    Value::Char(c),
                    Value::record("Cursor", vec![("text", text.clone()), ("at", Value::nat(at as u64 + 1))]),
                ])),
                None => Value::nil(),
            }
        }
        "?ParseNat" => match a0 {
            Value::Text(t) => {
                let s = t.to_string();
                if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) {
                    Value::some(Value::Nat(BigInt::parse_bytes(s.as_bytes(), 10).unwrap()))
                } else {
                    Value::nil()
                }
            }
            _ => return Err(bad("takes a text")),
        },
        // The POD printed form: one top-level node per line (Type System ch. 2).
        "Print" => match a0 {
            Value::Array(xs) => {
                let mut out = String::new();
                for x in xs.iter() {
                    out.push_str(&ops::show_plain(x));
                    out.push('\n');
                }
                Value::text(&out)
            }
            _ => return Err(bad("prints an array of documents")),
        },
        // ----- numbers -----
        "Int" => match ops::int_of(a0) {
            Some(n) => Value::Int(n),
            None => return Err(bad("widens an integer")),
        },
        "?Nat" => match ops::int_of(a0) {
            Some(n) if !n.is_negative() => Value::some(Value::Nat(n)),
            Some(_) => Value::nil(),
            None => return Err(bad("narrows an integer")),
        },
        "?U8" => match ops::int_of(a0).and_then(|n| n.to_u8()) {
            Some(n) => Value::some(Value::U8(n)),
            None => Value::nil(),
        },
        "?Div" | "?Mod" => {
            if ops::as_float(a0).is_none() && ops::int_of(a1).is_some_and(|n| n.is_zero()) {
                Value::nil()
            } else {
                let op = if name == "?Div" { prism_syntax::ast::BinOp::Div } else { prism_syntax::ast::BinOp::Mod };
                let r = ops::binop(&interp.l, op, a0, a1)?;
                match r {
                    Value::Opt(_) => r,
                    other => Value::some(other),
                }
            }
        }
        // `Float` (D14).
        "Float" | "ToFloat" => match ops::num_f64(a0) {
            Some(f) => ops::float(f),
            None => return Err(bad("takes a number")),
        },
        "Cos" | "Sin" => match ops::num_f64(a0) {
            Some(f) => ops::float(if name == "Cos" { f.cos() } else { f.sin() }),
            None => return Err(bad("takes a Float")),
        },
        "Abs" | "Sign" => {
            if let Some(f) = ops::as_float(a0) {
                ops::float(if name == "Abs" { f.abs() } else if f > 0.0 { 1.0 } else if f < 0.0 { -1.0 } else { 0.0 })
            } else {
                match a0 {
                    Value::Int(n) => Value::Int(if name == "Abs" { n.abs() } else { n.signum() }),
                    Value::Nat(n) => Value::Nat(if name == "Abs" { n.clone() } else { n.signum() }),
                    Value::U8(n) => Value::U8(if name == "Abs" { *n } else { (*n > 0) as u8 }),
                    _ => return Err(bad("takes a number")),
                }
            }
        }
        // ----- `&` blocks (Semantics ch. 5) -----
        "Buffer.Numbered" => {
            let n = ops::index_of(a0).ok_or_else(|| bad("takes a count"))?;
            Value::array((0..n).map(|i| Value::nat(i as u64)).collect())
        }
        "Buffer.Empty" => Value::array(vec![]),
        "Table.Empty" => Value::record("Table", vec![("keys", Value::array(vec![])), ("map", Value::map(vec![]))]),
        // Constant: a `Buffer` is its array; a `Table` its pairs in insertion order.
        "Freeze" => match a0 {
            Value::Record(r) if &*r.name == "Table" => {
                Value::array(table_entries(a0).into_iter().map(|(k, v)| Value::tuple(vec![k, v])).collect())
            }
            other => other.clone(),
        },
        // ----- structural trait methods, reached directly -----
        "Show" | "Compare" => structural(interp, name, args, loc)?,
        // ----- events -----
        "!Filter" | "!Present" | "!Union" | "!MergeAll" | "!MergeWith" | "!Both" | "!Except" | "!Gate" | "!Updated" => {
            let b = b.ok_or_else(|| bad("makes a node, and there is no widget building here"))?;
            event_combinator(interp, name, args, b, loc)?
        }
        // ----- widgets -----
        n if n.starts_with('@') => {
            let b = b.ok_or_else(|| bad("is a widget binding, bound with `<-` in a widget block"))?;
            widget(interp, n, args, b, loc)?
        }
        _ => return Err(interp.fault(loc, format!("the prelude declares {name}, and prism_zero has no builtin of that name"))),
    })
}

/// The structural answers of the derivable traits (Type System ch. 3, "Derive, out
/// loud"): equality and order by structure, `Show` in the transcript's spelling.
pub fn structural(interp: &Interp, name: &str, args: &[Value], loc: Loc) -> R<Value> {
    let a0 = arg(args, 0);
    let a1 = arg(args, 1);
    Ok(match name {
        "Show" => Value::text(&ops::show_plain(a0)),
        "Compare" => ops::ordering_value(ops::compare(&interp.l, a0, a1)),
        "Children" => Value::array(vec![]),
        op => {
            let bin = match op {
                "==" => prism_syntax::ast::BinOp::Eq,
                "!=" => prism_syntax::ast::BinOp::NotEq,
                "<" => prism_syntax::ast::BinOp::Lt,
                "<=" => prism_syntax::ast::BinOp::Le,
                ">" => prism_syntax::ast::BinOp::Gt,
                ">=" => prism_syntax::ast::BinOp::Ge,
                "+" => prism_syntax::ast::BinOp::Add,
                "-" => prism_syntax::ast::BinOp::Sub,
                "*" => prism_syntax::ast::BinOp::Mul,
                "/" => prism_syntax::ast::BinOp::Div,
                "+?" => prism_syntax::ast::BinOp::AddQ,
                "-?" => prism_syntax::ast::BinOp::SubQ,
                "*?" => prism_syntax::ast::BinOp::MulQ,
                _ => return Err(interp.fault(loc, format!("no structural {op}"))),
            };
            ops::binop(&interp.l, bin, a0, a1)?
        }
    })
}

fn event_combinator(interp: &Interp, name: &str, args: &[Value], b: &mut Build, loc: Loc) -> R<Value> {
    let a0 = arg(args, 0);
    let a1 = arg(args, 1);
    let events_of = |v: &Value| -> R<Vec<Value>> {
        match v {
            Value::Array(xs) => Ok(xs.iter().cloned().collect()),
            _ => Err(interp.fault(loc, format!("{name} takes an array of events"))),
        }
    };
    match name {
        "!Filter" => b.filter(a0),
        "!Present" => b.present(a0),
        "!Union" => b.union(a0, a1),
        "!Gate" => b.gate(a0, a1),
        "!Updated" => Ok(b.updated(a0)),
        // Every value that fired, in array order; linear in the array.
        "!MergeAll" => {
            let es = events_of(a0)?;
            b.combine(
                &es,
                Closure::pure("!MergeAll", |vs| {
                    let fired: Vec<Value> = vs.iter().filter_map(|v| if let Value::Opt(Some(x)) = v { Some((**x).clone()) } else { None }).collect();
                    Ok(if fired.is_empty() { Value::nil() } else { Value::some(Value::array(fired)) })
                }),
            )
        }
        // Coincident values folded with the function.
        "!MergeWith" => {
            let es = events_of(a1)?;
            let me = interp.weak();
            b.combine(
                &es,
                Closure::new("!MergeWith", vec![a0.clone()], move |cap, vs| {
                    let interp = me.upgrade().ok_or_else(|| Fault::runtime("the interpreter is gone"))?;
                    let mut acc: Option<Value> = None;
                    for v in vs {
                        if let Value::Opt(Some(x)) = v {
                            acc = Some(match acc {
                                None => (**x).clone(),
                                Some(a) => interp.apply(&cap[0], vec![a, (**x).clone()], None, loc)?,
                            });
                        }
                    }
                    Ok(acc.map(Value::some).unwrap_or(Value::nil()))
                }),
            )
        }
        "!Both" => b.combine(
            &[a0.clone(), a1.clone()],
            Closure::pure("!Both", |vs| match (&vs[0], &vs[1]) {
                (Value::Opt(Some(x)), Value::Opt(Some(y))) => Ok(Value::some(Value::tuple(vec![(**x).clone(), (**y).clone()]))),
                _ => Ok(Value::nil()),
            }),
        ),
        "!Except" => b.combine(
            &[a0.clone(), a1.clone()],
            Closure::pure("!Except", |vs| match (&vs[0], &vs[1]) {
                (Value::Opt(Some(x)), Value::Opt(None)) => Ok(Value::some((**x).clone())),
                _ => Ok(Value::nil()),
            }),
        ),
        _ => unreachable!(),
    }
}

fn blueprint_of(interp: &Interp, v: &Value, loc: Loc) -> R<Blueprint> {
    match v {
        Value::Widget(bp) => Ok(bp.clone()),
        other => Err(interp.fault(loc, format!("a blueprint was wanted, and {} is not one", ops::show_plain(other)))),
    }
}

fn closure_of(interp: &Interp, v: &Value, loc: Loc) -> R<Closure> {
    match v {
        Value::Closure(c) => Ok(c.clone()),
        other => Err(interp.fault(loc, format!("a function was wanted, and {} is not one", ops::show_plain(other)))),
    }
}

/// A supplier, `fn Path, Element c o -> @o`, as the blueprint `prism_rt` builds per
/// child place: apply it, and build the blueprint it gives.
fn supplier(interp: &Interp, f: &Value, loc: Loc) -> R<Blueprint> {
    let f = closure_of(interp, f, loc)?;
    let me = interp.weak();
    Ok(Blueprint::new("supplier", vec![Value::Closure(f)], move |b, cap, args| {
        let interp = me.upgrade().ok_or_else(|| Fault::runtime("the interpreter is gone"))?;
        let r = interp.apply(&cap[0], args.to_vec(), Some(&mut *b), loc)?;
        match r {
            Value::Widget(bp) => b.call(&bp, &[]),
            Value::Element(e) => b.emit((*e).clone()),
            other => Ok(other),
        }
    }))
}

fn widget(interp: &Interp, name: &str, args: &[Value], b: &mut Build, loc: Loc) -> R<Value> {
    let a0 = arg(args, 0);
    let a1 = arg(args, 1);
    let wrap = |f: Fault| interp.wrap_fault(loc, f);
    match name {
        "@unit" => Ok(Value::Unit),
        // Fires once, in the tick after the widget is built; takes a position.
        "@started" => Ok(b.started()),
        // The element statement as a function.
        "@Element" => match a0 {
            Value::Element(e) => b.emit((**e).clone()).map_err(wrap),
            other => Err(interp.fault(loc, format!("@Element takes an element, and {} is not one", ops::show_plain(other)))),
        },
        "@HostDocument" => {
            let child = blueprint_of(interp, a0, loc)?;
            let s = supplier(interp, a1, loc)?;
            b.host_document(&child, &[], &s).map_err(wrap)
        }
        "@HostSome" => {
            let child = blueprint_of(interp, a0, loc)?;
            let give = closure_of(interp, a1, loc)?;
            b.host_some(&child, &[], &give).map_err(wrap)
        }
        "@HostList" => {
            let child = blueprint_of(interp, a0, loc)?;
            let s = supplier(interp, arg(args, 2), loc)?;
            b.host_list(&child, &[], a1, &s).map_err(wrap)
        }
        "@Switch" => b.switch(a0).map_err(wrap),
        "@Flatten" => b.flatten(a0).map_err(wrap),
        "@Defer" => b.defer(a0).map_err(wrap),
        "@Later" => b.later(a0).map_err(wrap),
        "@Machine" => {
            let step = closure_of(interp, a0, loc)?;
            b.machine(step, a1.clone(), arg(args, 2)).map_err(wrap)
        }
        "@Run" => {
            let f = a0.clone();
            let x = a1.clone();
            let me = interp.weak();
            let cycle = Closure::new("@Run.cycle", vec![f, x], move |cap, job| {
                let interp = me.upgrade().ok_or_else(|| Fault::runtime("the interpreter is gone"))?;
                run_cycle(&interp, &cap[0], &cap[1], &job[0], loc)
            });
            // The job: cycles done, the result and its cycle-point count once known, and
            // the budget explored so far.
            let start = Value::tuple(vec![Value::nat(0), Value::nil(), Value::nat(0)]);
            b.run(cycle, start).map_err(wrap)
        }
        _ => Err(interp.fault(loc, format!("the prelude declares {name}, and prism_zero has no builtin of that name"))),
    }
}

/// One cycle of a `@Run` job (see `interp::Unbounded`): `Continue job` or `Done v`.
fn run_cycle(interp: &Interp, f: &Value, x: &Value, job: &Value, loc: Loc) -> R<Value> {
    let Value::Tuple(t) = job else { return Err(Fault::runtime("a job is a tuple")) };
    let done = ops::int_of(&t[0]).and_then(|n| n.to_u64()).unwrap_or(0) + 1;
    let explored = ops::int_of(&t[2]).and_then(|n| n.to_u64()).unwrap_or(0);
    let known = match &t[1] {
        Value::Opt(Some(k)) => Some((**k).clone()),
        _ => None,
    };
    let finish = |points: u64, result: Value| -> Value {
        // The function passes `points` cycle points, so it takes `points + 1` cycles.
        if done > points {
            Value::ctor("Done", vec![result])
        } else {
            let k = Value::tuple(vec![Value::nat(points), result]);
            Value::ctor("Continue", vec![Value::tuple(vec![Value::nat(done), Value::some(k), Value::nat(explored)])])
        }
    };
    if let Some(Value::Tuple(k)) = known {
        let points = ops::int_of(&k[0]).and_then(|n| n.to_u64()).unwrap_or(0);
        return Ok(finish(points, k[1].clone()));
    }
    let budget = (explored.max(512)).saturating_mul(2);
    let saved = UNBOUNDED.with(|u| (u.depth.get(), u.budget.get(), u.points.get()));
    UNBOUNDED.with(|u| {
        u.depth.set(0);
        u.budget.set(Some(budget));
        u.points.set(0);
    });
    let r = interp.apply(f, vec![x.clone()], None, loc);
    let points = UNBOUNDED.with(|u| u.points.get());
    UNBOUNDED.with(|u| {
        u.depth.set(saved.0);
        u.budget.set(saved.1);
        u.points.set(saved.2);
    });
    match r {
        Ok(v) => Ok(finish(points, v)),
        Err(f) if f.kind == CYCLE_BUDGET => Ok(Value::ctor(
            "Continue",
            vec![Value::tuple(vec![Value::nat(done), Value::nil(), Value::nat(budget)])],
        )),
        Err(f) => Err(f),
    }
}

// ----- `Table k v` (Semantics ch. 5): keys in insertion order, and a map -----

pub fn table_entries(t: &Value) -> Vec<(Value, Value)> {
    let (Some(Value::Array(keys)), Some(Value::Map(m))) = (t.field("keys"), t.field("map")) else { return vec![] };
    keys.iter().filter_map(|k| m.get(k).map(|v| (k.clone(), v.clone()))).collect()
}

pub fn table_get(t: &Value, k: &Value) -> R<Value> {
    match t.field("map") {
        Some(Value::Map(m)) => Ok(m.get(k).cloned().map(Value::some).unwrap_or(Value::nil())),
        _ => Err(Fault::runtime("a table has a map")),
    }
}

/// Set a table's entry: `? v` inserts (a new key goes last), `nil` removes.
pub fn table_set(t: Value, k: Value, v: Value) -> R<Value> {
    let (Some(Value::Array(keys)), Some(Value::Map(m))) = (t.field("keys").cloned(), t.field("map").cloned()) else {
        return Err(Fault::runtime("a table has keys and a map"));
    };
    let (mut keys, mut m) = (keys, m);
    match v {
        Value::Opt(None) => {
            m.remove(&k);
            keys.retain(|x| *x != k);
        }
        Value::Opt(Some(x)) => {
            if !m.contains_key(&k) {
                keys.push_back(k.clone());
            }
            m.insert(k, (*x).clone());
        }
        other => {
            if !m.contains_key(&k) {
                keys.push_back(k.clone());
            }
            m.insert(k, other);
        }
    }
    let _ = Rc::<str>::from("");
    let _ = Element::plain;
    Ok(Value::record("Table", vec![("keys", Value::Array(keys)), ("map", Value::Map(m))]))
}
