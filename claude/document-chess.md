# Chess the document way

*Written by Claude on the claude-document-chess branch; subject to change.*

## What the example is

- **`examples/chess` is chess as a document.** One game of chess is a state in memory with
  pure rules; a board, a terminal, a text interface and a script are participants, each
  holding a seat. A participant's `emit` line is everything it can reach: its interface's
  family and its seat's language. None has a name for the game.
- **It is written the way it should work.** Every form is the design's, with no line
  bent around a compiler bug. Today's compiler refuses the tree; the table below is
  what it does with each test, and "Each form and what it waits on" is the work list.
- **A version that runs today is beside the design:** `jhartzell42/prism-notes`,
  `claude/04-domains-and-documents/evidence/chess`, the same document, rules, participants
  and tests under the forms today's compiler takes. Every expectation in this example's
  tests that says "from a run" is that version's output.
- **`examples/basic-chess` is the chess that was `examples/chess`:** one board with a
  settings dialog, its own rule book, its window library the module `BasicWindow`. The
  two examples share no module.
- **Bug numbers** are those of `claude/04-domains-and-documents/evidence/documents/BUGS.md`
  in `jhartzell42/prism-notes`, where each has its program and the compiler's output.

## The module map

- **`app.prism`:** the root. `@main` is the hot seat in a window: `@Window` around
  `Tables.@boards`.
- **`documents/mod.prism`, `Documents`:** the hosting library, for every document.
  - `Response a`, `Stepped s d`; the trait `^Seat r`, a role as a seat type, with `State`,
    `Request`, `Reason`, `Query`, `Told`, `Step`, `Read`.
  - `Never`, the type with no value; `Seat.Leave`, the library's tag for leaving a seat.
  - `Place`, `Responded`, `Asked`, `Asking`, `AskingAs`; `Notice`, a request brought or a
    seat kicked.
  - `OneATick`, `ByPlace`, `StepTick`: the order of one tick's requests.
  - `@InMemoryOrdered`, `@InMemory`: the implementation in memory.
  - `@Told`, `@Sitting`, `?ResponseTo`, `Among`, `!LeavesOf`, `Heard`, `Happening`,
    `RequestsOf`, `?Notices`, `Reseated`: what a participant and a role's host are written
    with.
- **`rules/mod.prism`, `Rules`:** the rule book as plain functions. `Move` is
  `Moving from, to` or `Promoting from, to, man`; `MoveFault` says why a move is not
  played; `Lasting` and `Insufficient` serve the draws. `rules/tests.prism` checks it
  alone.
- **`game/mod.prism`, `Game`:** the document. `ChessGame` (the position, the moves, the
  count of moves since a capture or a pawn's move, every earlier position, a standing draw
  offer, the result), `Player`, `Spectator`, `PlayerRequest`, `ChessReason`, `Result`,
  `StepBy`, the tags `Play.Request`, `Play.Query`, `Watch.Query`, and `@HostPlayers`,
  `@HostSpectators`.
- **`notation.prism`, `words.prism`, `squares.prism`, `lines.prism`:** `?ParseMove` and
  `Spelled`; `ResultText`, `StatusText`, `ReasonText`; `e2`, `e4`; `~Line` and the typed
  fault `Line.Declined`.
- **`board.prism`, `Board`:** `@BoardPlayer`, with `@Squares`, `@Panel`, `@PromotionForm`.
- **`console.prism`, `Console`; `command_line.prism`, `CommandLine`:** the terminal's two
  tags, and `@LinePlayer`, `@LineSpectator`.
- **`text_players.prism`, `TextPlayers`:** `@TextPlayer`, `@ScriptedPlayer`,
  `@SpectatorView`.
- **`window/`, `Window`:** the window library, copied from
  `examples/basic-chess/basic_window`, with its family named `Pane` and its child module
  `Window.Panes`. A program holds one family of a name, and the text participants use the
  prelude's `Ui`.
