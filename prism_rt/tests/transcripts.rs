//! Transcript tests: programs built by hand against the `Build` API, run by the Test
//! World from a script in the test language, and compared with their transcripts byte
//! for byte. Where a book prints a transcript for the program, the expected text is the
//! book's.

// Programs here pass a dynamic as `&[d.clone()]`, as an interpreter would.
#![allow(clippy::cloned_ref_to_slice_refs)]

mod common;

use common::*;
use prism_rt::families::{gui, test, ui};
use prism_rt::world::test_world::TestWorld;
use prism_rt::{Blueprint, Closure, Fault, Program, ResultKind, Value};

fn verify(program: Program, script: &str, expected: &str) {
    let world = TestWorld::new().program(program);
    if let Err(e) = world.verify(script, expected) {
        panic!("{e}");
    }
}

// ---------------------------------------------------------------------------------------
// The half counter (Compiler ch. 3) and the counter (Semantics ch. 7, ch. 12; D13)
// ---------------------------------------------------------------------------------------

/// ```prism
/// @halfcounter:
///     !increment <- Ui.Button(label=$"Increment")
///     let !delta = +1 on !increment
///     $count <- 0 then $count + !delta
///     Ui.Label(text=$"Count: {$count}")
/// ```
fn halfcounter() -> Program {
    Program::new(
        "halfcounter",
        Blueprint::pure("@halfcounter", |b, _| {
            let increment = button(b, "Increment")?;
            let delta = b.tag(&increment, &Value::int(1))?;
            let count = b.hold(Value::int(0));
            let next = b.sample(&delta, &[count.clone()], ints("+", |a| a[1] + a[0]))?;
            b.hold_on(&count, &next)?;
            let shown = b.lift(&[count], text("Count", |a| format!("Count: {}", show(&a[0]))));
            label(b, shown)?;
            Ok(Value::Unit)
        }),
    )
}

#[test]
fn the_half_counter_counts() {
    verify(
        halfcounter(),
        r#"
test half_counter_counts, @halfcounter:
    build
        => expect Ui.Label(text="Count: 0")
    click Ui.Button(label="Increment")
        => expect Ui.Label(text="Count: 1")
    click at 0
        => expect Ui.Label(text="Count: 2")
"#,
        r#"test half_counter_counts

tick 0
  added:
    0: Ui.Button(label="Increment")
    1: Ui.Label(text="Count: 0")

tick 1
  in:
    0: click
  changed:
    1: Ui.Label(text="Count: 1")

tick 2
  in:
    0: click
  changed:
    1: Ui.Label(text="Count: 2")
"#,
    );
}

/// ```prism
/// @counter:
///     !inc <- Ui.Button(label=$"Increment")                 # 0
///     !dec <- Ui.Button(label=$"Decrement")                 # 1
///     $count <- 0 then $count + (-1 on !dec <| +1 on !inc)
///     Ui.Label(text=$"Count: {$count}")                     # 2
/// ```
fn counter() -> Program {
    Program::new(
        "counter",
        Blueprint::pure("@counter", |b, _| {
            let inc = button(b, "Increment")?;
            let dec = button(b, "Decrement")?;
            let minus = b.tag(&dec, &Value::int(-1))?;
            let plus = b.tag(&inc, &Value::int(1))?;
            let delta = b.merge(&[minus, plus])?;
            let count = b.hold(Value::int(0));
            let next = b.sample(&delta, &[count.clone()], ints("+", |a| a[1] + a[0]))?;
            b.hold_on(&count, &next)?;
            let shown = b.lift(&[count], text("Count", |a| format!("Count: {}", show(&a[0]))));
            label(b, shown)?;
            Ok(Value::Unit)
        }),
    )
}

#[test]
fn two_clicks_in_one_tick_count_minus_one() {
    // D13: both events fire, and `<|` lets its left operand win.
    verify(
        counter(),
        r#"
test counter_counts, @counter:
    build
        => expect Ui.Label(text="Count: 0")
    click Ui.Button(label="Increment"), click Ui.Button(label="Decrement")
        => expect Ui.Label(text="Count: -1")
    click Ui.Button(label="Increment")
        => expect Ui.Label(text="Count: 0")
"#,
        r#"test counter_counts

tick 0
  added:
    0: Ui.Button(label="Increment")
    1: Ui.Button(label="Decrement")
    2: Ui.Label(text="Count: 0")

tick 1
  in:
    0: click
    1: click
  changed:
    2: Ui.Label(text="Count: -1")

tick 2
  in:
    0: click
  changed:
    2: Ui.Label(text="Count: 0")
"#,
    );
}

