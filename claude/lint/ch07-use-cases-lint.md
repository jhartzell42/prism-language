# Lint: ch07-use-cases.md

## Part 1: rule violations

### Rule 1 — history / superseded designs

None found. "In the first implementation" (lines 395, 733, 1256) and "future work" (218, 1449, 1473) describe the plan, not a superseded design.

### Rule 2 — status tracking

- Line 218: "Until there is an `update` block (future work), writing that structure in Prism is the same functional-update ceremony as in any pure language, and the recommendation is that the rope is native code the program calls, like text shaping."
  Rule 2 (advisory/status register: "the recommendation is"). Fix: "…and the rope is native code the program calls, like text shaping."
- Line 557: "…the recommendation is that it is native code the prelude exposes, like text shaping, and the Prism program is the thing that decides when to call it…"
  Rule 2. Fix: "…it is native code the prelude exposes, like text shaping, and the Prism program decides when to call it…"
- Line 1473: "…and in each the recommendation was the same: the structure is a plain value from the program's side, whatever it is inside."
  Rule 2. Fix: "…and in each the answer was the same: …"

### Rule 3 — quoting or attributing Jimmy

None found.

### Rule 4 — banned words

- Line 1443: "The honest cost is latency inside the occurrence: …"
  Rule 4 (near miss on "honestly"; the adjective does the same hedging). Fix: "The cost is latency inside the occurrence: …"
- Line 398: "…what it owes the synchronous tradition is honesty about timing, which the last part of this section pays."
  Rule 4 (same family) and rule 5 (cleft). Fix: "…and it owes the synchronous tradition a plain statement about timing, which the last part of this section gives."

(`Carry` as a mode name at line 517, `Shape` as a function name at line 1441, "text shaping" at 218/557 and `Compose` as a function name at 1076/1083/1093/1118 are names, not prose; not flagged. Consider renaming `Compose` to `Stack` anyway so the word does not appear forty times in a document that bans it.)

### Rule 5 — clefts and non-default word order

- Line 292: "What it can tell, because the type makes it say so, is whether the book is `Live`; a strategy with no arm for `Resyncing` does not compile."
  Fix: "It can tell whether the book is `Live`, because the type makes it say so; a strategy with no arm for `Resyncing` does not compile."
- Line 398: "What Prism adds is that the structure can change while the loop runs, and what it owes the synchronous tradition is honesty about timing…"
  Fix: "Prism adds that the structure can change while the loop runs, and it owes the synchronous tradition a plain statement about timing…"
- Line 717: "What the program paid is one wasted send and one line in the World's diagnostics; what it would have paid in the callback version is a write on a closed descriptor…"
  Fix: "The program paid one wasted send and one line in the World's diagnostics; the callback version would have paid a write on a closed descriptor…"
- Line 735: "…what the program cannot do is stream it, because a request firing is atomic per occurrence."
  Fix: "…the program cannot stream it, because a request firing is atomic per occurrence."
- Line 744: "What it is not is a fast compiler, and the end of this section says where the line is."
  Fix: "It is not a fast compiler, and the end of this section says where the line is."
- Line 867: "What the version of this with a watcher-as-stream and a mutable dirty set does in occurrence 42 is the bug in the first paragraph: …"
  Fix: "The version with a watcher-as-stream and a mutable dirty set does, in occurrence 42, the bug from the first paragraph: …"
- Line 1123: "What a game also shows better than anything else is that rendering is a description: …"
  Fix: "A game also shows, better than anything else, that rendering is a description: …"
- Line 1406: "What it has is the hour's rows as `$shown`, which it chose to keep, labelled as loading."
  Fix: "It has the hour's rows as `$shown`, which it chose to keep, labelled as loading."
- Line 1479 (heading): "### What the first bucket is, then"
  Fix: "### The first bucket".

### Rule 6 — lecturing tone

- Line 346: "Four things to read." / line 1081: "Three things to read."
  Rule 6 (announcing a numbered-points walk). Fix: cut the announcement; the paragraphs stand on their own.
- Line 159: "Read the holds first, because they are the whole specification of what can happen to a buffer."
  Rule 6 (instructing the reader). Fix: "The holds are the whole specification of what can happen to a buffer."

### Rule 7 — dry definition before motivation

