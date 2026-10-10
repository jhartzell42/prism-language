# Lint: ch08-errors.md

Line numbers refer to /home/claude/book/ch/ch08-errors.md as read.

## Part 1: rule violations

### Rule 1 — History / superseded designs (2)

- L29: "That buffering was adopted so that nothing can observe a half-updated graph."
  Narrates design history. Fix: "The buffer exists so that nothing can observe a half-updated graph."
- L52: "The Rust runtime's `propagate` already contains the mechanism, because a panic in any closure had to be survivable before any of this was designed."
  "before any of this was designed" is history. Fix: "…because a panic in any closure has to be survivable."

### Rule 2 — Status tracking (0)

None. L62, L87, L120, L144 and L233 all say what would change the tentative choice and move on, which is the permitted form.

### Rule 3 — Quoting / attributing Jimmy (0)

None.

### Rule 4 — Banned words (0)

None. "the old generation" (L60) is the technical previous `dyn` generation, not a superseded design; "no longer exists" (L3) is literal.

### Rule 5 — Clefts and non-default word order (2)

- L190: "The thing a `catch` would offer is for a parent to react to a child's failure in the occurrence the failure happened, by running an arm that was not otherwise going to run."
  Pseudo-cleft. Fix: "A `catch` would let a parent react to a child's failure in the occurrence it happened, by running an arm that was not otherwise going to run."
- L48: "The only place a smaller unit is coherent is one with a boundary of its own, and there is exactly one such place."
  Inverted order. Fix: "A smaller unit is coherent only where something already has a boundary of its own, and exactly one thing does."

### Rule 6 — Lecturing tone (0)

None. The bulleted lists (L31–38, L80–83, L222–227, L500–519) are mechanism and policy, not points the reader knows.

### Rule 7 — Dry definition before motivation (4)

- L116, section "Static causality and runtime detection", opens: "The compiler finds every cycle it can see, and the question is what it can see."
  Lead instead with the lived case already in the chapter: a button that relabels itself is legal and useful, and `@Double !out` one mode over is a loop, and nothing in the block's text tells the two apart; then the edge taxonomy.
- L194, section "What the World sees", opens with the run-call contract and goes straight into a Rust enum.
  Lead with the moment the value is for: a differential test fails, the transcript line says "cycle", and you need the Prism line from that one value without opening the runtime. Then the struct.
- L231, subsection "Errors inside `Defer` turns", opens with "A turn is several occurrences when `Defer` slots fire…", a definition.
  Lead with the sandbox: the denial occurrence committed, the child's reaction in the next occurrence of the same turn overflows, and the user's one click is now half a turn. Then the rule.
- L239, subsection "Errors in World component code", opens with "A World component is host code and can fail in every way host code fails."
  Lead with one concrete failure: the host `read` that panics on a path with a NUL byte in it, or a toolkit call that returns an error code the component author never checked. Then the policy per component kind.

### Rule 8 — Presence or drop as a trigger (0)

None. Quarantine happens by the World's `quarantine:` input (L76), exit is not mentioned, the `Defer` scenario (L488–492) is request-driven throughout, and teardown of an old generation at commit (L60) is `dyn` semantics, not a trigger.

### Rule 9 — Design conflicts with the BIBLE Decisions (11)

- L90–98: the `ErrorKind` choice lists `Cycle Overflow UnknownPath WrongResponse DoubleResponse UnsuppliedState DeferBackwards Component`.
  Decisions: `ErrorKind` is `Cycle | Overflow | UnknownPath | UnsuppliedState | DeferBackwards | Component | Build`; a wrong-typed or duplicate response from the World is kind `Component`; runaway recursion is kind `Overflow`. Corrected: drop `WrongResponse` and `DoubleResponse`, add `Build`, and say in prose that wrong-typed and duplicate responses report as `Component` with the detail in `message`.
- L108–110: "tag Error of Element:\n    error: RuntimeError"
  Decisions: one tag `Error` with fields `kind: ErrorKind`, `spans`, `occurrence` (directly on the tag). Corrected: `tag Error of Element:\n    kind: ErrorKind\n    spans: [Span]\n    occurrence: Nat\n    turn: Nat\n    message: Text\n    owner: ?Path`. Then L174 `Error(error=...)` becomes `Error(kind=Cycle, ..)` and L183 `IsError s.element` matches on the tag.
