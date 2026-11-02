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

`tests/prismc/lexer-checks.prismtest` and `tests/prismc/parser-checks.prismtest` are
`check`s on small inputs, over the helpers in the `.prism` of their names; they run
compiled with the front end's modules, transcripts byte-equal to their `.expected`:

```
tests/compiled/run.sh lexer-checks parser-checks
./prism test tests/prismc/lexer-checks.prism prismc/ast.prism prismc/lexer.prism
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

- **IB6** (fixed): template attributes bind by parameter name in compiler zero too.
  left={{a}}/>` passes `b` as `left`; the checker reads attributes as named arguments, and
  prismc's lowering binds each to the parameter of its name
  (`tests/conformance/be-template-attrs`, whose `.expected` is written by hand).

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

`tests/prismc/check/run.sh` checks every program of the corpus, the 87 negative cases and
the two typed dumps. A program the books make wrong has its exact report in
`tests/prismc/check/pos/NAME.expected`; each is listed here with the rule.

- **Clean:** `compiler-ch03-halfcounter`, `conf-ch01-counter`, `conf-ch01-journal`,
  `counter`, `half-counter`, `todos`, `sem-appB-widgets`, `sem-ch01-counter`,
  `sem-ch04-birth`, `sem-ch04-given`, `sem-ch04-quiz`, `sem-ch06-birds`, `sem-ch07-births`,
  `sem-ch07-normalize`, `sem-ch07-paged`, `sem-ch07-rounds`, `sem-ch09-hosts`,
  `ts-ch03-traits`, `ts-ch04-lifting`, `prelude`, `chess`.
- **`conf-ch01-motd`** (1): `!motd` holds a `!~Contents`, and a name shows its type's whole
  stack (Type System ch. 1, "The whole stack, every time"; D12). Conformance ch. 1 writes
  `!motd`, Pitch ch. 8 `!~motd` (doubt CF below).
- **`sem-ch04-labels`** (1): the lambda's `left` is a `?Nat`, so its name shows `?` (ch. 1;
  ch. 5, a lambda's marks declare its parameters' layers).
- **`sem-ch05-and-blocks`** (2): `Components`' two `while`s follow parent links, which end
  only by an invariant; the book says itself that `Components` as written is unbounded
  (Semantics ch. 5, "`while` and every tick ends").
- **`sem-ch08-viewer`** (2): the file declares its own `type Key` and `type Contents`,
  which are not `Ui.Key` and `Fs.Contents`, the types `Ui.TextInput` and `Fs.Contents`'s
  tag answer (Type System ch. 2: a declared type is its own; the book's listing has no
  `use Fs.*`, doubt VW below).