- Line 44–46: "### The vocabulary" opens straight into a type/tag block. Fix: one sentence first, e.g. "Three things reach a buffer from outside, and each is a tag:" then the code; the paragraph at line 80 says this and could move above.
- Line 445–446: "### The structure" (control loop) opens with a 45-line code block. Fix: lead with the one-line picture ("A PID loop is a hold that moves on the tick, and the safety policy is one merge in front of the motors:") before the code.
- Line 564–566: "### The vocabulary" (chat) opens with code. Fix: lead with "A connection is three things at once, which is why it is a three-part element:" or move the line-609 paragraph up.
- Line 746–748: "### The vocabulary" (build) opens with code. Fix: move the first sentence of line 797 ("Files are state… A compiler is a process…") above the block.
- Line 1125–1127: "### The vocabulary" (game) opens with code. Fix: move line 1152's first sentence above it.
- Line 1267–1269: "### The vocabulary" (dashboard) opens with code. Fix: move line 1315's first two sentences above it.

### Rule 8 — presence or drop as a trigger

None that break the rule. Line 1152 ("a paused game puts the slot inside a `dyn` over the pause flag and the World stops asking for frames") and line 1140 ("while a slot exists") describe a subscription that the slot is, like any state element; acceptable, but say "the slot is the subscription" so it does not read as the World reacting to a disappearance.

### Rule 9 — design conflicts with the BIBLE Decisions

- Line 9: "`Gui.Scene(window, ops)` is the draw list for a window…" and lines 943–945: "tag Gui.Scene of Element: window: Text / ops: [Op]", lines 1135–1137 (same), line 1023 "`$ops: [Op]`", line 1206 "`ops=(Concat (Values $sprites))`", line 1077 "`ops=$scene`", lines 1118, 1120, 1258 ("The vocabulary of `Op`…").
  Conflict: the decided `Gui.Scene` has a `draw` field holding a plain list of `Draw.*` values (`Draw.Rect`, `Draw.Text`, `Draw.Image`, `Draw.Clip`); there is no `Op` type. Fix: `tag Gui.Scene of Element: window: Text / draw: [Draw]`, rename `$ops`/`Op` to `$draw`/`Draw` throughout sections 6 and 7, and at 1118/1258 say "a `Draw.Image` that names a handle the World already holds".
- Line 1128: "type Frame = frame" and lines 1139–1140: "tag Gui.Frames of Element: responds: Frame # once per frame…" and line 1213: "a program that wants the frame's own time asks for `Gui.Frames` to respond with it instead."
  Conflict: `Gui.Frames` responds `Frame(now: Instant)`. Fix: `type Frame = record: now: Instant`; then the tick is `Tick (!frame.now)` and the whole "one frame behind" paragraph at 1213 goes, or becomes a remark that `$clock` through a tag would have been one frame behind, which is why the frame brings its own time.
- Line 9: "`Gui.Window(title)` responds with pointer, key and resize events and has the window's size as state"; lines 933–941: "type WindowInput = choice: Pointer Point / Pressed Key / Released Key" with "state: Size"; lines 1130–1133 the same.
  Conflict: the decided `Gui.Window` responds with pointer, key and resize events; resize is a response, and the type as written has no resize arm. Fix: add `Resized Size` to `WindowInput`; keep `state: Size` only if the Worlds chapter declares it (it is not in the Decisions), otherwise derive the size with a hold over `Resized`.
- Line 989: "a shell that wants the child to outlive it says so with a `detach` field, and a World that supports one leaves the process alone."
  Conflict: "`Process.Child` has no `detach` field." Fix: cut the clause; or say "a shell that wants a child to outlive it spawns it through a supervisor the World offers, since `Process.Child` is always killed on teardown."
- Line 161: "…the World answers the oracle `Missing` for a file it has not looked at, looks, and pushes `Modified`…, so a freshly opened buffer is sometimes read twice."
  Conflict: "`Fs.Watch` is answered from a real stat at slot creation (`Missing | Modified(stamp)`)." The oracle never answers `Missing` for a file that exists. Fix: cut the double-read story; `@started` reads once, and the stamp's first push is a real change. `Unless` still earns its keep for the save echo.
- Line 214: "the Test World runs this editor with `State Fs.Watch(path="notes.txt"), Modified (Instant 7000)` and `Send Fs.File(path="notes.txt"), Contents "v2"` inside a `Together` with the Undo click"
  Conflict: `State` steps precede the build; a mid-run change of state is a `Push` (as lines 547 and 901 write it). Fix: "`Push Fs.Watch(path="notes.txt"), Modified (Instant 7000)`".
- Line 1112: "…and an `ExpectNone` on frame 5's label afterwards…"
  Conflict: the suite's steps are `Expect`, `ExpectTopNone`, `Unrequested`, …; there is no `ExpectNone`. Fix: "an `ExpectTopNone` for frame 5's label" (or `Expect` on the scene without 5's draw list).
