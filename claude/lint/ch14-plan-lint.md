# Lint: ch14-plan.md

Line numbers are from `/home/claude/book/ch/ch14-plan.md` as read.

## Part 1: rule violations

### Rule 1 — history / superseded designs (3)

- L346: "The twelve integration programs brought to the current syntax (numbers by inference, `tag` declarations for the `Gui` set, `Click` as a unit type, the `intercept` signature with its three arguments)". "Current syntax" implies an earlier one. Fix: "The twelve integration programs, written against the syntax of the language chapters, …" and drop the parenthetical list of what changed.
- L367: "The counter test from the worked examples chapter, including `counter_left_wins` and the sandbox script with its corrected expectations, passes as written there." "Corrected" refers to an earlier wrong version. Fix: "…and the sandbox script, passes as written there."
- L379: "Bringing the twelve programs to current syntax will surface sentences in the language chapters that two readers would read differently." Fix: "Writing the twelve programs against the language chapters will surface sentences that two readers would read differently."

(L43, L51, L95 describe the state of the code on the branch the plan starts from, which the BIBLE supplies as "current Rust runtime facts". Not flagged: a plan has to say where it starts. L613 "the previous model's choices" is about the process, not the design.)

### Rule 2 — status tracking (1)

- L262: "`prism dump` of the counter is this, or the frozen equivalent once the form settles during the milestone". "Settles" is status. Fix: "`prism dump` of the counter is the dump the language-implementation chapter gives" (and show that dump; see rule 9).

### Rule 3 — quoting / attributing Jimmy (0)

None. L595 "The author will execute this plan with a model" is a statement about the process, not an attribution.

### Rule 4 — banned words (1)

- L328: "the hand-written counter has to be shaped like compiler output". Fix: "has to have the structure of compiler output" or "has to be built the way the compiler would build it".

### Rule 5 — clefts and word order (0)

None found. L293 "This is where the tracks meet", L187 "It is the milestone that makes the claim … testable" are demonstrative sentences with the subject first; not clefts.

### Rule 6 — lecturing tone (0)

None. The numbered "first/second/third/fourth" in L607–613 is a procedure, and the bullet lists are work items.

### Rule 7 — dry definition before motivation (1)

- L484–508, "The vocabulary": the section opens with five tag declarations and only afterwards (L510) says why `Text.Measure` is state and what the library does with the tags. Lead with the thing the reader has lived through: a layout that needs the width of a string before it can place the next widget, and a World that has not loaded the font yet; then the declarations.

### Rule 8 — presence or drop as a trigger (0)

None. L406 ends the run on `Exit`'s request and on "nothing can cause another occurrence", both of which the BIBLE permits. L405 "read only while a slot exists" and L514 "a turn per frame while a slot exists" describe World behaviour for a presence element, which is description, not a trigger. L395 `created`/`removed` on `Component` are the World's own bookkeeping hooks for presence elements and the cancel-on-teardown obligation.

### Rule 9 — conflicts with the BIBLE Decisions (31)

Tag vocabulary: `Gui.*` is the primitive vocabulary (`Gui.Window`, `Gui.Scene`, `Gui.Focusable`, `Gui.Metrics`, `Gui.Frames`); the library's tags are `Ui.*`; worked examples under the Test World use `Ui.Button` responding `Click`.

