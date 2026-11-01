//! What the Rust that `prismc` emits calls, under short names (`use prism_dyn::rt::*;`).
//!
//! The emitted code is straight-line Rust over these functions: every Prism value is a
//! [`Value`], every application goes through [`apply`] (the lifting table on runtime
//! values, decisions.md D16), and every function is a plain Rust `fn` with its
//! environment passed in explicitly, so the collector sees every captured value
//! (decisions.md D5). The build a statement runs in, `b`, is always the last argument,
//! so calls nest: `apply(&f, vec![apply(&g, vec![], b)?], b)?`.
//!
//! The shapes of the emitted functions:
//!
//! - a function body, [`BodyFn`]: `fn(cap, args, b) -> R<Value>`;
//! - a widget block, [`BuildFn`]: `fn(b, cap, args) -> R<Value>`, with `b` always
//!   `Some`;
//! - the arms of a `match` or `if`, [`Arms`]: `fn(b, scrutinee, env) -> R<Value>`.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub use prism_rt::{Blueprint, Build, Closure, Element, Fault, FieldKind, OutputKind, Program, ResultKind, Row, TagDecl, Value};

pub use crate::func::{Bx, FnMeta, Layer};
pub use crate::lens::Step;
pub use crate::ops::{Op, R};
pub use crate::tables::CtorMeta;
pub use crate::values::{MethodMeta, UnOp};
pub use crate::widget::{give_up, given_answer, given_bind, Arms};
pub use crate::{lens, pat, tables};

/// The layers, by one letter, as the emitted tables spell them.
pub const W: Layer = Layer::Whole;
pub const V: Layer = Layer::Var;
pub const C: Layer = Layer::Concrete;
pub const O: Layer = Layer::Opt;

/// A function body: `body(captured, args, b)`.
pub type BodyFn = fn(&[Value], Vec<Value>, &mut Bx) -> R<Value>;

/// A widget block: `build(b, captured, args)`.
pub type BuildFn = fn(&mut Bx, &[Value], &[Value]) -> R<Value>;

// ----- constants -----

pub fn unit() -> Value {
    Value::Unit
}
pub fn nil() -> Value {
    Value::nil()
}
pub fn boolean(x: bool) -> Value {
    Value::Bool(x)
}
/// An integer literal: an `Int` until a signature, a field or a part says `Nat` (pick LT).
pub fn small(n: i64) -> Value {
    Value::Int(num_bigint::BigInt::from(n))
}
pub fn int(digits: &str) -> Value {
    Value::Int(crate::pat::parse_int(digits).unwrap_or_default())
}
pub fn float(f: f64) -> Value {
    Value::Float(f)
}
pub fn text(s: &str) -> Value {
    Value::text(s)
}
pub fn ch(c: char) -> Value {
    Value::Char(c)
}
pub fn duration(nanos: i128) -> Value {
    Value::Duration(nanos)
}
pub fn tuple(items: Vec<Value>) -> Value {
    Value::tuple(items)
}
pub fn array(items: Vec<Value>) -> Value {
    Value::array(items)
}

// ----- globals -----

thread_local! {
    static GLOBALS: RefCell<HashMap<&'static str, Value>> = RefCell::new(HashMap::new());
    static IN_PROGRESS: RefCell<HashSet<&'static str>> = RefCell::new(HashSet::new());
}

/// A global, made once per thread.
pub fn global(key: &'static str, make: impl FnOnce() -> R<Value>) -> R<Value> {
    if let Some(v) = GLOBALS.with(|g| g.borrow().get(key).cloned()) {
        return Ok(v);
    }
    if !IN_PROGRESS.with(|p| p.borrow_mut().insert(key)) {
        return Err(Fault::new("prism.Eval", format!("{key} is defined in terms of itself: a cycle among plain values (Semantics ch. 4)")));
    }
    let r = make();
    IN_PROGRESS.with(|p| p.borrow_mut().remove(key));
    let v = r?;
    GLOBALS.with(|g| g.borrow_mut().insert(key, v.clone()));
    Ok(v)
}