// ---------------------------------------------------------------------------------------
// A keyed `for` whose key set changes (Semantics ch. 4, `@labels`)
// ---------------------------------------------------------------------------------------

/// ```prism
/// @labels:
///     !add <- Ui.Button(label=$"Add a tag")                       # 0
///     $next <- 1 then $next + 1 on !add
///     let !adds = {$next: ? "tag {$next}"} on !add
///     !removes <- for $tags:                                      # 1/[key]
///         Ui.Label(text=$_)                                       # 1/[key]/0
///         !remove <- Ui.Button(label=$"Remove")                   # 1/[key]/1
///         @ nil on !remove
///     $tags <- {} then Apply $tags, (!Union !adds, !removes)
/// ```
fn labels() -> Program {
    Program::new(
        "labels",
        Blueprint::pure("@labels", |b, _| {
            let add = button(b, "Add a tag")?;
            let next = b.hold(Value::int(1));
            let bump = b.sample(&add, &[next.clone()], ints("+1", |a| a[1] + 1))?;
            b.hold_on(&next, &bump)?;
            let adds = b.sample(
                &add,
                &[next],
                Closure::pure("adds", |a| {
                    Ok(Value::map(vec![(a[1].clone(), Value::some(Value::text(&format!("tag {}", show(&a[1])))))]))
                }),
            )?;
            let tags = b.hold(Value::map(vec![]));
            let row = Blueprint::pure("row", |b, args| {
                label(b, args[1].clone())?;
                let remove = button(b, "Remove")?;
                b.tag(&remove, &Value::nil())
            });
            let removes = b.for_each(&tags, ResultKind::Event, &row)?;
            let ops = b.union(&adds, &removes)?;
            let apply = Closure::pure("Apply", |a| {
                let (Value::Map(ops), Value::Map(mut tags)) = (a[0].clone(), a[1].clone()) else {
                    return Err(Fault::runtime("maps"));
                };
                for (k, op) in ops.iter() {
                    match op {
                        Value::Opt(Some(t)) => {
                            tags.insert(k.clone(), (**t).clone());
                        }
                        _ => {
                            tags.remove(k);
                        }
                    }
                }
                Ok(Value::Map(tags))
            });
            let applied = b.sample(&ops, &[tags.clone()], apply)?;
            b.hold_on(&tags, &applied)?;
            Ok(Value::Unit)
        }),
    )
}

#[test]
fn rows_come_and_go_with_their_keys() {
    verify(
        labels(),
        r#"
test rows_come_and_go, @labels:
    build
    click Ui.Button(label="Add a tag")
        => expect Ui.Label(text="tag 1")
    click Ui.Button(label="Add a tag")
        => expect_count Ui.Button(label="Remove"), 2
    click at 1/[1]/1
        => expect_none Ui.Label(text="tag 1")
    click at 0, click at 1/[2]/1
        => expect_count Ui.Label(..), 1
        => expect Ui.Label(text="tag 3")
"#,
        r#"test rows_come_and_go

tick 0
  added:
    0: Ui.Button(label="Add a tag")

tick 1
  in:
    0: click
  added:
    1/[1]/0: Ui.Label(text="tag 1")
    1/[1]/1: Ui.Button(label="Remove")

tick 2
  in:
    0: click
  added:
    1/[2]/0: Ui.Label(text="tag 2")
    1/[2]/1: Ui.Button(label="Remove")

tick 3
  in:
    1/[1]/1: click
  torn down:
    1/[1]/0: Ui.Label(text="tag 1")
    1/[1]/1: Ui.Button(label="Remove")

tick 4
  in:
    0: click
    1/[2]/1: click
  added:
    1/[3]/0: Ui.Label(text="tag 3")
    1/[3]/1: Ui.Button(label="Remove")
  torn down:
    1/[2]/0: Ui.Label(text="tag 2")
    1/[2]/1: Ui.Button(label="Remove")
"#,
    );
}

// ---------------------------------------------------------------------------------------
// `rebuild`: the paged viewer of Semantics ch. 7, its transcript as the book prints it
// ---------------------------------------------------------------------------------------