- L126: "`Button` creating a `Gui.Button` slot inside a `dynamic_widget`". Fix: `Ui.Button`.
- L136–148 (transcript): `Gui.Button(label="Increment")`, `Gui.Label(text="Count: 0")`. Fix: `Ui.Button`, `Ui.Label`. Also L140, L145: `responses: {0/#0/0: Click}` — unit values are lowercase (`click`), and chapter 9's transcript prints `click`. Fix: `{0/#0/0: click}`.
- L266–267, L272 (dump): `element Gui.Button(label=l)`, `element Gui.Label(text=t)`. Fix: `Ui.Button{label=…}`, `Ui.Label{text=…}` per chapter 11 (see also the text-form item below).
- L346: "`tag` declarations for the `Gui` set". Fix: "for the `Ui` set".
- L351–361: `tag Gui.Label of Element:`, `tag Gui.Button of Element:`, `tag Gui.VBox of Element:` with `responds: Gui.ChildInput`. Fix: `Ui.Label`, `Ui.Button`, `Ui.VBox`, and the suite's VBox responds `ChildInput(path, response)` (a `ChildInput` type with those two fields, not `Gui.ChildInput`).
- L340: "a `Gui.Children(envelopes=…)` response to route inward". Conflicts with L363 and with the Decision that the suite's VBox responds `ChildInput(path, response)`. Fix: "a `ChildInput(path, response)` response to route inward".
- L363: "re-emit the `Gui.*` ones inside one `Gui.VBox` element … route `Gui.ChildInput(path, response)` inward". Fix: "re-emit the `Ui.*` ones inside one `Ui.VBox` element … route `ChildInput(path, response)` inward".
- L370: "`@VBox` is an interceptor and `@Button` is `@Element` plus a filter" — fine, but the preceding "GUI is a library" sentence should name `Ui.*`.
- L407: "a `Gui.Button` at the root getting `Unsupported`". Fix: `Ui.Button`.
- L489–508: `tag Gui.Window … responds: Resized (Nat, Nat) | Closed`; `tag Gui.Draw of Element: commands: [DrawCommand]`; `tag Gui.Focusable of Element: region: Rect, responds: Pointer … | Key … | FocusIn | FocusOut`; `tag Gui.Frame … responds: Tick Instant`; `tag Text.Measure … state: Unavailable | Known (Nat, Nat)`. Fix: `Gui.Window` responds with pointer, key and resize events (one per window); `Gui.Scene` (presence) with a `draw` field that is a plain list of `Draw.Rect`, `Draw.Text`, `Draw.Image`, `Draw.Clip` values; `Gui.Focusable` with a `name` field only, no region and no responses (focus and hit-testing are library concepts); `Gui.Frames` responds `Frame(now: Instant)`; `Gui.Metrics` (state) for text measurement. Draw commands are values, not tags.
- L510: "producing `Gui.Draw` and positioned `Gui.Focusable` leaves … keystrokes arriving at a `Gui.Focusable` with no dynamic fields". Fix: the layout interceptor produces one `Gui.Scene`; pointer and key events arrive on `Gui.Window` and the library routes them; `Gui.Focusable` has only a name.
- L514: "Components: `Gui.Window`, `Gui.Draw` …, `Gui.Focusable` (pointer and key events from `winit`, routed to the region under the pointer or the focused region; a turn per event), `Gui.Frame` …, `Text.Measure`". Fix: `Gui.Window` (delivers pointer, key, resize), `Gui.Scene` (presence; rasterised after each turn), `Gui.Focusable` (name only; the World does no geometry), `Gui.Frames`, `Gui.Metrics`.
- L518: "`Gui.Draw` becomes absolutely positioned `div`s …; `Gui.Focusable` becomes a focusable element with listeners; `Text.Measure` is answered from a hidden `canvas`". Fix: `Gui.Scene` becomes the `div`s; pointer and key listeners sit on the window element and are delivered as `Gui.Window` responses; `Gui.Metrics` from the canvas.
- L522: "turns its children's `Gui.Draw` lists into request bytes … turns `Received` bytes into `Pointer` and `Key` responses routed to its children's `Gui.Focusable` slots … `Text.Measure` is pure". Fix: `Gui.Scene`'s `draw` field; responses routed as `Gui.Window` events to the child the library hit-tests; `Gui.Metrics`.
- L534: "Measurement by a state element is right" — keep, but name it `Gui.Metrics`, and note the Decision that `Gui.Metrics` is answered synchronously (so the "font loaded later → push" case is a World that answers `Unavailable` synchronously and pushes later, which is what the paragraph says; just fix the name).

Process tags: exit, args and env are `Process.*`; `Cli.*` is only stdin, stdout, stderr, signals.

- L405: "`Cli.Args` and `Cli.Env` (state, constant after start)". Fix: `Process.Args`, `Process.Env`.
- L405: "`Cli.Exit` (`requests: !Code`; ends the run after the turn's writes)". Fix: `Process.Exit`.
- L406: "Exit: on `Cli.Exit`'s request, after the turn". Fix: `Process.Exit`.
- L405: `Fs.Contents` "(state, stat-and-reread, or a watch where one exists)" — fine; but `Fs.Watch` (state holding `Missing | Modified(stamp)`, answered from a real stat at creation), `Process.Spawn`/`Process.Child(id)` and `World.Tags` are missing from the component list. Add them (also a Part 2 item).

