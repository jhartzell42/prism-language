# examples/chess

Chess in Prism, a program tree (claude/modules-style-pass.md): `app.prism` (the root:
sixty-four squares in a grid, the selection and the game as holds, a status label),
`rules/` (`Rules`, the game as plain functions), `window/` (`Window`, a window library:
layout, hit testing and focus as host documents in Prism, over the four `Gui` tags a
drawing World accepts; its sections `ui`, `hosting`, `widgets`, `layout`, `hits`), and
`pieces/` (the piece images, which no `mod` line declares and the tree does not read).

- **Run it:** `./prism run examples/chess` (or `make chess`) opens a window under the
  canvas World.
- **Test it:** `./prism test examples/chess` runs the test modules `tests.prism` (the
  board's) and `rules/tests.prism` (the rules', with the positions and moves its checks
  are written with) under the Test World against the `tests.expected` beside each;
  `./prism test examples/chess/rules` runs the rules' alone.
