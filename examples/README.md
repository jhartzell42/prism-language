# Examples

*Written by Claude, 31 October 2026, on the `claude-self-hosting` branch. Work in progress
and subject to change: an implementer reading these uses their own discretion. The
design is in the books of `prism-notes/claude/prism/`, a curated set of design prompts, not
a specification; where a program here and a book disagree, the disagreement is listed
under Doubts below. Every `.expected` file is a transcript the compiled program produces
under `tests/compiled/run.sh`.*

## The programs

- **`chess/`**: chess in three modules, the program the self-hosting branch compiles and
  opens in a window.
  - `rules.prism`, module `Chess`: the rule book as plain functions, with no time in them
    (Pitch ch. 11). A game is a value; `?Move` gives the next game or `nil`.
  - `board.prism`, the root file: sixty-four `Ui.Image` squares in a `@Grid`, a hold for the
    selection, a hold for the game, a status label, all inside `@Window`. Its root is
    `@$main`.
  - `window.prism`, module `Window`: the window library. It declares the `Ui`, `Gui` and
    `Draw` families, the container host documents (`@Grid`, `@Row`, `@Column`, `@Layers`,
    `@Framed` and its modifiers, `@VBox`, `@HBox`), `@TextInput`, the layout and hit testing
    of prism-reflex's `layout.md`, and `@Window`. It is general on purpose: the card games
    will use its rows, layers and turned frames.
  - `pieces/`: the thirteen 64-by-64 PNGs the squares show, from prism-reflex.
  with a Star that is the row's own hold.

Each program `X.prism` has:

- **`X.prismtest`**: its `test` and `check` declarations, in the Conformance book's test
  language (Conformance ch. 1 and App. A). A `.prismtest` file holds declarations of the
  module in the `.prism` file of the same name, as tests are declarations of their module
  and have no `use` lines of their own (Semantics ch. 6; doubt TF).
- **`X.expected`**: the transcripts its tests print under the Test World, byte for byte
  after the `# not yet run` line, which the test runner ignores. The rules' checks and the
  board's scenes were confirmed with a small model of `rules.prism` and of the window
  library's layout for the board's one tree; the model is not an implementation, and the
  files are still not yet run.

## How they will run

- **Tests:** `./prism test examples/chess` compiles the three files with prismc, builds
  them, and runs `board.prismtest` and `rules.prismtest` under the compiled Test World,
  each transcript byte-equal to its `.expected`. `board.prism` is the one file with no
  `module` line, so its `@$main` is the root (Semantics ch. 6).
- **Chess in a window:** `./prism run examples/chess` (`make chess`) builds it with the
  canvas World, which the root's `Gui.*` row asks for, and opens it.
- **Today** prismc checks chess clean and refuses to compile it: the window library
  hosts (`@HostSome`), takes elements apart by pattern and builds `Draw.*` values, which
  the back end does not lower yet (prismc/README.md, "Not lowered yet").
- The counter, the half counter and the todo list emit bare `Ui` elements, and the canvas
  World answers only `Gui` (a thin World, Pitch ch. 6). They run under their tests, or in a
  window when a root wraps them: `@Window $"Counter", (Extent 320.0, 200.0), @counter`.
- Every file parses with `cargo run -p prism_syntax --example dump -- FILE` (and `--pretty`),
  except six sigiled field accesses the parser does not read yet (doubt FA). The window
  library and the board use `Float`, which `prism_syntax` flags as out of this compiler
  (doubt FL).

## Spelling, prism-reflex to the books

