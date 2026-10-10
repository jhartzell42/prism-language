# Lint: ch09-conformance-testing.md

## Part 1: rule violations

### Rule 1 — history / superseded designs (2)

1. L1149: "The programs as written here are updated to the current syntax: numbers by inference with `-1` and `+1` where a literal needs a sign, tags declared with their fields and response types, `click` as the click, the three-argument `intercept`, and the `ChildInput` convention."
   Rule 1: "updated to the current syntax" refers to an earlier state of the programs. Fix: cut the sentence, or state it as fact: "The programs use inference for numbers, `-1` and `+1` where a literal needs a sign, tags declared with their fields and response types, `click` as the click, the three-argument `intercept`, and the `ChildInput` convention."

2. L1334: "**07. A rebuild swaps slots.** The `@toggle` widget of the law tests, with the original two scripts:"
   Rule 1: "the original two scripts" points at a prior version of the program file. Fix: "The `@toggle` widget of the law tests, with two scripts of its own:".

### Rule 2 — status tracking (0)

The three "tentative" sentences (L186, L213, L2060) each say what would change them and so pass this rule; two of them conflict with Decisions instead (see rule 9, items 2 and 6).

### Rule 3 — quoting / attributing Jimmy (0)

### Rule 4 — banned words (0)

### Rule 5 — clefts and non-default word order (5)

1. L13: "What that excludes is as deliberate as what it includes."
   Fix: "The exclusions are as deliberate as the inclusions."

2. L231: "What the language fixes is that leaves are elements and containment is interception; how the children of a container are spelled is the library's."
   Fix: "The language fixes only that leaves are elements and containment is interception; the spelling of a container's children belongs to the library."

3. L848: "it is the Test World's own script component that is checked here, and the real Worlds are checked against the same sentence in their own tests."
   Fix: "this test checks the Test World's own script component, and the real Worlds are checked against the same sentence in their own tests."

4. L1850: "what matters is that the Haskell frontend prints the same characters for the same source."
   Fix: "the Haskell frontend must print the same characters for the same source."

5. L2077: "What is never done is resolving a divergence by making one implementation copy the other's behaviour without a sentence to point at, because…"
   Fix: "A divergence is never resolved by making one implementation copy the other's behaviour without a sentence to point at, because…"

### Rule 6 — lecturing tone (0)

The bullet lists at L1975–2005, L2013–2019 and L2029–2037 are lists of test names, invariant items and obligations, which the rule allows.

### Rule 7 — dry definition before motivation (3)

1. L17, opening of "The Test World": "The Test World is a World with no effects, written once per host language on top of the world core, with one component that answers to a script instead of to an operating system. Three properties make it the suite's World."
   Lead instead with the fake-filesystem problem that is now buried at L19 ("Suppose instead the Test World had components, a fake filesystem… a program that expects `Missing` for an absent file is testing the fake's opinion"), then the definition.

2. L67, opening of "Patterns": "A pattern is what `match` accepts for an element, written where the step wants one: a tag with named fields and `..` for the rest…"
   Lead with the lived case at the end of the paragraph: a label whose text changed is a different slot, so `Expect Gui.Label(text="Count: 1")` after a click is a complete check; then say what a pattern is.

3. L2025, opening of "World tests against the world core": "A World is the world core plus its components, and the core's obligations are the same for every World, so they are tested once, in the host language, with a fake component."
   Lead with the problem: a CLI World and a GUI World that each re-implemented the turn loop would each get the `Defer` bound or the snapshot order wrong in their own way, and nobody would notice until a program behaved differently under two Worlds. Then the fake component.

### Rule 8 — presence or drop as a trigger (1)

1. L2064: "*File first, then stdin*: a program that reads a file from `@started` and builds its `Cli.Stdin` slot inside a `dyn` after the response. Pins that stdin is not read until the slot exists, which also keeps the transcript free of a race."
   "stdin is not read until the slot exists" states that a slot's existence starts a read. Fix: "Pins that no stdin chunk is delivered before there is a slot to deliver it to, which also keeps the transcript free of a race." (The World delivers responses to known paths; it does not act because a slot appeared.)

### Rule 9 — conflicts with the BIBLE Decisions (12)

