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

End to end (compile, `cargo build`, transcript byte-equal to its `.expected`), by
`tests/compiled/run.sh`, whose default set is what passes:

- **Pass (2):** `counter` and `half-counter`, which `prism_rtl`'s hand-written
  `tests/counter.rs` verifies against the same `.expected` files, and whose emitted node
  has its shape (fields `count: Hold<Int>` and the merge's two `Stamped` inputs,
  `inc`/`dec` recording and asking for the wake, `wake` deciding left first, `patch`
  pulling the label; the half counter's `increment` calling `delta` calling the hold's
  write, with no wake).
- **The rest of the corpus** runs by name (`tests/compiled/run.sh sem-ch07-normalize`)
  and is not in the default set until a chunk makes it pass: the lowering above reaches
  `for` rows, `rebuild`, jobs, traits and modules, but nothing past the counters is
  verified against the typed back end. Programs the checker refuses by the books
  (`conf-ch01-motd`, `sem-ch04-labels`, `sem-ch05-and-blocks`, `sem-ch08-viewer`)
  `compile` refuses too.
- **Not yet:** hosting (`sem-ch09-hosts`, chess's window library), `@Machine`, `@Once`,
  `!MergeAll`, local functions (`sem-appB-widgets`), functions lifted over time
  parameters (`ts-ch04-lifting`), a generation reading its block's dynamic
  (`sem-ch07-paged`).
- **Speed:** a native prismc (stage 1) compiles the counter in 3 s and the half counter
  in 2 s; cargo builds each in about 1 s against a shared target directory. Compiler zero
  takes about 7 s and 4 s. prismc's `--check` of its own 16 files takes about 6.5
  minutes natively.
- **prismc itself** checks clean under `--check`.

### The bootstrap

- **Stage 0** is a prismc binary of the old dynamic chain (its back end emitted
  `prism_rt` values applied through `prism_dyn`; its sources are in git history, 2524e50
  and before), kept at `target/keep/prismc-old-chain-1101-0619` (`STAGE0=` names another).
  Compiler zero is not a stage: it is orders of magnitude slower than a native prismc,
  and is the fallback only for a feature stage 0 refuses.
- **Stage 1** (`tests/bootstrap/stage1.sh`): stage 0 compiles prismc's sources, and cargo
  builds the crate (about 3.5 minutes, then 6) into `target/stage1/prismc`, a native
  prismc with the typed back end. Development iterates with it.
- **Stages 2 and 3**: each prismc before compiles the sources with the typed back end;
  `tests/bootstrap/check.sh` wants their crates byte-equal.
- **Stage 0's one quirk met so far:** it resolves a record type's name across modules, so
  `Lower`'s environment record is `LoEnv`, not `Env` (which `CheckExpr` has).

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
