# Handoff: where the self-hosting branch stands

*Written by Claude on the claude-self-hosting branch, 4 November 2026, for the move from
the cloud container to a local macOS run; subject to change. Everything still to do, with
the state each item is in. Delete an item when it is done; delete the file when it is
empty.*

## Where things are

- **Branch `claude-self-hosting`** is green at `f8ac5b5`: `make test` (cargo, both
  compiled suites, the checker harness, the driver test, `make -C runtime/cpp test`, the
  bootstrap).
- **Branch `claude-cpp-headers-wip`** (`61ba1b2`) holds per-module C++ headers, half-done
  and never compiled. Not for merge; continue it (below).
- **The books** are `jhartzell42/prism-notes` at `e66aba7` or later. The Semantics
  RULINGS file ends with round 4 (TX, Text).
- **The bootstrap.** `bootstrap/prismc-rust.tar.xz` is the Rust seed.
  `bootstrap/bootstrap-cpp.sh` and `make bootstrap-cpp` reach the C++ fixed point, but
  the C++ seed `bootstrap/prismc-cpp.tar.xz` is not written yet (Jimmy wants both seeds).
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

- **First, the GUI benchmark** (queue item 10) **and the bug fixes:**
  - the installed prismc ignoring `--target cpp` silently;
  - the C++ template's missing canvas World;
  - the stale canvas README;
  - gaps LP, LW, DM and SR;
  - **a lambda supplier's `o` is not rank-2.** An inline supplier that answers two tags
    with different outputs is refused (`Fs.Contents(..): @ $ ...` then `Fs.File(..): @
    never` gives "expected `$~Contents`, found `!t20`"), while the same supplier as a named
    function with a signature is accepted.
    - Cause: tag arms refine only a rigid `o` (`InferMatch`/`InferArm` in
      `check-expr.prism`), and a rigid variable comes only from a declared signature; a
      lambda argument gets `@HostDocument`'s `o` as a flexible variable from
      instantiation. Hosting is not special-cased, and the checker has no rank handling.
    - The books: Type System ch. 2, "The supplier is rank-2 in `o`, and the signature
      supplies it"; ch. 5, "Refinement at tag matches"; Semantics ch. 9's `@Offline` is
      the refused case.
    - Design question for Jimmy: which rule makes `o` rank-2 in a signature. Claude's
      recommendation: a variable that appears only inside one function-typed parameter is
      quantified there (a grep finds only the three host forms in the prelude, prismc,
      the examples and the tests). Alternatives: an explicit quantifier in the
      parameter; special-casing the host forms (refused: it makes hosting privileged).
    - Fix: at a call, skolemize the confined variable for a lambda argument and check
      the lambda against the parameter, with an escape check. Tests first:
      `ts-ch05-supplier-rank2` (the refused program, both places answered) and a
      `refinement-escape` negative.
  - **a cycle through a host's own `$doc` is refused as a prompt cycle** (`$doc`,
    `$answers`), though `$doc` is structural and lags a tick; a hold breaks it. Either
    the prompt-cycle check ignores structural lag, or the lag is not meant to break a
    cycle. Needs Jimmy's ruling before a fix.
  Failing test first for each bug.
- **Then the queue below, items 1 to 9.** All the emitter changes go first, then one
  fixed point at the end.
- **Last, regular expressions:** after everything else, designed in conversation with
  Jimmy before any code.

## On macOS

- **The canvas World in Rust** (`worlds/canvas`) runs natively; chess runs under it.
- **`worlds/canvas-cpp` needs a macOS `src/app.cpp`:** Cocoa (an `NSWindow` whose view
  draws the cairo surface as a `CGImage`, the loop on the main thread) or SDL2. `brew
  install cairo freetype pkg-config`.
- **The Xvfb smoke tests and the XTest robot are Linux-only.** On macOS, drive the
  benchmark through `Hooks::settled` and synthetic presses, not a robot.
- **GTK:** `brew install gtk+3` (it runs on Quartz).
- **Cuis:** clone `Cuis-Smalltalk/Cuis-Smalltalk-Dev` for the image, and get the Cog VM
  for macOS. On a Mac the OpenSmalltalk release binaries may be reachable; otherwise
  build `opensmalltalk-vm`'s macOS Cog Spur build from source.