1. **`Gui.*` vs `Ui.*`, throughout.** Decisions: "`Gui.*` is the primitive vocabulary every GUI World speaks: `Gui.Window`, `Gui.Scene`, `Gui.Focusable`, `Gui.Metrics`, `Gui.Frames`. `Ui.*` is the Prism GUI library's own tags (`Ui.Button`, `Ui.Label`, `Ui.TextInput`, `Ui.VBox`, …) … Worked examples that show a button under the Test World use `Ui.Button` responding `Click`." The chapter uses `Gui.Button`, `Gui.Label`, `Gui.VBox` and `Gui.*(..)` in every script, transcript and declaration. Representative sentences:
   - L34: "`0/#0/0: Gui.Button(label="Increment")`" → "`0/#0/0: Ui.Button(label="Increment")`".
   - L257–262: "`tag Gui.Label of Element:` … `tag Gui.Button of Element:`" → `tag Ui.Label of Element:` / `tag Ui.Button of Element:`.
   - L1095: "`tag Gui.VBox of Element:`" → "`tag Ui.VBox of Element:`".
   - L1099–1101: "`let IsGui: fn Element -> Bool = \|e| match e: Gui.*(..): True`" → `IsUi` matching `Ui.*(..)`.
   - L19: "one that creates a `Gui.Button` is tested without a screen" → `Ui.Button`.
   - L2065: "*Unsupported*: a `Gui.Button` at the root." → `Ui.Button` (which is exactly a tag the CLI World does not accept).
   Fix: global rename `Gui.Button`/`Gui.Label`/`Gui.VBox`/`Gui.*` → `Ui.Button`/`Ui.Label`/`Ui.VBox`/`Ui.*` in this chapter; keep `Gui.*` only if a script ever shows `Gui.Window`/`Gui.Scene`. Chapter 11's golden (its L1040) already prints `Ui.Button`.

2. **`default:` in tag declarations.** Decisions: "`State` steps may precede the build and must for any state tag the program creates at build (no `default:` in tag declarations)." Conflicting sentences:
   - L204: "A state element with no `State` step has the value its tag declares as a default, if it declares one; otherwise the slot's creation is the unsupplied-state error above."
   - L210: "`default: Unavailable`" and L266: "`default: Instant 0`", L291: "`default: Unavailable`".
   - L213: "The spelling is tentative because it is the only thing in a tag declaration that is not about the element's meaning; if it ends up elsewhere, say in the test file as `Default Clock, Instant 0`, the scripts change by one line each."
   - L1575–1577: "`test the_default_answers_when_nothing_was_set: Test @watcher: array: Send Gui.Button(label="Open"), click; Expect Gui.Label(text="-")`".
   Fix: delete the "Declared defaults" subsection and the three `default:` lines; the rule is "a state tag the program creates at build must have a `State` step before the build, and one created later must have one before the turn that creates it; otherwise the slot's creation is the unsupplied-state error". Rewrite `the_default_answers_when_nothing_was_set` as `an_unset_state_fails_when_the_slot_is_made`: `Send Ui.Button(label="Open"), click` then `Errors Error(kind=UnsuppliedState, ..)`. Every script that pushes `Clock` into a program with no `Clock` slot (`@on_button`, `@starter`, `@started_result`, `@silent`, `@restarter`) is unaffected; the ones with a `Clock` slot already have a `State` first.

3. **The error type.** Decisions: "one tag `Error` with fields `kind: ErrorKind` (a closed choice: `Cycle`, `Overflow`, `UnknownPath`, `UnsuppliedState`, `DeferBackwards`, `Component`, `Build`), `spans`, `occurrence`"; boundary type `RuntimeError { kind: ErrorKind, spans, occurrence, ... }`. The chapter makes `Error` an extensible variant with one tag per kind:
   - L186: "Errors are values of an extensible variant `Error`, so the patterns are element-style patterns: `Error.UnknownPath(..)`, `Error.DeferBackwards(..)`, `Error.Cycle(..)`, `Error.UnsuppliedState(element=Clock)`, `Error.Overflow`."
     → "Errors are `Error` values with a `kind`, so the patterns are record patterns on the kind and the fields the test cares about: `Error(kind=UnknownPath, ..)`, `Error(kind=DeferBackwards, ..)`, `Error(kind=Cycle, ..)`, `Error(kind=UnsuppliedState, element=Clock, ..)`, `Error(kind=Overflow, ..)`."
   - L46: "An aborted occurrence prints `error: Error.UnknownPath(path=0/#1/0)`" → "`error: Error(kind=UnknownPath, path=0/#1/0, occurrence=1)`".
   - L197, L868, L1038, L1073, L1706: `Errors Error.UnsuppliedState(element=Secret)` / `Errors Error.UnknownPath(..)` / `Errors Error.DeferBackwards(..)` → the `Error(kind=…, ..)` form.
   - L1967: "`Error.UnknownPath(path=0/#1/0, turn=1)`, `Error.DeferBackwards(path=1/[3]/#0/1, turn=0, occurrence=2)`, `Error.Cycle(spans=[...])` … `Error.UnsuppliedState(element=Secret, path=0)`" → `Error(kind=UnknownPath, path=0/#1/0, occurrence=…)` etc., with `spans` and `occurrence` as the common fields.
   - L186: "a later `Expect Error.*(..)` will see the quarantine element in the list" → `Expect Error(..)` (and see item 12 on quarantine).

