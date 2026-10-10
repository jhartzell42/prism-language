# Lint: ch05-elements-interception.md

Chapter: `/home/claude/book/ch/ch05-elements-interception.md` (1397 lines, ~20.7k words). Line numbers below are the chapter's.

## Part 1: rule violations

### Rule 1 — history / superseded designs

None found. (Line 431 "the list would no longer be plain data" is inside the declared-dynamic-field hypothetical, not history. "old slot"/"old label" throughout means the prior `dyn` generation.)

### Rule 2 — status tracking

None found. (The "cost is not yet paid for … escape hatch" sentence at 433 says what would change it, which is the permitted form.)

### Rule 3 — quoting or attributing Jimmy

None found.

### Rule 4 — banned words

None found. (`Text.Shape` at 1030 is a tag name.)

### Rule 5 — clefts and non-default word order

1. Line 1203: "The suite's `mock_timing` test checks exactly that, and it is the test that catches the class of mock that hides a timing bug the real World would have shown."
   Cleft ("it is the test that…") plus a stacked relative chain. Fix: "The suite's `mock_timing` test checks exactly that; it catches the mocks that hide a timing bug the real World would have shown."

### Rule 6 — lecturing tone

1. Line 273: "Read the second sentence twice, because it is the argument and not just the rule."
   Instructs the reader how to read; he knows the rule. Fix: cut the sentence; start at "Suppose an element could be created and answered in one occurrence."
2. Line 517: "Read `@Forward` slowly, because every interceptor in this chapter is a variation on it."
   Same. Fix: "Every interceptor in this chapter is a variation on `@Forward`."

### Rule 7 — dry definition before motivation

1. Line 23 (section "Tags" opens): "A tag is the declaration of one kind of element: its name, its fields, and which of three things it does."
   Definition then a code block; the motivation is five paragraphs up in another section. Fix: open with the one-line reason tags exist ("You want a socket to be one thing with three parts, not three elements you have to keep together. A tag says that in one place.") and let the declaration follow.
2. Line 237 (section "`@Element`" opens): "`@Element e` is a widget. Binding it in a widget block creates a slot for `e` in the enclosing node and returns what `e`'s tag declares:" followed by a type table.
   Fix: lead with the grep observation now buried at 246 ("a reader who wants to know everything a widget asks of the world greps for one word") or with the three-line `@viewer` example at 253, then the table.
3. Line 323 (section "The element list" opens): "The element list of a widget is an ordinary dynamic, `$(Path ==> Slot)`: a map from path to slot, in tree order because paths are ordered that way."
   Fix: lead with what you get for free because it is a dynamic (the `GuiOnly $slots` example at 333, and "a `for` over it is how every interceptor is written"), then the type.
