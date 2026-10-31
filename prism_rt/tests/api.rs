//! Tests at the level of the Rust API: the first-value query with the tick's pushes on
//! top, the sandbox law at the forwarding arm, `@HostSome` and `@HostList`, demand,
//! persistence of the state, and the smaller combinators.

// Programs here pass a dynamic as `&[d.clone()]`, as an interpreter would.
#![allow(clippy::cloned_ref_to_slice_refs)]

mod common;

use std::cell::Cell;
use std::rc::Rc;

use common::*;
use prism_rt::document::ReadyMap;
use prism_rt::families::fs;
use prism_rt::{tick, Blueprint, Closure, Inputs, Program, ResultKind, Row, State, Value};

fn path(p: &str) -> prism_rt::Path {
    prism_rt::pod::read_path(p).unwrap()
}

fn element(s: &str) -> Value {
    prism_rt::pod::read(s).unwrap()
}

fn ready(entries: &[(&str, Value)]) -> Rc<ReadyMap> {
    Rc::new(ReadyMap(entries.iter().map(|(e, v)| (element(e), v.clone())).collect()))
}

fn build(program: &Program, inputs: &Inputs) -> (State, prism_rt::Outputs) {
    let t = tick(program, None, inputs);
    (t.state.unwrap(), t.outputs.unwrap())
}

fn next(program: &Program, st: State, inputs: &Inputs) -> (State, prism_rt::Outputs) {
    let t = tick(program, Some(st), inputs);
    let out = t.outputs.unwrap();
    (t.state.unwrap(), out)
}

/// A program with an older watch of `/a` from the start, and a button that builds a
/// second watch of `/a` in a `rebuild`.
fn two_watches() -> Program {
    Program::new(
        "watches",
        Blueprint::pure("@watches", |b, _| {
            let open = button(b, "Open")?;
            let older = contents(b, "/a")?;
            let shown = b.lift(&[older], text("older", |a| format!("older {}", show_response(&a[0]))));
            label(b, shown)?;
            let opening = b.tag(&open, &Value::Bool(true))?;
            let opened = b.hold_then(Value::Bool(false), &opening)?;
            b.rebuild(&opened, ResultKind::Unit, &Blueprint::pure("newer", |b, args| {
                if args[0] == Value::Bool(true) {
                    let newer = contents(b, "/a")?;
                    let shown = b.lift(&[newer], text("newer", |a| format!("newer {}", show_response(&a[0]))));
                    label(b, shown)?;
                }
                Ok(Value::Unit)
            }))?;
            Ok(Value::Unit)
        }),
    )
}

#[test]
fn a_newborn_place_takes_the_ticks_push_as_its_first_value() {
    // Semantics ch. 8: the first value comes from the ready state as the tick opened,
    // with the tick's pushes on top; the two places end the tick agreeing.
    let program = two_watches();
    let old = ready(&[("Fs.Contents(path=\"/a\")", Value::ok(fs::contents_value("old")))]);
    let (st, out) = build(&program, &Inputs::with_ready(old.clone()));
    assert_eq!(out.patch.added[2].1, element("Ui.Label(text=\"older old\")"));
    let inputs = Inputs::with_ready(old)
        .answer(path("0"), Value::click())
        .push(element("Fs.Contents(path=\"/a\")"), Value::ok(fs::contents_value("new")));
    let (_, out) = next(&program, st, &inputs);
    let added: Vec<String> = out.patch.added.iter().map(|(p, e)| format!("{p}: {e}")).collect();
    assert_eq!(added, vec!["3/#1/0: Fs.Contents(path=\"/a\")", "3/#1/1: Ui.Label(text=\"newer new\")"]);
    let changed: Vec<String> = out.patch.changed.iter().map(|(p, _, e)| format!("{p}: {e}")).collect();
    assert_eq!(changed, vec!["2: Ui.Label(text=\"older new\")"]);
}

#[test]
fn a_newborn_shares_an_older_places_fact() {
    // Graph ch. 4, External: a place built at the end of a tick takes its first value
    // from an older place with the same element, even when the ready state moved on.
    let program = two_watches();
    let old = ready(&[("Fs.Contents(path=\"/a\")", Value::ok(fs::contents_value("old")))]);
    let (st, _) = build(&program, &Inputs::with_ready(old));
    let stale = ready(&[("Fs.Contents(path=\"/a\")", Value::ok(fs::contents_value("other")))]);
    let (_, out) = next(&program, st, &Inputs::with_ready(stale).answer(path("0"), Value::click()));
    assert_eq!(out.patch.added[1].1, element("Ui.Label(text=\"newer old\")"));
}

