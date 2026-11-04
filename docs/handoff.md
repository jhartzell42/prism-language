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
- **Branch `claude-headers-rebased`** (a worktree at `../prism-headers`) is
  `claude-cpp-headers-wip` cherry-picked onto this branch and merged with the canvas
  World's template; it type-checks and has never been built (queue item 1).
- **The books** are `jhartzell42/prism-notes` at `e66aba7` or later. The Semantics
  RULINGS file ends with round 4 (TX, Text).
- **The bootstrap.** Both seeds exist and match the tree: `bootstrap/prismc-rust.tar.xz`
  (the Rust crate) and `bootstrap/prismc-cpp.tar.xz` (the C++ program), each written from
  its fixed point on 4 November. Queue item 8 rewrites them once more at the end.
- **The benchmarks** are `bench/` (the GUI table, four ways) and `bench/compiler/` (Rust
  against C++, each all one language); `RESULTS.md` beside each is the last run.
- **No agents are running**, and no scheduled watchdogs remain.

## Measured so far

On the container: 2 cores, 7 GB, g++ 13, nothing else running unless noted.

| Run | Rust | C++ |
|---|---|---|
| Build the compiler | 112 s (cargo release, opt-level 3, both cores) | 848 s (g++ -O1, `make -j1`, one big header) |
| prismc/ compiled by the Rust-built prismc | 224–246 s to Rust, 2.53 GB peak | 262–265 s to C++ |
| The compiled prismc compiling itself | 224–232 s | 129 s (to C++, 158,788 lines, -O1) |
| Instructions, compiling `sem-ch07-normalize.prism` to Rust (callgrind) | 4.07 G | 1.60 G |
| lexer-checks / parser-checks suites | 19 s / 35 s | 43 s / 87 s |
| `prism check prismc/` | about 45 s | |

