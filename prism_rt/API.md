# `prism_rt`: the API an interpreter calls

*Written by Claude, 31 October 2026, on the `claude-self-hosting` branch, as the
interface between `prism_rt` and compiler zero (and, later, `prismc`'s generated code).
Subject to change: implementers use their discretion. The books in `prism-notes` are the
specification; each item cites the rule it keeps in the crate's doc comments.*

## The shape of it

An interpreter turns a widget block into a [`Blueprint`]: a Rust closure that, given a
[`Build`], runs the block's statements as `Build` methods and returns the block's result.
A program is its root blueprint. A World calls [`tick`] on the program's state, once per
tick, and the runtime does the rest: propagation by height, construction at the end of the
tick, the patch, the requests.

```rust
use prism_rt::{tick, Blueprint, Element, Inputs, Program, Value};
use prism_rt::families::ui;

// @halfcounter:
//     !increment <- Ui.Button(label=$"Increment")
//     let !delta = +1 on !increment
//     $count <- 0 then $count + !delta
//     Ui.Label(text=$"Count: {$count}")
let root = Blueprint::pure("@halfcounter", |b, _args| {
    let increment = b.emit(Element::new(&ui::button(), vec![("label", Value::text("Increment"))])?)?;
    let delta = b.tag(&increment, &Value::int(1))?;
    let count = b.hold(Value::int(0));
    let next = b.sample(&delta, &[count.clone()], add)?;   // add: a Closure
    b.hold_on(&count, &next)?;
    let shown = b.lift(&[count], format_count);             // format_count: a Closure
    b.emit(Element::new(&ui::label(), vec![("text", shown)])?)?;
    Ok(Value::Unit)
});
let program = Program::new("halfcounter", root);
let built = tick(&program, None, &Inputs::none());          // tick 0, the build
```

## Values (`value`)

| Item | What it is |
| --- | --- |
| `Value` | The dynamic value: `Unit`, `Bool`, `Nat(BigInt)`, `Int(BigInt)`, `U8`, `Char`, `Text`, `Duration`, `Record`, `Tuple`, `Choice`, `Map(OrdMap)`, `Array(Vector)`, `Opt` (`?`), `Fallible` (`~`, an `Error` record on failure), `Closure`, `Element`, `Path`, and the handles `Event(NodeId)` (`!`), `Dynamic(NodeId)` (`$`), `Widget(Blueprint)` (`@`), `Ref` (`&`). |
| `Value::{nat, int, text, some, nil, ok, error, record, ctor, tuple, array, map, element, click}` | Constructors. `error(kind, message)` is a failed `~T`. |
| `Value::{field, as_bool, as_i128, as_text, as_element, is_plain}` | Accessors. |
| `Text` | A rope: `len` constant; `take`, `drop_front`, `concat`, `char_at`. |
| `Record`, `Choice` | Named fields in declaration order; a constructor and its payload. |
| `Closure::new(name, captured, f)`, `Closure::pure(name, f)`, `.call(args)` | A function value: `f(captured, args) -> Result<Value, Fault>`. Captured values are explicit so the collector traces them. Compares by identity. |
| `Blueprint::new(name, captured, f)`, `Blueprint::pure(name, f)` | An `@` value: `f(&mut Build, captured, args) -> Result<Value, Fault>`. |
| `Ref::new(v)`, `.get()`, `.set(v)` | An `&` cell for `&` blocks. |
| `TagDecl::new(name, fields, output)` | `tag Fam.Tag of Element o: fields`. `FieldKind::{Identity, Live, Request}`; `OutputKind::{Unit, Event, Dynamic, Plain}`. |
| `Element::new(&decl, fields)` | An element of a declared tag; live fields take a `Value::Dynamic` or a plain value, request fields a `Value::Event` (or nothing, for `never`). |
| `Path`, `Segment::{Pos, Gen, Key}` | Paths; ordered as the book orders them. |
| `Fault { kind, message }` | A failure that escapes a tick: the tick never happened. Return it from a closure or a blueprint. |
| `nat_checked_sub`, `is_zero` | `Nat`'s `-?`, and a zero test. |

`families::{ui, fs, process, time, test, gui}` hold the tag declarations the Worlds answer
(`ui::button()`, `fs::contents()`, `process::exit()`, ...) and the values of their answers
(`fs::contents_value(text)`, `fs::read()`, `fs::write(text)`, `gui::press(x, y)`).

## Building a widget (`widget::Build`)

Every method is one statement of a widget block. Positions are taken by `emit`,
`rebuild`, `for_each`, `started`, `host_document` (and its forms) and `run`; nothing else
takes one, and `call` is transparent.

| Method | Prism |
| --- | --- |
| `set_emit(Row)` | `emit Ui.* \| Fs.*` as the block's first statement. `Row::Any`, `Row::of(&["Ui", "Fs"])`. |
| `emit(Element) -> Value` | An element statement; returns its output: `unit`, an event, a dynamic or a plain value. In a host's child document the host's supplier answers it. |
| `call(&Blueprint, args) -> Value` | Applying a widget function in this block: transparent. |
| `started() -> Value` | `@started`. |
| `hold(init) -> Value`, `hold_on(&hold, &event)`, `hold_then(init, &event) -> Value` | `$x <- init then !e`. `hold` then `hold_on` lets a line read a hold defined below it. |
| `forward_event() -> Value`, `resolve_event(&fwd, &event)` | An event bound below the line that reads it. |
| `constant(v) -> Value`, `never() -> Value` | `$ v`, `never`. |
| `map(&event, f)` | A function over one event. |
| `sample(&event, &[dyns], f)` | Lifting over an event and dynamics: `f(v, d1, ..)`, the dynamics as the tick opened. |
| `tag(&event, &x)` | `x on !e`. |
| `lift(&[dyns], f) -> Value` | Lifting over dynamics, the change law. |
| `merge(&[events])` | `a <\| b <\| ..`, the leftmost merge. |
| `combine(&[events], f)`, `union(&a, &b)` | Several events in one tick: `f` gets `? v` or `nil` per input and returns `? r` or `nil`. `!Union` is provided. |
| `filter(&e)`, `present(&e)`, `gate(&cond, &e)`, `updated(&dyn)` | `!Filter`, `!Present`, `!Gate`, `!Updated`. |
| `read(&dyn) -> Value` | A dynamic's value after this tick, the value construction reads. |
| `dynamic_of(&v) -> NodeId` | The node behind a `$` position (a constant for a plain value). |
| `switch(&outer)`, `flatten(&outer)` | `@Switch $!d`, `@Flatten $$d`. |
| `defer(&e)` | `@Defer !e`. |
| `machine(step, init, &e)` | `@Machine step, init, !e`; `step(s, a)` returns `((s', ?b))`. |
| `run(cycle, start)` | `@Run f, x` over `@Machine` and `@Defer`; `cycle(state)` returns `Continue next` or `Done result`, one cycle per tick. |
| `later(&e)` | `@Later !e`, through `Time.After`. |
| `rebuild(&src, ResultKind, &bp)` | `rebuild $x:`; `bp` gets `_`. Returns `unit` or a dynamic of the current generation's result. |
| `for_each(&src, ResultKind, &bp)` | Keyed `for`; `bp` gets the key, the row's value (a dynamic over a dynamic collection) and `ix`. Returns `unit`, `!(K ==> T)`, or `$(K ==> T)`. Over a plain collection, built once. |
| `host_document(&child, args, &supplier)` | `@HostDocument`; `supplier` gets `(Path, Element)`. Returns `((result, $doc))`. |
| `host_some(&child, args, &give)`, `host_list(&child, args, &answers, &supplier)` | `@HostSome` (`give` returns `? @b` or `nil` to forward), `@HostList`. |
| `tick()`, `next_path()` | The construction's tick, and the path the next positioned statement takes. |

`ResultKind::{Unit, Event, Dynamic, Plain}` says what a `rebuild`'s block or a `for`'s rows
end on; compiler zero has no checker (D3), so the interpreter reads it off the binder's
sigil.

## The tick (`tick`)

| Item | What it is |
| --- | --- |
| `Program::new(name, root)` | A program: its root blueprint. |
| `tick(&Program, Option<State>, &Inputs) -> Ticked` | `tick: ?State, inputs -> State, outputs, pending`. `None` is the build. |
| `Ticked { state, outputs: Result<Outputs, Fault>, pending }` | On a fault, `state` is the state given (the tick never happened). |
| `Inputs { answers, pushes, ready }`, `Inputs::none()`, `with_ready(rc)`, `.answer(path, v)`, `.push(element, v)` | Answers by path, pushes keyed by element `Level`, and the ready state. |
| `ReadyState::first_value(&level) -> Option<Value>` | The World's ready state; `ReadyMap(OrdMap)` and `NoReadyState` are provided. |
| `Outputs { tick, patch, requests, exits, result }` | `Patch { added, changed, removed }`, each in path order; `Request { path, tag, field, value, .. }` in document order; `exits`, the `Process.Exit` codes; `result: Option<ResultReport>`. |
| `State::{document, live_cells, live_nodes, pending, tick_number}` | The document the World last saw, the debug allocator's counts, and the `pending` flag. `State` clones in constant time. |

## The collector (`gc`)

| Item | What it is |
| --- | --- |
| `gc::collect(&mut State) -> Collected` | Mark-sweep rooted at the widget tree; run between ticks. |
| `gc::debug::live_functions()` | Closures and blueprints alive on this thread. |

## Worlds (`world`)

| Item | What it is |
| --- | --- |
| `World` trait: `ready`, `perform`, `next_inputs`, `exit_code`, `failed` | A foreign World as the driver sees it. |
| `world::drive(&mut dyn World, &Program, patience) -> u8` | Build, then tick while the World has inputs or the program is `pending`; collects after every tick. |
| `world::{EXIT_FAILED, EXIT_UNCHECKED}` | 1 and 2, CLI World policy. |
| `world::cli::{CliWorld, MemFs, RealFs, FileSystem}` | `Fs.Contents`, `Fs.Output`, `Fs.File`, `Process.Args`, `Process.Exit`, `Time.After`. |
| `world::test_world::TestWorld::{new, program, evaluator, run, verify}` | Runs a test-language script; `verify` compares the transcript byte for byte. `Report { transcript, failures, exit_code }`. |
| `world::test_world::print_tick` | One tick in the transcript's format. |

## `Pod` (`pod`)

| Item | What it is |
| --- | --- |
| `pod::print(&Value)`, `pod::print_path(&Path)` | The transcript's spelling. |
| `pod::read(&str)`, `pod::read_path(&str)` | Reading it back. |
| `pod::read_pattern(&str) -> Pattern`, `Pattern::matches(&Value)` | The test language's patterns: `ignore`, `..`, `Fam.*`. |

## Document helpers (`document`)

`document::clickable(x, y, w, h, path)` makes the `Draw.Clickable` scene item (D9);
`document::find_clickable` and `document::elements_within` are what the Test World uses
to resolve clicks and match checks at any depth.
