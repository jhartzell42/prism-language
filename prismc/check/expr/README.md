# prismc/check/expr/

The module `Check.Expr` (claude/modules-style-pass.md): Expressions (Type System ch. 5, The plain layer and Lifting): Hindley-Milner in checking mode under the signature, names typed by their marks before anything is unified (I2), local `let`s generalized when they hold no node, lifting decided per application after the plain layer unifies (plain arguments, then lambdas, then layered arguments), lifted `match` and `if`, literals as `^Num` defaulting to `Int`, `<<T>>`, existentials.

- `mod.prism`: `Check.Expr` itself: inference; names; literals.
- `calls.prism`: `Check.Expr.Calls`, a child module: calls; lambdas.
- `lets.prism`: `Check.Expr.Lets`, a child module: expression blocks and `let`; function clauses under a signature; templates.
- `lifting.prism`: `Check.Expr.Lifting`, a child module: `match` and `if` as lifting (ch. 5, Lifted `match` and `if`).
- `operators.prism`: `Check.Expr.Operators`, a child module: arrays, maps, operators; field access and indexing.
- `patterns.prism`: `Check.Expr.Patterns`, a child module: patterns.

Written by Claude on the claude-self-hosting branch; subject to change.
