# prismc/check/passes/

The module `Check.Passes` (claude/modules-style-pass.md): The separate checkers (Type System ch. 5, "The separate checkers, in order"), run over the elaborated program after inference, a definition that failed inference skipped: the prompt-cycle check per block with its summaries, the shrink check by the size-change principle with the `unbounded` color and D6's cycle rule, borrows and escapes in `&` blocks, the element bind table's remaining rows, inclusion with the forwarding-arm remainder, the root row, the full-stack name check (most of it made where each name is bound), and exhaustiveness over rows and choices (Maranget's usefulness, from memory). `CheckProgram` runs the whole checker over a program's files.

- `mod.prism`: `Check.Passes` itself: Exhaustiveness and reachability; Inclusion and the forwarding-arm remainder (ch. 5, pass 5).
- `borrows.prism`: `Check.Passes.Borrows`, a child module: `&` blocks: escapes, freezes, borrows (Semantics ch. 5).
- `cycles.prism`: `Check.Passes.Cycles`, a child module: The prompt-cycle check (Semantics ch. 7, There are no cycles).

Written by Claude on the claude-self-hosting branch; subject to change.