Errors: one type `RuntimeError { kind: ErrorKind, spans, occurrence, … }`; `ErrorKind` is the closed choice `Cycle | Overflow | UnknownPath | UnsuppliedState | DeferBackwards | Component | Build`; `Outcome` has `errors: Vec<RuntimeError>`; a failed `dyn` build does not fire the `dyn`'s event and is reported through `Errors`; the World offers `Errors (responds: Error)`; quarantine is a `quarantine: [Path]` input on the next run call.

- L49: "expects `Err(RuntimeError::Cycle { .. })`". Fix: "expects `Err` with `kind == ErrorKind::Cycle`".
- L51: "becomes `RuntimeError::Panic { message, node: Option<NodeId> }`". There is no `Panic` kind. Fix: a caught panic is kind `Build` when it happens inside a build and `Component`-adjacent otherwise is not right either; the closest Decision is: a panic in an action aborts the occurrence as a `RuntimeError` whose kind is `Build` (if during construction) or `Overflow`/`Cycle` as detected; state which, and put the panic message in the report's text. At minimum: "becomes a `RuntimeError` with the message in its report; nothing resumes it."
- L105: "`RuntimeError::UnknownPath { path }`". Fix: "a `RuntimeError` of kind `UnknownPath` naming the path".
- L117–121: `pub struct Outcome { list, requests, result }` lacks `errors: Vec<RuntimeError>`; `Inputs` lacks `quarantine: Vec<Path>`; both use `AnyValue` where the Decision names the boundary's plain value type `Value`. Fix: add `errors` and `quarantine`, and spell the boundary type `Value` (keep `AnyValue` as the runtime's internal enum if the chapter wants, but the boundary structs use `Value`).
- L197: "`HarnessError::DeferredBackwards { path }`". Fix: kind `DeferBackwards` (one spelling across the book).
- L215: "`RuntimeError::UnsuppliedState { tag, path }`". Fix: "a `RuntimeError` of kind `UnsuppliedState` with the tag and path in its report".
- L548: "the kind (`Cycle`, `UnknownPath`, `UnsuppliedState`, `WrongResponseType`, `DeferredBackwards`, `Panic`)". Fix: "the kind (`Cycle`, `Overflow`, `UnknownPath`, `UnsuppliedState`, `DeferBackwards`, `Component`, `Build`); a wrong-typed or duplicate response from the World is `Component`; runaway recursion is `Overflow`".
- L549: "wrap it so that an error inside leaves the `dyn`'s slot holding an `Error(span, message)` element instead, and the rest of the occurrence continues. The `Error` tag is declared in the prelude as a presence element." Fix: a failed `dyn` build does not fire the `dyn`'s event, the old generation stays, and the failure is reported in `Outcome.errors` and through the World's `Errors` stream element (`responds: Error`, with `kind`, `spans`, `occurrence`). No `Error` element in the list.
- L551: "Checked arithmetic as the default spelling with overflow returning `?`, an unchecked form for the hot loop". Fix: plain `+ - * /` abort the occurrence on overflow (kind `Overflow`); `Checked.*` returns `?`; `Wrap.*` wraps.
- L552: "Quarantine of the owning subtree (replace the lowest common owning widget's node with an `Error` element in the next occurrence) as a World option on the core". Fix: the runtime remembers the owners it named in its last error; the World passes `quarantine: [Path]` on the next run call; the quarantined subtree is removed, not replaced by an element.
- L558: test `a_cycle_found_while_a_dyn_rebuilds_aborts_the_build_and_leaves_an_error_element_in_the_list`. Fix: `…_aborts_the_build_keeps_the_old_generation_and_reports_it_in_errors`.
- L562: `quarantine_replaces_the_owning_subtree_with_an_error_element_when_the_world_asks_for_it`. Fix: `quarantine_removes_the_subtrees_the_world_names_on_the_next_run_call`.
- L567: "an interceptor that wants to handle a child's failure handles an `Error` element like anything else a child emits". Fix: "…subscribes to the World's `Errors` element like any other element".
- L571: "Whether aborting an occurrence opened by a `Defer` answer aborts the rest of the turn is a World decision; the core should abort the turn". The Decision is fixed: an occurrence aborted inside a turn stops the rest of the turn and the World acts on what committed; a failed turn drops its responses and re-queues its pushes. Fix: state it as the rule and drop "is a World decision".
- L631: "an error the program can observe other than as an `Error` element". Fix: "other than through the `Errors` element".

