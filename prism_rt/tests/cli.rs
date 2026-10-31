//! The command line's World over an in-memory file system: `Fs`, `Process` and `Time`,
//! the keep-ticking promise, and the CLI World's exit codes (Semantics ch. 9).

// Programs here pass a dynamic as `&[d.clone()]`, as an interpreter would.
#![allow(clippy::cloned_ref_to_slice_refs)]

mod common;

use common::*;
use prism_rt::families::{fs, process};
use prism_rt::world::cli::{CliWorld, MemFs};
use prism_rt::world::{drive, EXIT_FAILED};
use prism_rt::{Blueprint, Closure, Fault, Program, Value};

/// Uppercase `in.txt` into `out.txt`, exit 0 once written, or 3 if `in.txt` is missing.
///
/// ```prism
/// @main:
///     !started <- @started                                      # 0
///     $~in <- Fs.Contents(path="in.txt")                        # 1
///     $state <- Fs.Output(path="out.txt", text=Upper $~in)      # 2
///     let !written = !Present (?Written (!Updated $state))
///     let !missing = !Filter (?Missing $~in on !started)
///     Process.Exit(code=(!missing <| 0 on !written))            # 3
/// ```
fn upper() -> Program {
    Program::new(
        "upper",
        Blueprint::pure("@main", |b, _| {
            let started = b.started();
            let input = contents(b, "in.txt")?;
            let upper = b.lift(&[input.clone()], text("Upper", |a| match &a[0] {
                Value::Fallible(Ok(c)) => show(c.field("text").unwrap()).to_uppercase(),
                _ => String::new(),
            }));
            let state = emit(b, fs::output(), vec![("path", Value::text("out.txt")), ("text", upper)])?;
            let updates = b.updated(&state);
            let written = b.map(&updates, Closure::pure("?Written", |a| {
                Ok(if a[0] == fs::written() { Value::some(Value::Unit) } else { Value::nil() })
            }))?;
            let written = b.present(&written)?;
            let ok = b.tag(&written, &Value::U8(0))?;
            let missing = b.sample(&started, &[input], Closure::pure("?Missing", |a| {
                Ok(if matches!(a[1], Value::Fallible(Err(_))) { Value::some(Value::U8(3)) } else { Value::nil() })
            }))?;
            let missing = b.filter(&missing)?;
            let code = b.merge(&[missing, ok])?;
            emit(b, process::exit(), vec![("code", code)])?;
            Ok(Value::Unit)
        }),
    )
}

#[test]
fn a_file_is_read_written_and_the_program_exits_0() {
    let mut files = MemFs::default();
    files.files.insert("in.txt".into(), "hello".into());
    let mut world = CliWorld::new(files, vec![]);
    let code = drive(&mut world, &upper(), 100);
    assert_eq!(code, 0);
    assert_eq!(world.fs.borrow().files.get("out.txt").map(String::as_str), Some("HELLO"));
}

#[test]
fn a_missing_file_exits_3() {
    let mut world = CliWorld::new(MemFs::default(), vec![]);
    assert_eq!(drive(&mut world, &upper(), 100), 3);
}

/// Exit with the number of arguments: `Process.Args` is plain, answered once.
fn count_args() -> Program {
    Program::new(
        "args",
        Blueprint::pure("@main", |b, _| {
            let started = b.started();
            let args = emit(b, process::args(), vec![])?;
            let n = match &args {
                Value::Array(a) => a.len() as u8,
                _ => return Err(Fault::runtime("args")),
            };
            let code = b.tag(&started, &Value::U8(n))?;
            emit(b, process::exit(), vec![("code", code)])?;
            Ok(Value::Unit)
        }),
    )
}

#[test]
fn process_args_are_plain() {
    let mut world = CliWorld::new(MemFs::default(), vec!["a".into(), "b".into()]);
    assert_eq!(drive(&mut world, &count_args(), 10), 2);
}

/// `@Later` rides on `Time.After`: the World answers the wake in a later tick.
fn later() -> Program {
    Program::new(
        "later",
        Blueprint::pure("@main", |b, _| {
            let started = b.started();
            let seven = b.tag(&started, &Value::U8(7))?;
            let late = b.later(&seven)?;
            emit(b, process::exit(), vec![("code", late)])?;
            Ok(Value::Unit)
        }),
    )
}

#[test]
fn later_goes_through_the_world() {
    let mut world = CliWorld::new(MemFs::default(), vec![]);
    assert_eq!(drive(&mut world, &later(), 10), 7);
}

/// A program with nothing to do ends with 0 when nothing can cause another tick; one with
/// an unread `@started` owes nothing (prism-reflex ch. 6, "The exit rule's four lines").
#[test]
fn an_idle_program_exits_0() {
    let idle = Program::new(
        "idle",
        Blueprint::pure("@main", |b, _| {
            b.started();
            label(b, Value::text("nothing"))?;
            Ok(Value::Unit)
        }),
    );
    let mut world = CliWorld::new(MemFs::default(), vec![]);
    assert_eq!(drive(&mut world, &idle, 10), 0);
}

#[test]
fn a_failed_tick_exits_1() {
    let failing = Program::new(
        "failing",
        Blueprint::pure("@main", |b, _| {
            let started = b.started();
            let boom = b.map(&started, Closure::pure("boom", |_| Err(Fault::new("Test.Boom", "no"))))?;
            emit(b, process::exit(), vec![("code", boom)])?;
            Ok(Value::Unit)
        }),
    );
    let mut world = CliWorld::new(MemFs::default(), vec![]);
    assert_eq!(drive(&mut world, &failing, 10), EXIT_FAILED);
}

/// `@Run` keeps a CLI World ticking while `pending` is set, with no inputs.
#[test]
fn pending_compute_keeps_the_world_ticking() {
    let counting = Program::new(
        "counting",
        Blueprint::pure("@main", |b, _| {
            let cycle = Closure::pure("CountDown", |a| {
                let k = int(&a[0]);
                Ok(if k > 0 {
                    Value::ctor("Continue", vec![Value::int(k as i64 - 1)])
                } else {
                    Value::ctor("Done", vec![Value::U8(42)])
                })
            });
            let done = b.run(cycle, Value::int(20))?;
            emit(b, process::exit(), vec![("code", done)])?;
            Ok(Value::Unit)
        }),
    );
    let mut world = CliWorld::new(MemFs::default(), vec![]);
    assert_eq!(drive(&mut world, &counting, 100), 42);
    let mut impatient = CliWorld::new(MemFs::default(), vec![]);
    assert_eq!(drive(&mut impatient, &counting, 5), EXIT_FAILED, "a World's patience is its own");
}
