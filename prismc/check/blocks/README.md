# prismc/check/blocks/

The module `Check.Blocks` (claude/modules-style-pass.md): Blocks (Semantics ch. 4 and 5; Type System ch. 5): widget blocks as binding groups, holds, binds (a build, an emit, or a `rebuild` or `for` handing back its result), the keyed `for` and `rebuild` with what each hands back, hosting through the prelude's signatures, `given:` bodies with guards (a failed guard under `~` is `Prelude.Unmet`, D11), and `&` blocks with their references, locals, loops, borrow and escape rules.

- `mod.prism`: `Check.Blocks` itself: widget blocks.
- `borrows.prism`: `Check.Blocks.Borrows`, a child module: `&` blocks (Semantics ch. 5).
- `structure.prism`: `Check.Blocks.Structure`, a child module: `rebuild` and `for`; `given:`.

Written by Claude on the claude-self-hosting branch; subject to change.