/// A function value over an emitted body: its meta made once per site `key`.
pub fn function(key: &'static str, name: &'static str, layers: &'static [Layer], nat: &'static [bool], peel: bool, body: BodyFn, cap: Vec<Value>) -> Value {
    let id = crate::func::site(key, || FnMeta {
        name: Rc::from(name),
        layers: layers.to_vec(),
        nat: nat.to_vec(),
        peel,
        ctor: None,
        user: usize::MAX,
        body: Rc::new(body),
    });
    crate::func::closure(id, name, cap)
}

/// An unbounded function's body runs under [`crate::unbounded::unbounded`].
pub fn unbounded(body: impl FnOnce() -> R<Value>) -> R<Value> {
    crate::unbounded::unbounded(body)
}

/// A local function's own value, from inside its body: it sees itself by name.
pub fn again(key: &'static str, name: &'static str, cap: &[Value]) -> Value {
    let id = crate::func::site(key, || panic!("{key} runs before it was made"));
    crate::func::closure(id, name, cap.to_vec())
}

/// A widget function: applying it gives a blueprint of its arguments (Semantics ch. 4),
/// built where it is bound by `build(b, captured, args)`.
pub fn widget_function(key: &'static str, name: &'static str, arity: usize, build: BuildFn, cap: Vec<Value>) -> Value {
    let id = crate::func::site(key, || FnMeta {
        name: Rc::from(name),
        layers: vec![Layer::Whole; arity],
        nat: vec![],
        peel: false,
        ctor: None,
        user: usize::MAX,
        body: Rc::new(move |cap, args, _| {
            let mut c = vec![Value::array(args)];
            c.extend(cap.iter().cloned());
            Ok(Value::Widget(Blueprint::new(name, c, move |bd, cap, _| {
                let Some(Value::Array(args)) = cap.first() else { unreachable!() };
                let args: Vec<Value> = args.iter().cloned().collect();
                build(&mut Some(bd), &cap[1..], &args)
            })))
        }),
    });
    crate::func::closure(id, name, cap)
}

/// A blueprint over a widget block: `@counter:`, `@widget:`, a `rebuild`'s or a `for`'s
/// block, `@ v`.
pub fn blueprint(name: &'static str, cap: Vec<Value>, build: BuildFn) -> Value {
    Value::Widget(Blueprint::new(name, cap, move |bd, cap, args| build(&mut Some(bd), cap, args)))
}

/// A prelude function written in Rust (`prism_dyn::builtins`), lifted by its signature.
pub fn builtin(key: &'static str, name: &'static str, layers: &'static [Layer], nat: &'static [bool]) -> Value {
    if name.starts_with('@') {
        // A widget binding: applied, a blueprint of its arguments.
        let id = crate::func::site(key, || FnMeta {
            name: Rc::from(name),
            layers: vec![Layer::Whole; layers.len()],
            nat: vec![],
            peel: false,
            ctor: None,
            user: usize::MAX,
            body: Rc::new(move |_, args, _| Ok(builtin_widget(name, args))),
        });
        return crate::func::closure(id, name, vec![]);
    }
    let id = crate::func::site(key, || FnMeta {
        name: Rc::from(name),
        layers: layers.to_vec(),
        nat: nat.to_vec(),
        peel: true,
        ctor: None,
        user: usize::MAX,
        body: Rc::new(move |_, args, b| crate::builtins::call(name, &args, b)),
    });
    crate::func::closure(id, name, vec![])
}

/// A widget builtin applied to its arguments: binding it calls the `Build` method.
pub fn builtin_widget(name: &'static str, args: Vec<Value>) -> Value {
    Value::Widget(Blueprint::new(name, args, move |bd, cap, _| crate::builtins::call(name, cap, &mut Some(bd))))
}

/// A builtin that is a value, not a function: `@started`, `Buffer.Empty`.
pub fn builtin_value(name: &'static str) -> R<Value> {
    if name.starts_with('@') {
        return Ok(builtin_widget(name, vec![]));
    }
    crate::builtins::call(name, &[], &mut None)
}