/// ```prism
/// @loggedPages:
///     !next <- Ui.Button(label=$"Next")                       # 0
///     $page <- 0 then $page + 1 on !next
///     let $src = $"page{$page}.png"
///     ignore <- rebuild $page:                                # 1/#g
///         Ui.Label(text=$"Page {_}")                          # 1/#g/0
///         ignore <- rebuild $src:                             # 1/#g/1/#h
///             !started <- @started                            # 1/#g/1/#h/0
///             Test.Log(requests=(_ on !started))              # 1/#g/1/#h/1
///             Ui.Image(src=_)                                 # 1/#g/1/#h/2
/// ```
pub fn logged_pages() -> Program {
    Program::new(
        "loggedPages",
        Blueprint::pure("@loggedPages", |b, _| {
            let next = button(b, "Next")?;
            let page = b.hold(Value::int(0));
            let turned = b.sample(&next, &[page.clone()], ints("+1", |a| a[1] + 1))?;
            b.hold_on(&page, &turned)?;
            let src = b.lift(&[page.clone()], text("src", |a| format!("page{}.png", show(&a[0]))));
            let generation = Blueprint::new("page", vec![src], |b, cap, args| {
                label(b, Value::text(&format!("Page {}", show(&args[0]))))?;
                let image = Blueprint::pure("image", |b, args| {
                    let started = b.started();
                    let request = b.tag(&started, &args[0])?;
                    emit(b, test::log(), vec![("requests", request)])?;
                    emit(b, ui::image(), vec![("src", args[0].clone())])?;
                    Ok(Value::Unit)
                });
                b.rebuild(&cap[0], ResultKind::Unit, &image)?;
                Ok(Value::Unit)
            });
            b.rebuild(&page, ResultKind::Unit, &generation)?;
            Ok(Value::Unit)
        }),
    )
}

#[test]
fn a_nested_rebuild_builds_once_per_click() {
    verify(
        logged_pages(),
        r#"
test pages_build_once, @loggedPages:
    build
        => unrequested Test.Log
    tick
        => requested Test.Log, "page0.png"
    click Ui.Button(label="Next")
        => expect Ui.Image(src="page1.png")
        => expect_none Ui.Image(src="page0.png")
    tick
        => requested Test.Log, "page1.png"
"#,
        r#"test pages_build_once

tick 0
  added:
    0: Ui.Button(label="Next")
    1/#0/0: Ui.Label(text="Page 0")
    1/#0/1/#0/1: Test.Log
    1/#0/1/#0/2: Ui.Image(src="page0.png")

tick 1
  requested:
    1/#0/1/#0/1: "page0.png"

tick 2
  in:
    0: click
  added:
    1/#1/0: Ui.Label(text="Page 1")
    1/#1/1/#0/1: Test.Log
    1/#1/1/#0/2: Ui.Image(src="page1.png")
  torn down:
    1/#0/0: Ui.Label(text="Page 0")
    1/#0/1/#0/1: Test.Log
    1/#0/1/#0/2: Ui.Image(src="page0.png")

tick 3
  requested:
    1/#1/1/#0/1: "page1.png"
"#,
    );
}

/// `hold_starts_over` (Semantics ch. 7): a score clicked in the same tick as New round
/// counts in the old round, whose `round 0: 2` is never in any document.
fn rounds() -> Program {
    Program::new(
        "rounds",
        Blueprint::pure("@rounds", |b, _| {
            let again = button(b, "New round")?;
            let score = button(b, "Score")?;
            let round = b.hold(Value::int(0));
            let next = b.sample(&again, &[round.clone()], ints("+1", |a| a[1] + 1))?;
            b.hold_on(&round, &next)?;
            let block = Blueprint::new("round", vec![score], |b, cap, args| {
                let points = b.hold(Value::int(0));
                let more = b.sample(&cap[0], &[points.clone()], ints("+1", |a| a[1] + 1))?;
                b.hold_on(&points, &more)?;
                let r = show(&args[0]);
                let shown = b.lift(&[points], text("round", move |a| format!("round {r}: {}", show(&a[0]))));
                label(b, shown)?;
                Ok(Value::Unit)
            });
            b.rebuild(&round, ResultKind::Unit, &block)?;
            Ok(Value::Unit)
        }),
    )
}

