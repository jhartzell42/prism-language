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

## Checker

Type System ch. 5 as a program: the environment of every file's declarations, inference
under each signature with the marks read first (DB1c, D12), rows with families as the
unit, traits with coherence, lifting decided per application, the color, then the
separate passes in the book's order. It reports ch. 5's errors by their names, each with
`file:line:`, the line, carets and the fix.

```
prism_zero run prismc/*.prism prismc/templates/*.prism -- --check FILES... [--out OUT] [--world Ui,Fs] [--prelude DIR]
prism_zero run prismc/*.prism prismc/templates/*.prism -- --typed FILES... [--out OUT]
tests/prismc/check/run.sh [--update] [NAMES...]
```

`--check` writes the diagnostics, then `no errors` or `N errors`, to OUT, and gives up
(exit 1) when there is an error. The prelude's files from `--prelude DIR` join the program
unless it replaces one (a module of the same name, or a tag of the file's family: the chess
window library declares its own `Ui` tags, so `ui.prism` stays out); diagnostics are shown
for the program's own files only. `--world` names the families a World answers, for the
root row check. `--typed` writes the typed program (below), then the diagnostics as `;`
lines.

### Files

- `check-errors.prism` (`CheckErrors`): a diagnostic, a source file, rendering, sorting.
- `check-types.prism` (`CheckTypes`): the three layers of a type (plain, stack, color),
  the inference state, unification with rows (Rémy's algorithm with families for labels,
  lacks sets), schemes, the color's join, and the typed output's recorders.
- `check-env.prism` (`CheckEnv`): modules, bare-name tables (a collision is an error at
  the use), types and kinds, tags, traits, impls, coherence, signatures read as schemes
  (one color variable per closure-taking signature without `$`, `!` or `@`, D8), trait
  resolution with defaulting, waiting field accesses.
- `check-expr.prism` (`CheckExpr`): expressions, the application rule, lifted `match` and
  `if`, patterns and binders, local `let`s and their generalization (DB1a, doubt MR),
  clauses under a signature.
- `check-blocks.prism` (`CheckBlocks`): widget blocks, holds, binds, `rebuild`, `for`,
  `given:`, `&` blocks, and binding groups' dependency order.
- `check-passes.prism` (`CheckPasses`): exhaustiveness and reachability (Maranget's
  usefulness), the prompt-cycle check, `&` borrows and escapes, the shrink check (size-change
  graphs), inclusion and forwarding, the root row; `CheckProgram`, the whole checker.
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
  of what its body applies.
- `methods: [MethodNote]` and `impls: [ImplNote]`: every method call with its trait, and how
  every constraint at that place was solved (`impl FILE:LINE`, `derived FILE:LINE`,
  `builtin`, `given`, `parts`, `packed`, `quantified`).
- `groups: [GroupNote]`: every widget block's and expression block's statements in
  strongly connected components, dependencies first, ties in page order.
- `binders: [BinderNote]` and `defs: [DefNote]`: every binder's type and every top-level
  definition's scheme.

The dump walks the tree, so an expression the checker never annotated prints `unknown`;
run.sh checks that chess's and the conformance counter's dumps have none. A variable left
in a dump (`(var t12)`) is one nothing decides and nothing constrains (a supplier's `o`,
an empty array's element), or one a local `let` quantified.

### Where it stands

`tests/prismc/check/run.sh` checks every program of the corpus, the 51 negative cases and
the two typed dumps. A program the books make wrong has its exact report in
`tests/prismc/check/pos/NAME.expected`; each is listed here with the rule.

- **Clean:** `compiler-ch03-halfcounter`, `conf-ch01-counter`, `conf-ch01-journal`,
  `sem-ch01-counter`, `sem-ch04-birth`, `sem-ch04-given`, `sem-ch04-quiz`, `sem-ch06-birds`,
  `sem-ch07-births`, `sem-ch07-normalize`, `sem-ch07-paged`, `sem-ch07-rounds`,
  `sem-ch09-hosts`, `ts-ch03-traits`, `ts-ch04-lifting`, `examples/counter`,
  `examples/half-counter`, `chess`.
- **`conf-ch01-motd`** (1): `!motd` holds a `!~Contents`, and a name shows its type's whole
  stack (Type System ch. 1, "The whole stack, every time"; D12). Conformance ch. 1 writes
  `!motd`, Pitch ch. 8 `!~motd` (doubt CF below).
- **`sem-appB-widgets`** (1): `Shown !numbered` needs `^Show` of `Click`, which
  `prelude/ui.prism` declares without `derive` ("Nothing is derived without the line",
  Type System ch. 3).
- **`sem-ch04-labels`** (1): the lambda's `left` is a `?Nat`, so its name shows `?` (ch. 1;
  ch. 5, a lambda's marks declare its parameters' layers).