- L157–159: "tag Errors of Element:\n    responds: RuntimeError"
  Decisions: "the World offers `Errors` (responds: Error)". Corrected: `responds: Error`, the same value the `Error` presence element holds, so there is one error value in Prism code and one at the boundary.
- L196–213: `pub enum RuntimeError { Cycle {..}, Overflow {..}, UnknownPath {..}, WrongResponse {..}, DoubleResponse {..}, UnsuppliedState {..}, Build {..}, Native {..} }`
  Decisions: "one type `RuntimeError { kind: ErrorKind, spans, occurrence, ... }`"; wrong-typed or duplicate response is kind `Component`. Corrected: a struct `RuntimeError { kind: ErrorKind, spans: Vec<Span>, occurrence: u64, turn: u64, owner: Option<Path>, message: String, detail: Detail }` where `Detail` holds the per-kind payload (the loop's edges, the overflow's operands, the sender chain). `WrongResponse`/`DoubleResponse` → `kind: Component`; `Build` is a kind; `Native` is not a kind the Decisions list, so a native failure during construction is `Build` and a native failure during propagation needs a kind the chapter must pick from the closed list and say so (the Decisions already put runaway recursion under `Overflow`).
- L218: "A `DeferBackwards` error is not in this enum because the runtime does not know what `Defer` is; the world core produces it, with the slot's path and chain, and it is reported through the same channel."
  `DeferBackwards` is in `ErrorKind` (and in the chapter's own choice at L97). Corrected: "The world core, not the runtime, constructs the `DeferBackwards` value, because the runtime does not know what `Defer` is; it is the same `RuntimeError` type, reported through the same channel."
- L194: "The run call returns either a result or an error, never both"
  Decisions: `Outcome` has `errors: Vec<RuntimeError>` for contained build failures. Corrected: "An aborted occurrence returns an error and no list. A completed occurrence returns the list and, in `Outcome.errors`, every build failure it contained." Also add at L60–62 that a contained build failure is reported in `Outcome.errors` and from there reaches `Errors`; the chapter currently only says "whoever reads the `Errors` stream".
- L148: "An implementation can do better by counting depth in the closure compiler and aborting the occurrence past a limit, which turns a runaway recursion into a reported error with the function's span; the limit is the implementation's and the specification says only that a conforming implementation does not hang."
  Decisions: a recursion depth guard in the closure compiler is the design, and runaway recursion is kind `Overflow`. Corrected: "The closure compiler counts depth and aborts the occurrence past a limit, as an `Overflow` error with the function's span; the limit is the implementation's." Consequently L21 "Only the last has a language answer" is wrong: stack depth in a plain function has one too.
- L235: "Other `Defer` slots that fired in committed occurrences and have not yet been answered are still owed an answer, and the world core continues the turn with them."
  Decisions: "an occurrence aborted inside a turn stops the rest of the turn and the World acts on what committed". Corrected: "The turn ends there. `Defer` slots that fired in committed occurrences and were not yet answered are left waiting, as a program is always left when a response does not come; the World acts on the turn's committed state." Rewrite the opening of L233 ("Abort only that occurrence") to say the occurrence is what rolls back and the turn is what stops.
- L437: "@Element Gui.Button(label=_)" and L347: "path 0/#0/0 (Gui.Button(label="Increment"))" and L305: "sent by: the World (the CLI World, Gui component)".
  Decisions: `Gui.*` is only `Window`, `Scene`, `Focusable`, `Metrics`, `Frames`; a button in a worked example is `Ui.Button` responding `Click`. Corrected: `Ui.Button(label=_)`, `Ui.Button(label="Increment")`, and "sent by: the World (its `Ui` component)" or just "the World".
- L472: "With no `State` step, and `Game.Score` declaring no default, the test fails at the slot's creation"
  Decisions: no `default:` in tag declarations; a `State` step must precede the build for any state tag the program creates at build. Corrected: cut "and `Game.Score` declaring no default".
- L76–85 (section "Containment to an owning subtree"): nothing says quarantine is Rust-only.
  Decisions: "quarantine scripts are Rust-only and chapters 8 and 9 say so"; the Haskell implementation rolls back by replay on a fresh local timeline. Corrected: add after L76: "Quarantine is the Rust runtime's. The Haskell implementation rolls an aborted occurrence back by replaying on a fresh local timeline and takes no `quarantine` input, so the conformance scripts that exercise it are Rust-only."

### Rule 10 — Unverified specifics (2)

- L27: "Reflex programmers know the rule from the other side: an exception inside propagation leaves Spider in a state nobody has thought about, so nothing in the network may throw, and that rule is enforced by discipline rather than by the library."
  Not in research-notes.md. The Reflex sources under /tmp/claude-0/rx/ (Spider.hs) show only `EventLoopException` thrown for a causality loop and no catch inside propagation; no stated "nothing may throw" rule. Verify or mark from memory or cut.
- L225: "ending the run with a dialog is honest and is what most desktop toolkits do when their equivalent happens."
  Attribution to "most desktop toolkits" is not in research-notes.md. Verify or mark from memory or cut.

The current-runtime facts at L52 and L128–130 (`catch_unwind` in `propagate`, action map cleared, height reset, deferred actions dropped, end-of-occurrence clears run, panic rethrown; the trigger's `scheduled` flag cleared in the action's `Drop`; the height-must-increase panic; `try_lock` on a per-node height mutex panicking "event cycle") check against /home/claude/jhartzell42/prism-language/prism_runtime/src/runtime/mod.rs, event/external.rs and event/switch_hold.rs. The arithmetic in the overflow scenario (2^62, 2^63, 2^64 against a 64-bit `Nat`) is right.

### Rule 11 — Code that cannot be Prism (0)

None found. `\|e| @Element e` as a forwarding supplier (L181) matches the BIBLE's own forwarding idiom; `$last <- Nil <= Some !err`, `$gen <- 0 <= $gen + 1 on !restart`, `let !denied = !Filter (IsDenied !r)` (with `!Filter: fn !?a -> !a` from chapter 2) and the `match _:` inside a `dyn` block all fit Part II syntax. The field-name problems in the `Error` tag are counted under rule 9.

### Rule 12 — Filler (3)

- L68: "The build-time abort does not need to abort the occurrence, and it is worth saying why that is sound rather than merely convenient."
  Fix: "The build-time abort does not need to abort the occurrence, and the reason is sound rather than convenient: the build happened at some height…" (or just cut the clause).
- L152: "That is the weakness of the occurrence as the unit, and it is worth being precise about what the weakness is."
  Fix: cut the second clause; the next sentence is the precision.
- L78: "What quarantine costs, stated plainly, since the costs are the argument against it:"
  Fix: "What quarantine costs, and the costs are the argument against it:"

### Counts

| Rule | Count |
|---|---|
| 1 History | 2 |
| 2 Status | 0 |
| 3 Attribution | 0 |
| 4 Banned words | 0 |
| 5 Clefts / word order | 2 |
| 6 Lecturing | 0 |
| 7 Definition before motivation | 4 |
| 8 Presence/drop as trigger | 0 |
| 9 BIBLE conflicts | 11 |
| 10 Unverified specifics | 2 |
| 11 Non-Prism code | 0 |
| 12 Filler | 3 |
| Total | 24 |

## Part 2: structural notes

1. **`owner` is defined for cycles only.** L74 defines the owner for a cycle (lowest common owning widget node of the loop). `RuntimeError.owner: ?Path` is then used for overflow (L205), build failures (L211) and in the overflow scenario (L426: "the owner is `@totals`"), but nothing says what the owner of an overflow is (the node that owns the lifted operation's closure?), why `UnknownPath` has none, or who computes it (the runtime, from the node that was running when the error was found). One paragraph after L74 stating the owner per kind, and that the runtime remembers the owners it named so it can check a later `quarantine:` input against them (the BIBLE's rule), closes the gap. Right now nothing says what happens when the World sends a path the last error did not name: refused, ignored, or performed.

2. **The error node has no path spelling.** A failed `dyn` build leaves "a node whose element list is one presence element, `Error`, at the position the block's slots would have had" (L60). Chapter 9's `Expect Error.*(..)` needs to know where that is in the transcript: is the `Error` element at `0/#1/0` (generation `#1`, slot 0), or at `0/#1` with no slot segment? Same for a quarantined owner at `2` (L399 writes `2: Error(...)`, which has no slot segment, while a `dyn` generation's error would). State the rule once and use it in both scenarios.