#[test]
fn a_hold_starts_over_with_its_generation() {
    verify(
        rounds(),
        r#"
test hold_starts_over, @rounds:
    build
        => expect Ui.Label(text="round 0: 0")
    click Ui.Button(label="Score")
        => expect Ui.Label(text="round 0: 1")
    click Ui.Button(label="New round"), click Ui.Button(label="Score")
        => expect Ui.Label(text="round 1: 0")
        => expect_none Ui.Label(text="round 0: 2")
"#,
        r#"test hold_starts_over

tick 0
  added:
    0: Ui.Button(label="New round")
    1: Ui.Button(label="Score")
    2/#0/0: Ui.Label(text="round 0: 0")

tick 1
  in:
    1: click
  changed:
    2/#0/0: Ui.Label(text="round 0: 1")

tick 2
  in:
    0: click
    1: click
  added:
    2/#1/0: Ui.Label(text="round 1: 0")
  torn down:
    2/#0/0: Ui.Label(text="round 0: 1")
"#,
    );
}

// ---------------------------------------------------------------------------------------
// `@Switch` and `@Flatten` lag a tick (Semantics ch. 4 and 7)
// ---------------------------------------------------------------------------------------

/// ```prism
/// @birth:
///     !go <- Ui.Button(label=$"New")                                  # 0
///     $gen <- 0 then $gen + 1 on !go
///     $!starts <- rebuild $gen:                                       # 1/#g
///         @started                                                    # 1/#g/0
///     !heard <- @Switch $!starts
///     $count <- 0 then $count + 1 on !heard
///     Ui.Label(text=$"heard={$count}")                                # 2
/// ```
fn birth() -> Program {
    Program::new(
        "birth",
        Blueprint::pure("@birth", |b, _| {
            let go = button(b, "New")?;
            let generation = b.hold(Value::int(0));
            let next = b.sample(&go, &[generation.clone()], ints("+1", |a| a[1] + 1))?;
            b.hold_on(&generation, &next)?;
            let starts = b.rebuild(&generation, ResultKind::Event, &Blueprint::pure("gen", |b, _| Ok(b.started())))?;
            let heard = b.switch(&starts)?;
            let count = b.hold(Value::int(0));
            let more = b.sample(&heard, &[count.clone()], ints("+1", |a| a[1] + 1))?;
            b.hold_on(&count, &more)?;
            let shown = b.lift(&[count], text("heard", |a| format!("heard={}", show(&a[0]))));
            label(b, shown)?;
            Ok(Value::Unit)
        }),
    )
}

#[test]
fn a_switch_hears_the_birth_a_tick_late() {
    verify(
        birth(),
        r#"
test a_switch_hears_the_birth, @birth:
    build
        => expect Ui.Label(text="heard=0")
    tick
        => expect Ui.Label(text="heard=1")
    click Ui.Button(label="New")
        => expect Ui.Label(text="heard=1")
    tick
        => expect Ui.Label(text="heard=2")
"#,
        r#"test a_switch_hears_the_birth

tick 0
  added:
    0: Ui.Button(label="New")
    2: Ui.Label(text="heard=0")

tick 1
  changed:
    2: Ui.Label(text="heard=1")

tick 2
  in:
    0: click

tick 3
  changed:
    2: Ui.Label(text="heard=2")
"#,
    );
}

/// The viewer of Semantics ch. 7 ("The viewer's lag") and ch. 8: a `rebuild` of
/// `Fs.Contents` places feeds a `@Flatten`, so the label follows a tick after the watch is
/// replaced. Each place's first value comes from the first-value query.
fn viewer() -> Program {
    Program::new(
        "viewer",
        Blueprint::pure("@viewer", |b, _| {
            let path = b.hold(Value::text("/etc/hostname"));
            let key = emit(b, ui::text_input(), vec![("name", Value::text("path")), ("text", path.clone())])?;
            let edit = Closure::pure("Edit", |a| {
                let text = a[1].as_text().unwrap().clone();
                match &a[0] {
                    Value::Choice(k) if &*k.ctor == "Backspace" => Ok(Value::Text(text.take(text.len().saturating_sub(1)))),
                    Value::Choice(k) if &*k.ctor == "Letter" => Ok(Value::Text(text.concat(k.payload[0].as_text().unwrap()))),
                    _ => Ok(Value::Text(text)),
                }
            });
            let edited = b.sample(&key, &[path.clone()], edit)?;
            b.hold_on(&path, &edited)?;
            let watches = b.rebuild(&path, ResultKind::Dynamic, &Blueprint::pure("watch", |b, args| {
                contents(b, &show(&args[0]))
            }))?;
            let current = b.flatten(&watches)?;
            let shown = b.lift(&[current], text("ShowResponse", |a| show_response(&a[0])));
            label(b, shown)?;
            Ok(Value::Unit)
        }),
    )
}