- **`tables.prism`, `Tables`:** the seatings.
- **`tests/documents/tally/`:** the hosting library's second document, a count that
  tellers add to, keepers clear and viewers watch, with its fifteen tests. It is a program
  of its own under `tests/`, since it declares tags and impls, which a test module may
  not, and it exists to be tested; it reads the library as a second source, the way the
  GUI benchmark reads the basic chess's window library.

## How to run each participant

- **The boards:** `make chess`, `bin/chess`, or `./prism run examples/chess`. One window,
  two boards, each from its own side. A press on one's own piece selects it, and the
  squares it may move to turn green; a second press sends the move. A pawn moved to its
  last rank opens a dialog: Queen, Rook, Bishop, Knight, Cancel. Back and Forward show
  earlier positions; Resign, Offer a draw, Accept the draw send their requests. The line
  under the status words the reason of the last declined request.
- **The terminals:** `Tables.@terminals`, two players and a spectator. One line in, one
  line out, and no board drawn.
  - In: a move in coordinates (`e2e4`; a promotion names its piece, `e7e8q`), `resign`,
    `draw`, `accept`, `moves`, `legal`, `leave`.
  - Out: `ok`; `declined: …`, the typed reason in words; another seat's move alone on its
    line, when it is played; `draw offered`; `over: …`, the result; the moves played or
    the legal moves on one line, a space between two.
  - A terminal is two elements of the program's own family, `Console.Read(name)` and
    `Console.Print(name, !lines)`. No World answers them: the CLI World reads no standard
    input. They run under the Test World (`terminal_tests.prism`).
- **A board against a terminal, and boards watched from a terminal:**
  `Tables.@againstTerminal`, `Tables.@watched`.
- **The text interfaces and the scripts:** `Tables.@hotSeat`, `@foolsMate`, `@castling`,
  `@enPassant`, `@promotion`, `@repetition`, `@bothOpen`, under the Test World
  (`tests.prism`).
- **The tally:** `./prism test tests/documents/tally examples/chess/documents`.

## What today's compiler does with each test

- **The compiler:** `741b87ec`, bootstrapped to its fixed point. The branch's base,
  `cc4b6fb`, did not bootstrap on the machine this was written on: from the `741b87ec`
  binary rustc refuses the first generation (`error[E0061]: this function takes 2
  arguments but 1 argument was supplied`, at `::prism_rtl::HostRow::entries(self)`), and
  from the repository's seed rustc refuses the seed's crate with the same message. Every
  result below is `741b87ec`'s.
- **What passes today:**
  - `./prism test examples/chess/rules`: `ok`, four `check` blocks, 21 lines, against the
    kept `rules/tests.expected`.
  - `./prism check examples/chess/window`: `no errors`.
  - `./prism test examples/basic-chess`: `ok`, as before its move.
- **The whole tree:** `./prism test examples/chess` stops at the parser, and so every test
  module but the rule book's is not reached. First message, verbatim:
  `examples/chess/documents/mod.prism:35:20: expected an indented block after ` `` `:` `` `
  (bug 29). The tally stops the same way:
  `tests/documents/tally/tally.prism:112:49: expected an indented block after ` `` `:` `` `.
- **With today's form put in place of one refused form at a time** (in a copy; the tree
  itself keeps the design's):
  - in place of `Never` (29): 29 checker errors, among them
    `error[prompt-cycle]: a prompt cycle: `` `!arrived`, `!due`, `!outcome` `` ` (08),
    ``error[mismatch]: the fault kind `Line.Declined` declares a field, `request`; a fault kind is its tag alone`` (30),
    ``error[mismatch]: expected `[!t2241]`, found `[Heard !(Asked ChessGame Player (Responded Player ChessReason))]` `` (25);
  - and of the typed fault (30), the direct responses (08) and the constructors over
    events (25): 17 errors, all
    ``error[blueprint-operand]: `a map literal` over a blueprint; bind it first`` (05), one
    at each seat of `tables.prism`;
  - past 05 the checker takes the forms, and the back end's refusals are those recorded
    for the same compiler in the evidence: 01, 02, 06, 10, 12, 14, 15, 16, 18, 20, 21, 24.

