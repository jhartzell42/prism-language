# Lint: ch01-worked-examples.md

Line numbers refer to `/home/claude/book/ch/ch01-worked-examples.md` as linted.

## Part 1: rule violations

### Rule 1 — history / superseded designs (1)

- L670: "A cache that wanted to avoid even that would hold the live dynamic itself and `$Join` it, choosing between the live dynamic and a frozen one; this version is kept because it shows the hold around the forwarded query in the plainest way."
  Rule 1 (and 9: the BIBLE's canonical cache idiom is hold-of-dynamics plus `$Join`, with no hiccup). "This version is kept" argues from an alternative that is in fact the binding design. Fix: rewrite example 9's supplier as the `$Join` form and drop the hiccup paragraph; if the plain-hold form is shown at all, show it as the hypothetical ("suppose the cache held the value instead of the dynamic; then for one tag read…").

### Rule 2 — status tracking (0)

None.

### Rule 3 — quoting or attributing Jimmy (0)

None.

### Rule 4 — banned words (0)

None found (`carry`, `shape`, `compose`, `genuinely`, `honestly`, `straightforward`, `leverage` all absent).

### Rule 5 — clefts and non-default word order (6)

- L297: "What makes the cycle legal is `!Switch`, which chooses its inner event from before the occurrence, and the tag on `$count` in `$count + 1`."
  Fix: "`!Switch` makes the cycle legal: it chooses its inner event from before the occurrence, and the tag on `$count` in `$count + 1` does the rest."
- L635: "What the language fixes is only that leaves are elements, containment is interception, and routing is against the opened list."
  Fix: "The language fixes only three things: leaves are elements, containment is interception, and routing is against the opened list."
- L842: "What is specific to the CLI World is standard input, standard output and exit, and those are three tags."
  Fix: "The CLI World adds only standard input, standard output and exit, and those are three tags." (Exit is not CLI-specific under the Decisions anyway; see rule 9.)
- L1107: "What the language supplies is that an interceptor which re-emits everything and supplies by forwarding is observably not there, so each layer can be put in and taken out without the children knowing, and that routing is always against the opened list, so no layer can send an input to something the user has not seen."
  Fix: "The language supplies two things: an interceptor that re-emits everything and supplies by forwarding is observably not there, so …; and routing is always against the opened list, so …"
- L840: "The reason arguments are not parameters of `@main` is the sandbox of example 7: …"
  Fix: "Arguments are not parameters of `@main` because of the sandbox of example 7: …"
- L476: "the point of leaving it is that correctness does not depend on it."
  Fix: "it stays because correctness does not depend on it."

### Rule 6 — lecturing tone (2)

- L122: "Three things to notice in a program that looks too small to have three things in it."
  An announced list of points for the reader. Fix: cut the announcement; start with "The tag law decides that…" and let the three sentences stand.
- L1003: "`Requested Time.Every(..), ignore` is not a very good step, since `Time.Every` requests nothing; it is here to show the form, and a World that saw it would report that the slot has no request to match, which is a failing step, not a passing one. Delete it before running the test."
  Instructing the reader to repair the chapter's own example. Fix: replace the step with `Unrequested Time.Every(..)` (a suite step per the Decisions) and delete the sentence.

### Rule 7 — dry definition before motivation (1)

- L7: "Occurrences are numbered from zero. Occurrence 0 is the build of the program; nothing is delivered to it. From then on every occurrence is opened by the World with a set of inputs…"
  The conventions section opens with the numbering and path grammar before saying why a reader needs them. Fix: one sentence first, e.g. "Every walkthrough below answers the same question: two clicks landed in one frame, a label rebuilt in the same moment, a response arrived late — what did the program do? To answer it exactly the chapter needs a way to say *when* and *where*: when is an occurrence number, where is a path." Then the definitions.

### Rule 8 — presence or drop as a trigger (3)

- L459: "`Time.Every(period=100)` is bound as a statement and its tick event is discarded; the element exists to make the World come back every hundred milliseconds, and each time it comes back it opens an occurrence and pushes the clock."
  The element's presence described as what makes the World act. Fix: "`Time.Every` is a stream element like `Cli.Stdin`: the World delivers a `Tick` to the slot every hundred milliseconds, and each delivery is an occurrence in which it also pushes the clock."
- L738: "The World reads the list, sees a socket element for an address it has no connection to, and opens one."
  Fix: state the obligation, not the trigger: "`Net.Socket` has state, and the World must supply a true `Link` for every slot that asks; supplying `Connected` means having connected, so the World connects." (The `Received`/`Closed` stream follows from the same obligation.)
- L909: "The World binds port 8080 because a listener element exists in the list, and starts accepting."
  `Net.Listener` has only `responds`, so this is an effect on presence with no request and no state to supply. Fix: either give `Net.Listener` `state: Listening` (then binding is how the World supplies the state, as with `Net.Socket`) or make listening a request (`requests: !unit`, fired from `@started`), and say which.

### Rule 9 — conflicts with the BIBLE Decisions (16)

- L46: "`Element` is the extensible variant every element is a value of; the tags under `Gui` are the ones a World that knows about screens is assumed to accept, and later examples add tags as they need them, each declared before its first use."
  Decision: `Gui.*` is the primitive vocabulary (`Gui.Window`, `Gui.Scene`, `Gui.Focusable`, `Gui.Metrics`, `Gui.Frames`); buttons, labels, inputs and boxes are the library's `Ui.*` tags, which reach a World only if it accepts them (the Test World does). Fix: declare `Ui.Label`, `Ui.Button`, `Ui.VBox`; say they are library tags the Test World accepts, and that a GUI World sees them only through the library's interceptors as `Gui.Scene` draw lists.
- L53–59 and throughout (every `Gui.Label`, `Gui.Button`, `Gui.VBox` in code, prose and element lists): "tag Gui.Label of Element: … tag Gui.Button of Element: …"
  Fix: `Ui.Label`, `Ui.Button`, `Ui.VBox` everywhere (`Ui.Button` responds `Click` is the sanctioned Test World spelling). `Gui.Focusable` stays `Gui.*`.
- L51: "type Click = choice Click" and L416: "type Tick = choice Tick"
  Decision: unit-like responses are unit types with lowercase values: `type Click = click`, `type Tick = tick`. Fix the declarations and every `Send … , Click` / `Tick` / `{…: Click}` to `click` / `tick`.
- L133–135: "tag Gui.Focusable of Element: name: Text; responds: Key"
  Decision: `Gui.Focusable` is name-only; key events are delivered by `Gui.Window` and routed by the library's focus interceptor. Fix: either keep example 2 under the Test World with a library tag (`Ui.TextInput` responding `Key`, or a `Ui.Focus(name)` the library interceptor targets) and say that under a GUI World the library turns `Gui.Window` key events into these; or keep `Gui.Focusable` presence-only and show keys arriving through a `Gui.Window` slot. Either way the sentence "responds with keys" at L166 changes with it.
- L775–777 and L792: "tag Cli.Exit of Element: requests: !Code" / "@Element Cli.Exit(requests=(Code 0 on !end))"
  Fix: `Process.Exit`. Same at L798 ("a `Cli.Exit` element").
- L772–773: "tag Cli.Stdout of Element: requests: !Text"
  Decision: `Cli.Stdout`/`Cli.Stderr` are `requests: !Text, responds: WriteFailed`. Fix: add `responds: WriteFailed` and bind the response (`!werr <- @Element Cli.Stdout(...)`) or bind it to `ignore`.
- L821–825 and L840: "tag Cli.Args of Element: state: [Text]" / "`State Cli.Args, ["notes.txt"]`" / "A `Cli.Env` is the same again."
  Fix: `Process.Args`, `Process.Env`. And L842's "standard input, standard output and exit … three tags" becomes stdin, stdout, stderr and signals; exit is a process tag.
- L280: "the consolidated event of a `for` merges the blocks that existed as the occurrence opened, the way `!Switch` picks its inner event: a block born in an occurrence has its own events heard from the next one." and L901: "It passes through the tag on `$conns` in `ApplyDeltas` and through the delayed membership of the `for`'s merge".
  Decision: `for` consolidation is prompt; a block born in n contributes what it fires in n (only birth firings such as `@started`), and the compiler checks that a row's result does not fire at birth. Fix: "`for` consolidation is prompt, so the cycle is legal only because nothing a new row fires at birth reaches its result: its events are clicks (responses), and a response never arrives in the slot's creation occurrence. The compiler sees that; `@ Delete on !started` as a row result would be rejected." Same correction at L901.
- L505–506 and L521–523: "!resp <- @Element _.element / @ resp" (catch-all arms)
  Decision: catch-all arms write `(!resp, ignore) <- @Element _.element`, since the generic `@Element` returns `@(!Response, $State)`. Fix in `@Forward` and in the sandbox's `ignore:` arm.
- L528, L594, L650, L1048–1049: "ignore: @Element e" / "$ \|e| @Element e" (suppliers)
  The supplier must return a widget whose result is the state dynamic; the generic `@Element e` returns a pair. Fix: `\|e| @widget: (ignore, $s) <- @Element e; @ $s` (or define one `@Supply` in the shared declarations and use it in examples 7, 8, 9 and 14).
- L9 and L550, L1081: "A slot created by a widget that an interceptor's supplier built is written `s`" / "s: Fs.Contents(path=…)" / "s: Clock"
  Decision: supplier-built slots print `s/n/...`, the supplied widget's own segments after the prefix. Fix: `s/0/0` (and say what `n` counts in the conventions).
- L9 and L382–383, L728–731: "Holds and `let`s do not take a bind position; only widgets do, and `@started` is a widget, so it takes one" / "3/0: Gui.Focusable(name="input")" (the slot inside `@TextInput`)
  Decision: only slots, `dyn` generations, `for` keys and intercept children take segments; a plain widget-function application makes a node but adds no segment. The chapter gives the `@TextInput` application a segment (`3/…`) and `@started` one. Reconcile: either the conventions say every bind that builds a node takes a segment (and the Decision's wording is about not adding a *second* segment for the application around a `dyn`), or the paths change. State the rule once in the conventions in the Decision's words.
- L572–573 and L635, L1107: "tag Gui.VBox of Element: children: Path ==> Element; responds: Pointer" / "`Send Gui.VBox(..), Pointer(row=0)`"
  Decision: the suite's VBox responds `ChildInput(path, response)`, so a script can reach a nested button with `Send Ui.Button(label="Increment"), Click`. Fix: `Ui.VBox` responds `ChildInput(path, response)` and the `HitTest`-by-row routing moves to the GUI-World path (a library interceptor over `Gui.Window` pointer events and a `Gui.Scene`), mentioned in a sentence; examples 8 and 14's walkthroughs then route `ChildInput` by path, which is shorter.
- L1017–1018: "Requested Defer, unit / Push Clock, Instant 0"
  Decision: `State` for any state tag the program creates at build must precede the build, and pushes for values nobody asked about are recorded, not delivered. `@probe` creates no `Clock`, so the `Push` is a no-op turn, and the chapter then deletes it (L1024). Fix: drop the `Push` from the script as printed and show the corrected script, not the wrong one followed by a correction.
- L414, L767, L847: "type Instant = Instant Int" / "type Code = Code Int" / "type Accepted = Accepted Nat" with `.value` access (L437, L868)
  Single-constructor types with a `.value` accessor are not in the BIBLE's syntax summary; `Instant` already exists (the `Gui.Frames` response uses it). Fix: verify against spec-v1 Part II; otherwise `type Instant = record: ms: Int` or a `choice` with a `match`.
- L1003: "`Requested Time.Every(..), ignore` is not a very good step"
  Decision: `Unrequested pattern` exists. Fix as under rule 6.

### Rule 10 — unverified specifics (2)

- L833: "@ !never"
  `!never` is not in the chapter's prelude (L15–40) or research-notes. Fix: add `!never: !a` to the prelude or write the arm with a filter that never passes.
- L890: "Net.Conn(id=key, requests=!reply)"
  `key` as the name of the current key inside a `for` block is not established; the chapter uses `$_` for the value (L231) and never introduces `key`. Fix: verify the spelling against spec-v1's `for` section and say it in the conventions, or add a `$_`-style convention for the key.

### Rule 11 — code that cannot be Prism (4)

- L51, L416: "type Click = choice Click" / "type Tick = choice Tick" — fix as rule 9: `type Click = click`.
- L775: "tag Cli.Exit of Element" — fix: `Process.Exit`.
- L64 and L69: "@Label: fn $Text -> @unit = \@widget:" / "@Button: fn $Text -> @!Click = \@widget:"
  `\@widget:` is a bare lambda whose body is a widget block and whose argument is `_`, then read as `$_` inside the `dyn` — plausible under the bare-`\` rule but unlike every other function in the chapter. Fix: `\|$text| @widget: dyn $text: …` for both, which also removes the `$_`/`_` double meaning at L75.
- L528 etc.: supplier lambdas returning a pair where `@$State` is required — see rule 9.

### Rule 12 — filler (6)

- L193: "Focus is not in this program either, and it is worth saying where it would go." Fix: "Focus is not in this program either. It would go in an interceptor: …"
- L241: "The events deserve a slow read." Fix: cut; begin "Each row's block returns an event of `Change`…"
- L323: "Worth noticing what `!Switch` not being prompt costs here: nothing." Fix: "`!Switch` not being prompt costs nothing here."
- L670: "There is one hiccup worth seeing." Fix: cut with the paragraph (rule 1/9).
- L816: "The tag law is doing something here that deserves a second look." Fix: cut; begin "`$lines on !end` is the count *before* the occurrence…"
- L1024: "That is the point of the script as written: a reader who expects `"(reading...)"` has the turn wrong." Together with L1003's "Delete it before running the test", the chapter prints wrong scripts and then corrects them. Fix: print the right script and give the wrong expectation as a one-sentence hypothetical.

**Counts:** rule 1: 1; rule 2: 0; rule 3: 0; rule 4: 0; rule 5: 6; rule 6: 2; rule 7: 1; rule 8: 3; rule 9: 16; rule 10: 2; rule 11: 4; rule 12: 6. Total 41.

## Part 2: structural notes

1. **The GUI vocabulary needs one paragraph that matches the Decisions.** The "Shared declarations" section (L44–75) says the `Gui` tags are what a screen World accepts. The book's design is two layers: `Gui.*` primitives (`Window`, `Scene` with `Draw.*` values, `Focusable`, `Metrics`, `Frames`) that every GUI World speaks, and `Ui.*` library tags that the Test World accepts directly and a GUI World sees only after the library's interceptors have turned them into a scene. Every example in the chapter runs under the Test World, so the chapter needs only the `Ui.*` layer, but it should say in two sentences that the other layer exists and that examples 8 and 14 are the library's side of it. Without that, a reader who reaches chapter 6 will think the chapter and the GUI chapter disagree.

2. **Example 2 and focus.** With `Gui.Focusable` name-only and keys arriving through `Gui.Window`, the text-input example needs a stated delivery path for keystrokes under the Test World. The paragraph at L193 already sketches the focus interceptor; make it the actual route (or a `Ui.TextInput` library tag) rather than a hypothetical, and show one occurrence of a keystroke reaching the input through it. This is the example a reader will copy for every form.

3. **Examples 8 and 14 duplicate hit-testing against the suite's `ChildInput` route.** If `Ui.VBox` responds `ChildInput(path, response)` under the Test World, the `HitTest`-by-row machinery is GUI-World-only and belongs in the GUI chapter's layout section; here the VBox becomes a path-forwarder, and the shell's "click lands on the app the user was looking at" point survives unchanged (route against `$shown` as opened). Cut `HitTest`, `Pointer`, `GuiOnly`/`NonGui` to what the Test World needs, and name the GUI chapter for the rest.

4. **The `for` cycle argument is stated wrongly in two places (L280, L901) and the correct argument is more useful.** Under prompt consolidation the todo list and the server are legal because a row's result is built from responses only, and the compiler can check that. One short paragraph in example 3 should say who is bound (the program: a row's result may not fire at birth; the compiler enforces it statically) and show the rejected form `@ Delete on !started` so the reader knows what the check looks like. Example 12 then just points back.

5. **Example 9 should be the canonical cache idiom.** The hold-of-dynamics-plus-`$Join` form is the one the book names as canonical, and the one-occurrence hiccup disappears with it. Rewrite the supplier that way; the walkthrough's occurrence 3 (push and click together) still shows the tag law.

6. **Invariants stated without a binder or an enforcer.** L175–176 ("a dynamic's change event is allowed to fire with the value it already had") — say this is the no-dedupe law, the runtime guarantees it, and `$Distinct` is the opt-out. L317 ("sending to a path not in the opened list is an error") — say the runtime raises `UnknownPath` and the World is the party bound. L816 ("a slot receives at most one response per occurrence") — say the World is bound and the runtime reports a duplicate as a `Component` error. L407 ("that is the equality law across the boundary") — name the World's obligation (consistent oracle) that makes it hold.

7. **Example 13's test scripts are printed wrong and then corrected in prose (L1003, L1022–1024).** A reference reader will copy the script block. Print the correct scripts; put the instructive mistake (expecting `"(reading...)"` after turn 0) in one sentence as something a reader might expect. Add an `Unrequested` and an `ExpectTopNone` step somewhere so all the suite steps the chapter uses are shown once.

8. **Path conventions (L9) need to match the Decision's rule and the `s/n` spelling, and need one worked derivation.** Derive `1/[3/1/#0/0]/#0/0` from example 7 segment by segment in the conventions section rather than in the middle of the sandbox walkthrough (L553), and state the rule for supplier-built slots with an `n`.

9. **Missing example: a program that exits.** The chapter has `Process.Exit` only inside `wc`. Since "exit is a request, not a presence" is one of the few places the design is counter-intuitive, example 11 could add two sentences and a list line showing what the World sees after the exit fires and that the slot is still in the list.

10. **Thin spots for a reader returning in months.** Example 10 never shows the list after occurrence 4 beyond one line (L749); print the full list, since the point is that the old socket is gone and the text inputs are unchanged. Example 12's buffered variant (L933–940) introduces `CompleteHead` in prose only; declare it in the code block. Example 6 should say what `Time.Every`'s tick carries if anything and that missed ticks are skipped (a Decision the stopwatch is the natural place to show).
