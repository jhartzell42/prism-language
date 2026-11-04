# examples/chess

Chess in Prism: `rules.prism` (the game as plain functions), `window.prism` (a window
library: layout, hit testing and focus as host documents in Prism, over the four `Gui`
tags a drawing World accepts), `board.prism` (the root: sixty-four squares in a grid, the
selection and the game as holds, a status label), and `pieces/` (the piece images).

- **Run it:** `./prism run examples/chess` (or `make chess`) opens a window under the
  canvas World.
- **Test it:** `./prism test examples/chess` runs `rules.prismtest` and `board.prismtest`
  under the Test World against their `.expected` transcripts.