| Test module | Tests | Today | What it is held to | Waits on |
|---|---|---|---|---|
| `rules/tests.prism` | `the_opening`, `a_promotion_is_its_own_move`, `insufficient_material`, `lasting_moves` | passes | `tests.expected`, from a run | nothing |
| `checks.prism` | `moves_and_turns`, `check_and_mate`, `stalemate`, `castling`, `en_passant`, `promotion`, `resigning_and_draws`, `draws`, `notation` | not reached: parse errors | `checks.expected`, from a run | 29 |
| `checks.prism` | `declined_line` | not reached | expected by the design, not from a run: the fault holds its request, side and reason | 29, 30 |
| `terminal_tests.prism` | `fools_mate_at_terminals`, `declined_at_terminals`, `draw_agreed_at_terminals`, `repetition_at_terminals`, `resigning_and_leaving_at_terminals` | not reached | each step's `requested` lines, from a run | 29, 08, 25, 05, 01, 02, 06, 10, 12, 14, 18, 20, 21, 24 |
| `tests.prism` | `scholars_mate`, `illegal_move_declined`, `out_of_turn_declined`, `resignation`, `draw_by_agreement`, `spectator_joins_and_leaves`, `resign_and_leave` | not reached | its `expect` lines, from a run | the same, and 15 |
| `tests.prism` | `castling`, `en_passant`, `promotion`, `draw_by_repetition` | not reached | its `expect` lines, from a run | the same, and 15, 16 |
| `tests.prism` | `spectator_sees_each_move`, `both_players_ask_in_one_tick` | not reached | expected by the design, not from a run: a scripted player asks a tick after the build, one tick sooner than under today's forms | the same, and 15, 16; the tick is 01 and 06 |
| `board_tests.prism` | `boards_show_the_opening`, `fools_mate_on_two_boards`, `declined_on_a_board`, `back_and_forward`, `promotion_dialog`, `draw_by_buttons`, `board_against_a_terminal` | not reached | its `expect` lines, from a run | as the terminals' |
| `board_tests.prism` | `boards_with_a_terminal_watching` | not reached | its steps, from a run with the terminal's seat beside the window | as the terminals', and 31 |
| `tests/documents/tally/tests.prism` | `accepted`, `declined`, `wrong_turn`, `viewer_sees`, `join_and_leave`, `keeper_clears`, `two_requests_in_one_tick`, `three_requests_in_one_tick`, `two_roles_in_one_tick`, `own_order`, `request_and_leave`, `unremoved_seat_request_lands`, `removed_seat_request_dropped`, `removed_seat_waiting_request`, `sitting_changes` | not reached: parse errors | its `expect` lines, from a run | as the terminals', and 17 |

- **Counts:** 4 `check` blocks pass today; 10 `check` blocks and 41 tests wait.
- **Kept transcripts:** `rules/tests.expected` and `checks.expected`. A test with ticks
  has none: the transcripts of the version that runs are 111,600 bytes for the text and
  terminal tests and 4.2 MB for the boards, over the 10 KB a kept transcript may be
  (`tests/compiled/run.sh`); and a place's path under a seat (the seat's key, the
  generation of its `rebuild`) is the compiler's to give. Each such test is held to its
  own `expect` and `requested` lines, which name no path.
- **The runners do not list the example:** `tests/compiled/run.sh`, `tests/checker/run.sh`
  and `tests/driver/run.sh` run the basic chess under its new path, so `make test` stands
  as it stood. The example joins them when it compiles.

## Using it as a target

- **Green is:** `./prism test examples/chess` and
  `./prism test tests/documents/tally examples/chess/documents` both `ok`, with no line of
  either tree changed.
- **A smaller step first:** each bug's own program is in the evidence's `bugs/` directory;
  the forms below say where this example uses each.
- **When it compiles:** save each ticked test module's transcript beside it if it is
  under 10 KB, and add the example and the tally to `tests/compiled/run.sh`.

## Each form and what it waits on

- **29 A choice with no case cannot be written.** `type Never = choice:` in `Documents`; a
  spectator's and a viewer's `Request` and `Reason` are `Never`, and their `Step` is
  `match request:` with no arm.