`Runtime` is `Send` by `unsafe impl` with the unique-ownership argument:

- L47: "If `Runtime` ends up `!Send` because something inside holds an `Rc` to a thread-local, that is a bug in this step, not a reason to add `unsafe impl Send`." Fix: "`Runtime` is `Send` by an `unsafe impl` whose justification is unique ownership (every `Rc` inside is reachable only through the one `Runtime`), and `!Sync`; the test moves it to a thread to show the claim holds."
- L621: "Milestone 1: `Arc`, `Mutex`, `unsafe impl Send`, …". Fix: drop `unsafe impl Send` from the grep list (there is exactly one, on `Runtime`); grep for a second one.

Paths: only slots, `dyn` generations, `for` keys and intercept children take segments; a plain widget-function application adds no segment; supplier-built slots print `s/n/…`.

- L101: "`Segment = Bind(u32) | Key(ElementValue) | Gen(u64)` … `bind` increments the node's bind counter and the child's path is the parent's plus `Bind(n)`". Fix: `Segment = Bind(u32) | Gen(u64) | Key(Value) | Supplied(u32)` as chapter 10 has it; a position is taken by a slot, a `dyn`, a `for`, an intercept's captured child and `@started`, numbered in build order within the nearest addressable node; a plain `<-` of a widget application makes a node and adds no segment. Add `s/n` to the `Display` note.

Text form per chapter 11:

- L265–272: the dump uses `\l -> element Gui.Button(label=l)`, `map2 (+ : Int)`, `fmt "Count: "`, `const -1`, and marks every graph-rooted binding `[build]`. Chapter 11's dump of the same program has `bind @Button(const "Increment")`, `element Ui.Button{label=label} responds`, `leftmost [map !decrement \|ignore| -1:Int, …]`, `hold 0:Int (tag !delta [$count] \|d, c| Num.Add[Int] c, d)`, `dmap [$count] \|c| cat [...]`, and `[occ]` on `!delta`. Fix: replace the block with chapter 11's dump verbatim (lines 1051–1059 there), and say the frontend milestone's golden is that text.
- L256: "`name: sigil type [mark] [group] = expr @span`, with every lift an explicit node (`map`, `tag`, `hold`, `leftmost`, `switch`, `join`, `dyn`, `for`, `element`, `intercept`, `placeholder`, `close`)". Add `bind`, `dmap`, `hold_first`, `map_n`/`tag_map_n` per the Decisions and chapter 11, and the "can fire at birth" flag printed on every `for` node.

`IrWidget` and `Program`:

- L301: "`Program { functions: Vec<Function>, tags: Tags, steps: Vec<Step> }`, `IrWidget { program: Rc<Program>, function: FunId, captures: Vec<AnyValue> }`". Fix: `IrWidget { program, block, captures, span }` and `Program { functions, blocks, tags, constants }`.
- L303: "`started` as a `dyn` over a constant" — fine. "`hold`" step: add `hold_first` (the dynamic-returning widget lift uses it, L287).

Suite steps: `Unrequested` and `ExpectTopNone` exist; `State` steps may precede the build and must for any state tag created at build; no `default:` in tag declarations.

- L338: the step list "`Expect`, `ExpectTop`, `ExpectNone`, `ExpectCount`, `Send`, `Together`, `State`, `Push`, `Returns`, `Fired`, `Quiet`, `Requested`, `Errors`". Fix: add `Unrequested` and `ExpectTopNone`; note `Returns` on an event result is a script error.
- L340: "fails the slot's creation with the unsupplied-state error if a state element has no `State` and its tag declares no default". Fix: "…if a state element has no `State` step before it is created; `State` steps may precede the build and must for every state tag the program creates at build".

World core snapshot: held still across a turn; pushes enter only with the first occurrence.

- L392: "before each occurrence the oracle closure answers from the previous snapshot and the pushes are the diff; the swap happens after the occurrence". Fix: "before each *turn* the oracle answers from the previous snapshot and the pushes are the diff, delivered with the first occurrence; the snapshot is held still across the turn and swapped after it; a failed turn drops its responses and re-queues its pushes".
- L107: "Pushes at the start of an occurrence fire the trigger for each pushed value." Fix: "Pushes arrive with the first occurrence of a turn" (the runtime sees one occurrence per call, so inside the runtime this is "pushes in `Inputs` fire at height 0"; say that and leave the turn rule to the core).

