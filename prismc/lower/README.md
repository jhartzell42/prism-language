# lower/

The module `Lower` (claude/modules-style-pass.md): Lowering: the checked program to the core form a printer spells (Compiler ch.

- `mod.prism`: the main file: the header; the context: the environment, the typed tables by place, the module being lowered, names.
- `expr.prism`: a section; literals, plain expressions.
- `functions.prism`: a section; functions, traits and impls.
- `jobs.prism`: a section; unbounded functions as step machines, `while` in an `update` block of a job.
- `patterns.prism`: a section; patterns: a Rust pattern, the conditions after it, and the names it binds.
- `program.prism`: a section; the program.
- `templates.prism`: a section; templates, `given:` and `update` blocks.
- `types.prism`: a section; types, values: what a Prism name stands for, and how a value is borrowed or owned.
- Submodules: `model/` (`Lower.Model`), `node/` (`Lower.Node`).

Written by Claude on the claude-self-hosting branch; subject to change.
