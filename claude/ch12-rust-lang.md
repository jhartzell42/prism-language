# Chapter 12 — The Rust language implementation

A Prism compiler does not emit code that computes. It emits code that, run once, calls the runtime's constructors in the right order with the right captures, and the product of running that code is a graph, which does the computing for the rest of the program's life. Everything in the `prism_lang` crate follows from taking that sentence seriously: the frontend's only job is to decide what every line means and write the decision down in one tree, the typed AST; a lint checks that the tree is consistent after every pass that touches it; and a closure compiler walks the tree once and hands the runtime an `IrWidget`, a value the runtime cannot tell from a widget written in Rust by hand. There is no instruction set, no bytecode, no second syntax and no verifier between two forms, because there is only one form. This chapter is that crate, stage by stage, with its data structures, its invariants, its text form, its diagnostics and its tests, written so that a reader who wants to change a decision can see what the decision costs.

## The thesis

You have written a compiler for a language that runs. Its back half was a loop: fetch an instruction, do it, fetch the next, and every feature you added had to find its place inside that loop, with a frame, a stack and a scheduler to keep it company. Then you wrote a reactive framework on top of a language like that, and the framework turned out to be a second runtime inside the first: its own notion of when things run, its own dirty-tracking, its own idea of what a closure may capture, fighting the host's loop for control of time. The afternoon you spent finding out why a handler ran twice was an afternoon spent at the seam between the two.

Prism has no seam because it has no first runtime. A widget block does not run; it is built. Reading it top to bottom is reading a recipe for a node: these children, these holds, these edges between events, these slots. The code that follows the recipe is ordinary code, it runs once per build, and when it returns the node exists and the recipe is finished with. From then on the runtime of the previous chapter owns time, and the only thing left of the program is a graph of nodes with closures in them, which the runtime calls when an occurrence reaches them. The compiler therefore has two kinds of thing to produce, and only two: *construction code*, which runs once per build and makes builder calls, and *leaves*, pure closures the graph calls at occurrence time, which take values and return a value and cannot build, schedule or keep state.

That split is the whole architecture. Suppose a compiler for this language were written the ordinary way, as a bytecode machine with a dispatch loop. Its loop would run exactly once per build and then sit idle while the runtime propagated, because nothing in the program is a loop; the only code that runs repeatedly is the leaves, and the leaves are reached from the runtime's closures, not from any loop the compiler owns. A VM for a language whose programs run once is a VM that spends its life waiting to be called back. The closure compiler is what that VM becomes when the waiting is removed: each node of the typed AST compiled once into a Rust closure, construction nodes into closures that take the builder, leaves into closures that take values, and the runtime holding the leaves exactly as it holds a hand-written Rust widget's.

### The product is a graph

Three consequences fall out of the thesis, and each one is a thing the compiler can stop worrying about.

*The compiler never decides when anything runs.* The occurrence, the height order, the commit, catch-up for a widget built mid-occurrence: all of that is the runtime's, and a compiled widget gets it by being a widget. A `dyn` block compiled to an `IrWidget` is rebuilt by the runtime's `dynamic_widget` at the right height in the right occurrence, and the block's code does not know whether this is its first build or its hundredth. A leaf does not know what height it runs at. The compiler emits no scheduling code because there is nothing to schedule.

*Construction is a value.* A widget function plus its captures is a thing the runtime can build from at any time: at the first build, at every `dyn` rebuild, at every `for` key's appearance, at every state supply. The compiler therefore produces widgets as values, `IrWidget { program, block, captures }`, and the same value type serves every one of those places. This is why a `dyn` can rebuild into a widget of a different kind, why a supplier is a function from an element to a widget, and why compiled and hand-written widgets sit side by side in one node tree.

*Construction ties knots.* Prism allows recursive bindings in a widget block and the language is strict, so a knot cannot be tied with laziness the way a Haskell `mdo` ties one. The knot is explicit: open a placeholder, build what refers to it, close it to the real node, and never sample it while it is open. The compiler's job is to place the placeholders, and the resolver's dependency analysis says exactly where: at the delayed edges, which every legal cycle has at least one of. Nothing else in the compiler is about recursion.

### What the compiler does not need

The refusals that shrink the runtime shrink the compiler. What is absent, listed because a model handed this chapter will otherwise reach for each of these out of habit:

*No inference over functors.* The sigils say what every name is. `$count + !delta` is an event because its operands say so, and no unification variable ever stands for "some functor". Hindley–Milner runs on the value layer, where it is principal and well understood, and never sees time. The lifting rule is a table over the operands' outermost sigils, which the elaborator looks up; it is not a search through instances.

*No effect system, no monad transformer machinery.* There is one block type per functor and the set is fixed: widget blocks, `dyn`, `for`, `match`. A widget block is a `do` block without the `m`, and the compiler knows which `m` it is by the sigil on the block.

