# prismc/syntax/parser/

The module `Syntax.Parser` (claude/modules-style-pass.md): The parser: tokens to the tree of ast.prism (Semantics ch. 3 to 6 and Appendix A; Templates ch. 2; Conformance ch. 1 and Appendix A). `prism_syntax/src/parser/` is the oracle, function for function: mod.rs (the cursor, the pairing rule, recovery), decls.rs, expr.rs (the Pratt loop over the precedence table), types.rs, patterns.rs and tests.rs.

- `mod.prism`: `Syntax.Parser` itself: The cursor (mod.rs); Names (types.rs, expr.rs).
- `blocks.prism`: `Syntax.Parser.Blocks`, a child module: Blocks and statements.
- `decls.prism`: `Syntax.Parser.Decls`, a child module: the top-level forms.
- `expr.prism`: `Syntax.Parser.Expr`, a child module: expressions.
- `test_language.prism`: `Syntax.Parser.TestLanguage`, a child module: the test language.
- `tests.prism`: `Syntax.Parser.Tests`, its test module: The parser's test module: its checks and the helpers they need, which only `prism test` and `prism check` read (claude/modules-style-pass.md).
- `types.prism`: `Syntax.Parser.Types`, a child module: types, rows and bounds, patterns.

Written by Claude on the claude-self-hosting branch; subject to change.