| prism-reflex | The books |
| --- | --- |
| `@InterceptSome`, `@InterceptEach`, `@InterceptList` | `@HostSome`, `@HostDocument`, `@HostList` |
| `!a \| !b`, the merge | `!a <\| !b` |
| `let @counter:` | `@counter: @unit`, then `@counter:` |
| `@board`, `@main` for widgets whose result is a `$Text` | `@$board`, `@$main`: a value shows its whole stack |
| `@Button $"Go"`, `@Label "Hi"`, `@Element Ui.X(..)` | `Ui.Button(label=$"Go")`, `Ui.Label(text="Hi")`, `Ui.X(..)` bound where it stands (D10) |
| `type Side = choice White \| Black` | `type Color = choice:` with one alternative a line |
| `derive ^Eq for Side` | `derive ^Eq` on the line after the type |
| `type Element = open choice` | `type Element o = row`, the prelude's |
| `tag Ui.Image of Element:` with `responds: Click` | `tag Ui.Image of Element !Click:`, the output on the tag line |
| `Response`, `Slot` | an answer typed by its tag (`Input` here), a place |
| `Rgb ((Float, Float, Float))`, `Draw.Rect ((Rect, Color))` | `Rgb Float, Float, Float`; `Draw.Rect(rect=r, paint=p)`, a tag of the row kind `Draw` |
| `Map xs, \|x\| ..` over an array | `Each xs, \|x\| ..`; `Map` takes pairs to a map |
| `Append xs, x`, one element | `Push xs, x`; `Append` joins two arrays |
| `Union2`, `Cat`, `Prefix t, n` | `Union`, `Append`, `Before` of `?CursorAt t, n` (`TextBefore`: a text is cut at a cursor, its caret a byte offset) |
| `Fold m, z, \|acc, k, v\| ..` over a map | `Fold (Keys m), z, ..` or `Fold (Values m), z, ..` |
| `Mod a, b`, `Div a, b` | `a mod b`, `Or (a / b), 0` |
| `IsSome x` | `x != nil`, or the pattern `? ignore` |
| `ToFloat n` | `Float n` (doubt FC) |
| `Filter`, `Any`, `All`, `?Find`, `Range`, `Flatten`, `Abs`, `Sign`, `First`, `Second`, `Reverse`, `Rest`, `Prepend`, `Split`, `JoinText`, `Sin`, `Cos`, `MapValues`, `DropLast` | not in Semantics App. B; local Prism helpers (`AnyOf`, `AllOf`, `Kept`, `?FindOf`, `Indices`, `Words`, `Joined`, `Sine`, `Cosine`), patterns, or `Take`/`Drop` (doubt PN) |
| `Initial`, `Squares`, `Plain`, `Nothing`, `WK`: values with capitals | `initial`, `squares`, `plain`, `nothing`, `wk`: case marks a value |
| `?First !squares` for the picked square | `!Filter (?Picked !squares)`: `?First` gives a pair (doubt PZ) |
| chess `Side`, `Piece` (the kind), `Man` (color and kind) | `Color`, `Man` (the kind), `Piece` (`color`, `man`) |
| chess `Sq x, y` of `Int` | `Sq file, rank` of `Nat`, printing as `e2` |
| chess `Game` with `white`, `black` (`Castle` flags), `phantom`, `count` | `Game` with `corners` (unspent), `passed`; no `count` |
| chess `?Basic` then `?Move` | `?Move` over `?Legal`, `?TwoSteps`, `?CastleTo`, `After`, a `Manner` and `RanksOf` |
| window `Color` with `White`, `Black` | `Paint` with `Paper`, `Ink` (doubt CO) |
| window `Ui.Image(src, bg)` | `Ui.Image(src, paint)` |
| window `Routed(path, response)` | `Routed(path, input)`, `Input = Clicked \| Typed Key \| Inner Routed` |
| window `!raw` | `!incoming`: `raw` is a keyword |
| window `Env $sizes, $?focus` | `EnvOf $sizes, $?focus` (doubt EN) |
| `Ui.Focusable` | `Ui.TextInput of Element !Key` |
| tests inside the program file, `prism test`/`prism gui` | `X.prismtest` beside `X.prism`; `prism_zero test` |
| `expect_drawn "Add"` | gone; `returns`, element checks, `click element 0/[e2]/0` (D9) |
| `Press (Point 280.0, 400.0)` in a step | `Press Point(x=280.0, y=400.0)` (doubt PP) |