- Line 1451: "…which means a `dyn` rebuilding per step, which the world core treats as the same `Defer` reborn and reports as deferring backwards."
  Conflict: a `Defer` reborn at the same tree position counts as the same `Defer` for the once-per-turn bound; exceeding that bound is the error, and "deferring backwards" is a different one. Fix: "…which the world core treats as the same `Defer` reborn, so the second firing in the turn exceeds the once-per-turn bound and is an error."
- Line 364: path "`1/[7]/1`" vs line 377: "`Acked` to `2/[8]/2` and `Acked` to `2/[9]/2`."
  Conflict with the path rule (same `for`, same bind position, same slot position). Fix: `1/[8]/1` and `1/[9]/1`.

### Rule 10 — unverified specifics

- Line 398: "The synchronous languages (Esterel, Lustre, Lucid Synchrone) were built for exactly this and got the law right thirty years ago"
  "Thirty years" is not in the research notes (the Lustre and Esterel papers there are 1991/1992, so closer to thirty-five). Fix: verify or say "decades ago".
- Line 502: "`integral = 0.0 -> pre integral + inc`"
  Lustre syntax; the notes mark `pre` and `->` as [T] (not read from the primary source). Fix: verify against the Halbwachs paper or mark from memory.
- Line 507: "A Lustre program's static causality analysis, which rejects `x = x + 1` with no `pre`, is the static cycle check"
  The notes record Esterel's refusal of `present A else emit A end`, not Lustre's rejection of that equation. Fix: verify, or use the Esterel example the notes have.
- Line 395: "that is microseconds to tens of microseconds per increment on ordinary hardware, which is three orders of magnitude slower than a tuned kernel-bypass loop and one or two orders faster than it needs to be"
  Unmeasured numbers. Fix: mark as an estimate ("on the order of", "a guess the runtime chapter should replace with a measurement") or cut the comparison.
- Line 1232: "That is the input lag that game programmers measure with high-speed cameras"
  Attribution to a practice; fine as colour but mark or cut if it must be verifiable.

### Rule 11 — code that cannot be Prism

- Line 341: "`@ ev`"
  `!ev` is the bound name. Fix: `@ !ev`.
- Line 641: "`let IdOf: fn Accepted -> Nat = \|a| a.value`"
  `type Accepted = Accepted Nat` is a single-constructor choice, not a record with a `value` field. Fix: `\|a| match a: Accepted n: n`.
- Lines 696 and 700: "`requests=!Filter (!said[key])`" and "`!said` is a second output of `StepAndRoute` (make its result a triple)"
  `!said` is never defined and the code at 668–670 still takes `StepAndRoute` as a pair; also the request expression needs parentheses as the chapter writes them elsewhere. Fix: change `StepAndRoute`'s type to a triple at line 630, add `let !said = Third !stepped` at 670, and write `requests=(!Filter (!said[key]))`. `!put` at 696 is bound and unused; drop the bind.
- Lines 657, 672, 673 and 978, 983, 984: "`{IdOf !accepted: Opened}`", "`\|ignore| Shut`", "`ApplyDeltas`"
  `Opened`/`Shut` and `ApplyDeltas` are never declared; section 1 declares `type Change = choice Open | Close` and `ApplyChanges`. Fix: reuse `Change`/`ApplyChanges` or declare the new ones once.
- Line 329: "`@MarketMaker: fn (Text, Params) -> @$Int`"
  `Params` is never declared. Fix: add `type Params = record: …` or say it is the record `Desired` takes.
- Line 1128: "`type Frame = frame`" also collides with `type Frame = record: app: App; rect: Rect` at line 1003 (different programs, same chapter). Fix: the Decisions' `Frame(now: Instant)` resolves it; rename the compositor's record `Pane` if the collision still reads badly.

### Rule 12 — filler

- Line 39: "One rule from the worked examples is worth restating because every deep dive below hits it: …"
  Fix: "Every deep dive below hits one rule from the worked examples: …"
- Line 159: "…and the merge there is worth a second look: a keystroke and a save in one occurrence leave the buffer dirty…"
  Fix: "…and the merge there matters: a keystroke and a save…"
- Line 719: "Key order is doing work in 300 and it is worth being clear about what."
  Fix: "Key order is doing work in 300."