4. **`Cli.Exit` / `Cli.Args` / `Cli.Env`.** Decisions: exit/args/env are `Process.Exit` (`requests: !Code`), `Process.Args`, `Process.Env`; `Cli.*` is only stdin/stdout/stderr/signals.
   - L2059: "and exit by `Cli.Exit`'s request" → "`Process.Exit`'s request".
   - L2060: "a program that prints and never fires `Cli.Exit`" → "`Process.Exit`".
   - L2061: "two `Cli.Exit` slots firing in one occurrence, which is an error" → see item 5.
   - L2063: "a program that prints `Cli.Args` and `Cli.Env` from their state elements" → "`Process.Args` and `Process.Env`".

5. **Two exit firings.** Decisions: "two `Process.Exit` firings in one turn: the first in occurrence-then-list order wins."
   - L2061: "two `Cli.Exit` slots firing in one occurrence, which is an error" → "two `Process.Exit` slots firing in one turn, where the first in occurrence-then-list order is the code the process exits with".

6. **The end of a CLI run is decided.** Decisions: "a CLI run ends on `Process.Exit` or when nothing can cause another occurrence (stdin until end, signals, in-flight requests and timers keep it alive; args/env/clock/watch do not)."
   - L2060: "This is the tentative rule about ending a run; if a run instead waits for an explicit exit, the transcript changes to a timeout and the sentence changes with it." → cut; replace with the list of what keeps a run alive: "Pins exit 0 when nothing can cause another occurrence: no stdin slot, no signal slot, no request in flight, no timer; args, env, clock and watch slots do not keep a run alive."

7. **The typed-AST text form.** Decisions: "the typed-AST text form is `name: sigil type [mark] group = expr @span` with `placeholder`/`close` lines … The typed-AST text form is as chapter 11 defines it; a binding whose root is a graph node is marked `[occ]`." Chapter 11 (its L973–978, L1040) prints `!!clicks: !!Click [build] g1 = dyn $label \|label| @widget:` and `_: !Click [build] g1 = element Ui.Button{label=label} responds`, with `close name name.hold`.
   - L1836: "`  g0  !increment : !Click   build  = bind @Button (const $Text "Increment")   @2:5-2:38`" → "`!increment: !Click [build] g0 = bind @Button (const $Text "Increment") @2:5-2:38`", and likewise for every line of the counter dump (group after the mark, mark in brackets, no space before the colon).
   - L1842: "`  g3  close $count`" → "`close $count $count.hold`".
   - L1850: "Elements print as `slot Gui.Button {label: ...}` with the tag's kind" → "Elements print as `element Ui.Button{label=…} responds`, as chapter 11 defines"; and "`for` as `for $m (\key $_ -> block) : consolidate !` with the consolidation kind" → add the birth-firing flag the Decisions require the text form to print: "`for $m (\key $_ -> block) : consolidate ! birth-firing`" (or whatever spelling chapter 11 fixes). The counter golden as a whole should be regenerated from chapter 11's definition rather than kept as an independent rendering.

8. **Path segments.** Decisions: "only slots, `dyn` generations, `for` keys and intercept children take segments; a plain widget-function application makes a node but adds no segment … Supplier-built slots print `s/n/...` with the supplied widget's own segments after the prefix."
   - L48: "bind positions from the root joined by `/`, a `dyn` generation as `#n` from `#0`, a `for` key as `[k]` with the key printed as a value." → "slot and intercept-child positions from the root joined by `/`, a `dyn` generation as `#n` from `#0`, a `for` key as `[k]` with the key printed as a value; a plain widget-function application adds no segment, which is why `@Button`'s button is `0/#0/0` and not `0/0/#0/0`."
   - L48: "Slots created by widgets that an interceptor's supplier built print as `s/n`, numbered in creation order within the interceptor's node" → "print as `s/n/…`, `n` numbered in creation order within the interceptor's node and the supplied widget's own segments after it".

9. **Snapshot per turn, not per occurrence.** Decisions: "the snapshot is held still across a turn (pushes enter only with the first occurrence)."
   - L136: "since a World gathers everything that changed into one snapshot per occurrence." → "into one snapshot per turn, delivered with the turn's first occurrence."
   - L1586: "has forgotten that a World gathers everything that changed into one snapshot per occurrence." → same fix.

