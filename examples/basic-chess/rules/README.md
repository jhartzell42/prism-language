# examples/basic-chess/rules/

The module `Rules` (claude/modules-style-pass.md): the rule book as plain functions.

- `mod.prism`: `Rules` itself: the pieces, the position, a move.
- `tests.prism`: `Rules.Tests`, its test module: the rules' checks and the positions and
  moves they are written with ("For the checks"), which only `prism test` and
  `prism check` read; `tests.expected` holds its transcript.

Written by Claude on the claude-self-hosting branch; subject to change.