- **30 A fault tag with fields is refused.** `Line.Declined(request, by, reason)` in
  `lines.prism`: a fault holds typed values.
- **08 A prompt cycle is reported through a function parameter.** `@InMemoryOrdered` hands
  `ResponsesOf !outcome` straight to `HostSeats`; the responses reach the requests only
  through a host's answer, the next tick's input.
- **07 A prompt cycle is reported through a widget's result.** The same cycle, met when the
  implementation gives the state and the responses back as its result.
- **25 A constructor applied to an event is not lifted.** `Seating !seat`, `Removing
  !remove`, `Hearing !heard`, `Requested (Asking seat, place, !requests)` in a role's host.
- **05 A map of blueprints cannot be written.** Every seat of `tables.prism`:
  `{white: (@BoardPlayer white), black: (@LinePlayer black)}`. A seat holds the blueprint
  of whoever sits there, so one role seats a board beside a terminal in one map.
- **01 and 06 A host's dynamic answer that reads a widget's result, a parameter, or a
  `for`'s binder is not lowered.** `Play.Query($query=$asks): @ Read $state, seat, $asks`
  in each role's host: a seat's answer is the state read, live, from the tick the seat is
  built. No hold that starts `nil`, no `(loading)`, no first answer two ticks late.
- **02 A `$` field bound in an arm and read in a dynamic answer names nothing.** The same
  line: `$asks` is the query's own question.
- **09 A plain field bound in an arm and read in a dynamic answer names nothing.** The
  same construct with a plain field; this example binds none.
- **10 An element with an event field in a `rebuild`'s generation under a host is refused
  by rustc.** `@Sitting`: `rebuild $@guest: _`, around every participant, each of which
  places a request.
- **16 The same in a building `match`'s arm.** `@ScriptedPlayer`: `match script:` with
  `Play.Request` in each arm.
- **12 A widget function's parameter written as a constructor pattern names nothing.**
  `fn @TextPlayer (Player color):`, and every participant.
- **13 A tuple pattern as a `for`'s key is refused by rustc.** Not written here: a seat is
  its `for`'s key alone, since what sits there is the map's value.
- **14 `!MergeWith` is not found by rustc; an entry's line that reads an event of the
  enclosing block is refused.** `!MergeWith Append, [[…]]` joins a terminal's lines of one
  tick, and the tally's two role hosts' notices; `!responses`, the role host's parameter,
  is read in the entry's arm.
- **15 A block's own widget function passed as an argument is refused.** `@hotSeat` and
  `@Scripted`: `fn @Seats $state, !responses = …` in the block, reading the block's own
  names, handed to `@InMemory`.
- **17 A function parameter marked `bounded fn`, read by a lambda the function gives back,
  is refused by rustc.** `AskingAs` in `Documents`, which the tally's hosts call.
- **18 An event field of `unit` bound in a plain function's arm is refused by rustc.**
  `Seat.Leave(!leaves=!leaves)` in `!LeavesOf`.
- **19 A lambda's parameter named as a unit type's value is taken for the value.** Not
  written here: the unit type that stood in for `unit` under 18 is gone.
