# syntax/

The module `Syntax` (docs/modules-style-pass.md): The tree of Prism, as `prismc` builds it: `prism_syntax/src/ast.rs` one to one, node for node and field for field, so the two parsers print the same S-expressions (sexp.prism).

- `mod.prism`: the main file: the header; positions and diagnostics, names, the file, types, patterns, literals, expressions, blocks and statements, templates.
- `decls.prism`: a section; declarations, tests.
- Submodules: `lexer/` (`Syntax.Lexer`), `parser/` (`Syntax.Parser`), `sexp/` (`Syntax.Sexp`).

Written by Claude on the claude-self-hosting branch; subject to change.
