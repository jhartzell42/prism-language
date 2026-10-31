# `prism_syntax`

*Written by Claude, 31 October 2026, on the `claude-self-hosting` branch. Work in
progress and subject to change: Jimmy rewrites this crate by hand, and an implementer
reading it uses their own discretion. The specification is the books in
`prism-notes/claude/prism/`; where this crate and a book disagree, the book wins and the
disagreement is a bug here, unless it is listed under "Picks" or "Disagreements" below.*

The lexer, parser, tree and printer of Prism, implementing Semantics chapter 2 (lexical
structure), chapters 3 to 6 and Appendix A (syntax), Templates chapter 2 (code
templates), and Conformance chapter 1 and Appendix A (the test language). No
dependencies. Compiler zero (`prism_zero`) parses with it; `prismc` will have its own
parser, in Prism (D7).

## Lexing a file

```rust
let lexed = prism_syntax::lex(src);
for t in &lexed.tokens { /* t.kind, t.span, t.touches_prev, t.line_start */ }
for d in &lexed.diagnostics { /* lexical errors with spans */ }
print!("{}", prism_syntax::corpus::render(&lexed.tokens)); // `open`/`sep`/`close` notation
```

- The stream ends in `Eof` and carries the layout tokens `Layout(Open | Sep | Close)`.
- A name is one token with its attached sigils (`@$?draft`), a qualified name one token
  (`Ui.Button`), a family pattern one token (`Ui.*`).
- Every bracket is its own token with `touches_prev`; the parser pairs them.
- A text literal is one token holding its segments and each placeholder's tokens; a
  template is one token holding its text runs, placeholders, brace blocks and calls,
  after Templates ch. 2's whitespace rules.
- Inside a `test` block, `=>` is a token and a path after `at` or `click element` is one
  `Path` token; inside a `check` block, `=>` is a token.

## Parsing a module

```rust
let parsed = prism_syntax::parse_module(src);
let module: &prism_syntax::ast::Module = &parsed.tree;
parsed.parsed_ok();   // no lexical or parse error (out-of-scope features allowed)
parsed.clean();       // no diagnostic at all
eprintln!("{}", prism_syntax::diag::render_all(src, &parsed.diagnostics)); // `line:col: message`
```

- A diagnostic's `stage` is `Lex`, `Parse` or `OutOfScope`. An out-of-scope diagnostic
  names its feature in `feature`, spelled as decisions.md's list spells it; the tree is
  complete. `features::OUT_OF_SCOPE` lists the features and what the parser detects.
- The parser recovers at statement boundaries, so one file reports every broken statement
  and keeps the rest.
- `parse_fragment(src)` parses a run of statements and declarations at column 0, the way
  the books show a widget block's lines without their widget; statements become
  `DeclKind::Stmt`.

## Parsing a test file

```rust
let parsed = prism_syntax::parse_test_file(src);
for t in &parsed.tree.tests { /* t.name, t.blueprint, t.seed, t.steps */ }
for c in &parsed.tree.checks { /* c.name, c.lines: expr => pattern */ }
let module = &parsed.tree.module; // the declarations the tests see
```

- A test file is a module: tests are declarations of their module and see it (Semantics
  ch. 6), so there is one parser and `parse_test_file` gathers the `test` and `check`
  declarations.
- A step is `withs`, an `action` (`Build`, `Tick`, `Compute { until_idle }`, `Push`,
  `Deliver`) and its `checks`; a delivery targets a pattern, `at` a path, or `element` a
  path.

## Printing

```rust
let text = prism_syntax::pretty::module(&parsed.tree); // parses back to an equal tree
let dump = prism_syntax::sexp::module(&parsed.tree);   // S-expressions, no spans
```

`cargo run -p prism_syntax --example dump -- [--tokens|--pretty|--fragment] FILE` prints
either for a file, then its diagnostics.

## The tree's top types (`ast`)

- `Module`: `name`, `emit: Option<Row>`, `uses`, `decls: Vec<Decl>`, and `template` for a
  template file.
- `Decl` / `DeclKind`: `Signature`, `Fn(FnClause)`, `Let`, `Widget` (`@counter:` and a
  block), `Type(TypeDecl)` with `TypeBody::{Record, Choice, Row, Newtype, Unit, Alias}`,
  `Tag`, `Derive`, `Trait`, `Impl` (items: signatures, `fn` clauses, associated types and
  rows), `Check`, `Test`, and `Stmt` from fragments.
- `Stmt` / `StmtKind`: `Emit`, `Bind` (with `then`: a hold), `Ignore`, `Let`,
  `Signature`, `Fn`, `Widget`, `Set` and `Accum` (`&r <= e`, `&r += e`), `Require` and
  `Forbid` (the condition alone, D15), `While`, `Return`, `Expr`.
