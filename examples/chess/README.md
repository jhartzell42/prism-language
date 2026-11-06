# examples/chess

Chess in Prism, a program tree (claude/modules-style-pass.md): `app.prism` (the root:
sixty-four squares in a grid, the selection and the game as holds, a status label),
`rules/` (`Rules`, the game as plain functions), `window/` (`Window`, a window library:
layout, hit testing and focus as host documents in Prism, over the four `Gui` tags a
drawing World accepts; its child modules `Window.Ui`, `Window.Hosting`, `Window.Widgets`,
`Window.Layout`, `Window.Hits`, each a file of `window/`), and
`pieces/` (the piece images, which no `mod` line declares and the tree does not read).

- **Run it:** `./prism run examples/chess` (or `make chess`) opens a window under the
  canvas World.
- **Test it:** `./prism test examples/chess` runs the test modules `tests.prism` (`Tests`,
  the board's, which reaches the root by `use App.*`) and `rules/tests.prism`
  (`Rules.Tests`, the rules', with the positions and moves its checks
  are written with) under the Test World against the `tests.expected` beside each;
  `./prism test examples/chess/rules` runs the rules' alone.

## The interface

*Written by Claude on the claude-self-hosting branch; subject to change.* What the window
shows, in `app.prism`'s widgets (`@Board`, `@Panel`, `@settingsForm`, `@$chess`), all on
the window library's widgets:

- **The title,** a label over the board.
- **The board,** on a dark border whose width is a setting; a first click on one's own
  piece selects it (yellow), a second click moves it if the move is legal and clears the
  selection either way. A promotion is always to a queen.
- **The panel** beside it:
  - the status (whose turn, in check, checkmate, stalemate), naming the player;
  - the move counter, "Move 2 of 5": the position shown and the moves played;
  - **Back** and **Forward**: the position before or after the one shown; each stops at
    its end;
  - **Reset**: the game starts over and every move is forgotten; the settings stay;
  - **Settings**: opens the settings dialog, and closes it if it is open;
  - **the move list**: a line per full move, `2. Ng1-f3 Nb8-c6`, the move that led to the
    position shown in brackets. It shows at most sixteen lines (`listRows`), scrolled so
    that move stays in view.
- **The settings dialog,** in the window's bottom right corner, over the move list and
  the board's right edge, below the status and the buttons, which stay usable: a box per
  setting, each applied as it is typed:
  - the copy: the title, every button's label (Settings, Close, Reset, Back, Forward), the
    move list's heading, the move counter (`{move}` and `{moves}`), and the four status
    lines (`{name}` for the player);
  - the players' names;
  - the dark and light squares' colours, as `#rrggbb` in either case;
  - the border: a slider from 0 to 32 points and a box, which move together;
  - a line saying the settings apply as typed and start at their defaults each run;
  - **Close**.

### Choices (Claude's)

- **MF A move made on an earlier position** discards the moves after it, and the game
  goes on from there, as most chess programs do; Forward then has nowhere to go.
- **RS Reset** forgets the whole game, history included; the settings, the dialog's
  state and the dialog's open or closed state stay.
- **BB Back, Forward and Reset clear the selection.** So does any button.
- **IV A colour that is not `#rrggbb`,** and a border that is not a whole number from 0 to
  32 (an empty box included), leaves the setting at its last good value; the field's
  caption says what is wrong ("Dark squares: not #rrggbb"). The box keeps what was typed.
- **DM The dialog is modeless:** the board can still be played where the dialog does not
  cover it; the dialog's background takes the presses that land on it.
- **ST The settings start at their defaults every run** and live in the dialog's holds
  (each box's text, the last good colours, the border); nothing is read from or written
  to a file. The dialog keeps them when closed: it is hidden, not torn down
  (window/README.md, doubt HS).
- **TT The window's title follows the title setting** a tick behind it: `@$main` holds it,
  fed by the interface's result, since the window is built around the interface.
- **CO The default copy is the rule book's** (`Rules.Status`), with "White" and "Black"
  as the names, so the status reads as it did; the squares are brown and cream
  (`#b58863`, `#f0d9b5`), no longer the window library's `Grey` and `Paper`.

### Doubts

- **PC** (Claude's) The tests click the dialog's boxes by their places in its grid
  (`click element 29`; Close is 4), since a `Draw.Clickable` names the innermost path of its chain,
  and a caption and box per row in a column of rows would give every box the same one.
  Recommendation: let `Draw.Clickable` carry the whole chain and `click element` take it,
  so a form can be built of rows; that changes every transcript's scene.
- **DT** (Claude's) Text boxes size to their text (at least 200 wide), so the dialog is as
  wide as its longest status line. Recommendation, when it matters: a fixed-width box that
  scrolls its text, which needs the caret's scroll offset in the library.
- **NT** (Claude's) Moves are written in coordinates (`Ng1-f3`, `e4xd5`), not standard
  algebraic notation, which needs disambiguation and check marks from the rule book.
