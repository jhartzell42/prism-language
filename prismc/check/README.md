# check/

The module `Check` (docs/modules-style-pass.md): `Check`, the checker (prismc/README.md, Checker): `CheckProgram` runs every pass over a program's modules, in order; its submodules are the passes and what they share.

- `mod.prism`: the main file: the header; the program.
- Submodules: `blocks/` (`Check.Blocks`), `env/` (`Check.Env`), `errors/` (`Check.Errors`), `expr/` (`Check.Expr`), `passes/` (`Check.Passes`), `shrink/` (`Check.Shrink`), `typed/` (`Check.Typed`), `types/` (`Check.Types`).

Written by Claude on the claude-self-hosting branch; subject to change.
