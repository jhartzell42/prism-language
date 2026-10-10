# Lint: ch06-worlds.md

Line numbers refer to /home/claude/book/ch/ch06-worlds.md as read.

## Part 1: rule violations

### Rule 1 — history / superseded designs (1)

1. L265: "The chapter 1 script that expected to see `"(reading...)"` after the sandbox held a request back was wrong for exactly this reason: by the end of turn zero the denial had landed."
   Rule 1: argues by saying an earlier version of a script was wrong. Fix: "A script that expects `"(reading...)"` after a sandbox denial would fail: by the end of turn zero the denial has landed."

### Rule 2 — status tracking (1)

1. L469: "Names and fields are sketches until a World is written; the kinds, which parts a tag declares and what each part means, are what the language fixes."
   Rule 2: a blanket "tentative pending implementation" disclaimer over the whole catalogue. Fix: cut the first clause; keep "The language fixes the kinds: which parts a tag declares and what each part means. A World may spell its own tags differently."

### Rule 3 — quoting / attributing Jimmy (0)

### Rule 4 — banned words (0)

("shaping"/`Shape` at L929, L934, L967 is the typography term and a function name; it goes anyway under the `Gui.Metrics` correction in rule 9.)

### Rule 5 — clefts and non-default word order (5)

1. L183: "The one thing every push must satisfy is the equality law: in one occurrence, every slot with equal elements sees one value."
   Fix: "Every push must satisfy the equality law: in one occurrence, every slot with equal elements sees one value."
2. L243: "That rule is what bounds a turn without anyone picking a number."
   Fix: "That rule bounds a turn without anyone picking a number."
3. L461: "that is the same information with the component in charge of timing, and timing is what the core exists to own."
   Fix: "…with the component in charge of timing, and the core exists to own timing."
4. L1233: "One `Defer` per delivery is what keeps the pipe inside the turn rule:"
   Fix: "One `Defer` per delivery keeps the pipe inside the turn rule:"
5. L1257: "It needs no runtime support beyond the run call being cheap, and the no-callback rule is what makes it free:"
   Fix: "…and the no-callback rule makes it free:"

### Rule 6 — lecturing tone (0)

### Rule 7 — dry definition before motivation (7)

1. L215–223, "Turns and `Defer`": opens with the definition of a turn and the `Defer` tag declaration; the problem (an interceptor that cannot answer in the occurrence of the request, the sandbox that must say `Denied`) is in the next subsection. Fix: open with the sandbox that has a `Read` in hand and nothing it may do with it this occurrence, then the tag, then the turn.
2. L475–521, "Fs": opens with forty lines of tag declarations. Fix: lead with the one-line "fire a `Read`, get `Contents` back next occurrence, and two widgets watching one path never disagree" and the `@SaveButton` example, then the declarations.
3. L560–579, "Time and Clock": opens with declarations. Fix: lead with "There is no `now()`" (L581) and the debounce that is testable with a `Push`, then the declarations.
4. L607–643, "Process": opens with declarations. Fix: lead with the identity question at L645 ("one process or two? the element's fields cannot say") and `@RunOnce`.
5. L681–735, "Net": opens with fifty lines of declarations. Fix: lead with "one connection, two conversations" (L737) or the proxy, then the declarations.
6. L784–792, "Errors as a stream": opens with the record and tag. Fix: lead with "A program cannot catch an error, because the occurrence it happened in did not happen" (L794).
7. L890–930, "The vocabulary": opens with the full tag block after a one-sentence intro at L886. Fix: move the "you have lost a cursor to a native text box" paragraph (L886) to the top of the subsection and put a small example (one `Gui.Scene` with two `Draw.Rect`s) before the block.

### Rule 8 — presence or drop as a trigger (0)

### Rule 9 — design conflicts with the BIBLE Decisions (24)

1. `Cli.Exit` throughout: L83 "a `Code 0` on `Cli.Exit`"; L816–817 `tag Cli.Exit of Element: requests: !Code`; L838 `@Element Cli.Exit(requests=(Code 0 on !end))`; L841 "When `Cli.Exit` fires in a turn"; L843 "A program that never fires `Cli.Exit`"; L850, L855 (`turn 0` list / `Code 0`); L872; L1207 ("with the child's `Cli.Stdin`, `Cli.Stdout` and `Cli.Exit` held back"); L1227 `Except [Cli.Stdout, Cli.Exit]`; L1287.
   Corrected form: `Process.Exit` with `requests: !Code`. `Cli.*` is only `Cli.Stdin`, `Cli.Stdout`, `Cli.Stderr`, `Cli.Signals`.