- **`prismc`** (clean): the checker over prismc's own 17 files, 17.5 thousand lines, the
  back end included. The back end is in progress (`lower.prism`, `emit.prism`,
  `templates/`, `front.prism`'s `compile` branch), so an error this line shows between
  its chunks is the back end's, not the checker's.

The negative cases (`neg/`, 87) give one diagnostic each, the error their name says, or a
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
  `rebuild` or a `for`, and a delayed binding's arguments. A lambda passed to a call is
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
  the first positional one). Lowering (`Lower.LoTemplateArgs`) and compiler zero read it
  the same way: each place's names from every clause, through parentheses and without
  marks, the body in the last place, attributes in theirs, never the written order
  (`be-template-body`).
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
./prism compile DIR|FILES... -o OUT             # the crate only
./prism test DIR|FILES...                       # compile, build, run the .prismtest scripts
./prism run [--world cli|canvas] DIR|FILES... [-- ARGS...]   # compile, build, run the root
./prism check DIR|FILES...                      # the checker's report
prismc compile FILES|DIR... -o OUT [--name NAME] [--prelude DIR] [--runtime PATH]
OUT/target/.../NAME [--world cli|canvas|test] [ARGS...]      # the root, under the World its row asks for
OUT/target/.../NAME test [--print] [SCRIPTS.prismtest...]    # the Test World; compares with .expected
tests/compiled/run.sh [NAMES...]                # every program, byte-equal
```

`prismc` is `$PRISMC`, or the newest of `target/bootstrap/prismc` and
`target/keep/prismc-*` ("The bootstrap" below); the `prism` script, `tests/compiled/run.sh`
and `tests/prismc/check/run.sh` use it. `prism test` builds debug, `prism run` release;
`prism run` picks the canvas World when the root's row has `Gui.*` and the CLI World
otherwise, and refuses a root whose row has `Ui.*` without `Gui.*`, since no World answers
Ui elements alone. The compiled binary takes `--world` only as its first argument; a later
one is the program's own (prismc's `--check --world Ui`). `make chess` is
`./prism run examples/chess`.

### Files

- `lower.prism` (module `Lower`, 3,700 lines): the core form and lowering. It reads the
  checker's environment (`CheckEnv.BuildG`: modules, types, constructors, tags, traits,
  impls, every definition's clauses) and the typed tables (`TProgram`, indexed by place)
  and decides everything: names, Rust types, borrows and clones, each widget's node (its
  fields, event graph, heights, places, answers, hooks, rows and generations), each
  unbounded function's job.
- `emit.prism` (module `Emit`, 520 lines): the core form spelled; one function per
  construct, nothing decided (K9).
- `templates/rust.prism` (module `RustTemplates`, 495 lines): every item and every
  multi-line expression as a function-form code template with doubled braces;
  placeholders alone on a line indent to their column; doc comments quote the Prism
  source through a `<@TmQuoted text={{..}}/>` call.
- `templates/prelude.prism` (module `PreludeTemplate`, 670 lines): the prelude in Rust,
  emitted as `src/prelude.rs` beside every program (pick PL).
- `fs.prism` (module `Fs`): the prelude's `Fs` with `Fs.List`, which `compile DIR` reads
  (pick FL).
- `front.prism`: `@main`'s `compile` branch (`@CompileMain`, `CompileProgram`).

The emitted crate: `Cargo.toml` (`prism_rtl`, `prism_world_cli`, and
`prism_world_canvas_rtl` behind the `canvas` feature, on by default when the root emits
`Gui`), `src/main.rs` (the programs, the Test World's registrations and compiled `check`
blocks, `main`), `src/prelude.rs`, and `src/NAME.rs` per Prism module.

### Where it stands

End to end (compile, `cargo build`, transcript byte-equal to its `.expected`), by
`tests/compiled/run.sh`, which runs every program every time: 44 of 49 pass.

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
  self-compile needed: `be-let-fn-pair`, `be-unbounded-value`, `be-unbounded-lambda`,
  `be-row-local`, `be-template-attrs`, `be-error-try`, `be-nat-literal`, `be-inference`,
  `be-unbounded-prelude`, `be-local-names`, `be-tuple-show` (`prism_rtl` shows and pods
  tuples up to six), `be-function-values`, `be-lambda-patterns` (a job lambda takes its
  parameters apart by pattern), `be-fold-tuples`, `be-template-body`, `be-closure-names`,
  `be-update-loops` (a `for` and a guarded `match` as statements of an `update`),
  `be-pipe-record` (a `Start`'s function parameters are `Start`s, whatever color a
  `let`-bound lambda's own type gives them), `be-while-guard` (a shrinking `while` is
  one unit of work, gap LP), `be-lambda-cycle` (mutually recursive unbounded lambdas, a
  cycle a call), `be-row-values` (a row kind's values built, compared, shown,
  matched and sent inside an element, pick RV), `be-bundle` (pick BA),
  `be-widget-args` (pick WB).
  `be-clause-widget` fails by gap CW. The rest of the corpus that passes: the conformance
  counters (`conf-ch01-counter`, `sem-ch01-counter`, `compiler-ch03-halfcounter`),
  `conf-ch01-journal`, `todos`, `sem-ch04-birth`, `sem-ch04-given`, `sem-ch04-quiz`,
  `sem-ch06-birds`, `sem-ch07-births`, `sem-ch07-rounds`, `sem-ch07-paged` (gap DY),
  `sem-ch09-hosts` (pick HO), `sem-appB-widgets` (pick MA), `ts-ch03-traits`,
  `ts-ch04-lifting`, chess's `rules.prismtest` (`chess-rules`), and prismc's own
  `lexer-checks` and `parser-checks`.
- **Refused by the checker,** as the books have them wrong (`conf-ch01-motd`,
  `sem-ch04-labels`, `sem-ch05-and-blocks`, `sem-ch08-viewer`): `compile` refuses too.
- **Not lowered yet,** each refused loudly at compile: chess's board, whose window
  library hosts a widget parameter's argument (`@Grid 8, @widget: ..`, `@Window ..,
  @$board`) and takes its result back across the host, answers with event blueprints
  built from the host's own events (`?Boxed`'s `@ !Filter (?ClickTo p, !in)`, a plain
  function with a `!` parameter), and gathers a `for` whose rows give a dynamic
  (`$sizes`, `Gui.Metrics` first values). `ts-ch04-lifting` passes: a function lifted over an event
  runs at its firing, so its `x on !e` is `x`, and an `if` over events gates each branch
  by the condition and merges them.
- **Speed:** the kept prismc compiles a conformance program in under a second, and cargo
  builds it in about 1 s against the shared target directory; the whole of
  `tests/compiled/run.sh` takes about 2.5 minutes. prismc compiles itself in about 4.5
  minutes and cargo builds it in about 2.5; `tests/prismc/check/run.sh` takes about 1.5
  minutes.
- **prismc itself** checks clean under `--check`.

### The bootstrap

- **The kept binary** is a native prismc at a self-hosting fixed point, kept at
  `target/keep/prismc-*` (the newest; `PRISMC=` names another).
- **`tests/bootstrap/bootstrap.sh`** (`make bootstrap`): the kept binary compiles
  `prismc/` (the working tree, templates included) into the crate
  `target/bootstrap/gen1`, cargo builds it, and gen1's binary compiles `prismc/` again into
  `target/bootstrap/gen2`. The two crates must be byte-equal: the compiler compiles itself
  to the crate it was built from. gen1's binary is then installed as
  `target/bootstrap/prismc`, which the scripts pick as the newest.

### Back-end picks

Each is Claude's, for Jimmy to overturn.

- **NM Names.** Rust's conventions from the program's names: `$count` is the field
  `count`, `!inc` the method `inc`, `ParseNat` the function `parse_nat`, `?Reduce`
  `reduce`. Sigils go; when two values of a module share a base (`?ParseNat`,
  `~ParseNat`) the sigiled ones take a suffix (`parse_nat_opt`, `parse_nat_try`); so do
  locals (`?half` is `half_opt` beside `half`; a capitalised local, a function value, is
  `k_fn` beside a plain `k`, `be-closure-names`), and a local named as a function of its
  module or the prelude takes a `_` (`kind_` beside `Kind`'s `kind`, `be-local-names`); a
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
  binder and an equality guard, a number of the scrutinee's type (`0` against a `Nat` is
  `Nat::from(0u32)`, `be-nat-literal`). A type gets `ToPod`/`FromPod` only when nothing
  it holds, through the declarations it names, is a function or a layer. A closure given
  to a prelude function has its parameter types written (`|found: &Option<Nat>, x:
  &Nat|`), as does an `update`'s mutable local, and a job's type parameters are
  `'static` (`be-inference`). A pattern through an `Rc` payload cannot be a Rust
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
  `be-unbounded-lambda` 7 ticks, `be-lambda-cycle` 5). A lambda is unbounded when its
  checked type is, or when its body calls an unbounded function or a `Start` local: the
  checker fixes a lambda's color before its parameters' types are known
  (`be-pipe-record`'s `\|ignore| K v`). A job calls a value now (`call_value`, the callee's first cycle in this
  one); direct-style code runs it to its result (`run_to_end`). A function type whose
  color is not `bounded` (unbounded, or a color variable `u`, D8) is a `Start`, so a
  function with a `u fn` parameter takes one whatever its caller gives: a bounded
  function value is wrapped in a job that answers in its first cycle, a constructor is
  a closure (`Make Circle, n`), a lambda is a job lambda (`be-function-values`). A
  color-polymorphic function (`Twice: u fn Nat, (u fn Nat -> Nat) -> Nat`) has a job
  too. A call that instantiates its color unbounded (an argument at a function parameter
  whose checked type is unbounded) is a cycle point, a frame or `Turn::More` like a named
  unbounded function's (D6 as ruled; `be-function-values` 4 ticks); any other call
  enters it at once (`start_twice(..).cycle()`). Its calls through its closures are
  cycle points as their values decide.
