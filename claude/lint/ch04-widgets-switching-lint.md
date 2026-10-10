# Lint: ch04-widgets-switching.md

Line numbers refer to `/home/claude/book/ch/ch04-widgets-switching.md` as read (834 lines). Runtime names (`WidgetBuilder`, `build_root`, `bind`, `add_cyclic_event`, `close_cyclic_event_by_name`/`_by_index`, `add_cyclic_dynamic`, `fold_dyn`, `ForwardEvent`, `cyclic_events`, `backend_data`, `DynamicWidget`, `DynamicWidgetEvent`, `DWAction`, `already_fired`, `prepare_destruction`, `switch_hold`, `usize::MAX`, `max(height, current + 1)`, the leftmost-of-two result) and all five test names were checked against `prism_runtime/src` and match. `networkView`, `dyn = networkView`, `coincidence`, `listWithKey`, `simpleList` are in `/tmp/claude-0/rx/`. `ix`/`jx`/`kx` and `${expr}` interpolation are in spec-v1.

## Part 1: rule violations

### Rule 1 — history / superseded designs (2)

- L57: "The current code also holds public events and dynamics, triggers, a JSON `backend_data` field and a delegate; those are the backend channel the run call replaces, and none of them survive into the target."
  Rule 1 ("replaces the"; current-vs-old framing). Fix: "The target node has no backend channel: no public event table, no `backend_data`, no delegate. The run call is the only boundary." (The BIBLE permits current-runtime facts, so the `Arc`/`Rc` and child-index-0 remarks at L59, L312, L370 are left alone; this one argues by what is being replaced.)
- L316: "The current code's `prepare_destruction` call on the old node notifies the backend delegate; that is the backend channel, and in the target there is nothing to notify, because nothing outside the program is told that a node is going except by the list it reads afterwards."
  Same. Fix: "Nothing is notified when a node goes: the only way anything outside the program learns of it is the list it reads afterwards, so the drop action has no callback."

### Rule 2 — status tracking (0)

### Rule 3 — quoting / attributing Jimmy (0)

### Rule 4 — banned words (0)

(`no longer` at L7 is physical; "the list of what a widget does is the list…" at L9 is a copula, not a cleft.)

### Rule 5 — clefts and non-default word order (3)

- L128: "Where this matters is the thing people expect `let` to do and it does not."
  Pseudo-cleft. Fix: "This matters because people expect `let` to do something it does not."
- L162: "What a widget function cannot do is bind the same argument twice and have it be one thing."
  Pseudo-cleft. Fix: "A widget function cannot bind the same argument twice and get one thing."
- L237: "What a rebuild does *not* do is cut the old node off early."
  Pseudo-cleft. Fix: "A rebuild does not cut the old node off early."

### Rule 6 — lecturing tone / roadmap (2)

- L3: "The chapter defines widgets and their blocks, then the three constructs that let a program change its own structure while it runs … It ends with five patterns worked occurrence by occurrence …"
  Chapter roadmap ("In this section we will"). Fix: cut the two roadmap sentences; keep the first sentence and the last ("The underlying claim is that one ownership tree … replaces reconciliation, lifecycle and garbage collection at once").
- L563: "… and the point of writing the five out is to make that reading reflexive."
  Tells the reader what the exercise is for. Fix: end the sentence at "what the `dyn`s and `for`s are over."

### Rule 7 — dry definition before motivation (3)

- L337: "`!Switch: fn !!A -> !A`. At occurrence n it delivers the firing, if any, of the inner event that the outer event most recently delivered *before* n."
  The subsection opens on the signature and the timing rule. Fix: lead with the clicker's problem (the parent has `!!clicks`, one click event per generation, and wants to count clicks across all of them), then the signature.
- L347: "`$Join: fn $$A -> $A`. At occurrence n its value before n is the value before n of the inner dynamic current before n."
  Same. Fix: lead with the `Fs.Contents` example that follows (a file viewer whose path changes and whose contents should be one dynamic), then the signature.
- L431–L438: "A `for` has one result, not a map of results, and what it is depends on what the block returns:" followed by the table.
  Table before any example. Fix: open with the `!rows` example at L451–L458 (two rows clicked in one occurrence, one firing with `{3: Toggle, 9: Delete}`), then the table.

### Rule 8 — presence or drop as a trigger (0)

### Rule 9 — conflicts with BIBLE Decisions (7)

- L69: "`!click <- @Element Gui.Button(label=$_)`"
  `Gui.*` is the five-tag primitive vocabulary; buttons are `Ui.*`. Corrected: "`!click <- @Element Ui.Button(label=$_)`".
- L166: "They are state elements, `Cli.Args` and `Cli.Env`, with a value in the occurrence they are created …"
  Corrected: "They are state elements, `Process.Args` and `Process.Env`, …".