Intercept signature `$(Path ==> Slot)` where `Slot` has `element` and `!request`:

- L191: "`Output = (W::Output, Dynamic<Rc<BTreeMap<Path, ElementValue>>>)`" — the captured list maps paths to element values only, but L199's `Forward` "re-creating the element with the same request event" needs the request event per slot. Fix: the captured list's value type is a `Slot { element: ElementValue, request: Option<Event<Value>> }`, and `@Element`'s result has no graph edge from the request field (the creation rule is a compiler fact).

`Kind` record:

- L99: "`TagKind { requests: Option<TypeId>, responds: Option<TypeId>, state: Option<TypeId> }`". The language-level record is `Kind { requests: Bool, responds: Bool, state: Bool }` and `Kind: fn Element -> Kind` is a prelude function. Fix: keep the Rust `TagKind` if the `TypeId`s are wanted for debug assertions, but say that `Kind` (three `Bool`s) is what the prelude exposes and that `TagKind` reduces to it.

Haskell `coincidence`:

- L462: "no `switchPromptly`, `switchPromptlyDyn` or `coincidence`". Fix: "…or `coincidence`, except the one `coincidence` over the membership patch of a `for` whose typed-AST node is flagged as able to fire at birth".
- L457: "`for` is `listWithKey`". Fix: "`for` is `listWithKey`, plus one `coincidence` over the membership patch when the node's can-fire-at-birth flag is set".
- L629: "Milestone 9: `current` outside `tag`; `switchPromptly`; `getPostBuild`; `performEvent` doing work." Add "`coincidence` anywhere but the flagged `for`".

Rollback in Haskell is by replay on a fresh local timeline; quarantine scripts are Rust-only:

- L470: "the boundary (slots, paths, the run call, state supply, `Defer`) can be put on Reflex without changing it" — no conflict, but the milestone says nothing about how an aborted occurrence rolls back under Spider. Add the replay rule and that the quarantine scripts are excluded from the Haskell diff (also Part 2).

### Rule 10 — unverified specifics (9)

Each: verify, or mark from memory, or cut.

- L457: `fireEventsAndRead` and `newEventWithTriggerRef` — not in the Reflex sources under `/tmp/claude-0/rx/` nor in the research notes.
- L457: `DynamicWriterT` — not in the Reflex sources or the notes.
- L431: `megaparsec` — not in the notes (chapter 13 names it too, equally unverified).
- L514: `winit`, `softbuffer`, `fontdue` — `winit` is in chapter 16's verified list; `softbuffer` and `fontdue` are not in the notes.
- L526: `Xvfb`, `xwd` — not in the notes.
- L522: X11 request names `CreateWindow`, `MapWindow`, `PolyFillRectangle`, `ImageText8`, `ButtonPress`, `KeyPress`, `Expose`, `ConfigureNotify` — not in the notes.
- L579: `Cranelift` — not in the notes.
- L167: `static_assertions` — crate name not in the notes.
- L41: "38 tests passing" and L17 "a crate of five thousand [lines]" — 38 is in the BIBLE; "five thousand" is not.

(`listWithKey`, `holdDyn`, `switchHold`, `runSpiderHost`, `networkView`, `subscribeAndRead`, `PerformEventT`, `switchPromptlyDyn`, `getPostBuild`, `coincidence`, `dyn`, `leftmost`, `tag`, `current` are all in the Reflex sources.)

### Rule 11 — code that cannot be Prism (3)

- L266–272: typed-AST text with `\l -> …` lambdas and `Gui.Button(label=l)`. The text form's lambda is `\|l| …` and element construction is `Ui.Button{label=l}` (chapter 11). Fix as under rule 9.
- L492, L499, L502, L507: `responds: Resized (Nat, Nat) | Closed`, `responds: Pointer PointerEvent | Key KeyEvent | FocusIn | FocusOut`, `responds: Tick Instant`, `state: Unavailable | Known (Nat, Nat)`. Every other chapter's tag declaration names a declared type in `responds:`/`state:` (`responds: WindowEvent`) with the constructors in a `type` declaration or a trailing comment; inline `|` choices and `Constructor Payload` spellings are not the declaration syntax. Fix: declare the types and name them (and rename the tags per rule 9).
- L359: `children: Path ==> Element` — fine as a plain field, but if `@VBox`'s children keep their request events it cannot be `Element`; say the field is the plain element list (requests excluded from equality and from the field).