/// A trait method, dispatched on the runtime type of one argument.
pub fn method(key: &'static str, m: MethodMeta, layers: &'static [Layer]) -> Value {
    crate::values::method_value(key, m, layers)
}

/// A constructor written as an expression.
pub fn ctor(name: &str) -> R<Value> {
    crate::values::ctor_value(name)
}

/// A tag written as an expression.
pub fn tag(name: &str) -> R<Value> {
    crate::values::tag_value(name)
}

// ----- application -----

pub fn apply(f: &Value, args: Vec<Value>, b: &mut Bx) -> R<Value> {
    crate::func::apply(b, f, args)
}

/// Explicit instantiation, `F<<$Int>> x`: the layers it fixes.
pub fn apply_with(f: &Value, args: Vec<Value>, over: &[Option<Layer>], b: &mut Bx) -> R<Value> {
    crate::func::apply_with(b, f, args, Some(over))
}

/// Named arguments, `Point(x=1, y=2)`: a record constructor's.
pub fn apply_named(f: &Value, named: Vec<(Option<&str>, Value)>, b: &mut Bx) -> R<Value> {
    crate::values::apply_named(b, f, named)
}

/// An element: `Ui.Button(label=$"Go")`, fields positional or named.
pub fn element(tag: &str, args: Vec<(Option<&str>, Value)>) -> R<Value> {
    crate::values::element_args(tag, args)
}

pub fn binop(op: Op, x: Value, y: Value, b: &mut Bx) -> R<Value> {
    crate::values::binop(b, op, x, y)
}

pub fn unop(op: UnOp, x: Value, b: &mut Bx) -> R<Value> {
    crate::values::unop(b, op, x)
}

/// `x and y`, `x or y`: `y` runs only when needed.
pub fn and_or(op: Op, x: Value, y: impl FnOnce(&mut Bx) -> R<Value>, b: &mut Bx) -> R<Value> {
    crate::values::and_or(b, op, x, y)
}

/// `x on !e`: the sample, `Build::tag`; inside a lifted function the event's value
/// stands for the firing.
pub fn on(x: Value, e: Value, b: &mut Bx) -> R<Value> {
    match (&x, &e) {
        (Value::Event(_), Value::Event(_)) => Err(Fault::new(
            "prism.Eval",
            "`!a on !b` is two events in one operation; to mean the last `!a` when `!b` fires, hold `!a` first (Type System ch. 4)",
        )),
        (_, Value::Event(_)) => crate::widget::build(b)?.tag(&e, &x),
        (_, Value::Dynamic(_)) => Err(Fault::new("prism.Eval", "`on` samples at an event; its right operand is a dynamic")),
        _ => Ok(x),
    }
}

/// `!a <| !b`: the leftmost merge.
pub fn merge(x: Value, y: Value, b: &mut Bx) -> R<Value> {
    if crate::widget::is_never_marker(&x) {
        return Ok(y);
    }
    match (&x, &y) {
        (Value::Event(_), Value::Event(_)) => crate::widget::build(b)?.merge(&[x, y]),
        (Value::Event(_), _) | (_, Value::Event(_)) | (Value::Dynamic(_), _) | (_, Value::Dynamic(_)) => {
            Err(Fault::new("prism.Eval", "`<|` merges two events"))
        }
        _ => Ok(x),
    }
}

pub fn field(r: Value, name: &str, b: &mut Bx) -> R<Value> {
    crate::values::get_field(b, r, name)
}

pub fn index(r: Value, k: Value, b: &mut Bx) -> R<Value> {
    crate::values::get_index(b, r, k)
}

/// `? v`.
pub fn some(v: Value, b: &mut Bx) -> R<Value> {
    crate::values::wrap_some(b, v)
}

/// `~ v`.
pub fn okay(v: Value, b: &mut Bx) -> R<Value> {
    crate::values::wrap_ok(b, v)
}