- **`sem-ch05-and-blocks`** (2): `Components`' two `while`s follow parent links, which end
  only by an invariant; the book says itself that `Components` as written is unbounded
  (Semantics ch. 5, "`while` and every tick ends").
- **`sem-ch08-viewer`** (1): the file declares its own `type Contents`, which is not
  `Fs.Contents`, the type `Fs.Contents`'s tag answers (Type System ch. 2: a declared type is
  its own; the book's listing has no `use Fs.*`, doubt VW below).
- **`examples/todos`** (1): `Add "item {$next}"` passes a `$Text` to a constructor, which
  takes every argument whole (Type System ch. 5, the application rule).
- **`prelude`** (2): `gui.prism`'s `Turn` collides with `prelude.prism`'s `Turn b`
  (Semantics ch. 6); `ui.prism`'s `Ui.TextInput` answers `!Key`, and `ui.prism` declares
  no `Key`.
- **`prismc`** (213, the headline: the checker over its own 14 thousand lines, in about
  ten and a half minutes under compiler zero). The checker's own files (`check-*.prism`,
  `typed.prism`, `front.prism`'s `--check` branch) check clean; everything reported is in
  the files written before there was a checker: `lower.prism` 113, `emit.prism` 28,
  `parser.prism` 27, `sexp.prism` 23, `lexer.prism` 19, `front.prism` 2 (the front end
  proper), `templates/rust.prism` 1. By rule:
  - `mark-stack` (168): names and field reads that hide a `?` (`let first = clauses[0]`,
    `match arm.pattern:` for `arm.?pattern`; Type System ch. 1, "The whole stack, every
    time" and "Sigils on fields", 54 of them field reads); `mark-function` (12): functions
    returning `?T` named without `?` (D10).
  - `mismatch` (13): `given:` blocks whose last line is a plain value, not `? v` (Semantics
    ch. 4, "the last line is the answer"); `unit` used as a binder's name in `lower.prism`'s
    duration code (it is the unit value), with `not-exhaustive` (2) following from it.
  - `may-not-end` (11): `while &going:` loops and loops whose measure is a field
    (`LayoutLine`), `HexOf` recursing on a quotient, `SexpType` through `Spaced` (gap CT),
    `ZForceDeps` and `ZDeferredLets` over the program; each needs a measure or
    `unbounded fn` (Semantics ch. 7).
  - `calls-unbounded` (2): `PushOpSuffix` and the `u`-colored `LayoutItems` call unbounded
    functions (ch. 5, The color).
  - `local-escapes` (3): `Buffer.Numbered` used as an array outside an `&` block (Semantics
    ch. 5).
  - `emit-element` (1): `templates/rust.prism` writes no `emit` line (Semantics ch. 6).
  - `ambiguous` (1): `front.prism`'s `@Compile` gives up with an `Error` whose `~T` nothing
    decides (ch. 5, Defaulting).

The negative cases (`neg/`, 51) give one diagnostic each, the error their name says, or a
case of it (`live-field-bare` is a `mark-stack`; `values-not-shrinking` and
`summary-not-parts` are `may-not-end`s that keep doubt TR's map rule and pick SM's
summaries from accepting a cycle that grows). `refinement-escape` and `existential-escape`
have no case, since the checker does not report them (gaps below).

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
  the prelude's delayed ones (`@Defer`, `@Later`, `@Run`, `@Machine`, `@started`).
- **SM A function that returns parts of a parameter has a summary**, Semantics ch. 7's
  size-change principle across the call, so a cycle may traverse what a helper returns.
  A function is summarized on its first parameter `i` for which every clause's result is
  within `i`:
  - no larger than it: the parameter, a part, `Values` or `Keys` of one, an index on one;
  - an array or map literal of parts;
  - a `match` or `if` whose every arm is within `i`;
  - a `Filter`, or an `Each` of parts, over a collection within `i`;
  - a `Fold` over one, from a start within `i`, whose step returns its accumulator or the
    accumulator with parts added by `Push` or `Insert`, or a within added by `Append`.

  A part is strictly smaller than `i`, or a tuple of such (a tuple is not a recursive type,
  so a cycle reaches only its pieces). A call of the function is no larger than its
  argument `i`, and strictly smaller when that argument is. A summary is read without
  other summaries, so none depends on itself or on a chain, and a name the calling clause
  binds itself (a parameter, a pattern, a local `fn`) is not the summarized function
  (`neg/summary-not-parts`: a function returning `[[Node m]]` gets no summary). Chess's
  `Kids cs` is summarized on `cs`, which clears `LayStack`, `LayLayers` and `LayGrid`.
  *Refused:* inlining `Kids` at its call sites, which bends the program to the checker;
  summaries chained to a fixed point, which no program needs yet.

### Checker doubts

- **TR** An element a prelude traversal (`Fold`, `Each`, `Filter`) hands its closure is
  strictly smaller than the collection, for the shrink check, and a named function passed
  as the step is a call on an element. Semantics ch. 7 lists "a part ... taken apart by
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
- **UI** `prelude/ui.prism`'s `Click` derives nothing, so `^Show` of a click fails
  (`sem-appB-widgets`), and its `Ui.TextInput` names a `Key` it does not declare.
  *First ruling (Claude's):* `type Click = click` takes `derive ^Eq, ^Show`, and the tag
  takes `Gui.Key` or a `Key` of its own.
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
  *First ruling:* OutsideIn(X)'s discipline as ch. 5 has it, with the error, next.
- **EE `existential-escape`.** `any e` solves its bounds at `e`'s type; an `any` meets only
  an `any`, and is never unpacked into a rigid variable at a parameter (doubt EP).
  *First ruling:* unpack at application, as ch. 5 says.
- **R2 Suppliers' outputs are not rigid.** A host's supplier lambda gets a flexible `o`, not
  one rigid per arm, so the dumps keep `o` as a variable. *First ruling:* rigid per arm
  with the arm's equality, with RE.
- **DG Binding groups.** Blocks are checked in BG's order, not dependency order, so a `let`
  used above its line is used at its monomorphic pre-bound type, and mutually dependent
  `let`s are not generalized together. *First ruling:* check in the dependency order the
  typed program already computes.
- **CT The shrink check's call test** for closures is not there: a closure passed to a
  program's own higher-order function (`Spaced args, SexpType`) is a call with no
  edges. *First ruling:* the call test, as Semantics ch. 7 has it.
- **SU Prompt-cycle summaries** are per block, not computed bottom-up over the call graph
  (pick PS). *First ruling:* summaries, as ch. 5's pass 1 has them.
- **FM A field read through a waiting access** (pick FD) is not checked for its marks.
  *First ruling:* check them when the record is decided.
- **AT Associated types** other than `Quotient` of the built-in numbers do not normalize.
  *First ruling:* read each impl's associated type into the normalizer.

## Back end

`compile` checks the program, lowers it to a core form and emits a typed Rust crate over
`prism_rtl` through code templates. `docs/backend.md` is the contract: what Rust each
construct becomes and which library call it makes. The old back end, which emitted
`prism_rt` values and applied them through the dynamic layer (D16), is gone.

```
./prism run DIR [ARGS...]                 # compile, cargo build, run the root under its World
./prism test DIR                          # compile, build, run the .prismtest scripts
./prism compile DIR -o OUT                # the crate only
prism_zero run prismc/*.prism prismc/templates/*.prism -- compile FILES|DIR... -o OUT [--name NAME] [--prelude DIR] [--runtime PATH]
OUT/target/.../NAME [--world cli|canvas|test] [ARGS...]      # the root, under the World its row asks for
OUT/target/.../NAME test [--print] [SCRIPTS.prismtest...]    # the Test World; compares with .expected
tests/compiled/run.sh [NAMES...]          # every conformance program and example, byte-equal
```

The `prism` script uses the newest `target/stageN/prismc` and otherwise compiler zero;
`make chess` is `./prism run examples/chess`.

### Files

- `lower.prism` (module `Lower`, 3,450 lines): the core form and lowering. It reads the
  checker's environment (`CheckEnv.BuildG`: modules, types, constructors, tags, traits,
  impls, every definition's clauses) and the typed tables (`TProgram`, indexed by place)
  and decides everything: names, Rust types, borrows and clones, each widget's node (its
  fields, event graph, heights, places, answers, hooks, rows and generations), each
  unbounded function's job.
- `emit.prism` (module `Emit`, 520 lines): the core form spelled; one function per
  construct, nothing decided (K9).
- `templates/rust.prism` (module `RustTemplates`, 490 lines): every item and every
  multi-line expression as a function-form code template with doubled braces;
  placeholders alone on a line indent to their column; doc comments quote the Prism
  source through a `<@TmQuoted text={{..}}/>` call.
- `templates/prelude.prism` (module `PreludeTemplate`, 640 lines): the prelude in Rust,
  emitted as `src/prelude.rs` beside every program (pick PL).
- `fs.prism` (module `Fs`): the prelude's `Fs` with `Fs.List`, which `compile DIR` reads
  (pick FL).
- `front.prism`: `@main`'s `compile` branch (`@CompileMain`, `CompileProgram`).

The emitted crate: `Cargo.toml` (`prism_rtl`, `prism_world_cli`, and
`prism_world_canvas_rtl` behind the `canvas` feature, on by default when the root emits
`Gui`), `src/main.rs` (the programs, the Test World's registrations and compiled `check`
blocks, `main`), `src/prelude.rs`, and `src/NAME.rs` per Prism module.

### Where it stands

End to end under compiler zero (compile, `cargo build`, transcript byte-equal to its
`.expected`), by `tests/compiled/run.sh`:

STATUS_LIST

### Back-end picks

Each is Claude's, for Jimmy to overturn.

- **NM Names.** Rust's conventions from the program's names: `$count` is the field
  `count`, `!inc` the method `inc`, `ParseNat` the function `parse_nat`, `?Reduce`
  `reduce`. Sigils go; when two values of a module share a base (`?ParseNat`,
  `~ParseNat`) the sigiled ones take a suffix (`parse_nat_opt`, `parse_nat_try`). A Rust
  keyword or a name the emitted code uses (`cx`, `node`, `path`, `v`) takes a `_`. A
  merge with no name is `anon`; a node's internal names are first come, first served
  (`run`, `run_2`). A tag's `Level` struct is its family and name run together
  (`UiButton`, as `prism_rtl`'s tests have it). *Refused:* the old back end's sigil
  letters (`ld_count`), which read as a mangling.
- **MO Modules.** One Rust module per Prism module (`program` for the root module); a
  prelude module (`Ui`, `Fs`, ...) is emitted only when a program module uses it or names
  its family on its `emit` line. A reference to another module is `ui::Click`.
- **OW Ownership.** A function takes every argument by reference and returns a value;
  lowering tracks how each Rust place holds its value (`ref`, `own`, `rc`, `rcown`) and
  spells a use as a borrow or a clone accordingly. A closure passed to a parameter is a
  borrowed `&dyn Fn`; a closure as a value is `Rc<dyn Fn(&A) -> B>`, its captures cloned
  in. A recursive payload is behind `Rc` (a type that reaches itself by value, directly,
  through `?` or a tuple).
- **CH Clauses and patterns.** A function's clauses are one `match` over the parameters
  that are not the same name in every clause; a number or text literal in a pattern is a
  binder and an equality guard. A pattern through an `Rc` payload cannot be a Rust
  pattern, so such a `match` is an if-let chain in a labelled block (`'c: { if let .. &&
  let .. = &**f { break 'c ..; } .. }`).
- **GV `given:`.** A labelled block: a bind is `let Some(p) = e else { break 'g None }`,
  a guard `if !(c) { break 'g .. }`; under `~` a failed guard is `Prelude.Unmet` with the
  statement's source text (D11). A function body of binds and guards is a `given:` without
  the word.
- **UP `&` blocks.** `update x:` is a block over `let mut this = x.clone()`: `&_.f <= v`
  is an assignment, `+=` an assignment of the sum, `&m[k] <= v` an `insert`, `&xs[i]` the
  prelude's `at_mut`, `let &i` a `mut` local, `while` a Rust `while`.
- **PL The prelude** is Rust written once in `templates/prelude.prism` and emitted per
  program, not compiled from `prelude/prelude.prism`: most of the prelude is builtins
  with no Prism body, and the Prism-written ones (`Each`, `Filter`, `Or`, `Max`) are a
  line of Rust each. A name serving an array, a text and a map has variants (`len`,
  `len_text`, `len_map`), lowering picking by the argument's type. *Refused:* a crate of
  its own, which `prism_runtime_lib/` (not this task's to change) would have to host.
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
  counts reads through a hold's `then` as edges (gap DG; docs/backend.md cuts them).
- **RN `@Run`** expands to its Appendix B parts in the node: `@started` (it takes a
  number, Semantics ch. 8 RP), an `@Defer`, the merge `!started <| !again`, and the job's
  `Hold`, cycled once per firing.
- **JB Jobs.** An unbounded function that `@Run` is given, and every unbounded function
  it tail-calls, has a job struct of its parameters (`struct Normalize { t: Term }`), its
  `Job::cycle` the function's body with each tail call a `Turn::More(Rc::new(G { .. }))`
  and each other result `Turn::Finished`; `start_normalize` is its `Start`. Every
  unbounded function is also a direct-style Rust function (doubt DS).
- **RW `for` rows.** A row is a node of its own (`TodosRow`) at `n/[k]`, with `key`, a
  `value` hold for `$_`, and a weak reference to its block; the block's builder (a
  template, `TmForBuild`) tears down left keys, builds new ones and updates kept ones from
  the source after the tick, run at the build and from the build list when a hold the
  source reads is written. A row's event result goes up to a gather in the block, which
  fires `!(K ==> T)` once a tick at a height above the rows'.
- **GN `rebuild`.** A generation is a node (`QuizGen`) at `n/#g` with `_` a plain field;
  the builder (`TmGenBuild`) leaves the old one and builds the next when the source's
  stale flag is set; an answer under `n/#g` reaches the current generation only. A
  generation's event result fires the block's event, which is what `@Switch $!x` hears
  (round 46 RD): `$!x <- rebuild` binds as that event.
- **PO Ports.** An event of a block that a row or generation names reaches it through a
  `Port` in the block (`Port::fire` when the event fires) and a subscription the row makes
  when built and drops when it leaves (prelude `subscribe_row`, `Row::leave_row`).
- **WH Wakes by height.** A node with combinators at several heights records the heights
  it asked for (prelude `Wakes`) and its `wake` runs the lowest one asked, since
  `Node::wake` is not told its height (contract gap WH below).
- **PT Tests.** A `.prismtest` is read joined after the `.prism` of its name (a test
  holds declarations of its module and has no `module` line, examples/README.md). `test`
  declarations are taken out before checking (a test's blueprint names a `@name:` block,
  which has no signature for the checker to read) and a `check` line's pattern too (the
  Test World reads patterns from the script; the checker mistypes `Show (? 3) => "? 3"`).
  A test's root is found by name; its arguments arrive as `Pod`s.
- **FV First values.** A place whose element answers a dynamic asks `cx.first_value` at
  the build and fails the build when the World has none; its `Hold` is the place's.
- **TR Traits.** A program's trait is a Rust trait whose methods are associated
  functions over `Self` (`Loud::loud(&x)`), defaults in the trait; an impl is a Rust
  impl. An `impl ^Show` is a function of its own (`show_sq`) and `impl Show` over it,
  and the type then crosses as `Pod::Shown` (docs/backend.md, "Printing"); an `impl ^Eq`
  is `PartialEq` over its `==`.
- **FL `Fs.List`.** prismc's own `fs.prism` replaces the prelude's `Fs` to declare it,
  since a family's tags are declared in the family's module (Semantics ch. 6) and the CLI
  World answers it for the program that declares it (worlds/cli/README.md). Compiler
  zero's World answers no listing, so under compiler zero `compile` takes files and the
  `prism` script lists a directory itself.
- **RT The runtime path.** `Cargo.toml` finds the library and the Worlds at `--runtime
  PATH`, by default one `..` per directory of OUT (OUT under the repository root), so the
  stages emit the same file.
- **SC The code's site.** An inlined widget's code is lowered in its own module's names
  but placed in its caller's module, and a check line's in `main.rs`: lowering keeps the
  module that resolves names apart from the one paths are written from.

### Contract gaps

Where docs/backend.md is silent or the library lacks what it describes, with the first
ruling taken (Claude's).

- **WH `Node::wake` has no height.** The contract has a node run "its combinators in
  distance order, once per height"; the hook is not told the height. *First ruling:* the
  node keeps the heights it asked for (pick WH). *If otherwise:* `Node::wake(height)`.
- **DS Non-tail unbounded calls.** D6 puts a cycle point at every call of an unbounded
  function from an unbounded function; this back end makes one only at tail calls of a
  job, and runs every other unbounded call direct-style inside the cycle (on a 1 GB
  stack). Cycle counts match wherever an unbounded function calls others only in tail
  position (`Normalize`: `3 ticks, 1 to 3`); elsewhere a job takes fewer ticks than D6
  says. *If otherwise:* the job keeps a stack of continuation frames.
- **PF A port's fault.** `Port::fire` returns `Done`, and an event method returns
  nothing, so a subscriber's fault is dropped. *First ruling:* event methods return
  `()`; faults in rows' subscribers are lost. *If otherwise:* every event method returns
  `Done` and calls `?`.
- **TU Tuples beyond pairs** have no `Show` in `prism_rtl` and the orphan rule keeps a
  program from adding one.
- **DY Dynamics across nodes.** A row reads its block's events through ports, but not its
  block's holds or dynamic `let`s (`sem-ch07-paged`'s inner `rebuild $src`): the
  contract's `Dynamic` with its update port is not used yet.
- **CL Clause choice.** A `rebuild` or a building `match`/`if` whose arms are widgets of
  several clauses needs the clause chosen at the build; generations do it for one clause
  only.

### Not lowered yet

Each emits a `todo!` or a `// ...` note naming it, so rustc or the transcript shows where:
hosting (`@HostDocument`, `@HostSome`, `@HostList`, `@Element`), `@Flatten`,
`@Machine`, `@Once` and the prelude's Prism-written widgets, `!MergeAll`, `!Updated`,
local functions, lifted functions with time parameters (`fn !Bump $n, !go`), building
`match` and `if`, a `for` whose rows give a dynamic, pushes to a live place
(`Node::push`), `Time.After`, dynamics read across nodes, and the zombie and dead states
(`leave`, `clean_up`, `edges`).

## Doubts

- **TC** `<@` calls in `inline_text` go past Templates ch. 2, which gives the function form
  no call form ("a nested template is a function called in a placeholder"); the task asked
  for `<@` calls. Jimmy rules which one stands; the templates use one (`TmQuoted`, in
  every doc comment that quotes Prism), and the back end lowers `<@Name a={{x}}>body</@Name>`
  to `name(&x, &body)`, so dropping TC costs one line.

- **PF** reads less like the books' recursive descent than a language with exceptions
  would; every step after a failure still runs, as a no-op. If Prism gains early exit from a
  step sequence (a `given:` that can carry a payload out), the bad cursor goes.
- The front end matches `prism_syntax`, not the books directly: a disagreement between
  `prism_syntax` and a book (its API.md lists them) is reproduced here.