2. `Cli.Args` / `Cli.Env`: L99 "`Cli.Args` gets the arguments"; L670–674 `tag Cli.Args of Element: state: [Text]` / `tag Cli.Env of Element: state: Text ==> Text`; L677 ("a different `Cli.Args`", "`State Cli.Args, ["notes.txt"]`"); L843; L1162 ("the same program with a different `Cli.Args`"); L1182.
   Corrected form: `Process.Args` and `Process.Env`. (L667 already says they belong to the process; the names should say so too.)
3. `Proc.*`: L619 `tag Proc.Spawn of Element`; L638 `tag Proc.Child of Element`; L650, L657, L665, L1123, L1131 (the component list "`Proc`"), L1135 ("a `Proc.Spawn` in a browser").
   Corrected form: `Process.Spawn` (request) and `Process.Child(id)` (state keyed by id).
4. L665: "When the `Proc.Child` slot goes away, the World is free to kill the child or leave it, and says which in the component's documentation; a program that wants a child killed fires `Signal Term` and keeps the slot until `Exited` arrives."
   Decision: a child process is killed under the World's cancel-on-teardown obligation. Corrected form: "When the last `Process.Child(id)` slot for an id is gone, the World kills the child: that is its cancel-on-teardown obligation, the same one that aborts an in-flight read. A program that wants a graceful stop fires `Signal Term` first and keeps the slot until `Exited`."
5. `Time.After` type: L569–571 `tag Time.After of Element: requests: !Duration / responds: Elapsed`; L564 `type Elapsed = choice Elapsed`; L583 "fire a duration, get `Elapsed` back"; L590 `requests=(d on !in)`; L597 "`Send Time.After(..), Elapsed`"; L859; L866 `requests=(Duration 30000 on …)`; L876; L1089 `Input::Response(path, Elapsed)`.
   Decision: `Time.After` is `requests: !Nat, responds: Nat`, a token echoed back. Corrected form: declare it as chapter 1 does (token in, same token out, the duration a plain field); the debounce then matches the echoed token against `$last`'s generation instead of counting, and the REPL's "old timer still fires" bug at L876 becomes "compare the token to the latest one issued".
6. `Time.Frames`: L577–578 `tag Time.Frames of Element: responds: Tick`; L257 ("a frame tick is a `Time.Frames` stream element"); L601; L1287.
   Decision: `Gui.Frames` responds `Frame(now: Instant)`. Corrected form: `tag Gui.Frames of Element: responds: Frame` with `type Frame = record: now: Instant`.
7. GUI vocabulary, L890–930 and everything built on it: `type Draw = choice: Rect … | Text … | Image … | Clip …`; `tag Gui.Window of Element: title / size / draws: [Draw] / responds: WindowEvent`; `tag Gui.Region` (rect, cursor, responds Pointer); `tag Gui.Focusable` with a `rect` field; `tag Gui.Clipboard`; `tag Gui.Fonts` (state FontSet).
   Decision: the five `Gui.*` tags are `Gui.Window` (responds pointer, key, resize), `Gui.Scene` (presence; `draw` field is a plain list of `Draw.*` values: `Draw.Rect`, `Draw.Text`, `Draw.Image`, `Draw.Clip`), `Gui.Focusable` (name only), `Gui.Metrics` (state), `Gui.Frames`. No `Gui.Region`, no `rect` on a focusable, no `Gui.Fonts`; draw commands are `Draw.*` values, not a `Draw` choice; the World does no geometry. Corrected form: declare the five tags and the `Draw.*` values; move the draw list from `Gui.Window` to `Gui.Scene`; drop `Gui.Region`; `Gui.Clipboard` can stay only as an extra tag a World may accept, said as such.
