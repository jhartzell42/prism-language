# Lint: ch03-events-dynamics-time.md

Chapter: /home/claude/book/ch/ch03-events-dynamics-time.md (928 lines). Line numbers refer to that file.

## Part 1: rule violations

### Rule 1 — history / superseded designs

None found. The Reflex comparisons are prior art, not superseded Prism designs, and every alternative is argued as a hypothetical.

### Rule 2 — status tracking

None found. "settled" at lines 309 and 313 is the propagation sense (a node's sources have fired), not a status word.

### Rule 3 — quoting or attributing Jimmy

None found.

### Rule 4 — banned words

None found (grep for carry/shape/compose/genuinely/honestly/straightforward/leverage over the whole file is empty).

### Rule 5 — clefts and non-default word order

1. Line 37: "What makes this liveable is that simultaneity is rare and, when it happens, the program has already said what it wants." — pseudo-cleft. Fix: "This is liveable because simultaneity is rare, and when it happens the program has already said what it wants."
2. Line 45: "What you wanted from `!always` is always available some other way." — pseudo-cleft. Fix: "Everything you wanted `!always` for is available some other way."
3. Line 117: "The thing that got swallowed is whatever the new block fired at n before the switch had adopted it." — pseudo-cleft. Fix: "The switch swallows whatever the new block fired at n before it was adopted."
4. Line 241: "What it buys the program is that every read of state is visible as such." — pseudo-cleft. Fix: "The program gains this: every read of state is visible as such."
5. Line 243: "What it buys the implementation is freedom of order." — pseudo-cleft. Fix: "The implementation gains freedom of order."
6. Line 273: "That holds commit at the end is what makes the rest possible." — cleft. Fix: "Holds committing at the end makes the rest possible."
7. Line 295: "The reason the distinction matters is that only prompt edges constrain the order of propagation within an occurrence, and only prompt edges can form a cycle that means anything." — pseudo-cleft. Fix: "The distinction matters because only prompt edges constrain propagation order within an occurrence, and only prompt edges can form a cycle that means anything."
8. Line 313: "What the specification requires is the observable consequences, and only those: …" — pseudo-cleft. Fix: "The specification requires only the observable consequences: …"
9. Line 414: "What is left is a function from inputs to outputs, and functions are testable." — pseudo-cleft. Fix: "A function from inputs to outputs is left, and functions are testable."
10. Line 779: "What remains observable is exactly what the laws define: before-values, firings at this occurrence, and the values pushed in by the World." — pseudo-cleft. Fix: "Only what the laws define remains observable: before-values, firings at this occurrence, and the values the World pushed in."
11. Line 913: "The map is the collection because the key is an identity." — non-default word order (the BIBLE's own bad example). Fix: "This program uses a map because the key is an identity."

### Rule 6 — lecturing tone

12. Line 155: "Two things to notice in the definition." — pedagogical pointer. Fix: cut the sentence; the two paragraphs that follow stand on their own.
13. Line 161: "The reason it is right, and not merely consistent, comes in three parts." — announces structure. Fix: cut; the three "The tag law makes …" paragraphs are self-announcing.

### Rule 7 — dry definition before motivation

14. Line 99 (`### !Switch`): opens "`!Switch: fn !!A -> !A`. At occurrence n it delivers the firing, if any, of the inner event …" before the use case at line 101. Fix: lead with the relabelled button whose clicks the parent wants as one `!click`, then give the signature.
15. Line 191 (`### Hold`): opens with the formal statement "`$x <- v <= !e`, allowed only as a statement in a widget block, defines `$x` whose value …". Fix: lead with the `$checked` line at 196 ("There is the whole story of `$checked`") and give the formal statement after it.
16. Line 231 (`### Equality of readings`): opens with the law. Fix: lead with the two-widgets-reading-one-config-file example at 233 (`Fs.Contents` in two widgets that have never met), then state the law.
17. Line 412 (`### Determinism`): opens with the formal property. Fix: lead with the bug report that replays ("the transcript of what the World sent is the program's entire input, and replaying it replays the bug", line 414), then the property.

### Rule 8 — presence or drop as a trigger

None found. Teardown at commit (lines 201, 911) is the ownership-tree obligation, not a trigger.

### Rule 9 — design conflicts with the BIBLE Decisions

18. Lines 436–441 (and every use: 452, 457, 477–483, 498–502, 517, 536–538, 559–560, 580–582, 592, 598–608, 628–636, 652, 656–660, 681–687, 702–705, 848, 871, 879–886 — 30 occurrences in all): `tag Gui.Label of Element:` / `tag Gui.Button of Element:` and `Expect Gui.Label(text=…)`, `Send Gui.Button(label=…), click`. Decision: `Gui.*` is the five-tag primitive vocabulary (`Gui.Window`, `Gui.Scene`, `Gui.Focusable`, `Gui.Metrics`, `Gui.Frames`); the library's tags are `Ui.*`, and "worked examples that show a button under the Test World use `Ui.Button` responding `Click`." Fix: rename to `Ui.Button` / `Ui.Label` throughout the law-test vocabulary and scripts (the Test World accepts every tag, so the scripts are otherwise unchanged); line 430's "a program may write `@Element Gui.Button(label=$x)` directly" becomes `Ui.Button`.
19. Lines 460–462: "`@Updated: fn $a -> @!a = \|$x| @widget:` … `!v <- dyn $x: @ _`". Decision: a dynamic's change event is obtained with the plain prelude function `!Updated: fn $a -> !a` (fires with the new value in every occurrence the dynamic changes; never at build). No widget is needed. Fix: drop the `@Updated` definition and the comment "plus one firing at build".
20. Line 465: "The language gives no direct name to a dynamic's change event, and this is how a program gets one when it needs it … The extra firing at build is not a flaw; it is the `dyn` law, and the scripts below count it." — conflicts with `!Updated` existing in the prelude. Fix: replace the paragraph with one sentence: "`!Updated $d` is `$d`'s change event, a plain prelude function; it never fires at build."
21. Lines 531–538 (`@diamond` / `diamond_changes_once`): `!d_changed <- @Updated $d` and `Expect Gui.Label(text="d=1 fires=1")` … `"d=16 fires=2"`, with line 541 "`fires` is 1 after the build, from the `dyn`'s first firing." Fix: `let !d_changed = !Updated $d`; expectations become `fires=0` then `fires=1`; the explanation becomes "After the click `$fires` is 1: `$d` changed exactly once at that occurrence. An implementation that let `$d` change per input would show `fires=2`."
22. Lines 575–582 (`@switch_law`): `!built <- @Updated $gen` and `Expect … built=0` then `built=1`. Fix: `let !built = !Updated $gen`; expectations become `built=-1` after the build (nothing fired) and `built=1` after the click; line 585's "The parent still knows the block was built: `!built`, the `dyn`'s own event, says so" should name the `dyn`'s own event `!!born` rather than `!built`, since that is the one that fires at build.
23. Line 187: "a derived `dyn` over `$edit.text` will rebuild on a cursor move, and a program that minds can derive from a hold that it feeds only on real changes." Decision: `$Distinct` (prelude, equality-bound) suppresses equal-valued changes. Fix: "… and a program that minds writes `dyn ($Distinct $edit.text):`."
24. Lines 848–850: "`tag Gui.Focusable of Element:` / `name: Text` / `responds: Char`". Decision: `Gui.Focusable` has only a `name` field; keys arrive through `Gui.Window` and the library routes them. Fix: for the debounce example declare a test-only input tag (`tag Keys of Element: responds: Char`) or use `Ui.TextInput` (`responds: Text`) and feed `!typed` from it; update lines 871, 879, 881 to match.
25. Line 722: "`Errors Cycle(..)`". Decision: one tag `Error` with `kind: ErrorKind`, `spans`, `occurrence`; the `Errors` step matches an `Error` value. Fix: `Errors Error(kind=Cycle, ..)`.
26. Lines 120–124: "`!resp <- @Element Fs.File(path=_, requests=(Read on !started))`". The BIBLE's file vocabulary is the state element `Fs.Contents` (and `Fs.Watch`); a request-style `Fs.File` with a `Read` request is not among the adjudicated tags. Fix: either use the state form (`$contents <- @Element Fs.Contents(path=_)` returning a dynamic, with the `dyn` result `$Join`ed) or use chapter 6's request tag by its adjudicated name once chapter 6 fixes it; the point of the example (a block acting on its own birth with `@started`) survives either way.

### Rule 10 — unverified specifics

27. Line 406: "It is also the rule Reflex's Spider implements, under the name `subscribeAndRead`: subscribing to an event returns its value for the current frame if it has one, and a `hold` subscribed mid-frame adopts that value at the frame's end." `subscribeAndRead` is in /tmp/claude-0/rx/Spider.hs (line 276) and the hold-init deferral queue (`SomeHoldInit`, Spider.hs 987–1022) supports the second half, but neither is in research-notes.md. Fix: verify the hold half against Spider.hs or mark "from a reading of Spider.hs" in a footnote.
28. Line 171: "Most of the subtle bugs in Reflex programs live in that choice." — an unverifiable empirical claim. Fix: soften to the verified doc text ("the `tagPromptlyDyn` docs say outright that `tag (current d) e` is preferred when creating cyclic data flows") or cut.
29. Line 249: "`sample`, `tag`, `attach`, `attachWith`, `attachPromptlyDyn`, `tagPromptlyDyn`: the library has two of everything" — all six names exist in the Reflex sources on disk (`attachWith` in Query.hs; the rest in research-notes.md §3), so the names are fine; "two of everything" overstates (`sample` has no prompt twin). Fix: "the library has two of most things".
30. Line 122: `Fs.File` with a `Read` request (see item 26) — a tag name not in research-notes.md or the BIBLE. Fix: verify against chapter 6 or cut.

### Rule 11 — code that cannot be Prism

31. Line 123: "`@ resp`" after "`!resp <- @Element Fs.File(…)`" — the result is an event and must be written with its sigil. Fix: `@ !resp`.
32. Line 835: "`let !When = \|!e, $open| !Filter ((\|x, g| if g: Some x else: Nil) !e, $open)`" — a lambda is a plain value; naming it `!When` puts an event sigil on a function. Fix: `let When = \|!e, $open| …` and line 836 `let !accepted = When !submitted, (Not $busy)`.
33. Line 872: "`let !typed = Concat [$query, ToText !key]`" — an array literal holding a dynamic and an event is a plain array (the chapter says so itself at lines 89 and 830: "a tuple with an event inside it is a plain tuple, not an event"), so `Concat` of it does not lift and `!typed` is not an event. Fix: `let !typed = (\|q, c| Concat [q, ToText c]) $query, !key`.

### Rule 12 — filler

34. Line 25: "and it is worth holding on to against the other sense the word has acquired" — "worth" scaffolding. Fix: "and it is the one to hold on to against the other sense the word has acquired".
35. Line 43: "There is no `!always`, and the reason is worth a paragraph because it is the first place the language says no to something that seems harmless." Fix: "There is no `!always`. It is the first place the language says no to something that seems harmless."
36. Line 344: "`for` deserves a closer look, because the structure that looks like the cycle above is also the structure of every list program." Fix: "The structure that looks like the cycle above is also the structure of every list program, so `for` needs its own rule."
37. Line 396: "Each of the rules an implementation might have adopted instead is worth seeing fail, because the failures are the bugs you would otherwise meet in production." Fix: "Each rule an implementation might have adopted instead fails in a way you would otherwise meet in production."
38. Line 408: "The realisation is simpler than the consequences suggest, and the sketch is here because it is the kind of thing a reader changing the design wants to know is possible." Fix: "The realisation is simpler than the consequences suggest."
39. Line 465: "`@Updated` is worth a sentence." — filler (and the paragraph goes under item 20).
40. Line 541: "which is the point; the count is what makes their absence checkable." — "which is the point" is filler. Fix: "The count is what makes their absence checkable."

## Counts per rule

| Rule | Count |
|---|---|
| 1 History | 0 |
| 2 Status | 0 |
| 3 Attribution | 0 |
| 4 Banned words | 0 |
| 5 Clefts / word order | 11 |
| 6 Lecturing | 2 |
| 7 Definition before motivation | 4 |
| 8 Presence/drop trigger | 0 |
| 9 BIBLE conflicts | 9 (one of them 30 occurrences of `Gui.Button`/`Gui.Label`) |
| 10 Unverified specifics | 4 |
| 11 Non-Prism code | 3 |
| 12 Filler | 7 |
| **Total** | **40** |

## Part 2: structural notes

1. **Lifting over `@` is used before it is introduced.** `!a <- @Button $"A"` (line 471 and throughout) depends on the Clarification that binding a dynamic of widgets lifts to `dyn` with the bound name being the `!Switch` of the `dyn`'s event. The chapter writes `@Button` out longhand (line 455) but never says that `!click <- @Button $label` is the shorthand for it, and line 358 ("`!delete` is a `!Switch` inside `@Button`, which reads before") assumes the reader knows. One paragraph in "`!Switch`" or "Events are values" saying this once would let every later example stand alone.
2. **`$Join` is used in a law test (line 613–639) but defined only by a forward reference** ("the switching chapter", line 261). The join law test needs at least the one-sentence rule here: `$Join $$x` at n is the inner's value after n, including when the outer and the new inner both change at n. Also the Decisions call for a third `Dynamic::join` law test (outer switches to an inner that does not change); the switching chapter owns it, but this chapter's "Laws as tests" should either include it or name the chapter that does.
3. **The construction rule names no enforcer.** "A widget built in occurrence n behaves as though it had been built immediately before n's inputs arrived" (line 368) binds the runtime, and the chapter says "an implementation must honour and a test can check", but it never says which suite scripts are the check (the nested-`dyn` and reset-hold tests at 641–690 are) nor that the Haskell implementation gets it from Spider for free while the Rust runtime must build the catch-up cache described at line 408. One sentence tying law, enforcer, and test together would close it.
4. **Glitch freedom / at-most-once: who is bound.** Lines 214–227 state the law and sketch heights, but do not say that the World is also bound (a World that pushes two values for one state element in one occurrence would make a dynamic change twice) or that the runtime rejects that as a `Component` error. The Equality-of-readings section does say the World is bound; glitch freedom should too.
5. **Simultaneity and the two-events compile error (line 35) has no example of the error text or of the `?`-lifting interaction.** `!a + !b` is refused, but `!?a` and `?`-lifting at the plain layer get no mention anywhere in the chapter even though `!Filter` turns `!?A` into `!A` (line 93). A short passage on how `?` lifts inside an event or dynamic (`$?x` vs `?$x` is at line 261, but the applicative `?` layer is not) belongs either here or in the types chapter with a pointer.
6. **The `for` static cycle rule (lines 344–358) states what the compiler can see but not the conservative case.** Decision: `for` over a widget parameter whose body is not visible is treated as able to fire at birth, and the elaborator records "can fire at birth" on each `for` node. The chapter should say both, since the todo example's legality depends on the row body being visible in the same block.
7. **Duplication with chapter 2 / the widgets chapter.** "Hold" (lines 189–210) explains ownership of holds, `dyn` reset and `for` key lifetime, which is the widgets chapter's material; "Construction in the middle of an occurrence" repeats the path-minting consequence ("paths are off by one", line 402) that the widgets chapter owns. Keep the time-law content here and point to the widgets chapter for ownership and paths.
8. **The negative space overlaps the body.** "No sampling" (737–745) restates lines 235–245 nearly sentence for sentence, and "No prompt switching" (747–755) restates 113–117. Either cut the body versions down to the laws and keep the reasoning in the negative-space section, or make the negative-space entries three lines each with a back-reference.
9. **The debounce idiom (843–889) would benefit from the `Time.Every` contrast** and from saying what the World does with the stale token requests it is "free to fire" (the Decision that missed `Time.Every` ticks are skipped, and that a late result for a gone slot is discarded). As written a reader could think stale `Time.After` tokens are cancelled rather than answered and ignored.
10. **"Laws as tests" lacks the `State`/`Push` form.** Every script sends responses; none exercises a state element, so the equality-of-readings law's cross-World half ("two results of it in one occurrence must match", line 233) has no test. A script with `State Clock, Instant 0` before the build and `Push Clock, Instant 5100` into a program that reads `$clock` in two widgets would pin it, and the Decision that `State` steps must precede the build for any state tag created at build should be shown once here.
