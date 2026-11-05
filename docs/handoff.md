# Handoff: where the self-hosting branch stands

*Written by Claude on the claude-self-hosting branch, 4 November 2026, for the move from
the cloud container to a local macOS run; subject to change. Everything still to do, with
the state each item is in. Delete an item when it is done; delete the file when it is
empty.*

## Where things are

- **Branch `claude-self-hosting`** is green on macOS (an M2 Pro): the Rust and the C++
  fixed points, the C++ prismc emitting the Rust prismc's Rust byte for byte, both
  compiled suites, the checker, the driver tests, the C++ runtime's and Worlds' tests
  (ASan+UBSan under Homebrew clang; Apple clang's hangs at startup on macOS 26).
- **Merged today (4 November):** per-module C++ headers (queue item 1); the three bugs of
  `docs/bugs/` and gaps LP, LW, SR (DM is documentation); `Text` as UTF-8 with cursors
  (item 6). Running: an agent on items 2–5 and gap NL (branch `claude-perf`).
- **The books** are `jhartzell42/prism-notes` at `e66aba7` or later. The Semantics
  RULINGS file ends with round 4 (TX, Text).
- **The bootstrap.** Both seeds exist and match the tree: `bootstrap/prismc-rust.tar.xz`
  (the Rust crate) and `bootstrap/prismc-cpp.tar.xz` (the C++ program), each written from
  its fixed point on 4 November. Queue item 8 rewrites them once more at the end.
- **The benchmarks** are `bench/` (the GUI table, four ways) and `bench/compiler/` (Rust
  against C++, each all one language); `RESULTS.md` beside each is the last run.
- **No agents are running**, and no scheduled watchdogs remain.

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

- **The queue below, items 2 to 5, 7 to 9** (1 and 6 are done). The emitter changes first,
  then one fixed point at the end.
- **Then the style pass** (Jimmy, 4 November), after the queue and before the regex code:
  - a module divided into sections becomes separate files; more hierarchy in the tree;
  - every module's main file is `mod.prism` (Rust's rule); `lib.prism`, `mod.prism` or
    `app.prism` declares its submodules, and `app.prism` the main widget;
  - top-level items that exist only for testability move out of the module into a
    `test.prism` beside it, run as `.prismtest` is today.
  A module-system change as much as a reshuffle: prismc's front end, the checker's module
  graph, the `prism` script and the Semantics chapter on modules. Write the design as a
  short note first, do it, and put the calls in it as doubts.
- **Then regular expressions** (the code; the books are written on `claude-regex-books`).
  Jimmy (4 November): the design is done and good; no design round.
- **Then the final benchmarks** (bench/building, bench/running, bench/gui), on a quiet
  machine, and from them **a recommendation: Rust or C++ as the default back end**, argued
  from both models, running and building (Jimmy).
- **Makefiles and paths (IC):** the emitted Makefile takes `PRISM_ROOT` from the
  environment, the relative path only as the fallback, and `prism`, the bootstrap scripts
  and the test runners export it from their own location (`pwd -P`), so a build tree
  reached through a symlink (a `target/` moved out of iCloud) still finds the runtime.
  Check what cargo does with a symlinked manifest directory and fix the Rust side the
  same way. Right after the current merges.

## On macOS

- **Both canvas Worlds run natively:** `worlds/canvas/rust` (winit) and `worlds/canvas/cpp`
  (`src/app_macos.mm`, Cocoa). Each has `Hooks` and a smoke test driven through them
  (`hooks_smoke`, `macos_smoke`); the Xvfb tests and the XTest robot are Linux-only.
- **Homebrew:** `cairo freetype pkg-config gtk+3 llvm` (llvm for the sanitizer run).
- **Timings:** quit everything else, plug in power; `bench/run.sh` and
  `bench/compiler/run.sh` record the machine and the load average.
- **Instruction counts** come from macOS's `time -l` ("instructions retired"); valgrind
  does not run on arm64 macOS.

## The queue, in order