8. World-side hit-testing, following from 7: L17 "It reads a draw list and delivers keys and pointers to focusable regions"; L932 "the World delivers a pointer event to the region whose rectangle contains the point, topmost in list order, which is the one piece of hit-testing the World does, because only it knows where the pointer is"; L932 "the World delivers keys to the focusable the program has told it is focused … or by a `focused: Bool` field the World honours"; L955 "A library that preferred the World to do the first level of hit-testing would emit a region per row instead, and the World would deliver to the region; both are conforming"; L967 "Pointer events are matched against `Gui.Region` rectangles in the committed list and delivered by path"; L985 "captures the GUI library's `Gui.Window` and `Gui.Region` elements"; L999 "on a region's rectangle changing, nothing"; L1287 "a scripted click at a point reaching the right region".
   Decision: the World does no geometry; the library hit-tests; `Gui.Focusable` is name-only. Corrected form: pointer events go to `Gui.Window` and the library routes them; keys go to the one `Gui.Focusable` the library emits; drop the `focused: Bool` alternative and the per-row-region alternative.
9. `Gui.Fonts` + native `Shape`, L928–934: "`tag Gui.Fonts of Element: state: FontSet`", "text shaping is a pure function `Shape: fn (FontSet, Text, Style) -> [Glyph]` the World provides natively".
   Decision: `Gui.Metrics` (state: text measurement/fonts, answered synchronously). Corrected form: `tag Gui.Metrics of Element: state: Metrics`, with measurement a plain function over the state value; say the same thing about "no occurrence between asking and knowing".
10. `Gui.*` where the library's `Ui.*` is meant: L9 "`Gui.Button { label: "Increment" }`"; L108, L110, L112 `Gui.Label(text=…)` in the occurrence-zero outcome; L321 "A `Gui.Button` under a CLI World"; L946 "emits a `Gui.VBox` with the three as children"; L957 "row 0 is the `Gui.VBox`'s first child".
   Corrected form: `Ui.Button(label="Increment")`, `Ui.Label`, `Ui.VBox`; `Gui.*` only for the five primitive tags.
11. L938: "`@Button` is a `Gui.Region` and a label; `@TextInput` is a `Gui.Focusable`, a hold with the text and cursor, and draw commands".
    Corrected form: "`@Button` is a `Ui.Button` element and the interceptor above it turns window pointer events into `Click`s by the layout it holds; `@TextInput` is a `Gui.Focusable` (name only), a hold with text and cursor, and `Draw.*` values the layout interceptor positions."
12. `Cli.Stdout` / `Cli.Stderr`, L810–814: `tag Cli.Stdout of Element: requests: !Text` and the same for `Cli.Stderr`, with no `responds`; L1287 "a closed standard-output pipe producing `Failed`".
    Decision: `requests: !Text, responds: WriteFailed`. Corrected form: add `responds: WriteFailed` to both and make the platform test say `WriteFailed`.
13. Errors at the boundary, L53–60: `pub enum RunError { UnknownPath, WrongResponseType, UnsuppliedState, DuplicateResponse, Cycle, Panic }`; L400 uses `RunError::DeferBackwards`, which the enum does not declare; L785–788 `type RuntimeError = record: kind: ErrorKind # Cycle | Overflow | Protocol | Panic; spans; message`.
    Decision: one type `RuntimeError { kind: ErrorKind, spans, occurrence, … }` with `ErrorKind` the closed choice `Cycle | Overflow | UnknownPath | UnsuppliedState | DeferBackwards | Component | Build`; a wrong-typed or duplicate response from the World is kind `Component`; runaway recursion is `Overflow`; the `Errors` stream responds with that type. Corrected form: replace `RunError` with `RuntimeError` and that `ErrorKind`; add `occurrence`; drop `Protocol`, `Panic`, `WrongResponseType`, `DuplicateResponse` as kinds.
