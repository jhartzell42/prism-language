# `prismc`: the front end, in Prism

*Written by Claude, 31 October 2026, on the `claude-self-hosting` branch. Work in progress
and subject to change: Jimmy rewrites it by hand, and an implementer reading it uses their
own discretion. The books in `prism-notes/claude/prism/` are a curated set of design prompts, not a specification;
`prism_syntax` is the oracle this front end is diffed against, and where the two disagree
the book decides which one has the bug.*

The lexer, parser and tree of Prism, written in Prism and run by compiler zero
(`prism_zero`). It gives the same tree as `prism_syntax`, node for node, and prints it in
`prism_syntax`'s S-expression notation so the two can be diffed.

## Files

prismc is a tree of nested modules (claude/modules-style-pass.md): each module one file,
`name.prism`, or a directory whose `mod.prism` is the module when it has children; each
directory's README lists its modules.

- `app.prism` (the root): the driver, `@main`, `@FrontMain`; its child modules
  `reading.prism` (`Reading`: files and trees read, parsed: `Assemble`, `ParseProgram`),
  `checking.prism` (`Checking`: `--check`, `--typed`) and `compile.prism` (`Compile`).
- `fs.prism` (`Fs`): the prelude's `Fs` with `Fs.List` (pick FL).
- `syntax/` (`Syntax`): the tree, `prism_syntax/src/ast.rs` one to one, with a `Range` on
  every node; `Diag`, the diagnostic; `Syntax.Decls`. `syntax/lexer/` (`Syntax.Lexer` and
  its children `Chars`, `Layout`, `Words`, `Literals`, `Templates`, `Corpus`): characters
  to tokens with layout (Semantics ch. 2, the ten layout rules, the template frame;
  Conformance A.1 and A.2), as an `&` block over the text (Q-L1 (b)); `RenderTokens`, the
  layout corpus's notation; Rust's `{:?}` escaping. `syntax/parser/` (`Syntax.Parser`
  and its children `Decls`, `Expr`, `Blocks`, `Types`, `TestLanguage`): `unbounded`
  recursive descent over a token cursor (D7), with the pairing rule, a Pratt loop over
  the 14-level table, recovery at statement boundaries, out-of-scope diagnostics, and the
  test language. `syntax/sexp.prism` (`Syntax.Sexp`): the S-expression printer,
  `prism_syntax/src/sexp.rs` exactly. The lexer and the parser each have a test module,
  `Tests` (`tests.prism`).
- `check/` (`Check`, `CheckProgram`), `check/errors.prism` and the directories `types/`,
  `env/`, `expr/`, `blocks/`, `passes/`, `shrink/`, `typed/`, each a module with child
  modules: the checker (Checker, below).
- `lower/` (`Lower` and its children `Types`, `Expr`, `Patterns`, `Templates`,
  `Functions`, `Jobs`, `Program`), `lower/model.prism` (`Lower.Model`, the core form) and
  `lower/node/` (`Lower.Node`, widget blocks as nodes): lowering (Back end, below).
- `emit/` (`Emit`, `mod` lines only): `emit/rust/` (`Emit.Rust`, `Emit.Rust.Items`) with
  `templates/` and `prelude.prism` (the Rust templates and the emitted prelude),
  `emit/cpp/` (`Emit.Cpp` and its children) with the same two for C++.

## Running it

```
prismc FILE [OUT]              # the tree in S-expression notation
prismc --tokens FILE [OUT]     # the tokens, in the layout corpus's notation
prismc --fragment FILE [OUT]   # a run of statements
```

`prismc` is the newest native binary, as the `prism` script picks it (Back end, below).

`@main` reads FILE through `Fs.Contents`, runs the front end through `@Run`, and writes
exactly what `cargo run -p prism_syntax --example dump -- FILE` prints (the dump, then the
diagnostics, then a newline) through `Fs.Output` to OUT, `FILE.sexp` when OUT is left out.
When there is a lexical or parse error the file is written all the same, and the program
then gives up through `Prelude.Unwrap` with the rendered diagnostics as its message, which
the CLI World prints to standard error before it exits 1. Out-of-scope features are
reported in the output and do not fail the run.

The lexer's and the parser's checks are their modules' test modules
(`Syntax.Lexer.Tests` and `Syntax.Parser.Tests`, claude/modules-style-pass.md),
`check`s on small inputs over the helpers declared beside them; they run compiled with
the modules they reach, transcripts byte-equal to the `tests.expected` beside each:

```
tests/compiled/run.sh lexer parser
./prism test prismc/syntax/lexer
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
- **CH Texts are walked with cursors** (Semantics Appendix B, Text; round 4 TX). The lexer
  holds its source as one `Text` and its position as a byte offset; `Peek` makes a
  cursor there with `?CursorAt` and reads the character as a one-character `Text`, a view
  of the source (`CharAt`: `Before` of one `?Step` into `After`). A character is a
  one-character `Text` throughout the front end, so the classifiers compare texts; a
  `Char` literal holds a one-character `Text`. Every offset (a `Range`, a line start) is
  a byte; the diagnostics count their columns in characters, by walking the line
  (`ColumnOf`), so `line:col` is what `prism_syntax` prints. The text helpers
  (`lexer.prism`: `Head`, `AfterFirst`, `LastChar`, `BeforeLast`, `HasPrefix`,
  `DropPrefix`, `?BeforeSuffix`, `Between`, `FoldChars`, `MapChars`, `AllChars`,
  `TrimLeading`, `TrimTrailing`, `CountChars`) are cursor walks and cuts; nothing in
  prismc indexes or takes from a text.
  - **What it costs** (`/usr/bin/time -l <prismc> compile
    tests/conformance/sem-ch07-normalize.prism --prelude prelude -o target/norm --name
    norm`, M2 Pro, other jobs running), over a rope of characters (`ropey` in Rust, a
    `std::u32string` in C++) and now:

    | prismc built by | instructions, characters | instructions, cursors | peak memory, characters | peak memory, cursors |
    |---|---|---|---|---|
    | Rust (cargo release) | 4.79 G to 4.82 G | 1.40 G to 1.43 G | 43 MB | 33 MB |
    | C++ (-O1) | 1.88 G to 1.94 G | 1.80 G to 1.86 G | 33 MB | 38 MB |

    The C++ "characters" binary is the one the main checkout had installed on 4 November
    (its compiler not recorded); the "cursors" one is clang++ -O1. In the C++ prismc the
    lexer was never the cost (malloc and reference counting are, handoff item 4).
  - **Doubts for Jimmy:**
    - **TXC** A `Cursor` is `^Eq`, `^Ord` and `^Show` like every primitive; it compares
      by its byte index alone (the book: "Cursors compare by it"), so cursors into two
      texts at one index are equal. It shows, and leaves as plain data for a `check`
      line, as `Cursor(at=N)`. The book is silent on both.
    - **TXS** Only `?Step` shrinks in the shrink check (Semantics ch. 7 names it); a loop
      over `?StepBack` takes fuel (`TrimTrailing`). Counting `?StepBack` too would let a
      loop that steps both ways pass.
    - **TXP** `Split` (not in Appendix B) stays, for a non-empty separator; an empty one
      gives up (a panic), since it was the way to a text's characters.
    - **TXK** `Take` and `Drop` are `fn [a], Nat -> [a]`, so `Take` on a text is a
      `mismatch` at its argument (and its result), with no fix line of its own; `t[i]` on
      a text has its own message and fix.
    - **TXU** Invalid UTF-8 is never repaired. A `Text` is made from bytes only at a
      World's boundary; in `prism_rtl` a `Text` from bytes that are not UTF-8 is a fatal
      error naming the byte (C++: a `Panic` from the constructor; Rust: a `Text` is made
      only from a `str`, and `std::env::args` itself panics on such an argument). The CLI
      World checks a file's bytes first and answers `Fs.Contents` and `Fs.File` with an
      `Error` of kind `Fs.NotText` (`prelude/fs.prism`), message `PATH: not UTF-8: byte N
      is malformed`, in both Worlds (their tests
      `a_file_that_is_not_utf8_reads_as_fs_not_text`).
  - **Gaps:**
    - **PL A parenthesized lambda argument loses its types.** `MapChars t, (\|c| F c)`
      checks, but the lambda's parameter reaches lowering with no type (`p0: ()`, "a
      type the checker did not find") and rustc refuses the crate. Written without the
      parentheses it lowers; prismc does so. Not fixed.
    - **CI Interpolating a `Char` shows it quoted.** `"{c}"` with `c: Char` places `'a'`,
      since a placeholder places a `Text` or a number as it is and anything else as it
      shows; no prelude function makes a `Text` of a `Char` (gap UC). Chess's text input
      (`Key`'s `Char Char`) therefore inserts `'a'` for a typed `a`; its caret moves by the
      inserted text's bytes.
- **UB What is `unbounded`.** Every function that recurses through the grammar or loops
  with no measure a checker could see (the lexer's loops over the text, the parser's
  descent, `Recover`); predicates and cursor primitives are bounded. The closure-taking
  helpers (`LayoutItems`, `Commas`, `Bracketed`) take their closure's color by default
  (pick CD).
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

- **IB6** (fixed): template attributes bind by parameter name in compiler zero too.
  left={{a}}/>` passes `b` as `left`; the checker reads attributes as named arguments, and
  prismc's lowering binds each to the parameter of its name
  (`tests/conformance/ts-ch05-template-attrs`, whose `.expected` is written by hand).

## Checker

Type System ch. 5 as a program: the environment of every file's declarations, inference
under each signature with the marks read first (DB1c, D12), rows with families as the
unit, traits with coherence, lifting decided per application, the color, then the
separate passes in the book's order. It reports ch. 5's errors by their names, each with
`file:line:`, the line, carets and the fix.

```
prismc --check FILES... [--out OUT] [--world Ui,Fs] [--prelude DIR]
prismc --typed FILES... [--out OUT]
./prism check [--world Ui,Fs] DIR|FILES...      # the report on standard output
tests/checker/run.sh [--update] [NAMES...]
```

`--check` writes the diagnostics, then `no errors` or `N errors`, to OUT, and gives up
(exit 1) when there is an error. The prelude's files from `--prelude DIR` join the program
unless it replaces one (a module of the same name, or a tag of the file's family: the chess
window library declares its own `Ui` tags, so `ui.prism` stays out); diagnostics are shown
for the program's own files only. `--world` names the families a World answers, for the
root row check. `--typed` writes the typed program (below), then the diagnostics as `;`
lines.

### Files

- `check/errors.prism` (`Check.Errors`): a diagnostic, a source file, rendering, sorting.
- `check/types/` (`Check.Types`): the three layers of a type (plain, stack, color),
  the inference state, unification with rows (Rémy's algorithm with families for labels,
  lacks sets) and colors, schemes, the color's joins and their solution, and the typed
  output's recorders.
