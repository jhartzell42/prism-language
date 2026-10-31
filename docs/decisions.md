# Decisions on the self-hosting branch

*Written by Claude, 31 October 2026, and appended to as the work goes. Each entry: the
decision, the reason, the alternative refused. Jimmy's rulings from the planning chat
are marked Jimmy's; the rest are Claude's and he overturns them by hand. The books in
`prism-notes` are the specification; these are the choices the specification leaves to
an implementation, plus the places this branch knowingly departs from the Compiler
book's plan.*

## D1. Rust is the only target (Jimmy's)

The Compiler book planned Rust and C++ in one pass. One target gets to self-hosting
faster; `prism-language` is a Cargo crate; rustc checks the emitted code. C++ with Boehm
GC and `immer` was the alternative.

## D2. Compiler zero is an interpreter in Rust, not a Haskell front end

The Compiler book's bootstrap plan starts from prism-reflex's Haskell front end as a
library. This container has no Haskell toolchain and cannot fetch one, and a second
front end in Haskell would be deleted at self-hosting anyway. So compiler zero is a Rust
interpreter of Prism over `prism_rt`: it runs `prismc` once to compile `prismc`, and is
deleted when stage 3 lands. It emits nothing; `prismc` owns all codegen, so the codegen
is written once, in Prism. The refused alternatives: a Rust compiler that emits Rust (a
second codegen to throw away), and waiting for a Haskell toolchain.

## D3. Compiler zero has no type checker (Jimmy: keep compiler zero minimal)

Lifting and trait dispatch are decided dynamically in the interpreter: a runtime value
knows whether it is plain, a dynamic or an event, and sigils on names say the declared
layer. Rows are dynamic. The full checker lives in `prismc`, in Prism, and stage 2 (the
compiler compiling its own source) is where every program on this branch, `prismc`
included, is checked. Compiler zero never ships. The alternative, a second checker in
Rust, doubles the checker and is deleted anyway.

## D4. Graph's propagation, minus the parts an AOT demo doesn't need (Jimmy's)

Heights, subscriptions, demand, double-buffered holds and the end-of-tick walks, as the
Graph book specifies them. Not compaction, degradation, the memory layout, JIT, threads,
wire serialization or checkpoints. A whole-tree recompute per tick was refused as
unnecessary once the graph is in.

## D5. Memory: refcounted immutable values, tracing mark-sweep over the value heap
## rooted at the widget tree, between ticks (Jimmy: no leaks, ever)

Zombie constants can form loops (a dead widget's hold capturing a closure over another
dead widget's hold); refcounts alone leak them. The sweep frees them. A test constructs
the loop and checks the debug allocator reports zero live cells after the sweep. Graph's
book refuses a tracing collector for Graph itself on cost grounds; this is the compiler's
runtime, and the refusal is recorded as a doubt there.

## D6. Compute ticks, `@Defer`, `@Run` (Jimmy's, with Claude's spellings)

- `@Defer: fn !a -> @!a`: every firing fires again in the next tick, by the runtime,
  with no World involved; an armed `@Defer` sets `pending`.
- `tick` returns `pending`; a World that sees it ticks again, with no inputs if it has
  none. The World knows there is pending compute and nothing else about it.
- An unbounded function is a bounded body iterated. One cycle is one turn. The compiler
  puts a cycle point at every call of an unbounded function from an unbounded function
  (the simple canonical rule, Jimmy's), saving the continuation as the machine state.
  `@Run f, x` is `@Machine step, (start f x), !tick` with `!tick <- @Defer`; one cycle
  per tick; the result is an event in the tick the machine finishes.
- The Test World is a foreign World with the same `tick`. The test writes `compute`
  (a tick with no inputs) and checks outputs; it never supplies a result. `compute
  until idle` is allowed and prints the count; exact counts are recorded.
- No gas, no background thread, no result as a World request. Those were refused: a
  World must never implement a pure function, and a test must never override math.
- `@Later` stays as it is, through `Time.After`: a World really does implement time.

## D7. The parser of `prismc` is `unbounded` recursive descent

Written as ordinary recursive descent over a token cursor with a Pratt loop; the
compiler generates the cycle points. The `Parser` monad in the books stays a library for
small formats. A measure-based shrink proof for the parser was the alternative.

## D8. `Fold` and every closure-taking prelude function is polymorphic in the color

Bounded in, bounded out; unbounded in, unbounded out. One color variable per
closure-taking signature, resolved at each call.

## D9. Hosting is the general feature; the canvas World is the one foreign World (Jimmy's)

Chess nests `@Window` over `@Grid`; no depth limit. "One level" means one foreign World
under the program. Hit testing stays in Prism (the window library's `$placed` and
`?HitAt`); the scene carries `Draw.Clickable(rect, path)` so the Test World can resolve
"click the element at path" to a `Press`. The macOS World ignores that tag.

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

## Features in and out

In, complete and tested: records, choices, newtypes, unit types; maps, arrays, options,
tuples; traits, `derive`, bounds, default methods, associated types; the six sigils with
lifting, `@` never lifting, lifted `match`/`if`; holds with `then`/`on`/`<|`, `!Filter`,
`!Present`, `?First`; keyed `for`; `rebuild`; `@Switch`/`@Flatten`; `given:`,
`require`/`forbid`, `match` with guards; modules, `emit`, `use`; rows, families, row
variables, `otherwise:`; hosting (`@HostDocument`, `@HostSome`, `@HostList`), paths,
patches, first-value query; `Nat`/`Int` as bignums, `U8`; `~`, `Error`, `Fault`,
`Prelude.Unwrap`/`Exceptions`, exit codes; `&` blocks with the borrow rule; code
templates; `Pod`; `test`/`check`/`compute`; the shrink check, the `unbounded` color,
`@Defer`, `@Run`; the prompt-cycle check; zombies; existentials (`any ^Show`, `Elem`);
`<<T>>` explicit instantiation; `@Later` via `Time.After`.

Out for this compiler, each rejected with a diagnostic naming the feature, never
silently: `T monad:` blocks and `^Monad`; higher-kinded traits; sized ints other than
`U8`; `Float`, `Instant`, `Duration` arithmetic; sparse tuples, `!Align`, non-empty
arrays, suffixed operators; named lenses `&Home` and `Over` (the `update` block's lens
paths stay in); POD hosting (`@Collect`, `@View`, `@Edit`, `@Translate`); the Prism
World; live update; checkpoints; the template notation as a second syntax for element
languages (code templates only); indexed choices/GADTs for user types.
