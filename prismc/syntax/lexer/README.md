# prismc/syntax/lexer/

The module `Syntax.Lexer` (claude/modules-style-pass.md): The lexer: characters to tokens, with layout (Semantics ch. 2; Templates ch. 2, Lexing; Conformance Appendix A.1 and A.2). `prism_syntax/src/lexer.rs` is the oracle, rule for rule; the two must give the same tokens, and `prismc --tokens` prints them in the layout corpus's notation so the two can be diffed.

- `mod.prism`: `Syntax.Lexer` itself: Tokens (prism_syntax/src/token.rs); The lexer's state; Token kinds; Pushing tokens.
- `chars.prism`: `Syntax.Lexer.Chars`, a child module: Small helpers over texts and arrays; Characters (ASCII only: "Names are ASCII letters, digits and underscores"); Reading characters.
- `corpus.prism`: `Syntax.Lexer.Corpus`, a child module: Rust's `{:?}` and `escape_debug` for texts and characters, which both printers use.
- `layout.prism`: `Syntax.Lexer.Layout`, a child module: Streams; Layout (rules 1 to 4 and 10; Conformance A.1 inside a `test` block).
- `literals.prism`: `Syntax.Lexer.Literals`, a child module: Literals.
- `templates.prism`: `Syntax.Lexer.Templates`, a child module: Templates (Templates ch. 2; Semantics ch. 2, "The template frame").
- `tests.prism`: `Syntax.Lexer.Tests`, its test module: The lexer's test module: its checks and the helpers they need, which only `prism test` and `prism check` read (claude/modules-style-pass.md).
- `words.prism`: `Syntax.Lexer.Words`, a child module: Tokens; Sigils and words.

Written by Claude on the claude-self-hosting branch; subject to change.