/// `$ v`.
pub fn constant(v: Value, b: &mut Bx) -> Value {
    crate::widget::constant(b, v)
}

pub fn never(b: &mut Bx) -> Value {
    crate::widget::never(b)
}

/// A map literal, lifted over its handles (pick ML).
pub fn map(entries: Vec<Value>, b: &mut Bx) -> R<Value> {
    crate::values::map_literal(b, entries)
}

/// A text literal with placeholders.
pub fn interpolate(parts: &[Option<&str>], vals: Vec<Value>, b: &mut Bx) -> R<Value> {
    crate::values::interpolate(b, parts, vals)
}

/// A `match` or `if`: its arms here over a plain scrutinee, one lifted node over a
/// handle.
pub fn branch(s: Value, env: &[Value], arms: Arms, b: &mut Bx) -> R<Value> {
    crate::widget::branch(b, s, env, arms)
}

pub fn is_handle(v: &Value) -> bool {
    crate::values::is_handle(v)
}

/// An `if`'s condition, a guard, a `while`'s test.
pub fn truth(v: &Value) -> R<bool> {
    match v {
        Value::Bool(x) => Ok(*x),
        other => Err(Fault::new("prism.Eval", format!("a condition is a Bool, and this one is {}", crate::ops::show_plain(other)))),
    }
}

pub fn no_clause(name: &str, args: &[Value]) -> Fault {
    let shown: Vec<String> = args.iter().map(crate::ops::show_plain).collect();
    Fault::new("prism.Eval", format!("no clause of {name} matches {}", shown.join(", ")))
}

pub fn no_arm(s: &Value) -> Fault {
    Fault::new("prism.Eval", format!("no arm matches {}", crate::ops::show_plain(s)))
}

pub fn mismatch(what: &str, v: &Value) -> Fault {
    Fault::new("prism.Eval", format!("the {what} pattern does not match its value, {}", crate::ops::show_plain(v)))
}

/// A fault from the program, placed at a source position.
pub fn at(place: &'static str) -> impl Fn(Fault) -> Fault {
    move |f| {
        if f.kind == crate::unbounded::CYCLE_BUDGET || f.message.contains(".prism:") {
            f
        } else {
            Fault::new(f.kind.clone(), format!("{place}: {}", f.message))
        }
    }
}

// ----- widget blocks -----

pub fn set_emit(families: &[&str], b: &mut Bx) -> R<()> {
    crate::widget::build(b)?.set_emit(Row::Families(families.iter().map(|f| f.to_string()).collect()));
    Ok(())
}

/// What a statement does with its value: an element emitted, a blueprint built.
pub fn bind_build(v: Value, b: &mut Bx) -> R<Value> {
    crate::widget::bind_build(b, v)
}

/// A hold, `$h <- init then ..`: a dynamic initial value is read as the construction
/// reads it (R7).
pub fn hold(init: Value, b: &mut Bx) -> R<Value> {
    let b = crate::widget::build(b)?;
    let init = b.read(&init)?;
    Ok(b.hold(init))
}

/// A hold's `then`, wired once every line has run.
pub fn hold_on(h: &Value, e: &Value, b: &mut Bx) -> R<()> {
    match e {
        Value::Event(_) => crate::widget::build(b)?.hold_on(h, e),
        other => Err(Fault::new(
            "prism.Eval",
            format!("`then` takes an event: the hold changes when it fires; this is {}", crate::ops::show_plain(other)),
        )),
    }
}

/// An event read above the line that binds it.
pub fn forward(b: &mut Bx) -> R<Value> {
    Ok(crate::widget::build(b)?.forward_event())
}

pub fn resolve(fwd: &Value, e: &Value, b: &mut Bx) -> R<()> {
    if !matches!(e, Value::Event(_)) {
        return Err(Fault::new("prism.Eval", format!("an event is bound here, and the line gives {}", crate::ops::show_plain(e))));
    }
    crate::widget::build(b)?.resolve_event(fwd, e)
}