10. **A failed turn re-queues its pushes.** Decisions: "a failed turn drops its responses and re-queues its pushes." L186 says only "the turn ends there; a check after an `Errors` step sees the list as it stood before the failed turn." Add: "A `State` whose push was in the failed turn is not lost: it goes out with the next turn, so a `Push` that fails and a `Send` that follows is a turn with that push in it." (And a law test for it: `a_failed_turn_requeues_its_push`.)

11. **Supplier identity under a changed supplier.** Decisions: "a new child slot whose element value already has a live supplied widget joins it regardless of the current supplier dynamic." The chapter's "Two child slots of equal value share one supplied widget" (L984) and program 19 test only the static case. Not a contradiction, but the sentence has no script; add one where the supplier dynamic changes between the two readers' creation and `ExpectCount Fs.Contents(path="/x"), 1` still holds.

12. **Quarantine is Rust-only, and chapter 9 says so.** Decisions: "quarantine scripts are Rust-only and chapters 8 and 9 say so." The chapter mentions quarantine once (L186) and never says this. Add, where `Errors` is introduced or in the differential-test section: "Scripts that exercise quarantine run under the Rust implementation only; the Haskell implementation rolls back by replay and has no quarantine, so those scripts are excluded from the differential diff."

### Rule 10 — unverified specifics (1)

1. L1823: "The corpus has about forty entries, the eight kinds above and their variants" — a count of a corpus that does not yet exist, stated as fact. Fix: either list the entries (the sentence already names six variants; name the rest) or say "the eight kinds above and their variants, listed in the corpus's README".

(L1080 "forty-four scripts" is correct: 44 `test` statements between the shared declarations and that sentence. L1084 "Twelve of them exist" matches `/home/claude/prism-tests/`, though its tenth file is `10_dynamic_records.prism`, not `10_tag_matching`; see structural note 7.)

### Rule 11 — code that cannot be Prism (4)

1. L1050, L1057: "`$log <- [] <= (Append $log, "a" on !r1) | (Append $log, "b" on !r2)`" — by the comma rule the chapter itself cites at L1854 (`intercept @_, !Union !kid_ins, !fwd_ins` parses as `intercept(@_, Union(!kid_ins), !fwd_ins)`), `Append $log, "a" on !r1` is `Append($log, "a" on !r1)`, which lifts `on` inside the call and is fine only by accident; every other script writes the call in parentheses (`Insert ($keys, Size $keys, unit) on !add`). Fix: "`(Append ($log, "a") on !r1) | (Append ($log, "b") on !r2)`".

2. L1900–1907, the negative golden "cannot decide the numeric type" for "`$n <- 0 <= $n + 1 on !tick`" — contradicts the chapter's own law tests, which write exactly this line (`$n <- 0 <= $n + 1 on !b`, L371, L391, L407, L482, L555, L621 and more) with nothing pinning the literal, and the counter dump at L1840 that resolves `$count` to `$Int` only because `-1` is in scope. Either the law tests do not compile or the golden is wrong. Fix: decide the rule in one place (chapter 2's literal section) and make this chapter follow it: if an unpinned integer literal defaults to `Nat`, the negative golden is cut; if it is an error, every law-test hold needs `$n: $Nat <- 0 <= …` or an `Int` operand. The lint cannot pick; the chapter must be consistent with itself.

3. L1419: "`ExpectTop Test.Box(value=0, inner=Inner "i")`" — a constructor application unparenthesised inside a field pattern, where the construction at L1389 writes `inner=(Inner "i")`. Fix: `inner=(Inner "i")`.

4. L2059–2063: `Cli.Exit`, `Cli.Args`, `Cli.Env` (also listed under rule 9 item 4). Fix: `Process.Exit`, `Process.Args`, `Process.Env`.

### Rule 12 — filler (0)

## Part 2: structural notes

1. **Two scripts are duplicates.** `unsupplied_state_fails_at_creation` (L196) and `an_unsupplied_state_fails_at_creation` (L1072) are the same script on the same widget. Keep the one under the law tests and have the `Errors` step's example point at it, or give the first one a different error (an unknown-path error is the obvious one, and the chapter has no plain example of it before `@Eager`).

2. **The `Send`-to-container mechanics and the `s/n` prefix are stated without saying who prints the `children` key.** L229 says the Test World wraps the response in `ChildInput`s "innermost first" and delivers to the outermost container's path. It does not say what the `path` inside a `ChildInput` is in the transcript (the child's path in the container's own list, which is the intercepted child's path, not a root path) or how `Expect` prints a nested slot's path in the transcript (L50 says `children` print "with their keys as paths" but the transcript example never shows a container). Add a `@VBox` transcript, two turns, so the reader can see `0/#0/0: Ui.VBox(children={0/#0/0: Ui.Button(...), ...})` and the `responses: {0/#0/0: ChildInput(path=0/#0/0, response=click)}` header.

