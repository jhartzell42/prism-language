# Handoff: the self-hosting branch as it stands

*Written by Claude on the claude-self-hosting branch, 6 November 2026, at the end of the
cloud session that finished the branch; subject to change. What exists, how to build and
test it from a fresh clone, what is Jimmy's to decide, and what to rerun on the Mac. The
PR's description (#20) says the same at more length; the numbers are in
`claude/benchmarks-preliminary.md` and the three `RESULTS.md`.*

## What exists

- **`prismc`**, a self-hosting compiler written in Prism (`prismc/`, a module tree:
  `syntax/`, `check/`, `lower/`, `emit/rust/`, `emit/cpp/`), which emits a Rust crate over
  `runtime/rust` or a C++20 program over `runtime/cpp`. Both reach a fixed point
  (`bootstrap/`), and the C++ prismc emits the Rust prismc's Rust byte for byte. The two
  seeds (`bootstrap/prismc-rust.tar.xz`, `prismc-cpp.tar.xz`) are the fixed points of this
  tree.
- **Two runtimes and four Worlds:** `runtime/{rust,cpp}` (the graph: heights, holds,
  hosts, `@PerKey`, `@Distinct`, the persistent `Map` and `Array`, regular expressions over
  `regex`/RE2, `Text` as UTF-8 with cursors, Unicode 17 case mapping); `worlds/cli/{rust,
  cpp}` (files, arguments, the Test World); `worlds/canvas/{rust,cpp}` (a window on
  winit/softbuffer and on Cocoa or X11, painting only the damage of a new scene, each
  window in sRGB).
- **The language as built:** modules as a tree (`mod Name`, `test mod Tests`), regular
  expression literals, arrays, the host document as `$(Path ==> $Elem c)`, `@Distinct`,
  `@PerKey`, and `for k, $v <- $each:` over a map of dynamics binding the value under the
  key (ruling FV: the `for`'s key walk tells a row whose dynamic was replaced; no row is
  woken to ask, none is rebuilt).
- **The C++ target as Rust's twin, and jemalloc for C++** (7 and 8 November, each asked
  for by Jimmy, each green on every suite and measured in claude/benchmarks-preliminary.md's
  Linux section):
  - the C++ `Map` changes its nodes in place when nothing else shares them, as the Rust
    `Map` does through `Rc::make_mut` (runtime/cpp/README.md, "Arrays and maps: the costs");
  - a choice is a `std::variant` held by value, as the Rust enum is, and a recursive payload
    a `prism_rtl::Rc<const T>`, whose `==` and `<=>` go by the value where `T` has them, as
    Rust's `Rc<T>` does; identity is `ptr_eq` (prismc/README.md, picks CH and PT);
  - the C++ printer moves where Rust moves: a bare read of an owned local, and a move
    closure's capture `let`s as moving init-captures (pick MV);
  - the C++ programs allocate with jemalloc, linked through `pkg-config`, sanitizer builds
    excepted (pick AL); the Rust programs use the system allocator (jemalloc through
    `tikv-jemallocator` was measured and taken out: it builds jemalloc from source on every
    clean build; claude/todo-for-jimmy.md).
  - `Float`'s `==` is bit identity with every NaN one value, and its order the IEEE total
    order over the same bits (Type System round 6 FB and NO): `prism_rtl::Float` in both runtimes (pick BF), every `^Eq` a Rust `Eq`, every
    `^Ord` an `Ord`, `Rc`'s `==` pointer first; prismc no longer looks for floats in types
    (`Floaty` gone), nor do the maps and arrays keep self-equality caches.
  - What it bought, the same night, six interleaved runs each: jemalloc took the C++
    prismc's self-compile from 105.1 s to 75.7 s and the Rust prismc's from 69.9 s to
    51.6 s (the Rust target has since gone back to the system allocator); FB took them,
    both on jemalloc, on a later boot, from 76.9 s to 61.3 s and from 49.9 s to 39.6 s; the
    moves and the one `Rc` were flat. Under the same allocator C++ takes about 1.5 times
    Rust's time (doubt CH1); `bench/running/RESULTS.md` has the tip's numbers as built.
- **Programs:** `examples/chess` (the rules, the board, and a window with a move list,
  Back, Forward, Reset and a settings dialog, on the window library
  `examples/chess/window`), the GUI table `bench/gui/prism` (js-framework-benchmark's
  operations), `bench/gui/gtk` (the same in C with GTK 3), the conformance corpus
  `tests/conformance/` (130 programs), the checker's corpus (`tests/checker/`, 162
  negative programs), the cost pairs (`tests/cost/`, four).