/// `rebuild $x:` and its block.
pub fn rebuild(src: Value, kind: ResultKind, bp: Value, b: &mut Bx) -> R<Value> {
    let Value::Widget(bp) = bp else { unreachable!() };
    crate::widget::build(b)?.rebuild(&src, kind, &bp)
}

/// Keyed `for` and its block; over a plain collection the rows' results are gathered.
pub fn for_each(src: Value, kind: ResultKind, bp: Value, b: &mut Bx) -> R<Value> {
    let Value::Widget(bp) = bp else { unreachable!() };
    let dynamic = matches!(src, Value::Dynamic(_));
    let b = crate::widget::build(b)?;
    let r = b.for_each(&src, kind, &bp)?;
    if dynamic {
        return Ok(r);
    }
    crate::widget::gather_plain_rows(r, b)
}

/// A `for` row's bound names: the key, the row's value, `ix`.
pub fn row_arg(args: &[Value], i: usize) -> Value {
    args.get(i).cloned().unwrap_or(if i == 2 { Value::nat(0) } else { Value::Unit })
}

// ----- templates -----

/// A placeholder's value written into a template's text: an array's items one after
/// another, anything else by display; alone on its line (`column`), every line of it
/// indented to the column.
pub fn place(out: &mut String, v: &Value, column: Option<usize>, b: &mut Bx) -> R<()> {
    let text = match v {
        Value::Array(items) => {
            let mut s = String::new();
            for i in items.iter() {
                s.push_str(&crate::values::display(i, b)?);
            }
            s
        }
        other => crate::values::display(other, b)?,
    };
    place_text(out, &text, column);
    Ok(())
}

/// A placeholder's text written into a template's text (Templates ch. 2, "Whitespace and
/// escapes"): sharing a line, as written; alone on its line, it stands for whole lines:
/// its text split into lines, a final newline dropped, each non-empty line indented to
/// the placeholder's column, each ending its line, and an empty text leaving no line.
pub fn place_text(out: &mut String, text: &str, column: Option<usize>) {
    match column {
        Some(c) => {
            let text = text.strip_suffix('\n').unwrap_or(text);
            if text.is_empty() {
                return;
            }
            let pad = " ".repeat(c);
            for l in text.split('\n') {
                if !l.is_empty() {
                    out.push_str(&pad);
                }
                out.push_str(l);
                out.push('\n');
            }
        }
        None => out.push_str(text),
    }
}

/// A brace block's or a call's value written into a template's text.
pub fn place_value(out: &mut String, v: &Value, b: &mut Bx) -> R<()> {
    if !matches!(v, Value::Unit) {
        out.push_str(&crate::values::display(v, b)?);
    }
    Ok(())
}

// ----- `&` blocks -----

/// A field step of a lens path.
pub fn step(name: &str) -> Step {
    Step::Field(Rc::from(name))
}

/// An index step.
pub fn at_key(k: Value) -> Step {
    Step::Index(k)
}

/// A `while` turn: a cycle point inside an unbounded function.
pub fn turn() -> R<()> {
    crate::unbounded::cycle_point()
}

// ----- the program -----

/// The tables a program declares, installed before its first tick.
pub fn install(ctors: Vec<CtorMeta>, tags: Vec<(Rc<TagDecl>, bool)>) {
    tables::reset();
    for c in ctors {
        tables::add_ctor(c);
    }
    for (t, fault) in tags {
        tables::add_tag(t, fault);
    }
}

/// A constructor's table entry.
pub fn ctor_meta(name: &str, type_name: &str, record: bool, fields: &[&str], nat: &[bool], layers: &[Layer], index: usize) -> CtorMeta {
    CtorMeta {
        name: Rc::from(name),
        type_name: Rc::from(type_name),
        record,
        fields: fields.iter().map(|f| Rc::from(*f)).collect(),
        nat: nat.to_vec(),
        layers: layers.to_vec(),
        index,
    }
}

