# examples/chess

*Written by Claude on the claude-document-chess branch; subject to change.*

Chess the document way: one game of chess is a document in memory, and every board,
terminal and script is a participant that holds a seat of it. A program tree
(claude/modules-style-pass.md); claude/document-chess.md has what runs today and what each
form waits on.

- **`app.prism`**, the root: `@main`, the hot seat in a window, two boards side by side.
- **`documents/`**, `Documents`: the hosting library. `Response`, `Stepped`, the trait
  `^Seat` a role's seat type implements, `Never`, the library tag `Seat.Leave`, the order
  of one tick's requests (`OneATick`), `@InMemory`, and what a role's host is written with.
- **`rules/`**, `Rules`: this program's rule book as plain functions: positions, moves
  (`Moving`, `Promoting`), why a move is not played (`MoveFault`), what the draws ask
  (`Lasting`, `Insufficient`). `rules/tests.prism` is its checks.
- **`game/`**, `Game`: chess as a document: `ChessGame`, the roles `Player` and
  `Spectator`, `PlayerRequest`, `ChessReason`, `Result`, `StepBy`, the languages `Play.*`
  and `Watch.*`, the hosts `@HostPlayers` and `@HostSpectators`.
- **`notation.prism`**, `words.prism`, `squares.prism`, `lines.prism`: moves as text and
  text as moves; results, statuses and reasons as words; the squares by name; lines of
  play as values, for checks.
- **Participants:**
  - `board.prism`, `Board`: `@BoardPlayer`, the graphical board, on `window/`.
  - `console.prism`, `Console`, and `command_line.prism`, `CommandLine`: `@LinePlayer` and
    `@LineSpectator`, a line in and a line out.
  - `text_players.prism`, `TextPlayers`: `@TextPlayer`, `@ScriptedPlayer`,
    `@SpectatorView`, over the prelude's `Ui` family.
- **`window/`**, `Window`: the window library, copied from the basic chess's
  (`examples/basic-chess/basic_window`), its element family named `Pane`.
- **`tables.prism`**, `Tables`: the programs that seat participants: `@boards`,
  `@watched`, `@againstTerminal`, `@terminals`, `@hotSeat`, the scripted ones.
- **Test modules:** `checks.prism` (`Checks`, with `checks.expected`), `tests.prism`
  (`Tests`), `terminal_tests.prism` (`TerminalTests`), `board_tests.prism`
  (`BoardTests`), and `rules/tests.prism` (`Rules.Tests`, with `tests.expected`).
- **The piece images** are the basic chess's, read where they are
  (`examples/basic-chess/pieces/`); none is copied here.

- **Run it:** `./prism run examples/chess` (or `make chess`, or `bin/chess`).
- **Test it:** `./prism test examples/chess`; the rule book alone,
  `./prism test examples/chess/rules`.
