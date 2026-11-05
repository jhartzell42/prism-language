# prismc/syntax/

The module `Syntax` (claude/modules-style-pass.md): The tree of Prism, as `prismc` builds it: `prism_syntax/src/ast.rs` one to one, node for node and field for field, so the two parsers print the same S-expressions (sexp.prism).

- `mod.prism`: `Syntax` itself: Positions and diagnostics; Names; The file; Types; Patterns; Literals; Expressions; Blocks and statements; Templates (Templates ch. 2).
- `decls.prism`: `Syntax.Decls`, a child module: Declarations; Tests (Conformance ch. 1 and Appendix A).
- `sexp.prism`: `Syntax.Sexp`, a child module: Declarations; Tests; Blocks and statements; Literals and expressions; Templates; Patterns and types.
- `lexer/`: `Syntax.Lexer`, a child module with children of its own (its README).
- `parser/`: `Syntax.Parser`, a child module with children of its own (its README).

Written by Claude on the claude-self-hosting branch; subject to change.
