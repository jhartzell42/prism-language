# prismc/check/

The module `Check` (claude/modules-style-pass.md): `Check`, the checker (prismc/README.md, Checker): `CheckProgram` runs every pass over a program's modules, in order; its submodules are the passes and what they share.

- `mod.prism`: `Check` itself: the program.
- `errors.prism`: `Check.Errors`, a child module: The checker's diagnostics: Type System ch. 5's errors, each with its name, its position and its fix, and how one prints (ch. 5, "The errors": `file:line:`, the line below, the source with carets, and the fix).
- `blocks/`: `Check.Blocks`, a child module with children of its own (its README).
- `env/`: `Check.Env`, a child module with children of its own (its README).
- `expr/`: `Check.Expr`, a child module with children of its own (its README).
- `passes/`: `Check.Passes`, a child module with children of its own (its README).
- `shrink/`: `Check.Shrink`, a child module with children of its own (its README).
- `typed/`: `Check.Typed`, a child module with children of its own (its README).
- `types/`: `Check.Types`, a child module with children of its own (its README).

Written by Claude on the claude-self-hosting branch; subject to change.