#[test]
fn the_label_lags_the_watch_by_a_tick() {
    verify(
        viewer(),
        r#"
test the_label_lags_the_watch, @viewer:
    with Fs.Contents(path="/etc/hostname") = ~ Contents(text="box"), build
        => expect Ui.Label(text="box")
    with Fs.Contents(path="/etc/hostnam") = Error(kind=Fs.NotFound, message="no such file"), send Ui.TextInput(..), Backspace
        => expect Ui.Label(text="box")
        => expect Fs.Contents(path="/etc/hostnam")
    tick
        => expect Ui.Label(text="(no such file)")
"#,
        r#"test the_label_lags_the_watch

tick 0
  added:
    0: Ui.TextInput(name="path", text="/etc/hostname")
    1/#0/0: Fs.Contents(path="/etc/hostname")
    2: Ui.Label(text="box")

tick 1
  in:
    0: Backspace
  added:
    1/#1/0: Fs.Contents(path="/etc/hostnam")
  changed:
    0: Ui.TextInput(name="path", text="/etc/hostnam")
  torn down:
    1/#0/0: Fs.Contents(path="/etc/hostname")

tick 2
  changed:
    2: Ui.Label(text="(no such file)")
"#,
    );
}

#[test]
fn a_state_element_nothing_answers_aborts_its_tick() {
    // Conformance ch. 1: "A state element the program makes that no entry answers fails
    // the test at the place's creation, naming the element."
    let r = TestWorld::new().program(viewer()).run(
        r#"
test unanswered, @viewer:
    build
"#,
    );
    assert_eq!(r.exit_code, 1);
    assert!(r.transcript.contains("  aborted\n  | World.Unanswered"), "{}", r.transcript);
    assert!(r.failures[0].contains("Fs.Contents(path=\"/etc/hostname\")"), "{:?}", r.failures);
}

// ---------------------------------------------------------------------------------------
// Zombies (Semantics ch. 7, "Holds die with their widget"; "A detached widget runs to the
// end of its tick")
// ---------------------------------------------------------------------------------------