/// A host whose row is `Ui.*` forwarding everything of a child that emits `Fs.*` too.
fn leaky_host() -> Program {
    Program::new(
        "leaky",
        Blueprint::pure("@Offline", |b, _| {
            b.set_emit(Row::of(&["Ui"]));
            let child = Blueprint::pure("@child", |b, _| {
                label(b, Value::text("hi"))?;
                contents(b, "/etc/motd")?;
                Ok(Value::Unit)
            });
            let forward = Blueprint::pure("forward", |b, args| {
                let Value::Element(e) = &args[1] else { unreachable!() };
                b.emit((**e).clone())
            });
            b.host_document(&child, &[], &forward)?;
            Ok(Value::Unit)
        }),
    )
}

#[test]
fn the_sandbox_law_holds_at_the_forwarding_arm() {
    // Semantics ch. 9: re-emission is checked against the host's row at the arm.
    let t = tick(&leaky_host(), None, &Inputs::none());
    let fault = t.outputs.unwrap_err();
    assert_eq!(fault.kind, "Prelude.Sandbox");
    assert!(fault.message.contains("Fs.Contents"), "{fault}");
    assert!(t.state.is_none());
}

#[test]
fn host_some_answers_some_places_and_forwards_the_rest() {
    let program = Program::new(
        "some",
        Blueprint::pure("@some", |b, _| {
            let child = Blueprint::pure("@child", |b, _| {
                let motd = contents(b, "/etc/motd")?;
                let shown = b.lift(&[motd], text("motd", |a| show_response(&a[0])));
                label(b, shown)?;
                Ok(Value::Unit)
            });
            let give = Closure::pure("?Give", |a| {
                let Value::Element(e) = &a[1] else { unreachable!() };
                Ok(if &*e.tag == "Fs.Contents" {
                    Value::some(Value::Widget(Blueprint::pure("mock", |b, _| {
                        Ok(b.constant(Value::ok(fs::contents_value("mocked"))))
                    })))
                } else {
                    Value::nil()
                })
            });
            b.host_some(&child, &[], &give)?;
            Ok(Value::Unit)
        }),
    );
    let (_, out) = build(&program, &Inputs::none());
    let added: Vec<String> = out.patch.added.iter().map(|(p, e)| format!("{p}: {e}")).collect();
    assert_eq!(added, vec!["0/[1]/0: Ui.Label(text=\"mocked\")"]);
}

#[test]
fn host_list_sends_answers_down_by_path() {
    let program = Program::new(
        "list",
        Blueprint::pure("@list", |b, _| {
            let send = button(b, "Send")?;
            let answers = b.tag(&send, &Value::map(vec![(Value::Path(path("0")), Value::click())]))?;
            let child = Blueprint::pure("@child", |b, _| {
                let press = button(b, "Inner")?;
                let n = b.hold(Value::int(0));
                let more = b.sample(&press, &[n.clone()], ints("+1", |a| a[1] + 1))?;
                b.hold_on(&n, &more)?;
                let shown = b.lift(&[n], text("n", |a| format!("pressed {}", show(&a[0]))));
                label(b, shown)?;
                Ok(Value::Unit)
            });
            let forward = Blueprint::pure("forward", |b, args| {
                let Value::Element(e) = &args[1] else { unreachable!() };
                b.emit((**e).clone())
            });
            b.host_list(&child, &[], &answers, &forward)?;
            Ok(Value::Unit)
        }),
    );
    let (st, _) = build(&program, &Inputs::none());
    let (st, out) = next(&program, st, &Inputs::none().answer(path("0"), Value::click()));
    assert_eq!(out.patch.changed[0].2, element("Ui.Label(text=\"pressed 1\")"));
    // A key that is a path prints as its notation, so the place is found by its
    // printed path, as the Test World finds it.
    let inner = st.document().keys().find(|p| p.to_string() == "1/[0]/0").unwrap().clone();
    let (_, out) = next(&program, st, &Inputs::none().answer(inner, Value::click()));
    assert_eq!(out.patch.changed[0].2, element("Ui.Label(text=\"pressed 2\")"));
}

#[test]
fn nothing_runs_that_nothing_needs() {
    // Semantics ch. 12, "Demand": a dynamic nothing reads and an event nothing hears are
    // never computed.
    let lifted = Rc::new(Cell::new(0));
    let mapped = Rc::new(Cell::new(0));
    let (l, m) = (lifted.clone(), mapped.clone());
    let program = Program::new(
        "lazy",
        Blueprint::new("@lazy", vec![], move |b, _, _| {
            let click = button(b, "Go")?;
            let count = b.hold(Value::int(0));
            let more = b.sample(&click, &[count.clone()], ints("+1", |a| a[1] + 1))?;
            b.hold_on(&count, &more)?;
            let l = l.clone();
            b.lift(&[count], Closure::pure("Analyze", move |a| {
                l.set(l.get() + 1);
                Ok(a[0].clone())
            }));
            let m = m.clone();
            b.map(&click, Closure::pure("Expensive", move |a| {
                m.set(m.get() + 1);
                Ok(a[0].clone())
            }))?;
            Ok(Value::Unit)
        }),
    );
    let (mut st, _) = build(&program, &Inputs::none());
    for _ in 0..3 {
        st = next(&program, st, &Inputs::none().answer(path("0"), Value::click())).0;
    }
    assert_eq!((lifted.get(), mapped.get()), (0, 0));
}