- **Why C++ runs faster:** in the Rust-built prismc, about 64% of the instructions are
  `ropey` (Rust's `Text` is a rope; the lexer makes one per character, and `Map<Text,
  _>` compares ropes). In the C++ prismc, about 25% is malloc/free and about 10% is
  `shared_ptr`'s atomic count.
- **Every difference in the self-compile favors Rust** (opt-level 3 against -O1, and the
  C++ run emits 158k lines against 91k), so the like-for-like run can only widen the
  gap.
- **The like-for-like script** (all four jobs: both binaries to Rust and to C++, with
  peak memory through a getrusage wrapper; `/usr/bin/time` is not installed on the
  container) was in the container's scratchpad and is gone. Rewrite it in
  `bench/compiler/`.
- **The Rust binary for these runs** must be a current prismc. The installed
  `target/bootstrap/prismc` dated 3 November has no `--target` and ignores it (below).

## The order (Jimmy's)

- **First, the bug fixes** (the GUI benchmark is done):
  - gaps LP, LW, DM and SR;
  - **a `for` row that reacts to its block's event reacts to the wrong event**
    (`docs/bugs/for-row-block-event.md`): lowering resolves the block's event by index in
    the row's own event table. Silent wrong code on both targets. Fix first: it is the
    worst of the three.
  - **an inline supplier's `o` is not rank-2** (`docs/bugs/supplier-rank2.md`): two
    tag arms with different outputs are refused. The rule is settled there: an
    element's output variable confined to one function-typed parameter is quantified
    at it. Implement it, failing tests first, and add the rule's sentence to Type System
    ch. 5.
  - **a `for` misses a real prompt cycle through its own gathered dynamic**
    (`docs/bugs/host-doc-prompt-cycle.md`), which the host form correctly refuses. The
    checker stays blind to which dynamics are structural (Jimmy). Make a `for`'s and a
    `rebuild`'s result prompt in what their rows read, failing tests first. The design
    direction at the end of that file (using this to remove owed ticks) is for a design
    round with Jimmy, not for code.
  Failing test first for each bug.
- **Then the queue below, items 1 to 9.** All the emitter changes go first, then one
  fixed point at the end.
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

1. **Per-module headers** (continue `claude-cpp-headers-wip`).
   - What exists:
     - Lowering records `ModDef.complete` (modules whose types this module holds by
       value) and `ModDef.declared` (the rest of its `use` closure and emitted families),
       plus `Model.elemComplete`, `elemDeclared` and by-value `cycles`. The Rust printer
       ignores them.
     - `emit_cpp` writes `NAME.decl.hpp`, `NAME.types.hpp`, `NAME.hpp` per module, plus
       `elem.decl.hpp` and `elem.types.hpp`, and moves the programs into `main.cpp`.
     - A by-value module cycle is refused, naming it. The Makefile uses `-MMD -MP`.
     - `prism check prismc/` passes.
   - Next: rebuild the prismc in `target/bootstrap/next-cpp` from the WIP branch, run
     `tests/compiled/run.sh --target cpp`, fix missing includes, then time
     `make -j$(nproc)` of the C++ prismc before and after.
   - Pinning test: `ts-ch05-crossed-modules`.
   - Doubt: the design refuses any by-value cycle between modules even when no type
     cycle exists, which the one big header accepted; prismc itself may hit it.
   - Doubt: `declared` assumes a module names types only from its `use` closure; this is
     untested for emit-only families and for trait namespaces called from other modules'
     generics.
   - Jimmy: "There shouldn't be one big-file header... Figure out how to track includes
     even if it's code Rust backend ignores." The precompiled-header idea was dropped.
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
6. **Text as Semantics round 4 TX specifies** (Appendix B, Text; read it first). This is
   a language change, Jimmy's: "Do Rust's philosophy and have byte indexes and character
   iteration only! ... YAGNI".
   - The prelude:
     - A `Text` is UTF-8; `Len` is bytes.
     - `Front`, `Back: fn Text -> Cursor`.
     - `?Step`, `?StepBack: fn Cursor -> ?((Char, Cursor))`.
     - `Before`, `After: fn Cursor -> Text`, constant views.
     - `At: fn Cursor -> Nat`.
     - `?CursorAt: fn Text, Nat -> ?Cursor`, `nil` past the end or inside a character
       (Jimmy: "you can get a cursor from a byte offset why not?").
     - `Cursor` is opaque.
     - `Take`/`Drop` on a text and `t[i]` on a text are type errors; `Take`/`Drop` stay
       for arrays.
     - Texts order bytewise.
   - Both runtimes: a shared UTF-8 buffer plus a byte range (Rust `Rc<str>` and a range;
     C++ a shared `std::string` and a range, in place of the `u32string`). Drop `ropey`.
   - prismc itself:
     - The lexer walks the source with cursors instead of `Chars src` (an array of
       one-character texts, `cs: [Text]`, `Peek l = Or l.cs[l.pos], ""`).
     - `Chars`, `AllChars`, `Reverse`, `Join`, `TrimStart`, `TrimEnd` and `LeadingIn` move
       to cursors.
     - `index_text`, `take_text` and `drop_text` leave both preludes and lowering.
   - Failing tests first:
     - one conformance program per prelude function, including a non-ASCII text;
     - checker tests rejecting `Take` on a text and `t[i]` on a text. These edit
       `prismc/check-*.prism`; no other agent owns those files now.
   - Any transcript that counted characters in a non-ASCII text is re-derived and the
     rule named in the commit.
   - Instruction count before and after.
7. **(h) Split `prismc/lower.prism`** (about 7,000 lines, 27% of prismc) along its seams:
   expressions, nodes and widgets, hosting, the model and its types. No behavior change;
   the fixed point and the suites are the check. Claude's call; ask Jimmy whether to
   revert.
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
- **Remaining gaps** (`prismc/README.md`):
  - LP, a `while` nested in a `match` or `for`;
  - LW, a widget-block `let` at several instantiations;
  - DM, `host_k_doc` rebuilt whole on each read;
  - SR, a row entry recomputing its source on each read.
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