thread_local! {
    static BUMPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Two rows whose holds listen to each other (through the `for`'s gathered result) and to
/// a live Clear button. Deleted, they run nothing on Clear.
fn zombie_rows() -> Program {
    Program::new(
        "rows",
        Blueprint::pure("@rows", |b, _| {
            let clear = button(b, "Clear")?;
            let delete = button(b, "Delete all")?;
            let items = b.hold(Value::map(vec![(Value::int(1), Value::text("a")), (Value::int(2), Value::text("b"))]));
            let emptied = b.tag(&delete, &Value::map(vec![]))?;
            b.hold_on(&items, &emptied)?;
            let counts = b.hold(Value::Unit);
            let row = Blueprint::new("row", vec![clear, counts.clone()], |b, cap, args| {
                let n = b.hold(Value::int(0));
                // `$n <- 0 then (Bump $n, $counts) on !clear`: reads the other rows too.
                let bump = Closure::pure("Bump", |a| {
                    BUMPS.with(|c| c.set(c.get() + 1));
                    Ok(Value::int(int(&a[1]) as i64 + 1))
                });
                let next = b.sample(&cap[0], &[n.clone(), cap[1].clone()], bump)?;
                b.hold_on(&n, &next)?;
                let name = show(&args[1].clone());
                let item = args[1].clone();
                let shown = b.lift(&[item, n.clone()], text("row", |a| format!("{}: {}", show(&a[0]), show(&a[1]))));
                let _ = name;
                label(b, shown)?;
                Ok(n)
            });
            let gathered = b.for_each(&items, ResultKind::Dynamic, &row)?;
            // The rows read `$counts`, the gathered result, through a hold that follows it.
            let follow = b.updated(&gathered);
            b.hold_on(&counts, &follow)?;
            let shown = b.lift(&[gathered], text("counts", |a| show(&a[0])));
            label(b, shown)?;
            Ok(Value::Unit)
        }),
    )
}

#[test]
fn zombies_run_nothing() {
    BUMPS.with(|c| c.set(0));
    verify(
        zombie_rows(),
        r#"
test deleted_rows_run_nothing_on_clear, @rows:
    build
    click Ui.Button(label="Clear")
        => expect Ui.Label(text="a: 1")
    click Ui.Button(label="Clear"), click Ui.Button(label="Delete all")
        => expect_none Ui.Label(text="a: 2")
    click Ui.Button(label="Clear")
        => expect Ui.Label(text="{}")
"#,
        r#"test deleted_rows_run_nothing_on_clear

tick 0
  added:
    0: Ui.Button(label="Clear")
    1: Ui.Button(label="Delete all")
    2/[1]/0: Ui.Label(text="a: 0")
    2/[2]/0: Ui.Label(text="b: 0")
    3: Ui.Label(text="{1: 0, 2: 0}")

tick 1
  in:
    0: click
  changed:
    2/[1]/0: Ui.Label(text="a: 1")
    2/[2]/0: Ui.Label(text="b: 1")
    3: Ui.Label(text="{1: 1, 2: 1}")

tick 2
  in:
    0: click
    1: click
  changed:
    3: Ui.Label(text="{}")
  torn down:
    2/[1]/0: Ui.Label(text="a: 1")
    2/[2]/0: Ui.Label(text="b: 1")

tick 3
  in:
    0: click
"#,
    );
    // Two rows bumped in tick 1 and, running to the end of their tick, in tick 2; none in
    // tick 3.
    assert_eq!(BUMPS.with(|c| c.get()), 4);
}

// ---------------------------------------------------------------------------------------
// `@Defer` and `@Run`: compute ticks with exact counts (decisions.md D6)
// ---------------------------------------------------------------------------------------

/// `@Normal` of Semantics ch. 7, on a term two reductions from normal form. The cycle
/// stands for `Normalize`'s bounded body between two cycle points.
fn normal() -> Program {
    Program::new(
        "normal",
        Blueprint::pure("@Normal", |b, _| {
            let cycle = Closure::pure("Normalize", |a| {
                let k = int(&a[0]);
                Ok(if k > 0 {
                    Value::ctor("Continue", vec![Value::int(k as i64 - 1)])
                } else {
                    Value::ctor("Done", vec![Value::text("Lam (Var 0)")])
                })
            });
            let done = b.run(cycle, Value::int(2))?;
            let shown = b.hold_then(Value::text("working"), &done)?;
            label(b, shown)?;
            Ok(Value::Unit)
        }),
    )
}

#[test]
fn run_takes_three_compute_ticks() {
    verify(
        normal(),
        r#"
test normalize_in_three_cycles, @normal:
    build
        => expect Ui.Label(text="working")
    compute until idle
        => expect Ui.Label(text="Lam (Var 0)")
    compute until idle
"#,
        r#"test normalize_in_three_cycles

tick 0
  added:
    1: Ui.Label(text="working")

compute until idle: 3

tick 1

tick 2

tick 3
  changed:
    1: Ui.Label(text="Lam (Var 0)")

compute until idle: 0
"#,
    );
}

#[test]
fn run_by_single_compute_steps() {
    verify(
        normal(),
        r#"
test normalize_by_steps, @normal:
    build
    tick
    compute
        => expect Ui.Label(text="working")
    compute
        => expect Ui.Label(text="Lam (Var 0)")
"#,
        r#"test normalize_by_steps

tick 0
  added:
    1: Ui.Label(text="working")

tick 1

tick 2

tick 3
  changed:
    1: Ui.Label(text="Lam (Var 0)")
"#,
    );
}

/// `!again <- @Defer !click`: each click heard again in the next tick.
fn echo() -> Program {
    Program::new(
        "echo",
        Blueprint::pure("@echo", |b, _| {
            let click = button(b, "Echo")?;
            let again = b.defer(&click)?;
            let heard = b.hold(Value::int(0));
            let more = b.sample(&again, &[heard.clone()], ints("+1", |a| a[1] + 1))?;
            b.hold_on(&heard, &more)?;
            let shown = b.lift(&[heard], text("heard", |a| format!("heard {}", show(&a[0]))));
            label(b, shown)?;
            Ok(Value::Unit)
        }),
    )
}

#[test]
fn defer_fires_in_the_next_tick() {
    verify(
        echo(),
        r#"
test deferred_echo, @echo:
    build
    click Ui.Button(label="Echo")
        => expect Ui.Label(text="heard 0")
    compute
        => expect Ui.Label(text="heard 1")
    compute until idle
"#,
        r#"test deferred_echo

tick 0
  added:
    0: Ui.Button(label="Echo")
    1: Ui.Label(text="heard 0")

tick 1
  in:
    0: click

tick 2
  changed:
    1: Ui.Label(text="heard 1")

compute until idle: 0
"#,
    );
}

// ---------------------------------------------------------------------------------------
// Host documents (Semantics ch. 9)
// ---------------------------------------------------------------------------------------

/// A child that watches `/etc/motd` and counts.
fn motd_child() -> Blueprint {
    Blueprint::pure("@child", |b, _| {
        let inc = button(b, "Increment")?;
        let c = b.hold(Value::int(0));
        let next = b.sample(&inc, &[c.clone()], ints("+1", |a| a[1] + 1))?;
        b.hold_on(&c, &next)?;
        let motd = contents(b, "/etc/motd")?;
        let shown = b.lift(&[c, motd], text("child", |a| format!("{} {}", show(&a[0]), show_response(&a[1]))));
        label(b, shown)?;
        Ok(Value::Unit)
    })
}

/// ```prism
/// @host:
///     ((ignore, $doc)) <- @HostDocument @child, \|p, e| match e:
///         Fs.Contents(..): @ $ (~ Contents(text="mock"))
///         otherwise: @Element e
///     Ui.Label(text=$"{Size $doc} places")
/// ```
fn mocking_host() -> Program {
    Program::new(
        "host",
        Blueprint::pure("@host", |b, _| {
            let supplier = Blueprint::pure("supplier", |b, args| {
                let Value::Element(e) = &args[1] else { unreachable!() };
                if &*e.tag == "Fs.Contents" {
                    Ok(b.constant(Value::ok(prism_rt::families::fs::contents_value("mock"))))
                } else {
                    b.emit((**e).clone())
                }
            });
            let hosted = b.host_document(&motd_child(), &[], &supplier)?;
            let Value::Tuple(pair) = hosted else { unreachable!() };
            let shown = b.lift(&[pair[1].clone()], text("size", |a| match &a[0] {
                Value::Map(m) => format!("{} places", m.len()),
                _ => "?".into(),
            }));
            label(b, shown)?;
            Ok(Value::Unit)
        }),
    )
}

#[test]
fn a_host_answers_each_child_place_with_a_supplier() {
    verify(
        mocking_host(),
        r#"
test a_supplier_per_child_place, @host:
    build
        => expect_none Fs.Contents(..)
    click at 0/[0]/0
        => expect Ui.Label(text="1 mock")
"#,
        r#"test a_supplier_per_child_place

tick 0
  added:
    0/[0]/0: Ui.Button(label="Increment")
    0/[2]/0: Ui.Label(text="0 mock")
    1: Ui.Label(text="3 places")

tick 1
  in:
    0/[0]/0: click
  changed:
    0/[2]/0: Ui.Label(text="1 mock")
"#,
    );
}

/// `@HostDocument @loggedPages, \|p, e| @Element e`: a host that forwards everything.
fn forwarding_host() -> Program {
    Program::new(
        "forwarded",
        Blueprint::pure("@forwarded", |b, _| {
            let forward = Blueprint::pure("forward", |b, args| {
                let Value::Element(e) = &args[1] else { unreachable!() };
                b.emit((**e).clone())
            });
            let pages = logged_pages().root;
            b.host_document(&pages, &[], &forward)?;
            Ok(Value::Unit)
        }),
    )
}

/// Read a hosted transcript's paths back: `0/[p]/0` is the child's `p`.
fn unhost(transcript: &str) -> String {
    transcript
        .lines()
        .map(|l| match l.strip_prefix("    0/[").and_then(|r| r.split_once("]/0: ")) {
            Some((p, rest)) => format!("    {p}: {rest}"),
            None => l.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_host_that_forwards_everything_is_not_there() {
    // Semantics ch. 9: "the same answers in the same ticks, under deeper paths outside";
    // teardown composes out to the World with no code.
    let script =
        |p: &str, next: &str| format!("test transparent, @{p}:\n    build\n    tick\n    click at {next}\n    tick\n");
    let world = TestWorld::new().program(logged_pages()).program(forwarding_host());
    let bare = world.run(&script("loggedPages", "0"));
    let hosted = world.run(&script("forwarded", "0/[0]/0"));
    assert_eq!(bare.exit_code, 0, "{:?}", bare.failures);
    assert_eq!(hosted.exit_code, 0, "{:?}", hosted.failures);
    assert!(hosted.transcript.contains("0/[1/#1/1/#0/1]/0: Test.Log"), "{}", hosted.transcript);
    assert!(hosted.transcript.contains("torn down:\n    0/[1/#0/0]/0"), "{}", hosted.transcript);
    assert_eq!(unhost(&hosted.transcript), unhost(&bare.transcript));
}

// ---------------------------------------------------------------------------------------
// A failed tick never happened (Semantics ch. 9, ch. 10; Conformance ch. 1, doubt AB)
// ---------------------------------------------------------------------------------------

fn fragile() -> Program {
    Program::new(
        "fragile",
        Blueprint::pure("@fragile", |b, _| {
            let inc = button(b, "Increment")?;
            let other = button(b, "Other")?;
            let count = b.hold(Value::int(0));
            let checked = Closure::pure("Checked", |a| {
                let n = int(&a[1]) + 1;
                if n > 2 {
                    Err(Fault::new("Test.Overflow", "the count would pass 2"))
                } else {
                    Ok(Value::int(n as i64))
                }
            });
            let next = b.sample(&inc, &[count.clone()], checked)?;
            b.hold_on(&count, &next)?;
            let others = b.hold(Value::int(0));
            let more = b.sample(&other, &[others.clone()], ints("+1", |a| a[1] + 1))?;
            b.hold_on(&others, &more)?;
            let shown = b.lift(&[count, others], text("both", |a| format!("{} {}", show(&a[0]), show(&a[1]))));
            label(b, shown)?;
            Ok(Value::Unit)
        }),
    )
}

#[test]
fn a_failed_tick_leaves_the_state_before_it() {
    verify(
        fragile(),
        r#"
test a_failed_tick_never_happened, @fragile:
    build
    click at 0
    click at 0
    click at 0, click at 1
        => aborted
    click at 1
        => expect Ui.Label(text="2 1")
"#,
        r#"test a_failed_tick_never_happened

tick 0
  added:
    0: Ui.Button(label="Increment")
    1: Ui.Button(label="Other")
    2: Ui.Label(text="0 0")

tick 1
  in:
    0: click
  changed:
    2: Ui.Label(text="1 0")

tick 2
  in:
    0: click
  changed:
    2: Ui.Label(text="2 0")

tick 3
  in:
    0: click
    1: click
  aborted
  | Test.Overflow: the count would pass 2

tick 4
  in:
    1: click
  changed:
    2: Ui.Label(text="2 1")
"#,
    );
}

#[test]
fn an_unexpected_abort_fails_the_test() {
    let r = TestWorld::new().program(fragile()).run(
        "test t, @fragile:\n    build\n    click at 0\n    click at 0\n    click at 0\n",
    );
    assert_eq!(r.exit_code, 1);
    assert!(r.failures[0].contains("the tick aborted"), "{:?}", r.failures);
}

// ---------------------------------------------------------------------------------------
// `Draw.Clickable(rect, path)` in a scene (decisions.md D9)
// ---------------------------------------------------------------------------------------

fn window() -> Program {
    Program::new(
        "window",
        Blueprint::pure("@window", |b, _| {
            let target = prism_rt::pod::read_path("1/#0/0").unwrap();
            let scene = Value::array(vec![prism_rt::document::clickable(0, 0, 10, 10, target)]);
            let press = emit(b, gui::window(), vec![("title", Value::text("w")), ("scene", scene)])?;
            let shown = b.map(&press, Closure::pure("Show", |a| Ok(Value::text(&a[0].to_string()))))?;
            let last = b.hold_then(Value::text("none"), &shown)?;
            label(b, last)?;
            Ok(Value::Unit)
        }),
    )
}

#[test]
fn a_click_by_path_reaches_the_window_through_the_scene() {
    verify(
        window(),
        r#"
test click_through_the_scene, @window:
    build
    click at 1/#0/0
        => expect Ui.Label(text="Press(x=5, y=5)")
"#,
        r#"test click_through_the_scene

tick 0
  added:
    0: Gui.Window(title="w", scene=[[Draw.Clickable(rect=Rect(x=0, y=0, w=10, h=10), path=1/#0/0)]])
    1: Ui.Label(text="none")

tick 1
  in:
    0: Press(x=5, y=5)
  changed:
    1: Ui.Label(text="Press(x=5, y=5)")
"#,
    );
}
