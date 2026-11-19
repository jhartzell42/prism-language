# A Test World written in Prism

*Written by Claude on the claude-self-hosting branch, 3 November 2026, for Jimmy's audit;
subject to change. Why the Test World cannot be a Prism program today, and what would let
it be.*

## The goal

- **The Test World is a World like any other** (Semantics ch. 9: "no privileged World"),
  so it should be writable in the language, the way `prismc` is.
- **What that buys:** one Test World for every implementation (Rust, prism-reflex),
  with no per-backend transcript printer to keep byte-identical; the test language's
  meaning stated as a Prism program instead of prose in Conformance ch. 1; and a check
  that the World rules are expressible at all, since a World the language cannot write is
  a hint that the rules depend on something outside it.
- **The constraint it puts on everything else:** what a transcript shows (which ticks
  exist, which are owed, what each patch holds) must be a function of the program under
  the language's rules, never of an implementation's analysis. Semantics round 3 TK refuses
  skipping an owed tick that nothing reads for this reason.

## What the Test World does today

`runtime/rust/src/world/test_world.rs`, about 800 lines of Rust:

- **Drives ticks.** It holds the program under test as a value and calls `tick` on it:
  `build`, one tick per script line, `compute` (a tick with no inputs), `compute until
  idle` (tick while `pending`, count them).
- **Answers first-value queries** synchronously from `with` and `push` entries, then from
  its picture policy (`Gui.Window`, `Gui.Metrics` from a fixed font).
- **Resolves clicks** by path or by element, through the scene's `Draw.Clickable` tags.
- **Prints the transcript**: each tick's inputs and its patch (`added`, `changed`, `torn
  down`, `requested`, `result`) as plain data, in the transcript spelling.
- **Runs `check` blocks** and reports failed `=> expect` lines.

## Why it cannot be Prism today

- **Hosting is one graph and one tick** (Semantics ch. 9, round 3 HO). A host sees its
  child's document as a dynamic and answers its places, but the child's ticks are the
  host's ticks. A tester needs the opposite: the program under test ticks only when the
  tester says so, and the tester sees each of those ticks as an event of its own. No
  hosting form gives a child its own clock.
- **`pending` is not observable from inside.** An owed tick is the whole program's; a
  Prism tester cannot ask "does the program under test owe a tick?" because, hosted, the
  owed tick would be the tester's own.
- **The patch is not a value.** A World receives the patch as plain data (`Pod` levels by
  path). Inside Prism a host sees `$(Path ==> Elem c)` snapshots, typed by the child's
  row, which is close, but it sees them as a dynamic, not as the per-tick difference
  (`added`, `changed`, `torn down`) the transcript prints, and not with the paths a World
  sees.
- **A first-value query is a synchronous call into the World mid-construction.** A
  tester written in Prism answers a hosted child's first values already (a supplier's
  dynamic output, read in the birth tick), so this part works; it fails only because of
  the first bullet.
- **The Prism World is out of this compiler** (`claude/decisions.md`, "Features in and
  out"), and the books describe it only in a sentence (Semantics ch. 9: "one graph can be
  another graph's World").

## Options

- **(a) Programs as values.** The compiler exposes a module's root as a pure value:
  `Start: fn Program p -> State p`, `Tick: fn State p, Inputs p -> ((State p, Outputs
  p))`, where `Outputs` holds the patch, requests, result and `pending`. A Test World is
  then an ordinary `unbounded` function from a script to a transcript, written like
  `prismc`.
  - *Cost:* the state of a running program must be a Prism value, so the runtime's
    mutable nodes become something a function can hold and return. Either a persistent
    representation (expensive: every node update is a path copy), or a uniquely owned
    handle threaded through `&` and `update` (cheap, and the borrow rule already exists
    for `&` blocks).
  - *Buys:* the test language's meaning as a pure function, the strongest form of the
    goal.
- **(b) A Prism World form.** A hosting form that runs its child as its own graph:
  `@World @child, policy` where the host's events drive the child's ticks (`!step`, one
  child tick per firing), the child's patch arrives as `!Patch` per child tick, and its
  `pending` as `$owed`. The tester is then a widget whose `@Machine` walks the script.
  - *Cost:* a second clock inside one program, which the Graph book has to give a
    meaning (the child's ticks nest inside the host's, or interleave with them), and the
    Semantics book has to state how a child's tick relates to the host's tick it runs in.
  - *Buys:* Worlds written in Prism in general (mocks of a whole OS, recorded sessions,
    replays), with the Test World as the first one.
- **(c) A specified Test World, implemented natively.** Keep the Test World foreign, and
  give Conformance a precise specification of it, as data: the transcript as a function
  of the script and the program's `tick` outputs, with no implementation choices left.
  - *Cost:* nothing new in the language; every backend still writes its own printer.
  - *Buys:* the transcript's meaning pinned, without the goal itself.

## Claude's recommendation

- **(a), with the uniquely owned handle.** It needs no second clock and no new hosting
  form, and it reuses machinery the language already has (`&` references, `update`
  blocks, `unbounded` functions, `@Run` for a long script). It also gives `prismc` a way
  to test itself in-process later. (b) is the better long-term feature but asks the Graph
  and Semantics books for a nested clock, a bigger design question than the Test World
  needs. (c) is worth doing anyway, in Conformance, as the reference the Prism Test World
  is checked against.
- **What it would take in this compiler:** a `Program p` type and the two functions in
  the prelude; lowering a root to a node tree reachable through a handle; the handle as
  an `&` value so a pure function can step it; `Pod` and `Path` as Prism types (they are
  half there: `Pod` exists as a trait); the transcript printer moved from Rust into a
  Prism module beside the test-language parser.