- **Timings:** quit everything else, plug in power, and run each timing several times.
  Record the machine (chip, cores, memory) in `bench/RESULTS.md`.
- **Container-only tools** (`valgrind`, `xvfb-run`) may be missing. For instruction
  counts on macOS, use `valgrind` if Homebrew has it for the chip, else Instruments or
  `perf`-like sampling, and say which.

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
10. **The GUI benchmark** (Jimmy's choices: the C++ canvas World, the js-framework-benchmark
    operations, Cuis Smalltalk on the Cog JIT VM built from source, builds before
    timings).
    - Done: `worlds/canvas-cpp` (`f8ac5b5`).
      - Xlib with MIT-SHM, cairo and FreeType; the Rust World's DejaVu Sans; the Rust
        crate's unit tests; the Xvfb smoke test.
      - Chess compiled to C++ plays e2–e4 and exits 0:
        `PRISMC=target/bootstrap/next-cpp/prismc make -C worlds/canvas-cpp chess-smoke`.
    - On macOS, only `worlds/canvas-cpp/src/app.cpp` (the Xlib loop and painting) needs
      replacing: Cocoa (an `NSWindow` whose view draws the cairo surface as a
      `CGImage`) or SDL2. Cairo and FreeType come from Homebrew. The robot and the smoke
      tests are X11-only.
    - The emitted C++ has no canvas World: `CppRunRoot` in `prismc/templates/cpp.prism`
      prints "the C++ target has no canvas World". The port works around it with `make
      program PROG=DIR ROOT=MainProgram`, which renames the emitted `main`. Fix it in the
      template: the canvas branch calls `prism_world_canvas::run<P>()`, and the emitted
      Makefile links `worlds/canvas-cpp` for a canvas root.
    - `bench/` itself, the GTK app (C; `libgtk-3-dev`), Cuis with Morphic, the Cog VM
      build and `bench/run.sh` with `bench/RESULTS.md` are not started. The design the
      agent planned:
      - each app runs its own script of operations;
      - Prism presses at a button's clickable centre and stops the clock at the canvas
        World's `Hooks::settled`;
      - GTK calls `gtk_button_clicked` and waits for the frame clock's after-paint, then
        a display sync;
      - Cuis performs the button's action, then forces a World cycle;
      - the Prism build uses -O2, as `./prism run --target cpp` does;
      - the timing is median and spread over N runs, plus peak memory.
    - Expected cost: the window library recomputes the whole layout as pure functions on
      every change, so 10,000 rows will be slow; the benchmark measures it.
    - Timings need a quiet machine: on macOS that is the whole machine, with nothing
      else building.
    - Doubt: a display without MIT-SHM is painted through `XPutImage` with a message on
      stderr. Jimmy may want a hard error there instead.

## Regular expressions as a primitive (Jimmy's direction, to design)

After everything else, and in conversation with Jimmy: propose, wait for his go, then
write the books, then implement.

Jimmy: Prism is a general-purpose language with systems-programming uses, and a lexer
written character by character is "shooting yourself in the foot". Regular expressions
belong in the runtime beside `Text`, as a primitive, so that prismc's lexer (and every
program's) shrinks. Write the design before implementing; the books need it (Semantics
ch. 2 for the literals, Appendix B for the prelude, Type System for the types).

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

- **The installed prismc ignores `--target cpp` silently.** `./prism compile --target
  cpp examples/chess -o target/x` with the 3 November `target/bootstrap/prismc` exits 0
  and writes a Rust crate. prismc must refuse an unknown option, and `./prism` should
  check that the output is the target asked for. Failing test first. Re-installing (item
  8) hides it but does not fix it.
- **Dead code:** 10 top-level definitions are never referenced. Delete them, and add a
  checker warning for unused top-level definitions so this cannot build up again:
  - `check-env.prism`: `IsIntType`, `TypeSpan`;
  - `check-passes.prism`: `ParamRels`;
  - `check-types.prism`: `FreeRowVars`;
  - `emit_cpp.prism`: `ArmConds`;
  - `lower.prism`: `?AppAt`, `HeightOf`, `HostField`, `HeldTy`, `PortDef`.
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
- **Stale doc:** `worlds/canvas/README.md` says the World "serves the interpreter
  (prism_zero)". There is no interpreter.
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
