# `prismc`: the front end, in Prism

*Written by Claude, 31 October 2026, on the `claude-self-hosting` branch. Work in progress
and subject to change: Jimmy rewrites it by hand, and an implementer reading it uses their
own discretion. The books in `prism-notes/claude/prism/` are the specification;
`prism_syntax` is the oracle this front end is diffed against, and where the two disagree
the book decides which one has the bug.*

The lexer, parser and tree of Prism, written in Prism and run by compiler zero
(`prism_zero`). It gives the same tree as `prism_syntax`, node for node, and prints it in
`prism_syntax`'s S-expression notation so the two can be diffed.

## Files

- `ast.prism` (module `Ast`): the tree, `prism_syntax/src/ast.rs` one to one, with a
  `Range` on every node; `Diag`, the diagnostic.
- `lexer.prism` (module `Lexer`): characters to tokens with layout (Semantics ch. 2, the ten
  layout rules, the template frame; Conformance A.1 and A.2), as an `&` block over the text
  (Q-L1 (b)); `RenderTokens`, the layout corpus's notation; Rust's `{:?}` escaping.
- `parser.prism` (module `Parser`): `unbounded` recursive descent over a token cursor
  (D7), with the pairing rule, a Pratt loop over the 14-level table, recovery at statement
  boundaries, out-of-scope diagnostics, and the test language.
- `sexp.prism` (module `Sexp`): the S-expression printer, `prism_syntax/src/sexp.rs`
  exactly.
- `check-errors.prism`, `check-types.prism`, `check-env.prism`, `check-expr.prism`,
  `check-blocks.prism`, `check-passes.prism`: the checker (Checker, below).
- `front.prism` (the root): `@main`.

## Running it

```
prism_zero run prismc/*.prism -- FILE [OUT]              # the tree, as `dump FILE` prints it
prism_zero run prismc/*.prism -- --tokens FILE [OUT]     # the tokens, as `dump --tokens`
prism_zero run prismc/*.prism -- --fragment FILE [OUT]   # a run of statements, `dump --fragment`
tests/prismc/compare.sh [--tokens|--fragment] [FILES...] # diff against prism_syntax
```

`@main` reads FILE through `Fs.Contents`, runs the front end through `@Run`, and writes
exactly what `cargo run -p prism_syntax --example dump -- FILE` prints (the dump, then the
diagnostics, then a newline) through `Fs.Output` to OUT, `FILE.sexp` when OUT is left out.
When there is a lexical or parse error the file is written all the same, and the program
then gives up through `Prelude.Unwrap` with the rendered diagnostics as its message, which
the CLI World prints to standard error before it exits 1. Out-of-scope features are
reported in the output and do not fail the run.

`tests/prismc/compare.sh` runs both dumps over every `.prism` and `.prismtest` under
`examples/`, `tests/conformance/`, `prelude/` and `prismc/`, and exits 1 listing the files
that differ. `tests/prismc/lexer-checks.prism` and `tests/prismc/parser-checks.prism` are
`check`s on small inputs:

```
prism_zero test prismc/ast.prism prismc/lexer.prism tests/prismc/lexer-checks.prism
prism_zero test prismc/ast.prism prismc/lexer.prism prismc/parser.prism prismc/sexp.prism tests/prismc/parser-checks.prism
```

## Where it stands

- All 64 corpus files diff clean, `prismc/`'s own five included, in both the tree and the
  token mode.
- All 147 book examples in `prism_syntax/tests/book/` diff clean in their expected mode
  (138 as modules, 9 as fragments), the deliberately broken ones included: same
  diagnostics, same positions, same recovered tree.
- 670 files made by mutating corpus files (inserted brackets, keywords, sigils, operators
  and junk; deleted runs; duplicated lines) diff clean in both modes, which exercises the
  error paths and recovery much harder than the corpus does.