3. **The harness's transcript comparison for forwarders is under-specified.** L951 and L1285: the transcript of `@Transparent (@Transparent @w)` is compared with `@w`'s "paths aside" / "element by element". Say what the comparison is: strip path prefixes? sort elements by value and compare multisets? The `Clock` example (L951) says the outer list has the `Clock` more than once, so the lists are not equal even up to paths; the comparison must be on distinct element values, or on the GUI subset. State the rule and who implements it (the harness, in the host language).

4. **"Who is bound and what enforces it" is missing for the script-language invariants.** Three rules are stated with no enforcer named: a `State` pattern with `..` or `ignore` "is a script error" (L128); a turn ending in an error with no `Errors` step "fails the script" (L186); `Returns` on an event result "is a script error" (L150). Say whether these are frontend errors in the test file (the frontend knows the widget's result kind and the pattern's completeness) or harness errors at run time, since the chapter elsewhere promises the frontend checks step values against tag declarations (L69).

5. **The Haskell side of the suite is only implied.** The chapter says the Test World is "written once per host language" (L17) and the differential test runs "under both implementations" (L2073), but never says which parts of this chapter the Haskell implementation runs: the layout corpus (yes, L1823), the typed-AST goldens (yes), the transcripts (yes), the runtime unit tests and the invariant (Rust only, by construction), the world-core tests (both? the Haskell Test World sits on what?), the CLI transcripts (Rust only? there is no Haskell CLI World in the BIBLE). One short paragraph in "The differential test" listing which layers diff and which are single-implementation, and that quarantine scripts are excluded (rule 9 item 12).

6. **Missing law scripts for Decisions this chapter is the home of.** No script for: `$Distinct` suppressing an equal-valued change (the equal-value test at L448 covers the hold; its complement with `$Distinct` is the natural second script); `!Updated` on a derived dynamic firing with the new value and never at build; a `Defer` reborn by a `dyn` at the same tree position counting as the same `Defer` for the once-per-turn bound; a wrong-typed or duplicate response from the World being a `Component` error (a world-core test, not a script, but the list at L2029 does not have it); a failed turn re-queuing its pushes (rule 9 item 10). Each is one sentence of the chapters on time, switching and Worlds, and the chapter's own rule at L2087 says a sentence with no test is suspect.

7. **Program 10 and the program table.** The table at L1134 calls program 10 `10_tag_matching`; the directory has `10_dynamic_records.prism`. Since the chapter rewrites the program entirely, say in the table that the file is renamed (without reference to what it was), or give the new file its number and name once and use it consistently. Also the table says what each program pins but not which of the law tests each program duplicates (L1149 promises duplication is kept); a column or a sentence per program would let a reader delete the right one when a sentence changes.

8. **The layout corpus's token kinds are a second interchange format with no definition.** `ident`, `kw`, `sigil`, `nat`, `int`, `text`, `field`, `dot`, `merge`, `lbar`, `rbar`, `lambda`, `open`/`sep`/`close` appear only by example. Either point at the chapter on the lexer (chapter 11) for the token table, or list the kinds once here, since the differential test diffs these files byte for byte and both lexers need the same spellings.

9. **The CLI transcript format is thinner than the Test World's.** L2046 shows the fixture, but not how stderr diagnostics (L2065 "the World's error on stderr naming the interceptors") are compared when they contain spans and paths, whether `env` is the whole environment or an overlay, how the harness sends a signal "from the harness" (after which chunk?), or what the pause between chunks is relative to (the previous turn's completion, presumably, since L2069 says order is the only thing checked). Say that the harness waits for the process to finish writing before the next chunk, and how it knows (a `--sync` flag on the binary, or stdout flush per turn).

10. **The lead paragraph is one 170-word sentence.** L3 lists every section of the chapter in a single sentence after the colon. Break it: the first sentence (the specification/description point) and the second (test from the World's side) are the lead; the catalogue of sections is either cut or becomes a short second paragraph.

## Counts

- Rule 1: 2
- Rule 2: 0
- Rule 3: 0
- Rule 4: 0
- Rule 5: 5
- Rule 6: 0
- Rule 7: 3
- Rule 8: 1
- Rule 9: 12 (item 1 is a chapter-wide rename)
- Rule 10: 1
- Rule 11: 4
- Rule 12: 0

Total: 28 rule items; 10 structural notes.