## Doubts

Each is a place where the books disagree with each other, with the running program, or
with the parser, or say nothing; with Claude's first ruling, for Jimmy to overturn.

- **FL Float.** `claude/decisions.md` puts `Float` arithmetic out of this compiler, and
  `prism_syntax` flags every `Float` (257 in `window.prism`). The window library is `Float`
  throughout, as layout.md's Q-V3 (logical points as floats) and Pitch ch. 5's quoted
  window have it. *First ruling:* `Float`, since the layout design stays unchanged; chess
  cannot run under `prism_zero` or `prismc` until `Float` comes in or the library moves to
  whole points. This blocks chess.
- **FA Sigiled field access.** Type System ch. 1 reads a field with its stack, `g.?passed`,
  `s.?width`; `prism_syntax` refuses a sigil after `.` (its own translated chess-rules
  test wrote `g.phantom`). *First ruling:* the books' spelling stays. The six accesses
  (`rules.prism` 213; `window.prism` 536, 678, 681, 685, 694) are the only parse errors,
  and with the `?` removed every file parses clean and round-trips through `--pretty`.
- **GU A comma after `require`.** Pitch ch. 11 writes `require Clear g, from, corner`;
  `prism_syntax` reads a comma after a guard as a second argument (its pick GU, which D11
  refuses). *First ruling:* the guards are parenthesized, `require (Clear g, from,
  corner)`, which reads the same under both.
- **TC A comma inside a delivered value.** Pitch ch. 8 writes `send Gui.Window(..), Press
  (Point 280.0, 400.0)`; Conformance A.3 splits a step only at bracket depth zero, but
  `prism_syntax` refuses the comma inside the group. *First ruling:* `Press Point(x=280.0,
  y=400.0)`, which is also how a transcript prints it (doubt PP).
- **RK `raw` and `row` as names.** Pitch ch. 5's window binds `!raw`, and the window's grid
  had `let row`; both are keywords (Semantics ch. 2). *First ruling:* `!incoming` and
  `down`.
- **RD The book's chess against the running chess.** Pitch ch. 11 prints a different design
  (`Color`, `Man`, `Piece(color, man)`, `Sq file, rank` of `Nat`, unspent corners,
  `g.?passed`, `?Legal`, `Manner`, `RanksOf`) from prism-reflex's (`Side`, castling flags
  per side, a phantom square, a move counter). *First ruling:* the book's design wherever
  it prints one; the running program's behaviour and its tests where the book is silent
  (selection, promotion to the chosen man, the status sentences). `count` goes, since
  nothing read it. The book types `?Move`'s promotion as `Man` and compares it with
  `King`, which fits the renames.
- **EM The rules module's `emit` line.** Pitch ch. 11 says the rule file has no `emit` line;
  Semantics ch. 6 says every library writes one, and its doubt EM gives a module with no
  blueprints `emit r`. *First ruling:* `emit r`, following Semantics.