- Speed under compiler zero: the chess window library (890 lines) in about 15 seconds; the
  whole corpus in under two minutes.

## Picks

The places where writing the front end in Prism left a choice. Each is Claude's, for Jimmy
to overturn.

- **PF A failed parse step is a bad cursor.** Every parse function takes the cursor `Ps`
  and gives back `((tree, ps))`. A step that fails marks the cursor `bad`, and a bad cursor
  sees the end of the input and does not move, so everything after the failure falls
  through at once to the nearest recovery point (a statement boundary: `LayoutItems`,
  `StmtsUntil`, `MatchArms`, `ModuleItem`) or speculation (`OtherStmt`, `ParseCond`,
  `ForBindersOf`), which is where `prism_syntax`'s `?` lands. Diagnostics live in the
  cursor, so rolling a speculation back drops its diagnostics with its position, as
  `speculate` does. *Refused:* a result choice (`Ok a, Ps` or `No Ps`), which needs a
  `match` per step and stairs every function rightward; `?` names with `given:` blocks,
  whose failure is `nil` and so loses the diagnostic and the position recovery starts from;
  and a `^Monad` parser, which is out of this compiler (decisions.md).
- **AS The tree's spellings.** A sigil stack is the text of its sigils (`"@$?"`); a binary
  operator, a keyword, a bound name, an accumulation and a template keyword are the text of
  their spelling; a constructor carries a prefix of its type (`EName`, `PName`, `TName`,
  `SBind`), since Prism's constructors share one namespace per module; `Span` is `Range`,
  the prelude having a `Span` already (Q-D2 (b) names it `Range` too). Fields named for
  keywords are renamed: `emit` is `emitRow`, `of` is `ofType`, `trait_` is `traitRef`, a
  `Use`'s `module` is `target`.
- **CH Characters are one-character texts.** The lexer splits its text once into an array of
  one-character `Text`s and indexes it; a `Char` literal holds a one-character `Text`.
  Offsets are characters, not bytes; the diagnostics' `line:col` come out the same, since
  `prism_syntax` counts columns in characters.
- **UB What is `unbounded`.** Every function that recurses through the grammar or loops
  with no measure a checker could see (the lexer's loops over the text, the parser's
  descent, `Recover`); predicates and cursor primitives are bounded. The closure-taking
  helpers (`LayoutItems`, `Commas`, `Bracketed`) take a color variable (D8).
- **NA A non-ASCII character glued to a name** counts as a letter: Rust asks
  `char::is_alphabetic`, which Prism's prelude has no counterpart for. Both report an error;
  only the message for a glued non-letter symbol differs, and no corpus file has one.
- **DE Rust's `{:?}`** is reproduced for what Prism programs write: `\0 \t \r \n \\`, the
  quote, and `\u{..}` for the other ASCII controls; any other character prints as itself,
  where Rust would also escape unprintable and grapheme-extending code points.
- **OU The output goes to a file.** The CLI World has no standard output (`Fs.Output`
  writes beside the target and renames, which `/dev/stdout` cannot take), so the dump is
  written to OUT and `compare.sh` reads it from there.

## Workarounds

Places where compiler zero or the prelude lacks something, worked around in Prism.

- **UC `\u{hex}` reaches only ASCII.** No prelude function makes a `Char` (or a one-
  character `Text`) from a code point, so the lexer decodes `\u{0}` to `\u{7f}` from a table
  and reports any higher code point as an error naming this gap. No corpus file writes one.
- **SK The stack.** compiler zero's evaluator recurses on the Rust stack, about 15 KB per
  Prism call (IB3 below). The default 8 MB holds every corpus file, but brackets nested
  between 80 and 150 deep overflow it; `compare.sh` raises the limit with `ulimit -s`.
- **NS Sorting.** The prelude has no sort; the diagnostics are put in order by a stable
  insertion (`InsertDiag`), as `prism_syntax` sorts and dedups them.

## Interpreter bugs found