3. **`Errors` as a responds-only element is thin.** L157 declares it and L161 says the World answers every `Errors` slot in the first occurrence after the error. Missing: that it is responds-only (no `requests`), so a `Kind` of it is `{requests: false, responds: true, state: false}`; how the Test World scripts it (a `Send` to the `Errors` slot's path, or automatically from the recorded error?); whether an error from occurrence n is delivered to an `Errors` slot created in n+1 (presumably yes, since delivery is in n+1 and routing is against n+1's opening list, but say so); and that it collides by name with chapter 9's `Errors` step. The rule "a World that offers it answers every slot" is stated with no enforcer named: it is a World obligation under the core, like the others in the collected list, and should appear there (it does at L512 but without the "if it offers one" being a `World.Tags` matter).

4. **Interceptor routing of `Errors` by owner path (L172) is asserted, not shown.** "routing each error, by the owner path it names, to the child whose subtree contains it" needs the interceptor to compare a `Path` it was given against its child's paths, which L112 allows ("compare it, and look it up in a list it was given"), but a prefix test on an opaque path needs a prelude function (`IsUnder: fn (Path, Path) -> Bool` or similar). Name it or drop the claim to "a shell can offer `Errors` itself and filter by owner".

5. **The `for` build failure is claimed and never walked through.** L64 and L226 rest the server story on one connection's build failing in isolation, and the worked scenarios cover `dyn` builds, a switch adoption, overflow, unknown path, unsupplied state and deferring backwards, but no `for` key. A sixth scenario (a `for` over connections where one key's block closes a cycle through a widget function, showing the consolidated map without that key and the other keys untouched) would pin the one case that makes quarantine-under-`for` worth having.

