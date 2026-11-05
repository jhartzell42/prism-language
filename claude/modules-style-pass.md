# Modules nest, and the style pass

*Written by Claude on the claude-self-hosting branch; subject to change. Jimmy's
direction of 4 November 2026 and his ruling of 5 November are marked Jimmy's; every other
call is Claude's, with the alternative refused, and Jimmy overrules by editing. Book
citations are to `prism-notes/claude/prism/`: Semantics (01), Type System (02),
Conformance (04), Platform (08).*

## Jimmy's direction

- A module divided into sections is separate files.
- More hierarchy in the directory and module tree.
- Top-level items that exist only for testability move into a declared test module beside
  the module, which only `prism test` and `prism check` read.
- Every module's main file is `mod.prism` (Rust's rule); `lib.prism`, `mod.prism` or
  `app.prism` declares the submodules, and `app.prism` the main widget.

## Jimmy's ruling of 5 November: modules nest, and there are no sections

"We are NOT doing a hardline distinction between modules and sections, why can't modules
just nest indefinitely a la Rust." Everything below follows from it.

- **Every `mod Name` line declares a child module**, with its own namespace, named by the
  dotted path (`Check.Env.Traits`). Its file is `name.prism` beside the declaring file, or
  `name/mod.prism` when it has children of its own (Rust's `foo.rs` or `foo/mod.rs`); both
  present is an error. Modules nest to any depth.
- **Every file is one module and has its own header**: `emit`, `mod` lines, `use` lines.
  It reaches its parent, siblings and children only through `use`, by full path.
- **No lowercase `mod name`, no shared namespace across files.** What were sections
  (`reading`, `checking`, `compile` under the root; `expr`, `functions`, ... under `Lower`)
  are child modules. A family's cases are all in one file again.
- **A test module is a child module declared `test mod Tests`** (`tests.prism`,
  `Parent.Tests`), read only by `prism test` and `prism check`, as Rust's
  `#[cfg(test)] mod tests`; it names its parent's declarations through `use Parent.*`.
  The rules on what it may declare and on nothing outside naming it stand.
- **Output follows:** one module is one Rust file and one C++ unit (cut by size only past
  SF's limit).

## The layout

- **A tree's root is `app.prism` (a program) or `lib.prism` (a library)**, never both in
  one directory (`two-roots`).
  - `app.prism` is the root file of Semantics ch. 6 ("The root, and what a set of files
    means"): no namespace, the implied `emit Element`, and `@main`, defined there or bound
    there by `let @main = @Board.Game`.
  - `lib.prism` holds only `mod Name` lines, the library's top-level modules
    (`lib-header`). *Refused:* `lib.prism` as the namespace of a module named after its
    directory (Rust's crate root); Prism has no crates yet (Semantics ch. 6 Q-C1,
    Appendix C), and the prelude's `Ui` would become `Prelude.Ui`.
- **Every file is one module.** Its header is `emit`, then `mod` lines, then `use` lines:
  what before from where (Semantics ch. 6, "A file is a module": "`emit` comes before
  `use`"), and what a module is made of is a what. A file in a tree has no `module` line;
  its path names it (`module-line`). *Refused:* the line kept and checked against the
  path, a line that can only be wrong.
- **`mod Name` declares the child module `Parent.Name`.** Its file is `name.prism` beside
  the declaring file, the name in snake case (`CheckEnv` is `check_env.prism`), or
  `name/mod.prism` when the child has children of its own.
  - Both files there is `mod-both`; neither is `mod-missing`; a `mod` line that names the
    declaring file itself (`mod App` in `app.prism`) is `mod-self`.
  - **A module with children is a directory** (`mod-leaf`, doubt ML): `shapes.prism`
    declaring `mod Square` is refused, with the fix "move shapes.prism to
    shapes/mod.prism". A module whose only child is a test module is a directory too
    (`counter/mod.prism` and `counter/tests.prism`).
  - **A module's name is capitalised** (`mod-lowercase`): `mod shapes` is refused, since
    every `mod` line declares a module with a namespace of its own, and case is the mark,
    as everywhere in Prism (Semantics App. A.2, "Case is a mark"). A capitalised name is
    never a keyword, so any word names a module (`Check`, `For`, `Static`).
  - *Refused:* the file list taken from the directory with no `mod` lines, under which a
    file existing changes the program (Semantics ch. 6, M5) and a module's file no longer
    says what the module is made of.
- **Every `.prism` file of a tree is declared**, by `mod` or `test mod` (`mod-stray`, which
  names the `mod` line that would declare it); other files (chess's `pieces/`) are not
  looked at.

## Names, `use`, rows, visibility

- **A module's name is the dotted path from the root:** `lower/node/finish.prism` is
  `Lower.Node.Finish`; the root's children are top-level (`Fs`, `Lower`). Dotted names are
  already the language's (Semantics ch. 6: "several joined by dots (`Birds.Field`)"); the
  path gives them.
- **A qualified name's last segment is the declaration, the rest its module:**
  `Check.Env.BuildG` is `BuildG` of `Check.Env`, one token (Semantics ch. 2, "Qualified
  names and families"). A `use` names a module only.
- **`use` takes the full path,** `use Lower.Model.*`, from anywhere, the parent and the
  siblings included. *Refused:* relative paths (`use Model` inside `Lower`, Rust's
  `self::`/`super::`): two spellings of one module, and a child's name would shadow a
  top-level one (M3).
- **`App` names the root of a program tree** (doubt RA): a module of the tree reaches
  `app.prism`'s declarations by `use App.*` or `App.Name`, as a Rust module reaches the
  crate root's by `crate::`. `mod App` cannot be the root's child (its file would be
  `app.prism` itself, `mod-self`), so the name is free in every program tree; a module
  named `App` elsewhere (a loose file's `module App`, a library's top-level `App`) is that
  module. A loose root file stays out of reach (Semantics ch. 6: "nothing can `use` it").
- **A module is a module like any other:** its parent and siblings only through `use`,
  mutual `use` allowed, as the graph allows cycles (prismc/README.md, MH3). Two bare names
  that collide are an error at the use, and a module's own declarations do not shadow what
  a `use M.*` brought in (Semantics ch. 6), so a namespace split into a parent and children
  whose files `use` each other keeps every bare name it had.
- **Every module writes its own `emit` row** (Semantics ch. 6, "Every library writes
  `emit`"), a test module too (doubt ER); its parent's row neither bounds nor supplies it.
  A widget block in it may still set its own (Semantics ch. 8).
- **A family is its declaring module's** (Semantics ch. 6 doubt FM): every tag of a family
  is declared in the family's module (Type System ch. 2), so a child module cannot add
  tags to its parent's family (`tests/checker/neg/family-across-modules`), and a family's
  cases are all in one file (doubt FS). **Everything is public** (ch. 6, Negative space;
  Q-M3): hierarchy organises, it does not hide.

## Test modules

- **A test module is a child module declared as a test module** (Jimmy, 4 November: "test
  modules should be declared... declared as test modules, and so are only incorporated in
  `prism test`"; 5 November: a child module): `test mod Tests` in a module's file declares
  `Parent.Tests`, its file `tests.prism` beside it (so the declaring module is
  `app.prism` or a directory's `mod.prism`). Any capitalised name; more than one is
  allowed (`test mod More`); none beside `lib.prism`. A test module's own children are
  test modules too (doubt TC).
- **`prism test` and `prism check` read them; `prism run` and `prism compile` never do**,
  so the program's files hold the program and nothing is stripped. A directory argument
  that is a module of a tree (`prismc/syntax/lexer`) brings its own test modules with it.
- **It reaches its parent through `use`,** `use Counter.*` or, for the root's,
  `use App.*`, with whatever else it names bare (`use Counter.Tests.*` for another test
  module's helpers). It writes its own `emit` line, as any module does.
- **In a tree, `test` and `check` stand only in a test module** (`test-outside`).
  *Refused:* tests anywhere, as in a loose file; the helpers would still need a home, and
  the program files would still carry tests.
- **What it may declare:** functions, values, widgets (a mock host document, Semantics
  ch. 6, "`test` and `check` are declarations"), types with their `derive`s. No tag,
  trait or other impl (`test-declares`, whose fix names the module it tests).
- **Nothing outside a test module names what one declares** (`test-only-reached`); a test
  module may name another module's test helpers (shared fixtures). With the two rules,
  dropping every test module changes no other declaration's meaning: M6 ("The program you
  test is the program you run") holds as Semantics ch. 6 argues it for tests. *Refused:*
  helpers reachable only from their own module's test modules; chess's board tests would
  rebuild the rules' positions.
- **Its transcript is `name.expected` beside it** (`tests.expected` for `tests.prism`),
  compared per test module. A test module is also the Test World's script: the Test
  World passes over its header and its helper declarations (doubt TW).
- **The items moved, found by a scan of every top-level name for uses outside tests:**
  the lexer's `Toks`, `Errs` and the parser's `Dump`, `Errors` (in `Syntax.Lexer.Tests` and
  `Syntax.Parser.Tests`), and chess `Rules`' "For the checks" (`Setup`, `wk` `wr` `wn`
  `wp` `bk` `br`, `kingsAndRooks`, `CastleOk`, `?Then`, `?kingWalk`, `?foolsMate`,
  `?passing`, in `Rules.Tests`). prismc's own modules have none; window's `Anchor`,
  `Aspect`, `Backed`, `Turned` are unused API, not test helpers.

## What stays loose

- **Files on the command line keep Semantics ch. 6's rule:** one file with no `module`
  line is the program, the rest are one-file modules by their `module` lines, `test` and
  `check` inline (Platform ch. 1, "Several files on the command line"). The books'
  examples and `tests/checker/`'s one-file programs are loose files. *Refused:* trees
  only, every one-file example a directory.
- **A directory argument is a tree:** its `app.prism`, `lib.prism`, or `mod.prism` (that
  module, named by the nearest ancestor root, with its test modules and the modules they
  reach by `use`); any other directory is refused (`no-root`).
- **`.prismtest` is gone:** in a tree its role is a test module's, in a loose file the
  declarations are inline (`prismtest`, `mod-stray`).

## Output

- **One module is one Rust file:** `Check.Env.Traits` is `crate::check::env::traits` in
  `src/check/env/traits/mod.rs`, declared by `pub mod traits;` in `src/check/env/mod.rs`;
  the root is `src/program.rs` (doubt RF). A reference to another module is from the crate
  (`crate::check::env::BuildG`), so a child named as a top-level module (`a::types` beside
  `types`) never stands in for it.
- **One module is one C++ unit:** namespace `check::env::traits`, headers
  `src/check/env/traits/{decl,types,module,generic}.hpp`, its source
  `src/check/env/traits/mod.cpp`, cut into more files only past SF's 250,000-character
  chunks. Every reference is qualified from the global namespace (`::check::env::BuildG`).
- **A module's name segment that is a Rust or a C++ keyword takes a `_` in both
  targets** (`Shapes.Static` is `shapes::static_`, `Shapes.Switch` is `shapes::switch_`),
  so the two printers name every module alike.

## The tree for this repository

Line counts are of each file. A name with children is a directory (`name/mod.prism`); the
rest are files (`name.prism`).

```
prismc/app.prism             the root (217)
  check/                   Check (274)
    blocks/                Check.Blocks (390)
      borrows.prism        Check.Blocks.Borrows (172)
      structure.prism      Check.Blocks.Structure (212)
    env/                   Check.Env (287)
      decls.prism          Check.Env.Decls (313)
      names.prism          Check.Env.Names (261)
      reading.prism        Check.Env.Reading (396)
      traits.prism         Check.Env.Traits (326)
    errors.prism           Check.Errors (160)
    expr/                  Check.Expr (365)
      calls.prism          Check.Expr.Calls (563)
      lets.prism           Check.Expr.Lets (410)
      lifting.prism        Check.Expr.Lifting (233)
      operators.prism      Check.Expr.Operators (247)
      patterns.prism       Check.Expr.Patterns (244)
    passes/                Check.Passes (418)
      borrows.prism        Check.Passes.Borrows (207)
      cycles.prism         Check.Passes.Cycles (278)
    shrink/                Check.Shrink (163)
      calls.prism          Check.Shrink.Calls (216)
      locals.prism         Check.Shrink.Locals (290)
      relations.prism      Check.Shrink.Relations (197)
      summaries.prism      Check.Shrink.Summaries (413)
      types.prism          Check.Shrink.Types (307)
      walk.prism           Check.Shrink.Walk (465)
    typed/                 Check.Typed (151)
      print.prism          Check.Typed.Print (232)
    types/                 Check.Types (335)
      colors.prism         Check.Types.Colors (231)
      recording.prism      Check.Types.Recording (208)
      schemes.prism        Check.Types.Schemes (175)
      unify.prism          Check.Types.Unify (271)
  checking.prism           Checking (115)
  compile.prism            Compile (177)
  emit/                    Emit (6)
    cpp/                   Emit.Cpp (329)
      expr.prism           Emit.Cpp.Expr (529)
      items.prism          Emit.Cpp.Items (393)
      patterns.prism       Emit.Cpp.Patterns (265)
      prelude.prism        Emit.Cpp.Prelude (877)
      program.prism        Emit.Cpp.Program (295)
      stmts.prism          Emit.Cpp.Stmts (290)
      templates/           Emit.Cpp.Templates (356)
        items.prism        Emit.Cpp.Templates.Items (268)
    rust/                  Emit.Rust (393)
      items.prism          Emit.Rust.Items (356)
      prelude.prism        Emit.Rust.Prelude (987)
      templates/           Emit.Rust.Templates (175)
        items.prism        Emit.Rust.Templates.Items (461)
  fs.prism                 Fs (46)
  lower/                   Lower (297)
    expr.prism             Lower.Expr (1259)
    functions.prism        Lower.Functions (313)
    jobs.prism             Lower.Jobs (597)
    model.prism            Lower.Model (550)
    node/                  Lower.Node (385)
      def.prism            Lower.Node.Def (439)
      dynamics.prism       Lower.Node.Dynamics (283)
      elements.prism       Lower.Node.Elements (277)
      finish.prism         Lower.Node.Finish (743)
      keyed.prism          Lower.Node.Keyed (268)
      static.prism         Lower.Node.Static (492)
      switch.prism         Lower.Node.Switch (438)
    patterns.prism         Lower.Patterns (352)
    program.prism          Lower.Program (603)
    templates.prism        Lower.Templates (379)
    types.prism            Lower.Types (387)
  reading.prism            Reading (589)
  syntax/                  Syntax (333)
    decls.prism            Syntax.Decls (228)
    lexer/                 Syntax.Lexer (297)
      chars.prism          Syntax.Lexer.Chars (290)
      corpus.prism         Syntax.Lexer.Corpus (147)
      layout.prism         Syntax.Lexer.Layout (288)
      literals.prism       Syntax.Lexer.Literals (260)
      templates.prism      Syntax.Lexer.Templates (344)
      tests.prism          Syntax.Lexer.Tests (39)
      words.prism          Syntax.Lexer.Words (366)
    parser/                Syntax.Parser (497)
      blocks.prism         Syntax.Parser.Blocks (208)
      decls.prism          Syntax.Parser.Decls (440)
      expr.prism           Syntax.Parser.Expr (722)
      test_language.prism  Syntax.Parser.TestLanguage (288)
      tests.prism          Syntax.Parser.Tests (59)
      types.prism          Syntax.Parser.Types (391)
    sexp.prism             Syntax.Sexp (414)
prelude/lib.prism            the root (10)
  fs.prism                 Fs (39)
  gui.prism                Gui (107)
  prelude.prism            Prelude (281)
  process.prism            Process (14)
  test.prism               Test (5)
  time.prism               Time (10)
  ui.prism                 Ui (28)
examples/chess/app.prism     the root (93)
  rules/                   Rules (263)
    tests.prism            Rules.Tests (85)
  tests.prism              Tests (87)
  window/                  Window (128)
    hits.prism             Window.Hits (124)
    hosting.prism          Window.Hosting (55)
    layout.prism           Window.Layout (435)
    ui.prism               Window.Ui (89)
    widgets.prism          Window.Widgets (135)
bench/gui/prism/app.prism    the root (140), compiled with examples/chess/window
tests/conformance/NAME/      app.prism, tests.prism (Tests), tests.expected; child modules
                             (birds/, mod-nested/shapes/, ...) as files or directories
```

- *Refused:* keeping the old top-level names (`CheckEnv`, `Ast`, `EmitCpp`); Jimmy asked
  for hierarchy, and the renames are mechanical. `Chess` is `Rules` so the module is not
  `chess/chess/`.
- A child module that is one part of its parent's work (`Lower.Expr`, `Check.Env.Names`)
  uses its parent, its siblings and every module its parent uses, and the parent uses
  each such child, so the names they share stay bare; every module that uses the parent's
  names bare uses those children's too. A qualified name names the module that declares
  it (`Lower.Program.LowerProgram`, doubt QX).

## Where it is in prismc

- **Parser** (`Syntax.Parser.Decls`): `mod` and `test mod` lines in the header,
  `Module.mods` (name, test, range); Sexp prints `(mod Name)`.
- **Reading** (`Reading`): a directory argument read as a tree: `Fs.List` the directories,
  find the root, walk each file's `mod` lines (`WalkModule`, `Child`), give each module its
  name from its path and its role (`main`, `test`), and refuse what the layout refuses
  before parsing; test modules are kept only under `--tests`. Each module is one file,
  parsed alone.
- **Checker** (`Check.Env`): `App` read as the root's `""` in a `use` line or a qualified
  name (`AppRoot`, `UseName`, `QualModule`); `test-outside` and `test-declares` over the
  parsed trees (`Reading.TreeDeclDiags`), `test-only-reached` over the references
  (`Check.Shrink.TestOnlyReached`).
- **Lowering and both printers:** `RustModName` is the module's path, a keyword segment
  escaped (`ModSegment`); the Rust and C++ files follow it.
- **The Test Worlds** (runtime/rust, runtime/cpp): a script's header lines and helper
  declarations are passed over.
- **Runners:** `tests/compiled/run.sh` runs `tests/conformance/*/`, `examples/chess`,
  `examples/chess/rules`, `prismc/syntax/lexer`, `prismc/syntax/parser`;
  `tests/checker/run.sh` takes a directory under `pos/` or `neg/` as one program, its
  `.expected` beside it.

## The books' sentences that change

- Semantics ch. 6, "A file is a module": "**One file is one module.** Its header is the
  `module` line, then the `emit` line, then the `use` lines" stays, and gains: in a tree a
  file's path names its module, and its header is `emit`, `mod`, `use`; a `mod Name` line
  declares a child module, `name.prism` or `name/mod.prism`.
- Same section: "**`module Birds`** names the file's namespace" gains: in a tree the path
  names it, and the root is `app.prism` or `lib.prism`.
- Same section: "**A file with no `module` line is a root file.** It has no namespace and
  nothing can `use` it" gains: in a program tree, `App` names it.
- Ch. 6, "The root, and what a set of files means": "**Exactly one file has no `module`
  line, and it is the program.**" and "**Every other file is a module named by its
  `module` line**" hold for loose files; a tree's rule is added beside them.
- Ch. 6, "`test` and `check` are declarations": "A test declares no type, tag or impl, so
  ignoring it changes no other declaration's meaning" gains the test module's two rules.
- Ch. 6, Negative space: "**No dynamic loading, no test-only code.**" becomes: test-only
  code only in test modules, which nothing outside one names.
- Ch. 6, "The declaration forms": rows for `mod Name` and `test mod Name`.
- App. A.2: `mod` is listed under declarations as well as operators. App. A.6: `header
  ::= ("module" qname SEP)? ("emit" row SEP)? ("use" qname ".*"? SEP)*` gains `("test"?
  "mod" Name SEP)*` before the `use`s.
- Conformance ch. 1, Principles: "**The test is in the file**, beside the code it tests"
  becomes: inline in a loose file, in a test module beside its module in a tree.
- Platform ch. 1, "Several files on the command line": "A directory argument means every
  `.prism` file under it" becomes: a directory argument is a tree.
- Unchanged and relied on: Type System ch. 2 and ch. 5 on a family declared whole in one
  module; Semantics ch. 2 on qualified names.

## Tests

- Conformance (`tests/conformance/`, both targets): `mod-nested` (files and directories to
  depth 4, mutual `use` both ways, modules named by a Rust and a C++ keyword);
  `mod-test-module` (`test mod Tests` of the root and of `Counter`, each reaching its
  parent by `use`, the root's naming `Counter.Tests`' helper); `mod-test-helpers` (two
  test modules of the root, the second using the first's helpers); `mod-submodules`,
  `mod-nested-names` (`A.Types` beside a top-level `Types`, for C++ qualification and
  Rust paths); every program's `tests.prism` is its root's test module.
- Checker, `neg/`: `mod-lowercase`, `mod-both`, `mod-leaf`, `mod-missing`, `mod-stray`,
  `module-line`, `test-outside`, `test-declares`, `test-only-reached`, `two-roots`,
  `no-root`, `old-module`, `family-across-modules`. `pos/`: `mod-tree`, `lib-tree`.
- Driver: `--print-world` over a tree whose `app.prism` emits `Gui.*` (`gui-tree`, with a
  child module).

## Doubts

- **RA** (Claude's) `App` names a program tree's root, so its test module and its other
  children reach it (`use App.*`), as Rust's `crate::`; or the root stays unreachable and
  its test module sees it some other way (bare, as a section did), which the ruling's
  "no shared namespace" refuses? Taken: `App`.
- **RF** (Claude's) Every Rust module is `src/PATH/mod.rs`, a module of one file too
  (`src/lower/expr/mod.rs`); or `src/lower/expr.rs` for a module with no children,
  mirroring the Prism file? Taken: `mod.rs` throughout, one rule for every module, the C++
  headers' directories the same.
- **ML** (Claude's) A module with children must be `name/mod.prism` (`mod-leaf`); or Rust
  2018's `name.prism` beside `name/child.prism`? Taken: the ruling's directory.
- **TC** (Claude's) A test module's children are test modules; or a test module declares
  no `mod` line? Taken: they are test modules, so nothing a test module brings in reaches
  `prism run`.
- **ER** (Claude's) A test module writes its own `emit` (`emit r` for checks), as every
  module does; or a test module takes its parent's row? Taken: its own, no special case.
- **FS** (Claude's) A family lives in one file, its module's, as the ruling has it; none in
  prismc, chess or the prelude is split, and `neg/family-across-modules` refuses a tag in
  a child module.
- **QX** (Claude's) A qualified name of a declaration in a parent's child names the child
  (`Lower.Program.LowerProgram`); or the parent re-exports its children's names (Rust's
  `pub use`)? Taken: the full path; Prism has no re-export.
- **LO** Loose files kept beside trees for the books and the checker's tests; or trees only?
- **RN** The renames (`Ast` to `Syntax`, `CheckEnv` to `Check.Env`, `Chess` to `Rules`,
  `EmitCpp` to `Emit.Cpp`): worth the churn?
- **TO** `test` and `check` only in test modules within a tree; or anywhere? And the
  spelling `test mod Name` (Jimmy ruled that test modules are declared, and that they are
  child modules; the spelling is Claude's).
- **TS** A test module naming another module's test helpers; or its own module's only?
- **WL** The window library named across trees by the bench; or its own library,
  `examples/window/lib.prism`?
- **TW** (Claude's) A test module is also the Test World's script, so the Test World
  passes over its header and helper declarations (a line at the left edge starting `emit`,
  `mod`, `test mod`, `use`, `type`, `derive`, `fn`, `let` or `Name:`); or prismc writes a
  script holding only the `test` and `check` declarations? Taken: the Test World skips
  them.
- **CK** (Claude's) `prism check` reads test modules, so it takes `test` declarations and
  `check` patterns out as a compile does (pick PT); or the checker learns to check a test
  against a `@name:` block with no signature? Taken: taken out.
- **QP** (Claude's) A check line's case pattern may be qualified (`Sizes.Large 20`); the
  Test World matches its last segment against the value's case, which prints bare. It
  holds no types to check the module path against, and the checker resolves only the
  line's expression (pick PT). Open: a pattern's qualifier is not checked.
