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
  both runtimes (`claude/regex-dialect.md`, `tests/regex/corpus.txt`), not yet in the
  compiler; per-module C++ headers; the first three bugs and gaps LP, LW, SR; `Text` as
  UTF-8 with cursors; the performance queue (a non-atomic `prism_rtl::Rc`, less string
  churn, gap NL, `?ParseNat`, the emitted-code cleanups).
- **Branches in flight, each an agent's, merged here when green:**
  - `claude-arrays`: `claude/bugs/small-arrays-whole-chunks.md`. The C++ array's first
    chunks sized to fit and Rust's `Map` as the C++ AVL tree are done; one persistent
    tree-of-chunks `Array` in both runtimes, keeping Appendix B's costs (which the C++
    view breaks for `Append` and for `Push` onto an older view), is in progress.
- **The books** are `jhartzell42/prism-notes` `main` at `8260397` or later: Semantics
  round 4 (TX, Text), round 5 (regular expressions), round 6 (MT, TD: the module tree and
  test modules); Type System rounds 4 and 5.
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
- **The benchmarks proper** are `bench/building/run.sh` and `bench/running/run.sh`, to be
  run on a quiet machine after the style pass and regex (item 8), and they write
  `RESULTS.md` beside themselves.

## The order (Jimmy's)

1. **The array fix** (`claude-arrays`), merged.
2. **Regular expressions in the compiler** (lexer, parser, checker, lowering, both
   printers, the prelude), then prismc's own lexer written with them. Jimmy: the
   design is done and good; no design round.
3. **The layout bug** another agent is reporting under `claude/bugs/` (Jimmy: after regex
   in the lexer).
4. **The GUI table's costs**, traced and written up under `claude/bugs/`
   (`claude/benchmarks-preliminary.md`: every small operation costs what building the
   table costs on both targets; create-10,000 is 57 s in C++ against 8 s in Rust).
5. **The final fixed point and both seeds**, then **the final benchmarks** on a quiet
   machine, and from them **a recommendation: Rust or C++ as the default back end**,
   argued from both models, running and building (Jimmy).
6. **The umbrella PR's write-up** (below).

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
- **SC1 to SC7** (prismc/README.md, "Strings: what is copied"): the string copies left,
  among them SC3, C++ `Upper`/`Lower` being ASCII only where Rust maps all of Unicode.
- **The performance table for the PR** comes from `bench/building/run.sh` and
  `bench/running/run.sh`: cargo release against clang `-O3` only.

## Regular expressions as a primitive (Jimmy's direction; the design is settled)

Jimmy (4 November): implement it as below, the `%` literals included; the books first
(Semantics ch. 2, Appendix B, Type System), then the code, failing tests first.

Jimmy: Prism is a general-purpose language with systems-programming uses, and a lexer
written character by character is "shooting yourself in the foot". Regular expressions
belong in the runtime beside `Text`, as a primitive, so that prismc's lexer (and every
program's) shrinks.

- **Literals.** A pattern is a `Regex`; a pattern and a replacement is a
  `RegexReplacement` (names to settle), Perl and sed style. Two spellings considered:
  - **`%`, Jimmy's idea, and Claude's recommendation as a mark, not a sigil.**
    `%/pattern/` is a `Regex` and `%/pattern/replacement/` a `RegexReplacement`.
    - `%` is free: Semantics ch. 2 refuses a `%` operator so that "funny characters
      are all sigils or marks", and a literal's mark fits that.
    - No name is taken, and there is no ambiguity with division.
  - **Why not a sigil:** a sigil on a name says *when* (Pitch ch. 3; it is the
    outermost time layer of the name's type, D12). A regex is a value, not a time
    layer, so a `%name` would be the one sigil that does not answer "when".
  - **The refused spelling:** `r/pattern/` and `s/pattern/replacement/` take `r` and
    `s`, which would otherwise be names, and need the spacing rule to tell `r/x/` from
    `r / x`.
  - Flags after the closing slash, settled with the literal (`r/x/i`).
  - Other delimiters (`r{...}`, `r#/.../#`) for patterns holding `/`.
  - Interpolation is refused or explicit: a pattern built from text goes through a
    function, `?Regex: fn Text -> ?Regex`, never a placeholder inside a literal.
- **Checked at compile time.** A literal that is not a valid pattern is a diagnostic,
  before tick 0, never a fault at run time. Capture groups could give the match its
  type: a record of named groups, a tuple of numbered ones.
- **Semantics consistent with TX.** Matching is over UTF-8 bytes; a match reports
  cursors (`Before`/`After` of a match are views), never character counts.
- **Cost promise.** Linear in the text (RE2 and Rust's `regex`: no backreferences, no
  lookaround), so a program's cost stays a property of its text (Appendix B, TD-V2).
- **Prelude** (names to settle):
  - `?Match: fn Regex, Cursor -> ?Match` anchored at the cursor, which is what a lexer
    wants;
  - `?Find: fn Regex, Text -> ?Match`;
  - `Matches: fn Regex, Text -> [Match]`;
  - `Replace: fn RegexReplacement, Text -> Text`;
  - `Split: fn Regex, Text -> [Text]`.
- **Runtimes.**
  - Rust: the `regex` crate (or `regex-automata` for anchored longest-match lexing).
  - C++: RE2 (apt `libre2-dev`, Homebrew `re2`) rather than `std::regex`, which is slow
    and backtracks.
  - Both must give byte-identical results on the conformance suite, so the dialect is
    the intersection the books specify, not either library's.
- **The lexer afterwards.** Each token class becomes a literal (names, numbers,
  durations, operators by longest match, string bodies), with layout as a separate pass
  over the token stream (see the lexer items below). Measure: lines of `lexer.prism`
  before and after, and instructions on the normalize compile.
- **Doubt:** a lexer generator (a token table compiled to one automaton, like `logos` or
  re2c) may serve the lexer better than separate regexes tried in turn. Both can sit on
  the same `Regex` primitive.

## From the cloud GUI run (5 November), after the queue

- **A small change to the window still costs more than create1k**
  (`claude/bugs/window-relayout-every-change.md`, branch `claude-layout`): the window
  draws only what shows and the table is ten to a hundred times faster, but select lays
  the window out six times for one change. What is left needs an `^Eq` refire-dropping
  builtin (doubt DQ, against Appendix B's D-DT) or lowering work on the hosts.
- **C++ `Union` is quadratic in a fold** (`claude/bugs/cpp-union-big-into-small.md`).
- **The Cuis table crashes on `▶`; Linux Cuis times measure software painting**
  (`claude/bugs/bench-cuis-latin1-and-software-paint.md`).

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