- **IB1 A lambda's parameter does not shadow a `let` of its enclosing block.** The body
  reads the block's `let` instead, and the evaluation fails as a cycle. A `match` arm's
  binder shadows correctly. Found in `ParseTestDecl`; the parameter was renamed.

  ```prism
  Apply: fn (fn Nat -> Nat), Nat -> Nat
  fn Apply F, x = F x

  Twice: fn Nat -> Nat
  fn Twice n:
      let a = Apply \|q| q + 1, n
      let q = a * 2
      q

  check shadow:
      Twice 1 => 4        # error: a is defined in terms of itself: a cycle among plain values
  ```

- **IB2 A `check` line is split at its first `=>`, inside a text literal too.**
  `prism_zero/src/driver.rs`'s `script_expressions` cuts each check line's source at the
  first `=>` to find its expression, so a text holding `=>` fails to load (`prism_syntax`
  parses the line correctly). `tests/prismc/parser-checks.prism` spells that arrow
  `\u{3d}>`.

  ```prism
  check arrow:
      "a => b" => "a => b"     # error: `"a`: a text literal ends on its line
  ```

- **IB3 Deep recursion aborts the process.** About 15 KB of Rust stack per Prism call level
  (an 8 MB stack holds some 500 levels), and an overflow is a `SIGABRT` with no diagnostic,
  where a fault naming the depth would let a program or a test report it.

  ```prism
  Deep: fn Nat -> Nat
  fn Deep n = if n == 0: 0 else: 1 + (Deep (Or (n -? 1), 0))

  check deep:
      Deep 1000 => 1000        # thread 'main' has overflowed its stack; aborted
  ```

- **IB4 A placeholder alone on its line lost its line's newline** (fixed in
  `prism_zero/src/eval.rs`, via `prism_dyn::rt::place_text`). Templates ch. 2: "the
  placeholder's line is replaced by" its text's lines; compiler zero wrote the lines with no
  final newline, so the next line of the template joined the placeholder's last line.

  ```prism
  F: fn Text -> Text
  fn F body = inline_text <rust>
  fn f() {
      {{body}}
  }
  </rust>

  check alone:
      F "x" => "fn f() \{\n    x\n\}\n"      # was "fn f() {\n    x}\n"
  ```

- **IB5 Two modules declaring one constructor name** (a regression of the `prism_dyn`
  extraction, fixed before it shipped): a constructor is built from its own declaration
  (`prism_dyn::values::ctor_fn`), as it was before; the tables keep one entry per bare name,
  which is what `^Ord` and trait dispatch read (they have always been program-wide).

## Back end

`compile` lowers the program to a core form and emits a Rust crate through code templates
(decisions.md D1, D16):

```
prism_zero run prismc/*.prism prismc/templates/*.prism -- compile FILES... -o DIR [--name NAME]
target/stage3/prismc -- compile FILES... -o DIR [--name NAME]       # the same, compiled
(cd DIR && cargo build --release)                                     # add --features canvas for chess
DIR/target/release/NAME [--world cli|test|canvas] [-- ARGS... | SCRIPTS.prismtest...]
```

- `lower.prism` (module `Lower`): the program (modules, declarations, `use` scopes, tags,
  constructors, impls, trait methods) and the core form: names resolved to Rust locals and
  global getters, layers read off signatures, patterns resolved, expression blocks' `let`s
  in the order compiler zero forces them, widget blocks' lines in the order compiler zero
  runs them (pick BG of prism_zero/README.md, done statically: positional lines in page
  order, forward events, deferred event `let`s, the holds' `then`s last), captures explicit.
- `emit.prism` (module `Emit`): the core form to Rust, the compile job (`ZCompile`, run
  through `@Run`), `main.rs` and `Cargo.toml`. A nested function is a Rust `fn` item hoisted
  to the top of the function it stands in, so an expression stays on one line.
- `templates/rust.prism` (module `RustTemplates`): every multi-line piece of Rust, as
  function-form templates with doubled braces; placeholders alone on their lines indent.
- `front.prism`: `@main`'s `compile` branch, `@CompileMain`.
- The emitted code is straight-line Rust over `prism_dyn::rt`: every value a
  `prism_rt::Value`, every application `apply(&f, vec![..], b)?` through the lifting table
  compiler zero uses. `.prismtest` files among FILES are compiled into the binary too: their
  `check` lines and their `test name, @program:` blueprints, run by `--world test SCRIPTS`.
- The checker is not called yet: `check-passes.prism` exposes no `Check` entry the back end
  can call; `compile` does not run it.
- Bootstrap: `tests/bootstrap/stage1.sh`, `stage2.sh`, `stage3.sh`, `check.sh` (stage 2 and 3
  byte-equal), all from one snapshot of the sources (`target/bootstrap-src/`);
  `compiled-tests.sh [PRISMC|zero]` compiles every conformance program and example and runs
  its script under the compiled Test World against its `.expected`. `make bootstrap`,
  `make test`, `make chess` at the root.

### Back-end picks

- **NM Names.** A Prism name's sigils become letters (`$` d, `!` e, `@` w, `?` q, `~` t, `&` r):
  `$count` is the local `ld_count`, `@counter` the getter `gw_counter`; a Prism module is
  the Rust module `m_<name>`, the root `m_main`. Refused: a table of numbered names, which
  Jimmy could not read.
- **GL Every global is a getter** made once per thread (`rt::global`), a function its
  closure: applications are uniform, `apply(&crate::m_lexer::g_Join()?, ..)`. Refused:
  direct Rust calls to known functions, which need the layers at the call site (a later
  optimization, typed codegen's job).
- **HI Hoisted items.** Lambdas, arms, widget blocks become `fn` items at the top of the
  enclosing function, named by their source offset (`lam_312`), their environment passed
  as `cap`/`env` slices. Refused: Rust closures capturing locals, which hide captured values
  from the collector (D5).
- **ST Static binding groups.** The order of a group's lines is decided at compile time by
  simulating compiler zero's forcing; the free-name walk is scope-aware (compiler zero's
  over-approximation, IB1, would make some `let`s cycles at compile time).
- **CP Cycle points** at an unbounded function's entry, counted only when an unbounded
  function is running (`rt::within_unbounded`), and per `while` turn: compiler zero's CP,
  so a compiled `@Run` and an interpreted one take the same number of cycles.
- **TC `<@Name ...>` in `inline_text`** calls the plain function `Name` (its attributes,
  then its body's text) when no `@Name` is declared; compiler zero does the same now.
  Templates ch. 2 gives the function form no call form, so this is an extension; doubt below.
- **LO Locations.** Each emitted function places the faults inside it at its source
  position (`within("prismc/lexer.prism:312:5", || ..)`), the innermost first.
- **RT The runtime path.** `Cargo.toml` finds `prism_rt`, `prism_dyn` and `worlds/canvas`
  at `--runtime PATH`, by default one `..` per directory of `-o DIR` (DIR under the repo
  root), so stage 2 and stage 3 emit the same file.

## Doubts

- **TC** `<@` calls in `inline_text` go past Templates ch. 2, which gives the function form
  no call form ("a nested template is a function called in a placeholder"); the task asked
  for `<@` calls. Jimmy rules which one stands; the emitter uses placeholders everywhere and
  no `<@` call yet, so dropping TC costs nothing.

- **PF** reads less like the books' recursive descent than a language with exceptions
  would; every step after a failure still runs, as a no-op. If Prism gains early exit from a
  step sequence (a `given:` that can carry a payload out), the bad cursor goes.
- The front end matches `prism_syntax`, not the books directly: a disagreement between
  `prism_syntax` and a book (its API.md lists them) is reproduced here.