- **D8 The prelude's higher-order functions** given a closure that is not bounded are
  jobs: `fold_job`, `each_job`, `filter_job` and their map variants, which call the
  closure through `call_value`, so each call of a named unbounded function is a cycle
  point (`be-unbounded-prelude`: `Each xs, Double` over three, `4 ticks`). A job makes
  one a frame or a tail call; direct-style code runs it to its result. Given a bounded
  closure they are the plain functions over `&dyn Fn`. Every unbounded function is
  also a direct-style Rust function, for the calls gap DS leaves.
- **ER `Error(..)` where a `~T` is wanted** is `Err(..)`: as an argument (the checker's
  pick ER), in a branch of an `if` whose value is a `~T`, and as the firing of an event
  that feeds a `!~T` (`be-error-try`).
- **RW `for` rows.** A row is a node of its own (`TodosRow`) at `n/[k]`, with `key`, a
  `value` hold for `$_`, and a weak reference to its block; the block's builder (a
  template, `TmForBuild`) tears down left keys, builds new ones and updates kept ones from
  the source after the tick, run at the build and from the build list when a hold the
  source reads is written. A row's event result goes up to a gather in the block, which
  fires `!(K ==> T)` once a tick at a height above the rows'. A row has a field for each
  plain value of its block it reads (a parameter, a plain `let`), which the builder
  passes to its `build`, since a row is built where its block's locals are not in scope.
