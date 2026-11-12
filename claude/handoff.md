# Handoff: the self-hosting branch as it stands

*Written by Claude on the claude-self-hosting branch, 6 November 2026, at the end of the
cloud session that finished the branch; subject to change. What exists, how to build and
test it from a fresh clone, what is Jimmy's to decide, and what to rerun on the Mac. The
PR's description (#20) says the same at more length; the numbers are in
`claude/benchmarks-preliminary.md` and the three `RESULTS.md`.*

## What exists

- **`prismc`**, a self-hosting compiler written in Prism (`prismc/`, a module tree:
  `syntax/`, `check/`, `lower/`, `emit/rust/`), which emits a Rust crate over
  `runtime/rust`. It reaches a fixed point
  (`bootstrap/`). The
  seed (`bootstrap/prismc-rust.tar.xz`) is the fixed point of this
  tree.
- **The runtime and two Worlds:** `runtime/rust` (the graph: heights, holds,
  hosts, `@PerKey`, `@Distinct`, the persistent `Map` and `Array`, regular expressions over
  `regex`, `Text` as UTF-8 with cursors, Unicode 17 case mapping); `worlds/cli/rust`
  (files, arguments, the Test World); `worlds/canvas/rust` (a window on
  winit/softbuffer, painting only the damage of a new scene, each
  window in sRGB).
- **The language as built:** modules as a tree (`mod Name`, `test mod Tests`), regular
  expression literals, arrays, the host document as `$(Path ==> $Elem c)`, `@Distinct`,
  `@PerKey`, and `for k, $v <- $each:` over a map of dynamics binding the value under the
  key (ruling FV: the `for`'s key walk tells a row whose dynamic was replaced; no row is
  woken to ask, none is rebuilt).
  - the Rust programs use the system allocator (jemalloc through
    `tikv-jemallocator` was measured and taken out: it builds jemalloc from source on every
    clean build; claude/todo-for-jimmy.md).
  - `Float`'s `==` is bit identity with every NaN one value, and its order the IEEE total
    order over the same bits (Type System round 6 FB and NO): `prism_rtl::Float` (pick BF), every `^Eq` a Rust `Eq`, every
    `^Ord` an `Ord`, `Rc`'s `==` pointer first; prismc no longer looks for floats in types
    (`Floaty` gone), nor do the maps and arrays keep self-equality caches.
  - What it bought, the same night, six interleaved runs each: jemalloc took the
    Rust prismc's self-compile from 69.9 s to
    51.6 s (the Rust target has since gone back to the system allocator); FB took it,
    on jemalloc, on a later boot, from 49.9 s to 39.6 s; `bench/running/RESULTS.md` has the tip's numbers as built.
- **Programs:** `examples/basic-chess` (the rules, the board, and a window with a move list,
  Back, Forward, Reset and a settings dialog, on the window library
  `examples/basic-chess/basic_window`), the GUI table `bench/gui/prism` (js-framework-benchmark's
  operations), `bench/gui/gtk` (the same in C with GTK 3), the conformance corpus
  `tests/conformance/` (130 programs), the checker's corpus (`tests/checker/`, 162
  negative programs), the cost pairs (`tests/cost/`, four).
- **Claude's prose** lives under `claude/`: `architecture.md`,
  `backend.md` (the contract),
  `benchmarks-preliminary.md`, `decisions.md`, `incremental-host-document.md`,
  `modules-style-pass.md`, `regex-dialect.md`, `test-world-in-prism.md`, `todo-for-jimmy.md`,
  `bugs/` (one file per bug not yet fixed).
- **The books** are `jhartzell42/prism-notes` `main` (`claude/prism/`), at `db4232b` or
  later: Semantics round 9 (HD, PK, FV), with chapters 4, 9 and appendix B and Platform
  chapter 7 saying what this tree does.

## From a fresh clone

```
cargo fetch                                  # once; every later build is offline
bootstrap/bootstrap.sh                       # the Rust fixed point; installs target/bootstrap/prismc
make test                                    # every suite (below), or each by hand:
cargo test --workspace --release
tests/compiled/run.sh
tests/checker/run.sh; tests/driver/run.sh; tests/cost/run.sh
./prism check --unused prismc/
bench/building/run.sh; bench/running/run.sh; bench/gui/prism/check.sh   # with nothing else running
```

- **Linux needs** `make`, `pkg-config`,
  `libx11-dev`, `libxext-dev`, `libxtst-dev`, `xvfb`,
  GNU `time`; the cost tests
  count instructions with `perf` where the machine has counters and with valgrind's
  cachegrind where it has none (`bench/lib.sh`); the GUI benchmark and `damage_smoke` run
  under `xvfb-run -a` when there is no `DISPLAY`. **macOS needs** Homebrew's
  `pkg-config gtk+3`.
- **Time on two cores** (the cloud VM, 6 November): the Rust bootstrap 6.5 min from the
  seed, the Rust compiled suite 5 min, the
  building benchmark 40 min, the GUI benchmark 13 min.

