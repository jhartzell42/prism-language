# Architecture of the self-hosting prototype

*Written by Claude (the model configured as claude-fable-5-1; the serving model may
differ), 31 October 2026, on the `claude/self-hosting` branch. Jimmy iterates on all of
it by hand; nothing here is final. The language's design is in the books of
`jhartzell42/prism-notes` under `claude/prism/` (Semantics, Type System, Templates,
Conformance, Graph, Compiler), a curated set of design prompts rather than a
specification; where this document and a book disagree, the disagreement is raised as a
doubt. `decisions.md` records the choices made on this
branch and why.*

## The goal

One binary, `prismc`, written in Prism, that compiles Prism to Rust, compiles itself
(the crate it emits for itself is the crate it was built from), and compiles chess, which links a canvas World and
opens a window on macOS. Full front end: every language claim that is in (see
`decisions.md`, "Features in and out") is implemented completely, with tests. Simplified
back end: Graph's propagation (heights, subscriptions, demand, double-buffered holds, the
end-of-tick walks) without compaction, degradation, the memory layout, JIT, threads, wire
serialization or checkpoints.

## The crates and directories

```
runtime/rust/  prism_rtl, the runtime library every compiled program links:
                    nodes, heights, holds, keyed for, patches, first values, @Defer
                    and pending, jobs, the Test World (docs/backend.md)
prismc/             the compiler, in Prism: lexer, parser, checker, lowering, emission
                    through code templates, the driver (a rebuild over the source set)
prelude/            the Prism prelude the checker reads; its Rust is prismc's
                    templates/prelude.prism, emitted beside every program
examples/           chess (rules, board, window library)
runtime/{rust,cpp}/ the runtimes the emitted code runs on, one per target
worlds/cli/{rust,cpp}/   the CLI World: Fs, Process, Time, Prelude.Unwrap
worlds/canvas/{rust,cpp}/  the canvas World (winit + softbuffer + tiny-skia + fontdue;
                    Xlib or Cocoa + cairo + FreeType): Gui.Window, Gui.Scene,
                    Gui.Metrics, Gui.Focus
bench/              running Prism once compiled, building Prism, and the GUI table
tests/              conformance transcripts (the Conformance book's test language), the
                    checker's tests, the bootstrap's fixed point
prism               the command: compile, test, run, check
```

## The pipeline

- `prismc` (Prism) reads sources through `Fs.Contents` (a directory through `Fs.List`),
  lexes, parses, type checks, lowers, emits a Rust crate over `prism_rtl` through code
  templates, writes through `Fs.Output`.
- The bootstrap: a kept native prismc compiles `prismc/` into gen1, cargo builds it, gen1
  compiles `prismc/` into gen2, and gen1 and gen2 are byte-equal (the fixed point).
- Chess: `./prism run examples/chess` compiles it, builds it with the canvas World and
  opens the window; `./prism test examples/chess` runs its transcripts under the compiled
  Test World.

## The tick

`tick: ?State, inputs -> State, outputs, pending`. Build is `tick` on `nil`. Every tick
first advances every pending job (`@Defer`, and `@Run` machines built on it) by one
cycle, then propagates by height, then runs the end-of-tick walks (newborns built from
post values, detached widgets torn down, holds swapped), then diffs the document for the
patch and collects requests in path order. `pending` is true while any `@Defer` is
armed; a World that sees it ticks again, with no inputs if it has none.

## Single commands

- `make test`: `cargo test --workspace`, every compiled transcript
  (`tests/compiled/run.sh`), the checker's tests (`tests/checker/run.sh`), and the
  bootstrap's fixed point.
- `make chess`: build and open chess (`./prism run examples/chess`).
- `make bootstrap`: the kept prismc to gen1 to gen2, byte-equal; installs
  `target/bootstrap/prismc`.

The README states these three.