- `check/env/` (`Check.Env`): modules, bare-name tables (a collision is an error at
  the use), types and kinds, tags, traits, impls, coherence, signatures read as schemes
  (a color variable per function parameter, the signature's color their join, pick CD), trait
  resolution with defaulting, waiting field accesses.
- `check/expr/` (`Check.Expr`): expressions, the application rule, lifted `match` and
  `if`, patterns and binders, local `let`s and their generalization (DB1a, doubt MR),
  clauses under a signature.
- `check/blocks/` (`Check.Blocks`): widget blocks, holds, binds, `rebuild`, `for`,
  `given:`, `&` blocks, and binding groups' dependency order.
- `check/passes/` (`Check.Passes`): exhaustiveness and reachability (Maranget's
  usefulness), the prompt-cycle check, `&` borrows and escapes, inclusion and forwarding,
  the root row; `check/shrink/` (`Check.Shrink`), the shrink check (size-change graphs);
  `check/` (`Check`), `CheckProgram`, the whole checker.
- `typed.prism` (`Typed`): the typed program and its S-expression printer.

### The typed program

`CheckProgram` with recording on gives a `Typed.TProgram`, for the back end: the tree as
parsed, unchanged, with tables keyed by where each node stands (`At`: file and range),
every type followed to its end once the program's last defaults are in.

- `exprs: [TyNote]`: the type of every expression (callees included, as function types; a
  tag or constructor as the function of its fields).
- `apps: [AppNote]`: the lifting decision at every application, field access, text and
  lifted `match` or `if`: the node kind (`plain`, `const`, `map`, `sample`, `gate`), the
  result's time layer, the arguments peeled by position, the `?` or `~` peeled under it,
  the call's color.
- `fns: [FnNote]`: the color of every function, lambda and widget: declared, and the join
  of what its body applies. A lambda's two are its color as solved (below).
- `methods: [MethodNote]` and `impls: [ImplNote]`: every method call with its trait, and how
  every constraint at that place was solved (`impl FILE:LINE`, `derived FILE:LINE`,
  `builtin`, `given`, `parts`, `packed`, `quantified`).
- `groups: [GroupNote]`: every widget block's and expression block's statements in
  strongly connected components, dependencies first, ties in page order.
- `binders: [BinderNote]` and `defs: [DefNote]`: every binder's type and every top-level
  definition's scheme.
- `gens: [GenNote]` and `insts: [InstNote]`: every generalized local `let` (its binder and
  the variables its scheme quantified, `'12` and `'c14`) and every use of one (the name's
  place and what each of those variables is there). The binder's type and its value's
  types have the variables as `(var t12)` and `(color 'c14)`; lowering instantiates the
  value at each use (pick LG).

The dump walks the tree, so an expression the checker never annotated prints `unknown`;
run.sh checks that chess's and the conformance counter's dumps have none. A variable left
in a dump (`(var t12)`) is one nothing decides and nothing constrains (a supplier's `o`,
an empty array's element), or one a local `let` quantified (`gens` names those).

### Where it stands

`tests/checker/run.sh` checks every program of the corpus, the programs of its own
under `pos/` (`lambda-color`, clean, with its colors in `pos/lambda-color.colors`), the
88 negative cases and the two typed dumps. Every corpus program checks clean; a book program the checker refused
was a slip in the book, corrected there and here (Semantics round 3 CK).

- **Clean:** `compiler-ch03-halfcounter`, `conf-ch01-counter`, `conf-ch01-journal`,
  `conf-ch01-motd`, `counter`, `half-counter`, `todos`, `sem-appB-widgets`,
  `sem-ch01-counter`, `sem-ch04-birth`, `sem-ch04-given`, `sem-ch04-labels`,
  `sem-ch04-quiz`, `sem-ch05-and-blocks`, `sem-ch06-birds`, `sem-ch07-births`,
  `sem-ch07-normalize`, `sem-ch07-paged`, `sem-ch07-rounds`, `sem-ch08-viewer`,
  `sem-ch09-hosts`, `ts-ch03-traits`, `ts-ch04-lifting`, `prelude`, `chess`.
- **`prismc`** (clean): the checker over prismc's own tree (`prismc/`, its test modules
  read), the back end included.

The negative cases (`neg/`, 88) give one diagnostic each, the error their name says, or a
case of it (`live-field-bare` is a `mark-stack`; `values-not-shrinking` and
`summary-not-parts` are `may-not-end`s that keep doubt TR's map rule and pick SM's
summaries from accepting a cycle that grows; `drop-unknown` and `long-cycle` are
`may-not-end`s for pick SM's `Drop` rule and the closure's fixed point, `int-minus`,
`own-minus` and `min-tree` for pick SW's `Nat` rule, `type-arrow` for pick NT, `widget-cycle`,
`let-cycle`, `local-recursion` and `local-widget` for picks TD and LF, `prelude-module` a
`may-not-end` and `freeze-resolved` a `frozen` for pick RN, `prompt-let-lambda` a
`prompt-cycle` for pick PS; the five `*-rebinding` cases are
`may-not-end`s that keep pick SC's scoping honest; `guard-recursion`, `method-recursion`,
`template-recursion`, `nested-set` and `fuel-unrelated` are `may-not-end`s for picks GH and
PM, `operator-recursion` and `op-literal` for pick OP, `fuel-call` and `literal-set` for
pick SW, `local-shadow` for pick RN, `closure-call` and `closure-escape` for pick CT,
`guard-prompt-cycle` and `prompt-lambda` `prompt-cycle`s for picks GH and PS,
`template-unknown`, `template-attribute` and `template-type` an `unknown-name` and two
`mismatch`es for pick TK, each with a function or a block beside it that checks clean).
`impl-failed` gives two, a failed impl's own error and another impl's cycle, for pick FK. `refinement-escape` and `existential-escape` have no case, since the checker does
not report them (gaps below).

### Checker picks

The places where the checker had to choose. Each is Claude's, for Jimmy to overturn.

- **TO The typed program is tables beside the tree.** Every annotation is keyed by file
  and range, and the tree stays `Ast.Module`. *Refused:* a second tree with a type on
  every node, which doubles ast.prism's forty expression forms and has to follow every
  change to them.
- **BG Check order in a block.** Holds first (a hold's start value is where a block most
  often says what its names are), then `let`s and local functions, then the rest in page
  order, every name pre-bound to its marked type. The dependency order is computed for the
  typed program, not used for checking (gap DG).
- **FD A field of a record nothing has decided yet waits.** When several records have the
  field, its type is a fresh variable, read once a later line decides the record, and
  `ambiguous` at the end of the definition (ch. 5's principle: defer what a later line can
  decide). One record with the field decides it at once.
- **MS A wrong mark reports once, at the binder,** and the name then has the type its
  value has, so its uses do not report it again.
- **CR A collision recovers** with the file's own declaration, else the first candidate,
  after reporting both, so the rest of the line still checks.
- **LU `layer-unknown` is asked at the end of the definition.** A free-headed argument
  meeting a plain or variable parameter is taken whole there; at the end, if its layer is
  still a variable, or turned out to be a dynamic or an event, it is `layer-unknown` at
  the argument. *Refused for now:* deferring the application and retrying it (ch. 5's
  text), which needs the environment kept with each deferral (gap LR).
- **ER** An arm that is a bare `Error` waits for the other arms, so `if c: Error(..) else:
  ~ unit` is a `~unit`.
- **QD** The prelude's `/: fn a, a -> ?a` is read as Type System ch. 3 has it, `fn a, a ->
  Quotient a`, with `Quotient` normalized for the built-in numbers only.
- **TL0** Top-level `let`s without a signature are inferred first, in page order, and their
  monomorphic variables default at the program's end.
- **PS** In the prompt-cycle check a widget function is prompt in every argument, except
  the prelude's delayed ones (`@Defer`, `@Later`, `@Run`, `@Machine`, `@started`, by
  resolution, pick RN). A right side's reads are pick GH's walk, less what is not read
  where it is written: a lambda, a local `fn` or widget, a nested widget block, a
  `rebuild`'s or a `for`'s source and holds (building a row is never an edge, R7), and a
  delayed binding's arguments. A `for`'s or a `rebuild`'s result is prompt in its rows'
  other lines (Semantics ch. 7, a `for`'s consolidation), the checker not knowing which
  dynamics are structural: a row reading its own `for`'s gathered result is a cycle, as a
  host's child reading `$doc` is (`neg/sem-ch07-for-own-gather`). A lambda passed to a call is
  read, since the callee may call it now: a traversal's step, and a lambda given to any
  function but a prelude widget function (a supplier builds a blueprint, which is
  construction). `neg/prompt-lambda`'s `let a = Apply 1, \|x| x + b` reads `b`. A name a
  `let` binds to a lambda is read by following the binding, where it is passed (to any
  function but a prelude widget function) or called: `neg/prompt-let-lambda`'s `let f =
  \|x| x + b`, then `let a = Apply 1, f`, reads `b` (clean before). The lambda's body is
  read with the relations of the block the check is over, its parameters bound, each
  lambda once along a path. A name bound again inside the right side (a lambda's
  parameter, an arm's binder, an inner `let`) is not the block's.
- **SM A function that returns parts of a parameter has a summary**, Semantics ch. 7's
  size-change principle across the call, so a cycle may traverse what a helper returns.
  A function is summarized on its first parameter `i` for which every clause's result is
  within `i`:
  - no larger than it: the parameter, a part, `Values` or `Keys` of one, an index on one,
    `Drop` of one (below);
  - an array or map literal of parts;
  - a `match` or `if` whose every arm is within `i`;
  - a `Filter`, or an `Each` of parts, over a collection within `i`;
  - a `Fold` over one, from a start within `i`, whose step returns its accumulator or the
    accumulator with parts added by `Push` or `Insert`, or a within added by `Append`;
  - an `update` of a start within `i` whose every set of `&_`, at any depth, is no larger
    than `&_`, with no `+=` to it and no set of a field of it (Semantics ch. 5's
    `SkipWord`, `&_ <= next` with `next` from `?Step &_`).

  A part is strictly smaller than `i`, or a tuple of such (a tuple is not a recursive type,
  so a cycle reaches only its pieces). `Drop xs, k` (a positive literal `k`) is no larger
  than `xs`, and strictly smaller only where `xs` is known non-empty: in the arm, or the
  `if`, `while` or `given:` bind, of a refutable pattern that took an element of it (the
  `? x` of `xs[i]`, an array pattern of one element or more on `xs`), until something
  binds `xs` again. `Drop` of `[[]]` is `[[]]`, so `fn Loop xs = Loop (Drop xs, 1)` never
  ends (`neg/drop-unknown`, clean before); `(Len xs) > 0` shows nothing, since Semantics
  ch. 7's smaller array is a tail "taken apart by `match`". The `+` non-empty type would
  show it too, and the checker refuses non-empty arrays (`unsupported`), so no program
  reaches that case yet. `neg/local-shadow`'s `Shrink` and `neg/fuel-call`'s `Drain` now
  take an element first. A call of the function is no larger than its
  argument `i`, and strictly smaller when that argument is. A summary is read without
  other summaries, so none depends on itself or on a chain, and a name the calling clause
  binds itself (a parameter, a pattern, a local `fn`) is not the summarized function
  (`neg/summary-not-parts`: a function returning `[[Node m]]` gets no summary). A call reads
  the same in either spelling, `Kids cs` or `cs.Kids`. Chess's `Kids cs` is summarized on
  `cs`, which clears `LayStack`, `LayLayers` and `LayGrid`; `sem-ch05-and-blocks`'s
  `SkipWord c` is summarized on `c`, which clears `Idents`' `&c <= SkipWord next` under
  pick SW.
  *Refused:* inlining `Kids` at its call sites, which bends the program to the checker;
  summaries chained to a fixed point, which no program needs yet.

  The size-change closure itself runs to its fixed point: each round composes the graphs
  the last round found with the call graphs, until a round finds none. It ends because a
  graph is a caller, a callee and a set of edges between their parameters, finitely many.
  A cap of twelve rounds stopped before a ring of sixteen functions closed, and accepted
  it (`neg/long-cycle`, clean before).

- **SC Size relations are scoped.** This clause loops forever on a `Leaf`:

  ```prism
  fn Grow t = match t:
      Node l, r: Grow l
      Leaf n:
          let l = Node t, t
          Grow l
  ```

  A check that keeps one table of relations for the whole clause clears it: the `Node` arm
  says `l` is smaller than `t`, and that sentence leaks into the `Leaf` arm, where `l` is a
  different, bigger tree. So every call, every summary's result and every `while`'s set is
  read with the relations in scope where it stands:
  - a name bound again (a `let`, a pattern, a lambda's or a `for`'s parameter, a local
    `fn`) relates to nothing but what its new definition relates to;
  - a `match` arm's bindings, and an `if`'s `? x <- e` binding, are seen only in that arm;
  - a block is one binding group (pick BG pre-binds its names), so its names hide the
    outer ones on every line of it, and its `let`s and binds relate in page order.

  `neg/arm-rebinding` (the clause above), `neg/let-rebinding` (a `let` after a shrinking
  `match`), `neg/summary-rebinding` (a summary, pick SM), `neg/loop-rebinding` (a
  `while`) and `neg/field-rebinding` (`&s <= &s.tail` after a `let &s` in the loop's body)
  each checked clean before; `arm-rebinding`'s `Total (Node l, r) = (Total l) +
  (Total r)` is per-arm shrinking, and still checks clean. *Refused:* one table where a name
  bound twice keeps the weaker relation, which works only when both bindings relate to the
  same parameter and says nothing when the second relates to nothing; renaming every
  binder apart before the check, which is the same scoping as a separate pass over the
  tree.
- **GH One walk finds every call.** This function never ends, and its only call is in a
  guard:

  ```prism
  fn Spin n = match n:
      k if Spin k: True
      otherwise: False
  ```

  A collector that lists the forms it visits misses the ones it forgets: `Spin` got a
  reference edge from the names it mentions and no size-change graph from its call sites,
  so its cycle had nothing to fail. `n.Max (Spin n)`, a call inside a method call's
  argument, went the same way. So the shrink check reads a body through one walk
  (`ExprSpots`, `StmtSpots`): every expression and statement at any depth, each with the
  relations in scope where it stands:
  - a guard sees its arm's pattern, a `? x <- e if g` guard its condition's;
  - `recv.Method args` is the call `Method recv, args`, as inference reads it;
  - `a + b` is the call of the operator's trait method (pick OP);
  - `<@Item depth={{d}}/>` in a template is the call `Item`, its attributes named arguments
    and its body a text after them, as lowering passes them;
  - a named argument fills the callee's parameter of its name, not of its place: the one
    every clause of the callee that names it gives the same place, and none when no clause
    names it or two disagree;
  - text and template placeholders, `if`, `while` and `given:` blocks, `require`,
    `forbid`, `return`, `update` statements, lambda bodies, local `fn`s and widgets are all
    in it.

  The call sites, the reference graph's names, the summaries' seeds and the names a body
  binds, the `while`s the loop check visits and the sets it reads (pick PM), and the
  prompt-cycle check's reads (pick PS) all read this one list (`CheckBlocks`' dependency
  order reads its names too). `neg/guard-recursion`, `neg/method-recursion`,
  `neg/template-recursion` and `neg/guard-prompt-cycle` each checked clean before.
  *Refused:* adding the forms to each collector, which leaves six walks to drift apart
  again.
- **PM A loop's measure is the local it sets.** Each local a `while` sets is its own
  measure, and a set shrinks it only when the value is strictly smaller than that local.
  `&fuel <= less`, with `less` the `? less` of `&other -? 1`, is smaller than `&other`, not
  than `&fuel`; when the loop puts `&other` back every turn, it never ends
  (`neg/fuel-unrelated`, clean before). A path through the body is its statements and an
  `if`'s or a `match`'s arms; a set anywhere else (a nested loop, a `for`, a nested
  `update`, a `let` value, a lambda) is on no path for sure, so it may only keep the local
  strictly smaller, and a `+=` never does (`neg/nested-set`: an inner loop's `&fuel += 2`
  undoes the outer loop's `&fuel <= less`, clean before). A set through a field
  (`&v.kids <= ...`) is a set of `v`. *Refused for now:* a measure over several locals
  together (`&fuel` smaller than `&other` while `&other` never grows), which no program
  needs yet.
- **OP An operator is a call of its trait method, at the impl the solver chose.** This
  never ends:

  ```prism
  impl ^Num Money:
      fn * a, b = Times a, b

  fn Times a, b = a * b
  ```

  `a * b` is `Prelude.*` at Money, and the impl's `*` is a definition like any function, so
  the shrink check takes impls' methods and traits' default methods as definitions, and a
  trait method's name, an operator's spelling included, calls them
  (`neg/operator-recursion`).
  Which of them a call reaches is the solver's choice at that call. The solver notes how it
  answered every constraint (`St.choices`, by file, range and trait, recorded whether or not
  the typed program is): a written impl by where it stands, a derived impl by its type,
  `any` for a type variable under a bound, a packed value or a quantified `let`, `none` for
  a built-in type or a former's parts (each part is a constraint noted at the same place).
  So a method call or an operator reaches:
  - a written impl's definition of the method, or the trait's default when the impl has
    none;
  - through a derived impl, what its fields' types choose, at any depth (a type
    parameter's choice is the call's own constraint on it, at the same place);
  - every definition of the method, for `any` and for a call the solver noted nothing at;
  - none of the program's, for `none`.

  `neg/op-literal`'s `Times a, b = 2 * a` is `*` at `Money`, since the literal is a
  `Money`, so Money's `*` calls itself through `Times`. Chess's `fn Show s = Show (SqOf s)`
  reaches `Sq`'s impl, not `Spot`'s own, and an impl's `==` on a field (`a.key == b.key`)
  reaches the field type's. Two impls' methods share a definition key (`Prelude.Show impl
  Show`), so the check tells them apart by where each impl stands (`OwnKey`).
  *Refused:* guessing the type from how the argument is written (a literal, a
  constructor, a signature's result), which took `2` for a built-in number; the typed
  program's `ImplNote`s, which exist only when it is recorded and name an impl by its line.
- **RN Names are resolved where they stand.** This never ends:

  ```prism
  fn Grow xs:
      fn Drop ys, k = Push ys, k
      if (Len xs) == 0: 0 else: Grow (Drop xs, 1)
  ```

  A check matching the text `Drop` reads `Drop xs, 1` as the prelude's, strictly smaller
  than `xs` (`neg/local-shadow`). So the walk's relations mark every name bound where it
  stands as a local: a parameter, a `let`, a pattern, a lambda's or a `for`'s binder, a
  local `fn` or widget. A call of a local is never a global's call, summary or traversal.
  The prelude's names the checks read for their meaning (`Drop`, `Take`, `Filter`,
  `Remove`, `Min`, `Values`, `Keys`, `?Step`, `Fold`, `Each`, `Push`, `Insert`, `Append`,
  `Freeze`, `-?`, the delayed widgets) mean it only where the name resolves to the
  prelude's declaration and no local of that name stands (`PreludeSeeds`, per module).
  The prelude is known by identity, not by name: a declaration is the prelude's when
  module `Prelude` declares it in one of the prelude's own files, the ones `--prelude`
  adds after the program's (`G.preludeFrom`). A program that is its own `module Prelude`
  replaces the prelude's file, and its `Drop` means nothing special
  (`neg/prelude-module`, clean before). The reference graph, the summaries, the
  traversals, the loop check, the prompt walk and the `&` blocks' checks all read names
  this way: `Freeze` is the prelude's in either spelling, `Prelude.Freeze &b` included,
  and a local `fn Freeze` freezes nothing (`neg/freeze-resolved`: the first was missed and
  the second reported before); `Buffer.Numbered`, `Buffer.Empty` and `Table.Empty` are the
  checker's builtins by resolution. A `Freeze` anywhere in a statement freezes, and a name
  anywhere in a later statement reads, through the walk (pick GH). *Refused:* a separate
  renaming pass, the same scoping as a second walk.
- **FK A failed impl's method is its impl's alone.** The definitions that fail inference
  are skipped by the separate checks, and the list keys each by where its impl stands, as
  the shrink check does (pick OP): one key for every impl's `Show` dropped them all, and a
  cycle through another impl's went unchecked (`neg/impl-failed`).
- **TK A template call is a call.** `<@Name a={{x}}>body</@Name>` is the call of a function
  with its arguments put in place by name. The rule, which the checker
  (`CheckExpr.TemplateCallCheck`), the shrink check (`CheckPasses.ArgSlots`) and lowering
  must all read the same way:
  - **The callee** is `@Name` where it stands if that resolves, else `Name`. It must be a
    top-level function with clauses; its arity is its first clause's number of
    parameters.
  - **A parameter's name**, in one clause, is the pattern at that place, seen through
    parentheses, when it is a name: its spelling less its leading marks (`$text`, `?x`,
    `!e` and `&b` name `text`, `x`, `e` and `b`). Any other pattern (`(Leaf n)`, `ignore`,
    a tuple) names nothing there.
  - **An attribute** `a={{v}}` fills place `j` when `j` is the one place any clause names
    `a`: every clause that names `a` names it at `j`, and at least one does. Every clause
    is read, not only the first, so `fn Item (Leaf n), depth` and `fn Item t, depth` both
    name `depth` at place 1.
  - **The body**, when the call has one, is a text that fills the last place.
  - **Every place is filled once**, by an attribute or the body, and the call passes them
    in place order, whatever order the attributes are written in.
  - The result is a text, and the application rule checks each argument against the
    signature.

  A name nothing declares is `unknown-name` (`neg/template-unknown`); an attribute that
  fills no place (`neg/template-attribute`) or two, one given twice, a place left out, a
  body whose last place an attribute took or that has no place, and a callee without
  clauses are `mismatch`es, as is an argument of the wrong type (`neg/template-type`).
  The shrink check reads the body as the argument for the last place too (it read it as
  the first positional one). Lowering (`Lower.TemplateArgs`) and compiler zero read it
  the same way: each place's names from every clause, through parentheses and without
  marks, the body in the last place, attributes in theirs, never the written order
  (`ts-ch05-template-body`).
- **CT The call test for closures** (Semantics ch. 7, "Finding the cycles"). A closure
  passed to a program function is cleared when the function only calls it: in every
  clause, the parameter at that slot is the callee of calls (at any depth, but not inside
  a closure that may run later: a lambda other than a cleared one or a traversal's step,
  a local `fn`, a blueprint) or is passed whole to another cleared slot, and nothing else.
  The closure then runs only inside that call, so its calls are read where it is passed,
  with what the function's summary says it is given: for each of the closure's parameters,
  the function's parameter it is no larger than, or strictly smaller than, at every call
  of the closure.

  ```prism
  fn AddUp ts, F = Fold ts, 0, \|acc, t| acc + (F t)
  fn Total (Node ts) = AddUp ts, Total
  fn Spin t = AddUp [[t]], Spin
  ```

  `AddUp` calls `F` on parts of `ts`, so `AddUp ts, Total` is a call `Total` on a part of
  `ts`, and `ts` is a part of the `Node`: `Total` checks clean, and so does a lambda
  calling it (`neg/closure-call`), as `Each ts, Total` does (doubt TR). `Spin` passes
  `[[t]]`, whose element is `t` itself, and never ends. A closure at any other place is a
  call with no edges where it is written (the reference graph), so a function that keeps
  `F` in a lambda it returns (`neg/closure-escape`'s `Later`) clears nothing. The cleared
  slots and the summaries are greatest fixed points, each round read under the last
  round's: a slot stays cleared while what it passes on stays cleared, sound by induction
  on how deep the call that runs the closure is nested; summaries that never settle are
  dropped. *Refused:* tracking closures through `let`s, records and results, which needs
  flow analysis; a lambda's captured names stay related to the caller's parameters, its
  own parameters to nothing but the summary.
- **NT A type left of its own arrow is a cycle** (Semantics ch. 7, "A type that appears
  left of a function arrow in its own definition is a cycle"). This never ends, and no
  call graph shows it:

  ```prism
  type R = choice:
      Wrap (fn R -> Nat)

  fn Unwrap (Wrap F), r = F r
  fn Omega r = Unwrap r, r
  ```

  `Omega (Wrap Omega)` calls `Omega` through the function the `R` holds. The check reads
  the rule as the book states it: a program's type is `may-not-end` at its definition when
  it stands anywhere inside a parameter of a function type of its own definition, at any
  depth (a double left, `fn (fn R -> Nat) -> Nat`, counts too), and every other type the
  definition names is read through its definition with the arguments in, so `Q (Box R)`
  with `type Box a = B (fn a -> Nat)`, and two types that hold each other, are found. It
  is cleared when every arrow it stands left of is `unbounded fn`: a call through one is
  an unbounded call, and the color spreads (ch. 7, "Unbounded is a function color").
  `neg/type-arrow` (clean before) has a stream type right of its arrow and an `unbounded`
  knot, both clean. *Claude's:* the error at the type, not at the calls through it,
  since the type is what the book calls the cycle. *Refused:* reporting the call through
  the stored function, which needs flow analysis to find the function's type at each
  call; strict positivity, which would refuse `R` right of an arrow left of another.
- **TD Top-level widget blocks and `let`s are definitions** of the shrink check, each one
  clause without parameters. A widget that binds itself builds itself forever (ch. 7, "a
  widget function that binds itself is held to the same test"), and a `let` that reaches
  itself through the functions it calls never has a value (`neg/widget-cycle`,
  `neg/let-cycle`, clean before). Widget functions were already in it.
- **LF A local `fn` or widget is a definition too.** A call of a local is never a
  global's (pick RN), so `fn Spin k = Spin k` inside `Grow` was an edge to nothing
  (`neg/local-recursion`, `neg/local-widget`, clean before). Each local, at any depth of a
  definition's body, is keyed by where it stands; its walk starts from the relations where
  it is written less what they say of names (a captured name is no parameter of it), and
  its parameters'. A name means the local where the local's block binds it and nothing
  binds it again. The local's own graphs are its calls of locals; its calls of globals
  stay where they were, read in place as the enclosing definition's, with its captured
  names related to the enclosing parameters (pick GH). A cycle among locals is cleared
  when the enclosing definition is `unbounded fn`. *Refused:* reading a local's calls of
  globals as its own, which relates a captured name to nothing and refuses every local
  helper that recurses into the enclosing function on a part.
- **SW A set shrinks through a call only when the callee returns no more than an argument
  that does.** `&fuel <= Bump less`, with `Bump n = n + 2`, grows `&fuel` every turn, and a
  rule that takes any call of a smaller first argument as smaller clears the loop
  (`neg/fuel-call`, clean before). A value set into `v` is strictly smaller than `v` when:
  - the relations in scope where the set stands say so (pick SC): a part of `v`, a field
    or an index of one, `Drop` of `v` by a positive literal where `v` is known non-empty
    (pick SM), a summarized call on a part;
  - it is a call that returns no more than an argument strictly smaller than `v`: a
    summarized function's parameter (pick SM), or the first argument of the prelude's
    `Take`, `Drop`, `Filter`, `Remove` and `Min` (`Min` only on `Nat`s, below);
  - it is `x -? k` of such an `x`, the prelude's `-?` on a `Nat` (below).

  The `? m` of `n -? 1` is smaller than `n` (Semantics ch. 7, "a smaller natural number")
  only when the name resolves to the prelude's `-?` where it stands and the solver made
  `n` a `Nat`: inference notes the operand type of every `-?` by file and range
  (`St.sizeTys`, kept whether or not the typed program is), and the shrink check reads it
  once the program's defaults are in. An `Int` counts down forever (`neg/int-minus`), and
  a program's own `-?` is whatever it says (`neg/own-minus`: Money's adds); both were clean
  before, when the rule read the spelling `-?` alone. `Min a, b` is no larger than `a` the
  same way, only where the solver made it a `Nat`'s: by `^Ord` it is the smaller of two
  values, which may be the bigger tree (`neg/min-tree`, clean before).

  A literal is never smaller: `0` is smaller only than a local known to be greater, which
  the checker cannot know, and `while True: &n <= 0` sets `0` forever
  (`neg/literal-set`).

  The prelude's five join the summaries as calls no larger than their first argument, for
  the reference graph's cycles too. `fuel-call`'s `Drain`, `&xs <= Evens (Drop &xs, 1)`
  under `while ? ignore <- &xs[0]:`, was `may-not-end` before and checks clean. *Refused:* a set's value smaller by its type
  (any call returning `Nat` on a smaller `Nat`), which is the rule that cleared `Bump`.

- **CV Colors are variables, solved by joins** (ch. 5, The color; doubts UC and CJ; D8).
  A function type's color is bounded, unbounded, rigid, quantified or flexible. A
  lambda's color is a fresh variable, and a call's is its callee's, its signature's
  variables fresh at each call (pick CD). Unification unifies the colors of two function types (bounded and
  unbounded do not unify; a variable takes either); the joins are not unification but
  owed orders `lo ≤ hi`: a body owes its color whatever it applies, and a function
  argument at a function parameter owes the parameter's color its own (the two types
  then unify with the parameter's color, so one call takes a bounded and an unbounded
  closure, doubt CJ). The joins are solved for the least colors: a variable above an
  unbounded color is unbounded at once, above a rigid one that one at the definition's
  end, and the end checks every join (`calls-unbounded`, `lifted-unbounded` for a lift
  whose color was still a variable) and defaults every variable nothing bound to bounded.
  A local `let` generalizes its colors like its type variables: two of its own joined
  become one, one joined with a variable outside stays
  monomorphic, one under a bounded or rigid color is that color. A block binds its
  `let`s ahead (for its dependency order); a plain name's `let` still checks at a variable
  of its own level and generalizes, the variable bound ahead taking its monomorphic type.
  So `let Run = \|K| \|y| K
  y` is `fn (u fn a -> b) -> u fn a -> b`, and `Run Grow` is unbounded where `Run \|y| y +
  1` is bounded in the same body (`pos/lambda-color`, `neg/lambda-color`). The typed
  output gives each lambda, call and type its solved color, a quantified one as `(color
  'cN)`. *Claude's:* least solution, as the body's join was. *Refused:* a lambda's color
  fixed when it is read, wrong for `\|y| K y` while `K` is a variable; unification alone,
  which refuses a bounded closure beside an unbounded one at one `u`.

- **CD Every function parameter has a color variable, and the function's color is their
  join** (ch. 5, The color; Semantics ch. 7; round 3 CD, Jimmy's). In a signature that
  takes a function and writes no `$`, `!` or `@`, each function parameter whose color is
  unwritten has a variable of its own (`u` for one, `u1`, `u2`, ... for several), and an
  unwritten color inside it is that one. The function's unwritten color is the join of
  its function parameters' colors, as if it called each: unbounded when one is written
  `unbounded`, bounded when none is a variable, the variable when there is one, and a
  variable of its own above each (the scheme's `cjoins`, `lo ≤ hi`) when there are
  several. The result's and the other parameters' unwritten colors are the function's.
  `bounded fn` writes a bounded function type (`neg/bounded-param`); a signature
  `bounded fn` has its unwritten parameters bounded. A written variable stays as written,
  to tie colors together. A call instantiates every variable fresh and owes each join, so
  its color is the join of its arguments' (`pos/param-color`: `Apply n, \|y| y + 1` is
  bounded, `Apply n, \|y| Grow y` unbounded; `neg/param-color`). In the body a
  parameter's rigid color counts as the signature's above it: calling `F` and `G` costs
  `Twice` its `u`, and a join between two parameters' colors holds when both are under it.
  So `Apply: fn Nat, (fn Nat -> Nat) -> Nat` is color-polymorphic as the prelude's `Fold`
  is, and the prelude writes no `u`. *Claude's:* the signature decides, so a caller's color
  and a cycle point count read off the signature. *Refused:* inferring from the body which
  parameters it calls, which made a caller's color depend on a body it cannot see.

### Checker doubts

- **TR** An element a prelude traversal (`Fold`, `Each`, `Filter`, by resolution, pick
  RN) hands its closure is strictly smaller than the collection, for the shrink check, and
  a named function passed as the step is a call on an element. Semantics ch. 7 lists "a part ... taken apart by
  `match`" and does not say this. *First ruling (Claude's):* as stated; prismc's tree
  walks need it. *If otherwise:* every tree walk through `Fold` is `unbounded`.
  - A map's children are its parts, as Semantics ch. 7 lists them ("a map's child"):
    `Values m` and `Keys m` are no larger than `m`, so a traversal over them hands out
    parts of `m`, and `m[k]` is no larger than `m`, so the `? e` of a match on it is a
    part (an index on an array likewise, its element). This clears chess's `TextsOf` ->
    `TextsIn` -> `TextsOf`, folding over `Values cs`. `Values` of a map built bigger than
    the one given (`Values (Insert m, k, (Node m))`) is related to nothing
    (`neg/values-not-shrinking`). *First ruling (Claude's):* as stated, the book's own
    list. *If otherwise:* chess's `TextsIn` and `Kids` are `unbounded fn`, which stops
    `PlaceAt` lifting over the window's dynamics.
- **RP** A reference path writes its field steps bare (`&_.rootModule <= ? m`), and a field
  after an index step names the element's field (`&_[ix].partner <= ? j` sets a `?Nat`).
  Type System ch. 1 says a field access shows its stack; the parser reads a set only
  through a bare path. *First ruling (Claude's):* bare steps in references. *If
  otherwise:* the parser learns `&_.?rootModule <= ...`.
- **CF** Conformance ch. 1 writes `!motd <- Fs.File(..)` for a `!~Contents`, Pitch ch. 8
  `!~motd`. *First ruling (Claude's):* Pitch's; Conformance ch. 1 is the slip.
- **VW** Semantics ch. 8's viewer writes `~Contents` with no `use Fs.*`, and the
  conformance copy declares its own `Contents`. *First ruling (Claude's):* the listing
  needs `use Fs.*`.
- **UI** Semantics ch. 8's `Ui` family names a `Key` without declaring one, and Appendix B
  gives `Click` no derives, so `^Show` of a click (`sem-appB-widgets`) has no impl.
  *First ruling (Claude's):* `prelude/ui.prism` gives `type Click = click` `derive ^Eq,
  ^Show` and declares its own `Key` (`Letter Text`, `Backspace`) beside `Ui.TextInput`;
  `sem-appB-widgets` checks clean. *If otherwise:* the tag takes `Gui.Key`, and the books
  say where `Click`'s `^Show` comes from.
- **WL An element has two faces.** A blueprint's element, `Element r o` (what an element
  expression or `emit` makes, and what a host's supplier receives live), keeps each
  field's stack: a pattern binds `text: $Text` as `$t` (Type System ch. 1, "Sigils on
  fields"), and a bare `text=t` there is `mark-stack` (`neg/live-field-bare`). An element
  seen through a document, `Elem r` (an entry of the `$(Path ==> Elem r)` a host form
  gives, sampled by a plain function), is that tick's snapshot, Semantics ch. 8's `Level`:
  a document is a value per tick, and the transcript prints its elements with plain
  fields (`Ui.Label(text="Count: 0")`). A pattern on it binds each `$` field's current
  value, plain. So `TextsOf` and `Lay` are plain functions, as Pitch ch. 5 says ("Layout
  is `PlaceAt`, a plain function from the tree"). The checker tells the faces apart by
  the type alone, `Elem r` against `Element r o` (Type System ch. 2 already calls `Elem r`
  the document's value type), with no marker added.
  - The binder may be bare (`text=t`) or show the field's declared `$` (`text=$t`), which
    the level drops: either name is a `Text`. Chess's `TextsOf` and `Lay` write `$t`, as
    Pitch ch. 5's `Masked` writes `held=$n` on an entry of `(Path ==> Elem)`. The `$t`
    spelling names a plain value with a `$`, against ch. 1's "the whole stack, every time".
  - `Masked`'s other half, building a document's element from levels (`Cards.Hand(held=$n,
    ..)` with `$n` now a `Nat`), does not check: a plain value where a `$` field goes is a
    mismatch, and an `Element` does not pack into an `Elem`.
  - An `!` field of a snapshot still binds as an event, though a `Level` drops events.
  - *First ruling (Claude's), for Jimmy to confirm:* the two faces as stated, both binder
    spellings on a snapshot. *If otherwise:* only `text=t` on a snapshot, and chess's
    patterns go back to bare names; or fields keep their stacks everywhere, and `TextsOf`
    and `Lay` are lifted functions, against Pitch ch. 5.

### Gaps against ch. 5

What the checker does not do yet, each with Claude's first ruling.

- **LR Deferred applications.** No application is retried when a later line decides an
  argument's layer (pick LU). *First ruling:* keep LU; retry when a program needs it.
- **RE `refinement-escape`.** A tag arm over a rigid output substitutes the arm's equality
  into the expected type, but outside variables are not made untouchable in the arm.
  A call's rank-2 variable (pick R2) reaching the call's result or a name of the scope
  is `refinement-escape` (`Escaped`), a check no test reaches: a name's marks fix its
  stack, so no outside name takes a free-headed output, and the rule confines the
  variable to its parameter. *First ruling:* OutsideIn(X)'s discipline as ch. 5 has it,
  with the error, next.
- **EE `existential-escape`.** `any e` solves its bounds at `e`'s type; an `any` meets only
  an `any`, and is never unpacked into a rigid variable at a parameter (doubt EP).
  *First ruling:* unpack at application, as ch. 5 says.
- **R2 Rank-2 element outputs** (Type System round 4 R2). An element's output variable
  that appears only inside one function-typed parameter of a signature is quantified at
  that parameter: at a call it is a fresh rigid variable (`RankTwo`, named `o'N`), so a
  lambda given there has each tag arm refine it (`ts-ch05-supplier-rank2`), and a named
  supplier's own `o` unifies with it (`pos/supplier-named`); one not polymorphic in it is
  a `mismatch` (`neg/supplier-not-polymorphic`). Only element outputs: `fn (fn Nat -> b)
  -> unit` still takes `\|n| Show n` (`pos/rank-one-callback`). Explicit type arguments
  are never rigid.
- **DG Binding groups.** Blocks are checked in BG's order, not dependency order, so a `let`
  used above its line is used at its monomorphic pre-bound type, and mutually dependent
  `let`s are not generalized together. *First ruling:* check in the dependency order the
  typed program already computes.
- **SU Prompt-cycle summaries** are per block, not computed bottom-up over the call graph
  (pick PS). *First ruling:* summaries, as ch. 5's pass 1 has them.
- **FM A field read through a waiting access** (pick FD) is not checked for its marks.
  *First ruling:* check them when the record is decided.
- **AT Associated types** other than `Quotient` of the built-in numbers do not normalize.
  *First ruling:* read each impl's associated type into the normalizer.

## Back end

`compile` checks the program, lowers it to a core form and emits a typed Rust crate over
`prism_rtl` through code templates. `claude/backend.md` is the contract: what Rust each
construct becomes and which library call it makes. The old back end, which emitted
`prism_rt` values and applied them through the dynamic layer (D16), is gone.

```
./prism compile DIR|FILES... -o OUT             # the crate only
./prism test DIR|FILES...                       # compile, build, run the .prismtest scripts
./prism run [--world cli|canvas] [--print-world] DIR|FILES... [-- ARGS...]   # compile, build, run the root
./prism check DIR|FILES...                      # the checker's report
prismc compile FILES|DIR... -o OUT [--name NAME] [--prelude DIR] [--runtime PATH]
OUT/target/.../NAME [--world cli|canvas|test] [ARGS...]      # the root, under the World its row asks for
OUT/target/.../NAME test [--print] [SCRIPTS.prismtest...]    # the Test World; compares with .expected
tests/compiled/run.sh [NAMES...]                # every program, byte-equal
```

`prismc` is `$PRISMC`, or the newest of `target/bootstrap/prismc` and
`target/keep/prismc-*` ("The bootstrap" below); the `prism` script, `tests/compiled/run.sh`
and `tests/checker/run.sh` use it. `prism test` builds debug, `prism run` release;
`prism run` runs the root under the World prismc took from the root module's `emit` row
(the crate's `const WORLD`): the canvas World when the row has `Gui.*`, the CLI World when
it has neither `Gui.*` nor `Ui.*`, and none, a refusal, when it has `Ui.*` without
`Gui.*`, since no World answers Ui elements alone; `--world` overrides, and
`--print-world` prints the choice and stops before the build (`tests/driver/run.sh`). The compiled binary takes `--world` only as its first argument; a later
one is the program's own (prismc's `--check --world Ui`). `make chess` is
`./prism run examples/chess`.

### Files

- `lower/` (`Lower` and its child modules), `lower/model.prism` (`Lower.Model`, the core
  form) and `lower/node/` (`Lower.Node` and its child modules, a widget block's node):
  lowering. It reads the checker's environment
  (`Check.Env.BuildG`: modules, types, constructors, tags, traits,
  impls, every definition's clauses) and the typed tables (`TProgram`, indexed by place)
  and decides everything: names, types, borrows and clones, each widget's node (its
  fields, event graph, heights, places, answers, hooks, rows and generations), each
  unbounded function's job.
- `emit/rust/` (`Emit.Rust`, `Emit.Rust.Items`): the Rust printer, the core form spelled; one function per
  construct, nothing decided (K9).
- `emit/rust/templates/` (`Emit.Rust.Templates`): every item and every
  multi-line expression as a function-form code template with doubled braces;
  placeholders alone on a line indent to their column; doc comments quote the Prism
  source through a `<@Quoted text={{..}}/>` call.
- `emit/rust/prelude.prism` (`Emit.Rust.Prelude`): the prelude in Rust,
  emitted as `src/prelude.rs` beside every program (pick PL).
- `fs.prism` (`Fs`): the prelude's `Fs` with `Fs.List`, which `compile DIR` reads (pick FL).
- `compile.prism` (`Compile`, a child module of the root): `@main`'s `compile` branch
  (`@CompileMain`, `CompileProgram`).

The emitted crate: `Cargo.toml` (`prism_rtl`, `prism_world_cli`, and
`prism_world_canvas_rtl` behind the `canvas` feature, on by default when the root emits
`Gui`), `src/main.rs` (the programs, the Test World's registrations and compiled `check`
blocks, `main`), `src/prelude.rs`, `src/program.rs` (the root module), and
`src/PATH/mod.rs` per other Prism module (pick MO).

### The model

Lowering's output, `Model` (lower.prism), is a small imperative program that names
operations and never spells them: a printer spells it for its target, `Emit` for Rust.
Text in a node is a name (a local, a field, a method, a function's or constructor's
qualified path, a label) or a literal's digits, never syntax.

- **Items:** `ModDef` per Prism module, holding `Item`s: `TyDef` (a record, choice,
  newtype or unit type), `TagDef` (an element tag's level), `RowDef` (a row kind's values),
  `FnDef` (a function, a top-level `let`'s getter), `JobDef` (a step machine), `NodeDef` (a
  widget block's node), `TraitDef`, `ImplDef`; `ProgDef` per program, `CheckDef` per
  compiled `check` block, `AnswerDef` for `crate::Answer`.
- **A node:** `NodeDef` has its fields, the field initializers in the order written (the
  ones that clone a local before the locals moved in: lowering orders them), its build's
  `let`s and steps, `MethDef` methods (a receiver `Recv`, a read point `CxArg`), its answer
  arms (a position pattern and the call), hooks, `ForDef` and `GenDef` builders (each
  place's path and a row's parent are expressions), its rows as child nodes.
- **Types, `OutTy`:** the value types (`OutNat`, `OutInt`, `OutText`, `OutCursor`, `OutOpt`, `OutRes`,
  `OutTuple`, `OutArray`, `OutMap`, `OutNamed` with arguments, `OutGen` a type parameter);
  functions (`OutFn` bounded, `OutJobFn` a `Start`, `OutJob`); the runtime's pieces
  (`OutHold`, `OutStamped`, `OutSnapshot`, `OutDefer`, `OutPort`, `OutWakes`, `OutEvent`,
  `OutDyn`, `OutLive`, `OutPod`); `OutQuantified`, a generalized `let`'s variable no
  instantiation gave a type (pick LG), which the printer refuses; memory (`OutRc` shared, `OutWeak` to a node, `OutCell`
  mutable inside a shared node, `OutRef` a borrow, `OutObj` a shared trait object such as
  a node, a row or a host, `OutSlice`, `OutCount` a machine count). The C++ runtime's
  README maps each to its C++ form.
- **Expressions, `Ex`:**
  - names: `XVar` a local, `XPath` a global item, `XSelf` the node, `XCx` the read point,
    `XConst` a constant of the node's (a height);
  - literals: `XUnit`, `XNone`, `XBool`, `XNat`, `XInt` (plain decimal digits, every
    one kept: lowering converts a radix literal and drops leading zeros), `XByte`,
    `XFloat`, `XIndex` (a machine integer), `XChar`, `XStr` (a string, not a `Text`: a
    message, a name), `XText` (a `Text`), `XFormat` (a `Text` with values),
    `XFormatStr` (a message with values; a local is named in it);
  - calls: `XCall` of a named function, `XApply` of a function value, `XMethod`, `XCtor`,
    `XRecord`, `XTuple`, `XVec`;
  - places: `XField`, `XPart` (a tuple's), `XRef`, `XRefMut`, `XDeref`, `XTarget` (what a
    `for &s` loop updates in place);
  - sharing: `XClone` (a copy of a shared value), `XWeak` (a weak reference to a node),
    `XUpgrade` (the node again if it lives);
  - control: `XBin`, `XUnary`, `XIf`, `XMatch`, `XChain` (a match through shared
    payloads), `XBlock`, `XLabeled` and `XBreak` (a labelled exit with a value),
    `XBreakLoop`, `XClosure` (move or not, typed patterns as parameters), `XTry` (pass a
    fault on), `XDecode` (a value from plain data), `XCast`, `XLater` (a callback the
    runtime runs after the tick's answers), `XIsCase` (whether a value is one of these
    constructors), `XRefusal` (what lowering cannot lower), `XUnreachable`.
- **Patterns, `Pa`:** `PaWild`, `PaBind`, `PaMut` (a binder changed in place), `PaCtor`,
  `PaUnit`, `PaRecord`, `PaTuple`, `PaChar`, `PaBool`, `PaIndex`, `PaConst` (a height),
  `PaSlice` with `PaRest` (a place's path under it), `PaTyped`.
- **Statements, `Do`:** `DoLet`, `DoLetElse`, `DoAssign`, `DoWhile`, `DoWhileLet`, `DoFor`,
  `DoExpr`, `DoIf`, `DoIfLet`, `DoMatch`, `DoReturn`, `DoScope` and `DoLine` (statements
  the Rust printer keeps on one line), `DoNote` (a comment).
- **Names the Rust printer spells as they are:** the paths of runtime and prelude
  functions in `XCall` (`Hold::new`, `Rc::new`, `prism_rtl::Dynamic::of`), method names
  (`get`, `set`, `borrow`), a `for` row's node path (`ForDef.rowNode`, with its type
  arguments for a host's rows). The C++ printer maps them to its runtime's names
  (`CallName`'s cases and `MethodForm` in `Emit.Cpp`).
- **Types where a printer needs them:** a `match`, an if-let chain and a labelled block
  carry their value's type (`XMatch`, `XChain`, `XLabeled`), and `XTyped` says the type
  of an expression whose type C++ cannot infer (an empty array or map, `nil`, a loop
  step's `Ok` and `Err`). The Rust printer reads neither.

### Where it stands

End to end (compile, `cargo build`, transcript byte-equal to its `.expected`), by
`tests/compiled/run.sh`, which runs every program every time: all 81 pass, chess's board included.

- **Pass:** `counter` and `half-counter`, which `prism_rtl`'s hand-written
  `tests/counter.rs` verifies against the same `.expected` files, and whose emitted node
  has its shape (fields `count: Hold<Int>` and the merge's two `Stamped` inputs,
  `inc`/`dec` recording and asking for the wake, `wake` deciding left first, `patch`
  pulling the label; the half counter's `increment` calling `delta` calling the hold's
  write, with no wake).
  `sem-ch07-normalize`, the step machine `prism_rtl`'s `tests/normalize.rs` writes by
  hand (`struct Normalize { t: Term }`, `impl Job<Term>`, `compute until idle: 3 ticks,
  1 to 3`); `sem-ch07-walk`, a recursion that is not a tail call, one frame per pending
  `+` (`5 ticks, 1 to 5`); and the back end's own tests, one per construct prismc's
  self-compile needed: `ts-ch05-let-fn-pair`, `sem-ch07-unbounded-value`, `sem-ch07-unbounded-lambda`,
  `sem-ch04-row-local`, `ts-ch05-template-attrs`, `sem-ch10-error-try`, `ts-ch02-nat-literal`, `ts-ch05-inference`,
  `sem-ch07-unbounded-prelude`, `ts-ch01-local-names`, `ts-ch03-tuple-show` (`prism_rtl` shows and pods
  tuples up to six), `ts-ch02-function-values`, `ts-ch05-let-generic` (pick LG), `sem-ch07-lambda-patterns` (a job lambda takes its
  parameters apart by pattern), `ts-ch04-fold-tuples`, `ts-ch05-template-body`, `ts-ch02-closure-names`,
  `sem-ch05-update-loops` (a `for` and a guarded `match` as statements of an `update`),
  `sem-ch07-pipe-record` (a `Start`'s function parameters are `Start`s, whatever color a
  `let`-bound lambda's own type gives them), `sem-ch07-while-guard` (a shrinking `while` is
  one unit of work, gap LP), `sem-ch07-while-nested` and `sem-ch07-while-match-for` (gap LP), `sem-ch07-lambda-cycle` (mutually recursive unbounded lambdas, a
  cycle a call), `ts-ch02-row-values` (a row kind's values built, compared, shown,
  matched and sent inside an element, pick RV), `sem-ch09-bundle` (pick BA),
  `sem-ch08-widget-args` (pick WB), `sem-ch09-host-param` and `sem-ch09-host-dynamic-answer` (pick HO),
  `sem-ch09-event-values` (pick EV2),
  `sem-ch04-for-dynamic` (gap DY), `sem-ch09-root-dynamic` (pick RD), `sem-ch09-host-reentry`,
  `sem-ch09-host-doc-in-tick`, `sem-ch09-host-delay`, `sem-ch09-host-nested` and `sem-ch09-host-teardown` (pick
  HO), `sem-ch07-updated` (pick UP), `sem-ch09-host-updated`, `sem-ch07-structural` and `sem-ch07-structural-law`
  (pick ST),
  `sem-ch04-row-entry` (pick RE).
  `sem-ch08-clause-widget` (pick CS). The rest of the corpus that passes: the conformance
  counters (`conf-ch01-counter`, `sem-ch01-counter`, `compiler-ch03-halfcounter`),
  `conf-ch01-journal`, `todos`, `sem-ch04-birth`, `sem-ch04-given`, `sem-ch04-quiz`,
  `sem-ch06-birds`, `sem-ch07-births`, `sem-ch07-rounds`, `sem-ch07-paged` (gap DY),
  `sem-ch09-hosts` (pick HO), `sem-appB-widgets` (pick MA), `ts-ch03-traits`,
  `ts-ch04-lifting`, chess (`board.prismtest`, byte-equal to `board.expected`, and
  `rules.prismtest`; `chess-rules` alone too), and prismc's own `lexer-checks` and
  `parser-checks`. `./prism run examples/chess` under `xvfb-run -a` opens the window;
  a robot (Xlib and XTest, as `worlds/canvas/rust/tests/xvfb_smoke.rs` clicks) clicks e2
  then e4, the root's result (printed with `PRISM_RESULTS` set) goes from "White's turn"
  to "Black's turn", and closing the window exits 0.
- **Also passing:** `sem-ch05-and-blocks` (a prelude function hidden by a program's own,
  index writes, `Buffer.Numbered`, `for &s <- &_:`) and `sem-ch08-viewer` (`@Flatten` over
  a `rebuild`'s dynamic, its transcript moved by pick ST).
- **Lifting:** `ts-ch04-lifting` passes: a function lifted over an event runs at its
  firing, so its `x on !e` is `x`, and an `if` over events gates each branch by the
  condition and merges them.
- **Speed:** the kept prismc compiles a conformance program in under a second, and cargo
  builds it in about 1 s against the shared target directory; the whole of
  `tests/compiled/run.sh` takes about 2.5 minutes. prismc compiles itself in about 5
  minutes and cargo builds it in about 2.5; `tests/checker/run.sh` takes about 1.5
  minutes; chess's tests run in under a second (release) and build in about 8 s.
- **prismc itself** checks clean under `--check`.

### The bootstrap

- **The kept binary** is a native prismc at a self-hosting fixed point, kept at
  `target/keep/prismc-*` (the newest; `PRISMC=` names another).
- **`bootstrap/bootstrap.sh`** (`make bootstrap`): the kept binary compiles
  `prismc/` (the working tree, templates included) into the crate
  `target/bootstrap/gen1`, cargo builds it, and gen1's binary compiles `prismc/` again into
  `target/bootstrap/gen2`. The two crates must be byte-equal: the compiler compiles itself
  to the crate it was built from. gen1's binary is then installed as
  `target/bootstrap/prismc`, which the scripts pick as the newest.

### The C++ target

`compile --target cpp` (`./prism compile|test|run --target cpp`) prints the model through
emit/cpp/ (`Emit.Cpp`), emit/cpp/templates/ (`Emit.Cpp.Templates`) and
emit/cpp/prelude.prism (`Emit.Cpp.Prelude`, the prelude in C++, function for
function the Rust one) as a C++20 program over `runtime/cpp` and `worlds/cli/cpp`
(claude/backend.md, "The C++ side"). `./prism test --target cpp` builds it with make and
`$CXX` (g++ by default) into `target/prism-cpp/NAME` and runs its tests under the C++
Test World; `tests/compiled/run.sh --target cpp` runs every program so.

- **Where it stands:** every program of `tests/compiled/run.sh` passes under C++ (every
  conformance program, hosting included; chess's `rules.prismtest` and
  `board.prismtest`; prismc's lexer and parser checks), byte-equal to the `.expected`
  files the Rust target passes. A conformance program builds in about 8 s with g++.
- **The C++ bootstrap:** prismc itself compiles to C++ (about 163,000 lines: 38 source
  files and 73 headers, pick MH) and `bootstrap/bootstrap-cpp.sh` (`make bootstrap-cpp`)
  takes it to its fixed point and checks its Rust against the Rust prismc's
  (bootstrap/README.md). clang++ -O3 builds it in about 100 s wall (1,003 s CPU) with
  twelve jobs on an M2 Pro; g++ -O1 took about 14 minutes on two cores, one file at a time.

C++ picks, each a place where C++ needs what the model leaves to Rust:

- **CH: a choice is a handle.** A choice (and a row kind's values, `Elem`, `Answer`) is a
  class holding a `prism_rtl::Rc` to a `std::variant` of its case structs. What it buys:
  `Rc<T>` of any value type is `T` itself (`IsValueTy`), so a recursive type needs no box,
  every `*` and `&` of the model is a no-op in C++, and `.` reaches a value's fields
  whatever the Rust side derefs. A record is a plain struct; a cycle of records by value
  with no choice on it does not compile (gap CY).
- **SE: statement expressions.** A block is `({ ...; value; })`, a `match` a statement
  expression whose `Slot<T>` (`T` the model's type for it) the arm that holds sets before
  `goto _done`; a labelled block's `break` sets its slot and jumps to its label. What it
  buys: `return`, `TRY` and `break` inside an arm mean what they mean in Rust. The cost:
  a GNU extension, which g++ and clang++ both have.
- **CB: a `match` that is a function's value returns from its arms.** A function's (or a
  method's, or a job's cycle's) body that ends in a `match` or an if-let chain is the
  same tests as statements, each arm `return`ing its value, an arm whose value is a
  `match` returning from its own arms in turn (`subst` and `reduce` in
  `sem-ch07-normalize`): no statement expression, `Slot` or `goto`. The Rust printer
  writes Rust's own `match` there, and an if-let chain as a labelled block, which needs
  no slot.
- **TR: `?` is `TRY(x)`**, a GNU statement expression (g++ and clang++ only, as every
  block is): the function returns the failure, else the expression is the value, moved
  out of the operand. Every `?` lowering writes is of a temporary (a call's result), and
  lowering refuses `?` of a place, naming it, so nothing named is moved from. The
  runtime's own uses are the same macro.
- **TT: a job's turn as a conditional.** Lowering gives no type for a `match` in a job's
  tail (`OutUnknown "the job's turn"`); such a `match` is a chain of `?:`, whose type C++
  works out from the arms (`finished(..)` converts to any `Turn<B>`); a guard binds the
  pattern twice, once for its test and once for the value.
- **PT: `->` and `.`.** A node is reached through a `prism_rtl::Rc`, a value directly. The
  printer writes `->` where the model says a receiver is a pointer (`self`, a field or a
  `let` of a pointer type, `node` in a constructor) and, elsewhere, for a member name some
  node has; every record has an `operator->` returning itself, so the second rule is
  harmless on a value.
- **RN: a `let` that rebinds a name from itself** (Rust's shadowing, an `&` block's
  `let mut fuel = fuel.clone()`) binds a new name (`fuel_1`), which the statements after
  it read; a scope that binds the name anew hides the rename.
- **KW, QL: names.** A C++ keyword takes a `_` after it (`or_`, `int_`, `explicit_`); a
  record field named as a value type's member (`show`, `to_pod`, `cases`) too; a node
  method named as some node's field too. A call whose name a node's member, a module or
  a `std` function could take is qualified (`program::past`, `prelude::lower`).
- **TB: the runtime's names by table.** `MethodForm` maps each Rust method of the model to
  a member, a free function of `lang.hpp`, an assignment (`Cell::set`), an exchange
  (`Option::take`) or nothing (`RefCell::borrow`); `CppCall` maps the runtime's
  constructors (`Hold::new` is `{init, cx}`, `Port::new` inside `Rc::new` `make_new()`).
- **NC: a node's constructor takes its fields in declared order**, the initializers the
  model orders by moves being copies in C++.
- **SF: a module's definitions in files of about 250,000 characters** (`lower/mod.cpp`,
  `lower/mod_2.cpp`, ...): one file of `lower.prism`'s needs more than 7 GB to compile.
- **BS: a 4 GB stack.** `main` runs on a thread with a 4 GB stack (committed as used), as
  the Rust program runs on a 1 GB one: unbounded code runs direct-style within a cycle,
  and a C++ frame of the emitted code is larger than Rust's.
- **PB: a pattern binds by reference into its value**, which therefore lives as long as
  the bindings: the scrutinee is a local (`_m`), and a guard's `let` over a value that
  is no place binds it to a local (`_c1`) first (`ts-ch06-array-in-case`, under ASan).
- **PC: persistent collections.** The C++ runtime's `Map` is a persistent AVL tree (the
  Rust runtime's too) and its `Array` a persistent balanced tree of chunks with a tail,
  the Rust runtime's design (runtime/cpp/include/prism_rtl/collections.hpp), so a fold
  that inserts into a map or pushes onto an array is cheap in both; with copy-on-write
  ones prismc's own lexer was quadratic (runtime/cpp/README.md, "Arrays and maps: the
  costs").
- **MH: per-module headers.** No header holds the whole program. Each module has, in its
  directory (`src/check/env/` for `Check.Env`, whose namespace is `crate::check::env`),
  `decl.hpp` (its types, nodes and jobs declared, its choices' handles), `types.hpp` (its
  records, tags and newtypes, each after the types it holds, and its choices' cases),
  `module.hpp` (its prototypes, nodes and jobs) and, when it has templates, `generic.hpp`,
  each included from `src/` in angle brackets (`#include <check/env/decl.hpp>`: a quoted
  name is looked up beside the including file first, where `a/types/` would answer for the
  top-level `types/`); in what follows `NAME.decl.hpp` is `NAME/decl.hpp`, `NAME.hpp`
  `NAME/module.hpp`; `elem.decl.hpp` and `elem.types.hpp` hold `Elem` and `Answer`,
  `programs.hpp` the program classes (for `main.cpp` and bench/gui/prism/driver.cpp). A
  layer includes only the layers of the modules it needs, which lowering records though
  the Rust printer ignores them: `ModDef.complete`, the modules whose value types its own
  types hold by value (through `?`, `~`, a tuple, a cell, an `Rc`, a type argument), whose
  `NAME.types.hpp` it includes; `ModDef.declared`, every other module it reaches by `use`,
  emitted family, qualified name (`Emit.Cpp.EmitCpp` in compile.prism, which has no `use
  Emit.Cpp`) or impl solving one of its constraints, closed over, whose `NAME.decl.hpp` is
  enough for its types. A source file includes its own `NAME.hpp` and those it needs,
  then the `NAME.generic.hpp` of each that has one: a template's call of a trait method
  is qualified, so it sees only the overloads declared before the template, and every
  `NAME.hpp` of the file comes first. A trait method without a default is declared a
  deleted template over `Self`, so a template can name it in a file with no impl; an impl
  of another module's trait is in a namespace of its own module (`zoo::impl_aloud_Loud`,
  since a namespace `aloud` inside `zoo` would hide the module `aloud`), and a
  using-declaration at the crate's scope brings it into the trait's namespace
  (`cpp-trait-across-modules`). Modules that hold each other's types by value with no
  cycle among the types (prismc's `check_types::St` holds a `typed::TProgram`, whose notes
  hold `check_types::Scheme`) share one types header, `check_types-typed.types.hpp`, the
  types ordered across them (`ModDef.typesWith`, a cycle of `complete`); each member's
  `NAME.types.hpp` includes it. The Makefile tracks the includes with `-MMD -MP`.
  - **Its size:** prismc in C++ has 73 headers of 31,971 lines in all; each of its 38
    source files includes from 565 to 31,171 lines of them (median 16,056; 474,242 over
    all files), where the one header every file included had 30,029 (1,141,102 over all
    files).
  - **Its build time** (prismc from clean, `make -j12 CXX=clang++`, Apple M2 Pro, the
    runtime's library built beforehand, other agents' builds running beside it at load
    averages of 50 to 75, so the wall times are rough): at `-O1` 96 s wall and 822 s of
    CPU (user and system) against 116 s and 980 s for the one header; at `-O2` 122 s and
    1,019 s against 163 s and 1,243 s. The CPU time falls by about a sixth: the
    definitions, not the declarations, are most of each file's work.
- **Building prismc, C++ against Rust** (4 November 2026, Apple M2 Pro, 12 cores, from
  clean, the runtime's C++ library built beforehand; the load average 3.4 at the start,
  another agent's jobs raising it to 40 by the end, so the wall times are rough):
  - C++: `make -j12 CXX=clang++ OPT=-O3` of the emitted program (38 sources, 154,462
    lines with its headers), 100 s wall and 1,003 s of CPU (user and system), about 10
    of the 12 cores busy throughout.
  - Rust: `cargo build --release` of the emitted crate (103,850 lines), its
    dependencies included, with cargo's defaults: the release profile (opt-level 3, 16
    codegen units, no incremental build) and as many jobs as cores (12): 91 s wall and
    255 s of CPU, 2.8 cores busy on average; prismc's own crate is one unit for rustc's
    front end, which runs on one core, and only its 16 codegen units run in parallel.

Gaps:

- **No canvas World in C++**: `./prism run --target cpp` runs the CLI World only; chess's
  window is the Rust target's.
- **CY:** a cycle of records held by value with no choice on it.
- **GC:** a generic choice's case constructor (`Tree(Tree::Node{..})`) needs its type
  arguments, which CTAD cannot deduce from a nested case; no program here has one.

C++ doubts:

- **MH1** A module's qualified references are found in its source text: a line that is no
  comment and holds a module's name and a dot (`Zoo.`) names that module. A text literal
  holding one adds a module the file does not need (more includes, never fewer); the
  checker's resolution of each name would be exact, and the typed program does not keep
  it.
- **MH2** `declared` is closed over the modules each one reaches, so a module reached
  through a long chain (front reaches nearly every module) includes nearly every header;
  the per-file sizes above are what that gives. A module's own uses, without the closure,
  would miss a type it names only through another module's (a lambda's parameter type
  written out, CH of the back-end picks).
- **MH3** A group of modules sharing a types header (`typesWith`) is a cycle of modules,
  not of types; a cycle of types by value (gap CY) stays a C++ error, as in the one
  header. A group is one header for its modules, not for the program.
- **MH4** A trait call's overloads are the ones its file declares before the template: an
  impl in a module the instantiating file never reaches would be missed (a `deleted`
  error, not a wrong overload). `declared` follows each constraint's impl to its module,
  so no program here has one; it is not proved for generic impls instantiated through a
  third module's generic function.

### Back-end picks

Each is Claude's, for Jimmy to overturn.

- **NM Names.** Rust's conventions from the program's names: `$count` is the field
  `count`, `!inc` the method `inc`, `ParseNat` the function `parse_nat`, `?Reduce`
  `reduce`. Sigils go; when two values of a module share a base (`?ParseNat`,
  `~ParseNat`) the sigiled ones take a suffix (`parse_nat_opt`, `parse_nat_try`); so do
  locals (`?half` is `half_opt` beside `half`; a capitalised local, a function value, is
  `k_fn` beside a plain `k`, `ts-ch02-closure-names`), and a local named as a function of its
  module or the prelude takes a `_` (`kind_` beside `Kind`'s `kind`, `ts-ch01-local-names`); a
  top-level `let` that shares its base with a function takes `_value` (`let hexDigits`
  is `hex_digits_value()` beside `HexDigits`'s `hex_digits`). A Rust keyword or a name
  the emitted code uses (`cx`, `node`, `path`, `v`) takes a `_`. `Job` and `Turn`, which
  a program may name a type of its own (prismc has a `type Job`), are always written
  `prism_rtl::Job` and `prism_rtl::Turn`; the other runtime names (`Cx`, `Fault`, `Pod`,
  `Path`, ...) are still bare, so a program type of one of those names in a module with
  nodes would shadow it (gap SH below). A
  merge with no name is `anon`; a node's internal names are first come, first served
  (`run`, `run_2`). A tag's `Level` struct is its family and name run together
  (`UiButton`, as `prism_rtl`'s tests have it). *Refused:* the old back end's sigil
  letters (`ld_count`), which read as a mangling.
- **MO Modules.** One Rust module per Prism module (`program` for the root module, in
  `src/program.rs`), nested as the Prism names are: `Check.Env` is `crate::check::env`,
  in `src/check/env/mod.rs`, declared by `pub mod env;` in `src/check/mod.rs` (a parent
  no Prism module is gets a file of its `pub mod` lines only). A prelude module (`Ui`,
  `Fs`, ...) is emitted only when a program module uses it or names its family on its
  `emit` line. A reference to another module is from the crate, `crate::ui::Click`, so a
  child named as a top-level module (`a::types` beside `types`) never stands in for it
  (`mod-nested-names`); a module file has no `use` of the others. Every module is
  `src/PATH/mod.rs`, a module of one Prism file too (claude/modules-style-pass.md, doubt
  RF), and a path segment that is a Rust or a C++ keyword takes a `_` (`shapes::switch_`).
- **OW Ownership.** A function takes every argument by reference and returns a value;
  lowering tracks how each Rust place holds its value (`ByRef`, `Owned`, `RcRef`,
  `RcOwned`) and spells a use as a borrow or a clone accordingly. A closure passed to a
  parameter is a borrowed `&dyn Fn`; a closure as a value is `Rc<dyn Fn(&A) -> B>`, its
  captures cloned in. A recursive payload is behind `Rc` (a type that reaches itself by
  value, directly, through `?` or a tuple).
- **OC A binding of an owned scrutinee is cloned at its one use** (`Some(b2) =>
  Some(Term::Lam(Rc::new(b2.clone())))` in `sem-ch07-normalize`'s `?Reduce`), where a
  move would do. Not cheap to decide in lowering: each read is spelled on its own as the
  arm's body is lowered (`Own` of an `Owned` place is a clone), knowing nothing of the
  reads after it, and a move is right only for the last read on every path, outside a
  loop and outside a closure's body. Deciding it needs a count of the binder's reads
  before the body is lowered (`ExprsIn` gives the mentions, but a mention is not one
  read: an interpolation or a `?` of it lowers to several), and a `Holding` for "owned,
  read once" that each of lowering's 82 tests of a mode would have to take; or a pass
  over the model's `Ex` rewriting the one `XClone`, a walker over its 50 forms that
  lowering does not have. Each such clone is a count's increment (a `Term` holds `Rc`s);
  C++ has the same copy (`prism_rtl::rc(b2)` of an `auto&&`).
- **CH Clauses and patterns.** A function's clauses are one `match` over the parameters
  that are not the same name in every clause; a number or text literal in a pattern is a
  binder and an equality guard, a number of the scrutinee's type (`0` against a `Nat` is
  `Nat::from(0u32)`, `ts-ch02-nat-literal`). A number literal keeps every digit: up to 9
  digits a `u32` (`Nat::from(5u32)`, C++ `Nat(5u)`), up to 18 an `i64`, past that parsed
  once from its digits (`Int::parse("..")`), on both targets (`ts-ch02-big-literal`,
  `ts-ch02-radix-literal`). A type gets `ToPod`/`FromPod` only when nothing
  it holds, through the declarations it names, is a function or a layer. A closure given
  to a prelude function has its parameter types written (`|found: &Option<Nat>, x:
  &Nat|`), as does an `update`'s mutable local, and a job's type parameters are
  `'static` (`ts-ch05-inference`). A pattern through an `Rc` payload cannot be a Rust
  pattern, so such a `match` is an if-let chain in a labelled block (`'c: { if let .. &&
  let .. = &**f { break 'c ..; } .. }`).
- **GV `given:`.** A labelled block: a bind is `let Some(p) = e else { break 'g None }`,
  a guard `if !(c) { break 'g .. }`; under `~` a failed guard is `Prelude.Unmet` with the
  statement's source text (D11). A function body of binds and guards is a `given:` without
  the word.
- **UP `&` blocks.** `update x:` is a block over `let mut this = x.clone()`: `&_.f <= v`
  is an assignment, `+=` an assignment of the sum, `&m[k] <= v` an `insert`, `&xs[i]` the
  prelude's `at_mut`, `let &i` a `mut` local, `while` a Rust `while`.
- **PL The prelude** is Rust written once in `emit/rust/prelude.prism` and emitted per
  program, not compiled from `prelude/prelude.prism`: most of the prelude is builtins
  with no Prism body, and the Prism-written ones (`Each`, `Filter`, `Or`, `Max`) are a
  line of Rust each. A name serving an array, a text and a map has variants (`len`,
  `len_text`, `len_map`), lowering picking by the argument's type. *Refused:* a crate of
  its own, which `runtime/rust/` (not this task's to change) would have to host.
- **WG Widgets in static positions are inlined** (Semantics ch. 8: transparent, their
  places numbered in the caller's sequence): their parameters bound by layer (an event to
  the caller's event, a dynamic to its reader, a plain value to a field). A node is
  emitted per root widget only: the ones a test names and `@main`.
- **EV The event graph.** Within a node an event is a graph node (a source, a named
  `let !x`, a combinator) and a pipe of pure transforms that calls straight through, so
  `increment` calls `delta` calls the hold's write, as in Compiler ch. 3. A named event
  is a method; `<|`, `!Union`, `!Both`, `!Except` are combinators with a `Stamped` field
  per input, decided in `wake` (option (c), left first, CXb). A dynamic is its reader: the
  plain expression over the current values, holds read at the read point; a hold write
  marks the node dirty when a place reads it, through dynamic `let`s.
- **HT Heights** come from the node's graph (a source at 0, a named event at what fires
  it, a combinator one above its inputs), not from the checker's group order, which
  counts reads through a hold's `then` as edges (gap DG; claude/backend.md cuts them).
- **RN `@Run`** expands to its Appendix B parts in the node: `@started` (it takes a
  number, Semantics ch. 8 RP), an `@Defer`, the merge `!started <| !again`, and the job's
  `Hold`, cycled once per firing.
- **JB Jobs.** An unbounded function that `@Run` is given, and every unbounded function
  a job calls, has a job struct of its parameters (`struct Normalize { t: Term }`): the
  call about to be entered. Its `Job::cycle` is the function's body to the next cycle
  point (D6: every call of an unbounded function from an unbounded function). A tail
  call is `Turn::More(Rc::new(G { .. }))`; any other call is
  `call_then(start_g(..), move |g_705: B| ..)`, the prelude's `Frame` around the
  callee's job with the rest of the body as its continuation, which the result resumes in
  the cycle the callee finishes; any other result is `Turn::Finished`. Frames nest, so a
  job's state is its stack of pending callers (`sem-ch07-walk`: `Total` over a tree of
  four calls, `5 ticks, 1 to 5`). A continuation is a `move` closure: each borrowed local
  the rest of the source names is cloned into it first. `start_normalize` is the job's
  `Start`; its fields are public, so a tail call from another module builds it. An
  unbounded function used as a value is a `Start`, an
  `Rc<dyn Fn(A) -> Rc<dyn prism_rtl::Job<B>>>` (one argument, a tuple when there are
  several; contract gap UV). A named function's value is `entered(start_g(..))`, a job
  whose first cycle only enters it, so that entering a named function is a cycle point
  however it is reached; a lambda of unbounded type is a value whose job is its body
  lowered as a job body (`once_job`), so a call in it is a tail call or a frame, and
  `entered` too (D6 as ruled: a call of an unbounded lambda is a cycle point;
  `sem-ch07-unbounded-lambda` 7 ticks, `sem-ch07-lambda-cycle` 5). A lambda's color is the typed
  output's, as the checker solved it (pick CV; `sem-ch07-pipe-record`'s `\|ignore| K v` is
  unbounded since `K` is). A job calls a value now (`call_value`, the callee's first cycle in this
  one); direct-style code runs it to its result (`run_to_end`). A function type whose
  color is not `bounded` (unbounded, or a color variable `u`, D8) is a `Start`, so a
  function with a `u fn` parameter takes one whatever its caller gives: a bounded
  function value is wrapped in a job that answers in its first cycle, a constructor is
  a closure (`Make Circle, n`), a lambda is a job lambda (`ts-ch02-function-values`). A
  color-polymorphic function (`Twice: u fn Nat, (u fn Nat -> Nat) -> Nat`) has a job
  too. A call that instantiates its color unbounded (an argument at a function parameter
  whose checked type is unbounded) is a cycle point, a frame or `Turn::More` like a named
  unbounded function's (D6 as ruled; `ts-ch02-function-values` 4 ticks); any other call
  enters it at once (`start_twice(..).cycle()`). Its calls through its closures are
  cycle points as their values decide.
- **LG A generalized local `let` is lowered once per instantiation.** A local `let` the
  checker generalized (Type System ch. 5; D8 for its colors) is a Rust value of one type,
  so lowering instantiates it: each distinct instantiation its uses have (the typed
  output's `insts`) is the value lowered again with the scheme's variables replaced, a
  binding of its own (`twin_fn_0`, `twin_fn_1`; one instantiation alone keeps the plain
  name `twin_fn`), and each use names its instantiation's binding. A use inside another
  generalized `let` may be at that one's variables (prismc's `Take`, used in `Builder` at
  `Builder`'s color): it stands for each of `Builder`'s instantiations. A widget block's
  `let` is a node field: at its uses' one instantiation it is one field
  (`ts-ch02-function-values`'s `Double`), at several a field per instantiation, each
  use naming its own (gap LW's first ruling, `ts-ch05-widget-let-generic`). So `let Twin = \|x| ((x,
  x))` used at Nat and at Text is two closures, each bounded where its uses are, and an
  unused parameter (`\|ignore, y| y`) is typed like any other (`ts-ch05-let-generic`). A
  `let` nothing uses is not lowered. A quantified variable that reaches the Rust anyway
  is no type: the printer writes a `compile_error!` naming the binding, never `()`.
  *Claude's:* one closure per instantiation, since a Rust closure has one type, and
  instantiations repeat (a lambda used at Nat in five places is one closure). *Refused:*
  a Rust generic function per local lambda (it cannot capture); one closure over `Pod`
  values (a dynamic layer, D16); inlining the lambda at each use (code grows with uses).
- **D8 The prelude's higher-order functions** given a closure that is not bounded are
  jobs: `fold_job`, `each_job`, `filter_job` and their map variants, which call the
  closure through `call_value`, so each call of a named unbounded function is a cycle
  point (`sem-ch07-unbounded-prelude`: `Each xs, Double` over three, `4 ticks`). A job makes
  one a frame or a tail call; direct-style code runs it to its result. Given a bounded
  closure they are the plain functions over `&dyn Fn`. Every unbounded function is
  also a direct-style Rust function, for the calls gap DS leaves.
- **ER `Error(..)` where a `~T` is wanted** is `Err(..)`: as an argument (the checker's
  pick ER), in a branch of an `if` whose value is a `~T`, and as the firing of an event
  that feeds a `!~T` (`sem-ch10-error-try`).
- **RW `for` rows.** A row is a node of its own (`TodosRow`) at `n/[k]`, with `key` and a
  weak reference to its block; the block's builder (a template, `ForBuild`) tears down
  left keys and builds new ones from the source after the tick (Semantics R7), run at the
  build and from the build list when a hold the source reads is written; it writes
  nothing to a kept row. A row's event result goes up to a gather in the block, which
  fires `!(K ==> T)` once a tick at a height above the rows'. A row has a field for each
  plain value of its block it reads (a parameter, a plain `let`), which the builder
  passes to its `build`, since a row is built where its block's locals are not in scope.
- **RE A row's entry is a projection.** `for k, $v <- $xs:` is a projection of its source:
  the rows are the key set of `$xs`, and each row's `$v` (`$_`) is `$xs[k]`, a dynamic
  derived above `$xs`, never state and never written by anybody. The block gets a getter
  `{rows}_source` (the source expression, a dynamic `let` of the block for the marks),
  and the row's `entry(cx)` reads it at the row's read point and indexes by `key`: the
  values as the tick opened during propagation, the values after it from then on, like
  any `$`. Its reads are a `ParentRead` of the getter, so a row's places over `$_` are
  marked by the block's writes of the holds behind the source, and `!Updated $_` (and
  `!Updated` of any dynamic the row derives over its block's holds) has a height: the
  rows are finished with their block (`FinishNode` keeps them `pending` until the block's
  heights are known), each `!Updated` node of a row floors at one above the block's
  writers of the holds behind what it reads through the parent (`ParentFloor`, the
  block's own floors inherited by rows of rows), and the block's gather sits one above
  the row's heights. A write behind a name the row hears (`RowHears`: what its
  `!Updated` nodes read through the parent, and what its own rows hear) calls the row's
  `parent_wrote_{m}`, which sets the flags and asks the wakes, and marks the row's own
  rows in turn; so the event fires once at its height with the entry after the tick, in
  the tick of the change, deduped by nobody (an edit of one key fires every row's
  `!Updated $_`, pick UD). A key that leaves ends its row at the end of that tick (R8):
  the entry after that tick does not exist, so a read that finds the key gone reads the
  source as the tick opened (`Cx::before`), and the leaving keeps that value in
  `left_entry`, the row's constant from then on (claude/backend.md, "Leaving"). The build
  still takes the entry after the tick as an argument (`value`), for the holds and plain
  `let`s computed at the build: the same value the projection reads there. In the build
  `$_` is that argument; in the methods the projection. `sem-ch04-row-entry`. *Refused:*
  the entry as a `Hold` the builder writes after propagation, which made `$_` state
  written at the end of the tick and left `!Updated $_` without a height; a `Cache` on
  the source getter, which every write path would have to mark (a row reads the source
  again at each read; cost noted under doubts). *Not done for `rebuild`'s `_`:* the
  checker types it plain ("`_` is that value, plain"), and a generation is built from
  `$x` after the tick and torn down at the end of the tick `$x` changes, so a plain
  field holding `$x`'s value for that generation is the projection already; no
  `!Updated _` can be written over it.
- **GN `rebuild`.** A generation is a node (`QuizGen`) at `n/#g` with `_` a plain field;
  the builder (`GenBuild`) leaves the old one and builds the next when the source's
  stale flag is set; an answer under `n/#g` reaches the current generation only. A
  generation's event result fires the block's event, which is what `@Switch $!x` hears
  (round 46 RD): `$!x <- rebuild` binds as that event. A `rebuild` over a widget of
  several clauses builds the clause whose patterns match, passing each parameter from
  the arm's binders (`Error(message=m, ..)` passes `m`).
- **CS A widget of several clauses in a static position** is a generation built once (a
  `rebuild` of its first argument, which never changes), the clause chosen at the build
  like a `rebuild`'s, and transparent (Semantics ch. 8): the generation's node stands at
  its caller's path and its places take the caller's positions from the call's on, as an
  inlined widget's do (`sem-ch08-clause-widget`: `@Note`'s label is `@Notes`'s place 1), the
  caller forwarding the World's answers at those positions to it (`GenForwards`).
  Clauses with different numbers of places take the widest clause's positions, so the
  caller's later places have one position whichever clause is chosen; where a narrower
  clause is chosen, a later place of the caller stands one or more positions past where
  compiler zero, which numbers places as they are built, puts it (prismc's own
  `@FinishCheck`: `@WriteOut nil, ..` has no place and `(? path), ..` one, and the
  `Prelude.Unwrap` after it is place 1 either way). One in a hosted scope is refused.
  *If otherwise:* the caller's later positions offset at the build by the chosen clause's
  width.
- **PO Ports.** An event of a block that a row or generation names reaches it through a
  `Port` in the block (`Port::fire` when the event fires) and a subscription the row makes
  when built and drops when it leaves (prelude `subscribe_row`, `Row::leave_row`).
- **WH Wakes by height.** A node with combinators at several heights records the heights
  it asked for (prelude `Wakes`) and its `wake` runs the lowest one asked, since
  `Node::wake` is not told its height (contract gap WH below).
- **PT Tests.** A test module (`test mod Tests`, a child module,
  claude/modules-style-pass.md) is read only under `--tests` (`prism test`, `prism check`);
  a loose file's tests are inline. `test` declarations are taken out before checking (a
  test's blueprint names a `@name:` block, which has no signature for the checker to read)
  and a `check` line's pattern too (the Test World reads patterns from the script; the
  checker mistypes `Show (? 3) => "? 3"`). A test's root is found by name; its arguments
  arrive as `Pod`s. The compiled program's scripts are its test modules; each transcript
  is the `.expected` of its name (`tests.expected` for `tests.prism`), and the Test World
  passes over a test module's header and helper declarations.
- **FV First values.** A place whose element answers a dynamic asks `cx.first_value` at
  the build (a hosted place asks its host, `host_k.first_value`, which builds the
  supplier row for the place early, pick HO) and fails the build when there is none; its
  `Hold` is the place's.
  A place's answer type is its tag's output as the checker instantiated it at that
  application (`Prelude.Unwrap`'s `!a` is `!unit` there); a type variable left open is
  the answer as it came, a `Pod`.
- **TR Traits.** A program's trait is a Rust trait whose methods are associated
  functions over `Self` (`Loud::loud(&x)`), defaults in the trait; an impl is a Rust
  impl. An `impl ^Show` is a function of its own (`show_sq`) and `impl Show` over it,
  and the type then crosses as `Pod::Shown` (claude/backend.md, "Printing"); an `impl ^Eq`
  is `PartialEq` over its `==`.
- **FL `Fs.List`.** prismc's own `fs.prism` replaces the prelude's `Fs` to declare it,
  since a family's tags are declared in the family's module (Semantics ch. 6) and the CLI
  World answers it for the program that declares it (worlds/cli/rust/README.md). Every
  directory argument is listed, and the directories above a module directory, to find
  the tree's root.
- **TR2 Trees** (claude/modules-style-pass.md). A directory argument holding an app.prism
  or lib.prism is a tree; one holding a mod.prism is that module of the tree whose root
  is the nearest directory above it, with its test modules and the modules they reach by
  `use`; any other is refused (`no-root`). The front end reads each file's header from
  its text, walks the `mod` lines (each child `name.prism` or `name/mod.prism`), and
  gives each module its one file, named by its path. `mod-missing`, `mod-both`,
  `mod-leaf`, `mod-self`, `mod-lowercase`, `mod-stray` (a one-file module or a
  `.prismtest` no `mod` line declares among them), `module-line`, `two-roots`,
  `lib-header`, `no-root` and `prismtest` stop the run before parsing; `test-outside` and
  `test-declares` are found over the parsed trees, `test-only-reached` over the checker's
  references. `App` names a program tree's root in a `use` line or a qualified name.
- **RT The runtime path.** `Cargo.toml` finds the library and the Worlds at `--runtime
  PATH`, by default one `..` per directory of a relative OUT (OUT under the repository
  root, as `./prism` and the bootstraps call prismc), so the stages emit the same file
  and the seed names no machine's paths.
- **AR An absolute OUT.** With no `--runtime`, an absolute OUT takes `PRISM_ROOT`
  (`Process.Env`, which the CLI Worlds answer), as the C++ Makefile does; with neither, the
  compile is refused, naming `--runtime`, since `..` per segment would climb to `/`
  (`tests/driver`). *Refused:* `PRISM_ROOT` for a relative OUT too, which would write
  this machine's absolute paths into every crate the bootstraps emit, the seed's among
  them. A relative OUT from another directory than the repository's still lands on that
  directory's ancestors; no file-system check finds it.
- **RV Row values.** A row kind other than `Fault` is plain data: `type Draw = row` is a
  Rust `enum Draw` in its module with a variant per tag of the kind in the program, each
  holding the tag's struct (`Draw::DrawRect(DrawRect { rect, paint })`), so a `[Draw]`
  is an `Array<Draw>`. A tag applied in an expression builds the variant; a tag pattern
  is the variant around a struct pattern; `Draw.*` is a `matches!` over the family's
  variants. Every tag struct and row enum is `Clone`, `ToPod` and `Show` (printing as
  its `Pod`, `Draw.Rect(rect=Rect(..), paint=Grey)`), and `PartialEq` and `FromPod` when
  all its fields are plain data. A document's element, `Elem r` (the checker's WL), is
  `crate::Elem`, emitted in `main.rs`: a variant per element tag's `Level`, so a pattern
  binds each field plain (`Ui.Label(text=$t)` binds the text in that tick). A live
  element, `Element r o`, is `prism_rtl::Live<crate::Elem>`: its level at the build
  (`identity`: the tag and plain fields, which never change while it lives) and its
  level over time (`level`, a `Dynamic<Elem>`); a pattern matches the tag and plain
  fields on the identity and binds a `$` field to `level.map(..)`, the field's dynamic
  (gap LE). A tag and a record that share a key (`Fs.Contents`) are told apart as the
  checker does: the tag only when written with its family. `Dynamic`'s update port
  carries `()` (the value moved; readers read again), so a projection shares its
  source's port and `Dynamic::map` subscribes to nothing (`ts-ch02-row-values`).
  *Refused:* row values as `Pod`s, which made every field access a runtime decode.
- **BA Bundles.** A tag whose output is a tuple of plain parts and one event part
  (`Gui.Window`'s `((WindowId, !WindowInput))`), bound by a tuple pattern: the plain
  parts are the place's first value (one part itself, several a tuple), each bound part
  a field; the event part fires on each answer at the place (`sem-ch09-bundle`). A `$` part,
  or a second event part, is refused loudly.
- **WB Widget arguments.** A widget parameter (`@child`) binds its argument, a widget
  block (`@widget: ..`) or a widget expression (`@counter`), with the caller's
  environments; applying the parameter lowers the argument there, so its places take
  this node's next positions (transparent, Semantics ch. 8) and each application has
  its own holds (`sem-ch08-widget-args`). A parameter applied to arguments, or read as a value,
  is refused loudly. *Refused:* the argument as a blueprint value built at run time,
  which a static position never needs.
- **HO Hosts.** A host is Prism inside the same graph and the same tick (Semantics ch. 9
  as ruled), and a host document is a `for`. `@HostDocument @child, supplier` (and
  `@HostSome`, whose `nil` forwards, through prism_rtl's `forward_on_none`) inlines the
  child, a named widget or a widget parameter's argument, into the node of the host's line
  (its owner) in a scope of its own: `Wx.hosts` lists the node's hosts, a position is a
  `Pos`, a scope and a number (scope 0 the node's own places, scope `k` the child of host
  `k`'s), a hosted position is spelled `place_h{k}_{n}` in names and doc comments, and a
  hosted place emits into no document: it is keyed by its path relative to the child
  (`Path::root().at(n)`, `PlaceAt`) in the host's document. The owner gets, per host:
  - the snapshot `host_k_doc_snap`, taken in the construction step from `host_k_doc_now`,
    every child place as the program's `crate::Elem` (its static places by getter or
    constant, and under each `for` and `rebuild` of the child the rows' `Row::document`,
    recursively); the structure itself is `host_k_doc_direct(&self, cx) -> Map<Path,
    Elem>`, which the host's `for` and its rows' entries read, and the program's `$doc` is
    the getter `host_k_doc`, a structural dynamic one tick behind (pick ST). The live
    computation's reads (`dynReads` of `host_k_doc_now`) are everything the child's places,
    builders, generations and rows read, so a write behind any of them marks the snapshot,
    lists the build and dirties the host's rows; the owner's places over `$doc` are marked
    when it is published, `host_k_doc_publish`, the tick after. A child row that writes a
    hold of its own tells the owner through `document_moved` → `host_k_moved`, which marks
    the same (`HoldWritten` of `host_k_doc_now`);
  - the builder `host_k_rows_build`, the ordinary `for` template over the structure, whose rows
    are prism_rtl's `HostRow`s at `hostpos/[p]`, one per child place: built when the key is
    in the child's document after a tick and not before (R7), kept while it stays, torn down at the end of
    the tick its key leaves (R8), each marked by the owner's writes as any row (`RowMarks`);
  - the typed dispatch `host_k_answer(self: &Rc<Self>, place: &Path, v: &crate::Answer,
    cx) -> Done` from a child place's path into the child's input method, a hold the
    place's element answers (`DynPlaceWrite`, every mark a hold's sink makes), or the row of
    a `for` or `rebuild` under it (`Row::deliver`, typed, recursive). A variant of another
    tag is a fault of the tick;
  - the field `host_k: Rc<prism_rtl::Host<Elem, Answer, Owner>>`: the supplier (a lowered
    closure or named function over `&Path` and `&Live<crate::Elem>`), the rows, the
    enclosing host when the line stands in a hosted scope, and the two methods above as
    function pointers, made in the build before the node and linked to it after
    (`host_k.link(&node)`), since a child place's first value asks the host before the node
    exists.

  `crate::Answer` is emitted beside `crate::Elem` in `main.rs` (`AnswerEnum`): a variant
  per element tag carrying the tag's output type (a bundle's event part; an open type
  variable a `Pod`), an accessor per variant (`v.ui_button()?`), and the decode of a
  World's answer by the element's tag (`prism_rtl::AnswerOf<Elem>`). A supplier's `@ x` is
  typed by the tag of the arm it stands in (`TagPat` binds `#tag` on a live element;
  outside a tag arm it is refused loudly): `@ x` and a constant `@ $ v` are
  `plain_blueprint` (the first value once), `@ never` `never_blueprint`, `@ !e`
  `event_blueprint` over `e.map(Answer::Tag)` (each firing a computed answer, delivered as
  an input of the next tick, which is owed, `Cx::answer_later`), `@Element e`
  `element_blueprint` (re-emission at `hostpos/[p]/0`, the World's answers and pushes
  handed to the child in the same tick). A dynamic answer that is not a constant (`@ $d`,
  `$d` a hold or a derived dynamic of the owner; round 3 DY; `sem-ch09-host-dynamic-answer`) is
  `dynamic_blueprint` over a `Dynamic<Answer>`: its first value is read at the row's build,
  synchronously, and each later change is a push queued with `Cx::answer_later`, sampled
  when delivered as the next tick's input, which is owed, once per tick however many
  writes behind `$d` the tick had. The supplier is a closure made before the owner's node,
  so `DynamicAnswers` gives each such `@ x` of the supplier a getter of the owner,
  `host_k_dyn_j`, and a port, `host_k_dyn_j_port`, fired by every write behind it
  (`MovedBy`); the `Dynamic`'s read goes through the owner's `prism_rtl::Link`
  (`host_k_owner`, linked at the build) to the getter, and, while the owner's node is
  being made (an early row's first value), reads the dynamic's value at the build: every
  hold's initial value stands in a local `{f}_init` (`HoldLine`), a dynamic `let` over
  such values in `{rs}_init` too (`LetLine`), bound in the build environment under
  `{name}#init`, captured by the supplier's closure, and `InitEnv` reads the dynamic's
  expression over them. A dynamic answer reading what has no value at the build (a pushed
  place's hold, a structural dynamic, a `let` after the host's line) is refused loudly. A
  widget-function blueprint as an answer, and a bundle in a hosted scope, are refused
  too.

  A row of a hosted child's `for` or `rebuild` (`Wx.hostedBy`) emits nothing of its own
  either: it takes the enclosing host (`enclosing: Rc<dyn HostOf>`) as a parameter of its
  `build`, asks it its places' first values, implements `Row::document` and `Row::deliver`
  for its own places, and tells it when a place may have moved. Nested hosts compose:
  a host inside a hosted child builds its rows in the enclosing host's scope
  (`prism_rtl::Scope::Hosted`), so a forwarded element there is an entry of the enclosing
  document rather than a place of the World's, and its first value comes from the enclosing
  host; each computing layer costs a tick and a forwarding layer none (`sem-ch09-host-nested`).
  The child's result is the line's first part (`WrHosted`), the document the second
  (`BindHosted`). `sem-ch09-hosts`, `sem-ch09-host-param`, `sem-ch09-host-doc-in-tick`,
  `sem-ch09-host-updated` (pick ST), `sem-ch09-host-reentry`, `sem-ch09-host-delay`, `sem-ch09-host-nested`,
  `sem-ch09-host-teardown`. *Refused:* the host as a World in miniature (a `Host` node with a
  `Pod` boundary answering through `Node::answer` in the same tick), Jimmy's ruling.
- **EV2 Event values.** A named event (`!x <- ..`, `let !x = ..`) fires a port of its
  own (`x_v_port`, made in the build before the node so a closure can hold it), and
  `!x` read as a value is `Event::of(&x_v_port)`. An event expression outside the pipe
  that maps it is built by the library's combinators: a lifted call is `Event::map`,
  `!Filter` `filter_some`. A function that gives a blueprint (`?Boxed: fn Path, Element
  o, !Routed -> ?@o`) is not lifted over its `!` parameters: it takes each as an event
  value. `sem-ch09-event-values`.
- **PU Pushes.** A place whose element answers a dynamic keeps its element
  (`f_element`), registers with `cx.listen`, and answers a push by writing its hold with
  every mark a hold's sink makes (`DynPlaceWrite`: the node patched, the builders and rows
  over it listed, the `!Updated` nodes told); a hosted place's pushes come typed from its
  host the same way.
- **RD The root's dynamic result** goes to the World at the build and in the patch pass
  whenever it moved (`result_value`, `result_last`); a test root registers under its
  sigils (`@$board` is `$board`).
- **DO One decision per input a tick.** A combinator takes its stamped inputs as it
  decides them (`Stamped::take`), so a node woken again in its tick decides only what
  came since; a hold written twice in a tick is a fault of the tick, `Runtime.TwoWrites`
  (Semantics R3). A builder runs once a tick, after propagation, from the values after
  the tick (Semantics R7): its source's marks list it, and nothing lists it again; the
  owner's `build` runs a hosted child's builders before its host's (`InnermostFirst`), so
  a host's `for` over `$doc` sees the child's structure settled (`sem-ch09-host-doc-in-tick`).
- **PS `Pod::Shown`** compares, orders and hashes by its structure, its text only
  printed, so a key with an `impl ^Show` (chess's `Spot`, `e2`) orders by its fields.
- **MA `@Machine`, local functions, `!MergeAll`.** `@Machine step, init, !a` is a hold
  of the state and a sink on `!a` that calls the named step function, writes the state
  and fires the output on `? b`. A block's local function (`fn Paired k, v = ((k, v))`)
  is inlined where it is called, each parameter a local bound to its argument.
  `!MergeAll [[..]]` is a combinator of one stamped input per element of the literal,
  firing the list of what fired, in order (`sem-appB-widgets`). The prelude's
  Prism-written widgets (`@Once`, `@AfterFirst`, `@Numbered`) inline as any widget.
- **SC The code's site.** An inlined widget's code is lowered in its own module's names
  but placed in its caller's module, and a check line's in `main.rs`: lowering keeps the
  module that resolves names apart from the one paths are written from.
- **UP A dynamic is a behavior and an event.** The behavior half is pull: a sample in tick
  n (a getter call during propagation, `$d on !e`) reads the holds at the read point, the
  values as the tick opened (Semantics R2). The event half, `!Updated $d`, is a graph node
  of kind `GnUpdated`, decided in the wake like a merge: its height is one above every node
  whose sink writes a hold the dynamic reads (the reads closed over the dynamic `let`s,
  `ReadsClosed`, so `!Updated $b` with `let $b = $a + 1` sits above `$a`'s writers;
  `@Machine`'s state hold counts). Each such write sets the node's `Cell<bool>` flag
  `{name}_moved` and asks the wake at its height, as an answer or a push that writes a
  place's hold does at height 0; the decision takes the flag (`replace(false)`, so once a
  tick and a double firing is impossible) and fires the dynamic's value after the tick,
  read by the node's `{name}_value` getter under `Cx::after`, which moves the read point
  to n+1 for the read and back. So the event fires at n with the value after n, in the
  tick of the change, whenever the dynamic was recomputed, deduped by nobody (`$a * 0`
  fires on every write of `$a`; `sem-ch07-updated`). This is the rule for a dynamic over holds
  only; a structural dynamic is written the same way when it is published (pick ST). A row's `!Updated` over its entry or
  over its block's holds read through the parent is pick RE's (`sem-ch04-row-entry`). *Refused
  loudly* (`compile_error!`, not an event that never fires): an `!Updated` whose dynamic
  reads anything no sink of the node writes, and a row's that reads a block name no hold
  of the block stands behind, or a structural dynamic of the block (gap SU).
  *Refused:* a stamp on every hold compared in the wake, which fires nothing for a write
  of an equal value and so dedupes, against the ruling; and firing from the patch pass,
  which puts the event a tick late.

### Contract gaps

Where claude/backend.md is silent or the library lacks what it describes, with the first
ruling taken (Claude's).

- **UV An unbounded function value.** The contract has `fn(A) -> Rc<dyn rt::Job<B>>`, a
  function pointer, which a lambda that captures cannot be. *First ruling:*
  `Rc<dyn Fn(A) -> Rc<dyn Job<B>>>`. *If otherwise:* lambdas of unbounded type are
  refused as values, or lifted to top-level jobs over their captures.

- **WH `Node::wake` has no height.** The contract has a node run "its combinators in
  distance order, once per height"; the hook is not told the height. *First ruling:* the
  node keeps the heights it asked for (pick WH). *If otherwise:* `Node::wake(height)`.
- **DS Calls a frame does not reach.** A job makes a frame for a call that its
  expression makes whatever its value: in a statement, an argument, an operand, a
  scrutinee or a condition, at any depth. A call under a lambda (a closure an unbounded
  `Fold` is given, D8), in an arm of a `match` or a branch of an `if` that is not in tail
  position, or right of `and`/`or`, still runs direct-style inside the cycle; such a job
  takes fewer ticks than D6 says.
  *Next:* a non-tail `match` binds its continuation once (`let k = Rc::new(move |x|
  ..)`) and each arm frames into it; the prelude's higher-order functions as jobs. Cost:
  a cycle walks the frames, so it is linear in the pending calls.
- **LP A loop in an unbounded function** (D6 as ruled: a turn of a `while` the shrink
  check does not clear is a cycle point; a loop it clears is one unit of work). The
  checker runs its loop check on unbounded functions too and notes each loop it does not
  clear in the typed tables (`TProgram.unshrunk`), with no error; lowering reads them
  (`Lx.unshrunk`). Such a `while` at the top level of an `update` block in a job is a
  loop job (`loop_job`): the block's state (`this` and its `&` locals) is the job's,
  each turn a cycle, the rest of the block its continuation. A cleared loop is a Rust
  `while` inside the cycle (`sem-ch07-while-guard`, a `Nat` fuel: 3 ticks). A loop job under an
  `if` of the block binds what follows the `if` once, a continuation over the block's
  state that each branch ends in (`IfJob`); one in another loop's body makes that loop's
  turns jobs (`loop_turns`: a turn that reaches the inner loop ends after it, and the next
  test starts a cycle), so `sem-ch07-while-nested` costs what a top-level loop does (5 ticks for
  3 turns under an `if`, 10 for 2 by 2). A loop job in an arm of a `match` of the block binds
  what follows the `match` once, as under an `if` (`MatchJob`); one in a `for` of the block
  makes the `for` a loop over its entries whose test is taking the next one, each turn a
  job (`ForJob`, the preludes' `for_turns_keyed` and `for_turns_indexed`), so
  `sem-ch07-while-match-for` costs 5 ticks in a `match` and 9 for a `for` of two entries
  of 2 and 1 turns. A loop job in a `for &s <- &_:` is refused (its references cannot
  outlive a cycle).
- **FA A field's `.` or `->`** is chosen by the receiver's type where `?Pointer` knows it,
  and only otherwise by its name, `->` when any node has a member of that name
  (`cp.members`). Where the type is not known, a value's field named as some node's
  member is still spelled through a pointer.
- **SH Shadowed runtime names.** A program type, case or node named like a name the
  emitted code writes bare (`prism_rtl`'s `Row`, `Node`, `Hold`, `Cx`, `Pod`, `Path`...,
  and Rust's `Rc`, `Cell`, `Vec`...) takes a `_` in both targets (`lower.prism`'s
  `runtimeNames`, through `Camel`), as a Rust keyword does through `Snake`; printed
  values keep the Prism name (`cpp-runtime-names`). *Not yet:* a function whose Rust name
  is one of the runtime's free functions the emitted code calls bare (`call_value`,
  `entered`, `once_job`) still shadows it: a `CallValue` in a module with jobs breaks that
  module's frames, and a local `startBody` beside an unbounded `Body` is that function's
  `start_body`.
- **PF A port's fault.** `Port::fire` returns `Done`, and an event method returns
  nothing, so a subscriber's fault is dropped. *First ruling:* event methods return
  `()`; faults in rows' subscribers are lost. *If otherwise:* every event method returns
  `Done` and calls `?`.
- **ST Structural dynamics.** A host's `$doc` and a `for`'s gathered `$(K ==> T)` are
  lawful dynamics (R2/R3; Semantics ch. 7, "A dynamic is a sample and an update event")
  one tick behind the structure they describe (round 3 ST, the hub's ruling). Each keeps a
  `prism_rtl::Snapshot` (`{name}_snap`; `SnapDef`), taken from the live computation
  `{name}_now` once per tick when a write marked it, in the node's construction step where
  the structure behind it has settled (`BuildSteps`: a host's document after the child's
  builders and before the host's `for` over it, a gathered dynamic after its builder;
  builders and generations run scope by scope, innermost first): the snapshot S(n). The
  dynamic's value after m is S(m−1) (after the node's build, the build's snapshot), which
  the getter (`host_k_doc`, `rows_all`) answers at the read point. When S(n) differs from
  S(n−1) (`take_compared` where the type has equality, `TyHasEq`: `Elem` always, a gathered
  `T` when it derives `^Eq`; else every take), the node queues `{name}_publish` with
  `Cx::answer_later`, once a tick (`Snapshot::queue`), and tick n+1 is owed. The
  publication, among n+1's first firings, is `HoldWritten` of the dynamic's name: places
  over it dirtied, builders and generations over it listed, rows reading it marked, and
  every `!Updated` over it or over a dynamic lifted from it (`let $x = Tag $doc, $clicks`)
  told and woken, an ordinary pick UP node. So `!Updated $doc` fires at n+1 with S(n),
  while every sample at n+1 (`$old <- 0 then Size $doc on (!Updated $doc)`) reads S(n−1),
  and the patch pass and the builders of n+1 read S(n) (`sem-ch07-structural-law`); a hold
  written in n+1 beside it gives one firing. The live computation's reads are its own
  (`dynReads` of `{name}_now`, `SnapDef.live`): a write behind the structure marks the
  snapshot and lists the build (`SnapsBehind`, `ListsBuild`) and touches no reader of the
  dynamic. The host's own rows do not read `$doc`: its `for` and its rows' entries read the
  structure, `host_k_doc_direct` (`Snapshot::current`: S(n) from n's construction step
  on), so a newborn child place gets its supplier row in its birth tick and a host that
  forwards everything is observably not there; a current read of a marked snapshot before
  it is taken is a panic naming it, not a stale value. A row whose own hold moves its
  gathered result tells the block (`{name}_all_moved`, `resultMoved`). A `rebuild` that
  hands back an event forwards it from the current generation as it fires (`@Switch` is
  the identity on it), so a newborn generation's `@started` and first firings reach the
  block in their tick. A `rebuild` that hands back a dynamic, `$$a`, is a structural
  dynamic too (`{gen}_result`): the build keeps the newborn generation's result as a
  `Dynamic` (`result_dynamic`, kept in `{gen}_result_live`) and marks the snapshot, which
  the construction step takes after the generation is built. `@Flatten $$a` keeps its
  current inner in a `Hold` (`{name}_inner`), adopted at the build and, when the
  `rebuild`'s dynamic is published, among the first firings of the tick after
  (`{name}_adopt`, queued with `Cx::answer_later`, so that tick is owed); its dynamic
  samples the inner at the read point, and a write behind the current generation's result
  marks its readers (`{gen}_result_moved`). So a generation built at n is the `rebuild`'s
  dynamic after n+1 and the `@Flatten`'s from n+2: `sem-ch08-viewer`'s label lags the watch
  by two ticks, where the book has one (its transcript derives it). `@Flatten` of anything but a
  `rebuild`'s dynamic is refused. *Refused:* the whole dynamic a tick late, the host's
  rows included (a host's `for` would build a newborn's row a tick after the child did,
  breaking forwarding parity); the snapshot of n read by samples at n+1 beside the event
  carrying it (a sample in the event's tick would read the new value, against R2/R3); the
  event at n with the structure before n (a visible inconsistent state).
- **DY Dynamics across nodes.** A row (a `for`'s or a generation's) reads its block's events through ports (`RowEnv`: a port node in the row, subscribed to the block's port; `sem-ch04-row-block-event`, a row of a row through two), and its
  block's holds and dynamic `let`s through its parent at its own read point
  (`sem-ch07-paged`'s inner `rebuild $src`). A `for` row is marked by the block's writes
  of what it reads (`Wx.rowMarks`: its `ParentRead`s as the block's own, `RowWritten`),
  its `!Updated` nodes told through `parent_wrote_{m}` (pick RE), and the block is
  patched after its rows are built; a generation's reads through its parent are
  `ParentRead`s too (`RowEnv`), so a place of it over a block hold is live and the current
  generation is dirtied by the block's writes (`RowMarks` of kind `GenerationRow`), but a
  generation is told of no write, so an `!Updated` in it over a block name is refused
  loudly (`UpdatedGaps`). A row's port for a block event sits at height 0 in the row, below the
  event's height in the block; the runtime's clamp (`Cx::wake` at or below the running
  height runs at the next) orders it. An event reference names the node whose graph it
  indexes (`EvRef.owner`); one read or wired in any other node crossed without a port,
  a lowering bug, and is refused naming it (`Crossed`). *Next:* the contract's `Dynamic` with its update
  port.
- **LE A live element's `$` field in a plain function.** A plain function is lifted:
  its `$` values are read at the caller's read point, and it has no `cx` to read a live
  element's dynamic at. *First ruling:* refused loudly there (a supplier is lowered in
  the build, as a value, so it is refused there too; `Lx.inNode` is never set). An element
  applied as a plain value (not a place, not a supplier's argument) is refused too: only a
  host makes a live element. *If otherwise:* lifted functions take `&Cx`.
- **SU A row's `!Updated` over its block's structural dynamic.** A row hears the block's
  writes through `parent_wrote_{m}` and decides its `!Updated` nodes in its wake; the
  block's publication of a structural dynamic (pick ST) is `HoldWritten` of its name, so
  the row's marks are already in it, but no test pins a row's `!Updated` over one. *First
  ruling:* refused loudly (`UpdatedGaps`). *Next:* a test, then the refusal goes.
- **DM The document is built whole once per marked tick.** A host row's entry
  (`HostRow::entry`), the owner's places over `$doc` and the host's builder read the
  document's snapshot (pick ST: `host_k_doc` and `host_k_doc_direct`), a persistent map
  in both runtimes, so a read is a shared copy and a lookup, not a document. The live
  computation `host_k_doc_now` still builds the child's whole document (every place's
  level, every row's) when the child's writes marked it, once in that tick's
  construction step: a tick that moves one of `n` places costs one document of `n`, not
  `n` documents (`sem-ch09-hosts`, `sem-ch04-row-source`'s hosted test). *Next, if it
  shows in a profile:* the snapshot updated per place from the child's writes, each
  place's level recomputed only when its own reads moved.
- **SR A `for`'s source is computed once per write behind it.** The block's source getter
  (`{name}_source`, `Tag $doc, $clicks` in `sem-ch09-host-doc-in-tick`), which the
  builder and every row's entry read (pick RE), keeps its value in an `rt::Cache`
  (`{name}_source_cache`), marked by every write path through `MovedBy`
  (`SourceMarks`): a hold's sink, an answer or a push at a place, a snapshot's
  publication, `@Machine`'s state, the block's write a row hears through its parent
  (`sem-ch04-row-source`).

### Strings: what is copied

Where a text is made, copied or compared, in both runtimes, both emitted preludes and the
printers (handoff item 5). Without a copy: a `Text` literal in C++ (`Text::lit`, its
bytes where the program has them, no buffer or count); `Split`'s pieces (views of the
text's buffer, both targets); `Concat` (one buffer of the summed length, the pieces'
bytes copied once); `Upper`, `Lower`, `?ParseNat` and a template placeholder (`place`)
reading the text's bytes where they are; a number shown or interpolated (written into
the output, with no string of its own: Rust's `Printer::display`, C++'s `display`); and
interpolation's and `Concat`'s result in C++, which skips the UTF-8 check its pieces
already passed (`Text::of_utf8`). On the normalize compile this took the C++ prismc
from 1.777G to 1.516G instructions and its self-compile from 836.9G to 727.4G (68.7 s
to 60.6 s); the Rust prismc moved within noise (1.403G to 1.400G; 725.2G to 724.0G).
What stays, and why:

- **SC1 A Rust text literal is a buffer of its own** (`Text::lit` is `Text::new`), made
  each time the literal is evaluated. A `&'static str` beside the `Rc<str>` (an enum)
  made every read branch: 2% more instructions on the normalize compile than the
  allocations it saved, so it was taken out. Without `unsafe` (the runtime has none) a
  branch-free view needs the buffer always counted.
- **SC2 `format!` into a `String`, then `Text::from`** (interpolation in Rust, `show_text`):
  the `String` is copied once more into the `Rc<str>`, which cannot take a `String`'s
  buffer over. Writing straight into a counted buffer needs a builder the runtime does
  not have; C++ moves its `std::string` into the block instead.
- **SC3 C++ `Upper` and `Lower` are ASCII only; Rust maps all of Unicode**
  (`str::to_uppercase`): the targets disagree on a non-ASCII letter (`"é"`). No test has
  one; the fix is the Unicode tables in C++, or the books narrowing the functions.
- **SC4 A runtime-made C++ text is checked for UTF-8 each time** (`Text(std::string_view)`:
  a World's input, a `Pod`'s name), as Rust's `str` is by construction; it is the guard
  that a text never holds broken bytes, so it stays.
- **SC5 The Test World compares a place by its printed path** (`p.to_string() == *t.at`
  in test_world.cpp, and Rust's alike), once per step of a transcript, not per tick of a
  program.
- **SC6 Escaping pushes one character at a time** (both printers' `quoted`): each
  character is examined anyway; runs of plain characters could go in one append.
- **SC7 A cursor's step copies its text** (a count's increment and decrement, both
  targets): `?Step` returns a new cursor, which holds the text. Non-atomic in C++ now.

### Arrays and maps: what they hold

Peak memory, wall time and instructions retired (`/usr/bin/time -l`) of each prismc
compiling itself (`compile prismc --prelude prelude --name prismc`, to its own target)
and `examples/chess`, with the collections each runtime had before and has now
(runtime/rust/README.md; runtime/cpp/README.md, "Arrays and maps: the costs"). Apple M2
Pro, 5 November 2026, cargo release and clang++ -O3, other agents' jobs running (load
average 3 to 13), so the wall times are rough; "footprint" is macOS's peak memory
footprint, "RSS" the maximum resident set; instructions are the least of three runs on
chess and of two on the self-compile.

| Run | footprint | RSS | wall | instructions |
|---|---|---|---|---|
| Rust prismc, self: `im_rc` `Vector` and `OrdMap` | 2,878 to 2,890 MB | 2,808 to 2,896 MB | 75 to 83 s | 723.1 G |
| Rust prismc, self: AVL `Map`, `im_rc` `Vector` | 2,722 to 2,731 MB | 3,072 to 3,293 MB | 57 s | 692.6 G |
| Rust prismc, self: tree of chunks `Array`, AVL `Map` | 274 to 275 MB | 329 to 333 MB | 45 to 46 s | 586.0 G |
| C++ prismc, self: a view over 64-slot chunks | 996 MB | 1,168 to 1,174 MB | 61 to 64 s | 725.6 G |
| C++ prismc, self: the view, first chunks 1 to 64 slots | 293 MB | 395 MB | 62 s | 750.1 G |
| C++ prismc, self: tree of chunks `Array` | 310 to 324 MB | 382 to 403 MB | 65 to 68 s | 737.8 G |
| Rust prismc, chess: `im_rc` / AVL `Map` / tree of chunks | 146 / 144 / 21 MB | 159 / 157 / 33 MB | 1.2 / 1.0 / 0.8 s | 12.4 / 12.0 / 10.0 G |
| C++ prismc, chess: view / view, small chunks / tree of chunks | 71 / 25 / 27 MB | 96 / 50 / 52 MB | 1.1 / 1.1 / 1.2 s | 13.0 / 12.9 / 13.3 G |

- **Rust:** the self-compile's footprint falls tenfold (2.9 GB to 275 MB) and its
  instructions by 19%; the RSS measured between the AVL `Map` and the tree of chunks moved
  with memory compression under the other jobs.
- **C++:** the footprint falls by two thirds (996 MB to about 310 MB). The view over a
  shared buffer was cheaper to run, 726 G instructions against 738 G (2% more): an index
  in the tree descends it, and every array reaches its tail through a node of its own,
  where the view broke Appendix B's `Append` and `Push` promises (a push onto any view but
  the newest, and a concatenation, copied the array). Each of those is now kept, in both
  runtimes, by the same design.
- **The two targets** now measure the same collections: the Rust prismc does the
  self-compile in fewer instructions than the C++ one (586 G against 738 G) and less
  memory (275 MB against about 310 MB).
- **Emitted code** is unchanged by the runtimes' collections (the methods emitted code
  calls kept their names); the C++ prelude's `union_` changed (`Map::united`, the smaller
  map into the larger), and both seeds with it.

### Not lowered yet

Each is refused loudly: lowering emits a `compile_error!("prismc does not lower ..")`
naming the construct, and `CompileProgram` reports every distinct one as
`error[not-lowered]` and fails, the crate still written for reading:
`@HostList`; a named widget function as a host's supplier (gap NS: a widget function is
a node, not a value; a lambda supplier is lowered); a supplier's answer `@ x` outside a tag arm, a dynamic answer over what has
no value at the owner's build, a widget-function blueprint as an answer, and a bundle in
a hosted scope (pick HO); an element applied as a plain value and a live element's `$` field read in a plain
function (gap LE), `@Flatten` of a dynamic that is not a `rebuild`'s,
`!Updated` of a dynamic no sink of the node writes (pick UP), and in a row over a
structural dynamic of its block (gap SU), an `@Machine` whose step is
not a named function, a local function whose
parameters are not names or used as a value, `!MergeAll` of a list that is not a
literal, lifted functions with time parameters (`fn !Bump $n, !go`), building `match`
and `if`, a widget of several clauses in a static position of a hosted scope (pick CS),
`Time.After`, and the zombie and
dead states (`leave`, `clean_up`, `edges`).

## Doubts

- **TC** `<@` calls in `inline_text` go past Templates ch. 2, which gives the function form
  no call form ("a nested template is a function called in a placeholder"); the task asked
  for `<@` calls. Jimmy rules which one stands; the templates use one (`Quoted`, in
  every doc comment that quotes Prism), and the back end lowers `<@Name a={{x}}>body</@Name>`
  to `name(&x, &body)`, so dropping TC costs one line.

- **PF** reads less like the books' recursive descent than a language with exceptions
  would; every step after a failure still runs, as a no-op. If Prism gains early exit from a
  step sequence (a `given:` that can carry a payload out), the bad cursor goes.
- The front end matches `prism_syntax`, not the books directly: a disagreement between
  `prism_syntax` and a book (its API.md lists them) is reproduced here.