## Where the numbers are

- **`bench/building/RESULTS.md`, `bench/running/RESULTS.md`, `bench/gui/RESULTS.md`:**
  this machine (Linux, 2 cores, no counters): building from clean, running prismc, the
  GUI table against GTK 3, each with the machine named. Not comparable with the Mac's.
- **`claude/benchmarks-preliminary.md`:** the Mac's numbers (M2 Pro: the Low Power Mode
  run of 6 November, the busy-machine runs of 5 November, the GUI phase tables) and a
  section for this machine's.

## Jimmy's to decide

Each has Claude's pick where it stands; his word overrules by editing.

- **EP** (claude/incremental-host-document.md): each host-document entry published per
  place, structural, one tick per host level for a value change.
- **The window library's doubts** (examples/basic-chess/basic_window/): DG (a slider drags
  only once the canvas Worlds send pointer motion), OP, HS, FS; the chess doubts
  (examples/basic-chess/): PC, DT, NT (moves written in coordinates), TS (the board's
  and window's transcripts are not in the repository, over 10 KB), HT (13 ticks per press
  to idle).
- **The design note's doubts** (claude/incremental-host-document.md): ET, PB, SE, RF
  (routing through every child's filter, linear), MT (claude/bugs/window-relayout-every-
  change.md: a new word's extent reaches its layout a tick late).
- **The regex dialect's doubts** (claude/regex-dialect.md): CT, SH among them.
- **The checker's** (prismc/check/): TR,
  RP, CF, VW, UI, WL; gaps LR, RE, R2, DG (expression blocks, mutual `let`s), SU, AT;
  pick EE (`existential-escape` by its effect, the unpacking itself not done).
- **The back end's** (prismc/lower/, prismc/emit/): the picks
  from NM to FV, gaps UV to DI, doubts MH1 to MH4, and the `error[not-lowered]` refusals.
- **The modules pass's doubts** (claude/modules-style-pass.md) and the Test World in Prism
  (claude/test-world-in-prism.md: Claude recommends (a), programs as values).
- **`Sign` under the total order** (runtime/rust/src/float.rs): `Sign`, `Max` and `Min`
  use `<`, so `Sign -0.0` is -1.0 and every NaN's `Sign` is 1 (Claude: keep; else IEEE
  `copysign` for `Sign`).
- **Performance left:** remove still re-stacks the row list, in
  Rust 5.0 ms of CPU here after the Map change (claude/bugs/window-relayout-every-change.md: the
  path copies were a part of the gap, not its body; what remains is to profile); prismc's emit time grows faster than
  its input (about 2,100 lines a second on itself here against 12,000 to 14,000 on chess
  and the table, cause unknown); gap OC and the string copies SC1 to SC7 (prismc/emit/).
- **The bugs filed, not fixed:** `claude/bugs/bench-cuis-latin1-and-software-paint.md`
  (Cuis is out of the scripted runs).
- **Remote branches to delete** (merged into `claude-self-hosting`; Claude deletes none):
  `claude-arrays`, `claude-case`, `claude-chess-ui`, `claude-damage`, `claude-doc-design`,
  `claude-headers-rebased`, `claude-host-cheap`, `claude-incremental`, `claude-layout`,
  `claude-lowering-fixes`, `claude-lowering-rebased`, `claude-perf`, `claude-regex`,
  `claude-regex-compiler`, `claude-style`, `claude-text`, `claude-window-cache`, and this
  session's `claude-for-inner`, `claude-cpp-inline-choices`, `claude-cpp-moves`,
  `claude-cpp-rc-jemalloc`, `claude-float-bits`, `claude-widget-plain`; and `claude-cpp-jemalloc-wip`, not an
  ancestor but superseded (its work came in through `claude-cpp-rc-jemalloc`). Not merged by
  ancestry, from October (superseded by their rebased twins or by `main`, for Jimmy to
  judge): `claude`, `claude-reflex`, `claude-lowering`, `claude-cpp-headers-wip`,
  `claude-move-language`, `claude-q-g7-torn-down-request`, `claude-rt-1` to `claude-rt-6`.

## To rerun on the Mac

Everything here is green, and these are the parts the Mac alone can give:

- **Instruction counts** for `bench/running/run.sh` (the self-compile and normalize) and
  `bench/gui/prism/check.sh`'s phase table: this VM has no hardware counters, so the
  running results say "n/a" and the phase table is CPU milliseconds.
- **The GUI timings on a real display:** `bench/gui/run.sh` here ran under Xvfb (software
  painting, a frame handed to a server that shows nothing), so its milliseconds are not
  the Mac's; the relative shape (Rust ahead of GTK on
  creates) holds.
- **The Cocoa paths:** `macos_smoke`, chess in a window. The X11 paths, damage painting on X11
  and the on-screen colours after the sRGB change were checked here (the Worlds' tests and
  the GUI check's pixel comparison under Xvfb).