- `Block`: statements and whether it was written inline.
- `Expr` / `ExprKind`: names (`SName`, with the sigil stack as written), literals with
  their marks, tuples, arrays, maps, groups (kept), calls (`parens` tells `F(a)` from
  `F a`), binary and unary operators, free sigils, fields (`Field { recv, sigils, name }`:
  `g.?passed` keeps its stack, Type System ch. 1), indexes, topic fields and
  indexes, `.Method`, `<<T>>`, `as`, lambdas, `if` (with `Cond::Bind` for `p <- e`),
  `match` and its arms, `rebuild`, `for`, `given:`, `update`, `@widget:`, templates,
  `any`, lens values, and the out-of-scope forms (monad blocks, suffixed operators,
  absent components, the non-empty mark).
- `Pattern` / `PatternKind`: names, `ignore`, literals, `nil`, `unit`, constructors by
  commas, constructors with named fields and `..`, `? p` and `~ p`, tuples, arrays,
  groups, families.
- `Type` / `TypeKind`: named types with arguments, free-sigiled types, arrays, tuples,
  maps, function types with their `Color` (bounded, `unbounded`, a color variable),
  `emit` clauses, row unions, families, `any ^T`, `unit`, holes.
- `Template` / `TemplatePart`: `Text`, `Placeholder { expr, column }`, `Block`, `Call`.
- Every node has a `span`. `Span`'s `==` is always true, so trees compare without their
  positions; `Span::same_range` compares positions.

## The tests

