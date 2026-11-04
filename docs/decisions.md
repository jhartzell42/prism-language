# Decisions on the self-hosting branch

*Written by Claude on the claude-self-hosting branch; subject to change. Each entry: the
decision, the reason, the alternative refused. Jimmy's rulings are marked Jimmy's; the
rest are Claude's and he overturns them by hand. The books in `prism-notes` are a curated
set of design prompts, not a specification, and their RULINGS files record the language rulings made during this work
(Semantics rounds 1 to 3, Type System rounds 1 to 3: cycle points, the two halves of a
dynamic, construction not prompt, hosting in one graph and one tick, row entries as
projections). This file records only the choices the books leave to an
implementation. The back end's finer picks and its known gaps are in `prismc/README.md`.*

## D1. Rust is the only target for now (Jimmy's)

One target gets to self-hosting faster; `prism-language` is a Cargo crate; rustc checks
the emitted code. C++ comes after (Compiler ch. 5's second target), through a second set
of code templates and a C++ runtime.

## D4. Graph's propagation, minus the parts an AOT demo doesn't need (Jimmy's)

Heights, subscriptions, demand, double-buffered holds (the sampled half) and the
end-of-tick walks (construction, swaps, leaving, the patch), as the Graph book specifies
them. Not compaction, degradation, the memory layout, JIT, threads, wire serialization or
checkpoints. A whole-tree recompute per tick was refused as unnecessary once the graph is
in.

## D5. Memory: reference counts, and trial deletion over zombies (Jimmy: no leaks, ever)

Values and nodes are reference counted; the event flow is a DAG, so counts free
everything but one loop: zombies whose constants keep each other (row A's hold holds a
value with row B's `$b` inside and the reverse, both rows leaving in one tick). The
compact round runs trial deletion over the zombies the runtime watches
(`runtime/rust/src/gc.rs`): reach from the watched zombies along the nodes' strong
edges, count references from inside the set, keep what the outside still reaches,
release the rest. A test constructs the loop and checks nothing stays alive. Refused: a
tracing mark-sweep over the whole value heap (Graph refuses it for Graph on cost grounds,
and the one loop needs no more than this); accepting the leak.

## D6. Compute ticks, `@Defer`, `@Run` (Jimmy's)

- `@Defer: fn !a -> @!a`: every firing fires again in the next tick, by the runtime,
  with no World involved; an armed `@Defer` sets `pending`.
- `tick` returns `pending`; a World that sees it ticks again, with no inputs if it has
  none. The World knows there is pending compute and nothing else about it.
- An unbounded function is a bounded body iterated. One cycle is one turn. Cycle points
  sit exactly where the shrink check proved nothing: at every call of an unbounded
  function from an unbounded function (named, lambda, or a function whose color variable
  is instantiated unbounded), and at every turn of a `while` loop the shrink check did
  not clear. A bounded call and a shrinking `while` are one unit of work inside a cycle.
  The checker is part of the language, so which loops it clears is as determined as
  which programs it accepts, and the count is a function of the program under the books' rules
  (Semantics round 2 WL, LC). The compiler saves the continuation as the machine state.
  `@Run f, x` is `@Machine step, (start f x), !tick` with `!tick <- @Defer`; one cycle
  per tick; the result is an event in the tick the machine finishes.
- The Test World is a foreign World with the same `tick`. The test writes `compute` (a
  tick with no inputs) and checks outputs; it never supplies a result. `compute until
  idle` is allowed and prints the count; exact counts are recorded.
- No gas, no background thread, no result as a World request: a World must never
  implement a pure function, and a test must never override math.
- `@Later` stays as it is, through `Time.After`: a World really does implement time.

## D7. The parser of `prismc` is `unbounded` recursive descent

Written as ordinary recursive descent over a token cursor with a Pratt loop; the compiler
generates the cycle points. The `Parser` monad in the books stays a library for small
formats. A measure-based shrink proof for the parser was the alternative.

## D8. `Fold` and every closure-taking prelude function is polymorphic in the color

Bounded in, bounded out; unbounded in, unbounded out. Every function parameter has a
color variable of its own and the function's color is their join, as if it called each,
resolved at each call; nothing is written (round 3 CD, Jimmy's; prismc/README.md pick CD).

## D9. Hosting is the general feature; the canvas World is the one foreign World (Jimmy's)

Chess nests `@Window` over `@Grid` over squares; no depth limit. "One level" means one
foreign World under the program. Hit testing stays in Prism (the window library's
`$placed` and `?HitAt`); the scene carries `Draw.Clickable(rect, path)` so the Test World
can resolve "click the element at path" to a `Press`. The macOS World ignores that tag.
The hosting semantics themselves are the Semantics book's (ch. 9 as of round 3): one
graph, one tick, a computed answer delayed to the next tick and owing it, a forwarded
answer prompt.

## D10. Primitives are elements; widget functions are composites (Jimmy's)

`Ui.Button(...)` is an element; `@Button: fn $Text -> @!Click` wrapping it is not a
thing. Function names show only the outermost sigil of their result (`@Grid`, `@Switch`).

## D11. A failed guard under `~` raises `Prelude.Unmet` with the guard's source text

Jimmy refused `require c else e`. `forbid c, err` (a second argument) was the other
option; the prelude fault keeps one guard form everywhere.

## D12. Sigils are read and enforced by the checker (Jimmy's)

They are part of the name and the type: declared outermost time layers, free input to
inference. A wrong mark is an error, never a different program.

## D13. The counter prints `Count: -1` on two clicks in one tick

Both events fire; `<|` lets its left operand win. Semantics ch. 12's `Count: 0` was a
slip, fixed in the book.

## D14. `Float` is in

The window library (layout, hit testing, the scene) is `Float` throughout, so chess
cannot run without it. `Float` is IEEE double with shortest round-trip printing (Type
System ch. 2). `Instant` and `Duration` arithmetic stay out.

## D15. Guards take no second argument

Follows D11: `require c` and `forbid c` have one form; the parser reads a comma after a
guard as the guard's own argument list (`require Clear g, from, corner`), never as an
error value.

## D16. Typed emission: one Rust type per Prism type

The checker's typed program drives emission: a record is a struct, a choice an enum, a
hold a `Hold<T>`, every function a Rust function over the program's own types, and the
World boundary is the only place values are plain data (`Pod`). Refused: a dynamically
typed `Value` representation shared by compiled code and an interpreter (Jimmy: the
checker is not a gate in front of a dynamic runtime; there is no interpreter).

## D17. Modules are directories; test modules are declared (Jimmy's)

A module is one file, or a directory whose `mod.prism` heads it: its header's `mod name`
lines declare its sections (`name.prism`, the same namespace), `mod Name` its submodules
(`name/mod.prism`, named by their path from the root), `test mod name` its test modules,
which only `prism test` and `prism check` read. A tree's root is `app.prism` (a program)
or `lib.prism` (`mod` lines only). prismc reads a module's files as one source, each after
a newline, so the checker sees one module per namespace as before; each diagnostic names
the file and line it is in. Files on the command line stay loose (Semantics ch. 6). The
design, its rulings and the refused alternatives are docs/modules-style-pass.md.

## Features in and out

In, complete and tested: records, choices, newtypes, unit types; maps, arrays, options,
tuples; traits, `derive`, bounds, default methods, associated types; the six sigils with
lifting, `@` never lifting, lifted `match`/`if`; holds with `then`/`on`/`<|`, `!Filter`,
`!Present`, `?First`, `!Updated`; keyed `for` with projected entries; `rebuild`;
`@Switch`/`@Flatten`; `given:`, `require`/`forbid`, `match` with guards; modules, `emit`,
`use`; rows, families, row variables, `otherwise:`; hosting (`@HostDocument`, `@HostSome`,
`@HostList`) as a `for` over the child's document, paths, patches, first-value query;
`Nat`/`Int` as bignums, `U8`; `~`, `Error`, `Fault`, `Prelude.Unwrap`/`Exceptions`, exit
codes; `&` blocks with the borrow rule; code templates; `Pod`; `test`/`check`/`compute`;
the shrink check, the `unbounded` color, `@Defer`, `@Run`; `Float` (D14); the
prompt-cycle check; zombies; existentials (`any ^Show`, `Elem`); `<<T>>` explicit
instantiation; `@Later` via `Time.After`.

Out for this compiler, each rejected with a diagnostic naming the feature, never
silently: `T monad:` blocks and `^Monad`; higher-kinded traits; sized ints other than
`U8`; `Instant`, `Duration` arithmetic; sparse tuples, `!Align`, non-empty arrays,
suffixed operators; named lenses `&Home` and `Over` (the `update` block's lens paths stay
in); POD hosting (`@Collect`, `@View`, `@Edit`, `@Translate`); the Prism World; live
update; checkpoints; the template notation as a second syntax for element languages (code
templates only); indexed choices/GADTs for user types. The back end's own open gaps are
listed in `prismc/README.md`.