- **20 A choice with a type parameter as a map's key is refused by rustc.** `Happening k v
  a` holds `Seating (k ==> v)` and `Hearing (k ==> (Path ==> [Heard a]))`.
- **21 `!MergeAll` fires twice in one tick.** A role's host: `let !tick = !MergeAll
  [[(Seating !seat), (Removing !remove), (Hearing !heard)]]`.
- **22 `!Union` in a plain function's body is not found by rustc.** Not written here.
- **23 A request is lost when a hold is read by `!Updated` in the same host.** Not written
  here: no host keeps news for its seats.
- **24 A `for`'s gathered event fires twice in one tick.** A participant that asks and
  leaves at once fires `Play.Request` and `Seat.Leave` in one tick, two places of one
  guest (`@TextPlayer`'s last button, the tally's `@Teller`).
- **26 and 27 A host's arm cannot answer with a widget function that places an element,
  nor with `Prelude.Unwrap` under `@Element`.** Not written here: a role's host is never
  handed another role's reason (`Responded k a`, `Among`), so it has no fault to give up
  on.
- **03 rustc refuses a choice whose one case has the type's name, and a plain function
  lifted over an event its arm binds.** `!RequestsIn`, `!LeavesOf` bind an event in an arm
  and give it back.
- **11 Places are built twice under two levels of host.** Met when one host for the whole
  document stands around the role hosts; this example's implementation takes the seats
  function and has no outer host.
- **28 Places under a `for` are listed by the key's constructor name, not by its `^Ord`.**
  A transcript lists `Player Black` before `Player White`; the order of requests follows
  `^Ord`.
- **31 An element with an event field passed up through a box's `@HostSome` under two
  hosts aborts the tick.** `@watched`: the spectator's terminal sits in the window's row
  with the boards.

## What the language lacks

- **Associated types indexed by a role that is a value.** The design's `^Document d` has
  `Request d r`; here a role is a seat type and the trait is `^Seat r`.
- **Tag families with an index.** The design's `Seat.Request d r` and `Seat.Query d r`
  are library tags; here each document declares a family per role (`Play.*`, `Watch.*`)
  and writes each role's host. `Seat.Leave` holds no type of a role, so it is the
  library's.
- **`SeatOf` a path.** The design has one host per role and reads the seat out of a
  place's path; the prelude takes no path apart, so each seat has its own host.
- **A map's entries.** The prelude lists keys and values; `Documents.Entries` looks each
  key up again.
- **A World that reads lines.** `Console.*` runs under the Test World alone.
- **A role with no request has no request tag** (`Watch.*`): that part is the design's
  own, and `Never` is what its trait names.

## Rulings

- **The owner's:**
  - Two rule books: the basic chess keeps its own, and this example's is its own.
  - The window library of the basic chess is `BasicWindow`; this example copies it and
    the board, with no settings dialog. The directory is `basic_window/`, since a module's
    directory is its name in snake case (claude/modules-style-pass.md).
  - `Never` is a choice with no case, and is used.
  - A kicked seat's requests are dropped, all of them: those of the kick's tick and those
    that wait from earlier ticks (`Notice`, `Kicked`, `StepTick`).
  - The delay of a request that waits is one tick of `@Defer`, not `@Later`.
  - A player that leaves without resigning leaves the game standing. This document has no
    clock; in one that has, the leaver's clock runs out. A program that wants leaving to
    mean resigning sends the resignation: `@TextPlayer`'s last button does.
  - A promotion is a special move: a pawn put on its last rank by `Moving` is declined
    (`IllegalMove PromotionNotNamed`), and `Promoting` names its piece. The board asks in a
    dialog; a terminal reads the piece in the line.
  - A fault holds typed values.
  - The tally is kept as a test.
- **First rulings, not the owner's** (the Bible's session's and this branch's):
  - **Which request of one tick goes first:** the one that has waited longest; among one
    tick's own, by seat order, then by place.
  - **`Seat.Leave` is a library tag.**
  - **The draws are automatic:** insufficient material, a third repetition and fifty
    moves are each the result at the move that brings them about, with nobody claiming.
    A document has no arbiter. Otherwise: `ClaimDraw` is a request, declined with a reason
    when nothing holds.
  - **`Move` has two cases,** `Moving` and `Promoting`; castling and a capture in passing
    are `Moving`, told by their squares, and the rule book names how a legal move moves
    as a `Manner`. Otherwise: `Castling` and `InPassing` are cases too, and a participant
    looks at the position to pick one.
  - **A repeated position is a `Position` value compared with `==`,** the en passant
    square included whether or not a pawn can take there.
  - **Seats come and go by events the role's host is given,** and the host holds the map:
    a removal has to be seen in the tick it happens, beside that tick's requests.
  - **An accepted line prints `ok`,** so every line typed is answered by one line.
  - **The hot seat is two boards,** each holding one seat, where the basic chess has one
    board for both sides.
  - **The piece images are read from `examples/basic-chess/pieces/`,** not copied.
