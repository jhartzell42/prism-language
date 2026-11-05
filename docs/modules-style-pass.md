# Modules as directories, and the style pass

*Written by Claude on the claude-self-hosting branch; subject to change. Jimmy's
direction of 4 November 2026 is marked Jimmy's; every other call is Claude's, with the
alternative refused, and Jimmy overrules by editing. It comes after the performance queue
and before the regex code (docs/handoff.md, "The order"). Book citations are to
`prism-notes/claude/prism/`: Semantics (01), Type System (02), Conformance (04),
Platform (08).*

## Jimmy's direction

- A module divided into sections is separate files.
- More hierarchy in the directory and module tree.
- Top-level items that exist only for testability move into a declared test module beside
  the module, which only `prism test` and `prism check` read.
- Every module's main file is `mod.prism` (Rust's rule); `lib.prism`, `mod.prism` or
  `app.prism` declares the submodules, and `app.prism` the main widget.

## The layout

- **A module is a directory whose main file is `mod.prism`.** Its sections are sibling
  `.prism` files; its submodules are subdirectories, each with its own `mod.prism`.
- **A tree's root is `app.prism` (a program) or `lib.prism` (a library)**, never both in
  one directory.
  - `app.prism` is the root file of Semantics ch. 6 ("The root, and what a set of files
    means"): no namespace, the implied `emit Element`, and `@main` (the main widget),
    defined there or bound there by `let @main = @Board.Game`. It may have sections and a
    `tests.prism` like any module.
  - `lib.prism` holds only `mod` lines: the library's top-level modules. *Refused:*
    `lib.prism` as the namespace of a module named after its directory (Rust's crate
    root); Prism has no crates yet (Semantics ch. 6 Q-C1, Appendix C), and the prelude's
    `Ui` would become `Prelude.Ui`.
- **The header of a main file is `emit`, then `mod` lines, then `use` lines.** A header
  says what before from where (Semantics ch. 6, "A file is a module": "`emit` comes before
  `use`"), and what a module is made of is a what.
- **`mod name` declares a section, `mod Name` a submodule** (Claude's). The keyword is the
  operator `mod`, already reserved (Semantics App. A.2); case is the mark, as everywhere
  else in Prism (App. A.2, "Case is a mark").
  - `mod compile` reads `compile.prism` beside the main file, in the same namespace.
  - `mod Expr` reads `expr/mod.prism`; the directory is the name in snake case
    (`CheckEnv` is `check_env/`).
  - A section's name is a lowercase name and so never a keyword: no `for.prism` or
    `check.prism` can be declared; a test module is `test mod tests`, below.
  - `mod expr` and `mod Expr` in one header are refused (`mod-clash`).
  - *Refused:* a second keyword (`section expr`) beside `mod Expr`, a word more for the same
    line; the file list taken from the directory with no `mod` lines, under which a file
    existing changes the program (Semantics ch. 6, M5) and a module's main file no longer
    says what the module is.
- **Every `.prism` file of a module directory is declared**, by `mod` or `test mod`
  (`mod-stray`, `mod-missing`); other files (chess's `pieces/`) are not looked at.
- **A section has no header.** No `module`, `emit`, `use` or `mod` line (`section-header`);
  the main file's header is the whole module's. *Refused:* per-section `use` lines (Rust's
  per-file `use`): one namespace would get several bare-name tables, and a line's meaning
  would depend on which file of the module it sits in (Semantics ch. 6, M3).
- **A main file has no `module` line**; the path names it (`module-line`). *Refused:*
  the line kept and checked against the path, a line that can only be wrong.

## Names, `use`, rows, visibility

- **A submodule's name is the dotted path from the root:** `lower/node/mod.prism` is
  `Lower.Node`; the root's children are top-level (`Fs`, `Lower`). Dotted names are
  already the language's (Semantics ch. 6: "several joined by dots (`Birds.Field`)"); the
  path now gives them.
- **A qualified name's last segment is the declaration, the rest its module:**
  `Check.Env.BuildG` is `BuildG` of `Check.Env`, one token (Semantics ch. 2, "Qualified
  names and families"). A `use` names a module only, so `use Check.Env` is never a type.
- **`use` takes the full path,** `use Lower.Model.*`, from anywhere. *Refused:* relative
  paths (`use Model` inside `Lower`, Rust's `self::`/`super::`): two spellings of one
  module, and a child's name would shadow a top-level one (M3).
- **A section is the module.** It sees the module's declarations bare and declares into
  its namespace; every rule stated per module (one owner per name, bare-name collisions,
  the orphan rule, a family's tags in one module) is per module still, across its sections
  (Semantics ch. 6, "Every name has one owner"; Type System ch. 2, "Every tag of a family
  is declared in its family's module").
- **A submodule is a module like any other:** parent and siblings only through `use`,
  mutual `use` allowed, as today's graph allows cycles (prismc/README.md, MH3).
- **A submodule writes its own `emit` row** (Semantics ch. 6, "Every library writes
  `emit`"); its parent's row neither bounds nor supplies it. A section has the module's
  row; a widget block in it may still set its own (Semantics ch. 8).
- **A family is its declaring module's** (Semantics ch. 6 doubt FM); a submodule cannot
  add tags to its parent's. **Everything is public, as today** (ch. 6, Negative space;
  Q-M3): hierarchy organises, it does not hide.

## Test modules

- **A test module is declared, as a test module** (Jimmy, 4 November: "test modules should
  be declared... declared as test modules, and so are only incorporated in `prism test`"):
  `test mod tests` in the header of `mod.prism` or `app.prism` declares `tests.prism` as a
  test module of that module. Any lowercase name; more than one is allowed; none beside
  `lib.prism`. `prism test` and `prism check` read them; `prism run` and `prism compile`
  never do, so the program's files hold the program and nothing is stripped (today's pick
  PT strips `test` declarations before checking). *Claude's spelling:* `test mod name`, the
  two keywords in the order a reader says them; refused: a file named `tests.prism` found
  by convention and never declared, which Jimmy refused, and a `tests/` subdirectory, which
  would make a test module a submodule with a namespace of its own.
- **It is a section of its module:** no header, the module's names bare; its `test` and
  `check` declarations (Conformance ch. 1 and App. A, what a `.prismtest` holds today),
  and the helpers they need.
- **In a tree, `test` and `check` stand only in a test module** (`test-outside`).
  *Refused:* tests anywhere, as in a loose file; the helpers would still need a home, and
  the program files would still carry tests.
- **What it may declare:** functions, values, widgets (a mock host document, Semantics
  ch. 6, "`test` and `check` are declarations"), types with their `derive`s. No tag,
  trait or other impl (`test-declares`).
- **Nothing outside a test module names what one declares** (`test-only-reached`); a test
  module may name another module's test helpers (shared fixtures). With the two rules,
  dropping every test module changes no other declaration's meaning: M6 ("The program you
  test is the program you run") holds as Semantics ch. 6 argues it for tests.
  *Refused:* helpers reachable only from their own module's test modules; chess's board
  tests would rebuild the rules' positions.
- **Its transcripts are `NAME.expected` beside it** (`tests.expected`), compared per test module.
- **The items moved, found by a scan of every top-level name for uses outside tests:**
  `tests/prismc/lexer-checks.prism`'s `Toks`, `Errs` and `parser-checks.prism`'s `Dump`,
  `Errors` (root files that exist only for those checks), and chess `rules.prism`'s whole
  section "For the checks" (`Setup`, `wk` `wr` `wn` `wp` `bk` `br`, `kingsAndRooks`,
  `CastleOk`, `?Then`, `?kingWalk`, `?foolsMate`, `?passing`). prismc's own modules have
  none; window's `Anchor`, `Aspect`, `Backed`, `Turned` are unused API, not test helpers.

## What stays loose

- **Files on the command line keep Semantics ch. 6's rule:** one file with no `module`
  line is the program, the rest are one-file modules by their `module` lines, `test` and
  `check` inline (Platform ch. 1, "Several files on the command line"). The books'
  examples and `tests/checker/` are loose files. *Refused:* trees only, every one-file
  example a directory.
- **A directory argument is a tree**, no longer "every `.prism` under it": its
  `app.prism`, `lib.prism`, or `mod.prism` (that module, named by the nearest ancestor
  root, with the modules it reaches by `use`).
- **`.prismtest` goes**, with the join of pick PT: in a tree its role is `tests.prism`, in a
  loose file the declarations are inline.

## The tree for this repository

Line counts are the existing section headings' spans. Lowercase files are sections,
capitalised modules directories.

```
prismc/app.prism         root: the driver, @main, @FrontMain (~250)     front.prism
  checking.prism         --check and --typed (~110)
  compile.prism          compile FILES|DIR (185)
  fs/mod.prism           Fs, replacing the prelude's (47)               fs.prism
  syntax/mod.prism       Syntax, the tree (~320) + decls.prism (224)    ast.prism
  syntax/lexer/          Syntax.Lexer (1914)                            lexer.prism
    mod (tokens, state, kinds, pushing 265), chars (279), layout (streams, layout 277),
    words (tokens, sigils 355), literals (+ the test path 250), templates (333),
    corpus (notation, escapes 135), tests.prism (lexer-checks)
  syntax/parser/         Syntax.Parser (2419)                           parser.prism
    mod (cursor, names 442), decls (403), expr (704), blocks (190),
    types (+ patterns 373), test_language (269), tests.prism (parser-checks)
  syntax/sexp/mod.prism  Syntax.Sexp (407)                              sexp.prism
  check/mod.prism        Check: CheckProgram, the whole checker (218)   check-passes "The program"
  check/errors/          Check.Errors (119)                             check-errors.prism
  check/types/           Check.Types (1129)                             check-types.prism
    mod (types, helpers, fresh, stacks, printing ~306), recording (189), unify (252),
    schemes (156), colors (211)
  check/env/             Check.Env (1455)                               check-env.prism
    mod (holds, building 231), names (passes 1-2, resolution 239), reading (374),
    decls (third pass 291), traits (coherence, resolution 303)
  check/expr/            Check.Expr (1816)                              check-expr.prism
    mod (inference, names, literals 257), operators (+ fields 205), calls (+ lambdas 521),
    patterns (202), lifting (191), lets (blocks, clauses, templates 367)
  check/blocks/          Check.Blocks (657): mod (widget blocks 278), structure
                         (rebuild, for, given 170), borrows (129)       check-blocks.prism
  check/passes/          Check.Passes: mod (exhaustiveness, inclusion 363), cycles (234),
                         borrows (163)                                  check-passes.prism
  check/shrink/          Check.Shrink (1702): mod (78), types (NT 263), calls (CT 172),
                         locals (LF 246), relations (153), walk (421), summaries (369)
  check/typed/           Check.Typed: mod (140), print (211)            typed.prism
  lower/                 Lower (~3,800)                                 lower.prism
    mod (context, names 216), types (+ values 346), expr (literals, plain 1207),
    patterns (311), templates (+ given, update 338), functions (+ traits 271),
    jobs (556), program (558)
  lower/model/mod.prism  Lower.Model, the core form (507)
  lower/node/            Lower.Node (2987): mod (widget blocks 337), def (398),
    elements (236), static (static widgets, hosting 451), dynamics (242),
    finish (702), keyed (for 227), switch (rebuild, @Switch 394)
  emit/mod.prism         Emit: mod lines only
  emit/rust/             Emit.Rust (664): mod (types, expressions, patterns 345),
                         items (+ nodes, the crate 302)                 emit.prism
  emit/rust/templates/   Emit.Rust.Templates (624): mod (comments, files 164), items (449)
  emit/rust/prelude/     Emit.Rust.Prelude (988, one file)              templates/prelude.prism
  emit/cpp/              Emit.Cpp (1850): mod (lookups, types 260), expr (493),
    patterns (207), stmts (251), items (357), program (251)             emit_cpp.prism
  emit/cpp/templates/    Emit.Cpp.Templates (618): mod (files 345), items (262)
  emit/cpp/prelude/      Emit.Cpp.Prelude (888, one file)               templates/cpp_prelude.prism
prelude/lib.prism        mod Prelude, Ui, Gui, Fs, Process, Time, Test
  prelude/prelude/mod.prism (282), ui/, gui/, fs/, process/, time/, test/ (each mod.prism)
examples/chess/app.prism the board, @$main (85); tests.prism, tests.expected
  rules/                 Rules (was Chess): mod (helpers, position, move ~265);
                         tests.prism (rules.prismtest + "For the checks")
  window/                Window (917): mod (geometry, scene, drawing tags ~130), ui (81),
                         hosting (47), widgets (106), layout (426), hits (116)
bench/gui/prism/app.prism  bench.prism, compiled with examples/chess/window
tests/conformance/NAME/  app.prism, tests.prism, tests.expected (88 programs);
                         birds/, left/ right/, aloud/ zoo/ as submodules
```

- *Refused:* keeping today's top-level names (`CheckEnv`, `Ast`, `EmitCpp`); Jimmy asked
  for hierarchy, and the renames are mechanical. `Chess` becomes `Rules` so the module is
  not `chess/chess/`. A heading whose span is under ~150 lines shares a neighbour's file.
  *Refused:* a file per heading (the lexer's 43-line test path).

## What changes where

- **Parser** (`Syntax.Parser`, decls): the `mod` line in the header, `Module.mods`
  (name, case, range); Sexp prints `(mod expr)`. App. A.6's `header` gains it.
- **Front end** (`compile.prism`, `checking.prism`): a directory argument read as a tree:
  `Fs.List` the directories as today, find the root, give each file its module name and
  role (main, section, test, root, lib) from its path, check `mod` lines against the
  listing (`mod-missing`, `mod-stray`), and pass test modules only under `--tests`.
  `preludeFiles` gives way to `--prelude DIR`'s `lib.prism`. *Refused:* reading only what
  `mod` lines name, one tick per level and stray files unseen.
- **Checker** (`Check.Env`): `ModInfo.file: Nat` becomes `files: [Nat]` (main, sections in
  `mod` order, test); `AddModule` merges a section into its module where it now reports
  "two files declare `module N`"; the header comes from the main file only. New
  diagnostics: `mod-clash`, `mod-stray`, `mod-missing`, `section-header`, `module-line`,
  `test-outside`, `test-declares`, `test-only-reached`.
- **Lowering and Rust:** `RustModName` becomes a path, `Check.Env` is
  `crate::check::env`, the module's file `src/check/env/mod.rs`, declared by `pub mod
  env;` in `src/check/mod.rs`; the root stays `src/program.rs`. Sections print into their
  module's one file in `mod` order, each after a `// check/env/traits.prism` line.
  *Refused:* today's flat snake names, under which `CheckEnv` and `Check.Env` are both
  `check_env` mid-migration; `include!` per section, whose files are not modules.
- **C++:** nested namespaces (`namespace check::env`), every reference fully qualified
  (`::check::env::BuildG`) so an inner namespace never hides an outer one (the `aloud`
  case of MH, made general); headers `src/check/env/{decl,types,module,generic}.hpp`;
  one `.cpp` per section (`src/lower/node/finish.cpp`), SF's 250,000-character chunks only
  past that; the Makefile's sources and object directories follow.
- **The `prism` script:** a directory goes to prismc as is (no `find`, no `.prismtest`
  pairing); the name is the directory's; `test` passes `--tests`.
- **Runners:** `tests/compiled/run.sh` runs `tests/conformance/*/`, `examples/chess`,
  `examples/chess/rules`, `prismc/syntax/lexer`, `prismc/syntax/parser`;
  `tests/checker/run.sh` takes a directory under `pos/` or `neg/` as one program, its
  `.expected` beside it.
- **Bootstrap:** the scripts already pass `prismc` and `--prelude prelude` as
  directories; the seeds' file lists (`src/*.rs`, `src/*.cpp`) become nested trees, and the
  tar keeps `src` whole. A seed must accept the tree before prismc's sources use it.
- **Docs:** prismc/README.md (Files, PT, MO, MH, SF, FL), the examples', tests' and
  prelude's READMEs, architecture.md's tree; a decision entry when it lands.

## The books' sentences that change

- Semantics ch. 6, "A file is a module": "**One file is one module.** Its header is the
  `module` line, then the `emit` line, then the `use` lines" becomes: a module is one
  file, or a directory whose `mod.prism` heads it and whose `mod` lines name its sections
  and submodules; the header is `module` (loose files only), `emit`, `mod`, `use`.
- Same section: "**`module Birds`** names the file's namespace" and "**A file with no
  `module` line is a root file.**" gain: in a tree the path names it, and the root is
  `app.prism`.
- Ch. 6, "The root, and what a set of files means": "**Exactly one file has no `module`
  line, and it is the program.**" and "**Every other file is a module named by its
  `module` line**" hold for loose files; a tree's rule is added beside them.
- Ch. 6, "`test` and `check` are declarations": "A test declares no type, tag or impl, so
  ignoring it changes no other declaration's meaning" gains `tests.prism`'s two rules.
- Ch. 6, Negative space: "**No dynamic loading, no test-only code.**" becomes: test-only
  code only in `tests.prism`, which nothing outside one names.
- Ch. 6, "The declaration forms": rows for `mod name` and `mod Name`.
- App. A.2: `mod` is listed under declarations as well as operators. App. A.6: `header
  ::= ("module" qname SEP)? ("emit" row SEP)? ("use" qname ".*"? SEP)*` gains `("mod"
  name SEP)*` before the `use`s, and a section is `(decl SEP)*`.
- Conformance ch. 1, Principles: "**The test is in the file**, beside the code it tests"
  becomes: in the module, inline in a loose file or in its directory's `tests.prism`.
- Platform ch. 1, "Several files on the command line": "A directory argument means every
  `.prism` file under it" becomes: a directory argument is a tree.
- Unchanged and relied on: Type System ch. 2 and ch. 5 on a family declared whole in one
  module; Semantics ch. 2 on qualified names.

## Failing tests first

- Conformance (`tests/conformance/`, both targets): `mod-sections` (names bare across
  sections, a test in `tests.prism`); `mod-submodules` (`Shapes.Circle.Area` qualified and
  by `use Shapes.Circle.*`, mutual `use`); `mod-test-helpers` (a helper type with `derive`
  and a function in `tests.prism`, used by a `check`); `mod-family-sections` (a family's
  tags over two sections, matched exhaustively in a third); `mod-nested-names` (`A.Types`
  beside a top-level `Types`, for C++ qualification and Rust paths).
- Checker, `neg/`: `mod-clash`, `mod-stray`, `mod-missing`, `section-header`,
  `module-line`, `test-outside`, `test-declares`, `test-only-reached`, `two-roots`
  (`app.prism` and `lib.prism` together). `pos/`: `mod-tree`, `lib-tree`.
- Driver: `--print-world` over a tree whose `app.prism` emits `Gui.*`.

## Migration order

1. The failing tests above.
2. The feature, the old forms accepted beside it: a file with a `module` line in a tree
   directory, undeclared, is an old one-file module, with an `old-layout` warning.
   Bootstrap both targets; update both seeds, so the seed reads trees.
3. prismc, module by module, each step a bootstrap (Rust and C++) with every transcript
   and checker test green: `front.prism` to `app.prism` and its sections; `fs`; `Check`
   and `Check.Errors`, then each `check-*` file and `typed`; `syntax/` with the two
   `tests.prism`s (and `tests/prismc/` removed); `lower/`; `emit/`. Seeds once more.
4. prelude, chess and the bench, then the conformance programs (`git mv` by script; each
   `.expected` becomes `tests.expected`, its text unchanged).
5. The old forms dropped: `old-layout` becomes `mod-stray`, `.prismtest` and pick PT go,
   a directory stops meaning every file under it. Loose files stay (above).

## Doubts

- **MC** `mod name` for a section and `mod Name` for a submodule, told apart by case; or a
  `section` keyword?
- **LO** Loose files kept beside trees for the books and the checker's tests; or trees only?
- **PP** `prelude/prelude/mod.prism` reads twice; or a `lib.prism` that may hold one module?
- **RN** The renames (`Ast` to `Syntax`, `CheckEnv` to `Check.Env`, `Chess` to `Rules`,
  `EmitCpp` to `Emit.Cpp`): worth the churn?
- **TO** `test` and `check` only in test modules within a tree; or anywhere? And the
  spelling `test mod name` (Jimmy ruled that test modules are declared; the spelling is Claude's).
- **TS** A test module naming another module's test helpers; or its own module's only?
- **WL** The window library named across trees by the bench; or its own library,
  `examples/window/lib.prism`?
- **KW** Section files whose natural name is a keyword (`check`, `for`, `rebuild`) take
  another (`checking`, `keyed`, `switch`); acceptable?
- **SJ** (Claude's, in the implementation) A module's files are read as one source, joined
  by newlines, so the checker keeps one `ModInfo.file` per module and `Src.segs` maps an
  offset back to its file and line; or `ModInfo.files: [Nat]` with a file per declaration,
  as "What changes where" has it? Taken: the join (every pass reads `info.file`; the join
  changes none of them).
- **RS** (Claude's) The tree reading (~450 lines) is a root section of its own,
  `reading.prism`, beside `checking` and `compile`; or part of `compile.prism`? Taken: its
  own section.
- **TW** (Claude's) A test module is also the Test World's script, so the Test World passes
  over its helper declarations (a line at the left edge starting `type`, `derive`, `fn`,
  `let` or `Name:`); or prismc writes a script holding only the `test` and `check`
  declarations? Taken: the Test World skips them.
- **CK** (Claude's) `prism check` now reads test modules, so it takes `test` declarations
  and `check` patterns out as a compile does (pick PT); or the checker learns to check a
  test against a `@name:` block with no signature? Taken: taken out; PT's stripping stays,
  only the `.prismtest` join goes.
- **PS** (Claude's) Per-section output (a `// check/env/traits.prism` line per section in
  Rust, a `.cpp` per section in C++) is not done: a module's C++ is chunked by SF as
  before. Open.
