# prismc/lower/

The module `Lower` (claude/modules-style-pass.md): Lowering: the checked program to the core form a printer spells (Compiler ch. 3, "The translation, construct by construct"; claude/backend.md, the contract).

- `mod.prism`: `Lower` itself: The context: the environment, the typed tables by place, the module being lowered.
- `expr.prism`: `Lower.Expr`, a child module: Literals.
- `functions.prism`: `Lower.Functions`, a child module: functions, traits and impls.
- `jobs.prism`: `Lower.Jobs`, a child module: unbounded functions as step machines, `while` in an `update` block of a job.
- `model.prism`: `Lower.Model`, a child module: The core form.
- `patterns.prism`: `Lower.Patterns`, a child module: Patterns: a Rust pattern, the conditions after it, and the names it binds.
- `program.prism`: `Lower.Program`, a child module: The program.
- `templates.prism`: `Lower.Templates`, a child module: Templates, `given:` and `update` blocks.
- `types.prism`: `Lower.Types`, a child module: Types (claude/backend.md, "Types").
- `node/`: `Lower.Node`, a child module with children of its own (its README).

Written by Claude on the claude-self-hosting branch; subject to change.