4. Line 351 (section "Two laws across the boundary" opens): "Everything in this chapter rests on two laws, and each binds a named party."
   The laws follow immediately. Fix: one sentence of the lived failure each law excludes (two rows of a table showing different mtimes for one file; a mock that answered in the request's occurrence and hid a race) before the bold law statements.

### Rule 8 — presence or drop as a trigger

1. Line 221: "A subprocess is a socket with a command instead of an address: the World starts it because the element is in the list, stops it when the element leaves, and its exit status is a fact."
   The start is triggered by the element appearing. Fix: "A subprocess is spawned by a request: `Process.Spawn` fires with the command, responds with an id, and `Process.Child(id)` is the state of that process; the World kills it under its cancel-on-teardown obligation when the slot goes." (Also rule 9.)
2. Line 414 (table row): "`Process.Child(command)` | the command | right | A different command is a different process. The old one is stopped when its element leaves the list."
   Fix: drop the row or rewrite it for `Process.Child(id)`, whose only field is the id the World allocated and never changes; a new command is a new `Process.Spawn` request, not a field change.

### Rule 9 — design conflicts with the BIBLE Decisions

1. Lines 62 and 213–214: "`Cli.Exit`" in the table; "tag Cli.Exit of Element: # sink: the firing is the exit / requests: !Code".
   Fix: `tag Process.Exit of Element: requests: !Code`. Update the table row (`Cli.Stdout`, `Process.Exit`, `Log.Line`).
2. Lines 65, 204–205, 221, 1348: "`Cli.Args`" as a state element; "Arguments are state rather than parameters of `@main` so that an interceptor can give a subtree different ones"; "Cli.Args(..): @ $ []".
   Fix: `Process.Args` (state) everywhere; `Process.Env` is the companion. The reasoning sentence stands.
3. Lines 210–211 and 62: "tag Cli.Stdout of Element: # sink / requests: !Text", classed as a sink with "nothing comes back".
   Fix: `Cli.Stdout` and `Cli.Stderr` are `requests: !Text, responds: WriteFailed`. Move `Cli.Stdout` out of the sink row; `Log.Line` and `Process.Exit` can stay there.
4. Lines 173–175 and 64: "tag Time.After of Element: # request: fires back once, after the duration / requests: !Duration / responds: Instant".
   Fix: `requests: !Nat, responds: Nat`, the token echoed back (the duration in the request, the same value in the response so two in flight can be told apart). Drop `type Duration` unless something else uses it.
5. Lines 94–97, 651–654, 661, 991, 1008: "tag Gui.Window of Element: … responds: WindowInput / state: Size"; "the window's size is a state element, a fact the World pushes when the user resizes"; "The World pushes `Size 600, 300` for `Gui.Window(title="Form")`".
   Fix: `Gui.Window` responds with pointer, key and resize events and has no state; add `Resized Size` to `WindowInput`, hold `$size` from it in `@Window` (`$size <- initial <= !Filter (ResizedOf !raw)`), and rewrite occurrence 5 as a `Resized` response delivered to `1`. The argument at 661 about value identity not changing per frame then applies to the window's path, not its state.
6. Lines 99–101, 633–636, 656–658, 953, 995: "tag Gui.Scene of Element: window: Text / ops: [Op]" with `type Op = choice: Fill … | Text … | Clip …`.
   Fix: the field is `draw`, a plain list of `Draw.*` values (`Draw.Rect`, `Draw.Text`, `Draw.Image`, `Draw.Clip`); `Op` becomes `Draw` and `Place` emits `Draw.Rect (r, Grey)`, `Draw.Text (p, t)`, `Draw.Clip (r, ops)`. `@Element Gui.Scene(window=title, draw=$placed.draw)`.
7. Lines 28–33, 83–88, 19, 61–63, 249–251, 592: `Gui.Label` and `Gui.Button` declared as World-facing tags ("the GUI … vocabularies a World is assumed to accept"); "`@Button: fn Text -> @!Click = \|label| @widget: !click <- @Element Gui.Button(label=label)`"; "a `Gui.Button` under a CLI World".
   Fix: the primitive `Gui.*` vocabulary is `Gui.Window`, `Gui.Scene`, `Gui.Focusable`, `Gui.Metrics`, `Gui.Frames` only. Buttons and labels are `Ui.Button` and `Ui.Label` (the library's tags, which reach a World only if it accepts them). Rename in the first two declaration blocks, in `@Button` at 249, in the table at 61–63, and at 592.
8. Lines 104, 613, 617, 663: "The last two are the only GUI elements a World that draws pixels needs to accept"; "two World-facing tags"; "A World that offers these two tags can run every program in this section."
   Fix: there are five `Gui.*` tags; say the library in this section uses two of them (`Gui.Window`, `Gui.Scene`) and that `Gui.Metrics` (state: text measurement, answered synchronously), `Gui.Focusable` and `Gui.Frames` (responds `Frame(now: Instant)`) are the rest.
9. Lines 90–92 and 418: "tag Gui.Focusable of Element: # stream: keys arrive, when focused / name: Text / responds: Key"; "the keystrokes arrive at a `Gui.Focusable` leaf whose only field is a name".
   Fix: `Gui.Focusable` has only `name`; keys arrive at `Gui.Window` and the library routes them. Either drop `responds: Key` and say what a World does with a `Gui.Focusable` (IME anchoring, accessibility), or state that the library's `Ui.Focusable` is what the window interceptor handles and `Gui.Focusable` is what it would re-emit for a World that wants to know where focus is. Line 418 should name `Ui.Focusable`.
10. Lines 198–202 and 221: "tag Process.Child of Element: command: [Text] / requests: !ChildRequest / responds: ChildEvent / state: Status".
   Fix: `Process.Spawn` is a request element (`requests: !Spawn`, responds with the id); `Process.Child` is keyed by id (`id: Nat`) with the stdin/signal requests, stdout/stderr responses and `Status` state, and a child process is killed under the World's cancel-on-teardown obligation. The table rows at 67–68 then read `Process.Child(id)`.
11. Lines 679–681, 1014, 1021–1023: "tag Ui.Button of Element: label: Text / responds: Input"; "`Send Ui.Button(label="OK"), Clicked`".
   Fix: worked examples under the Test World use `Ui.Button` responding `Click` (value `click`). If the library wants one `Input` type for routing, say that the suite's `Ui.Button` responds `Click` and the library's container wrapping is `ChildInput(path, response)`, see next item.
12. Lines 670–674, 685–700, 767, 1014: "type Input = choice: Clicked | Keyed Key | Scrolled Int | Into (Path, Input)"; containers respond `Input` and route `Into (p, i)`.
   Fix: the suite's VBox responds `ChildInput(path, response)`. Rename `Into` to `ChildInput` throughout the library and the test at 1019–1024, and say the response carried is the child's own response type (so `Click` reaches a `Ui.Button`).
13. Line 1183: "checks that the second turn has `NotRequested Net.Http(..)` at the top".
   Fix: the suite step is `Unrequested Net.Http(..)`.
14. Lines 305 and 989: "Every slot has a path: its position in the widget tree, made of the bind positions, `for` keys and `dyn` generations from the root down to it"; "the text input's focusable at `0/0/1/#0/0` … (the box's second bind, generation zero, first slot)".
   Fix: only slots, `dyn` generations, `for` keys and intercept children take segments; a plain widget-function application makes a node but adds no segment (a `@Button` whose body holds the `dyn` prints as `0/#0/0`). State the rule that way at 305, and re-derive every path in "A form, occurrence by occurrence" under it (the `@TextInput` and `@Focusable` applications contribute no segment of their own). Supplier-built slots print `s/n/…`; mention this where the supplier is introduced (527).
15. Line 1030: "Text shaping with a real font, so that `Measure` is not eight pixels per character, is a request element: the text input fires a `Text.Shape` request".
   Fix: text measurement is `Gui.Metrics`, a state element answered synchronously by the World, so `Measure` reads it as a dynamic and there is no last-known-glyphs hold.

### Rule 10 — unverified specifics

None to flag. (`componentDidMount` at line 7 is in the research notes as a familiar example, not a documented recommendation, and the chapter uses it that way. `std::fs`, 403, 429, GTK, X11 are general knowledge, not quotes or attributions.)

### Rule 11 — code that cannot be Prism

1. Lines 213–214: `tag Cli.Exit of Element: requests: !Code` — see rule 9 item 1; `Cli.Exit` does not exist.
2. Line 1346–1347: "let supply = \|e| match e: / Clock: @ $clock" — a bare-tag pattern without parentheses while every other arm writes `Tag(..)`. Fix: `Clock(..): @ $clock` for consistency with the pattern syntax used elsewhere, or state once that a field-less tag may be matched bare.

### Rule 12 — filler

1. Line 57: "…and it is worth seeing them as combinations of three parts rather than as five different mechanisms:"
   Fix: "…and they are combinations of three parts, not five different mechanisms:"
2. Line 468: "This is what the first design is with a nicer type, and it fails the same way, but it is worth separating because its failure looks like a feature."
   Fix: "It is the first design with a nicer type, and it fails the same way; its failure looks like a feature, which is why it gets its own paragraph."
3. Line 1181: "The consequence is one you should expect by now:"
   Fix: cut; start at "two identical GETs in one occurrence from two child slots both miss".

## Part 2: structural notes

1. **The `dyn` expansion is given twice.** Line 267 says "The chapter says this once here and uses the short form from now on", then "Dynamic fields" (359–397) gives the full `!Switch` / `$Join` expansion again with code. Keep the code in "Dynamic fields" and cut 267 to one sentence pointing there, or the reverse.
2. **"Recapitulation and depth" (605–609) is mistitled and duplicates "The children convention, and tests that look inside" (1012–1026).** Both say that containers hold children as data, that the Test World descends `children` fields, and that `Send` wraps one `Into` per level. Merge into one place, after the containers are defined, and give the depth section a title that says what it is (containment by data).
3. **The path derivation at 989 is garbled** ("at the window's path `0`... but the window's child *is* the box"). Replace with a small table: each slot in the form, its owning block, its path under the segment rule, and which interceptor's list it is in. The occurrence walkthrough then reads against the table.
4. **`Net.Conn` (168) is referenced and never declared**, and `Net.Listener` responds `Accepted Nat` with no account of how the accepted connection is then addressed. Declare `Net.Conn(id: Nat)` with its parts, parallel to `Process.Child(id)`, and say the ids are the World's and never reused.
5. **Who is bound and what enforces it is missing in three places.** Line 185 "The clock is a fact the World pushes, at most once per turn": bound by the World, enforced by the snapshot being held still across a turn; say so. Line 279 "a conforming World does not [act on a slot appearing]": enforced only by the suite's `Unrequested` step after a build with no request firing; name it. Line 523/602 transparency: the differential test is named but not what transcript equality means for supplier-built slots (`s/n/…` prefixes must be stripped or the test compares below them).
6. **Errors are named only as "an error".** Unknown path (281, 337, 502), unsupplied state (592), deferring backwards (586) are `Error` with `kind: UnknownPath`, `UnsuppliedState`, `DeferBackwards`, aborting the occurrence and stopping the rest of the turn, reported through the World's `Errors` stream element. One paragraph in "Routing" or "Invariants" giving the kinds and what the World sees, with a pointer to the errors chapter.
7. **The once-per-turn `Defer` bound and `dyn`-reborn slots.** Every worked interceptor wraps each slot in `dyn $_:`; a `Defer` reborn at the same tree position counts as the same `Defer` for the bound. The "Answering a request" section should say this, since it is what makes the retry-loop detection hold when the child's slot is itself a fresh `dyn` generation per attempt.
8. **"What an interceptor may not do" (588–592) is thinner than its title.** It covers denial-must-be-typed and unknown tags. Add the other things the design forbids and what enforces each: it cannot read the child's captured scope (the `$theme` argument at 464 belongs here as the positive statement); it cannot address a path that is not in its child's list; it cannot answer in the request's occurrence; it cannot be reached by the child.
9. **"What the library does not do, yet" (1028–1030) should be rewritten against the five `Gui.*` tags.** Text measurement is `Gui.Metrics`; animation and frame-driven scrolling are `Gui.Frames`; images are `Draw.Image`. Size feedback needing a `Defer` is a good example and deserves three lines of code, since it is the one layout case the plain-function layout cannot do.
10. **The Prism-hosted World (1331–1368) leaves `@Launcher` and `@Tiled` undefined** and does not show how a guest's `Ui.*` elements reach the host's window (they are in `$lists`, and `@Tiled` must put them into one `children` field per guest and route `ChildInput` back by guest name then path). Six lines of `@Tiled` would close the loop and show that a guest's path space is isolated by the extra `for` key.

## Counts

| Rule | Count |
| --- | --- |
| 1 history | 0 |
| 2 status | 0 |
| 3 attribution | 0 |
| 4 banned words | 0 |
| 5 clefts / word order | 1 |
| 6 lecturing | 2 |
| 7 dry definition first | 4 |
| 8 presence/drop trigger | 2 |
| 9 BIBLE conflicts | 15 |
| 10 unverified specifics | 0 |
| 11 non-Prism code | 2 |
| 12 filler | 3 |
| Structural notes | 10 |