- L244: "`@Element Gui.Button(label=_)`"
  Corrected: "`@Element Ui.Button(label=_)`". (Also L253–L255 table cells `Button("Clicked 0 times")` are fine as transcript shorthand.)
- L374: "`@Element Gui.Label(text=$t)` puts a dynamic where a plain field goes."
  Corrected: "`@Element Ui.Label(text=$t)`".
- L563: "`Gui.Button(label)` responds with `click`, `Gui.Label(text)` is a draw command, …"
  Corrected: "`Ui.Button(label)` responds with `click`, `Ui.Label(text)` is a library tag the GUI library turns into `Draw.Text` in a `Gui.Scene`, …". A draw command is a `Draw.*` value, not a tag.
- L401: "For a plain result, the `dyn`'s event is an `!A` and binding it as a plain value is not possible: a plain result of a rebuilt widget is a value per generation, which is an event, and the program writes the `dyn` and takes the event."
  Decision: "binding a widget application whose result is plain gives a `$` holding the first build's value." Corrected: "For a plain result, binding gives a dynamic: `$x <- @W $d` is the `dyn`'s `!A` held from the first build's value, so `$x` is the current generation's result." (And L144's "a plain value" in the list of result kinds is then fine.)
- L371: "The one correctness condition the harness tests is the case where the new inner changes in the same occurrence as the swap …"
  Decision: `Dynamic::join` has three law tests (the third: the outer switches to an inner that does not change). Corrected: "The harness tests three cases: the inner changes alone; the outer swaps to an inner that changed in the same occurrence, where the change event must deliver the new inner's *new* value and the behavior must agree afterwards; and the outer swaps to an inner that does not change."

### Rule 10 — unverified specifics (2)

- L343: "Reflex has `coincidence` for exactly this and the warning in its documentation is the same as the warning in this paragraph."
  The research notes record the doc text as "You maybe looking for `switchHold never` instead", which is a redirect, not a height-cycle warning. Fix: "Reflex has `coincidence` for exactly this, and its documentation points you at `switchHold never` instead." (verified against research-notes.md L159) or cut the second clause.
- L545: "This is also Reflex's experience: `listWithKey` over a `Map` is the one everyone ends up using, and `simpleList` over a list is the one everyone regrets."
  The names are in `Basic.hs`; "everyone regrets" is not in any source. Fix: verify (a Reflex doc or issue) or mark from memory or cut to "Reflex offers both: `listWithKey` over a `Map` and `simpleList` over a list."

### Rule 11 — code that cannot be Prism (5)

- L199: "`!click <- @Button _`" inside `dyn $_:` where `@Button: fn $Text -> @!Click` (L68).
  `_` is plain `Text`; the parameter is `$Text`. Fix: "`!click <- @Button $ _`" (constant dynamic), or change L68 to take `Text` and let the library's `@Button` be the one that writes the `dyn`.
- L376: "`!x <- @Button $label` where `@Button` takes a plain label means `dyn $label:` around the build …"
  Contradicts L68's signature in the same chapter. Fix: either rename the hypothetical ("where `@PlainButton: fn Text -> @!Click`") or make L68 `fn Text` throughout and say the library's `@Button` is that function.
- L465: "`$v <- @TextInput initial`" and L762/L767: "`$name <- @TextInput answers.name`", "`$address <- @TextInput answers.address`".
  `@TextInput: fn Text -> @TextInput` returns the bundle `record: !changed: Text; $value: Text` (L139). Fix: "`TextInput(ignore, $v) <- @TextInput initial`" (and likewise `$name`, `$address`), or declare the result as `@$Text` once at L139.
- L397: "`$$s <- first <= !$ss             # first: the first build's result, which the build has in hand`"
  `first` is not a name any program can write. Fix: say the expansion is the compiler's and spell it with the runtime's `hold_first` ("the compiler emits `hold_first` over `!$ss`, which a program cannot spell"), or write it as the prelude form `$$s <- $Join (HoldFirst !$ss)` if that function is meant to exist.
- L758: "`let $page = ($step, $answers)`" then L759 "`dyn (step, answers) <- $page:`".
  Functor values nest inside plain aggregates, so `($step, $answers)` is a plain tuple holding two dynamics, not a `$(Step, Answers)`; `dyn` needs the latter. Fix: write the lift explicitly, "`let $page = Pair $step, $answers`" (a plain function lifted over two dynamics), or state once that tuple construction lifts like application. L783 ("lifted into one dynamic") then matches whichever is chosen.

### Rule 12 — filler (5)

- L37: "That last part is deliberate and worth saying plainly."
  Fix: cut; start at "A node is not a value."
- L405: "The chapter uses the shorthand freely from here on, as the worked examples do, and writes the expansion out only when the generations are the point of a walkthrough."
  Scaffolding. Fix: cut, or reduce to "Walkthroughs write the expansion out only when the generations are the point."
- L427: "… the chapter returns to this under arrays."
  Forward-reference scaffolding. Fix: cut the clause; the arrays section stands on its own.
- L520: "… and the chapter says so as loudly as the specification does: …"
  Self-reference. Fix: "It is allowed, and it is the wrong tool for anything with identity: an insertion at the front …"
- L723: "That is correct and it is wasteful, and both halves of that sentence matter."
  Fix: "That is correct and it is wasteful."

## Part 2: structural notes

1. **Runtime subsections duplicate the runtime chapter.** "What a node is, in the runtime" (L55–L59), "What the runtime does for `dyn`" (L310–L316), "In the runtime" under `!Switch`/`$Join` (L368–L370) and "Nested `for`, and the runtime" (L551) restate the `WidgetBuilder`/`DynamicWidget`/`switch_hold`/keyed-list design that the Rust runtime chapter owns. Keep the one-paragraph ownership argument (L59, three rules, no collector) here and reduce the rest to the invariant each enforces, with the mechanism in the runtime chapter.
2. **Failed builds are never mentioned.** The Decisions say a failed `dyn` build does not fire the `dyn`'s event and is reported through `Errors` (kind `Build`), and a cycle found at `!Switch` adoption aborts the switching occurrence. The chapter's "What a rebuild resets" and the invariants list should say what happens to the old generation when the new one fails to build (does the old one stay?), who reports it, and with which `ErrorKind`. Same for a `for` key whose block fails.
3. **"Every statement form" is missing forms.** `intercept` as a bound statement, `@Element` as a statement (it is mentioned at L86 in passing), and `match` with widget arms (allowed only inside `dyn`/`for` blocks per L407) are absent from a list that promises "every statement form". The reader coming back in months will treat this list as complete.
4. **Consolidation of `@A` results is one table row.** L438 "by the inner kind" needs an example: a `for` block whose last statement is a bound widget (`@Row $_`) with a bundle result, showing the field-by-field map. Also say what a block returning `unit` consolidates to (`$(K ==> unit)`, presumably, and whether the compiler elides it).
5. **The invariant list leaves out two that the chapter relies on.** "A path is never reused" (L231) and "a `for` key that returns is a new key" (L534) are stated in prose but not in the invariants section, with no party bound (runtime: generation counters are monotone per `dyn` node; program: id allocation) and no mechanism named. Add them, with the counter location and whether it survives a quarantine.
6. **Helpers used in patterns have no signatures.** `Styled`, `Describe`, `ReKey`, `StepAll`, `Gone`, `LastKey`, `IsConfirmed`, `Insert`, `NewItem`, `Keys` appear without types. L563 promises "a few plain helpers are assumed and named where they are used"; give each a one-line `fn` signature at first use so the examples could compile.
7. **The tabs "third version" (L663) is prose only.** The interceptor that hides inactive panes is the one construction in the chapter with no code. Either write the ten-line interceptor (capture the list, filter `Ui.*` slots by a `$tab`-lifted predicate, forward the rest) or name the interception chapter's example and say it is the same.
8. **Array `for` paths are unspecified.** L520 says arrays are index-keyed; nothing says how an index key prints (`[3]`? `3`?) or how the transcript distinguishes a key `3` of a `Nat ==> V` map from index 3 of an array. One sentence and one path.
9. **The cache idiom contradicts the worked-examples chapter by implication.** L360 says the worked examples' cache "holds plain values and has a one-occurrence hiccup". The BIBLE's clarification makes the hold-of-dynamics-plus-`$Join` the canonical idiom; if the worked examples chapter still has the plain-value version, one of the two needs to change, and this chapter should not describe a sibling chapter's example as deficient.
10. **`@started` definability.** L173–L177 define `@started` as `dyn $ unit: @ unit` and L179 says the runtime "implements it as a filter over the builder's done event". Say which the conformance suite checks (the law, not the implementation) and that the two must agree on the one observable: the firing lands in the build occurrence after the block is built, at `height + 1`, so a hold in the same block driven by it changes in that occurrence.

## Counts

| Rule | Count |
| --- | --- |
| 1 History | 2 |
| 2 Status | 0 |
| 3 Attribution | 0 |
| 4 Banned words | 0 |
| 5 Clefts / word order | 3 |
| 6 Lecturing | 2 |
| 7 Definition before motivation | 3 |
| 8 Presence/drop trigger | 0 |
| 9 BIBLE conflicts | 7 |
| 10 Unverified specifics | 2 |
| 11 Non-Prism code | 5 |
| 12 Filler | 5 |
| **Total** | **29** |