6. **"What the current runtime does" (L50–52) duplicates chapter 10.** The panic-recovery path through `propagate` belongs to the Rust runtime chapter; keep one sentence here ("the Rust runtime's `catch_unwind` in `propagate` is the backstop, chapter 10") and move the detail. Likewise "Errors in World component code" (L237–247) is world-core policy that chapters 6 and 12 own; say which chapter holds the normative text, or this one will drift from them.

7. **The build-failure report path is unstated.** A contained build failure (L60) is reported "through `Errors`" but the chapter never says how it leaves the runtime (it must be `Outcome.errors`, per the Decisions, flagged under rule 9) and whether a World that does not offer `Errors` sees it at all beyond the `Error` element in the list. The CLI World's policy (L224) is written for aborts; say what it does with a contained build failure: print and continue, or print and exit.

8. **Message forms and the error value do not line up field for field.** The build-form message (L271–282) names no turn; the overflow message (L286–295) names operands but no owner; the unknown-path message names the removing occurrence, which the struct has no field for. Chapter 9's transcript format should be derivable from the struct. Either add the missing fields to the struct (turn everywhere, `removed_in: Option<u64>` for unknown path) or say the message is rendered from `message` plus the fields, so the forms here are examples, not a format.

9. **Eager detection at switch adoption needs its "when" stated against the commit order.** L126 says the adoption's height walk runs "at the commit of the occurrence that switched, after propagation is complete and before holds commit". Chapter 3/10 define commit as holds then switches, or switches then holds; whichever it is, the walk has to run before either commits for the rollback to be clean, and the chapter should say the walk is the first step of commit and name the chapter whose commit order it relies on.

10. **The quarantine instruction's interaction with pending inputs is a World obligation without its enforcer.** L76 and L513: inputs to paths under a quarantined owner "are delivered to nothing and reported with their paths". Who reports: the runtime, in the same `Outcome`, as `unrouted` paths alongside the torn-down slots' request firings? Name the field and the party, or the Test World cannot check it.