/// A tag's table entry.
pub fn tag_decl(name: &str, fields: &[(&str, FieldKind)], output: OutputKind, fault: bool) -> (Rc<TagDecl>, bool) {
    (TagDecl::new(name, fields, output), fault)
}

/// What `main` does: `--world cli` (the default) runs `root` under the CLI World with
/// the arguments after `--`; `--world test SCRIPTS...` runs test scripts under the Test
/// World against `programs`; `--world canvas` is the caller's (it links the canvas
/// World). The exit code.
pub fn main_with(
    root: impl FnOnce() -> R<Value>,
    programs: impl FnOnce() -> R<Vec<(&'static str, Value)>>,
    eval: Rc<dyn Fn(&str) -> Option<R<Value>>>,
    canvas: Option<fn(Program) -> u8>,
) -> u8 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (world, rest): (&str, &[String]) = match args.first().map(|a| a.as_str()) {
        Some("--world") => (args.get(1).map(|s| s.as_str()).unwrap_or(""), &args[2.min(args.len())..]),
        _ => ("cli", &args[..]),
    };
    let rest: Vec<String> = match rest.first().map(|s| s.as_str()) {
        Some("--") => rest[1..].to_vec(),
        _ => rest.to_vec(),
    };
    match world {
        "test" => {
            let ps = match programs() {
                Ok(p) => p,
                Err(f) => {
                    eprintln!("{}", f.message);
                    return prism_rt::world::EXIT_UNCHECKED;
                }
            };
            let ps: Vec<(&str, Blueprint)> = ps.into_iter().filter_map(|(n, v)| if let Value::Widget(bp) = v { Some((n, bp)) } else { None }).collect();
            crate::world::run_tests(&ps, eval, &rest)
        }
        "cli" | "canvas" => {
            let bp = match root() {
                Ok(Value::Widget(bp)) => bp,
                Ok(_) => {
                    eprintln!("@main is not a blueprint");
                    return prism_rt::world::EXIT_UNCHECKED;
                }
                Err(f) => {
                    eprintln!("{}", f.message);
                    return prism_rt::world::EXIT_UNCHECKED;
                }
            };
            let program = Program::new("main", bp);
            if world == "canvas" {
                match canvas {
                    Some(run) => run(program),
                    None => {
                        eprintln!("this program was built without the canvas World: build it with `--features canvas`");
                        prism_rt::world::EXIT_UNCHECKED
                    }
                }
            } else {
                crate::world::run_cli(&program, rest)
            }
        }
        other => {
            eprintln!("--world takes `cli`, `test` or `canvas`, and was given `{other}`");
            prism_rt::world::EXIT_UNCHECKED
        }
    }
}

// ----- what emitted functions wrap their bodies in -----

/// A function's body, its faults placed at the function's source position.
pub fn within(place: &'static str, body: impl FnOnce() -> R<Value>) -> R<Value> {
    body().map_err(at(place))
}

/// An unbounded function's body: its call a cycle point when an unbounded function made
/// it (decisions.md D6), its faults placed.
pub fn within_unbounded(place: &'static str, body: impl FnOnce() -> R<Value>) -> R<Value> {
    crate::unbounded::unbounded(body).map_err(at(place))
}

/// A fault the program reaches: an out-of-scope feature, a statement where it does not
/// stand.
pub fn fault(msg: &str) -> Fault {
    Fault::new("prism.Eval", msg)
}

/// An expression that fails when it runs.
pub fn fail(msg: &str) -> R<Value> {
    Err(fault(msg))
}

/// A loop's `ix`.
pub fn nat(n: usize) -> Value {
    Value::nat(n as u64)
}

/// How a value enters a text (Type System ch. 4, interpolation).
pub fn display(v: &Value, b: &mut Bx) -> R<String> {
    crate::values::display(v, b)
}

/// An integer literal in a pattern, as written.
pub fn pat_int(v: &Value, digits: &str) -> bool {
    match crate::pat::parse_int(digits.trim_start_matches('-')) {
        Some(n) => crate::pat::int(v, &if digits.starts_with('-') { -n } else { n }),
        None => false,
    }
}
