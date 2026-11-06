# Handoff: where the self-hosting branch stands

*Written by Claude on the claude-self-hosting branch, 4 November 2026, for the move from
the cloud container to a local macOS run; subject to change. Everything still to do, with
the state each item is in. Delete an item when it is done; delete the file when it is
empty.*

## The `claude/` directory (Jimmy)

- **Claude-written prose lives in `claude/`, never in a directory called `docs/`.** Jimmy:
  calling Claude's write-ups "docs" "was not something I did or ever would sign off on.
  Slop code can be mixed with human code but docs hell no." The old `docs/` is now
  `claude/`, and every reference moved with it (`git mv` plus a rewrite of `docs/` to
  `claude/` across the tree).
- **Never create `docs/` again,** and never call these files docs in a commit, a README or
  a PR. New write-ups (bug reports, designs, the handoff) go under `claude/`.
- **The seeds were rewritten after the move,** since the templates' comments name these
  files and the emitted code changed with them.

## Where things are

- **Branch `claude-self-hosting`** is green on macOS (an M2 Pro): the Rust and the C++
  fixed points, the C++ prismc emitting the Rust prismc's Rust byte for byte, both
  compiled suites, the checker, the driver tests, the C++ runtime's and Worlds' tests
  (ASan+UBSan under Homebrew clang; Apple clang's hangs at startup on macOS 26).
  Everything builds and tests at cargo release and `-O3`, nothing lower (Jimmy).
- **Merged:** modules as a tree (Jimmy: they nest as Rust's do, `mod Name`, no sections;
  test modules are declared `test mod Tests` and read only by `prism test`;
  `claude/modules-style-pass.md`, whose doubts are open for him); regular expressions in
  both runtimes (`claude/regex-dialect.md`, `tests/regex/corpus.txt`) and in the compiler,
  prismc's lexer reading by them (Regular expressions, below); per-module C++ headers; the first three bugs and gaps LP, LW, SR; `Text` as
  UTF-8 with cursors; the performance queue (a non-atomic `prism_rtl::Rc`, less string
  churn, gap NL, `?ParseNat`, the emitted-code cleanups).
- **Also merged:** one persistent tree of chunks as `Array` in both runtimes and an AVL
  `Map` in Rust (`im-rc` gone; the Rust prismc compiling itself fell from 2.9 GB and 75 s
  to 275 MB and 45 s); `Upper` and `Lower` by Unicode 17's full case mapping in both
  runtimes; `@Distinct`, a dynamic `let` read in several places computed once, and the
  window library's first speedups (The window, below).
- **Branch in flight:** `claude-lowering-fixes` (lifts over `?` and `~` in lowering, a
  program's names kept off the runtime's helpers, both bootstraps' `--from-seed`, gap OC,
  small modules sharing C++ translation units), merged here when green.
- **The books** are `jhartzell42/prism-notes` `main` at `32597b0` or later: Semantics
  round 4 (TX, Text), round 5 (regular expressions), round 6 (MT, TD: the module tree and
  test modules), round 7 (CM, case mapping), round 8 (DU, `@Distinct`; CT, `Text` of a
  `Char`); Type System rounds 4 and 5; Conformance round 2. Claude edits them in the
  worktree `prism-notes-claude` and pushes to `main` fast-forward (Jimmy).
- **The bootstrap.** Both seeds match the tree: `bootstrap/prismc-rust.tar.xz` and
  `bootstrap/prismc-cpp.tar.xz`, each written from its fixed point.
- **The benchmarks** are `bench/building`, `bench/running` and `bench/gui`; each writes a
  `RESULTS.md` beside itself. The numbers so far are `claude/benchmarks-preliminary.md`.

## Measured so far

Apple M2 Pro (12 cores, 16 GB), 4 November 2026, each pipeline all one language, both at
the shipping configurations (cargo release, opt-level 3; clang++ -O3). Least of five runs
for instruction counts; wall times with other jobs running, so rough. The full tables and
their history are in prismc/README.md ("What it costs", "Strings: what is copied",
"Building prismc, C++ against Rust").

| Run | Rust | C++ |
|---|---|---|
| Build the compiler from clean (runtime library built beforehand) | 91 s wall, 255 s CPU (cargo, dependencies included) | 100 s wall, 1,003 s CPU (`make -j12`) |
| The compiled prismc compiling itself | 72 s, 724 G instructions, 3.5 GB peak | 61 s, 727 G instructions, 1.2 GB peak |
| Instructions, compiling `sem-ch07-normalize.prism` (`time -l`) | 1.40 G | 1.52 G |
| Lines emitted for prismc | 103,850 | 154,462 |

- **What moved the numbers:** `Text` as UTF-8 with cursors (Rust 4.8 G → 1.4 G, the
  `ropey` rope gone); per-module C++ headers (C++ build CPU down about a sixth); the
  non-atomic `prism_rtl::Rc` and less string churn (C++ 1.79 G → 1.52 G, the C++
  self-compile 77 s → 61 s).
- **This table predates the arrays, regex and module work;** the later preliminary numbers
  are `claude/benchmarks-preliminary.md`. The benchmarks proper are
  `bench/building/run.sh` and `bench/running/run.sh`, to be run on a quiet machine once
  the window is fixed, and they write `RESULTS.md` beside themselves.

## The order (Jimmy's)

1. **The array fix** and **regular expressions in the compiler and its lexer**: done.
2. **The layout bug** (The window, below): the library's part is done, the rest waits on
   Jimmy's rulings. He asked for no full benchmark run until it is fixed.
3. **The final fixed point and both seeds**, then **the final benchmarks** on a quiet
   machine, and from them **a recommendation: Rust or C++ as the default back end**,
   argued from both models, running and building (Jimmy).
4. **The umbrella PR's write-up** (below).

- **Other agents push to this branch** (bug reports under `claude/bugs/`, notes here):
  fetch before starting and before every merge.
- **A GUI benchmark opens windows on Jimmy's screen** and needs the display awake: ask or
  wait for a time he gives; the other benchmarks and every test are headless or brief.

## On macOS

- **Both canvas Worlds run natively:** `worlds/canvas/rust` (winit) and `worlds/canvas/cpp`
  (`src/app_macos.mm`, Cocoa). Each has `Hooks` and a smoke test driven through them
  (`hooks_smoke`, `macos_smoke`); the Xvfb tests and the XTest robot are Linux-only.
- **Homebrew:** `cairo freetype pkg-config gtk+3 llvm re2` (llvm for the sanitizer run).
- **RE2 is a prerequisite of every C++ build** (`brew install re2`; Linux: `apt install
  libre2-dev pkg-config`): the C++ runtime's library holds the regex engine, and its
  Makefile stops with that instruction when RE2 is missing.
- **Timings:** quit everything else, plug in power; `bench/run.sh` and
  `bench/compiler/run.sh` record the machine and the load average.
- **Instruction counts** come from macOS's `time -l` ("instructions retired"); valgrind
  does not run on arm64 macOS.

## Performance work still open

- **Gap OC** (prismc/README.md): Rust still clones an owned match binding used once.
- **SC1, SC2, SC4 to SC7** (prismc/README.md, "Strings: what is copied"): the string
  copies left.
- **The performance table for the PR** comes from `bench/building/run.sh` and
  `bench/running/run.sh`: cargo release against clang `-O3` only.

## Regular expressions

In both runtimes and in prismc: the literals `%/.../`, `%/.../.../` and the hashed form,
checked before tick 0 by `Check.Regex` against claude/regex-dialect.md (held to the
runtimes' corpus, tests/regex/corpus.txt); the prelude's `?Regex`, `?Match`, `?Find`,
`Matches`, `Replace` and `Split` (the old text splitter is `SplitOn`); prismc's own lexer
reading its token classes by anchored `?Match`, its output unchanged and its cost within
a percent (prismc/README.md, Regular expressions; claude/backend.md; D18). Left: the
lexer generator doubt (a token table as one automaton) stands, unmeasured; layout as a
separate pass over the token stream was not in this work.

## The window

- **A small change to the window still costs most of a create1k**
  (`claude/bugs/window-relayout-every-change.md`). Merged so far: `@Distinct`, cached
  `let`s, `for` builders that keep unchanged rows, equality that stops at shared nodes
  (create1k 702 ms to about 50 ms in C++, select 797 ms to about 42 ms, create10k 57 s to
  0.4 s). Jimmy's bar is a select under a tenth of a create1k; it stands at about 0.8 by
  instructions, split by phase in `claude/benchmarks-preliminary.md`.
- **The rest is designed, not built:** `claude/incremental-host-document.md` recommends an
  incremental host document and a `Cached` combinator, and asks Jimmy four things first
  (LG, CA, PB, RA there). Build nothing of it before he answers. His limits: the simplest
  thing that works, layout stays in Prism, no general retained-mode system.
- **The Cuis table crashes on `▶`; Linux Cuis times measure software painting**
  (`claude/bugs/bench-cuis-latin1-and-software-paint.md`); Jimmy paused the Cuis runs.

## Smaller items

- **The C++ Worlds moved and leave old build directories behind.** Since `15d1ab8` they
  are `worlds/{cli,canvas}/cpp`; a checkout that built at the old paths keeps
  `worlds/cli-cpp/build` and `worlds/canvas-cpp/build`, which show as untracked. Nothing
  to fix in the repo; delete them by hand.

- **Lexer simplifications** (ideas Jimmy asked about; "don't do it just noodle"):
  - walk with cursors (part of item 6);
  - layout as a separate pass over the token stream, the way Haskell's layout algorithm
    works;
  - template lexing (about 330 lines) as its own module;
  - the layout-corpus notation and Rust `{:?}` escaping (about 135 lines), which exist
    only to diff against `prism_syntax`'s lexer.
- **Over-engineering to look at** (Claude's suspicion, not read closely enough to claim):
  - the Rust-specific bookkeeping in lowering's shared model (clones, borrows, `Rc`),
    which may belong in the Rust printer;
  - the three hosting forms, which may be one form with two thin wrappers.
- **C++ build noise:** emitted chess C++ gives many g++ `-Wmaybe-uninitialized` warnings
  at -O1 (e.g. a `FrameSpec` copy in `lay_frame`). The runtime test silences a false one
  under ASan (`test_lang.cpp`).
- **`TRY`** is a GNU statement expression (g++ and clang only); the docs say so. Lowering
  refuses `?` of a place with a hard error, which no source reaches, so it has no test.
  `tests/compiled/run.sh` has no way to check a refusal yet.
- **Remaining gaps:** see `prismc/README.md` (NS, NL, RE, PL, CI, CY, GC, FA, MH1–MH4).
- **Doubts for the PR write-up:**
  - CW-color: signatures with `$`/`!`/`@` keep unwritten colors bounded;
  - pick CS, place numbering;
  - the driver refuses Ui-only roots;
  - the per-module-header cycle refusal.
- **The Test World in Prism:** `claude/test-world-in-prism.md`; Claude recommends (a),
  programs as values with a uniquely owned handle. Nothing to do until Jimmy rules.

## The umbrella PR

- One PR from `claude-self-hosting`, titled "Claude: ... (do not merge)". It includes the
  C++ target. There are no per-change PRs.
- One write-up covering everything:
  - the self-hosting fixed points (Rust and C++) and both seeds;
  - the layout of the repository;
  - the performance table;
  - the GUI benchmark's results, if done;
  - every doubt above and every remaining gap;
  - the books' rulings made during this work (Semantics rounds 1 to 4, Type System
    rounds 1 to 3), by name.
- End the description with the PR attribution lines.