#[test]
fn a_state_may_be_ticked_twice() {
    // Semantics ch. 7: same arguments, same results; keeping an old state is allowed.
    let program = two_watches();
    let r = ready(&[("Fs.Contents(path=\"/a\")", Value::ok(fs::contents_value("x")))]);
    let (st, _) = build(&program, &Inputs::with_ready(r.clone()));
    let inputs = Inputs::with_ready(r).answer(path("0"), Value::click());
    let (_, a) = next(&program, st.clone(), &inputs);
    let (_, b) = next(&program, st.clone(), &inputs);
    assert_eq!(a.patch, b.patch);
    assert_eq!(st.document().len(), 3, "the old state is untouched");
}

#[test]
fn a_for_over_an_array_is_keyed_by_index() {
    let program = Program::new(
        "array",
        Blueprint::pure("@array", |b, _| {
            let add = button(b, "Add")?;
            let xs = b.hold(Value::array(vec![Value::text("a")]));
            let grown = b.sample(&add, &[xs.clone()], Closure::pure("Push", |a| match &a[1] {
                Value::Array(v) => {
                    let mut v = v.clone();
                    v.push_back(Value::text("b"));
                    Ok(Value::Array(v))
                }
                _ => unreachable!(),
            }))?;
            b.hold_on(&xs, &grown)?;
            let row = Blueprint::pure("row", |b, args| {
                let ix = show(&args[2]);
                let shown = b.lift(&[args[1].clone()], text("row", move |a| format!("{ix}: {}", show(&a[0]))));
                label(b, shown)?;
                Ok(Value::Unit)
            });
            b.for_each(&xs, ResultKind::Unit, &row)?;
            Ok(Value::Unit)
        }),
    );
    let (st, out) = build(&program, &Inputs::none());
    assert_eq!(format!("{}", out.patch.added[1].0), "1/[0]/0");
    let (_, out) = next(&program, st, &Inputs::none().answer(path("0"), Value::click()));
    let added: Vec<String> = out.patch.added.iter().map(|(p, e)| format!("{p}: {e}")).collect();
    assert_eq!(added, vec!["1/[1]/0: Ui.Label(text=\"1: b\")"]);
}

#[test]
fn filter_present_gate_and_machine() {
    let program = Program::new(
        "small",
        Blueprint::pure("@small", |b, _| {
            let go = button(b, "Go")?;
            let toggle = button(b, "Toggle")?;
            let open = b.hold(Value::Bool(false));
            let flipped = b.sample(&toggle, &[open.clone()], Closure::pure("not", |a| Ok(Value::Bool(a[1] != Value::Bool(true)))))?;
            b.hold_on(&open, &flipped)?;
            // A machine counting presses, emitting every second one.
            let step = Closure::pure("every other", |a| {
                let n = a[0].as_i128().unwrap() + 1;
                let out = if n % 2 == 0 { Value::some(Value::int(n as i64)) } else { Value::nil() };
                Ok(Value::tuple(vec![Value::int(n as i64), out]))
            });
            let gated = b.gate(&open, &go)?;
            let evens = b.machine(step, Value::int(0), &gated)?;
            let maybe = b.map(&evens, Closure::pure("?", |a| Ok(Value::some(a[0].clone()))))?;
            let present = b.present(&maybe)?;
            let filtered = b.filter(&maybe)?;
            let last = b.hold_then(Value::int(0), &filtered)?;
            let seen = b.hold_then(Value::Unit, &present)?;
            let shown = b.lift(&[last, seen], text("s", |a| format!("{} {}", show(&a[0]), show(&a[1]))));
            label(b, shown)?;
            Ok(Value::Unit)
        }),
    );
    let click = |p: &str| Inputs::none().answer(path(p), Value::click());
    let (mut st, _) = build(&program, &Inputs::none());
    st = next(&program, st, &click("0")).0; // gate closed: nothing
    st = next(&program, st, &click("1")).0; // open
    st = next(&program, st, &click("0")).0; // 1: odd
    let (_, out) = next(&program, st, &click("0")); // 2: even, fires
    assert_eq!(out.patch.changed[0].2, element("Ui.Label(text=\"2 unit\")"));
}