14. `Outcome`, L43–51: has `list`, `requests`, `result` only.
    Decision: `Outcome` has `errors: Vec<RuntimeError>` for contained build failures (a failed `dyn` build does not fire the `dyn`'s event and is reported through `Errors`). Corrected form: add `errors`, and say under "What comes out" that a contained build failure is not a rollback of the occurrence.
15. Quarantine, L794: "For a World that quarantines a failing subtree (replacing it with an `Error(span, message)` presence element in the list)…"; `Inputs` at L33–41 has no `quarantine` field.
    Decision: quarantine is a `quarantine: [Path]` input the World may send on the next run call; the runtime remembers the owners it named in its last error. Corrected form: add `quarantine: Vec<Path>` to `Inputs` and describe quarantine as the World handing back paths from the last error, not as the runtime inserting an element.
16. Failed occurrence inside a turn, L380–382 and L398–400 (`self.snapshot = previous; … re-queue pushes; return self.failed_turn(e, occurrences)` after any occurrence of the turn) and L429 "`failed_turn` discards the turn's responses, puts the turn's pushes back into `incoming`".
    Decision: an occurrence aborted inside a turn stops the rest of the turn and the World acts on what committed. The code resets the snapshot and re-queues the pushes even when occurrence 1 committed with those pushes, so the program would see them twice; and it returns no list or firings for the committed occurrences. Corrected form: roll back the snapshot and re-queue pushes only when the *first* occurrence failed; otherwise return the list and firings committed so far with the error attached, and keep `self.snapshot`.
17. Pushes nobody asked about, L363–364 (`let pushes = self.incoming.drain().collect(); self.snapshot.extend(…)`) and L425.
    Decision: pushes for values nobody has asked about are recorded, not delivered. Corrected form: commit all of `incoming` into the snapshot but pass as pushes only the values that appear in `last_list` (or have been asked of the oracle).
18. The oracle never touches the source: L183 "that is why the oracle reads the snapshot map, never the disk"; L439 "the tag's unavailable case, never a syscall"; L538 "`Unavailable` until the first read lands"; L99/L1119 "`Fs.Contents` of a file the World has never looked at gets `Unavailable`"; L1121 "The tempting alternative is to let the oracle block once, on first ask … It costs…".
    Decision: `Fs.Contents`' oracle may read synchronously with a size cap beyond which it answers `Unavailable`; `Fs.Watch` is answered from a real stat at slot creation. Corrected form: keep the equality-law argument (one read per value per occurrence, cached into the snapshot before answering a second slot) and say the Fs component may read small files and stat on first ask, under a cap, and answers `Unavailable` above it; strike "never a syscall".
19. L599: "a World that fell behind delivers each tick in its own occurrence, or drops ticks and says so in the component's documentation, but never two in one."
    Decision: missed `Time.Every` ticks are skipped. Corrected form: "a World that fell behind skips the missed ticks and delivers the next one; it never delivers two in one occurrence."
20. L843: "Subscriptions to `Cli.Args`, `Cli.Env` and `Clock` do not keep a program alive".
    Decision: args/env/clock/watch do not keep a run alive; stdin until end, signals, in-flight requests and timers do. Corrected form: "`Process.Args`, `Process.Env`, `Clock`, `Fs.Watch` and `Fs.Contents` watches do not keep a program alive; a live `Cli.Stdin`, a `Cli.Signals` slot, a `Time.Every` slot, an in-flight request or timer, and a listener or socket do."
21. L504: `kind: Kind                 # File | Dir | Link | Other`.
    Decision: `type Kind = record: requests: Bool; responds: Bool; state: Bool` with `Kind: fn Element -> Kind` in the prelude. Corrected form: `kind: EntryKind` with `type EntryKind = choice: File | Dir | Link | Other`.
22. L286–287: `!resp <- @Element _.element` / `@ resp`.
    Decision: the catch-all arm is `(!resp, ignore) <- @Element _.element`. Corrected form: `(!resp, ignore) <- @Element _.element` then `@ !resp`.
23. L563–564: `type Tick = choice Tick` / `type Elapsed = choice Elapsed`.
    Clarification: unit-like responses are unit types, `type Click = click`, value `click`. Corrected form: `type Tick = tick` (and `Elapsed` goes under item 5).
24. L957: "fires `{0/#0/0: Click}` down".
    Corrected form: `{0/#0/0: click}` (the value is lowercase).

### Rule 10 — unverified specifics (9)

1. L123: Binance's documented order-book procedure (buffer the stream, snapshot stamped with a sequence number, drop updates at or below it, restart on a skipped id); "CME's recovery feed says the same thing"; Kubernetes list/`resourceVersion`/watch and `410 Gone`; Debezium's log-position-then-snapshot; ZooKeeper's one-shot watches and its documented guarantee of watch-before-data; inotify's man-page overflow advice. None is in research-notes.md. Verify each (link or page) or mark from memory or cut.
2. L125: "the epoll man page walks through the edge-triggered reader that takes one kilobyte of two and then blocks forever". Verify or mark from memory.
3. L125, L197: the Kubernetes quotation "the system is not required to touch base at 5 before changing to 3" (and "what Kubernetes calls not touching base at 5"). A verbatim quote with no source. Verify or mark from memory or cut.
4. L150: "The Fs component's inner loop is the Binance procedure with the occurrence as `lastUpdateId`." A library/API field name. Verify (it is a field in Binance's depth-stream docs) or cut.
5. L93: "Every framework with a `componentDidUpdate` has that table of rules, and every table has a footnote." A library function name. Verify or mark from memory.
6. L979: "the browser's `beforeinput` and composition events". DOM event names. Verify or mark from memory.
7. L1111, L1117: `spawn_local`, `spawn_blocking`. tokio function names. Verify or mark from memory.
8. L1146: "Wayland wants the IME cursor rectangle pushed rather than queried"; L1148: macOS "event loop must be created on the main thread", "Live resize puts the main loop into a tracking mode that expects synchronous layout and paint per size change". Platform claims with no source. Verify or mark from memory.
9. L1199: "A host that instantiates a runtime per request (a WASI HTTP handler)". Claim about WASI's HTTP model. Verify or mark from memory.

### Rule 11 — code that cannot be Prism (6)

1. L9: "`Gui.Button { label: "Increment" }`". Brace-record syntax for an element; everywhere else elements are `Tag(field=value)`. Fix: `Ui.Button(label="Increment")`.
2. L286–287: `!resp <- @Element _.element` / `@ resp`. `resp` is used without its sigil. Fix: as rule 9 item 22.
3. L549–550: `!r <- @Element Fs.File(path=path, requests=(Read on !started))` / `@ r`. Fix: `@ !r`.
4. L561–562, L697: `type Instant = Instant Int`, `type Duration = Duration Int`, `type Accepted = Accepted ConnId`. A single-constructor form not in the syntax. Fix: `type Instant = choice: Instant Int` (one-arm choice), or declare `Instant` and `Duration` as `Int` aliases and `Accepted` as `type Accepted = record: id: ConnId`.
5. L819: `type Signal = choice Interrupt | Terminate | Hangup | WindowChange`. Inline `|`-separated choice; elsewhere choices are block-form. Fix: block form, one arm per line.
6. L1227–1228: `@Forward (Except [Cli.Stdout, Cli.Exit], $ps)` / `@Forward (Except [Cli.Stdin], $cs)`. Tags are not values, so a list of bare tag names has no type; `Kind: fn Element -> Kind` is the only tag-level reflection. Fix: `Except` takes a predicate over elements: `@Forward (Except (\|e| match e: Cli.Stdout(..): True; Process.Exit(..): True; ignore: False), $ps)`, or the shell matches in a `for` arm as the `@Pipe`'s first `for` already does.

### Rule 12 — filler (4)

1. L417: "The snapshot moving once per turn rather than once per occurrence is a choice worth a sentence." Fix: cut; start at "A `Defer` occurrence could take fresh pushes…".
2. L471: "Three forms recur, and it is worth naming them before the catalogue." Fix: "Three forms recur."
3. L1127: "The line between the two is drawn by tag namespace, and it is worth being exact about what a program sees when it crosses it." Fix: "The line between the two is drawn by tag namespace."
4. L1293: "Most of these have appeared above; here they are together, because the list is the design." Fix: "The list is the design."

## Part 2: structural notes

1. **GUI section needs rebuilding around the decided vocabulary (L882–1005).** Rule 9 items 7–11 are not spot fixes: "The vocabulary", "The GUI library", "A GUI turn, end to end", "Software rasteriser" and "X11 over a socket" all assume `Gui.Region` and World-side hit-testing. The end-to-end turn should show `Gui.Window` receiving `Pointer (Down (50,10), Left)`, the layout interceptor routing it, and `Gui.Scene` with `Draw.*` values as the thing the rasteriser reads. The software rasteriser then has even less in it, which strengthens the chapter's own argument.
2. **Two `Process.Exit` firings in one turn** (decision: the first in occurrence-then-list order wins) is not stated anywhere. Put it in "Exit as a request" next to the "writes land first" rule, with who enforces it (the world core, when it performs firings).
3. **The turn-loop code (L337–413) cannot be read as Rust without a note.** `pushes` is moved in two `for (ev, v) in pushes` loops inside `loop`, `previous` is captured by the oracle closure and then assigned to `self.snapshot`, `clone_shallow` is undefined, and `Inputs` holds `&oracle` across iterations. It is labelled pseudocode at L21 but not here; either say so once above the block or fix the ownership (clone `pushes` into the re-queue, keep `previous` in an `Rc`).
4. **Obligations with no enforcer named.** "Treat the element list as the effect stream, and perform request firings in order" (L319) and "Answer a request on a tag it does not accept with `Unsupported`" (L321) say who they protect but not what enforces them: nothing in the runtime can; only the core test table (L1267–1283) does, and only for Worlds built on the core. Say so in the obligations section, and say that a World not built on the core is bound by the same table.
5. **Reconciliation idempotence (L81, L421)** is stated as an invariant ("a list that is the same twice means nothing twice") with no party bound and nothing enforcing it. It binds each component; the core cannot check it. Add one sentence, and a core test that reconciles the same list twice with a fake component and asserts one open.
6. **Undeclared tags and types used as if declared:** `Stored` (L1189), `PutResult` (L1194), `App.Lifecycle` (L1152), `Http.Stream` (L1058), `Net.Lines` (L1026, L1028), `Fs.Stream` (L556), `Md.Feed` / `Md.Snapshot` / `Md.Book` (L152–175), `HttpAnswer`, `ConnId`, `ReqId`, `ProcId`, `Sig`, `Code`, `Cursor`, `Size`, `Rect`, `Point`, `Style`. A reader in six months will want at least `Stored`, `PutResult`, `App.Lifecycle` and the `Md.*` three declared where they are used, since each is the point of its section.
7. **Thin sections:** "Load balancing" (L1180–1182) names `Shard.Rooms` without declaring it and says nothing about what happens to in-flight responses for a room mid-move; "Mobile" (L1152) and "A server" (L1154) are one paragraph each; "Which components are per platform" (L1131) names `Sys.Posix` / `Sys.Win32` / `Sys.Wasi` and gives no example of a `Sys.*` tag. Either give each one worked element or cut the subsection to a sentence in "Platforms".
8. **Duplication with other chapters.** "Sandboxing third-party widgets" (L1239–1243) and "A shell" (L1205–1207) restate chapter 1's sandbox and shell; keep the `@Pipe` code (new) and cut the restatement to one sentence each. "Errors as a stream" (L782–796) overlaps the errors chapter; this chapter should keep only the boundary view (what the stream's response type is, when it fires relative to the failed turn) and point to the errors chapter for the model. "Testing a World" (L1261–1289) overlaps the conformance chapter's Test World material; the core test table is this chapter's and should stay, the rest can go.
9. **Examples that would help.** (a) The Test World written as a World on the core, in twenty lines of Rust: it is the chapter's recurring proof ("a script can be a World", L95) and never appears. (b) `World.Tags` used in Prism (L1142 describes it in prose): a `dyn` over it choosing `Gui.Clipboard` or a fallback. (c) The debounce rewritten with the `Time.After` token echo, since that is the one place the token design earns its keep.
10. **The `Defer` generation rule (L247, L251) binds the program but is stated as a core rule.** Say that the program is bound (a `dyn` that rebuilds a `Defer` and fires it from `@started` on every answer is the program's loop), the core enforces it by path comparison up to `dyn` generation, and the runtime is not involved; and that the conformance suite's Test World must apply the same rule, or a program would pass the suite and fail under every real World.

## Counts

| Rule | Count |
| --- | --- |
| 1 history | 1 |
| 2 status | 1 |
| 3 attribution | 0 |
| 4 banned words | 0 |
| 5 clefts | 5 |
| 6 lecturing | 0 |
| 7 definition before motivation | 7 |
| 8 presence/drop trigger | 0 |
| 9 BIBLE conflicts | 24 |
| 10 unverified specifics | 9 |
| 11 non-Prism code | 6 |
| 12 filler | 4 |
| **Total** | **57** |

Structural notes: 10.