### Rule 12 — filler (1)

- L544: "adds the one containment that is clearly right". Fix: "adds the construction-abort" (and the reason is already in the next paragraph).

## Part 2: structural notes

1. **The whole chapter was written against an earlier GUI vocabulary and an earlier error story.** Milestones 10 and 11 need rewriting against the Decisions, not patching: `Gui.Scene` with `Draw.*` values, name-only `Gui.Focusable`, `Gui.Metrics`, `Gui.Frames`; the `Errors` stream element and `Outcome.errors`, no `Error` element in the list, `quarantine: [Path]` on the run call, `Wrap.*`/`Checked.*`. Milestone 6's library (L348–363) and milestone 2's transcript change tag names throughout. The lint entries above are the inventory.
2. **Milestone 2 has no step for `Outcome.errors` or the `quarantine` input, and milestone 11 does not say which milestone adds them to the boundary.** Say in milestone 2 that the two fields exist and are empty until 11, so that the boundary's shape is fixed once (the "nothing crosses the boundary but plain data" test depends on it).
3. **Milestone 7's core is missing several obligations the Worlds chapter states and the Decisions fix:** `World.Tags` as a core-provided state element; a failed turn dropping responses and re-queuing pushes; pushes for values nobody asked about recorded, not delivered; a `Defer` reborn by a `dyn` at the same tree position counting as the same `Defer`; `Fs.Watch`; `Process.Spawn`/`Process.Child(id)` with cancel-on-teardown; the rule that two `Process.Exit` firings in one turn resolve in occurrence-then-list order; missed `Time.Every` ticks skipped. Each needs a test name in the core's list or the CLI goldens. Chapter 12 has the material.
4. **Milestone 9 does not say how rollback works under Spider.** The Decision is replay on a fresh local timeline; a model reading this milestone will try to undo Spider's holds. Add a step, a test (`an_aborted_occurrence_under_spider_leaves_the_graph_as_it_was`), and the note that quarantine scripts are Rust-only and excluded from the differential diff.
5. **Milestone 4 does not mention the `for` can-fire-at-birth flag** that the elaborator must record and the text form print (L254–256). Without it the Haskell evaluator (milestone 9) has nothing to key its one `coincidence` on, and milestone 8's dump diff would miss it. Add it to step 4 and the printer, and a golden showing a `for` over a widget parameter flagged conservatively.
6. **Milestone 4's dump (L265–272) duplicates chapter 11's and disagrees with it.** Replace it with a pointer to chapter 11's golden plus the dump copied verbatim, so there is one printed form in the book. The same applies to milestone 2's transcript against chapter 9's (which uses `click` and the same paths): one of them should be the master.
7. **Milestone 2 says nothing about the canonical node structure** that a hand-written and a compiled widget share (the Decision says chapter 10 states it). L126 asks for the counter "to look like the compiler's output would" without saying what that is; point at chapter 10's rule (which binds take a position, which make nodes without segments) so the fixture is right the first time, since L328 admits the fixture is at fault as often as the compiler.
8. **Milestone 3's keyed widget projects a per-key dynamic "with the change event dropped when the value is unchanged" (L195).** Holds and derived dynamics never dedupe equal values; this is the one place the plan asks for deduping inside a derived dynamic. Either state that the projection is `$Distinct` applied by the compiler (and so needs equality on `V`), or drop the dedupe and let a neighbour's change reach the child as an equal-valued change (then the test on L221 is wrong). Decide and say which.
9. **Invariants without an enforcer.** L173 "nothing in the runtime reads the list during an occurrence except to build the next one" (the diode), L238 "if the runtime ends up with a `Defer`-specific branch, the diode has been broken", L324 "nodes own their closures and closures capture through the owning node's environment" — each is stated, but the chapter's only enforcement is a grep list at L619–631. Say per invariant who is bound (runtime, compiler, core) and which test fails when it is broken (the leak test for 324 is there; 173 and 238 have none; `one_run_call_is_one_occurrence` does not detect a `Defer` branch).
10. **"Working with a model" (L593–644) reads as process and would survive; the per-milestone grep lists (L619–631) are the part that will go stale.** Consider moving each grep line into the milestone's Risks so that the milestone brief a model receives verbatim (L599) includes its own forbidden list, and keep the section's prose about file plans, failing tests first, and the ambiguity list.