- **GN `rebuild`.** A generation is a node (`QuizGen`) at `n/#g` with `_` a plain field;
  the builder (`TmGenBuild`) leaves the old one and builds the next when the source's
  stale flag is set; an answer under `n/#g` reaches the current generation only. A
  generation's event result fires the block's event, which is what `@Switch $!x` hears
  (round 46 RD): `$!x <- rebuild` binds as that event. A `rebuild` over a widget of
  several clauses builds the clause whose patterns match, passing each parameter from
  the arm's binders (`Error(message=m, ..)` passes `m`).
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
  A place's answer type is its tag's output as the checker instantiated it at that
  application (`Prelude.Unwrap`'s `!a` is `!unit` there); a type variable left open is
  the answer as it came, a `Pod`.
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
  source's port and `Dynamic::map` subscribes to nothing (`be-row-values`).
  *Refused:* row values as `Pod`s, which made every field access a runtime decode.
- **BA Bundles.** A tag whose output is a tuple of plain parts and one event part
  (`Gui.Window`'s `((WindowId, !WindowInput))`), bound by a tuple pattern: the plain
  parts are the place's first value (one part itself, several a tuple), each bound part
  a field; the event part fires on each answer at the place (`be-bundle`). A `$` part,
  or a second event part, is refused loudly.
- **WB Widget arguments.** A widget parameter (`@child`) binds its argument, a widget
  block (`@widget: ..`) or a widget expression (`@counter`), with the caller's
  environments; applying the parameter lowers the argument there, so its places take
  this node's next positions (transparent, Semantics ch. 8) and each application has
  its own holds (`be-widget-args`). A parameter applied to arguments, or read as a value,
  is refused loudly. *Refused:* the argument as a blueprint value built at run time,
  which a static position never needs.
- **HO Hosts.** `@HostDocument @child, supplier` and `@HostSome` are a host node of
  prism_rtl's (`Host::each`, `Host::some`) at the line's position, holding the supplier
  (a lowered closure or named function over `&Path` and `&Live<crate::Elem>`). The
  child, a named widget, a widget parameter's argument or a block, is inlined into the
  host's owner (the node of the host's line) in a scope of its own: `Wx.scopes` lists the
  node's hosts, a position from `LoScopeBase k` is the child of host `k`'s place, spelled
  under `host_k_path` (the host's child path, `Segment::Host`), so it emits in the host's
  scope (gap SP); its answer arm is `[Segment::Host(h), Segment::Pos(n), ..] if h.id() ==
  self.host_k.id()`, and the host answers a child place through the owner's
  `Node::answer` under that path. The child reads its caller's names as any inlined
  widget does, marks included, and its result is the line's first part (`WrHosted`,
  bound as an event or a dynamic whatever type the checker gave the name). Per child
  place the host builds the supplier's blueprint at `pos/[p]` when the place asks its
  first value or is added, and tears it down when the place leaves. `@ x` as a value is
  `pure_blueprint(Answering::constant(&x))`, `@ !e` `Answering::event`, `@Element e` the
  forwarding `element_blueprint(&e)`, which re-emits the element at `pos/[p]/0` and
  answers the child's place with whatever comes back there (the transparent rule;
  pushes too, when the World gives the place a first value). The child's document is a
  field the block reads as a hold (`HostDoc<crate::Elem>`, before the tick during
  propagation and after it from then on); when it moves the owner is patched and its
  builders run, and the runtime reruns its build, swap, leave and patch walks until
  nothing new is listed. `sem-ch09-hosts`, `be-host-param`.
- **EV2 Event values.** A named event (`!x <- ..`, `let !x = ..`) fires a port of its
  own (`x_v_port`, made in the build before the node so a closure can hold it), and
  `!x` read as a value is `Event::of(&x_v_port)`. An event expression outside the pipe
  that maps it is built by the library's combinators: a lifted call is `Event::map`,
  `!Filter` `filter_some`. A function that gives a blueprint (`?Boxed: fn Path, Element
  o, !Routed -> ?@o`) is not lifted over its `!` parameters: it takes each as an event
  value. `be-event-values`.
- **PU Pushes.** A place whose element answers a dynamic keeps its element
  (`f_element`), registers with `cx.listen_at`, and answers a push or a host's answer by
  path by writing its hold and patching the node.
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