- **US `use Chess` or `use Chess.*`.** Pitch ch. 11's board writes `use Chess` and `use
  Window`, then calls `Status`, `?At`, `@Grid` bare, which only `.*` brings in. *First
  ruling:* `use Chess.*` and `use Window.*`.
- **FM Families and modules.** Semantics ch. 6's FM rules that a family is the namespace of
  the module that declares it, which would put the `Ui` tags in `module Ui` and the `Gui`
  tags in `module Gui`; Pitch ch. 11's board uses `Window` and emits `Ui.* | Gui.*`. *First
  ruling:* module `Window` declares the `Ui`, `Gui` and `Draw` families, one file as asked.
  If FM stands, the tags move to `ui.prism` and `gui.prism` and nothing else changes.
- **ST `@$board` or `@board`.** Type System ch. 1 has a value show its whole stack, and
  Pitch ch. 8 and 11 and Conformance ch. 1 write `@$board` and `@$main`; prism-reflex wrote
  `@board`. D10's "outermost sigil only" is about function names. *First ruling:* `@$board`,
  `@$main`; functions keep one sigil (`@Grid`, `@Window`).
- **GR `@Grid`'s parameter.** Type System ch. 1 (doubt GR) types it `fn Nat, $(Path ==> Elem)
  -> @!Routed`; Pitch ch. 11 and the running library call it with a block. *First ruling:*
  `fn Nat, @a -> @a`, the running library's, which the board's call needs.
- **TL `Ui.Tile` or `Ui.Image`.** Pitch ch. 11 draws `Ui.Tile(glyph, paint)`. *First ruling
  (as asked):* `Ui.Image` with the PNGs, as the running program draws, with the Tile's
  `paint` field name in place of `bg`.
- **SP Spot.** A grid lays its children out in their paths' order, and the book's `for sq,
  $shade <- $ shades` keys rows by `Sq`, whose derived order is a1, a2, ..., a8, b1: the
  board would come out turned. *First ruling:* the board keys its rows by `Spot(down,
  across)`, whose order is reading order from a8 and which prints as its square, so a
  place is still `0/[e2]/0` as Conformance ch. 1 writes it. The alternative is a
  hand-written `^Ord` for `Sq`, which no book spells.
- **PZ `!Present (?First !squares)`.** Pitch ch. 11's board takes the picked square this
  way, but App. B's ruling PZ has `!Present` drop the value, and `?First` gives a key and
  value pair. *First ruling:* `!Filter (?Picked !squares)`, with `?Picked` taking the
  value out of `?First`'s pair.
- **PN Prelude names missing from App. B.** The chapters use `Filter`, `Any`, `JoinText`
  and `Map xs, F` (Semantics ch. 4, Type System ch. 3 and 4, Pitch ch. 11); App. B lists
  none of them and says such a name is a slip in the appendix. prism-reflex also used
  `Range`, `Abs`, `Sign`, `Split`, `Sin`, `Cos`, `ToFloat` and others. *First ruling:*
  only App. B's names are taken from the prelude; the rest are written in Prism, under names
  no book gives the prelude (`AnyOf`, `Kept`, `Words`, `Sine`), so they cannot collide when
  App. B grows.
- **MP `Map` and `Each`.** Type System ch. 3 maps an array with `Map xs, Show`; App. B
  gives that job to `Each` and makes `Map` turn pairs into a map. *First ruling:* App. B.
- **FC Nat to Float.** No book says how a `Nat` becomes a `Float`; Type System ch. 2 widens
  with the type's name (`Int n`, `U32 b`). *First ruling:* `Float n`, by the same rule.
- **EN `Env $sizes, $?focus`.** Pitch ch. 5's quoted window builds the layout environment
  with the record constructor over two dynamics, but a constructor takes its arguments
  whole (Type System ch. 2), so that is a bundle, not a `$Env`. *First ruling:* `EnvOf
  $sizes, $?focus`, a function, which lifts.
- **AN How a box answers.** Semantics ch. 9 refuses `Response` (HL) and types answers sent
  down by path with `^Responds`'s `@Answered`, whose blueprint argument no book explains.
  *First ruling:* `?Boxed` matches each `Ui` tag and answers with that tag's own output
  type (`!Click` for a button or an image, `!Key` for an input, `!Routed` for a container,
  `unit` for a label), from a `Routed` record whose `Input` nests one level per container.
  Every arm is typed by matching, with no `@Answered`.
- **DR `Draw` as a row kind.** prism-reflex's `Draw` was a choice whose constructors had dots
  in their names; D9 and Conformance ch. 1 write `Draw.Clickable(rect, path)` with named
  fields. *First ruling:* `type Draw = row`, as `Fault` is, with a tag per command, so a
  command prints with its family and its field names.
- **CP The path on `Draw.Clickable`.** A tile's chain of paths is the grid's place in the
  board's document, then the tile's in the grid's child (`0/[e2]/0`); paths are opaque, so
  they cannot be joined. *First ruling:* the innermost path, as Conformance ch. 1's `click
  element 0/[e2]/0` reads; two grids with the same keys would make a click by path
  ambiguous, and the Test World fails such a line, naming both.
- **CC Where a click by path lands.** Conformance ch. 1 says the Press goes to the
  rectangle's centre and then prints `Press (Point 280.0, 400.0)` for e2, whose centre is
  (288, 416). *First ruling:* the centre; the transcripts print `Press Point(x=288.0,
  y=416.0)`. Pitch ch. 8's test, which presses at (280, 400) itself, is kept as written.
- **CT A click by path through a turn.** A `Draw.Clickable` inside a `Draw.Group` has its
  rectangle in the group's frame. *First ruling:* the Test World applies the enclosing
  groups' turns to the centre, so a turned card is clicked where it is drawn. Chess has no
  turns.
- **PP How a record argument prints.** Conformance ch. 1 writes `Press (Point 280.0,
  400.0)`; Type System ch. 2 prints a record with its fields named. *First ruling:* `Press
  Point(x=288.0, y=416.0)`.
- **AR How an array prints.** Type System ch. 2's printing rules list maps and records, not
  arrays. *First ruling:* as the literal is written, `[[a, b]]`.
- **KY A key in a path.** Conformance A.2 makes a path's `[k]` a Prism expression, and
  `e2` is no Prism value. *First ruling:* a key is matched by its printed spelling, as "a
  transcript's notation is the test's notation" says.
- **EV Reading the board under its window.** The window claims every `Ui` element and
  answers it in Prism, so under `@$main` the World's document holds only `Gui` places and
  `expect Ui.Image(..)` finds nothing. *First ruling:* the game's tests check `returns` and
  the window's own places (`Gui.Metrics` per word), and a separate test reads the board's
  elements under `@$board` alone.
- **WI The window's id under the Test World.** Pitch ch. 6 says the Test World answers
  `Gui.Window` with "a fixed id" and names none. *First ruling:* the window's title,
  `WindowId "Chess"`, as prism-reflex's Test World does.
- **SW First values in the Test World.** Conformance ch. 1 fails a test whose state element
  no `with` entry answers; Pitch ch. 6 has the Test World answer `Gui.Window` and
  `Gui.Metrics` by its own policy. *First ruling:* Pitch ch. 6's policy for the four `Gui`
  tags, so a GUI test needs no `with` lines.
- **TM The Test World's font.** Pitch ch. 6 says the Test World measures from "a font it
  bundles" and does not give it. *First ruling:* prism-reflex's rule, nine units a
  character and fourteen high. The board's scenes do not depend on it: the status is one
  line drawn at (4, 516) whatever its width.
- **ED What `in:` prints for a bundle's event.** `Gui.Window` answers `((WindowId,
  !WindowInput))`. *First ruling:* `in:` prints the event component that fired, `1: Press
  Point(..)`; the plain id was the first value and is not an input.
- **TF `.prismtest` files.** Semantics ch. 6 puts tests in their module, with no `use`
  lines of their own, and Pitch ch. 8 keeps them in the program's file. *First ruling (as
  asked):* a separate file whose declarations are the same-named module's, so the program
  files stay what they ship as and the runner reads both as one module.
- **CK The check transcript.** Conformance ch. 1 gives a check's transcript as its name and
  `1: ok` per line, and no example. *First ruling:* `check name`, then one line per check
  line numbered from 1 in its block, a blank line between checks.
- **HC The half counter's source.** The half counter is Compiler ch. 3's, not Semantics' or
  the Pitch's, and its name there is `@halfcounter`. *First ruling:* that name and that
  program.
- **TD The todo list's source.** No book prints the whole todo list: Type System ch. 2 has
  its `Todo` record, Pitch ch. 11 describes the Star, and prism-reflex has the program.
  *First ruling:* prism-reflex's program in the books' spelling, with a second test for
  the comparison run's identity case (a deleted row leaves the next row's star on it).