- `tests/layout_corpus.rs`: ch. 2's corpus as goldens; `01-book-sample` is the book's.
- `tests/book_examples.rs`: every ```prism example of the Semantics, Type System,
  Templates and Conformance books (`tests/book/`, extracted by
  `tests/extract_book_examples.py`), each with its expected outcome in
  `tests/book/EXPECTED`.
- `tests/snapshots.rs`: fourteen trees, checked in as S-expressions.
- `tests/roundtrip.rs`: print then parse gives the same tree, for every example that
  parses, every program in `tests/programs/` (prism-reflex examples translated by hand
  into the books' spelling), and every `.prism` and `.prismtest` file under the
  workspace's `examples/`, read where they live; the examples must also parse with no
  diagnostic.
- `tests/errors.rs`: error positions and messages, recovery, and one case per
  out-of-scope feature.
- `tests/api.rs`: the snippets above.

`UPDATE_GOLDENS=1`, `UPDATE_EXPECTED=1` and `UPDATE_SNAPSHOTS=1` rewrite the checked-in
files.

## Picks

The places the books leave to an implementation, or say two ways, and what this crate
does. Each is Claude's, for Jimmy to overturn.

- **PK Sigiled bound names.** `!key`, `$ix` lex as ordinary names: the bound names are
  plain, so a sigiled one can only be the program's. The books write `!key <-
  Ui.TextInput(...)` in five examples, against "`key` ... keywords".
- **LA `let` annotations.** `let byte: U8 = 255` (ch. 2, Numbers) parses, at the top and
  in blocks, though Appendix A.6's `letdef` has no annotation.
- **FA Applying a field.** `p.run input` applies the field (Type System ch. 3's `Parser`):
  a field's name is lowercase, so this is the only way to call one. Any other lowercase
  name with an argument after it is an error naming the case mark.
- **QL Qualified lowercase names.** `Birds.flock` lexes as field access on `Birds`, by
  ch. 2's rule ("A `.` before a lowercase name is field access"); ch. 6's reach into a
  module's values by `Name.value` is left to name resolution.
- **AT Array types.** Both `[a]` and `[[a]]` are array types (the books write both); the
  tree keeps which.
- **GU Guards' commas.** A guard's condition is a whole expression, so `require Clear g,
  from, corner` gives every comma to `Clear` (D15 and the comma rule); a comma left after
  the condition (`forbid n > 65535, Error(..)`) is an error naming D15.
- **FA Sigiled fields.** `g.?passed`, `input.!changed`, `a.$value.@?b`: a lowercase name
  after a touching `.` is a field with the sigils as written ("A field access shows the
  field's stack", Type System ch. 1). A field with sigils is applied like a bare one (pick
  FA's `p.run input`). The topic's `.f` takes no stack yet.
- **MS Sigiled methods.** `g.?Move a, b` is the `.Method` `?Move g, a, b`: a function's
  name shows its sigil (D10), and the dot keeps it.
- **SB A step's commas stop at a bracket.** Inside a test step no paren-less call takes a
  comma (Conformance A.3), but only at bracket depth zero: in a group, an index or a
  pattern's group the commas are Prism's, so `send p, Press (Point 280.0, 400.0)` and `=>
  fired Press (Point x, y)` give their comma to `Point`.
- **ME `.Method`.** `recv.Method a, b` means `Method recv, a, b`. Touching its receiver it
  is a postfix operator whose arguments take the rest, like a call's; at the start of a
  continuation line it takes everything above it back to the statement's `=`, `<-` or
  `then` (rule 9); spaced on one line it is an error asking for its own line.
- **FS Free sigils before brackets.** A run of sigils before a bracket is several free
  sigils (`@!(Text ==> ?Nat)`); a free `!` or `&` before a bracket lexes, since types need
  `!(K ==> V)`, and is an error in an expression. `!`, `&` followed by a space are lexical
  errors.
- **NE The non-empty mark.** `+` touching any `]` or `}`, with whitespace, a comma, a close
  or the end of the line after it, is `nonempty`: the lexer cannot know which brackets
  double.
- **TF The test frame.** A `test` or `check` keyword beginning a top-level line puts the
  lexer in the test language for that declaration's block. `=>` exists only there; a path
  is read only after `at` or `element` following a delivery word.
- **TW Template whitespace.** The opening tag "ends its line" when a newline follows `>`
  at once. The closing tag's column is its indentation when it begins its line, and 0
  otherwise. A placeholder alone on its line owns that line's indentation (its column),
  its trailing spaces and its newline.
- **RW `raw`.** `raw <lang>` is in scope for any language, Prism's multiline string, with
  no placeholders, calls or escapes; `output` and `inline_text` with `markdown` or `html`
  are the template notation, which is out.
- **TA Call attributes.** A template call's attribute value is `{{expr}}` or a text
  literal.
- **R8 `else` after an inline `if`.** Rule 8 attaches an `else` at the `if`'s column after
  an inline then-block too (`if c: e` above `else: f`).
- **IB Inline blocks.** An inline block ends at `else`, at a layout token or at a closing
  bracket; a continuation line continues it, since layout emits nothing there.
- **TH `then`.** `then` stands only in a bind (the hold); anywhere else it is an error
  naming the hold.
- **TD Type bodies.** `type N = N T` is a newtype when the right side's head is the
  declaration's own name; `type C = c`, a lowercase name that is not a parameter, is a unit
  type; anything else is an alias.
- **WB Where bounds.** `where a: ^Ord, ^Eq, b: ^Show`: after a comma, a trait continues the
  variable's list and `name:` begins the next.
- **CV Colors.** A lowercase name directly before `fn` in a type is a color variable
  (Semantics ch. 7, doubt CV).
- **AS Associated items.** `type Name params (= T)?` and `row Name` stand in trait and
  impl bodies; an operator names a signature only there (`+: fn a, a -> a`).
- **HO Holes.** `_` and `_1` with sigils parse as type holes anywhere; "legal only in an
  `impl` header" is the checker's.
- **WK Sigiled keywords.** `@unit`, `!unit`, `$nil` lex as keywords with sigils; `@widget`
  only with `@`; any other keyword with a sigil is an error.
- **CC The checked cast.** A `?` touching the type after `as` makes the cast checked
  (`as [Text]+?`, doubt PQ).
- **WD Parameterless widgets.** `Name:` followed by an indented block is a widget
  declaration, at the top and inside a block.
- **HK Higher-kinded traits.** A trait counts as higher-kinded when one of its parameters
  is applied to arguments in an item's signature (`f a`).
- **GC A group's comma.** A comma inside a single group is an error asking for a call at
  the group's head or a double bracket, which gives ch. 2's `(((a, b)))` error.
- **DP Decimal points.** `1.` and `.5` are errors; `1..` is a literal and `..`; `1.x` is a
  literal and a field access, which the checker refuses.
- **RB `}` in text.** A bare `}` in a text literal is an error asking for `\}`, since the
  escape list has `\}`.
- **MQ `-?`.** `F -?x` is the checked subtraction by longest match, with an error naming
  `-(?x)` (doubt MQ's "soft error" is reported as an error).
- **FL First line.** A file's first line starts at column 0; the extraction dedents the
  books' indented fragments.
- **CN Corpus notation.** Layout tokens begin new lines; a prefix `[` is not marked (a
  double is a literal whatever its spacing); a prefix `.` prints `.pre`; paths print
  `PATH(...)`, templates `TEMPLATE<lang>(...)`.
- **OS Out-of-scope by name.** `!Align`, `@Collect`, `@View`, `@Edit`, `@Translate`,
  `Over`, `^Monad`, and the type names `U16` to `U128`, `I8` to `I128`, `Instant`
  are flagged wherever they are written; a shadowing program would be flagged wrongly,
  which a checker with scopes can fix.

## Disagreements

- **GV Guards in a `~` block.** Semantics ch. 10 (doubt GV2) gives a guard its `Error` as a
  second argument, `forbid n > 65535, Error(...)`; decisions.md D11 and D15 refuse the
  second argument, and a failed guard raises `Prelude.Unmet` with its text. The tree has
  the one form, and the book's example (`tests/book/sem-ch10-03.prism`) is expected to be
  the D15 error.
- **MF Templates ch. 2's `Map FieldRs c.fields`.** By the books' comma rule ("`F G H x`
  is `F(G(H(x)))`") this is `Map (FieldRs c.fields)`; the parser reads it so. The intended
  reading is `Map c.fields, FieldRs` (as Type System ch. 3 writes `Map xs, Show`).
- **CS The corpus sample's input.** Semantics ch. 2 writes the third input line as `let s
  = F -pre 1, ...`, the notation's spelling; the program is `F -1`, and with that input
  the lexer gives the book's stream exactly.
