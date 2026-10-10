# Lint: ch12-rust-worlds.md

Line numbers refer to `/home/claude/book/ch/ch12-rust-worlds.md`.

## Part 1: rule violations

### Rule 1: history / superseded designs (1)

- L53 "Named stages of a turn for layout, focus and painting were considered and left out; they would be a second ordering mechanism…" — "were considered and left out" narrates the design process. Fix: "A turn has no named stages for layout, focus and painting: they would be a second ordering mechanism…"

### Rule 2: status tracking (2)

- L921 "The names are sketches until the library is written; the kinds are what the language fixes, and the reasoning for each kind is below with its component." — "sketches until" is a status note. Fix: "The kinds are what the language fixes, and the reasoning for each is below with its component; the field names follow the library."
- L999 "Text is `from_utf8_lossy` for now; a `Cli.StdinBytes` tag responding with `Bytes` is the version for programs that want the raw stream…" — "for now". Fix: drop "for now": "Text is `from_utf8_lossy`; a `Cli.StdinBytes` tag…"

### Rule 3: quoting / attributing Jimmy (0)

None.

### Rule 4: banned words (1)

- L1445 "as a state element its oracle is synchronous, the component shapes the text when asked, and the slot has its metrics…" and, same paragraph, "every slot asking for the metrics of `("Start", Default)` shares one shaping" — "shapes"/"shaping". This is the typography term, but the rule's only exception is a type name. Fix: "the component lays out the glyphs when asked" / "shares one glyph layout". (Also the code comment at L1458 "shaping and glyph rasterising" → "glyph layout and rasterising".)

### Rule 5: clefts and non-default word order (16)

- L9 "This is what makes the one-way-ness of the language real at the edge: a program cannot reach the World…" — pseudo-cleft. Fix: "This makes the one-way-ness of the language real at the edge: …"
- L94 "That table is what lets the core decide, for an element nobody accepts, which of the three `Unsupported` behaviours applies." — Fix: "That table lets the core decide, for an element nobody accepts, which of the three `Unsupported` behaviours applies."
- L173 "The `oracle` method is the one that most needs its contract spelled out." — Fix: "`oracle` most needs its contract spelled out."
- L353 "Why `Defer` is a request element with a response rather than, say, a state element the World toggles, is answered by the creation rule." — fronted why-clause. Fix: "The creation rule says why `Defer` is a request element with a response and not, say, a state element the World toggles."
- L458 "What the core does is make it visible: `Diagnostic::LateResult` goes to the report…" — cleft. Fix: "The core makes it visible: …"
- L458 "What the core does not do is deliver it, because delivering a response to a path the program's list does not have is the one protocol error…" — cleft. Fix: "The core does not deliver it, because …"
- L497 "*A state tag* nobody accepts is the one that cannot be deferred to the next turn, because a state element has a value in the occurrence it is created…" — Fix: "*A state tag* nobody accepts cannot be deferred to the next turn, because …"
- L552 "The harness is what the core offers so that each of those can drive the same turn with the same four lines." — Fix: "The core offers the harness so that each of those can drive the same turn with the same four lines."
- L623 "The `Wake` trait is what connects a thread that delivered something to the loop that should call `step`: …" — Fix: "The `Wake` trait connects a thread that delivered something to the loop that should call `step`: …"
- L1042 "Why exit is a request and not a presence was argued in chapter 1 from the program's side: …" — fronted why-clause. Fix: "Chapter 1 argued from the program's side why exit is a request and not a presence: …"
- L1170 "A `std::fs::read_to_string` on a pool thread cannot be interrupted, so what cancellation means here is that the result is not delivered; …" — cleft. Fix: "…so cancellation here means only that the result is not delivered; …"
- L1273 "The design that avoids both is the one the CLI World already has: a turn is synchronous and brief; …" — Fix: "The CLI World's design avoids both: a turn is synchronous and brief; …"
- L1359 "That is what backpressure means and the bounded inbox is how a Prism World gets it for free." — Fix: "That is backpressure, and a bounded inbox gives a Prism World it for free."
- L1361 "Batching in step 2 is where "simultaneous inputs" is decided." — Fix: "Step 2's batching decides what "simultaneous inputs" means."
- L1443 "`Gui.Region` is where input enters." — Fix: "Input enters through `Gui.Region`." (Tag itself conflicts with the BIBLE; see rule 9.)
- L1591 "What differs in the World is the whole middle." — cleft. Fix: "The whole middle of the World differs."