### Contract gaps

Where docs/backend.md is silent or the library lacks what it describes, with the first
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
  `while` inside the cycle (`be-while-guard`, a `Nat` fuel: 3 ticks). *Not yet:* an
  unshrinking `while` nested in an `if`, a `match`, a `for` or another `while` of the
  block, which still turns within the cycle.
- **CW A widget of several clauses in a static position** is a generation built once
  (a `rebuild` of its first argument, which never changes), the clause chosen at the
  build like a `rebuild`'s; its places stand under the generation (`1/#0/0`), where
  Semantics ch. 8 puts an inlined widget's places in its caller's sequence (`1`), as
  compiler zero does, so `be-clause-widget` fails under prismc. *Next:* the clause's
  node at its caller's path, its places numbered from the caller's position, the
  caller forwarding their answers.
- **SH Shadowed runtime names.** A program type named like a `prism_rtl` name the
  emitted code writes bare (`Cx`, `Fault`, `Pod`, `Path`, `Hold`, `Node`) shadows it in
  its module (`use prism_rtl::*` loses to a local item). prismc's `CheckEnv.Cx` does no
  harm, as that module has no nodes. *If otherwise:* such a type takes a suffix in Rust,
  or the templates write every runtime name qualified.
- **PF A port's fault.** `Port::fire` returns `Done`, and an event method returns
  nothing, so a subscriber's fault is dropped. *First ruling:* event methods return
  `()`; faults in rows' subscribers are lost. *If otherwise:* every event method returns
  `Done` and calls `?`.
- **DY Dynamics across nodes.** A row reads its block's events through ports, and its
  block's holds and dynamic `let`s through its parent at its own read point
  (`sem-ch07-paged`'s inner `rebuild $src`). *First ruling:* a row is not marked when
  they move; a generation that reads one is rebuilt only with its parent's. *Next:* the
  contract's `Dynamic` with its update port. A hosted child reads nothing of its host's
  block, and a host's block nothing of its child's but the document.
- **HS Hosting at the boundary of plain data.** The contract's supplier is typed per
  child element (`Fn(Path, ElementOf<c>) -> blueprint`); a supplier's `o` is its own per
  place (rank 2), so here a blueprint answers with an `Answering` (prism_rtl's `host`): a
  constant, an `Event<Pod>` or a `Dynamic<Pod>`, and the host answers its child by path
  through `Node::answer`, as a World does. *First ruling:* plain data at the host
  boundary. *If otherwise:* a typed answer per tag, which `@HostList`'s `^Responds`
  wants anyway.
- **SP The scope rides on the path.** The contract has every widget constructor take a
  `Scope`. Every constructor already takes its path, so a hosted child's paths start
  with `Segment::Host` and `Cx::added`, `changed`, `removed`, `request`, `first_value_at`
  and `listen_at` route by it; rows and generations inside the child inherit the scope
  with no argument. *First ruling:* the path. *If otherwise:* a `scope` parameter on
  every `build`, and a field on every node.
- **LE A live element's `$` field in a plain function.** A plain function is lifted:
  its `$` values are read at the caller's read point, and it has no `cx` to read a live
  element's dynamic at. *First ruling:* refused loudly there; a host's supplier, lowered
  inside its node (`Lx.inNode`), binds the dynamic. An element applied as a plain value
  (not a place, not a supplier's argument) is refused too: only a host makes a live
  element. *If otherwise:* lifted functions take `&Cx`.
- **CL Clause choice.** A `rebuild` or a building `match`/`if` whose arms are widgets of
  several clauses needs the clause chosen at the build; generations do it for one clause
  only.

### Not lowered yet

Each is refused loudly: lowering emits a `compile_error!("prismc does not lower ..")`
naming the construct, and `CompileProgram` reports every distinct one as
`error[not-lowered]` and fails, the crate still written for reading:
`@HostList`; a host whose child is not a named widget without arguments (a widget
parameter's argument, a block), and a hosted child's result bound to a name; an event
blueprint as a value (`@ !e`, the window library's `?Boxed`); an element applied as a
plain value and a live element's `$` field read in a plain function (gap LE), `@Flatten`,
`!Updated`, an `@Machine` whose step is not a named function, a local function whose
parameters are not names or used as a value, `!MergeAll` of a list that is not a
literal, lifted functions with time parameters (`fn !Bump $n, !go`), building `match`
and `if`, a `for` whose rows give a dynamic, `Time.After`, and the zombie and dead
states (`leave`, `clean_up`, `edges`).

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