- Line 849: "The read of `$versions` inside the `for` that produces `$versions` is a cycle, and it is legal for a reason worth spelling out."
  Fix: "The read of `$versions` inside the `for` that produces `$versions` is a cycle, and it is legal."
- Line 741: "TLS, as always, is the World's: …"
  Fix: "TLS is the World's: …"
- Line 1435: "None of this is a weakness to be fixed later; it is the price of a language whose only subject is time, and a language that tried to be good at these too would stop being good at the thing it is for." and line 1483–1484 restate the same point ("A Prism program is a thin layer of time over whatever computations it needs, and the thin layer is where the bugs were", already at line 909).
  Fix: keep one of the three.

## Part 2: structural notes

1. The Conventions section (lines 7–39) declares `Gui.*`/`Ui.*` but every walkthrough with a screen then redeclares `Gui.Window`/`Gui.Scene` with different fields (sections 6 and 7) and names the draw list `ops: [Op]`. Declare the final GUI vocabulary once, in the Decisions' form (`draw: [Draw]`, `Frame(now: Instant)`, `Resized` in the window's responses), and have sections 6 and 7 refer to it; the chapter is read in months and the reader will take the first declaration as true.
2. `@Frame` (lines 1035–1050) is the chapter's one real interceptor and it is under-explained for a reference: say where `!down`'s responses go (through `intercept`'s second argument, by the application's own paths), why `Gui.Window` is supplied by `supply` while its pointer events go through `!down` (state down one road, responses down the other), and what `@Forward` returns. It is also the one place the chapter states an invariant ("a click lands on what the user saw") without saying who is bound: the compositor (hit-test against `$desk` through a tag) and the runtime (routing against the list as the occurrence opened). Say both.
3. Section 4's persistence block (694–700) refers to `!said`, `Store.Value` and `Store.Append` but the server code never produces `!said`, and the "retry behind a `Time.After`" policy is named but not shown. Either show the triple and the retry (six lines) or cut the block to the two sentences about the sharding seam.
4. The market-data section's `Reconcile`, `Desired`, `PullOf` and `Params` are elided with `...`. The hard moment depends on `Reconcile` returning `Place` for both sides and on `PullOf` matching `Pull`; one short body for `PullOf` and a two-line statement of what `Reconcile` diffs would make occurrence 20 checkable.
5. The build system's `ReadyFor` (line 824) decides the whole hard moment and is never written. Give its body (`\|job, want| match job: Some j: if (Newer want, j.on): Some unit else: Nil; Nil: Some unit`), since the walkthrough at 861 argues from what it "says".
6. Section 8's cache walkthrough (occurrence 91, line 1408) depends on the Worlds chapter's `@HttpCache` holding `$cache`, firing `!hit` and a `Defer`; one sentence naming those three names at their first mention here, and that a `Defer` answer is a second occurrence in the same turn, keeps the section readable alone. The same for "the `Defer`-per-chunk pattern" at line 989 and "the router" at 702.
7. Paths in the walkthroughs follow the segment rule but the rule is never restated in this chapter beyond line 7; since `@started` takes a segment (it is a `dyn`), say so in Conventions, because the reader checking `1/[7]/1` and `s/0/3` against the code will otherwise count one short.
8. The latency paragraph (395) and "Hard real time" (1465) overlap with the runtime chapter's performance discussion; keep the use-case-specific sentences (increment-per-occurrence, native book coalescing, C++ loop supervised through a state element) and point at the runtime chapter for the numbers.
9. "Where Prism is the wrong language" (1433–1484) repeats, bucket by bucket, conclusions each section already drew ("the rope", "the ten-megabyte parse", "fifty thousand bullets", "the control loop said this plainly"). It would be stronger as half its length: one paragraph per bucket stating the test ("does a step wait for something outside the program?") and the foreign-function answer, without the recap sentences.
10. The chapter never shows a Test World script in full, though every "What the goals buy" says "the spec is the thing". One complete script, for the editor's occurrence 11 (it is "four lines"), would anchor every later "a `Together` of … and …" sentence; the Conformance chapter is the home for the format, but one instance belongs here.

## Counts

| Rule | Count |
|---|---|
| 1 History | 0 |
| 2 Status | 3 |
| 3 Attribution | 0 |
| 4 Banned words | 2 |
| 5 Clefts | 9 |
| 6 Lecturing | 2 |
| 7 Definition before motivation | 6 |
| 8 Presence/drop trigger | 0 |
| 9 BIBLE conflicts | 9 |
| 10 Unverified specifics | 5 |
| 11 Code not Prism | 6 |
| 12 Filler | 6 |