All the emitter changes go first; then one fixed point at the end.

2. **Re-time the C++ build at `make -j$(nproc)`** and record cargo's parallelism (codegen
   units), since the -j1 number was unfair (Jimmy).
3. **Cleanups that change emitted code**, one commit each, with build and self-compile
   times before and after:
   - (b) a `match` that is a whole function body lowers to an `if`/`return` chain, not
     `({ __label__ _done; Slot; goto })` (see `subst` in `sem-ch07-normalize`);
   - (c) drop `mutable` from node fields (only level readers like `label` are `const`,
     and they must not write);
   - (d) drop the `if (true)` / `if true` wrapper around merges in `wake`, in both
     targets;
   - (e) Rust: skip the clone of a binding used once, e.g. `Some(b2) =>
     Some(Term::Lam(Rc::new(b2.clone())))` in `?Reduce`, if cheap; otherwise list it.
4. **(f) A non-atomic count in C++:** a `prism_rtl::Rc<T>` (one allocation, non-atomic,
   weak where the runtime needs weak) in place of `std::shared_ptr`. Programs are
   single-threaded (D4). First check that no World and no `on_big_stack` hands values
   across threads concurrently; if one does, report it instead.
5. **The string-churn audit (Jimmy: "super important")** in both runtimes and templates:
   a `String` built and then copied into a `Text`, `format!` into a temporary and then
   `Text::new`, `to_string()` in comparisons, character-by-character pushes. Fix what's
   cheap, list the rest. Instruction counts before and after on the normalize compile.
7. **(h) Split `prismc/lower.prism`** (about 7,000 lines): folded into the style pass below
   (a module's sections become separate files), which does it for every module at once.
8. **The final fixed point:** `make bootstrap-cpp`, the C++-built prismc's Rust output
   byte-identical to the Rust-built prismc's, then both seeds (`bootstrap/bootstrap.sh
   --update` for the Rust seed if Rust emission changed; write
   `bootstrap/prismc-cpp.tar.xz`). Re-install `target/bootstrap/prismc` so `./prism` uses
   a current compiler.
9. **The performance table**, for the PR:
   - C++ at -O0, -O2 and -O3 with `make -j$(nproc)`; Rust as a cargo debug build and a
     release build (opt-level 3).
   - For each: build time, the four like-for-like self-compiles (both binaries to Rust
     and to C++), peak memory, and instruction counts on the normalize compile.
   - Jimmy: "that last data point—which amounts to, which compiled code is faster—is
     key."
   - If -O3 runs out of memory on a file, say which and how much.
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

## Smaller items

- **An absolute `-o` gets runtime paths relative to the wrong directory.** Run from a
  directory other than the repository root, `target/bootstrap/prismc compile prismc
  ... -o /abs/path/target/bench/r2r` wrote `prism_rtl = { path =
  "../../../../../../runtime/rust" }` into the crate's `Cargo.toml`, which resolves to
  `/runtime/rust` (seen on the cloud box, 5 November). The relative path is computed from
  the current directory, not from the output directory. `./prism` hides it by passing
  `--runtime` for an absolute output. prismc should compute it from the output's own
  location, or use `PRISM_ROOT` as the C++ Makefiles do. Failing test first.
- **The seeds carry macOS extended attributes.** Written on the Mac, both
  `bootstrap/prismc-rust.tar.xz` and `bootstrap/prismc-cpp.tar.xz` hold
  `LIBARCHIVE.xattr.com.apple.provenance` headers, so unpacking them on Linux prints one
  `tar: Ignoring unknown extended header keyword` warning per file (seen on the cloud box,
  5 November). Harmless, but noise in every fresh bootstrap. `--update` in both bootstrap
  scripts should pack without them: `COPYFILE_DISABLE=1` before `tar`, or bsdtar's
  `--no-xattrs` (GNU tar ignores it, so test the flag on both). Then rewrite both seeds.
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
- **The Test World in Prism:** `docs/test-world-in-prism.md`; Claude recommends (a),
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