### Rule 6: lecturing tone (2)

- L296 "Three things about the order deserve a sentence each, because each is an obligation made concrete." — section-plan scaffolding. Fix: cut; the three paragraphs that follow stand on their own.
- L653 "`has_gatherable` deserves a word." — scaffolding. Fix: cut; start at "A turn is caused by a delivery, …"

### Rule 7: dry definition before motivation (6)

- L175 "### Registering components and the World struct" opens on a bare `struct World` block. Lead with the problem: two components both claiming `Cli.Stdout`, or a World author wondering what the one struct they must build looks like; then the code; then the duplicate-tag rule at L205.
- L699 "### Execution" opens on `pub struct ScriptComponent`. Lead with the L725 sentence ("a Test World does nothing, so its component does nothing") and a one-line walk of what happens to a `Send` step before the code.
- L1003 "### `Cli.Stdout`, `Cli.Stderr`: buffered per turn, flushed at the end" opens on code. Lead with the lived case at L1032 (three lines written in one occurrence should appear together, and a pipe reader should see them at the turn's end).
- L1174 "### `Time.After`, `Time.Every`, `Clock`: deadlines in the loop's wait" opens on `pub struct Timers`. Lead with "there is no timer thread" (L1243) and the stopwatch reading the clock in the tick's occurrence (L1245).
- L1371 "### The vocabulary" opens on a 60-line tag block. Lead with the caret-placement problem at L1445 and the "label whose text changes is a fresh slot" example at L1441, then the declarations.
- L1599 "### What the World provides" opens on the `Net.Socket` declarations. Lead with one sentence: the World knows sockets and files and nothing else, and here is the only tag it needs for a display connection.

### Rule 8: presence or drop as a trigger (0)

None. Stream starts at slot creation (`Cli.Stdin`, `Cli.Signals`, `Time.Every`, `Gui.Frames`) are the only way a responds-only element can work, and the window/draw list (L1441, L1485) is description. Exit is by request throughout (name is wrong; see rule 9).

### Rule 9: design conflicts with the BIBLE Decisions (22)

- L882, L906–907, L1017, L1038 ("### `Cli.Exit`: a request, acted on after the turn"), L1040, L1042, L1044, L1253, L1265, L1870, L1873, L1883 — `Cli.Exit`. Corrected form everywhere: `Process.Exit` with `requests: !Code`.
- L881, L909–913, L923 ("### `Cli.Args` and `Cli.Env`: state, constant after start"), L929–931, L939 ("a test can set them with `State Cli.Args, ["notes.txt"]`"), L1803–1806, L1877 — `Cli.Args`/`Cli.Env`. Corrected: `Process.Args`, `Process.Env` (state); component is `process::Process`, not `cli::Process`.
- L900–904 "tag Cli.Stdout of Element: requests: !Text" (and `Cli.Stderr`), L882 table row "responds; requests; requests; requests", L1036 "A failed write (… `EPIPE`) is a diagnostic and a flag, not a response, because `Cli.Stdout` as declared does not respond. A variant tag that does, `Cli.StdoutChecked(…)`…" — the Decisions make `Cli.Stdout`/`Cli.Stderr` `requests: !Text, responds: WriteFailed`. Corrected: declare `responds: WriteFailed`; `perform` responds `WriteFailed` through `cx.respond` on `EPIPE` (next turn, creation rule); cut `Cli.StdoutChecked`.
- L1201 "// Time.After(requests: !Duration, responds: Elapsed)", L1203 `request.as_duration()`, L1228 `cx.respond(&path, elapsed())` — Decisions: `Time.After` is `requests: !Nat, responds: Nat` (the token echoed back). Corrected: the request is a `Nat` (milliseconds), stored with the deadline, and the response is that same `Nat`.
- L885 table "`Time.Every` … responds" versus L697 "`Requested Time.Every(..), ignore`", L807, L842 "requested 3/#0/1: Time.Every(period=100) <- Tick" — `Time.Every` has no `requests`, so it can never appear in a `requested` line or satisfy a `Requested` step. Corrected: use a request element in those examples (`Fs.File(..) <- Read`, `Cli.Stdout(..) <- "3\n"`), and keep `Time.Every` as `Send Time.Every(period=100), tick`.
- L1383–1385 "tag Gui.Window of Element: title: Text; state: WindowState", L1387–1402 `Gui.Rect`/`Gui.Text`/`Gui.Image` as tags, L1430–1434 `Gui.Region(rect, requests: !RegionRequest, responds: RegionEvent)`, L1437 `Gui.Frame`, L1471–1472 the component's tag list, L1626 "its child creates `Gui.Window`, `Gui.Rect`, `Gui.Text`, `Gui.Region` and `Gui.Metrics` slots" — the final GUI vocabulary is `Gui.Window` (responds: pointer, key, resize), `Gui.Scene` (presence; `draw` field is a plain list of `Draw.Rect`/`Draw.Text`/`Draw.Image`/`Draw.Clip` values), `Gui.Focusable` (name only), `Gui.Metrics` (state), `Gui.Frames` (responds `Frame(now: Instant)`). Draw commands are values, not tags. Corrected: one `Gui.Scene(draw=[Draw.Rect(..), Draw.Text(..)])` slot per window; input arrives on `Gui.Window`'s response; `Gui.Focusable(name=..)` replaces `Gui.Region`; `Gui.Frames` replaces `Gui.Frame`; the window's size and scale arrive as a `Resize` response, not a `WindowState` state.
- L1443 "It has a rect, which the World uses for pointer hit-testing (the last region in list order whose rect contains the point gets the event)", L1530–1534 `screen.hit(self.pointer, …)`, L1565 "`after_turn` is the diff: compare the new list's `Gui.*` slots against `scene` by path" (damage per primitive slot) — "The World does no geometry; the library hit-tests." Corrected: the World delivers pointer events to the `Gui.Window` slot with coordinates; the library's layout interceptor routes them to the widget under the point (as `X11.Route` at L1657 already does); damage is the whole scene or whatever the World computes from the `Draw.*` list diff.
- L1443 "focus is therefore keyed by the World on the element value plus tree position … That is a World policy" and L1506–1508 `TakeFocus` handled in `perform` — focus is a library concept (`Gui.Focusable` name-only). Corrected: the library holds focus; the World only reports key events to the window; `TakeFocus`, `Capture`, `ImeCursorArea` become library state or `Gui.Window` requests if the World needs them for IME (IME queries are World policy).
- L466 "has `Gui.Button` slots at the root", L495 "`Gui.Label` under the CLI World", L664–672 (`Gui.Label(text="Count: 0")`, `Gui.Button(label="Increment")`), L827–836 transcript, L1702–1798 conformance programs, L1879 "a `Gui.Button` at the root", L1880 "a `Gui.Window` state element at the root" — buttons and labels are library tags: `Ui.Button` (responds `Click`), `Ui.Label`. Corrected: `Ui.Button`/`Ui.Label` throughout; the L1880 case keeps `Gui.Window` but it is responds-only, so make the unsupplied-state case `Gui.Metrics`.
- L1080 "`Fs.Contents(path, state: Unavailable | Known Text)` is the state element…", L1139 `std::fs::read_to_string(&file)` with no cap, L1164 "The read in `oracle` is the synchronous read…", L884 table lacking `Fs.Watch` — Decisions: `Fs.Contents`' oracle reads synchronously with a size cap beyond which it answers `Unavailable`; `Fs.Watch` is a state element answered from a real stat at slot creation (`Missing | Modified(stamp)`). Corrected: add the cap to `oracle` and a `Fs.Watch` row/arm whose oracle stats and whose `gather` pushes the new stamp.
- L875 "There are at most four such threads: one reading stdin, one waiting on signals, a small pool for file operations, and none for timers", L879–886 table, L632–636 `main` — the CLI World includes `Net`. Corrected: add `net::Sockets` to the table and to `main`, and say where its IO runs (a tokio thread forwarding to the inbox, as L1365 describes for the desktop World).
- L306 "`Defer` is the one element the core implements itself" (and L205) — `World.Tags` is also core-provided: a state element listing accepted tags. Corrected: "The core implements two elements itself: `Defer` and `World.Tags`, whose oracle answers `by_tag.keys()`."
- L318 `answered: BTreeSet<Path>` / L337 `self.answered.insert(path.clone())` — a `Defer` reborn by a `dyn` at the same tree position counts as the same `Defer` for the once-per-turn bound, and a rebuilt slot has a fresh generation in its path (L460). Corrected: key `answered` by the path with generation segments erased (tree position), and say so in the L347 bound sentence.
- L497 "The transaction rolls back; the World reports the error with the slot's source span; the inputs that opened the occurrence are lost." and L548 "the `Err` comes back, the runtime is unchanged (the occurrence rolled back), the inputs that opened the occurrence are gone" — a failed turn drops its responses and re-queues its pushes. Corrected: "the responses that opened the occurrence are dropped and its pushes are re-queued for the next turn". The code needs it too: L247 `self.snapshot.apply_pushes(&pushes)` runs before `run_occurrence`, so on `Err` the push sits in `next`, is never in a push list again, and leaks into `prev` at the next `swap` without ever firing. Fix: apply pushes to `next` only after `Ok`, or roll `next` back to `prev` on `Err` and re-queue the push list.
- L255 `let outcome = self.run_occurrence(responses, pushes)?;` and L548 "the caller decides whether to log and continue, or exit" — an occurrence aborted inside a turn stops the rest of the turn *and the World acts on what committed*. The `?` returns before step 4, so requests from earlier, committed occurrences of the same turn are never performed. Corrected: on `Err`, break out of the loop with the records so far, run the slot diff and performs for them, then return the error alongside (e.g. `TurnOutcome { error: Some(e), .. }`).
- L506–510 `WorldError { Runtime, DeferBackwards, DuplicateTag }`, L497 and L1805 `RuntimeError::UnsuppliedState { element, span }` — one type `RuntimeError { kind: ErrorKind, spans, occurrence, … }` with `ErrorKind` a closed choice (`Cycle`, `Overflow`, `UnknownPath`, `UnsuppliedState`, `DeferBackwards`, `Component`, `Build`). Corrected: `DeferBackwards` is an `ErrorKind`, not a separate `WorldError` variant; match as `RuntimeError { kind: ErrorKind::UnsuppliedState, .. }`; L539's header is consistent and stays.
- L24–32 `pub struct Outcome { list, requests, result }` — `Outcome` has `errors: Vec<RuntimeError>` for contained build failures (a failed `dyn` build does not fire the `dyn`'s event and is reported through `Errors`). Corrected: add the field, and have `OccurrenceRecord` (L217) and the transcript (L821) show it.
- L14–22 `pub struct Inputs { responses, pushes, oracle }` — quarantine is a `quarantine: [Path]` input the World may send on the next run call. Corrected: add `pub quarantine: Vec<Path>` and a sentence under error reporting on when a World sends it.
- L546 "Diagnostics are printed in a shorter form…" and L548 "What the World does with an error is the World's policy" — the World offers `Errors` (responds: `Error`) as a stream element programs subscribe to, with `Error { kind: ErrorKind, spans, occurrence }`. The chapter never provides it. Corrected: a core-provided `Errors` element (like `Defer` and `World.Tags`) that `turn` responds to on the next turn with every error and contained build failure.
- L516 `Component { tag: Tag, message: String }` as a `Diagnostic`, used at L963, L1013, L1141 — a World component crash is a `Component` *error* (kind `Component`), never a diagnostic or an `Unsupported` response; a wrong-typed or duplicate response from the World is also kind `Component`. Corrected: keep `Diagnostic::Component` only for non-failures (the second-stdin note); a watch that cannot be started or a component panic goes out as `RuntimeError { kind: Component, .. }`, and `sort_inbound` (L421) checks response types against the tag table and raises `Component` on a mismatch.
- L678–692 `enum Step` — steps `Unrequested(Pattern)` and `ExpectTopNone(Pattern)` exist in the suite. Corrected: add both variants and their arms at L750–780 (`Unrequested` passes when no firing in the last turn matches; `ExpectTopNone` when no root slot matches).
- L704 `defaults: TagTable, // tags' declared default state values`, L715 `self.defaults.default_state(&e.tag)`, L716 "no State step and no declared default", L725 "no `State` step and no declared default" — there is no `default:` in tag declarations; `State` steps may precede the build and must for any state tag the program creates at build. Corrected: drop `defaults`; the oracle answers from `snapshot` or returns `Value::missing()`.
- L1208 `Duration::from_millis(el.field("period").as_int() as u64)` — `period` is a `Nat` (64-bit unsigned). Corrected: `as_nat()`.

### Rule 10: unverified specifics (9)

None of these appear in `/home/claude/book/intro/research-notes.md` or the Reflex sources; verify or mark from memory or cut.

- L160–162, L623, L1365: winit `EventLoopProxy::send_event`; L1524–1550 `ApplicationHandler<Delivery>`, `window_event`, `user_event`, `ActiveEventLoop::exit`, `WindowEvent::{CursorMoved, MouseInput, KeyboardInput, Ime, Resized, RedrawRequested, CloseRequested}`; L1510 `set_ime_cursor_area`, L1512 `set_cursor`.
- L1451, L1457: `softbuffer` and `softbuffer::Surface`.
- L1088–1089, L1123, L1140: `notify::RecommendedWatcher`, `notify::Event` with `.paths`, `.watch(_, NonRecursive)`, `.unwatch`.
- L1051–1066: `signal_hook::iterator::Signals::new([..])`, `.handle()`, `.forever()`, `Handle::close()` "restores the OS default".
- L1085: `threadpool::ThreadPool` with `.execute`.
- L162, L1283, L1330, L1359: `tokio::sync::Notify`, `tokio::select!`, `TcpStream::into_split`, `blocking_send`.
- L1036 "`SIGPIPE` is left at the OS default of being ignored, so that the failed write is seen as `EPIPE`" — the OS default for `SIGPIPE` is to terminate the process; Rust's standard runtime sets it to ignored at startup. Fix: "`SIGPIPE` is left as the Rust runtime leaves it, ignored, so that…" (from memory; verify).
- L1631, L1633: `~/.Xauthority` and `/tmp/.X11-unix/X0` as the authority file and socket path. Mark from memory.
- L1451 "wgpu is a drawing backend with no language consequence" — crate name from memory; mark or cut.

### Rule 11: code that cannot be Prism (6)

- L915 `type Signal = choice Interrupt | Terminate | Hangup | Other Int` — `|` is leftmost merge; choices elsewhere are block form. Fix: `type Signal = choice:` with one constructor per line.
- L1653 `ignore: @Element e` inside `supply`, whose type is `fn Element -> @$State` — `@Element` on an unknown tag returns `@(!Response, $State)`, not `@$State`. Fix: `ignore: @widget: (ignore, $s) <- @Element e; @ $s` (or whatever the book's block spelling is).
- L666–671 `Send Gui.Button(label="Increment"), Click` and L1709, L1728, L1752 `click()` naming — unit-like responses are lowercase values (`click`). Fix: `Send Ui.Button(label="Increment"), click` (Rust helpers can stay `click()`).
- L831, L838, L845, L854 transcript `response 0/#0/0 Click` — Fix: `click`.
- L842 `<- Tick`, L1815 and L1887 `Send Time.Every(..), Tick` — Fix: `tick`; and see rule 9 on `Time.Every` never being `requested`.
- L1705 and L1772 `@Element Gui.Label(text=$text)` as an unbound statement line inside a block — every other widget use in the book binds its result (`ignore <- @Element …`). Fix: `ignore <- @Element Ui.Label(text=$text)`, or confirm the bare-statement form is grammar.

### Rule 12: filler (4)

- L167 "Two alternatives are worth holding up against it." — Fix: cut; go straight to "Suppose a component were an async trait…"
- L1164 "The read in `oracle` is the synchronous read discussed under the trait, and it is the whole reason…" — back-reference scaffolding. Fix: "The synchronous read in `oracle` is the whole reason…"
- L1273 "The wrong design is seductive, so it is worth seeing why it is wrong." — Fix: cut; start at "Suppose the turn loop were an `async fn`…"
- L1844 "Each of these is a sentence from the specification, and the body is short enough to read in the failure message." — restates L1671. Fix: cut the first clause.

## Part 2: structural notes

1. **The GUI World section (L1367–1569) is built on a vocabulary the Decisions replaced.** Beyond the tag renames in rule 9, the whole middle (per-primitive slots diffed by path for damage, `Gui.Region` hit-testing and focus in the World, `WindowState` as state) has to be rewritten around one `Gui.Scene` slot per window holding a `Draw.*` list, pointer/key/resize responses on `Gui.Window`, and library-side hit-testing. The X11 section (L1626, L1657) and the Test World depth rule (L797) lean on the same vocabulary and need the same pass. The native-toolkit subsection (L1571–1593) survives as is.
2. **The error path is missing its program-visible half.** The chapter has `WorldError`/`Diagnostic`/`Report` but no `Errors` element, no `Outcome.errors`, no quarantine input, and no statement of how a contained `dyn` build failure reaches the program. One subsection "What the program sees" after L548, with the `Errors` element declared in Prism and a two-turn example (a cycle, then the program reading `Error(kind=Cycle, ..)`), would close it.
3. **Who is bound and what enforces it, for the turn's obligations.** L296–302 state three ordering rules (removals before creations, performs after the diff, pushes land once per turn) but say only that the core does them. Say per rule: bound party is the core; enforced by the loop's order of operations at L268–276; the Test World's transcript is what a wrong order shows up in; a component can break the "pushes once per turn" rule by pushing from `perform`, and nothing stops it, so say that `Cx::push` outside `gather` is held to the next turn (or refused).
4. **`has_gatherable` and `quiescent` are two per-component predicates with no trait method for the first.** L602 calls `self.world.has_gatherable()` and L653 explains it, but the `Component` trait at L101–131 has no such method, and L1166 says `Files` "reports `true`". Add `fn wants_turn(&self) -> bool` to the trait, and a table (component × {keeps alive, causes a turn}) summarising L1253–1255, so the judgment calls are in one place.
5. **The failed-turn path is not shown in code.** L255's `?` and L247's eager `apply_pushes` do not do what L548 and the Decisions say (act on what committed; re-queue pushes). Either fix the listing or add a short "the failing turn" listing after L302 that shows rollback of `snapshot.next`, the re-queue, and the partial perform.
6. **`Process.Spawn` / `Process.Child(id)` are absent from the CLI World.** They are general components like `Fs`/`Time`/`Net`; the chapter's component table (L879) and quiescence conjunction (L1253) should include a child process (kills under cancel-on-teardown; keeps the run alive while running) and the Decisions' "no `detach`".
7. **The Test World section duplicates chapter 1 and the suite chapter.** L801–817 ("How `State`, `Push`, `Requested` and `Errors` are implemented") re-explains step semantics the suite chapter owns; keep here only the one-line "which field of `TurnOutcome` each step reads" and point at the suite chapter for semantics. L805's rebuild walkthrough duplicates L399.
8. **The conformance test section (L1669–1846) is half stubs.** Eight of the thirteen tests are a comment. For a reference that will be executed by a model, each needs at least the program and the one assertion; the four worth writing out fully are `request_firings_are_performed_in_occurrence_order_then_list_order`, `a_request_from_a_slot_torn_down…`, `two_responses_to_one_path…`, and `an_aborted_occurrence_leaves_the_list_and_the_snapshot_as_they_were`, since each pins a rule the code in Part 1 currently gets wrong or does not show.
9. **The golden-transcript runner's `ready` handshake (L1889) is stated without saying who enforces per-turn flush.** It depends on `Stdio::gather` flushing (L1022–1027) *and* on flush before `wait` blocks, which the loop at L639–647 does not do: `gather` runs at the start of the next turn, so the last turn's output is not flushed until something else arrives. Either flush at the end of `turn` (an `after_turn` hook) or in `Harness::wait` before blocking; then say the handshake is sound because of that line.
10. **An example would help at the harness (L550–624).** Show one concrete trace: three deliveries queued (two responses to one path, one push) under `Drain`, what `step` returns, what is held, what the next `step` returns. The same trace under `OneAtATime`. It is the one place a World author will make a mistake and there is no worked case.

## Counts

| Rule | Count |
| --- | --- |
| 1 History | 1 |
| 2 Status | 2 |
| 3 Attribution | 0 |
| 4 Banned words | 1 |
| 5 Clefts / word order | 16 |
| 6 Lecturing | 2 |
| 7 Dry definition first | 6 |
| 8 Presence/drop trigger | 0 |
| 9 BIBLE conflicts | 22 |
| 10 Unverified specifics | 9 |
| 11 Non-Prism code | 6 |
| 12 Filler | 4 |
| Structural notes | 10 |