- **Claude's prose** lives under `claude/` and in the READMEs: `architecture.md`,
  `backend.md` (the contract, and "Rust or C++ as the default back end"),
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
CXX=clang++ JOBS=$(nproc) bootstrap/bootstrap-cpp.sh   # the C++ fixed point; target/bootstrap/prismc-cpp
make test                                    # every suite (below), or each by hand:
cargo test --workspace --release
make -C runtime/cpp test; make -C worlds/cli/cpp test; make -C worlds/canvas/cpp test
tests/compiled/run.sh; tests/compiled/run.sh --target cpp
tests/checker/run.sh; tests/driver/run.sh; tests/cost/run.sh [--target cpp]
./prism check --unused prismc/
bench/building/run.sh; bench/running/run.sh; bench/gui/prism/check.sh   # with nothing else running
```

- **Linux needs** `clang++` or `g++` (C++20), `make`, `pkg-config`, `libre2-dev`, `libjemalloc-dev`,
  `libcairo2-dev`, `libfreetype-dev`, `libx11-dev`, `libxext-dev`, `libxtst-dev`, `xvfb`,
  GNU `time`, and for the sanitizer builds under clang `libclang-rt-*-dev`; the cost tests
  count instructions with `perf` where the machine has counters and with valgrind's
  cachegrind where it has none (`bench/lib.sh`); the GUI benchmark and `damage_smoke` run
  under `xvfb-run -a` when there is no `DISPLAY`. **macOS needs** Homebrew's `cairo
  freetype jemalloc pkg-config gtk+3 llvm re2` (llvm for the sanitizer run; jemalloc not
  yet tried on the Mac).
- **Time on two cores** (the cloud VM, 6 November): the Rust bootstrap 6.5 min from the
  seed, the C++ bootstrap 18 min, the C++ compiled suite 23 min, the Rust one 5 min, the
  building benchmark 40 min, the GUI benchmark 13 min.

## Where the numbers are

- **`bench/building/RESULTS.md`, `bench/running/RESULTS.md`, `bench/gui/RESULTS.md`:**
  this machine (Linux, 2 cores, no counters): building from clean, running prismc, the
  GUI table against GTK 3, each with the machine named. Not comparable with the Mac's.
- **`claude/benchmarks-preliminary.md`:** the Mac's numbers (M2 Pro: the Low Power Mode
  run of 6 November, the busy-machine runs of 5 November, the GUI phase tables) and a
  section for this machine's.
- **`claude/backend.md`, "Rust or C++ as the default back end":** Claude's
  recommendation, Rust, argued from both models, with what would change it.

## Jimmy's to decide

Each has Claude's pick where it stands; his word overrules by editing.

- **The default back end:** Rust (claude/backend.md). What would flip it is listed there.
- **EP** (claude/incremental-host-document.md): each host-document entry published per
  place, structural, one tick per host level for a value change.
- **The window library's doubts** (examples/chess/window/README.md): DG (a slider drags
  only once the canvas Worlds send pointer motion), OP, HS, FS; the chess doubts
  (examples/chess/README.md): PC, DT, NT (moves written in coordinates), TS (the board's
  and window's transcripts are not in the repository, over 10 KB), HT (13 ticks per press
  to idle).
- **The design note's doubts** (claude/incremental-host-document.md): ET, PB, SE, RF
  (routing through every child's filter, linear), MT (claude/bugs/window-relayout-every-
  change.md: a new word's extent reaches its layout a tick late).
- **The regex dialect's doubts** (claude/regex-dialect.md): CT, SH among them.
- **The checker's** (prismc/README.md, "Checker doubts" and "Gaps against ch. 5"): TR,
  RP, CF, VW, UI, WL; gaps LR, RE, R2, DG (expression blocks, mutual `let`s), SU, AT;
  pick EE (`existential-escape` by its effect, the unpacking itself not done).
- **The back end's** (prismc/README.md, "Back-end picks" and "Contract gaps"): the picks
  from NM to FV, gaps UV to DI, doubts MH1 to MH4, and "Not lowered yet".
- **The modules pass's doubts** (claude/modules-style-pass.md) and the Test World in Prism
  (claude/test-world-in-prism.md: Claude recommends (a), programs as values).
- **CH1, the C++ self-compile's cost** (prismc/README.md, C++ doubts): under glibc the
  inline variants cost about 9% against the handles (not measured under jemalloc); what
  the jemalloc profile shows left is the records of maps (`G`, `Lx`, `St`, `TProgram`)
  moved, copied and destroyed, about 30% of samples, and the unbounded folds' job per
  element. The handle comes back from `7dc334d^`; `Rc`'s `==` could compare pointers
  first where the type has no `Float`, as Rust's `Rc<T: Eq>` does (untried).
- **`Sign` under the total order** (prismc/README.md, pick BF): `Sign`, `Max` and `Min`
  use `<`, so `Sign -0.0` is -1.0 and every NaN's `Sign` is 1 (Claude: keep; else IEEE
  `copysign` for `Sign`).
- **Performance left:** remove still re-stacks the row list, in C++ 7.5 ms of CPU against
  Rust's 5.0 here after the Map change (claude/bugs/window-relayout-every-change.md: the
  path copies were a part of the gap, not its body; what remains is to profile); prismc's emit time grows faster than
  its input (about 2,100 lines a second on itself here against 12,000 to 14,000 on chess
  and the table, cause unknown); gap OC and the string copies SC1 to SC7 (prismc/README.md).
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
  the Mac's; the relative shape (Rust ahead on bulk operations, both ahead of GTK on
  creates) holds.
- **The Cocoa paths:** `macos_smoke`, chess in a window, the C++ canvas World's Cocoa
  loop, and the sanitizer run under Homebrew clang. The X11 paths, damage painting on X11
  and the on-screen colours after the sRGB change were checked here (the Worlds' tests and
  the GUI check's pixel comparison under Xvfb).
- **The sanitizer run of the compiled suite** (`make test-sanitize`), last run one merge
  before the `for` change on the Mac; not run here (it takes the C++ suite's 23 minutes
  several times over on two cores).