*No reconciliation, no lifecycle analysis.* Construction is ordinary code and teardown is ownership. The compiler emits no diffing, no keys for a reconciler to match on (the keys of a `for` are the program's own), and no mount or unmount hooks, because nothing is triggered by a widget being dropped or an element appearing.

*No escape analysis for nodes.* Nodes are not values. A program cannot name a node, store it, pass it, or reach into it, so the compiler never asks whether a node outlives its scope.

*No exception lowering.* There are no exceptions. A program's own failures are values; a runtime error aborts the occurrence as a transaction, and the compiler's only contribution is the span it attaches to every node so that the abort names a line.

*No code generation in the usual sense.* No register allocation, no instruction selection, no calling convention beyond Rust's own. A plain-value expression compiles to a closure over an environment vector; an occurrence-time expression compiles to the closure that the corresponding graph node will call. The hard problems of a code generator exist in this crate only in the way they exist in any Rust program that builds closures.

*No garbage collector in the output.* Nodes own their closures, closures capture by index into the owning node's environment, and the graph's reverse edges are weak. Closure conversion makes this mechanical, and the lint checks it. A compiler that got this wrong would put a reference cycle on the heap that nothing frees; the rule that prevents it is one rule, stated under closure conversion below.

What is left is three jobs and one intermediate form: decide meaning (elaboration), check the decision is consistent (lint), emit the construction (the closure compiler). The stages before elaboration are the ordinary front of any compiler, and the rest of this chapter is in pipeline order.

## The pipeline

```
source text
   │  lex + layout        tokens with spans; indent/dedent; inline colons as plain tokens
   ▼
token stream
   │  parse               paren-less calls, precedence, blocks, patterns, declarations, tests
   ▼
AST (every node spanned)
   │  resolve             scopes, the pronoun, namespaces, the prelude, SCC groups with edge labels
   ▼
resolved AST
   │  elaborate           HM on values; lifting as explicit nodes; traits; marks; causality; exhaustiveness
   ▼
typed AST  ──── print/parse ────  text form (goldens; the Haskell bridge)
   │  convert             function ids, capture lists, placeholders and closes
   ▼
converted typed AST  ───  lint  (after every pass, in every test)
   │  compile             steps, closures, the tag table, boxing
   ▼
IrWidget  ──────────────────────  the runtime's PolymorphicWidget
```

One crate, `prism_lang`, with no dependency on `prism_runtime` until the last stage, which lives in its own module and is the only module that names a runtime type. The frontend up to the text form can be built, tested and frozen without a runtime, and the Haskell frontend of a later chapter reproduces it byte for byte from the same specification.

```
prism_lang/src/
  lex.rs          Token, Lexer, the spacing rule
  layout.rs       the column stack; indent/dedent/separator tokens
  parse.rs        Parser; ast.rs the parse tree
  resolve.rs      scopes, groups, edge labels
  elab/           types.rs (Type, unification), lift.rs (the sigil table),
                  traits.rs (Eq/Ord/Num), causal.rs (the cycle check),
                  exhaust.rs, mod.rs (the walk); typed.rs the typed AST
  typed/text.rs   the text form: one definition, printer and parser
  convert.rs      closure conversion
  lint.rs         the checker
  ir/             program.rs (Program, Step, Function), compile.rs (the walk),
                  widget.rs (IrWidget), leaf.rs (the evaluator), boxing.rs
  tags.rs         the tag table, shared by elaboration and ir
  diag.rs         Diagnostic, Span, rendering
```

Every stage after the lexer is a function from one tree to the next, and every tree has a span on every node. Spans are the one thing a compiler cannot retrofit cheaply, so they are in `ast.rs` from the first commit and copied forward without exception; a typed-AST node that lacks a span is a lint error, not a convention.

## Lexer and layout

You have debugged a layout-sensitive language by adding a line and watching an unrelated block close three lines down. The reason was always the same: the lexer and the parser disagreed about who owned the decision, and the error pointed at the parser's symptom rather than the lexer's cause. Here the contract is that every layout decision is the lexer's, every colon decision is the parser's, and the type checker never learns that there was a question. The language chapter states the rules; this section is what a lexer that follows them looks like, down to the token type and the algorithm, so that the two implementations' token streams can be diffed against the layout corpus.

### Tokens

```rust
pub struct Token { pub kind: TokenKind, pub span: Span, pub prefix: bool }

pub enum TokenKind {
    // names, with any attached sigils as part of the token
    Lower(Sigils, Name),            // $count, !delta, _, $_, x
    Upper(Sigils, Name),            // @Button, !Leftmost, Insert, Vector
    Qualified(Sigils, Vec<Name>),   // Ui.Button, Fs.Contents, Wrap.Add
    // literals
    Nat(u128),                      // integer-spelled; typed later
    Float(f64),                     // decimal-spelled
    Text(Vec<TextPart>),            // "Count: {$count}": literal and interpolated parts
    Char(char),
    // keywords
    Let, Type, Tag, Of, Fn, Match, If, Else, For, Dyn, Intercept, Widget,
    Record, Choice, Map, Array, Unit, Test, Ignore, On, Then, Key, Ix, Jx, Kx, Empty,
    Not, And, Or,                   // the boolean keywords: not a, a and b, a or b
    // operators and punctuation
    Bind, Define,                   // <-  =      (`then`, the hold operator, is the keyword above)
    Lt, Le, Gt, Ge, EqEq, NotEq,    // <  <=  >  >=  ==  !=   (longest match: `<-` and `<=` are two tokens, `<` `<=`)
    Merge,                          // |   (never the lambda bars)
    Plus, Minus, Star, Slash,       // + - * /     (Plus/Minus take the `prefix` flag)
    PlusInt, MinusInt, StarInt, SlashInt,       // +~ -~ *~ /~
    PlusFloat, MinusFloat, StarFloat, SlashFloat, // +. -. *. /.
    ToNat,                          // +?  (prefix only)
    Arrow, MapArrow, Colon,         // ->  ==>  :   (Colon is the inline colon)
    Comma, Dot, DotDot, Backslash,  // ,  .  ..  \      (Dot takes the `prefix` flag)
    ChainDot,                       // a `.` that is the first token of its line: a chain step
    Semi,                           // ;   separates statements in an inline block
    LambdaOpen, LambdaClose,        // the bars of \|a, b|
    LParen, RParen, LBracket, RBracket, LBrace, RBrace,   // LBracket takes the `prefix` flag
    TupleOpen, TupleClose,          // ((  ))   a tuple, a tuple type, a tuple pattern
    ArrayOpen, ArrayClose,          // [[  ]]   an array literal
    LAngle, RAngle,                 // < >  only inside generic parameter lists
    // free sigils: a sigil not attached to the following token
    FreeSigil(Sigil),
    // layout
    BlockOpen, BlockClose, Sep,     // ⟨ ⟩ and the layout separator of the corpus notation
    Eof,
}

pub struct Sigils(SmallVec<[Sigil; 3]>);  // leftmost outermost: !$ is [Event, Dyn]
pub enum Sigil { Event, Dyn, Widget, Opt }

pub enum TextPart { Lit(String), Interp(Vec<Token>) }   // Interp: the tokens between { and the matching }
```

Five things about this type are decisions rather than transcription.

*Sigils are part of a name token.* `$count` is one token, `Lower([Dyn], "count")`, and the parser never has to glue a sigil to a name. The alternative, a sigil token followed by a name token, would make the attachment rule the parser's and would let a later pass ask whether a space had been there; the language says the space is the whole difference between `$ x` and `$x`, so the lexer answers it once. A sigil that is not attached to a name is `FreeSigil`, and the parser treats it as a prefix operator at the precedence the language chapter gives it. That includes the `$` of `$"Increment"`: a text literal has no sigil in its token, so the `$` before the quote is a `FreeSigil` that makes the literal a constant dynamic.

*The merge bar and the lambda bars are different tokens.* `\|a, b| a | b` lexes as `Backslash LambdaOpen a Comma b LambdaClose a Merge b`. The lexer knows which is which by position: a `|` immediately after `\` is `LambdaOpen`, the next `|` at bracket depth zero relative to it is `LambdaClose`, and every other `|` is `Merge`. Inside the bars only patterns appear, so there is no merge to confuse them with.

*`prefix` is a flag on the token, set by the spacing rule.* `Plus`, `Minus`, `Dot`, `LBracket` and `ToNat` can be prefix or binary, and the lexer decides from whitespace: prefix when preceded by whitespace, an open bracket, a comma, a `\`, or the start of a line, and followed immediately by a non-space; binary otherwise. The parser reads the flag and nothing else. `F -1` is `F Minus(prefix) Nat(1)`; `F - 1` is `F Minus(binary) Nat(1)`; `F-1` is `F Minus(binary) Nat(1)`; `\.x` is `Backslash Dot(prefix) x`; `a.x` is `a Dot(binary) x`; `xs[0]` is `xs LBracket(binary) Nat(0) RBracket`, an index; `\[0]` is a prefix `LBracket`, the pronoun's index. The sigils never take the flag because a sigil is never binary, and `*` and `/` never take it because they are never prefix.

*Doubled brackets are one token each, found by matching.* `((a, b))` is a pair and `[[1, 2, 3]]` an array, while `((Size $elems) + 1)` is two groups and `xs[ys[0]]` is two indexings, so the lexer cannot fuse `((` or `]]` by looking at two characters. It fuses by pairs instead: after the character layer, a pass matches every bracket with its partner, and two opening brackets of one kind with no whitespace between them become one `TupleOpen` or `ArrayOpen` exactly when their two partners are also adjacent, with no whitespace, in the mirrored order. The pass runs outermost first, left to right, and a bracket already fused is not fused again. An opening `[` that follows an expression with no whitespace is an index and never starts a pair, so a map keyed by arrays is read with `m[[[1, 2]]]`: the index bracket, then the array `[[1, 2]]`. So `((G b, c))` is `TupleOpen G b Comma c TupleClose`; `(((G b, c), d))` fuses the outer two and leaves the third a group, which is the pair of `G(b, c)` and `d`; `((F (G x)))` fuses the outer two around a group; `[[]]` is `ArrayOpen ArrayClose`, the empty array; and `((Size $elems) + 1)` fuses nothing, because the inner group's partner is followed by `+`. A doubled group with nothing between the two levels, `((x))`, would mean nothing as two groups, so giving the spelling to tuples costs no program anything; the parser rejects a one-element tuple with "a tuple has two or more elements; to group, write `(x)`". In a type, a fused `[[A]]` is read back as the array of arrays it spells, since a type has no array literals.

*A `.` that starts a line is its own token.* When the first token of a line is a `.` followed by an uppercase name, the lexer emits `ChainDot` instead of `Dot`. Layout has already made such a line a continuation (it is indented more than its block), so the token changes nothing about blocks; it tells the parser that the line is a step of a method chain, which closes the call above it. A spaced `.F` in the middle of a line stays a prefix `Dot`, and the parser gives it the pronoun reading or an error, below. The lexer can decide this from the line structure it already tracks, and the parser then never asks where a line began.

`;` lexes as `Semi` and only as `Semi`. It never becomes a layout `Sep`, because the two end different things: a `Sep` ends a statement of a layout block and with it every inline block open on the line, while a `Semi` separates statements inside the innermost block, inline or layout, and leaves that block open.

A text literal is one token holding a list of parts, and each interpolation part is itself a token stream, lexed with the same lexer in bracket mode (layout suspended) from a `{` up to the matching `}`. Braces inside the interpolation nest, so a map literal or a nested text can sit in it. `\{` and `\}` are literal braces and join the surrounding literal part; a `$` in text is an ordinary character, so `"cost: $5"` has one part and `"$count"` is the six characters it shows. `"Count: {$count}"` is `Text([Lit("Count: "), Interp([Lower([Dyn], "count")])])`. The literal has no sigil of its own: the sigil of the interpolated name is the name token's, and the elaborator, not the lexer, decides that a literal with a dynamic inside is a dynamic.

The corpus prints a text token as `text(` followed by its parts separated by commas: a literal part as the quoted string with `\{` and `\}` replaced by the braces themselves and `\"` kept, and an interpolation as `interp(` followed by the tokens of its expression in the corpus's own notation, then `)`. A text without interpolation has one part, `text("a")`; the empty text is `text("")`; adjacent literal pieces are one part, so `"n={x}, \{ok\}"` is `text("n=", interp(ident(x)), ", {ok}")`, and a leading or trailing interpolation has no empty literal beside it, so `"{x}"` is `text(interp(ident(x)))`. Name tokens print their sigils inside the parentheses, `ident($n)`. This is the one notation for text tokens: the conformance chapter's layout corpus and the Haskell lexer's output use it unchanged.

Names are classified by their first character at lex time, and a name the lexer cannot classify is an error token with the message the language chapter gives, reported against the right span. A lowercase-first name with a capital inside, `isClick`, classifies as a value name and lexes as an ordinary `Lower` token; the lexer attaches a warning (`Level::Warning`, rendered like any diagnostic, below) that says to write `is_click`, and compilation continues. The warning is a lint of the source and the lexer is the only stage that sees the spelling, so it is raised there. A qualified name is one token only when there is no whitespace anywhere in it and every part is uppercase; `Gui.*` is three tokens, `Upper(Gui)`, `Dot(binary)`, `Star`, which the parser recognises as the wildcard pattern.

### The layout algorithm

The lexer runs in two layers: a character layer that produces raw tokens with their line and column, and a layout layer that consumes raw tokens and emits `BlockOpen`, `BlockClose` and `Sep` among them. The layout layer keeps three pieces of state and no more: a stack of block columns, the bracket depth, and whether the last significant token on the current line was a colon.

```
state:
  stack   : [Int]        initially [0]
  depth   : Int          bracket depth, initially 0
  pending : Bool         a line-final ':' is waiting for its block

on each raw line L with first-token column c (blank and comment-only lines are skipped):
  if depth > 0:
      emit L's tokens; continue            # inside brackets nothing below applies
  if pending:
      if c <= top(stack): error "expected an indented block after ':'" at the colon
      push c; emit BlockOpen; pending := false
      emit L's tokens; continue
  if c == top(stack):
      emit Sep; emit L's tokens; continue  # a new statement of the current block
  if c > top(stack):
      emit L's tokens; continue            # a continuation of the current statement
  # c < top(stack): close blocks until something matches
  while c < top(stack):
      pop; emit BlockClose
  if c != top(stack):
      error "unindent does not match any open block" (c, the open columns)
  emit Sep; emit L's tokens

on each raw token t within a line:
  if t is '(' '[' '{' '((' '[[':                         depth += 1   # fused brackets count once; a text part's braces are inside its token
  if t is ')' ']' '}' '))' ']]':                         depth -= 1
  if t is ':' and depth == 0:
      if t is the last token on its line (comments ignored): pending := true   # a layout block
      else: emit Colon                                                         # an inline block
  else if t is '.' and t is the first token of L and the next token is an uppercase name:
      emit ChainDot                                                            # a chain step
  else: emit t                                                                 # ';' is emitted as Semi

at end of file:
  if pending: error "expected an indented block after ':'"
  while stack has more than [0]: pop; emit BlockClose
  emit Eof
```

This is the whole of layout, and each piece of state answers to one rule of the language chapter by its content: the stack is the column rule (a block's column is its first statement's); `pending` is the line-final-colon rule; the three comparisons are the same-column, deeper and shallower cases of the next-line rule; `Colon` is the inline-block rule; `depth` is the brackets-suspend-layout rule; a block in the middle of a statement needs nothing special, because when its `BlockClose` is emitted the next line's column is compared against the enclosing block as usual; `else` is an ordinary token the parser attaches; top level is the initial stack; blank and comment-only lines are skipped before the comparison; end of file closes everything.

Two of the corpus cases are the ones that catch a wrong implementation. A dedent of two levels at once (`@a:` containing `!x <- dyn $b:` containing `@ unit`, then `@c:` at column 0) must emit two `BlockClose` tokens and then a `Sep`, which the loop does because it pops until the column matches and then falls through to the separator. And a `for`-then-merge (`| !other` at column 6 under a block at 8 inside a block at 4) must be an error naming the open columns, which the loop does because after popping to 4 the column is still not equal. An implementation that treats "less than top" as "close one block and continue the statement" gets the second case wrong and silently parses a program the language rejects.

### The inline colon

A colon with tokens after it on the same line is not a layout decision; the lexer emits it as `Colon` and the parser interprets it. The parser has one routine for every place the grammar wants a block:

```
block(ctx):
  if next is BlockOpen: consume; stmts := statements separated by Sep or Semi until BlockClose; consume
                        return Block(stmts)
  if next is Colon:     consume; stmts := [statement(ctx)]
                        while next is Semi: consume; stmts.push(statement(ctx))
                        expect the inline terminator; return Block(stmts)
  error "expected ':' and a block"

inline terminator: a Sep, a BlockClose, an Else, a closing bracket at this depth, or Eof
```

A `Semi` belongs to the innermost block open where it appears. `if c: Log x; Log y` is an `if` whose inline block has two statements, and `type Pair<a, b> = record: first: a; second: b` is a record whose inline block has two fields. A field's `name: type` is not a block: in a `record` or `tag` block the parser reads a statement as a field, and the colon in it as the field's separator, so the `;` after `first: a` belongs to the record. A `match` arm's colon does open a block, so `match x: A: 1; B: 2` puts `B: 2` inside the first arm, where it is an error; a match on one line with several arms is written with layout. A layout block accepts `;` too, as a second way to start a statement on the same line, and never needs it.

The same `Colon` token therefore means a one-statement block after `match` arms, `if c`, `dyn $x`, `for $m`, `@widget`, a `map:` key or a `record:` field, and means a type ascription after a pattern or inside parentheses: `let n: Nat = 0`, `Pair (x: Nat), y`. The parser knows which from what it was parsing when it met the colon, and never from spacing or position. If the inline statement itself ends in a line-final colon, the lexer has already turned that into a `BlockOpen`, and the parser's `statement` consumes the layout block as part of the inline statement, which is how `x: match y: Some v: dyn $v:` with a block under it works: each `Colon` opens an inline block whose statement ends in the next, the last is a `BlockOpen`, and the `BlockClose` after the layout block closes every inline block above it, because the terminator of an inline block includes a `BlockClose` it did not open.

`else` terminates an inline block and nothing else does besides line structure, so `if c: 3 else: 4` inside parentheses parses as two inline blocks, and `if c: match x:` followed by a layout block of arms and then `else:` at the column of `if` is legal, defined, and in the corpus.

Suppose instead the lexer decided the inline colon too, emitting `BlockOpen` and a synthetic `BlockClose` at the end of the line. Then `(x: Nat)` would need the lexer to know that a colon inside parentheses after a pattern is an ascription, which is parsing, and `if c: 3 else: 4` would need it to know that `else` ends a block, which is parsing. The split chosen puts every decision that needs grammar in the parser and every decision that needs columns in the lexer, and the layout corpus can be checked against the lexer alone.

### What the lexer keeps

The only context the lexer keeps is the column stack, the bracket depth, the bracket pairs it matched for fusing, and whether the previous character was whitespace. The token stream for a program is a pure function of its characters. The lexer has no symbol table, no knowledge of which names are widget functions, and no mode for "inside a widget block"; a lexer that needed any of those would be one whose output a reader could not predict from the text, and the layout corpus, which both implementations keep as a golden, is the test that neither one grew such a mode.

## Parser

The parser has one hard problem, paren-less calls, and the rest is a precedence climber with blocks. The hard problem is hard in the way that a reader's problem is hard: given `Insert items, id, Todo label, False`, who owns the commas? The language answers "the outermost call on the line", and once that rule is in the parser the rest of the grammar is a transcription. This section gives the parse tree and a grammar sketch for every construct, with the places a hand-written recursive-descent parser has to be careful marked as such, so that the Rust and Haskell parsers can be written from the same sketch and the goldens can say whether they agree.

### The parse tree

```rust
pub struct Module { pub items: Vec<Item> }

pub enum Item {
    Let     { name: SName, params: Vec<TypeParam>, ty: Type, value: Expr, span: Span },   // annotation required
    Widget  { name: SName, body: Block, span: Span },                                     // @counter: ...
    TypeDecl{ name: Name, params: Vec<Name>, def: TypeDef, span: Span },
    TagDecl { tag: QName, of: Name, fields: Vec<Field>, requests: Option<Type>,
              responds: Option<Type>, state: Option<Type>, span: Span },
    Test    { name: Name, body: Expr, span: Span },                                       // test name: expr
}

pub enum TypeDef {
    Record(Vec<Field>), Choice(Vec<Alt>), Newtype(Type), Unit(Name), Open,              // `choice ..`
}
pub struct Field { pub name: SName, pub ty: Type }
pub struct Alt   { pub name: Name, pub payload: Option<Type> }

pub struct Block { pub stmts: Vec<Stmt>, pub span: Span }

pub enum Stmt {
    Bind   { pat: Pat, widget: Expr, span: Span },      // pat <- widget
    Build  { widget: Expr, span: Span },                // widget          (result discarded, or the block's result)
    Let    { pat: Pat, value: Expr, span: Span },       // let pat = value
    Hold   { name: SName, init: Expr, event: Expr, span: Span },   // $x <- init then event
    Result { value: Expr, span: Span },                 // @ value   (a free @ on the last statement)
    Expr   { value: Expr, span: Span },                 // in non-widget blocks: the value
}

pub struct SName { pub sigils: Sigils, pub name: Name, pub span: Span }
pub struct QName { pub parts: Vec<Name>, pub span: Span }

pub enum Expr {
    Var(SName), QVar(Sigils, QName),
    Nat(u128), Float(f64), Char(char), Unit, Empty, EmptyMap,
    Text(Vec<TextPartAst>),                                  // "hi {name}"; a dynamic inside makes it a dynamic, decided by the elaborator
    Array(Vec<Expr>), MapLit(Vec<(Expr, Expr)>), Tuple(Vec<Expr>),   // [[a, b]]  {k: v}  ((a, b))
    Call   { f: Box<Expr>, args: Vec<Expr> },                // F a, b   (F a, b)   x.F a, b   and a chain step
    Named  { ctor: Box<Expr>, args: Vec<(Name, Expr)> },     // Vector(x=1.0, y=2.0), Ui.Button(label="OK")
    RecordBlock { ctor: Box<Expr>, fields: Vec<(Name, Expr)> }, // Vector: x: 0.0 ...
    Binary { op: BinOp, l: Box<Expr>, r: Box<Expr> },        // + - * / +~ ... < <= > >= == != and or |  on
    Prefix { op: PrefixOp, e: Box<Expr> },                   // + - +? not and free sigils
    Field  { e: Box<Expr>, field: SName },                   // e.x  e.!changed
    Index  { e: Box<Expr>, i: Box<Expr> },                   // e[i]
    Lambda { params: Vec<Pat>, body: Box<Expr> },            // \body  (params = [_])  or  \|a, b| body
    Match  { scrutinee: Box<Expr>, arms: Vec<Arm> },
    If     { c: Box<Expr>, t: Box<Expr>, e: Box<Expr> },
    For    { key: Option<Pat>, item: Option<Pat>, coll: Box<Expr>, body: Block },   // for k, $item <- $items; both None is the shorthand
    Dyn    { dynamic: Box<Expr>, body: Block },
    Intercept { child: Box<Expr>, responses: Box<Expr>, supplier: Box<Expr> },
    WidgetBlock(Block),                                      // @widget: ...
    LetIn  { stmts: Vec<Stmt>, value: Box<Expr> },           // a layout block of lets ending in a value
    Ascribe{ e: Box<Expr>, ty: Type },
}
pub struct Arm { pub pat: Pat, pub guard: Option<Expr>, pub body: Expr, pub span: Span }   // pat if guard: body

pub enum Pat {
    Wild,                                   // ignore
    Bind(SName),                            // x, $x, _
    Lit(Lit), Unit,
    Tuple(Vec<Pat>),                        // ((a, b))
    Array  { elems: Vec<Pat>, rest: Option<Box<Pat>> },   // [[]]  [[a, b]]  [[head, ..rest]]; rest is a binding or `ignore`
    Some(Box<Pat>), Nil,
    Ctor   { name: QName, payload: Option<Box<Pat>> },                 // Circle r, Stopped, Clock
    Record { name: QName, fields: Vec<(Name, Pat)>, positional: Vec<Pat>, rest: bool }, // T(f=p, ..)
    Wildcard { ns: QName },                 // Gui.*(..)
    Ascribe{ p: Box<Pat>, ty: Type },
}

pub enum Type {
    Named(QName, Vec<Type>),                // Nat, Pair<a, b>, Element
    Var(Name),                              // a lowercase name
    Sigiled(Sigil, Box<Type>),              // !T  $T  @T  ?T
    Fn(Vec<Type>, Box<Type>),               // fn A, B -> C
    Map(Box<Type>, Box<Type>),              // K ==> V
    Array(Box<Type>), Tuple(Vec<Type>), Unit,   // [A]   ((A, B))
}
```

Every variant has a span; the sketch omits the field for the ones where it is obvious. `SName` keeps the sigils the programmer wrote, and the elaborator checks them against the inferred kind rather than trusting them, which is what makes "`count` is bound to a dynamic, but its name has no sigil" a diagnostic rather than a silent reinterpretation.

### Grammar sketch

Written as the parser reads it, top down. `block` is the routine from the lexer section; `expr(p)` is the precedence climber at level `p`.

```
module      := item*
item        := 'let' sname tparams? ':' type '=' expr
             | sname ':' block                                    # a widget definition: @counter: ... (the @ is the name's sigil)
             | 'type' Upper tparams? '=' typedef
             | 'tag' qname 'of' Upper block                       # fields and parts, one per statement
             | 'test' lower ',' expr block                         # test counter, @counter: one turn per statement

typedef     := 'record' block | 'choice' ('..' | alt ('|' alt)* | block)
             | Upper type                                          # newtype: Meters Float
             | lower                                               # unit type: click
tag_stmt    := lower ':' type                                      # a field
             | 'requests' ':' type | 'responds' ':' type | 'state' ':' type

block_stmt  := pat '<-' expr 'then' expr                          # hold: pat must be one sigiled name
             | pat '<-' expr                                       # bind
             | 'let' pat '=' expr
             | '@' expr                                            # result, only as the last statement
             | expr                                                # build and discard, or the block's result

expr(hold)  := expr(chain) ('then' expr(chain))?                  # does not associate
expr(chain) := expr(merge) (ChainDot Upper call_args?)*            # a chain step on a continuation line
expr(merge) := expr(on) ('|' expr(on))*
expr(on)    := expr(or) ('on' expr(or))*
expr(or)    := expr(and) ('or' expr(and))*
expr(and)   := expr(not) ('and' expr(not))*
expr(not)   := 'not' expr(not) | expr(cmp)                        # looser than comparison: `not a == b` is `not (a == b)`
expr(cmp)   := expr(add) (cmpop expr(add))?                       # cmpop: < <= > >= == !=
expr(add)   := expr(mul) (addop expr(mul))*
expr(mul)   := expr(prefix) (mulop expr(prefix))*
expr(prefix):= prefixop expr(prefix) | freesigil expr(postfix) | expr(postfix)
             | Dot(prefix) sname | Dot(prefix) Upper call_args? | '[' expr ']'   # the pronoun forms: _.x, _.F a, _[i]; the resolver finds the binder
expr(postfix) := atom ('.' sname | '[' expr ']' | '.' Upper call_args?)*   # field, index, method (all binary, no space)
atom        := callable call_args?                                 # a paren-less call, see below
             | literal | '(' expr(hold) ')' | '((' expr (',' expr)+ '))'
             | '[[' (expr (',' expr)*)? ']]' | '{' map '}'
             | lambda | 'match' expr block | 'if' expr block 'else' block
             | 'for' binders? expr block | 'dyn' expr block | 'intercept' expr ',' expr ',' expr
             | '@' 'widget' block | ctor_named | ctor_block
callable    := sname | qname
call_args   := expr(merge) (',' expr(merge))*                      # the ',' repeats only when in_args is clear

lambda      := '\' body | '\' '|' pat (',' pat)* '|' body
             where body := expr(hold) running to the end of the enclosing expression
binders     := (pat ',')? pat '<-'                                 # for k, $item <- $items:  or  for $item <- $items:
ctor_named  := qname '(' (lower '=' expr) (',' lower '=' expr)* ')'
ctor_block  := qname block                                         # Vector: x: 0.0 ...

type        := 'fn' ptype (',' ptype)* '->' ptype                  # fn A, B -> C
             | ptype
ptype       := sigil ptype | qname ('<' type (',' type)* '>')? | lower | 'unit'
             | ptype '==>' ptype | '[' type ']' | '((' type (',' type)+ '))'
             | '(' type ')'                                        # grouping: fn (fn A -> B), [A] -> [B]

arm         := pat ('if' expr)? block                              # a guard: pat if cond: body
pat         := 'ignore' | sname | literal | 'unit' | '((' pat (',' pat)+ '))' | '(' pat ')'
             | '[[' (pat (',' pat)* (',' '..' pat)? | '..' pat)? ']]'     # [[]]  [[a, b]]  [[head, ..rest]]
             | 'Some' pat | 'Nil' | qname pat? | qname '(' fieldpats ')' | qname '.' '*' '(' '..' ')'
             | pat ':' type
fieldpats   := (lower '=' pat | pat | lower) (',' ...)* (',' '..')?
```

The places where a transcription of this sketch goes wrong, each with its answer:

**A call's arguments are whole expressions.** A call is recognised at the `atom` level, when a callable name is followed by a token that begins an expression, and each of its arguments is parsed at the `merge` level, which is the level just below the application's place in the precedence table. That is what makes a call extend to the end of the expression it sits in: `F a + 1` is `F(a + 1)` and `F a | G b` is `F(a | G(b))`, because the first argument absorbs everything up to the next comma or the end. `1 + F a` parses `1`, sees `+`, parses the right operand at the `mul` level, reaches the atom `F` followed by `a`, and makes the call there, so the result is `1 + F(a)`; nothing precedes a call's name that the call could absorb. The practical rule for the implementer: after parsing a callable name, look at the next token; if it begins an expression and is not a binary operator, a closing bracket, a comma, a `Sep`, a `Semi`, a `ChainDot` or a block token, the name is a function and what follows is its first argument.

**Commas belong to the outermost call.** `expr(app)` is entered once per enclosing expression, and inner names followed by arguments parse as one-argument calls because the inner call's `args` loop is suppressed: the parser passes a flag `in_args` down, and a call parsed with `in_args` set takes exactly one argument. `F a, G b, c` is `F(a, G(b), c)`.

**A group with a call in it is that call, with all its commas.** Single parentheses are grouping, and the parser clears `in_args` inside them, so the outermost call in the group takes every comma in the group: `F a, (G b, c)` gives `G` two arguments, and `(Insert $items, $next, todo) on !add` is one three-argument call sampled at `!add`. A comma inside single parentheses that no call takes, as in `(a, b)`, is the error "a comma in parentheses belongs to a call; a pair is `((a, b))`". A callable followed by `(` with no whitespace between is the same call in bracket form, `G(b, c)`, and `G(x=1, y=2)` with `lower =` items is `Named`; both forms exist because element construction reads best with named fields, and a positional call written the way most languages write one should mean that call.

**Tuples and arrays take their commas.** Inside `TupleOpen ... TupleClose`, `ArrayOpen ... ArrayClose` and `{ }`, the parser sets `in_args`, so commas belong to the bracket and a call inside takes one argument: `((G b, c))` is the pair of `G(b)` and `c`, `[[F a, G b]]` has two elements, and a multi-argument call inside one takes its own single parentheses, `(((G b, c), d))`. Single `[` after an expression with no space is always indexing; a prefix `[` is the pronoun's index (`_[i]`), so `F [1, 2]` is the error "single brackets index; an array is `[[1, 2]]`" (a comma inside a prefix `[`), and `[]` is "an empty array is `[[]]` or `empty`".

**Function types are comma lists.** In a type, `fn` takes comma-separated parameter types up to `->` and one result type after it, `fn A, B -> C`, the same way a call takes its arguments. A parameter or result that is itself a function type is parenthesised, `fn (fn A -> B), [A] -> [B]` and `fn A -> (fn B -> C)`, and an unparenthesised `fn` in either place is the error "a function type as a parameter or result is parenthesised: `(fn A -> B)`". `fn ((A, B)) -> C` takes one pair, and `fn (A, B) -> C` is the error "a comma in parentheses belongs to a call; a pair type is `((A, B))`", so the two readings never share a spelling.

**Method chains end at a continuation line.** `expr(chain)` sits between hold and merge. A `ChainDot` is not an operator any lower level knows, so it ends every paren-less call open on the lines above it, and `expr(chain)` takes the whole expression so far as the receiver of the step:

```prism
items.Filter IsDone
    .MapValues Label          # MapValues (Filter items, IsDone), Label
    .Sort                     # Sort (MapValues ...)
```

A step's own arguments are parsed as a call's and end at the next `ChainDot`. Inside brackets the rule is the same, because the lexer marks the line start whatever the bracket depth. A one-line spaced chain, `x.F a .G b`, is not a chain: the ` .G` is a prefix `Dot`, which is the pronoun where `_` is in scope and an error where it is not ("put each chain step on its own line").

**Trailing lambdas.** A `\` in argument position begins a lambda whose body runs to the end of the enclosing expression, including any commas: `Fold changes, items, \|acc, id, c| Step acc, id, c` gives `Step` three arguments inside the lambda and `Fold` three arguments outside it. The body is parsed with `in_args` cleared and at the `hold` level, and if it ends in a line-final colon the layout block belongs to the body (`\@widget:` and `\match _:` are this). A lambda that is not the last argument is parenthesised, and the parser's error for `F \x, y` where `y` was meant for `F` says so: "a trailing lambda takes the rest of the line; parenthesise it to pass more arguments".

**A free sigil applies to the whole call after it.** `expr(prefix)` on a `FreeSigil` parses its operand at the `postfix` level, where a callable name takes its arguments, so `@ ChecklistUpdate ix, !checked` is `@ (ChecklistUpdate ix, !checked)` and `$ x + 1` is `$ (x + 1)`. An attached sigil is part of the name token and never reaches this rule.

**Boolean operators.** `not` is a prefix keyword that takes an `expr(not)`, so it sits between `and` and comparison: its operand is a whole comparison, and `not a == b` is `not (a == b)`. `and` and `or` are left-associative and sit between `not` and `on`, with `and` tighter, so `x or y and z` is `x or (y and z)`, `a < b and c < d` needs no parentheses, and `$done or $n > limit on !click` is `($done or ($n > limit)) on !click`. `not $done and $n > limit` is `(not $done) and ($n > limit)`, since `not` binds tighter than `and`. The parser builds `Binary` and `Prefix` nodes and stops there; a non-`Bool` operand is the checker's error.

**Hold does not associate.** `expr(hold)` takes at most one `then`; a second is the error "`then` does not associate; a hold has one initial value and one event".

**Method form.** `x.F y, z` is parsed at the postfix level: `x`, then `.` with the binary flag, then an uppercase name, then arguments as for a call, and the tree is `Call { f: F, args: [x, y, z] }`. Method form associates left: `x.F.G` is `G (F x)`.

**The pronoun shorthand applies wherever `_` is in scope.** A prefix `.` before a lowercase name is a field of the pronoun, before an uppercase name the method applied to the pronoun, and a prefix `[i]` indexes the pronoun: `\.done`, `\.Len`, `\[0]`, `\not .done`, and the same inside a `for` body or a `dyn` body or a `match` arm that binds `_`. All are `Field`, `Call` and `Index` nodes over `Var(_)`. The parser does not track which constructs bind `_`: it builds these nodes wherever the lexer set the `prefix` flag, and the resolver, which owns scopes, finds the binder. Where no `_` is in scope the resolver reports "no pronoun here; `.x` means `_.x`, so write `_.x`, or put a chain step on its own line", so a stray space never quietly turns a method call into a pronoun read. A `.Method` that starts a continuation line is a `ChainDot`, never a prefix `Dot`, so indentation separates the two readings.

**Record forms.** `Vector 1.0, 2.0` is an ordinary `Call` and the elaborator learns that `Vector` is a constructor. `Vector(x=1.0, y=2.0)` is `Named`, recognised because a qualified or uppercase name is followed immediately (no whitespace) by `(` and the first thing inside is `lower =`. `Vector:` followed by a block of `field: value` statements is `RecordBlock`. Element construction is the same three forms with a qualified name: `Ui.Button(label="OK")`, `Ui.Button "OK"`, `Ui.Button:` block. A tag with no fields is a bare qualified name, `Clock`, and parses as `QVar`.

**`if` and `else`.** `if c` takes a block, then expects `Else`, then a block. After an inline `if c: a`, the `else` ends the inline block and is the next token; after a layout block, the lexer has emitted `Sep` before `else` at the block's column, and the `if` routine consumes the separator when the token after it is `Else`. An `else` belongs to the nearest `if` that has none yet; a layout `else` finds its `if` by column, because the dedent has already closed every block between them. An `if` with no `else` is the error the language chapter gives.

**`match` guards.** An arm is `pat`, then optionally `if` and an expression, then the block. The guard is parsed at the `hold` level up to the arm's colon, which the lexer has left as `Colon` (inline arm) or turned into `BlockOpen` (layout arm), so `Some n if n > limit: ...` needs no lookahead. The guard is stored on the `Arm`, not wrapped around the body, so that exhaustiveness can see it.

**`for` binders.** After `for`, the parser scans ahead on the line for a `<-` at bracket depth zero before the line-final or inline colon. If there is one, the tokens before it are one pattern or two comma-separated patterns, the key and the item (`for k, $item <- $items:`; with one pattern, `for $item <- $items:`, it is the item). If not, the line is the shorthand `for $items:` and `key` and `_` are bound by the loop. The scan is a token walk with no grammar, and it is sound because a `<-` can appear in an expression only inside a block, which the scan does not enter.

**Array patterns.** `[[` in a pattern is `ArrayOpen`: `[[]]` is empty, `[[a, b]]` is exactly two elements, and `[[head, ..rest]]` is a first element and the remainder. `..` takes a pattern after it, a name or `ignore`, and may only be last; the parser reports a second `..` or one in the middle ("`..` takes the rest of the array, so it comes last").

**Widget definitions.** `@counter:` at top level is `Item::Widget`; `@Button: fn $Text -> @!Click = \@widget:` is `Item::Let` with a lambda value whose body is a `WidgetBlock`. Both forms exist because the first reads as a declaration and the second as a function, and the parse trees differ only in whether there is a lambda around the block.

**Tag declarations.** `tag Ui.Button of Element:` opens a block whose statements are fields (`label: Text`) and parts (`responds: Click`). The three part names are not keywords; the parser recognises them as the first token of a `tag` statement, and a tag with a field named `state` is rejected with "`state` names a part of the tag; rename the field". Order is free; each part at most once; the `of` type names an extensible variant, which the resolver checks.

**`intercept`.** Three arguments, comma-separated, parsed as an atom so that the whole call is one expression: `((result, $slots)) <- intercept @child, !down, $ \|e| @Element e`. The third argument begins with a free `$`, which takes the lambda, which takes the rest of the line.

**Tests.** `test counter:` opens a block whose single statement is an expression, in practice `Test @counter: array:` with the steps as array elements. The parser keeps it as `Item::Test { body: Expr }` and the elaborator types the body against the conformance suite's `Test` function and step constructors from the prelude; a test is therefore ordinary Prism with one keyword to mark it, and a module's tests are collected by the harness without a second syntax.

**Patterns.** `Type(f, g)` as shorthand for `Type(f=f, g=g)` is recognised when every item inside the parentheses is a bare lowercase name; `Type(p, q)` with any non-name pattern is positional. A mix is an error: "name every field or none". `Ns.*(..)` is `Upper Dot(binary) Star` followed by `( .. )`, and the `..` is mandatory because the arm cannot know the fields. `p: Type` ascribes.

**Layout blocks of `let`s.** A lambda body, a `for` body or a `match` arm that opens a layout block containing `let` statements and ends in an expression is `LetIn`; the parser reads statements until the last, which must be an `Expr`, and a block ending in a `let` is the error "a block of lets must end in a value". Inside a widget block the same statements are the block's.

### Spans

Every node's span runs from its first token's start to its last token's end, and a compound node's span covers its children. The spans the diagnostics need most are the ones a careless parser loses: the span of an operator token inside a `Binary` (for "both are events" pointing at the two operands and the operator), the span of each argument in a `Call` (for "applied to -1"), and the span of each arm's pattern (for exhaustiveness). The parser therefore keeps operator and keyword spans as fields, not only node spans; `Binary` has `op_span`, `Call` has `arg_spans`, `Arm` has `pat.span`. Retrofitting these is the expensive part of a compiler, so they are in the first commit.

### What the parser does not decide

Whether `F` is a function or a constructor, whether `Button` is a widget function, whether `$count` is really a dynamic, whether `0` is a `Nat`, whether `|` is merging events or is a stray token in a type: none of these. The parser produces a tree that says what was written, with sigils as written and types as written, and the resolver and elaborator say what it means. A parser that consulted a symbol table to decide a parse would be one whose output changed when an import changed, and the goldens for the parser, which are parse trees printed, would then depend on the prelude; keeping the parser context-free is what lets its goldens be small files with no imports at all.

## Resolver

The resolver answers "which thing does this name mean" and "in what order can this block be built", and the second question is the one that distinguishes a Prism resolver from any other. A widget block's statements are mutually recursive and may be written in any order; the program is a set of equations. Someone has to find an order in which the equations can be turned into constructor calls, and that someone is the resolver, with Tarjan's algorithm over the block's dependency graph. Everything else here is the ordinary business of scopes.

### Scopes

```rust
pub struct Resolved { pub module: Module, pub refs: HashMap<NodeId, Binding>, pub groups: HashMap<BlockId, Vec<Group>> }

pub enum Binding {
    Local  { block: BlockId, index: usize, kind: Sigils },   // a statement of a widget block
    Param  { lambda: NodeId, index: usize },                 // a lambda argument
    Pronoun{ binder: NodeId },                               // _ in a lambda, dyn, for (shorthand) or match
    Binder { for_node: NodeId, which: ForVar },              // a for's key or item: named by the loop, or `key` / `_` in the shorthand
    Loop   { for_node: NodeId, which: LoopVar },             // ix jx kx: the indices of an update block's loops
    LetSeq { stmt: NodeId },                                 // a let in a sequential block
    Top    { item: ItemId },                                 // a module-level definition
    Prelude{ name: &'static str, intrinsic: Option<Intrinsic> },
    Tag    { id: TagId },
    Ctor   { ty: TypeId, alt: Option<usize> },               // Vector, Circle, Some
}
```

Scopes nest the obvious way: module, then each item, then blocks and lambdas inside it. Three rules are Prism's own.

*A widget block is one scope in which every statement's names are visible everywhere in the block,* including in statements above their definition and in their own definition. A name bound twice in one block is an error, "`$count` is bound twice in this block", because the block is a set of equations and a set has one equation per name. A `let` outside a widget block, in a lambda body or a `for` body, is sequential: it sees names above it and not below, and a reference to a later `let` is "`x` is used before its definition; plain bindings are sequential".

*The pronoun.* `_` is bound by the nearest enclosing lambda (`\body`), `dyn` block, `for` block or `match` arm pattern that binds it, and its sigiled forms `$_`, `!_`, `@_` name the same binder read at that kind. Inside `for $m:`, `_` is the entry's value as a plain name and `$_` is the entry as a dynamic; which of them the body uses is recorded on the binder, because the lambda rule of the language chapter says a body that mentions `$_` takes a dynamic and the elaborator reads that from the resolver's note. Only one pronoun is in scope at a time; an inner binder shadows an outer one and a program that needs both names one of them. `.x`, `.F` and `[i]` in prefix position are references to the pronoun and resolve the same way, in any of these binders; where none is in scope, they are the error the parser section gives.

*Loop binders.* `for k, $item <- $items:` names the key and the per-key dynamic, and the resolver binds each as `Binder { for_node, which }` with `ForVar` `Key` or `Item`; with one pattern before `<-` only the item is named. The shorthand `for $items:` binds `key`, a keyword, to the key, and `_` and `$_` (the pronoun) to the item. An inner shorthand `for` shadows an outer one's `key` and `_`, as inner pronoun binders always do; a body that needs both names the binders of one of them. Over an array the key binder is the index. A widget-level `for` has no index variables to count: its variables are the names the program gave it, so an outer binding cannot be captured by accident.

*Loop variables.* `ix`, `jx` and `kx` are keywords, and the resolver binds them by nesting depth in the loops of an `update` block: `ix` is the index of the innermost `for`, `jx` the next out, `kx` the next; the program does not name them. Using `jx` in a single `for` is the error the language chapter gives. They are keywords so that an outer binding cannot capture them, and the resolver's only work is counting.

Top-level names are visible throughout the module; the prelude is an outer scope the module shadows. Shadowing a prelude name is allowed and not warned about, except for the intrinsics below, where shadowing is an error, because a program that rebinds `@Element` has made a program nobody can read.

### Qualified names and namespaces

`Ui.Button` is one token and resolves in the tag table first: every `tag` declaration in the module and the prelude is interned into `tags: Vec<TagInfo>` with its namespace path, and a qualified name that matches a tag resolves to `Binding::Tag`. A qualified name that matches no tag resolves against the prelude's namespaced functions (`Wrap.Add`, `Checked.Mul`), and otherwise is "unknown tag or function `Fs.Content`; did you mean `Fs.Contents`?" with the nearest name by edit distance. In a pattern, `Gui.*` is the three tokens the lexer produces, and the resolver turns them into `Pat::Wildcard { ns: Gui }`, checking that at least one tag is declared under the namespace so that a typo in the namespace does not match nothing silently. A tag's `of` type must resolve to a type declared `choice ..`; declaring a tag of a closed choice is "`Shape` is a closed choice; declare it `choice ..` to add tags".

### Intrinsics

Some prelude names are not functions the compiler calls but nodes the elaborator builds, and the resolver marks them so that the elaborator's walk can dispatch on the mark instead of on a string:

| Name | Node |
|---|---|
| `!Switch` | `switch` over an event of events |
| `$Join` | `join` over a dynamic of dynamics |
| `!Leftmost` | `leftmost` over an array of events |
| `!never` | `never` |
| `@started` | `started` (a `dyn` over a constant, built as one node) |
| `@HoldFirst` | `hold_first`: `fn !a -> @$a`, a dynamic seeded by the event's firing in the occurrence it is built |
| `@Element` | `element`, with the tag's kind |
| `!Updated` | `updated`: a dynamic's change event |
| `$Distinct` | `distinct`: a derived dynamic that drops equal-valued changes, equality bound |
| `Kind` | `kind`: a tag-table lookup |
| `intercept` | `intercept` (a keyword, listed here because it is a node) |
| `Some`, `Nil`, `True`, `False`, `click` and every declared constructor | constructor application |

Everything else in the prelude is an ordinary function with an ordinary annotation, compiled from Prism source or bound to a Rust implementation by name (`Len`, `Insert`, `Sort`, `ToText`, `Wrap.Add`), and the resolver treats it as a `Top` binding of a module it did not write.

### Dependency analysis into groups

For each widget block, the resolver builds a graph whose vertices are the block's statements and whose edges are references: statement `s` depends on statement `t` when `s`'s right-hand side mentions a name `t` binds, at any depth, including inside lambdas, nested blocks and the `dyn` and `for` bodies that `s` contains. References inside a nested *widget* block (a `dyn` body, a `for` body, a `\@widget:` lambda) count as references of the statement that contains them, because the nested block is built by that statement and captures from its environment. The algorithm is Tarjan's strongly connected components, emitted in reverse topological order so that every group comes after every group it depends on:

```rust
pub struct Group { pub members: Vec<usize>, pub cyclic: bool, pub edges: Vec<Edge> }
pub struct Edge  { pub from: usize, pub to: usize, pub at: Span, pub label: EdgeLabel }
pub enum EdgeLabel { Prompt, Delayed(Delay) }
pub enum Delay    { Tag, Switch, Intercept }
```

A group with one member and no self-edge is acyclic and is built in one step. A group that is cyclic (two or more members, or one member that mentions itself, as the counter's `$count` does) is where placeholders go, and the edge labels decide where.

The resolver labels edges from syntax alone, which it can do because lifting is read off sigils and sigils are syntax:

- A reference to a dynamic `$x` as an operand of an operation or application that also has exactly one event operand is a *tag* read: `Delayed(Tag)`. `$count + !delta`, `$v on !e`, `(Insert $items, $next, todo) on !add` all read their dynamics this way.
- A reference to a dynamic anywhere in the event expression of a hold `$y <- v then e` is a tag read for the same reason: the event expression is an event, and dynamics inside it are read through tags.
- The argument of `!Switch` is `Delayed(Switch)`: the switch adopts a new inner at commit.
- A reference to the child's element list, the second result of `intercept`, from the responses argument of the same `intercept` is `Delayed(Intercept)`: routing consults the list as the occurrence opened.
- Every other reference is `Prompt`: a dynamic in an operation with no event (a derived dynamic reads its change event), the dynamic of a `dyn` or a `for` (rebuilt in the occurrence of the change), the argument of `$Join`, the event side of anything, a plain value.

One edge the resolver cannot label is the one from a `for`'s collection to its consolidated event, because its label depends on whether the block fires at birth, which is a property of the block's typed form. The resolver labels it `Prompt` provisionally and the elaborator's causality check, below, relabels it after the block is elaborated. The elaborator also checks every other label against the lift it actually inserted, and a disagreement is an internal error, since both were read from the same sigils.

Why the resolver and not the elaborator does the grouping: the elaborator types a cyclic group monomorphically as one unit and needs to know the groups before it starts, and the groups do not depend on types. Why the labels are computed here even though the check runs later: the labels are also what places the placeholders, and placing them is a property of the resolved tree that the text form prints, so that a golden shows where a knot is cut before any type is mentioned.

### Placing the placeholders

A cyclic group's build order is found by cutting the cycle at its delayed edges. Every legal cycle has at least one delayed edge (the causality check rejects the others), so removing all delayed edges from a group's subgraph leaves it acyclic, and the remaining prompt edges give a topological order. The targets of the removed delayed edges are the names that need a placeholder: each is referenced before its definition is built, and each is referenced only in a position that *subscribes* (a tag subscribes to a behavior and samples it at occurrence time; a switch subscribes to an event of events; routing subscribes to a list) and never in one that *samples at build*. The placeholder is opened before the group, the group is built in the order found, and the placeholder is closed to the real node after its definition's step.

The counter's group is `{ $count }` with one self-edge labelled `Delayed(Tag)`. Cut it: `$count` gets a placeholder; the hold is built with its tag reading the placeholder; the placeholder is closed to the hold. A group `{ !a, !b }` where `!a = F !b` and `!b = !Switch !!c` with `!!c <- dyn $d: ... !a ...`: the delayed edge is the switch's, so `!b` gets the placeholder, `!a` is built reading it, the `dyn` is built reading `!a`, the switch is built over the `dyn`'s event, and the placeholder is closed to the switch. In each case the thing that reads the placeholder at build is a subscription, which is what the runtime's cyclic slots support, and the thing that would need the real value at build (a `dyn` over the name) is on a prompt edge and therefore comes after the close. That is the property the lint checks under the name "nothing samples a placeholder before its close", and it holds by construction of the cut.

Suppose instead the compiler opened a placeholder for every member of every cyclic group, as the simplest implementation would. Then `dyn $count` inside the group would read the placeholder at build, the runtime would report "cyclic dynamic never closed", and the program, which is legal, would fail at run time with an error that names the runtime's mechanism rather than the program's cycle. Cutting at the delayed edges is what makes every legal program build.

## Elaboration

You have read a line of a reactive program and not known whether the dynamic in it was sampled or subscribed to, and you found out by running it. Elaboration is the stage that makes sure nobody ever has to find out that way again: it takes the resolved tree and produces a tree in which every lift is a named node, every number has a type, every binding says whether it runs once or at every occurrence, and every cycle has been checked for a delayed edge. Nothing below elaboration decides anything about meaning. The output is the typed AST, and the typed AST is the reference for what a Prism program means; the closure compiler walks it and the text form prints it, and if the two frontends print the same text for a program, they agree about that program entirely.

### The typed AST

```rust
pub struct TModule { pub items: Vec<TItem>, pub tags: Tags, pub types: Types }

pub enum TItem {
    Let    { name: SName, scheme: Scheme, value: TExpr, span: Span },
    Widget { name: SName, ty: Type, body: TBlock, span: Span },
    Test   { name: Name, body: TExpr, span: Span },
}

pub struct TBlock { pub bindings: Vec<TBinding>, pub result: TExpr, pub groups: Vec<Group>, pub span: Span }

pub struct TBinding {
    pub name: SName,            // or a generated name for a discarded result
    pub ty: Type,               // with its sigils
    pub mark: Mark,             // Build | Occ
    pub group: GroupId,
    pub def: TDef,
    pub span: Span,
}
pub enum Mark { Build, Occ }

pub enum TDef {
    Bind        { widget: TExpr, pat: TPat },                  // builder.bind
    Hold        { init: TExpr, event: TExpr },                 // Dynamic::hold
    Let         { value: TExpr },                              // a value, plain or lifted
    Placeholder { kind: PlaceholderKind },                     // opened before the group
    Close       { placeholder: BindingId, real: BindingId },   // closed after it
}

pub enum TExpr {
    // the plain layer
    Var(BindingRef, Type), Lit(Lit, Type), Unit,
    App   { f: Box<TExpr>, args: Vec<TExpr>, ty: Type },
    Lambda{ params: Vec<TPat>, body: Box<TExpr>, ty: Type, captures: Option<Vec<BindingRef>> },  // captures after conversion
    Ctor  { ty: TypeId, alt: Option<usize>, fields: Vec<TExpr> },
    Elem  { tag: TagId, fields: Vec<TExpr>, request: Option<Box<TExpr>> },
    Field { e: Box<TExpr>, index: usize, ty: Type },
    Index { e: Box<TExpr>, i: Box<TExpr>, ty: Type },          // returns ?T
    Match { scrutinee: Box<TExpr>, arms: Vec<TArm>, ty: Type, dispatch: Dispatch },
    If    { c: Box<TExpr>, t: Box<TExpr>, e: Box<TExpr>, ty: Type },
    Let   { pat: TPat, value: Box<TExpr>, body: Box<TExpr> },  // a sequential let in a lambda, for or arm body
    Array(Vec<TExpr>, Type), MapLit(Vec<(TExpr, TExpr)>, Type), Tuple(Vec<TExpr>),
    TextCat(Vec<TExpr>),                                       // interpolation, all parts Text
    Trait { which: TraitOp, at: Type, args: Vec<TExpr> },      // Num.Add[Int] a, b; Eq[Text] a, b
    Bool  { op: BoolOp, args: Vec<TExpr> },                    // Bool.Not a; Bool.And a, b; Bool.Or a, b (operands are Bool; And and Or short-circuit)
    Dict  { which: Trait, at: Type },                          // the hidden argument for a bound
    Maybe { over: Vec<TExpr>, body: Box<TExpr> },              // ? lifting at the plain layer
    // the lifts: every one explicit
    Const   { value: Box<TExpr> },                             // $ v : a constant dynamic
    DMap    { over: Vec<TExpr>, body: Box<TExpr> },            // derived dynamic, prompt
    Map     { event: Box<TExpr>, body: Box<TExpr> },           // event map
    Tag     { event: Box<TExpr>, dyns: Vec<TExpr>, body: Box<TExpr> },  // event with dynamics read before
    Leftmost(Vec<TExpr>),
    Never(Type),
    Switch  { events: Box<TExpr> },
    Join    { dyns: Box<TExpr> },
    Updated { dynamic: Box<TExpr> },
    Distinct{ dynamic: Box<TExpr>, eq: Box<TExpr> },
    HoldFirst { event: Box<TExpr> },                           // a hold seeded by the event's build-occurrence firing
    // construction
    Dyn     { dynamic: Box<TExpr>, block: Box<TLambda> },      // block: \|v| @widget
    For     { coll: Box<TExpr>, block: Box<TLambda>, kind: Consolidation, keyed: bool,
              birth_firing: bool },                            // can the block fire in the occurrence a key appears?
    Element { elem: Box<TExpr>, kind: ElemKind },              // kind: the tag's parts, or Generic
    Intercept { child: Box<TExpr>, responses: Box<TExpr>, supplier: Box<TExpr> },
    Started,
    WidgetBlock(Box<TBlock>),
    Kind    { elem: Box<TExpr> },
}
pub enum Consolidation { Plain, Event, Dynamic, Widget }
pub enum ElemKind { Known(TagId), Generic }
pub struct TArm { pub pat: TPat, pub guard: Option<TExpr>, pub body: TExpr, pub span: Span }   // the guard is a plain Bool over the pattern's bindings
pub enum Dispatch { Choice(TypeId), Tag, Optional, Array, Literal }
```

`Type` is the plain type with its sigils as constructors: `Type::Dyn(Box<Type>)`, `Type::Event(..)`, `Type::Widget(..)`, `Type::Opt(..)` over `Nat`, `Int`, `Float`, `Text`, `Char`, `Unit`, `Fn`, `Map`, `Array`, `Tuple`, `Named`, `Var`. A `Scheme` is a `Type` with quantified variables and their bounds. The lifts are nodes, not annotations: a `Tag` node says "this is an event that fires with `event`, with the dynamics in `dyns` read through tags, and the body applied to the firing and the readings". The closure compiler maps each one to one runtime constructor, and the text form prints each one by name, which is what "every lift explicit" means in practice.

A sequential `let` has a node of its own, `Let { pat, value, body }`, and it appears only inside a lambda body, a `for` body or a `match` arm, never as a binding of a widget block, where a `let` is a `TBinding` with `TDef::Let`. A body `let x = F a` then `let y = G x` then `H x, y` elaborates to `Let { x, F a, Let { y, G x, H x, y } }`, nested right, and prints as `let x = F(a); let y = G(x); H(x, y)`. The leaf evaluator compiles it to a closure that evaluates the value, binds it in the frame's locals, and evaluates the body; there is no inlining, because a `let` of an expensive call used twice is the programmer saying "once".

### Hindley–Milner on the value layer

The checker is Hindley–Milner with levels, over plain types only. The algorithm is the standard one (unification variables with a level, generalisation at the level where a binding is introduced, instantiation with fresh variables at each use) and the choices that the text form exposes are these:

*Where generalisation happens.* At a `let` whose right-hand side is a lambda, and nowhere else. `let Double = \|x| x + x` is generalised and may be used at `Nat` and at `Float` in one block; `let n = Len xs` is not generalised, which costs nothing since it has no free variables to generalise. A `let` of a non-lambda with a free type variable, which is rare and always a mistake in a strict language, is monomorphic and the variable is fixed by the first use. This is the value restriction in its syntactic form, chosen because the typed AST then has exactly one place where a `Scheme` appears below top level, and a reader of the text form knows that a `\` on the right of a `let` is the only thing that introduces polymorphism.

*Cyclic groups are monomorphic.* A group is typed as one unit with one set of unification variables, and nothing in it is generalised until the whole group is typed, as in ML's `let rec`. Two widget-block statements that refer to each other are the only way to make such a group, and they are events, dynamics and widgets, which are monomorphic by nature; the rule matters only in that the elaborator must not generalise a group member before the group closes.

*Top-level annotations are signatures.* A top-level `let` has a full annotation, and the annotation is the scheme; the body is checked against it, not inferred and compared. A top-level widget definition `@counter:` has no annotation and no parameters, and its type is inferred as `@T` from its result, because its only use is to be bound. This keeps every error local to one item, since the checker never guesses a signature from uses in another item.

*How type variables are named.* Unification variables are numbered during inference and renamed when the typed AST is finalised: in order of first appearance, reading the printed form left to right and top to bottom, as `a`, `b`, `c`, ... with the programmer's own names kept where the annotation gave them. Two frontends that infer the same types therefore print the same letters, which the differential test on the text form depends on. The renaming is a pass over the finished item, not a property of inference, so that inference can allocate variables in whatever order its algorithm likes.

*Sigils in annotations.* `$x: $Int <- ...` and `$x: Int <- ...` mean the same thing: an annotation on a sigiled name gives either the plain type under the sigil or the full type. The elaborator normalises to the full type and checks that the sigils agree with the name's; a disagreement is the "sigil does not match" diagnostic.

The elaborator's walk is bidirectional in exactly one place, the kind of a lambda's binder (below), and otherwise is plain inference: `infer(expr) -> (TExpr, Type)` with `check(expr, expected)` as `infer` plus unify, except for lambdas, where an expected type is pushed in.

### The sigil lattice and lifting

Every operation and application is lifted by a rule read off its operands' outermost sigils, and the elaborator applies that rule as a table lookup, producing one node per lift. The boolean operators are operations like `+`: `not $done` is a `DMap` over `[$done]` with the body `\|d| Bool.Not d`, and `$a and !b` is a `Tag`. A lifted `and` or `or` is a lift of the same function: the dynamics are all read and the body is the plain `Bool.And a, b`, which evaluates `b` only when `a` holds, so the lifted node fires and changes by the lifting rule and not by any extra laziness.

| Operands' outer kinds | Result | Node |
|---|---|---|
| all plain | plain | the operation itself |
| some `$`, no `!` | `$` | `DMap { over: the dynamics, body }` |
| some `$` (zero or more), exactly one `!` | `!` | `Tag { event, dyns, body }` or `Map { event, body }` when there are no dynamics |
| two or more `!` | error | "two events in one operation" |
| any `@` | not lifted | an error unless it is an application of a widget function, below |

The body is the plain operation with the lifted operands replaced by lambda parameters: `$count + !delta` becomes `Tag { event: !delta, dyns: [$count], body: \|d, c| Num.Add[Int] c, d }`. The elaborator builds the body as a plain `TExpr` over fresh parameters, types it at the plain layer (where `+` is resolved by the numeric trait), and wraps it. One lifted expression becomes one node: `$a * 2 + 1` is one `DMap` over `[$a]` with the body `\|a| Num.Add[Int] (Num.Mul[Int] a, 2), 1`, not a node per operator, because the elaborator lifts the maximal plain subtree, which is found by walking down until an operand is itself sigiled. `on` needs no rule: it is a binary operator returning its left operand, so `v on !e` is `Map { event: !e, body: \|_| v }` and `$v on !e` is `Tag { event: !e, dyns: [$v], body: \|_, v| v }`, by the table.

`match` lifts like application: `match $x:` with plain arms is a `DMap` over `[$x]` with the match as its body; `match !e:` is a `Map`; dynamics mentioned in the arms, in their guards included, join the `dyns` of a `Tag`. A `match` with a widget arm requires a plain scrutinee, and the error points at `dyn`.

`if` lifts the same way, as a `DMap` or `Map` or `Tag` around a plain `If` body.

`?` lifts at the plain layer, inside the time functors, and the node is `Maybe { over, body }`: `a / b + 1` is `Maybe { over: [Num.Div a, b], body: \|q| Num.Add q, 1 }` of type `?Nat`; the `?` comes from `/` at `Nat` and `Int`, which fails on a zero divisor. At `Float` the same `/` is plain and follows IEEE (infinities and NaN, never failing), so `x / y` over two `Float`s has no `Maybe` and `Num.Div[Float] x, y` is an ordinary node. The elaborator finds a `?` operand where a plain one is wanted, wraps, and continues; the `?` propagates outward until a binding names it or a `match` eliminates it. `$a / $b` is `DMap { over: [$a, $b], body: \|a, b| Num.Div a, b }` of type `$?Nat`, since the time functor lifts first and the `?` is in the body's result. `?$x + 1` is a type error: an optional dynamic is matched, not lifted.

**Field access and indexing lift like application.** `$slot.element` is `DMap { over: [$slot], body: \|s| s.element }`; `!!panes.!inbox` is `Map { event: !!panes, body: \|p| p.!inbox }` of type `!!Mail` when `inbox` is `!Mail`; `$items[3]` is `DMap` with an `Index` body, of type `$?Item`. The parser has already written the pronoun shorthand `\.x` and `\[i]` as accesses on `_`, so they lift like any other.

**Interpolation lifts.** A text literal with parts is `TextCat` over its parts, each converted to `Text` by the canonical rendering of its type (`ToText[Int]` for an `Int`, the identity for `Text`, an error for anything else), and the whole is lifted by the table: `"Count: {$count}"` is `DMap { over: [$count], body: \|c| TextCat ["Count: ", ToText[Int] c] }` and `"Got {!line}"` is a `Map`: a literal that interpolates a dynamic is a dynamic, and one that interpolates an event is an event, with no sigil written on the literal. Two interpolations of different functor kinds follow the lifting rules (a dynamic and an event is an event, two events an error). `$"Increment"` is a free `$` applied to a literal with nothing to interpolate, `Const "Increment"`, a constant dynamic; the `$` is the prefix operator, not part of the token, and `$"Got {!line}"` is the sigil error because the literal is an event.

**Lifting over `@`.** A widget operand is built, not computed over, and the one case with a meaning is the application of a widget function to a dynamic where the function wants a plain value. Take a button declared over plain text, `@Button: fn Text -> @!Click`. `@Button $label` is then an application of a widget function to a dynamic argument, and its type by the table would be `$@!Click`, a dynamic of widgets. Binding a dynamic of widgets with `<-` lifts to `dyn`, and the elaborator inserts the node:

```
!click <- @Button $label
```

elaborates as the binding `!click: !Click [occ] = Switch { events: Dyn { dynamic: $label, block: \|l| @widget: bind (@Button l) } }`, marked `[occ]` because its root is a graph node, the switch. The inserted block applies `@Button` to the plain value, binds it, and returns its result; the `dyn`'s event is an event of the results of each build; and the bound name is what the result kind says:

| The widget's result | The bound name | Node |
|---|---|---|
| an event `!R` | `!x: !R` | `Switch (Dyn ...)` |
| a dynamic `$R` | `$x: $R` | `Join (HoldFirst (Dyn ...))` |
| a plain `R` | `$x: $R` | `HoldFirst (Dyn ...)` |
| `unit` | nothing bound | `Dyn ...` as a discarded bind |
| a bundle | each component by its own kind | one `Dyn` for the whole and one projection per component, each lifted by its row above |

A *bundle* is a widget result that is a tuple or record with sigiled components: `intercept`'s `@((a, $(Path ==> Slot)))`, `@Focusable`'s `@((a, !Input))`, a panel's `@((!Click, $Text))`. Functor values nest inside plain aggregates, so the result is one plain value with an event and a dynamic inside it, and a pattern bind takes it apart: `((result, $slots)) <- intercept ...` binds the bundle and then projects each component into its own name, which the text form prints as one line for the bundle, named by the pattern, and one line per component. Through the dynamic-of-widgets lift the projections are what get lifted: `((!click, $text)) <- @Panel $cfg` with `@Panel: fn Text -> @((!Click, $Text))` becomes one `Dyn` over `$cfg` whose event fires with the bundle, then `!click` is `Switch (Map dyn \|b| b.0)` and `$text` is `Join (HoldFirst (Map dyn \|b| b.1))`, each component by its own row of the table. The `intercept` dump in the text form section shows the unlifted case.

`HoldFirst` is a hold whose initial value is the event's firing in the occurrence the hold is built, which exists because a `dyn` fires in its build occurrence and a widget built in n sees all of n; the runtime constructs it as a hold that takes its initial value from the catch-up read of the event it holds. A program reaches the same node by name: the prelude's `@HoldFirst: fn !a -> @$a` is an intrinsic the resolver maps to the `HoldFirst` node, so `$latest <- @HoldFirst !reading` elaborates to the binding `$latest: $T [occ] = hold_first !reading`, the same line the inserted lift prints inside `join (hold_first ...)`. The second row is the canonical cache idiom with the first build as the initial value and no one-occurrence hiccup. The elaborator inserts the `dyn` when the application's argument is a dynamic and the function's parameter is plain; an argument that is a dynamic where the parameter is `$T` is an ordinary application with no lift, which is how the book's `@Button: fn $Text -> @!Click`, written with a `dyn` inside, takes `$"Increment"` with no lift at the call site. The rule is the same for either signature: lift only where the parameter wants a plain value and gets a dynamic.

An element whose field is a dynamic is the same rule one level down: `@Element Ui.Button(label=$l)` is `@Element` applied to `Elem { tag: Ui.Button, fields: [$l] }`, the element construction is a plain record build applied to a dynamic argument, so it lifts to a `DMap` giving a `$Element`, and `@Element` applied to a `$Element` is a widget function applied to a dynamic where it wanted a plain value, so it lifts to `dyn`, which is what "a dynamic where a field goes lifts into a `dyn`, fresh slot per change" means.

**The kind of a lambda's binder.** A lambda whose body mentions a sigiled pronoun, `$_`, `!_` or `@_`, takes a binder of that kind and is not lifted over it; the resolver has recorded which forms the body uses, and two different sigiled forms of `_` in one body is the error "`_` is read as a dynamic and as an event in one lambda". A named argument has the sigil it is written with. A lambda with no sigiled pronoun and no annotation takes a plain binder and is a plain function, lifted afresh at each application, *unless* it is checked against an expected type whose parameter has a sigil: a lambda passed to a function whose annotation says `fn $Text -> @unit`, or bound by a `let` with an annotation, takes the sigil the annotation gives. The two blocks the language builds from lambdas need no push: a `dyn`'s block takes the plain value of its dynamic, and `intercept`'s supplier is `$(fn Element -> @$State)`, a plain function under a constant `$`. This is the one place `check` pushes a type into a lambda, and it is bidirectional checking in the smallest form that works.

**Lifting never crosses a sigil boundary.** `$$a + 1` is the error "cannot lift into the inner layer"; the table looks at the outermost sigil of each operand only, and the body it builds is typed at the layer beneath, where `$a` is not a number. `$Join` and `!Switch` are the flatteners, and they are nodes of their own.

### The three traits

`Eq`, `Ord` and `Num` are resolved by inference and nothing else is overloaded. The elaborator keeps a constraint set alongside unification: `Num t` for every arithmetic operator and every numeric literal, `Eq t` for every use of `==`, `!=` and `$Distinct`, `Ord t` for every comparison operator (`<`, `<=`, `>`, `>=`), every `Sort`, and every map key. Constraints are solved when their type variable is resolved:

- At a concrete type, the constraint is checked against the fixed table (`Num`: `Nat`, `Int`, `Float`; `Eq`: every plain type with no function and no functor, structurally; `Ord`: primitives (`Float` by IEEE totalOrder, so it can key a map), text, chars, tuples and arrays of ordered types, closed choices by declaration order, declared types whose parts are ordered, and extensible-variant values by tag name then fields) and the operation node records the type: `Trait { which: Add, at: Int, args }`. The closure compiler then picks the one implementation for that type, and nothing is looked up at runtime.
- At a type variable that is generalised with a bound, `let Max<a: Ord>: fn a, a -> a`, the constraint becomes a hidden parameter of the function: the scheme is `forall a. Ord a => ...`, the body's `x < y` is `Trait { which: Lt, at: a, args }` whose implementation is the hidden argument `Dict { Ord, a }`, and every call to `Max` at a concrete type passes `Dict { Ord, Text }` first. This is dictionary passing for three dictionaries, and the dictionary is an ordinary function value, erased like everything else.
- At a type variable with no bound in scope, the error names the operation and the variable: "`<` needs `Ord a`; add the bound `<a: Ord>` to `Max`".
- At a type variable that is never resolved and never generalised, which is the case of an integer-spelled literal nothing pins, the variable defaults to `Int` at the end of the item, and the text form prints the literal as `Int` like any other.

Records have equality when all fields do, and ordering when all fields are ordered, lexicographic in field declaration order, as Rust's derived `Ord` is; tuples, arrays, primitives and text are ordered; extensible variants are ordered by tag name, then by plain fields, with any request event excluded as it is from equality, so elements can key sorted maps; element equality is tag plus plain fields, excluding any request event, so that two `Fs.Contents(path="/etc/motd")` are equal whatever events they hold, which is the identity the World's oracle keys on. The elaborator computes these per type once, memoised in `Types`, and the answer for a declared type is derived from its parts.

The negative space: no user-defined traits, no instances, no associated types. The constraint set is three symbols; the solver is a table lookup at concrete types and a hidden parameter at bounded variables. A general trait system would put instance search into the elaborator and a dictionary per instance into generic code; the three traits are enough for sorting, map keys, element identity and arithmetic, and the elaborator's constraint set is written so that a fourth symbol is a table row, not a redesign.

### Literals

An integer-spelled literal is `Nat | Int` by inference: the elaborator gives it a fresh variable with a `Num` constraint and a *spelling restriction* that excludes `Float`; a decimal-spelled literal is `Float` outright; `-1` is the negation of a literal and is `Int` outright (negation excludes `Nat`); prefix `+` is the numeric identity and pins nothing; `+?` converts an `Int` to a `?Nat`. A `Nat` is pinned by an annotation, `let n: Nat = 0`, since `Nat` has no suffixed operator. When inference leaves a literal's variable unresolved at the end of its item, it defaults to `Int`, the type a programmer means by an unadorned counter; the suffixed spellings `+~` (`Int`) and `+.` (`Float`) pin the type at the operator when a program wants to say so: `$count +~ !delta` constrains both operands and the result to `Int`. Defaulting happens once, after the whole item is inferred, so it never overrides a constraint that arrives later in the item.

The counter's numbers in this light: `-1` is `Int`; `+1 on !increment` has the variable of `1` constrained by `Num` and excluded from `Float`; the merge unifies the two sides, so `1` is `Int`; `$count + !delta` unifies `$count`'s type with `Int`; the hold's `0` unifies with `$count`, so it is `Int`. Nothing needed the default, every literal has one type, and the text form prints each with it: `0:Int`.

### `for`

`for k, $v <- $m: block` in a widget block is `For { coll: $m, block: \|k, $v| @widget: ..., kind, keyed }`, where the block's parameters are the loop's binders (the shorthand `for $m:` is the same node with the parameters `key` and `$_`), and `kind` is the consolidation read off the block's result:

| Block result | Consolidation | Type |
|---|---|---|
| plain `R` | `Plain` | `$(K ==> R)`, changing when the key set changes |
| `!R` | `Event` | `!(K ==> R)`, firing with the entries that fired |
| `$R` | `Dynamic` | `$(K ==> R)`, changing when any entry or the key set changes |
| `@R` | `Widget` | an error: a block returns a built widget's result, not an unbuilt widget |
| a bundle | one `For` per field kind | the block is split into projections, as for `dyn` |

Over an array the same node has `keyed: false` and `K = Nat`. Outside a widget block, `for` over a plain collection is a plain mapping and elaborates to an ordinary `App` of the prelude's `MapValues` or `Map`, with the binders (or `key` and `_` for the shorthand) as parameters of the body lambda; over a dynamic collection outside a widget block it is the error the language chapter gives.

### Build-time and occurrence-time marks

Every binding of a widget block gets a `Mark`. The rule is one line: a binding is `Occ` when the root node of its definition is a lift (`DMap`, `Map`, `Tag`, `Leftmost`, `Switch`, `Join`, `Updated`, `Distinct`, `HoldFirst`, or a `For` with `Event` or `Dynamic` consolidation); it is `Build` otherwise (a `Bind`, a `Hold`, a `Placeholder`, a `Let` of a plain value, of a `Const`, of a widget, of a function). A `Build` binding's step runs once when the node is built; an `Occ` binding is a graph node the runtime recomputes. A `Build` binding may contain `Occ` subexpressions, as `@Label "Count: {$count}"` contains a `DMap` as its argument; the mark is on the binding and says when the binding's own step runs, and the nested lift is a node the closure compiler constructs on the way.

Why the mark is on the tree and not left for the compiler to recompute: it is a meaning. `let x = ExpensiveF a` runs once per build and `let $y = ExpensiveF $a` runs once per change, and a reader of the text form should see which without re-deriving the lifting rule. A binding that moves between marks is the kind of regression a golden catches and a reviewer reads.

### The causality check

A cycle in a widget block is legal exactly when it passes through a delayed edge, and the elaborator checks it over the resolver's groups with the labels finalised. The delayed edges, stated as things in the typed AST:

- a dynamic in the `dyns` of a `Tag` (read before the occurrence);
- the `events` of a `Switch` (the new inner is adopted at commit);
- a hold's value side, which is what a tag reads, so a hold referenced through a tag is the common case of the first; the hold's change *event*, which a `DMap`, a `dyn` or a `for` over the hold reads, is prompt;
- the child list consumed by the responses of the same `intercept` (routing against the opened list).

Everything else is prompt, and three things are prompt that a careful programmer might hope were not: a `dyn`'s event (it fires in the occurrence of the rebuild); a `DMap` over a hold (it reads the change event); and the dynamic of a `dyn` or a `for` (the rebuild is in the occurrence of the change). The negative golden for a cycle through `dyn` alone pins the first.

The `for` edge is the one the resolver could not label. The collection's change leads promptly to the consolidated event only through what a newly born block fires at birth, because a block that existed before the occurrence contributes to the merge on its own schedule and a block born in n cannot receive a response in n. So the edge from `coll` to a `For` with `Event` consolidation is `Prompt` if the block's result event is *birth-firing* and `Delayed` otherwise, and birth-firing is a syntactic property of the typed block, computed bottom-up:

- `Started` is birth-firing; a `Dyn`'s event is birth-firing; `Updated` of a hold whose event is birth-firing is birth-firing;
- `Map`, `Tag` and `Leftmost` are birth-firing if any event operand is; `Match` on an event likewise;
- `Switch` is never birth-firing (the creation-occurrence firing is swallowed); an element's response is never (routing against the opened list); a bound widget's result event is birth-firing if the callee's block says so, which the elaborator computes per widget function once and records on the function; a `For`'s consolidated event is birth-firing if its block's is;
- a widget whose body is not visible, because it is a parameter `@a`, is assumed birth-firing, and the diagnostic for a cycle through such an edge says "assumed to fire at build; pass a widget whose result is held or switched".

Under this rule the ordinary todo list is legal: its rows return events derived from element responses, which are not birth-firing, so the edge from `$items` to `!deletes` is delayed and the cycle `$items -> for -> !deletes -> hold -> $items` has its delayed edge. And `@ Delete on !started` as a row result is a compile error: `Delete on !started` is a `Map` over `Started`, birth-firing, so the edge is prompt, and the cycle through the hold's update event is all prompt. The error lists every binding in the cycle in order with spans and says which edge made it prompt.

The check itself: for each cyclic group, take the subgraph of its members with every edge labelled, remove the delayed edges, and look for a cycle among what remains with a depth-first search. A cycle found is the error; its members are reported in the order the search met them, starting from the member whose span is earliest. Cycles that pass through other widget functions, where a widget binds a child whose result feeds back into something the child was given, are not visible to a per-block check and are reported by the runtime's re-entry check with the spans the compiler attached to every cyclic placeholder, which is the division of labour the specification states.

Suppose there were no static check and every cycle were left to the runtime. The runtime does find them, and names the node; the cost is that the counter's author who writes `$count <- 0 then $count + 1` (no event: a derived dynamic reading itself promptly) learns about it at the first occurrence instead of at compile time, and learns the name of one node instead of the list of bindings. The static check is cheap because the resolver already computed the groups and the elaborator already placed the lifts, and a per-block check is the whole of what the typed AST can see.

### Exhaustiveness and the catch-all

A `match` on a closed `choice` must cover every alternative or end in a catch-all; one on `?T` must have `Some` and `Nil` or a catch-all; one on an array must cover every length (see below); one on a literal type must end in a catch-all; one on an extensible variant must end in a catch-all arm, `ignore:` or a bare name, because the set of tags is open. The elaborator records `Dispatch` on the `Match` node so that the closure compiler knows which test to emit, and checks coverage with the usual matrix algorithm for nested patterns (`Some (Circle r)` under `?Shape`), reporting the first missing constructor by name. Arms after a catch-all are "unreachable arm" errors rather than warnings, because an unreachable arm is a bug the reader will otherwise trust.

A guarded arm does not count toward coverage. The matrix is built from the unguarded arms only, because the elaborator cannot know the guard will hold; the guarded arms are still typed and still checked for reachability against the unguarded arms above them. So `Some n if n > 0:` followed by `Nil:` is a missing-`Some` error, the message names the guarded arm ("this arm has a guard, so `Some _` is still uncovered; add an arm without one"), and a guarded catch-all, `ignore if c:`, is not a catch-all: a `match` on an extensible variant or a literal type must end in an unguarded one.

Array patterns enter the matrix as constructors by length. `[[]]` is length 0, `[[a, b]]` is exactly 2, and `[[head, ..rest]]` is at least 1 (at least *k* for *k* elements before the `..`). Coverage is checked at the lengths 0 up to one past the longest pattern and then at "longer than that", where only a `..` pattern can match, so `[[]]` with `[[head, ..rest]]` covers every array and `[[a, b]]` with `[[]]` is missing the lengths 1 and 3 and up, reported as the first missing one. Element patterns inside are specialised in the usual way, and the `Dispatch::Array` the match records tells the closure compiler to emit length tests. Record patterns without `..` must name every field; `..` is required when they do not, and an element pattern `Ns.*(..)` always has it.

### The tag-indexed refinement

`type Slot = record: element: Element; !request: Request` is a plain record, and inside a `match` arm on a tag the types of its fields are refined. The rule is small and lives in the elaborator's environment rather than in the type:

- Inside an arm whose pattern is `T(...)` for a declared tag `T`, the scrutinee's binding, and any `Slot` whose `element` was the scrutinee, have `!request` at `T`'s declared request type (or no `!request` if `T` does not request), and `@Element` applied to an element known to be built from `T` returns exactly what `T` declares: `@!R` for `responds: R`, `@$S` for `state: S`, `@((!R, $S))` for both, `@unit` for neither. The elaborator keeps a map from binding to "known tag" that the arm pushes and pops.
- Inside a catch-all arm, or a `Ns.*(..)` arm, `_.!request` is the generic `!Request`, and `@Element _.element` is the generic `@((!Response, $State))`, with `ElemKind::Generic` on the node so the closure compiler emits the slot creation that checks the per-slot types at runtime. The pattern `((!resp, ignore)) <- @Element _.element` in every forwarding interceptor is this case.
- A supplier lambda `\|e| ...` is checked at `fn Element -> @$State` generically, and inside a `match e:` arm on `T` with `state: S`, the arm's widget must return `@$S`; the catch-all arm returns the generic `@$State`, which `@Element e` provides.

The refinement is a checker rule, not a runtime structure: the typed AST records `ElemKind::Known(T)` or `Generic` on each `Element` node, and the runtime's slot creation takes the tag's parts from the table either way. The refinement buys this: an interceptor with one arm per tag it handles is typed at those tags, and one that forwards what it does not know is typed generically and checked at the slot.

`Kind` is a prelude function `fn Element -> Kind` where `type Kind = record: requests: Bool; responds: Bool; state: Bool`, elaborated to a `Kind` node, which the closure compiler turns into a table lookup on the interned tag; a program that wants to know whether it is safe to read a generic slot's state asks `Kind` rather than matching every tag.

### `test` items

A `test counter, @counter: ...` form is typed as a widget and a list of turns, each turn a list of inputs (`click`, `send`, `with`, `push`, `build`, `idle`) and a list of checks (`expect`, `expect_top`, `expect_none`, `expect_top_none`, `expect_count`, `returns`, `fired`, `quiet`, `requested`, `unrequested`, `errors`, `settles`), and the patterns inside them are element patterns typed like any `match` pattern. The elaborated test is a `TItem::Test` the harness of the conformance chapter collects; nothing about it is special to elaboration beyond the prelude types, and that is the point of spelling tests in the language.

## Closure conversion

You have shipped a reactive program with a leak you could not find, and when you found it, it was a closure: a handler captured the thing that owned the handler, and the two kept each other alive after the view was gone. Every framework that lets a closure capture a pointer has this bug waiting, and the usual fix is a collector that walks the heap looking for it. Prism's runtime has no collector, and the reason it can do without one is a rule the compiler enforces mechanically: a closure never holds a strong pointer to anything in its own node's graph. It holds *indices* into the node's environment, and the node holds the environment. Closure conversion is the pass that makes every capture an index, and it is the pass whose output is the first form the closure compiler can run.

### Function ids and capture lists

Every `Lambda` in the typed AST becomes a function in a program-wide table, identified by `FunId`, with an explicit list of what it captures:

```rust
pub struct Function {
    pub id: FunId,
    pub params: Vec<TPat>,
    pub captures: Vec<Capture>,          // in order; the closure's environment is exactly this
    pub body: TExpr,                     // references to captures become Var(Capture(i))
    pub kind: FunKind,                   // Leaf | Widget
    pub span: Span,
}
pub enum Capture { Env(usize), Outer(usize) }   // from the enclosing block's environment, or from the enclosing function's captures
pub enum FunKind { Leaf, Widget }
```

The pass walks each item, assigns ids depth-first, and for each lambda computes its free variables: the names it mentions that are bound outside it. Each free variable becomes a capture, and the body's references to it are rewritten to `Var(Capture(i))`. A free variable that is itself a capture of the enclosing lambda is `Outer(j)`, so that a closure nested two deep reaches the block's environment through its parent's capture list rather than by its own path, which keeps the invariant that every capture is resolvable from the owning node's environment in one step. Top-level names and prelude names are not captures; they are `Global(ItemId)` references resolved from the program table. Constructors and tags are not captures.

`FunKind::Widget` marks a lambda whose body is a widget block: a `\@widget:` function, the block of a `dyn` or a `for`, a supplier's result. Its captures are what the block's bindings can see from outside: the enclosing block's environment entries. `FunKind::Leaf` is everything else, and a leaf's body contains no `Bind`, `Hold` or `Element`, which the lint checks.

**Recursive functions refer to each other by id.** Two top-level lambdas that call each other, or a lambda that calls itself, do not capture each other; a call to a function is `App { f: Global(id) }`, and the program table owns every function. A closure record at runtime is therefore `(FunId, captures)`, where the captures are values, and no closure holds another closure except as a captured *value*, which is a plain function value built from a function id and its own captures, pointing into the program table and never back at a node. There is no way to make two closures own each other because neither holds the other; they hold ids.

### Captures through the owning node's environment

A widget block compiles to a node, and the node has an environment: one slot per binding of the block, in group order, holding whatever the binding produced (an event handle, a dynamic handle, a plain value, a widget value, a function value). An occurrence-time closure compiled from `$count + !delta` needs `$count`, and it gets it as `Env(3)`, say: at build, the closure compiler resolves the index against the node's environment and copies the handle into the closure. The closure then holds an `Rc` to `$count`'s dynamic, and that is fine: the dynamic's node does not hold the closure. The closure is held by the *event node* that `Tag` constructs, which is subscribed to `!delta` and to `$count`'s behavior through *weak* subscriber entries. The strong edges run from the tag node to its inputs; the inputs reach the tag node only weakly. Dropping the widget's node drops the environment, which drops the tag node, which drops the closure, which drops its `Rc` to `$count`; and `$count`'s hold, which was the thing that could have kept the loop alive, holds only a weak reference to the tag node in its subscriber list.

The rule, then, is about direction: a closure captures things bound *earlier* in group order, which are things built earlier, and the graph's reverse edges are weak. Every strong edge points from later to earlier in build order, so there is no strong cycle. The one exception is a cyclic group, where a later binding is referenced by an earlier one, and that is exactly where the resolver placed a placeholder; a placeholder is a weak forward reference owned by the node as a unit, closed to the real node after the group, so the loop's only strong roots are the node's placeholder slots and the loop dies with the node.

Two more rules keep this true for the constructs that build at occurrence time. A `dyn` or `for` block compiled as a widget function captures from the enclosing block's environment, and the child node it builds holds handles *up* into its parent's environment, which is the allowed direction, since the parent owns the child. A supplied widget is built in the interceptor's node and captures from the supplier closure, which was built in the interceptor's block; again up. A child never holds a strong handle into a sibling or into its own descendants, because nothing in the language can name those; the resolver's scoping guarantees it.

Suppose instead the compiler let a closure capture a `Dynamic` handle directly from wherever it found it, including a sibling's node under a `for`. The handle is an `Rc`; the sibling's node holds the dynamic strongly; if the sibling's dynamic is ever subscribed to the capturing closure's node strongly, the pair is a cycle nothing frees. The runtime chapter lists the leak test that pins this: a widget whose occurrence-time closure reads a sibling dynamic, dropped, must free the dynamic's node, which it does only if the capture went through the environment and the subscription was weak.

### Placeholders and closes

The resolver found where the knots are cut; the converter writes the cut into the tree. For each cyclic group, every delayed-edge target gets a `TDef::Placeholder { kind }` binding inserted at the group's start, with `kind` `Event` or `Dynamic` by the name's sigil (`Widget` and plain placeholders do not exist, because a cycle through a widget or a plain value has no delayed edge and was rejected). The original definition keeps its name with a suffix the text form prints (`$count.hold` for a hold, `!x.def` otherwise) and the references that were on delayed edges are rewritten to point at the placeholder binding. After the group's last member, a `TDef::Close { placeholder, real }` binding is inserted. The group's members are reordered into the topological order the cut gave.

The output is the same typed AST with three annotations: `captures` filled on every `Lambda`, `Function` entries in the program table, and the placeholder and close bindings in place. The text form prints all three, so a golden of the converted tree shows every capture list and every cut, which is where the hard bugs at the runtime boundary are and where a reader wants to look.

## The lint pass

A compiler with one intermediate form and several passes over it needs one checker, run after every pass and in every test, so that the pass that breaks the form is the pass that gets blamed. The lint is Prism's `-dcore-lint` (GHC's Core-lint flag, from memory). It is a walk over the converted typed AST that re-derives what each pass claimed and compares. Each check is an invariant with a stated consequence, because an invariant without a consequence is a comment, and a checker that enforces comments is one that gets switched off the first time it is inconvenient.

**Types agree.** Every node's type is the type its children and the typing rules give it: a `Tag`'s type is `!` of its body's result; a `DMap`'s is `$` of its body's; a `Hold`'s is `$` of its initial's type and its event is `!` of that; an `App`'s is the function type's result after unifying the arguments; a `Switch`'s input is `!!T` and its output `!T`; a `Join`'s input is `$$T`. *What breaks if it fails:* the closure compiler emits a step whose operand kinds do not match the runtime constructor's, and the runtime's `Typed<T>` downcast panics in the middle of an occurrence. The lint reports the node and both types before anything is built.

**Every lift is explicit.** No `App`, `Trait`, `Field`, `Index`, `TextCat` or `Match` has an operand whose outermost type is `!` or `$` unless the node is itself the body of a lift and the operand is a parameter of that body. *What breaks:* a dynamic reaches the closure compiler as a plain operand, and the compiled leaf tries to add a `Dynamic` handle to an integer. The lint says "unlifted dynamic operand" with the node.

**Marks are right.** A binding marked `Occ` has a lift at its root; one marked `Build` does not; a `Build` binding reads no `Occ` binding's *value* (it may hold its handle). *What breaks:* a `Build` step runs once with the current value of something that was supposed to change, and the program silently shows its first value forever; this is the bug the marks exist to make visible, and the lint is what makes the marks trustworthy.

**Every placeholder has exactly one close.** For each `Placeholder` binding there is one `Close` naming it, after it, in the same block, with a `real` of the same kind and type. *What breaks:* a placeholder never closed panics in the runtime at the first sample ("cyclic dynamic never closed"); closed twice panics at the second close ("set cyclic dynamic twice"); closed at the wrong type panics at the close. The runtime's typed cyclic handles make the third a Rust compile error in hand-written widgets and the lint makes all three compile errors in compiled ones.

**Nothing samples a placeholder before its close.** Between a placeholder and its close, no binding has the placeholder in a *sampling* position: the dynamic of a `Dyn` or `For`, the `dyns` of a `Join`, the initial value of a `Hold`, the event of a `HoldFirst`, the child or supplier of an `Intercept`, or any plain position. Subscribing positions (the `dyns` of a `Tag`, the `events` of a `Switch`, the `event` of a `Hold` or `Map`, the responses of an `Intercept`) are allowed. *What breaks:* the build reads a value that does not exist yet; the runtime panics. The resolver's cut makes this hold by construction, and the lint is what catches a resolver that cut in the wrong place.

**Every capture is a free variable.** For each `Function`, its `captures` list equals the set of free variables of its body, in order, with no entry that the body does not mention and no mention that the list lacks; each `Env(i)` names a binding of the enclosing block that is bound *before* the function's binding in group order, or is a placeholder of the same group; each `Outer(j)` names a capture of the enclosing function. *What breaks:* a missing capture is a dangling environment index at build, which the closure compiler would turn into a wrong value or a panic; a capture of a *later* binding is a strong forward pointer, which is the heap cycle the whole design exists to prevent. The negative lint fixtures `lint_rejects_a_capture_of_a_bound_name` and the leak test in the runtime chapter are the two ends of this invariant.

**Leaves are pure.** A `FunKind::Leaf` body contains no `Bind`, `Hold`, `Element`, `Intercept`, `Dyn`, `For`, `Placeholder` or `Close`, and no `WidgetBlock`. *What breaks:* a leaf that built something would build it inside the propagation loop with no node to own it, and the runtime has no path for that; the construction rule has nothing to say about an orphan. The lint also checks the converse: a `Widget` function's body is a `WidgetBlock`.

**Build order respects the groups.** The bindings of each block appear in the order of their groups, every group after every group it depends on, and within a cyclic group in the order the cut gave. *What breaks:* a binding's step runs before an environment entry it needs has been filled, and the index is read as uninitialised. The lint recomputes the dependency order from the references and compares.

**Every node has a span.** *What breaks:* an error at runtime says "in `<unknown>`", which is the worst message a compiler can produce, and the one a careless pass produces by constructing a node with a default.

**Every `Element` node names a declared tag or is `Generic`,** and a `Known` tag's fields match its declaration in number and type. **Every `Match` with `Dispatch::Tag` ends in an unguarded catch-all,** and every guard is a plain `Bool`. **Every `Trait` node is at a concrete type or has a `Dict` in scope.** Each is a restatement of an elaboration rule, and each exists because a later pass could drop the annotation the rule produced.

The lint runs in every test of the compiler and after every pass in `prism dump --lint`; it has negative fixtures of its own, hand-edited converted trees that it must reject, one per check, because a checker nobody has seen fail is a checker nobody knows is on. The checks are cheap: one walk, with a map from binding to type and a stack of placeholder states. A checker that was expensive would be run only sometimes, and a checker run sometimes is where the bug it would have caught waits.

What the lint is not: a second elaborator. It does not infer; it compares what the tree says against what the tree's children say, and when the two agree it says nothing. If a rule of elaboration changes, the lint changes with it, and the two are kept in one crate so that they cannot drift apart unnoticed; a divergence between them fails every golden at once.

## The closure compiler

The last stage is the one with the least to decide and the most to get exactly right, because it is where the compiler meets a runtime that was written without it. The runtime asks for two things from a language: builder calls at build time, and Rust closures the graph calls at occurrence time. A compiled widget has to be a widget the runtime cannot tell from one written by hand, a compiled closure has to meet the closure bounds the constructors state, and nothing the compiler produces may hold the runtime's objects in a way the ownership rules forbid. The closure compiler is one walk over the converted typed AST that turns each node into one of those two things, and what follows is the walk, node by node, with the Rust types it produces.

### `IrWidget` and the program

```rust
pub struct Program {
    pub functions: Vec<CompiledFunction>,    // by FunId
    pub blocks: Vec<CompiledBlock>,          // by BlockId: the widget blocks
    pub tags: Rc<Tags>,                      // the interned tag table, handed to the runtime at build
    pub constants: Vec<Value>,               // literals and other plain constants, boxed once
}

pub struct CompiledBlock {
    pub steps: Rc<[Step]>,                   // in group order
    pub env_size: usize,
    pub result: Operand,
    pub span: Span,
}

pub struct IrWidget {
    pub program: Rc<Program>,
    pub block: BlockId,
    pub captures: Rc<[Value]>,               // the values the block captured from its enclosing node
    pub span: Span,
}

impl PolymorphicWidget<Value> for IrWidget {
    fn build(&self, builder: &mut WidgetBuilder) -> Value {
        let block = &self.program.blocks[self.block];
        let mut env: Vec<Option<Value>> = vec![None; block.env_size];
        for step in block.steps.iter() {
            run_step(step, &self.program, &self.captures, &mut env, builder, self.span);
        }
        read(&env, block.result)
    }
}
```

`IrWidget` is one Rust type for every compiled widget: the root `@main`, a `\@widget:` function's body once applied, the block of a `dyn`, the block of a `for`, the widget a supplier returns. It implements the runtime's `dyn`-compatible widget trait at `Value`, so a `dyn` can rebuild into a different block on each rebuild and compiled widgets and Rust widgets sit side by side in one node tree. The runtime builds it by calling `build` with a builder for a fresh node, which is what it does for every widget, and gets back the block's result as a value. The two things that distinguish a compiled widget from a hand-written one, the program table and the environment vector, are inside `build` and invisible outside it.

A widget *value* in the language (a `let @b = @Button $"OK"`, the result of applying a widget function, the thing a supplier returns) is an `IrWidget` boxed as `Rc<dyn PolymorphicWidget<Value>>` and stored as a plain value; binding it twice builds two nodes because `build` is called twice with two builders, which is exactly what "a widget is a description, a node is what one bind builds" asks for.

### The environment

A block's environment is a `Vec<Option<Value>>` with one slot per binding of the block, indexed in group order, plus the captures vector for what the block sees from outside. Every operand in a step is an `Operand`:

```rust
pub enum Operand { Env(usize), Capture(usize), Const(usize), Global(ItemId) }
```

`Env(i)` is a binding of this block, filled by an earlier step (or a placeholder, filled at the group's start); `Capture(i)` is an entry of the `IrWidget`'s captures; `Const(i)` is a boxed literal in the program's constant pool; `Global(i)` is a top-level definition, a function value or a plain constant computed once per program. The `Option` is there so that a read of an unfilled slot is a panic with the step's span in debug builds, rather than a wrong value; the lint's build-order check is what makes the panic unreachable, and the `Option` is what makes a lint bug loud instead of quiet.

The values in the environment are the runtime's `Value`: events and dynamics as handles (heap values holding an `Rc` to the runtime object), plain data as plain data, widgets as boxed `IrWidget`s, functions as closure records. Everything a step produces goes into the environment; everything a step consumes comes from it. There is no stack, no register file and no frame for construction code, because construction code runs once and its locals are the node's values.

### Steps

```rust
pub enum Step {
    Let       { dst: usize, expr: LeafId },                                  // a plain value, computed once at build
    Const     { dst: usize, value: Operand },                                // Dynamic::constant
    Bind      { dst: usize, widget: Operand, pat: Option<PatId> },           // builder.bind(&widget)
    Hold      { dst: usize, init: Operand, event: Operand },                 // Dynamic::hold(runtime, init, event)
    HoldFirst { dst: usize, event: Operand },                                // Dynamic::hold_first(event)
    Map       { dst: usize, event: Operand, f: LeafId, captures: Vec<Operand> },
    Tag       { dst: usize, event: Operand, dyns: Vec<Operand>, f: LeafId, captures: Vec<Operand> },
    DMap      { dst: usize, dyns: Vec<Operand>, f: LeafId, captures: Vec<Operand> },
    Leftmost  { dst: usize, events: Vec<Operand> },
    Never     { dst: usize },
    Switch    { dst: usize, events: Operand },                               // switch_hold
    Join      { dst: usize, dyns: Operand },                                 // Dynamic::join
    Updated   { dst: usize, dynamic: Operand },                              // dynamic.event()
    Distinct  { dst: usize, dynamic: Operand, eq: LeafId },
    Started   { dst: usize },
    Dyn       { dst: usize, dynamic: Operand, block: BlockId, captures: Vec<Operand> },
    For       { dst: usize, coll: Operand, block: BlockId, captures: Vec<Operand>, kind: Consolidation, keyed: bool },
    Element   { dst: usize, elem: Operand, kind: ElemKind },                 // builder.element(..)
    Intercept { dst: usize, child: Operand, responses: Operand, supplier: Operand },
    Placeholder { dst: usize, kind: PlaceholderKind, name: Span },           // add_cyclic_event / _dynamic
    Close     { placeholder: usize, real: usize },                           // close(handle, real)
    Project   { dst: usize, src: usize, field: usize },                      // a bundle's field after a bind
}
```

One `Step` per typed-AST node kind, each one call on the builder or one constructor on an event or dynamic, with operands resolved to environment indices. The mapping, which is the whole of how the language meets the runtime:

| Typed AST | Step | Runtime call |
|---|---|---|
| `Bind` of a widget | `Bind` | `builder.bind(&widget)`; the child's node is pushed onto this node's children; the result goes into `dst` |
| `Hold { init, event }` | `Hold` | `Dynamic::hold(init, event)`: value `init` until the event fires, commits at the end of each occurrence the event fires |
| `HoldFirst` | `HoldFirst` | a hold whose initial value is the event's catch-up value in the build occurrence; from `@HoldFirst` written in a program, or inserted for a dynamic-of-widgets bind |
| `Map { event, body }` | `Map` | `event.filter_map(closure)` with a closure that always returns `Some` (one node; `filter_map` is the runtime's primitive and `map` its special case) |
| `Tag { event, dyns, body }` | `Tag` | `event.tag_map_n(dyns, closure)`: the closure receives the firing and each dynamic's value *before* the occurrence; n-ary over the dynamics so that `$a + $b + !e` is one node |
| `DMap { over, body }` | `DMap` | `Dynamic::map_n(dyns, closure)`: a derived dynamic, prompt, one node for any arity |
| `Leftmost` | `Leftmost` | `Event::leftmost(vec)` |
| `Never` | `Never` | `Event::never()` |
| `Switch { events }` | `Switch` | `events.switch_hold()`: non-prompt; adopts the new inner at commit |
| `Join { dyns }` | `Join` | `dyns.join()`: prompt on both outer and inner |
| `Updated { dynamic }` | `Updated` | `dynamic.event()`: the change event is a value the dynamic already has; no node is built |
| `Distinct { dynamic, eq }` | `Distinct` | a derived dynamic whose change event is the source's filtered by `eq old, new` against the value before the occurrence; one node, with the equality closure |
| `Started` | `Started` | the builder's done event filtered to `unit`: fires once at `height + 1` in the build occurrence |
| `Dyn { dynamic, block }` | `Dyn` | `dynamic.map(|v| IrWidget { block, captures: captures + [v] }).dynamic_widget()`: a dynamic of compiled widgets handed to the runtime's `dynamic_widget`; its event is the `dyn`'s result event |
| `For { coll, block, kind }` | `For` | `coll.list_widget(|key, per_key| IrWidget { block, captures: captures + [key, per_key] }, kind)`: one child node per key, consolidated by `kind` |
| `Element { elem, kind }` | `Element` | `builder.element(elem_value, request)`: mints the path, creates the response output if the tag responds, asks the supply context for the state dynamic if it has state, stores the request event if it requests; returns what the tag's parts imply |
| `Intercept` | `Intercept` | `builder.bind(&InterceptWidget { child, responses, supplier })`: the child's node is marked captured, its list is the second result |
| `Placeholder` | `Placeholder` | `builder.add_cyclic_dynamic(name)` or `add_cyclic_event(name)`, returning a typed handle; the placeholder's dynamic or event goes into `dst` |
| `Close` | `Close` | `handle.close(real)`: the one write to a placeholder |
| `Kind { elem }` | a `Let` whose leaf reads the tag table | `tags[elem.tag]`'s three flags as a record |

The runtime's target API has three constructors in the table for the compiler's sake: `hold_first`, `map_n` over any number of dynamics (so that a lift over three dynamics is one node and not a tree of `map2`s that allocate pairs), and `tag_map_n` over a vector of dynamics for the same reason. Each is a small generalisation of what exists and each exists because the compiler would otherwise emit several nodes for one lift, and the text form, which prints one node per lift, would then be a lie about the graph.

**Build-time versus occurrence-time, in the steps.** A `Build` binding is a step that runs once inside `build` and writes its slot: `Let`, `Const`, `Bind`, `Hold`, `Placeholder`, `Close`, `Element`, `Intercept`, and the constructors that make a node without a closure. An `Occ` binding is a step that constructs a graph node holding a closure, and the closure is the thing that runs at occurrence time: `Map`, `Tag`, `DMap`, `Distinct`. The step itself still runs once, at build, to make the node; what runs per occurrence is the closure inside it. The marks on the typed AST and the two kinds of step line up one to one, and the lint's mark check is what guarantees that a `Build` binding never ends up as a closure the graph calls.

### Leaves

An occurrence-time closure, and a plain expression computed once at build, are both *leaves*: pure code over values. The closure compiler compiles each leaf once, when the program is compiled, into a tree of Rust closures, and the step that needs it refers to it by `LeafId`:

```rust
pub type Leaf = Rc<dyn Fn(&Frame) -> Value>;

pub struct Frame<'a> {
    pub args: &'a [Value],           // the closure's arguments: the firing, the dynamics' readings
    pub captures: &'a [Value],       // resolved at build from the node's environment
    pub program: &'a Program,
    pub depth: Cell<u32>,            // the recursion guard
}
```

Compilation of a leaf is the closure-generation technique in its plainest form: each typed-AST node becomes a closure that calls its children's closures and combines their results, so that evaluation is a tree of calls with no dispatch loop and no instruction stream. `Var(Capture(i))` compiles to `|f| f.captures[i].clone()`; `Var(Param(i))` to `|f| f.args[i].clone()`; `Trait { Add, Int, [a, b] }` to `|f| box_int(unbox_int(a(f)) + unbox_int(b(f)))` with overflow checked and reported as an `Overflow` error; `App { f, args }` to a call through the program table; `Match` to the dispatch the node records, each arm compiled with its pattern as a test-and-bind followed, when the arm has a guard, by the guard evaluated with the pattern's bindings in the frame, so that a false guard falls through to the next arm; `Bool.And` and `Bool.Or` to a closure that calls its second operand's closure only when the first does not decide the result, so an overflow in an operand that was not needed aborts nothing; `If` to a test of the condition that calls only the branch it takes; `Ctor`, `Elem`, `Field`, `Index`, `TextCat`, `Maybe`, `Array`, `MapLit` to the obvious thing over the value representation. Numbers stay unboxed inside a leaf where the Rust compiler can see them, and box at the leaf's boundary. A node whose static type is a type variable moves the one-word value without looking at it, which is erasure, below.

A lambda value inside a leaf is a closure record `(FunId, captures)` built by evaluating the capture operands into a vector; applying it is `program.functions[id]` called with the record's captures as `captures` and the arguments as `args`. The record is a plain value and may be stored in a map, returned from a function or passed to `Fold`. It holds values, never nodes, and so can be copied anywhere a value can.

The closure the runtime receives for a `Map` step is the one closure type for all of Prism:

```rust
move |firing: Typed<Value>| -> Option<Typed<Value>> {
    let frame = Frame { args: &[firing.into()], captures: &captures, program: &program, depth: Cell::new(0) };
    Some(leaf(&frame).into())
}
```

It holds an `Rc<Program>`, a `LeafId` and an `Rc<[Value]>` of captures, all counted at build and only borrowed per occurrence. With the runtime single-threaded there is no `send + Sync` bound and no `Arc`; a compiled closure captures `Rc`s the way a hand-written widget's closure does.

### The recursion depth guard

A plain function that calls itself without end is exhaustion, not an error, and a host stack overflow is not a panic in Rust: it ends the process. The compiler does better by counting. Every call through the program table increments `Frame.depth` and decrements on return, and a depth past the limit (a few thousand, configurable at program load) aborts the occurrence with an `Overflow` error naming the function's span, which is the error chapter's "a conforming implementation does not hang" made concrete. The same counter guards construction: a widget function that binds itself, `@loop: @loop` through a `let`, increments a depth on the builder, and the build past the limit is a `Build` error with the widget's span rather than a stack overflow in `build`. The guard costs one increment per call and is not optional, because the alternative is a process that dies with no span.

### Boxing

One module, `boxing.rs`, is the only place a Rust `u64`, `i64`, `f64`, `bool`, `char`, `Text`, record or closure record becomes a `Value` or comes back. Every leaf's arithmetic goes through `unbox_int`/`box_int` and friends, every element field through `box_plain`, every pattern test through `unbox_ctor`. Today the representation behind `Value` is the runtime's `AnyValue`, which has inline `U32`, `I32` and `Unit` and boxes everything else behind an `Rc`; the values chapter's one-word representation puts 63-bit integers and more inline; and because the module is the only one that knows which, the representation changes behind it with no change to the compiler's walk. A compiled program that reached into `Value`'s representation from anywhere else would be a program that had to be rewritten when the representation changed, and the representation is going to change.

### Attaching spans

Every node the compiler builds gets the span of the binding it came from, through the builder's `with_span`, so that a runtime error names Prism source: a cycle found at a placeholder names the placeholder's binding, a panic in a closure names the lift, an unknown path names the `@Element`, a build failure names the block. The runtime keeps the span on the node and the error chapter's `Error` element reports it in `spans`. A compiler that attached spans only to some nodes would produce errors that say `<unknown>` for the rest, and those would be the errors inside the lifts, which are the ones a programmer needs most.

### The canonical node structure

A hand-written Rust widget and a compiled one must produce the same node structure for the same program, because the conformance suite's paths are built from that structure: `0/#1/0` is the first bind's `dyn`'s second generation's first slot, whichever language wrote the widget. The structure the runtime chapter states, and the compiler follows: each `Bind` is one child node, in step order, with the next `Bind(i)` segment; each `Element` is one slot, sharing the counter with child binds; each `Dyn` is one child node whose generations are `Gen(g)`; each `For` is one child node whose children are `Key(k)`; an `Intercept` is one child node holding the child as its own child; a supplied widget is a child of the interceptor's node under `s/n`. The counter compiled from source and the counter written against the builder by hand give the same list with the same paths, and the milestone that proves the two tracks met is a byte-identical transcript.

### What a compiled widget looks like at runtime

Take the counter again and watch `build` run for the block of `@counter` (the `@Button` and `@Label` bodies are their own blocks, built when bound). The steps in group order, with the environment as it fills:

```
step 0  Const   env[0] = Dynamic::constant("Increment")
step 1  Bind    env[1] = builder.bind(&IrWidget{block: @Button, captures: [env[0]]})  -> the button's !Click (a switch)
step 2  Const   env[2] = Dynamic::constant("Decrement")
step 3  Bind    env[3] = builder.bind(&IrWidget{block: @Button, captures: [env[2]]})
step 4  Map     env[4] = env[3].filter_map(leaf: \ignore -> -1)
step 5  Map     env[5] = env[1].filter_map(leaf: \ignore -> +1)
step 6  Leftmost env[6] = Event::leftmost([env[4], env[5]])            -> !delta
step 7  Placeholder env[7] = builder.add_cyclic_dynamic("count @15:5") -> $count (open)
step 8  Tag     env[8] = env[6].tag_map_n([env[7]], leaf: \d, c -> c + d)
step 9  Hold    env[9] = Dynamic::hold(0:Int, env[8])                  -> $count.hold
step 10 Close   handle(env[7]).close(env[9])                           -> $count is the hold
step 11 DMap    env[11] = Dynamic::map_n([env[9]], leaf: \c -> "Count: " ++ ToText c)
step 12 Bind    env[12] = builder.bind(&IrWidget{block: @Label, captures: [env[11]]})
result  env[12]  (unit)
```

Steps 1, 3 and 12 each build a child node by calling the bound widget's own `build` with a sub-builder, which runs that block's steps; inside `@Button`'s block the one step is a `Dyn` over its argument whose block has one `Element` step, and inside `@Label`'s it is the same with no result. Thirteen steps, four graph nodes with closures (two maps, a tag, a derived dynamic), one hold, one placeholder closed once, three child nodes. After `build` returns, the environment is owned by the node and the steps are never run again for this node; the four closures are called by the runtime at occurrences 1, 2, 3, ... as clicks arrive. Nothing in the compiled program knows that it has been built, and that is the thesis in one trace.

## The typed-AST text form

You have changed one rule in a type checker and found out a month later, from a user, that it changed the meaning of a program you never tested. The fix every mature compiler arrives at is the same: print the elaborated form of every test program, keep the printout under version control, and make any change to it a diff someone reads. The typed-AST text form is that printout for Prism, and it does three jobs at once: it is what the goldens diff, it is what a person reads to see what the compiler made of a program, and it is the bridge to the Haskell frontend, whose dump of a program must be byte-identical to the Rust frontend's. One binding per line, every lift a named node, every number with its type, the mark, the group, the span.

### The format

Each binding of a widget block is one line:

```
name: type [mark] group = expr @span
```

- `name` is the binding's name with its sigil, exactly as written in the source (`!delta`, `$count`); a discarded result is `_`; a cyclic definition is `name.hold` for a hold and `name.def` otherwise; a bundle's projection is `name.field`.
- `type` is the full type with its sigils outermost: `!Int`, `$Text`, `@!Click`, `$(Path ==> Slot)`, `fn Element -> @$State`. Inferred type variables are single lowercase letters named by first appearance.
- `mark` is `[build]` or `[occ]`.
- `group` is `g` followed by the group's number in dependency order; placeholders, their definitions and their closes share the group number.
- `expr` is the expression with every lift explicit, in the node grammar below.
- `span` is `@line:col` of the binding's start in the source file; a block's own line takes the block's span.

A block is printed as its header line, its bindings indented by two spaces, and its result on a final line beginning `@`. Nested widget blocks (the block of a `dyn` or a `for`, a `\@widget:` body) indent again. A placeholder line is `name: type [build] gN = placeholder` and a close line is `close name name.hold` with no type, since it defines nothing. A top-level `let` is `name: scheme = expr @span` with the scheme's bounds in angle brackets where the annotation gave them.

The expression grammar, which is the typed AST's node set in prefix form:

```
expr  := atom
       | 'const' expr                                  # $ v
       | 'dmap' '[' dyns ']' lambda                    # derived dynamic
       | 'map' event lambda                            # event map
       | 'tag' event '[' dyns ']' lambda               # event with dynamics read before
       | 'leftmost' '[' events ']' | 'never'
       | 'switch' expr | 'join' expr | 'updated' expr | 'distinct' expr expr
       | 'hold_first' expr
       | 'dyn' expr lambda                             # lambda's body is a widget block
       | 'for' expr kind birth lambda                  # kind: plain | event | dynamic; birth: birth | no_birth, the can-fire-at-birth flag
       | 'element' (tag '{' fields '}' | expr) parts (expr)?   # parts: the tag's, or 'generic'; then the request if any
       | 'intercept' expr expr expr
       | 'started' | 'kind' expr
       | 'bind' expr                                   # in a Bind line: the widget being built
       | 'hold' expr expr                              # in a Hold line: initial, event
       | 'maybe' '[' exprs ']' lambda                  # ? lifting at the plain layer
       | 'match' expr dispatch '{' arm (';' arm)* '}'   # dispatch: choice | tag | optional | array | literal
       | 'if' expr expr expr
       | name '(' exprs ')' | name '{' field '=' expr, ... '}'  # application; constructor or element
       | trait '[' type ']' exprs                      # Num.Add[Int] c, d   Eq[Text] a, b   dict[Ord a]
       | 'Bool.Not' expr | ('Bool.And' | 'Bool.Or') expr ',' expr   # not a, a and b, a or b (And, Or short-circuit)
       | 'cat' '[' exprs ']'                           # text interpolation
       | expr '.' field | expr '[' expr ']'
       | lambda
atom  := name | literal ':' type | '"' text '"' | 'unit' | '[[' exprs ']]' | '{' pairs '}' | '((' exprs '))'
lambda:= '\' captures? '|' params '|' expr            # captures: '[' names ']' after conversion
       | '\' captures? '|' params '|' '@widget:' block
arm   := pat ('if' expr)? '->' expr                    # the guard, if any, is part of the arm
pat   := the source patterns in prefix form, array patterns as '[[' pats ('..' pat)? ']]'
```

Application prints with parentheses and commas, `Insert(items, id, todo)`, because the text form has no precedence and no paren-less calls: one syntax per node, read by a parser with no table. Literals print with their type, `0:Int`, `1.5:Float`, `"ok"` (text needs none). A `Trait` node prints the operation, the type in brackets, and the arguments, so that `+` resolved at `Int` and `+` resolved at `Float` are visibly different lines. The boolean operators have no type to resolve and print as `Bool.Not b`, `Bool.And a, b` and `Bool.Or a, b`. A guard prints after its pattern, so `match n optional {Some(m) if Ord.Gt[Int] m, limit -> m; Nil -> 0:Int}` is a guarded arm and an unguarded one, and `if c Some(x) Nil` is what `if c: x` prints as.

The `for` node prints the elaborator's can-fire-at-birth flag (the `birth_firing` field of the node) between the consolidation kind and the block: `birth` when the row block can fire in the occurrence its key appears, `no_birth` when it cannot, and `no_birth` on a `plain` kind, whose rows build nothing. The flag is a line of the dump and not a note beside it because the Haskell evaluator chooses its `coincidence` from it and the cycle check reads it; a frontend that computed a different flag would otherwise differ only in an invisible field.

### The counter, dumped

The source, with the two library widgets it uses:

```prism
@Button: fn $Text -> @!Click = \|$label| @widget:
    !!clicks <- dyn $label:
        @Element Ui.Button(label=_)
    @ !Switch !!clicks

@Label: fn $Text -> @unit = \|$text| @widget:
    dyn $text:
        @Element Ui.Label(text=_)
    @ unit

@counter:
    !increment <- @Button $"Increment"
    !decrement <- @Button $"Decrement"
    let !delta = -1 on !decrement | +1 on !increment
    $count <- 0 then $count + !delta
    @Label "Count: {$count}"
```

`prism dump counter.prism` after elaboration:

```
@Button: fn $Text -> @!Click = \|$label| @widget:                                   @1:1
  !!clicks: !!Click [build] g1 = dyn $label \|label| @widget:                          @2:5
    _: !Click [build] g1 = element Ui.Button{label=label} responds                     @3:9
    @ _
  @ switch !!clicks

@Label: fn $Text -> @unit = \|$text| @widget:                                        @6:1
  _: unit [build] g1 = dyn $text \|text| @widget:                                      @7:5
    _: unit [build] g1 = element Ui.Label{text=text}                                   @8:9
    @ unit
  @ unit

@counter: @unit                                                                       @11:1
  !increment: !Click [build] g1 = bind @Button(const "Increment")                     @12:5
  !decrement: !Click [build] g2 = bind @Button(const "Decrement")                     @13:5
  !delta: !Int [occ] g3 = leftmost [map !decrement \|ignore| -1:Int, map !increment \|ignore| +1:Int]   @14:9
  $count: $Int [build] g4 = placeholder                                               @15:5
  $count.hold: $Int [build] g4 = hold 0:Int (tag !delta [$count] \|d, c| Num.Add[Int] c, d)   @15:5
  close $count $count.hold                                                            @15:5
  _: unit [build] g5 = bind @Label(dmap [$count] \|c| cat ["Count: ", ToText[Int] c])   @16:5
  @ _
```

Read it as the compiler's statement of what it made of the program. `@Button` takes a dynamic and the `dyn` inside it is the programmer's; its block has one slot, `element Ui.Button{label=label} responds`, whose tag responds and does nothing else, so the binding's type is `!Click`. In `@counter`, the two binds are build-time and in their own groups. `!delta` is occurrence-time: `on` with a plain left side became a `map` over the event, the two maps are merged leftmost, and `-1` and `+1` are both `Int` because the merge unified them. `$count` is a cyclic group of one, cut at the tag: the placeholder line, the hold line reading `$count` through `tag`, and the close. `+` resolved at `Int` and prints as `Num.Add[Int]`; the hold's `0` prints as `0:Int`. The last line is a discarded bind of `@Label` applied to a derived dynamic, so it is build-time with an occurrence-time node inside. Every line says where in the source it came from.

After closure conversion, the same dump with `--converted` adds the capture lists, which here are all empty except the `dyn` blocks', which capture nothing either, since their argument is the only thing they use:

```
  $count.hold: $Int [build] g4 = hold 0:Int (tag !delta [$count] \[]|d, c| Num.Add[Int] c, d)   @15:5
```

A program whose lift reads a sibling would show it: `\[$total]|d| Num.Add[Int] d, $total` says the leaf captures `$total` from the environment, and the lint has checked that `$total` is bound earlier in group order.

### The `for` flag, dumped

A `for` is the one node whose line says something the source does not: whether its row block can fire in the occurrence a key appears. Two programs differ only in that, and their dumps differ in one word.

```prism
@deletes: fn $(Nat ==> Text) -> @!(Nat ==> Click) = \|$items| @widget:
    !rows <- for $items:
        !delete <- @Button $"Delete"
        @ delete
    @ !rows
```

```
@deletes: fn $(Nat ==> Text) -> @!(Nat ==> Click) = \|$items| @widget:                  @1:1
  !rows: !(Nat ==> Click) [occ] g1 = for $items event no_birth \|_| @widget:             @2:5
    !delete: !Click [build] g1 = bind @Button(const "Delete")                            @3:9
    @ !delete
  @ !rows
```

The row's result is an element's response, which the runtime routes against the list as the occurrence opened, so a new row cannot fire it at birth and the flag is `no_birth`. Replace the `@Button` with a widget that is a parameter of `@deletes`, whose body the elaborator cannot see, and the same line reads `for $items event birth \|_| @widget:`; so does a row that returns the event of a `dyn` or an `@started`-derived event. The flag is also what the cycle check reads for the edge from the collection to the consolidated event.

### The dynamic-of-widgets lift, dumped

The one place the elaborator inserts a `dyn` the programmer did not write is worth a golden of its own, because its two forms are easy to get wrong:

```prism
@panel:
    !click <- @Element Ui.Button(label=$label)
    $text <- @TextInput $initial
```

```
@panel: @unit                                                                         @1:1
  !click: !Click [occ] g1 = switch (dyn (dmap [$label] \|l| Ui.Button{label=l}) \|e| @widget:     @2:5
    _: !Click [build] g1 = element e generic
    @ _)
  $text: $Text [occ] g2 = join (hold_first (dyn $initial \|i| @widget:                @3:5
    _: $Text [build] g1 = bind @TextInput(i)
    @ _))
  @ unit
```

The first line is the lift of a dynamic field: the element construction became a `dmap` giving a `$Element`, `@Element` over it became a `dyn`, and the bound event is the `switch` of the `dyn`'s event, which is a graph node and so marks the binding `[occ]` even though the `dyn` inside it is construction. The inserted block's `element` is `generic`, because inside the block `e` is a plain `Element` whose tag the checker cannot see through the `dmap`; the runtime checks the slot's types per tag, as it does for every generic element. The second line is a widget function returning a dynamic, applied to a dynamic: `join` of a `hold_first` of the `dyn`, with the first build's `$Text` as the hold's initial value. A frontend that printed `hold` with some other initial, or `switch` for the second line, has read the specification differently, and the golden is where the difference shows.

### One definition for printer and parser

The printer and the parser of the text form are generated from one definition, so that the two cannot drift: a table that lists each node kind, its keyword, and its operand kinds in order, from which `print(&TExpr) -> String` and `parse(&str) -> TExpr` are both derived. In Rust the table is a `derive` on the typed AST enum with attributes naming each variant's keyword, and the two functions are the derive's output; in Haskell the same table is a datatype with a pretty-printer and a parser written from one `Grammar` value. The test is a round trip: print every fixture, parse the result, compare to the original tree; parse every golden, print it, compare to the file. A text form whose printer and parser were written separately would have cases the printer emits and the parser rejects, and the first such case would be found by the Haskell frontend's author, who would then not know whether the specification or the parser was wrong.

The parser of the text form exists for two consumers: the round-trip test, and hand-written fixtures. A fixture is a typed AST written by hand in this syntax, before the frontend can produce it, to drive the closure compiler against the runtime: the counter's converted dump, a `dyn` button, a `match` on a choice, a `@started` request. They are the closure compiler's first tests, and when the frontend arrives the compiled counter has to dump identically to the hand-written one, which is the check that the two were built against one reading.

### What the text form is not

It is not a bytecode and not a serialisation for shipping compiled code. It has no version, no constant pool and no load-time validation, and it refers to tags and functions by name. If compiled code ever has to outlive the process that compiled it (precompiled libraries, a browser bundle without the frontend, programs sent to a World that did not compile them), the format is a serialisation of this same tree with a version and a load-time check, and the loader decodes back into the tree the closure compiler already takes. That is one more printer and one more parser from the same definition, not a second intermediate form.

It is also not pretty. There is no layout, no precedence, no paren-less calls, one syntax per node. A reader who wants the program reads the program; a reader who wants to know what the compiler made of it reads this, and every decision is on its own line.

### Freezing

The form changes while the frontend is being written and that is fine; it stops changing before the Haskell frontend starts, and from then on a change to the form is a change to both frontends and to every golden. The file that defines the table is the one place the form is defined, and a comment at its head records the date it was frozen, so that a divergence between the two frontends is never a question of which version of the form each implemented.

## Elements and tags in the compiler

An element is the one value in the language that crosses to the World, and everything the World knows about it comes from a tag declaration the compiler read. The tag table is therefore the compiler's most consequential data structure after the typed AST: it decides what an element's fields are, what `@Element` returns, what a `match` arm binds, what the runtime checks at the boundary, and what a World is told it may accept. This section is how declarations become the table and how each use of an element compiles against it.

### The tag table

```rust
pub struct Tags { pub by_id: Vec<TagInfo>, pub by_name: HashMap<Vec<Name>, TagId> }

pub struct TagInfo {
    pub id: TagId,                    // interned: a u32, dense, assigned in declaration order
    pub name: Vec<Name>,              // ["Ui", "Button"]
    pub namespace: Vec<Name>,         // ["Ui"]: the prefix, for Ns.* tests
    pub of: TypeId,                   // the extensible variant it belongs to
    pub fields: Vec<(Name, Type)>,    // concrete types only
    pub requests: Option<Type>,
    pub responds: Option<Type>,
    pub state: Option<Type>,
    pub eq: bool,                     // every field has equality (always true: fields are plain)
    pub span: Span,
}
```

`tag Ui.Button of Element:` with `label: Text` and `responds: Click` becomes one row. The elaborator checks, at declaration: every field type is concrete (no type variable, with the diagnostic naming the field and the variable); every field type has equality, which plain types always do, so that element equality is structural; the three parts are each at most once and their types are plain data (a `requests: !R` is written with its sigil in the declaration and stored as `R`, the event being implied); and no two tags share a name. The table is interned once per program, and the ids are what the runtime's `ElementValue { tag, fields }` holds. The same table, as `Rc<Tags>`, is handed to the runtime at `build`, which consults it for which parts a tag has and the response and state types to check at the boundary, and a World's `World.Tags` state element is made from it by the world core.

There is one tag table per program because there is one program: modules beyond one file plus the prelude are deferred, and until they exist every tag is declared where the compiler can see it. The design does not depend on that. Ids are dense in declaration order so that a later module system can intern across files with an offset, and `namespace` is stored as data rather than expanded into sets, so that a `Ns.*` test compiled before a module added a tag still matches the new tag. The alternative, assigning ids by hashing the name so that separately compiled modules agree, is the thing to do when separate compilation arrives, and nothing in the compiled form would change but the `by_id` vector.

### Construction

`Ui.Button(label="OK")`, `Ui.Button "OK"` and the block form all elaborate to `Elem { tag, fields, request }`: a plain record build with the tag id, fields in declaration order (named arguments reordered, positional checked for count), each field checked against its declared type with lifting over `?` stopped at the field (an element's field is plain, and `Ui.Row(height=xs[i])` is the diagnostic). A `requests` part, if the tag has one, is given as a field named `requests` holding an event, `Cli.Stdout(requests=!line)`, and is stored separately on the node because it is the one part of an element that is not plain data: it is the program's event, which the slot subscribes to, and it is excluded from the element's equality. The compiled leaf builds an `ElementValue { tag, fields: Rc<[Plain]> }` through the boxing module and pairs it with the request event handle.

A tag with no fields is a bare name, `Clock`, and constructs the same way with an empty field vector. A field whose argument is a dynamic lifts the construction into a `DMap`, and `@Element` over the result lifts into a `dyn`; the compiled element inside the `dyn`'s block is `Generic`, since the block sees a plain `Element`.

### `@Element`

`@Element e` compiles to `Step::Element { elem, kind }`, which calls `builder.element(value, request)`. The runtime mints the slot's path, creates the response output if the tag responds, asks the supply context for the state dynamic if the tag has state, stores the request event if the tag requests, and returns `ElementResult { response, state }`. What the step writes into the environment depends on `kind`:

- `Known(T)`: the compiler knows the parts and writes exactly what the type says: the response event for `responds`, the state dynamic for `state`, a tuple of both, or `unit`. The types were checked at elaboration against `T`'s declaration and nothing is checked at runtime.
- `Generic`: the element's tag is a runtime value, the type is `@((!Response, $State))`, and the step writes the pair of the response event and the state dynamic as the runtime returns them: a `never` for a tag that does not respond, and for a tag with no state a dynamic that nothing can read, because the generic `$State` is opaque and the only thing a program can do with it is hand it back to a slot or discard it. The runtime checks, per slot, that a response delivered to the path has the tag's response type and that the supplied state has the tag's state type, which is the "checked at runtime per slot" of the elements chapter.

The step also writes the slot's path nowhere, because a program cannot name a path; paths are the runtime's and the World's.

### `match` dispatch

A `match` compiles by the `Dispatch` the elaborator recorded on it, and a `match` on an extensible variant is the `Tag` case:

```rust
pub enum Dispatch {
    Choice(TypeId),    // a jump table over the alternative index
    Tag,               // see below
    Optional,          // Some / Nil
    Array,             // length tests: empty, exactly n, at least n
    Literal,           // sequential tests
}
```

For `Tag`, the compiled leaf reads the scrutinee's `tag` id and tests arms in order: an arm `T(...)` is `tag == T`; an arm `Ns.*(..)` is `tags[tag].namespace starts with Ns`, a slice comparison on the interned namespace path, or a bitset per `match` precomputed over the table when the table is closed; the catch-all is last and always present, by the elaborator's rule. A tag-specific arm then binds fields by index from `fields`, which the declaration fixed, so there is no name lookup at runtime. The first matching arm is taken, as the language says; a tag that matches no specific arm and no namespace reaches the catch-all, which is where a forwarding interceptor forwards.

Why arms test in order rather than by a hash of the tag: a `match` with two tag arms and a catch-all is the common case, three comparisons, and a hash table per `match` would cost more to build than it saves. The `Choice` dispatch is a table because a closed choice's alternatives are dense indices; tags are dense too, and a `match` with many tag arms can use the same table, which is an optimisation the closure compiler may apply when the arm count passes a threshold and that changes nothing observable.

### `Ns.*`

A namespace wildcard compiles to a prefix test on the interned tag's namespace path. It appears only in patterns, so there is no value of "a namespace" at runtime and no `Ui` that could be passed around; the test is compiled into the `match` that wrote it. Nested namespaces match by prefix: `Ui.*` matches a hypothetical `Ui.Text.Input` because `["Ui"]` is a prefix of `["Ui", "Text"]`.

### `Kind`

`Kind e` is the prelude function that answers which parts `e`'s tag has, as a record of three booleans, and compiles to a leaf that reads `tags[e.tag]`. It exists so that generic code with a `Slot` whose tag it has not matched can decide, without a `match` arm per tag, whether to read the state dynamic `@Element` gave it; a sandbox that denies every request but passes every state element through is written with `Kind` and one `if`. The tag table is known to the compiler, so the function is a lookup, and it is the one place a program can ask a value about its type, which is allowed because a tag is data, not a type.

### `intercept`

`intercept @child, !responses, $supplier` compiles to `Step::Intercept`, which binds an `InterceptWidget` holding the compiled child (an `IrWidget` value from the environment), the responses event handle and the supplier dynamic handle, and writes the pair `((child's result, child's element list))` into the environment, with the list as a `$(Path ==> Slot)`. The supplier is a `$(fn Element -> @$State)`: a dynamic whose value is a closure record; the runtime, when a state slot is created under this interceptor, reads the closure record from the dynamic, applies it to the element through the program's call entry point, and gets back an `IrWidget` (the `@$State` is a widget value), which it builds as a child of the interceptor's node. The compiler's part of that is to make sure a supplier lambda compiles to a `Leaf` that returns a widget value, which the lint's "leaves are pure" rule allows because returning an unbuilt widget is not building.

The tag-indexed refinement of `Slot` fields and supplier results is a checker rule and has no runtime structure: a `Slot` at runtime is a record of an `ElementValue` and an event handle, whatever arm it is read in.

### Erasure

Generic code is compiled once and runs on values whose types it does not know. A function `Swap<a, b>` moves two one-word values; `Max<a: Ord>` compares two values with the dictionary it was handed; `Fold` applies a closure record. No type descriptor exists at runtime, no monomorphisation pass instantiates `Swap` per use, and no runtime type check happens anywhere in compiled code except at the two boundaries the specification names: a response or state value arriving at a generic slot, checked against the tag's declared type, and a value arriving from a World, checked by the runtime's run call.

The rule that makes erasure sound is the one on tag fields: a tag's fields have concrete types, so a value built in generic code never needs a type it does not have. Suppose that rule were dropped and `tag Test.Box of Element: value: a` were allowed. Then `let Wrap: fn a -> Element = \Test.Box(value=_)` would build an element whose field type depends on `a`, and a `match` arm `Test.Box(value=v)` elsewhere would bind `v` at a type the arm cannot know, and the only ways out are a runtime type descriptor on every value (which is a second type checker in the runtime), descriptors reified where a generic value enters a record (which makes `Wrap` take a hidden argument every caller must supply), or whole-program monomorphisation (which forbids separate compilation and polymorphic recursion). Passing descriptors generally is the hypothetical; the concrete-field rule is the design, and what it buys is that a value is one word and a type is a compile-time thing everywhere except the two boundaries.

The three traits are the one case where generic code needs something at runtime, and it gets a function value, not a descriptor: `Dict { Ord, a }` is a closure record passed as a hidden first argument, resolved at the call site where `a` is concrete. A caller at `Text` passes the text ordering; the callee calls it. Nothing inspects a value; the value's static type at the call site chose the function.

What erasure costs: a generic library cannot wrap its own type parameter in an element, and must take elements already built or be written at a concrete type. The diagnostic names the field and the variable, and the fix is to build the element where the type is known, which is where the World will need it to be known anyway.

## Diagnostics

A language whose syntax leans on spacing, sigils and inference is a language whose errors are part of its design. The mistakes a newcomer makes are predictable, and each one has a right answer the compiler can name. The language chapter fixes the content of each message; this section is the compiler's side: where each is detected, what it needs to have kept to say it well, and the exact rendering, which the negative goldens pin byte for byte. The rendering is one format for every diagnostic:

```rust
pub struct Diagnostic {
    pub level: Level,                  // Error | Warning
    pub message: String,               // one line, lowercase start, no period
    pub primary: (Span, String),       // the span under the caret, with its label
    pub secondary: Vec<(Span, String)>,
    pub notes: Vec<String>,            // the `=` lines: what the rule is and what the fix is
}
```

The test for every diagnostic is that the message contains the fix. A message that says what is wrong and not what to write is a message the programmer has to look up, and the ones below are the ones that are looked up most.

**Two events in one operation.** Detected in elaboration's lifting table when an operation or application has two operands whose outermost kind is `!`. Needs: the span of each operand and of the operator, kept by the parser on `Binary` and `Call`.

```
error: two events in one operation
  --> counter.prism:4:17
   |
 4 |     let !delta = !increment + !decrement
   |                  ^^^^^^^^^^   ^^^^^^^^^^ both are events
   = an operation may lift over any number of dynamics and at most one event
   = to fire on either, merge them: `!increment | !decrement`
   = to fire on both in one occurrence, hold one and sample it at the other
```

**A cycle with no delayed edge.** Detected by the causality check over a group. Needs: every binding in the cycle in order, the span of each, and the edge that made it prompt.

```
error: these bindings depend on each other in the same occurrence
  --> loop.prism:3:5
   |
 3 |     let $a = $b + 1
   |         ^^ depends on $b
 4 |     let $b = $a * 2
   |         ^^ depends on $a
   = a cycle in a widget block must pass through a hold's value, a tag, or a !Switch
   = a `dyn` is not a delay: its event fires in the occurrence of the rebuild
```

A cycle through `dyn` alone gets the same message with the last note as its reason, and a cycle through a `for` whose row fires at birth adds: "`!deletes` fires when a key appears, because its row's result is `Delete on !started`; a row that fires at birth is a prompt path from the collection".

**A negative number where a subtraction was meant.** Detected in elaboration when a `Call` whose head is a plain value fails to unify with a function type and the first argument is a prefix `-` literal. Needs: the parser's `prefix` flag preserved on the literal's sign and the argument's span.

```
error: `a` is not a function
  --> math.prism:7:12
   |
 7 |     let n = a -1
   |             ^ ^^ applied to -1
   = did you mean `a - 1`?
```

**A numeric operator at the wrong type.** There is no "cannot decide the numeric type" error: an integer-spelled literal that nothing pins is an `Int` at the end of its item, so `$n <- 0 then $n + 1 on !tick` is an `Int` counter, and a decimal-spelled literal is a `Float` from the start. What remains is the wrong suffixed operator: `+~` on two `Float`s is "operator `+~` is Int addition, but both operands are Float; use `+.` or `+`"; a bare literal in `Int` arithmetic where the other operand is pinned `Int` resolves silently (the literal unifies), and the message appears only when the literal is a `Nat` by annotation: "`+` on a Nat and an Int; write `+?` to convert, or make the literal an Int: `+1`".

**A sigil that disagrees with the inferred kind.** Detected when a binding's inferred type is compared with its written sigils.

```
error: `count` is bound to a dynamic, but its name has no sigil
  --> counter.prism:5:5
   |
 5 |     let count = $total + 1
   |         ^^^^^ a dynamic must be named with `$`
   = write `$count`
```

The reverse, `let $x = 3`, says "a plain value must be named without a sigil, or lifted: `$ 3`"; a name with the wrong sigil says which it is and which it should be.

**A state tag with no supplier.** Detected statically when an `intercept`'s supplier argument is a visible lambda, `$ \|e| match e: ...`, and a child under it creates an element of a state tag that no arm of the lambda handles and the catch-all does not forward with `@Element e`. Needs: the supplier lambda's arms and the set of state tags the child's blocks construct, which the elaborator records per widget function. The runtime reports the same thing at slot creation when the supplier is not visible.

```
error: no supplier for the state element `Fs.Contents` under this interceptor
  --> sandbox.prism:9:30
   |
 9 |     ((r, $slots)) <- intercept @child, !down, $ \|e| match e:
   |                                                     ^^^^^^^^^ handles Clock and denies the rest with `@ $ Unavailable`
12 |             ignore: @ $ Unavailable
   |                     ^^^^^^^^^^^^^^^^ the catch-all returns `Unavailable`, which is not a `Contents`
   = the child builds `Fs.Contents` at build.prism:4:5; supply it, or forward it: `ignore: @Element e`
```

**A match on an extensible variant without a catch-all**, **a non-exhaustive match on a closed choice**, **a match on a dynamic with a widget arm**, **a hold outside a widget block**, **lifting across a sigil boundary**, **an optional where a plain value is required at a boundary**, **a type variable in a tag's field**, **interpolating something without a rendering**, **the layout errors**, **reserved words**, **`for` over a dynamic outside a widget block**, **`ix` outside a `for`**, **an `if` without `else`**, **a guarded arm that leaves a case uncovered**, **a `..` that is not last in an array pattern**, **a camelCase value name** (the one warning in the list, rendered with `warning:` and the suggested spelling): each renders in the format above with the content the language chapter gives, and each has a negative golden. Two more the compiler adds because its own stages make them possible:

**A trailing lambda that was not last.**

```
error: a trailing lambda takes the rest of the line
  --> fold.prism:3:22
   |
 3 |     let r = Fold xs, \|a, x| a + x, 0
   |                      ^^^^^^^^^^^^^^^^ the lambda's body includes `, 0`
   = parenthesise the lambda to pass more arguments after it: `Fold xs, (\|a, x| a + x), 0`
```

**A capture that would point forward** is a lint error, not a user diagnostic, because the resolver's cut makes it impossible from source; it renders as "internal: capture of a later binding" with the binding and the span, and a program that produces it is a compiler bug to be reported with the dump attached.

Every diagnostic is rendered from `Diagnostic` by one function, so that the format cannot vary between stages, and the negative goldens compare the rendered text, so that a change to the renderer is a diff in every negative golden at once, which is the right size for a change to how every error looks.

## Testing the compiler

Six kinds of test cover the crate, and every one of them is a text file someone can read in a diff.

### Goldens

Each golden is a Prism file and its typed-AST dump after elaboration, and after conversion with `--converted`. Any change to elaboration shows up as a diff a reviewer reads. The set: the counter; each program of the conformance corpus; the GUI library; the worked examples of the first chapter; and one small file per rule of the lexical and expression chapters, including the paren-less call cases (`F a, G b, c`, `F a, (G b, c)`, `G(b, c)`), tuples and arrays (`((G b, c))`, `(((G b, c), d))`, `((Size $elems) + 1)`, `[[]]`), function types (`fn (fn A -> B), [A] -> [B]`, `fn ((A, B)) -> C`), a three-step chain on continuation lines, the boolean levels (`not a == b`, `a < b and c < d`, `x or y and z`), a guarded arm and an exhaustiveness failure caused by one, `for` with named binders and with the shorthand, array patterns of each form, `@HoldFirst` written in a program, `;` in an inline `record:` and an inline `if`, the lambda shorthand (`\.done`, `\[0]`), the spacing cases, the dynamic-of-widgets lift in both forms, `for` in each consolidation, a cyclic group cut at a switch, an interceptor with a tag-specific arm and a catch-all, a generic function with a bound, and a `test` item. What they catch, by name: a lift that changed kind (a dynamic read promptly where it should go through a tag); a literal or operator that resolved to a different numeric type; a binding that moved between `[build]` and `[occ]`; a group split or cut differently; a `for` consolidating to the wrong kind; a capture list that gained or lost an entry; a `dyn` inserted or not inserted at a widget application.

The layout corpus of the language chapter is the lexer's golden: a program in, its token stream out, a program's whole stream printed on one line in the conformance chapter's corpus notation, compared exactly, for both lexers.

### Negative goldens

Each is a rejected program and the exact rendered diagnostic, spans included. They catch errors that get worse silently, which nothing else notices: a message that lost its fix line, a span that moved to the wrong token, a diagnostic that stopped firing because a check moved to a stage that runs later. The set is every diagnostic above, with at least one program each, and for the four that need care from the start (two events, a rejected cycle, `a -1`, the numeric operators) several programs each, covering the variants: a cycle through `dyn` alone, a cycle through a `for` with a birth-firing row, `+~` on floats, `+` on a `Nat` and an `Int`.

### Lint fixtures

Hand-edited converted trees in the text form that the lint must reject, one per check: a placeholder closed twice, a placeholder never closed, a sample before close (`dyn` over a placeholder inside its group), a capture of a later binding, a capture the body does not use, a leaf containing a `bind`, a `[build]` binding whose root is a `map`, a node with a mismatched type, a `match` on a tag with no catch-all. Each fixture is parsed by the text form's parser, which is why the parser exists for hand-written input, linted, and the lint's message compared. A checker nobody has seen fail is a checker nobody knows is on.

### Round trip

Print every golden's tree, parse the printed text, compare the trees; parse every golden file, print, compare the text. Run on every fixture and every golden in CI. A text form whose printer and parser drift is a bridge the Haskell frontend cannot cross.

### End to end: the counter against the hand-written counter

The test where the frontend and the runtime meet. The runtime chapter has the counter written in Rust against the builder by hand, driven by the harness through three turns (build; click Increment; click Decrement), with its transcript. The compiled counter is built from `counter.prism` through every stage of this chapter into an `IrWidget`, handed to the same harness, driven through the same three turns, and its transcript must be byte-identical: the same element list with the same paths (`0/#0/0: Ui.Button(label="Increment")`, `1/#0/0`, `2/#0/0: Ui.Label(text="Count: 0")` after build; `2/#1/0: Ui.Label(text="Count: 1")` after the first click), the same request firings (none), the same results. If the paths differ, the compiler's node structure is not the canonical one; if the label's text differs, a lift or a literal is wrong; if the transcript has an extra turn, something fired at build that should not have. One comparison proves that the two tracks were built against the same reading of the design, and it is the milestone that ends the compiler's first phase.

After the counter, the same harness runs the compiled `@Forward` from the GUI library wrapped around the counter against the hand-written `Forward` fixture, which exercises `intercept`, `for`, `dyn`, generic `@Element` and the supplier; and then the twelve programs of the conformance corpus under the Test World with their scripts, which is the conformance chapter's test and not this one's, but is the first time most of the compiler's node kinds are exercised with real inputs.

The unit tests the closure compiler keeps beside the transcript tests pin the runtime facts compiled code depends on, each named for the sentence it checks: `a_compiled_hold_reads_its_own_old_value_through_a_tag`, `a_compiled_dyn_fires_its_event_in_the_build_occurrence_and_every_rebuild`, `a_compiled_switch_is_non_prompt`, `a_compiled_closure_captures_by_environment_index_and_not_by_rc` (a widget whose occurrence-time closure reads a sibling dynamic; drop the node; a `Weak` to the dynamic's node fails to upgrade), `compiled_and_hand_written_widgets_sit_in_one_tree` (a Rust `Button` bound inside a compiled widget and the reverse), `a_compiled_for_consolidates_a_birth_firing_promptly`, `the_depth_guard_aborts_a_runaway_recursion_with_its_span`.

### Differential, against the Haskell frontend

The Haskell frontend dumps every golden's program in the same text form, and the dumps are compared byte for byte. A difference is a sentence of the specification the two authors read differently, and the fix is a sentence in this book and then both frontends. The choices this chapter states because the differential test exposes them: where generalisation happens, how type variables are named, that cyclic groups are monomorphic, which edges are delayed, where placeholders are cut, how the dynamic-of-widgets lift is spelled in each of its forms, and that `Num.Add[Int]` is how a resolved operator prints.

## Deep in: a flat register listing as an alternative lowering

A native backend, if one is ever wanted, would not want a tree. Cranelift's input is SSA with block parameters (from memory); emitted Rust is made of expressions, but a code generator that is also doing register allocation wants its values named and its construction order flat. The lowering such a backend would take from the converted typed AST is worth writing down once, so that its relation to the closure compiler is clear and so that nobody mistakes it for a stage the compiler needs now.

The lowering names every intermediate value in a typed register, flattens each widget block into the build order the groups already give, keeps `match` structured, and keeps closure records as the converter produced them. The counter's knot in that form:

```
%count: $Int  = cyc.dyn "count @15:5"
%next:  !Int  = ev.tag_map_n %delta, [%count], @add_int
%held:  $Int  = dyn.hold 0, %next
cyc.close %count, %held
%label: $Text = dyn.map [%count], @fmt_count
```

Open, use, hold, close, then read. Every line is one runtime call, and the whole widget block is a list of such lines, one per step of `CompiledBlock.steps`. Leaves become functions in the same syntax:

```
fn @add_int(d: Int, c: Int) -> Int:
  %r: Int = int.add c, d
  ret %r
```

Read the two listings against the dump in the text form section: every line of the listing is a line of the dump with the nesting removed and a register name added. The listing adds no information, because the dump already has the lifts explicit, the captures explicit, the placeholders placed and the order fixed; it adds a syntax, and a syntax is a printer, a parser, a fixture set, a verifier and a golden set that every new node kind has to be added to after it has been added to the typed AST. For a backend that emits machine code, the syntax pays, because the backend's own input is flat and typed registers are what its allocator wants. For an evaluator that compiles to closures, the syntax is a second form saying the same thing flat, with its own verifier to keep it honest against the first, and the lint already does that job on the first.

The listing is readable as a work order, can be hand-written as a fixture before any parser exists, is checkable in one pass, and is a short step from an SSA backend. The closure compiler keeps the first three on the typed AST's text form (readable, hand-writable, linted) and gives up the fourth until a backend wants it. When one does, the lowering is a function from the converted typed AST to the listing, the listing's verifier is the lint restated over registers, and the differential test between the two backends is the transcript comparison the Test World already does: the counter compiled through the closure compiler and the counter compiled through the listing must produce one transcript. A bytecode, likewise, is a packed encoding of this listing with a version and a loader, wanted the day compiled code has to outlive the process that compiled it, and not before.

The one thing the listing would change in the compiler proper is nothing: it consumes the converted typed AST, and the converted typed AST is what the closure compiler consumes. That is why the line between elaboration and everything below it sits where it does.

## What is deferred

Each of these is absent on purpose, with what its absence buys now and what adding it would touch.

*Register code and native backends.* A consumer of the converted typed AST; the typed AST does not change.

*`read` and `update` blocks, `&` references, lenses.* The value layer has one operation on a record, construction, so the question of mutation inside a plain value never arises in the compiler: no aliasing analysis, no borrow rules, no in-place update in the leaves. When they come, they come as sugar over construction with an out-of-range write aborting the occurrence, and the leaves gain a few operations.

*User-defined traits.* The constraint set has three symbols and the solver is a table; a fourth symbol is a row. A general trait system would add instance declarations, instance search and coherence, and dictionaries per instance in generic code. The three traits are enough for sorting, map keys, element identity and arithmetic.

*Modules beyond one file plus the prelude.* One tag table, one program table, ids dense in declaration order. Separate compilation needs stable ids across files and a module scope in the resolver; the compiled form does not change.

*Incremental compilation.* The frontend compiles a program in one pass and the goldens are whole programs. A program large enough to need an incremental frontend is a program with modules, and that line comes first.

*Optimisation.* No inlining, no constant folding beyond what the Rust compiler does inside a leaf, no fusion of adjacent maps into one node, no dead-binding elimination. Per-occurrence cost sits at the runtime boundary where values cross into `Value`, and no evaluator avoids that; a pass that fused `map (map !e f) g` into one node would change the graph a reader sees in the dump, and until a profile says the boundary is cheap and the extra node is not, the dump should be the graph.

*A packed encoding of compiled code.* When something reads compiled code it did not compile: precompiled libraries, cached builds, a browser bundle without the frontend, a World receiving programs. A serialisation of the typed AST with a version and a load-time check, decoded into the tree the closure compiler takes; one more printer and parser from the same definition.

*Debug stepping through leaves.* A tree of closures shows nothing in a debugger, and that is a real loss against an interpreter with an instruction pointer. What the closure compiler offers instead is a span on every node and every closure, and the errors a Prism programmer meets (a cycle, a height violation, an unknown path) fire inside the runtime's propagation with no evaluator frame on the stack in either design. A tracing mode that logs each leaf's entry with its span is a flag on the `Frame`, and is where to start if stepping is wanted.

None of these changes the typed AST, which is the point of putting the line there: the frontend can be finished, frozen and matched against a second reading while every one of them is still absent, and each one, when it comes, is a consumer of the tree or a row in a table.
