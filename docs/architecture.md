# Architecture of the self-hosting prototype

*Written by Claude (the model configured as claude-fable-5-1; the serving model may
differ), 31 October 2026, on the `claude/self-hosting` branch. Jimmy iterates on all of
it by hand; nothing here is final. The language is specified by the books in
`jhartzell42/prism-notes` under `claude/prism/` (Semantics, Type System, Templates,
Conformance, Graph, Compiler); where this document and a book disagree, the book wins
and the disagreement is a bug here. `decisions.md` records the choices made on this
branch and why.*

## The goal

One binary, `prismc`, written in Prism, that compiles Prism to Rust, compiles itself
(stage 2 and stage 3 byte-equal), and compiles chess, which links a canvas World and
opens a window on macOS. Full front end: every language claim that is in (see
`decisions.md`, "Features in and out") is implemented completely, with tests. Simplified
back end: Graph's propagation (heights, subscriptions, demand, double-buffered holds, the
end-of-tick walks) without compaction, degradation, the memory layout, JIT, threads, wire
serialization or checkpoints.

## The crates and directories

```
prism_rt/        the runtime library every compiled program links: values, nodes,
                 heights, holds, keyed for, host suppliers, patches, first-value query,
                 @Defer and pending, the collector, the Test World, Fs and Process Worlds
prism_zero/      compiler zero: a Rust interpreter of Prism (lexer, parser, dynamic
                 evaluator over prism_rt). Scaffolding: it runs prismc once, to compile
                 prismc; deleted when stage 3 lands. No type checker (decisions.md D3).
prismc/          the compiler, in Prism: lexer, parser, checker, lowering, emission
                 through code templates, the driver (a rebuild over the source set)
prelude/         the Prism prelude, shared by prism_zero and compiled programs
examples/        chess (rules, board, window library), counter, half counter, todos
worlds/canvas/   the macOS canvas World (winit + softbuffer + tiny-skia + fontdue):
                 Gui.Window, Gui.Scene, Gui.Metrics, Gui.Focus; no hit testing
tests/           conformance transcripts (the Conformance book's test language),
                 goldens, the zombie-loop collector test, stage-2/stage-3 byte equality
prism_runtime/   Jimmy's earlier runtime prototype, untouched
```

## The pipeline

- `prism_zero run <files>` lexes, parses and evaluates a program under a World.
- `prismc` (Prism) reads sources through `Fs.Contents`, lexes, parses, type checks,
  lowers to SSA, emits Rust through code templates, writes through `Fs.Output`, exits
  through `Process.Exit`. A batch compile is three ticks plus the compute ticks of its
  unbounded parser.
- Stage 1: `prism_zero run prismc/*.prism -- prismc/*.prism` emits `prismc` in Rust;
  cargo builds it. Stage 2: that binary compiles `prismc` again. Stage 3: stage 2's
  output compiles `prismc`. Stage 2 and 3 are byte-equal.
- Chess: `prismc examples/chess/*.prism` emits Rust that links `prism_rt` and
  `worlds/canvas`; under the Test World the same program runs the conformance
  transcripts.

## The tick

`tick: ?State, inputs -> State, outputs, pending`. Build is `tick` on `nil`. Every tick
first advances every pending job (`@Defer`, and `@Run` machines built on it) by one
cycle, then propagates by height, then runs the end-of-tick walks (newborns built from
post values, detached widgets torn down, holds swapped), then diffs the document for the
patch and collects requests in path order. `pending` is true while any `@Defer` is
armed; a World that sees it ticks again, with no inputs if it has none.

## Single commands

- `make test` (or `cargo test --workspace`): every Rust test, every conformance
  transcript, the stage-2/3 byte check once `prismc` exists.
- `make chess`: build and open chess on macOS.
- `make bootstrap`: stage 1, 2, 3.

The README states these three.
