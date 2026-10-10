# Lint: ch16-appendices.md

Chapter: /home/claude/book/ch/ch16-appendices.md (523 lines). Line numbers below are from that file.

Counts per rule: R1 history/process 7 · R2 status 1 · R3 attribution 1 · R4 banned words 0 · R5 clefts 5 · R6 lecturing 0 · R7 definition-first 0 (see note) · R8 presence as trigger 2 · R9 Decision conflicts 27 · R10 unverified specifics 14 groups · R11 non-Prism code 4 · R12 filler 0.

## Part 1: rule violations

### Rule 1: history / superseded designs / process references

The chapter refers to the writing process ("this session", "fetched", "re-read") as if the reader had been there. The reader has no history; a Verified/From memory split is fine, but the sentences that say when and by whom the fetch happened are process notes.

- L3: "Appendix B lists the prior art, one entry per system, with what each contributed and which claim about it was checked against a source rather than remembered." Rule 1 (process). Fix: "…with what each contributed and which claims are quoted from a source."
- L95: "Each entry says what the system contributed to the design and which claim about it was checked against a source in this session." Rule 1. Fix: "Each entry says what the system contributed and quotes the source for each claim that has one; a claim marked *from memory* has no quoted source and should be read as general knowledge."
- L122: "*From memory:* `switchPromptlyDyn`, `fanMap`, `select`, and `listWithKeyShallowDiff` are as described in the correspondence table; these names were not re-read this session." Rule 1. Fix: drop the clause after the semicolon.
- L157: "*From memory:* the React Compiler's job as automatic memoization that replaces hand-written dependency arrays; no page about it was fetched this session." Rule 1. Fix: drop "; no page about it was fetched this session".
- L163: "*From memory, entirely.* Nothing about Solid, Svelte 5, Vue or Angular signals was fetched this session." Rule 1. Fix: "*From memory, entirely.*" and stop. (Also see R10: research-notes.md §4.4 does hold verified Solid and TC39 quotes, so the entry can be upgraded instead.)
- L175: "The OCaml manual's chapter on extensible variant types was not fetched this session. What was read from OCaml this session (the runtime's `Is_long` tagging in `mlvalues.h` and the `bytegen.mli` header) is about values and bytecode, under the compiler entry below, not about variants." Rule 1. Fix: "*From memory.* The OCaml manual's chapter on extensible variant types; the OCaml sources quoted under the compiler entry are about values and bytecode, not variants."
- L203: "AccessKit (https://github.com/AccessKit/accesskit) and wasi-http (https://github.com/WebAssembly/wasi-http) were read for the Worlds chapter's planning." Rule 1 (process). Fix: say what each contributes ("AccessKit, for what an accessibility tree wants from a draw list; wasi-http, for a request/response boundary with no callbacks") or cut.

### Rule 2: status tracking

- L47 (table): "removed for now; a `for` over a map is the patch". Rule 2 ("for now" is status). Fix: "removed; a `for` over a map is the patch. A patch type would return if `for` over a large map proved too slow to diff by key."

### Rule 3: attribution

- L53: "In every case met so far that is the program the author meant and the sample was a shortcut past it." "The author" reads as Jimmy and "met so far" is process. Fix: "In every such case the `dyn` or the tag is the program the programmer meant and the sample was a shortcut past it."

### Rule 4: banned words

None. ("Composable Memory Transactions" at L134 is a paper title.)

### Rule 5: clefts and non-default word order

- L51: "What Prism removes is the choice." Fix: "Prism removes the choice."
- L59: "What this gives up is the ability to write a function that is polymorphic over whether its argument is a dynamic; Reflex has that through the `Functor` instances." Fix: "This gives up functions polymorphic over whether their argument is a dynamic, which Reflex has through the `Functor` instances."
- L79: "What Reflex can do and Prism cannot is express a query that is a group with a `crop`, so that the interest set is patched rather than re-sent;" Fix: "Reflex can express a query as a group with a `crop`, so the interest set is patched rather than re-sent; Prism cannot:"
- L91: "What `for` adds is consolidation by result kind and promptness of the consolidated event." Fix: "`for` adds consolidation by result kind, and the consolidated event is prompt."
- L191: "What survived into the plan is the closure compiler over the typed AST and the principle that the product of running Prism code is a graph." Fix: "Two things survived into the plan: the closure compiler over the typed AST, and the principle that running Prism code produces a graph."

### Rule 6: lecturing tone

None.

### Rule 7: dry definition before motivation

Appendices C and D open with tables and definitions by design (the lead at L3 says they are for a reader who knows the design). Not flagged. Appendix A's sections each open with the lived problem. Fine.

### Rule 8: presence or drop as a trigger

- L495 (`Process.Child` row): "A child process started when the slot exists and killed when it is torn down." The start is a side effect of an element appearing. Decision: `Process.Spawn` is a request; `Process.Child(id)` is state keyed by id; the kill is the World's cancel-on-teardown obligation. Fix: replace with `Process.Spawn` (requests `!(Text, [Text])` or a `SpawnRequest`, responds with the id) and `Process.Child` (`id: Nat`, state `ProcessState`, requests `!ProcessRequest`, responds `ProcessEvent`): "Spawning is a request; the child's existence is state keyed by id, and the World kills it under its cancel-on-teardown obligation when no slot names the id."
- L496 (`Process.Run` row): "Run-to-completion form: each firing (command, args, stdin) starts one process; its output and exit come back as responses." Not itself a presence trigger (it is a request) but it is a second spawn vocabulary not in the Decisions; it duplicates `Process.Spawn`. Fix: cut, or define it as a library widget over `Process.Spawn` + `Process.Child`.

### Rule 9: conflicts with the BIBLE Decisions

Appendix C (syntax):

- L221: "Literals: integers and floats (polymorphic over the numeric types, resolved by inference)". Decision: integer-spelled literals are `Nat | Int` by inference, decimal-spelled are `Float`; `-1` is `Int`. Fix: "Literals: an integer-spelled literal is `Nat` or `Int` by inference (`-1` is `Int`; a `Nat` is pinned by annotation); a decimal-spelled literal is `Float`."
- L221: "Keywords: `let type fn match if else read update for dyn intercept widget record choice map array unit test ignore on key ix jx kx`." `tag` and `of` are used as keywords at L291 and are missing; `read`/`update` are future work and should say so. Fix: add `tag of`, and mark `read update` "(reserved; future work)".
- L221: "`never started main Switch Leftmost Join Filter Element` are prelude names written with their sigils." Decisions add `!Updated`, `$Distinct`, `Kind`, `Wrap.*`, `Checked.*`. Fix: list them: "`!Updated: fn $a -> !a`, `$Distinct` (needs equality), `Kind: fn Element -> Kind`, `Wrap.*`/`Checked.*` arithmetic."
- L248 (operator table, level 6): "`+ -` and `+~ -~` (Int), `+. -.` (Float) | suffixed spellings only when inference cannot decide". Consistent, but the table should say there are no suffixed `Nat` operators, since the Lexical paragraph implies the literal is polymorphic over all three. Fix: add "no suffixed `Nat` spellings; `Nat` is pinned by annotation".
- L298: "`@Element e` returns `@unit`, `@!R`, `@$S` or `@(!R, $S)` by the tag's parts." Decision: a tag with requests and state but no responds returns `@$S` only. Fix: "…`@unit` (fields only, or requests only), `@!R` (responds), `@$S` (state, with or without requests), `@(!R, $S)` (responds and state)."
- L298 and the whole Tag declarations section: element construction never shows the `requests=` spelling (`Cli.Stdout(requests=!line)`), nor that the request event is excluded from element equality. Decision: both. Fix: add one line: "Construction spells the request event as a named field: `Cli.Stdout(requests=!line)`; it is excluded from equality, so two slots differing only in their request events are equal elements."
- L315–331 (test script table): missing `Unrequested pattern` and `ExpectTopNone pattern`. Decision: both exist. Fix: add rows "`ExpectTopNone pattern  # no root-level slot matches`" and "`Unrequested pattern  # no matching slot fired its request in the last turn`".
- L324 and L333: "`State pattern, value  # set the snapshot for a state element; next turn's oracle and push`" and "The build is turn zero." Decision: `State` steps may precede the build and must for any state tag the program creates at build (no `default:` in tags). Fix: "`State` may appear before the build and must for every state tag the program creates at build, since the oracle has to answer in occurrence zero."
- L326: "`Returns pattern  # the widget's result matches (a dynamic by current value)`". Decision: `Returns` on an event result is a script error. Fix: append "; on an event result, a script error (use `Fired`/`Quiet`)".
- L303 (`intercept` signature) is correct; the paragraph at L310 should also state the Decision that a new child slot whose element value already has a live supplied widget joins it regardless of the current supplier dynamic. Fix: add that sentence after "once per distinct element value".

Appendix D (vocabulary):

- L443: "A World that knows about screens accepts the *primitive* rows; the *library* rows are what a Prism GUI library makes out of them and are listed because the examples use them. Under a World that accepts only the primitives, `Gui.Button` is an interceptor's business". Decision: library tags are `Ui.*`, primitives are `Gui.*`. Fix: "`Ui.Button` is an interceptor's business"; and rename every library row to `Ui.*`.
- L447 (`Gui.Window`): fields `title, width, height`, responds "`Resized Nat, Nat \| Closed`", "Its children are draw commands and focusable regions under an interceptor, not fields." Decision: `Gui.Window` responds with pointer, key and resize events; the World does no geometry. Fix: responds `Pointer Pointer \| Key Key \| Resized Nat, Nat \| Closed` (pointer and key arrive at the window; the library hit-tests and routes).
- L448–451 (`Gui.Rect`, `Gui.Text`, `Gui.Image`, `Gui.Clip` as tags). Decision: draw commands are values, not tags: `Draw.Rect`, `Draw.Text`, `Draw.Image`, `Draw.Clip`, held in `Gui.Scene`'s `draw` field (a plain list). Fix: delete the four rows; add `Gui.Scene` (fields `window: Text`? or by window path, `draw: [Draw]`; presence; identity path; "the draw list for a window, as data, read after each turn") and declare `type Draw = choice: Rect Rect, Text; Text Rect, Text, Text; Image Rect, Text; Clip Rect, [Draw]` in the value-types block.
- L452 (`Gui.Focusable`): fields "`name: Text`, `rect: Rect`", responds "`Key \| Pressed Pointer \| Released Pointer \| Moved Pointer \| Focused \| Blurred`", "pointer events when hit". Decision: name only; focus is a library concept; no geometry in the World. Fix: fields `name: Text`; responds `Key \| Focused \| Blurred`; "Pointer events arrive at `Gui.Window`; the library hit-tests against its own layout and routes them."
- L453 (`Gui.Frames`): responds `Tick`. Decision: responds `Frame(now: Instant)`. Fix: responds `Frame Instant`, "one per displayed frame, with the frame's time".
- Missing `Gui.Metrics` (state: text measurement/fonts, answered synchronously). Fix: add the row.
- L454–457 (`Gui.Label`, `Gui.Button`, `Gui.TextInput`, `Gui.VBox`). Decision: `Ui.Label`, `Ui.Button`, `Ui.TextInput`, `Ui.VBox`. Fix: rename. L455 "responds with `click` after the occurrence of the press" → responds `Click` (value `click`).
- L457 (`Gui.VBox`): responds "`ChildInput Path, Pointer`". Decision: the suite's VBox responds `ChildInput(path, response)`. Fix: `ChildInput Path, Response` with "routes any child-addressed response inward by path".
- L459: "A `Gui.Rect` appearing is a description the World draws; a `Gui.Button` disappearing stops nothing but the delivery of clicks". Fix: "A `Gui.Scene` appearing is a description the World draws; a `Ui.Button` disappearing…".
- L468 (`Fs.Watch`): responds `Change`, identity path, "Each change after the slot exists, nothing at start." Decision: `Fs.Watch` is a state element holding a modification stamp, answered from a real stat at creation (`Missing | Modified(stamp)`), no `Unknown`. Fix: state `Stamp` (`type Stamp = choice: Missing; Modified Instant`), identity value, "The file's modification stamp as state: a real stat answers the oracle at creation; pushed when the World sees a change. `Overflow` is not needed: a program re-reads when the stamp changes." Then L470 ("`Fs.Contents` and `Fs.Watch` are different kinds on purpose: one is a level… the other is an edge stream") must go, and `type Change` at L391–395 becomes unused.
- L477 (`Time.After`): "`!Duration` | `Instant`". Decision: `requests: !Nat, responds: Nat` (a token echoed back). Fix: "`!Nat` | `Nat` | path | Each firing asks for a wake-up; the response echoes the token so a program with several timers in flight can tell them apart. The delay is the token's meaning only by convention of the program; the World treats it as milliseconds."  (If the Decision's `Nat` is the delay itself, say "the request is the delay in milliseconds and the response echoes it".)
- L478 (`Time.Every`): add the Decision "missed ticks are skipped (one `tick` per turn at most, never a backlog)".
- L495–496: `Process.Child` with `command`/`args` fields and presence-started; see R8. Decision: `Process.Spawn` (request) + `Process.Child(id)` (state keyed by id). Also "`Process.Child` has no `detach` field" (already true).
- L500 and L504–505: "`Args` and `Env` are not under `Cli` for the same reason `Clock` is not" and the bare rows `Args`, `Env`. Decision: `Process.Args` and `Process.Env`. Fix: rename rows; rewrite L500 as "`Process.Args` and `Process.Env` are not under `Cli` because every program has them, a GUI program included, and a sandbox wants to give a subtree different ones."
- L507–508 (`Cli.Stdout`, `Cli.Stderr`): no responds. Decision: `requests: !Text, responds: WriteFailed`. Fix: add responds `WriteFailed` ("a write the World could not complete, in a later occurrence") and declare `type WriteFailed = choice: Closed; Failed Text` (or as chapter 6 spells it).
- L509 (`Cli.Exit`): Decision and BIBLE voice rule 11: exit is `Process.Exit` with `requests: !Code`. Fix: move the row to the `Process.*` table as `Process.Exit`; add "two firings in one turn: the first in occurrence-then-list order wins".
- Missing `World.Tags`: Decision: a core-provided state element listing accepted tags. Fix: add to the "Every World" table: "`World.Tags` | | | | `[Text]` | value | The tags this World accepts natively, as state; what a library consults before deciding which interceptors to install."
- Missing `Store.*`: Decision names it as the persistence vocabulary as chapter 6 declares it. Fix: add a short table or a one-line pointer: "`Store.*` is chapter 6's persistence vocabulary and is listed there."
- Missing `Error`/`ErrorKind` declarations: L517 (`Errors` responds `Error`) refers to a type the value-types block never declares. Decision: `Error { kind: ErrorKind, spans, occurrence }`, `ErrorKind` closed: `Cycle, Overflow, UnknownPath, UnsuppliedState, DeferBackwards, Component, Build`. Fix: declare both in the block at L343–439.
- L516 (`Defer`): add the Decision "a `Defer` reborn by a `dyn` at the same tree position counts as the same `Defer` for the once-per-turn bound; independent `Defer`s fired in one occurrence are answered together."
- L519: "`Unsupported` is not a tag; it is the response a World gives, in the next turn, to a request on a tag it does not accept". Consistent; but add the Decision "a World component crash is a `Component` error, never an `Unsupported` response", since the sentence invites the confusion.
- L339: "*Identity* is by path for responses and by element value for state: two slots with equal elements observe one state." Decision: element equality is tag plus plain fields, excluding any request event. Fix: append "; equality is the tag and the plain fields, never the request event."

Appendix A:

- L26 (table): "(none) | `switchHoldPromptly`, `switchPromptlyDyn`, `coincidence` | removed". Decision: the Haskell evaluator uses one `coincidence` over the membership patch when a `for` is flagged "can fire at birth". The Prism surface has none, so the row stands, but the `for` row (L31) should say it. Fix L31: "keyed, prompt consolidation (the Haskell implementation's one `coincidence`)".

### Rule 10: unverified specifics

The lint's standard is /home/claude/book/intro/research-notes.md plus the Reflex sources under /tmp/claude-0/rx/ (`Adj.hs`, `Basic.hs`, `Class.hs` [= PostBuild/Class.hs], `Network.hs`, `PostBuild.hs`, `Query.hs`, `QueryBase.hs`, `Requester.hs`, `Spider.hs`, `Xhr.hs`). Appendix B labels nearly everything **Verified**, but most of it is in neither place. What is covered: Elliott's push-pull abstract and the Hackage `reactive` line (notes §2.2); Reflex `Class.hs` Behavior/Event/Dynamic/hold/tag/leftmost/switchHold ("absolutely sure") and PostBuild/Requester/Adjustable/networkView/networkHold (notes §3 and rx/); `queryDyn`, `askQueryResult`, `runQueryT` signature (rx/Query.hs, QueryBase.hs); `requesting` and "later time than" (rx/Requester.hs); `subscribeAndRead` signature, "hold cannot examine its event until after the phase is over", "causality loop detected" (rx/Spider.hs); `dyn = networkView`, `widgetHold = networkHold` (rx/Basic.hs); `useSyncExternalStore` and Suspense page quotes in part (notes §4.3); RxJS BehaviorSubject (notes §4.6); Kafka stream/table duality via Confluent docs, not the GitHub md (notes §4.6); Esterel via Edwards' notes and embedded.com, not Wikipedia/the Primer (notes §4.8). Everything else below: verify, mark *from memory*, or cut.

- L101: "original at http://conal.net/papers/push-pull-frp/, read through a mirror at https://dcmpx.remotevs.com/… because the original was unreachable. … `data Reactive a = a ‘Stepper‘ Event a`; a reactive value's meaning "is a step function, which is fully defined by its initial value and discrete changes"; "reactive values can be evaluated in push style, leaving pull for time functions"; "continuous time is one of the reasons for choosing pull-based evaluation"." The abstract and Hackage line are in the notes; the mirror URL and the four body quotes are not. Also drop "because the original was unreachable" (R1 process).
- L107: "(version 0.10.0.0 per reflex.cabal)" and the file list including `Spider/Internal.hs`, `Adjustable/Class.hs`, `Network.hs`, `Widget/Basic.hs`: version not verifiable; files are in rx/ under other names. Fix: drop the version or mark from memory.
- L109: "the `Reflex` class requires `Monad (Dynamic t)`, so `join` on dynamics exists." Not in rx/ (rx/Class.hs is PostBuild) nor the notes. Verify or mark.
- L113: "The returned Event of network results fires at post-build time and when the 'Dynamic' updates" is in rx/Network.hs: fine. L114 "adjust itself so that its net effect is as though it had originally been run with the new value" is in rx/Adj.hs: fine.
- L115: "with the result holding the occurrence value if the event already fired in the frame" is a paraphrase of `SubscribeResult s occ`: fine as reading, but say "(a reading of `SubscribeResult`)".
- L116: `listWithKey` full signature with constraints, `simpleList` keyed by index, `Collection.hs` URL. Not in rx/ or notes.
- L117: `fireEventsAndRead` signature, `Host/Class.hs`; `PerformEvent/Class.hs` quote; `newTriggerEvent` signature. Not in rx/ or notes. L83's paraphrase of the PerformEvent doc ("will generally occur later than its input since most actions cannot be performed during event propagation") depends on it.
- L118: `DynamicWriter/Class.hs` quote. Not found.
- L119: readthedocs `RecursiveDo` and `networkView`/`networkHold` quotes; README "fully-deterministic, higher-order…". Not found.
- L120: reflex-dom `Builder/Immediate.hs`: `holdDyn v0 $ leftmost [valueChangedBySetValue, valueChangedByUI]`, `v0 <- Input.getValue domInputElement`, "We get the value after setting it in case the browser has mucked with it somehow". Not found (rx/Xhr.hs is the only reflex-dom file present besides Basic.hs).
- L128 (Esterel/Lustre/Lucid Synchrone): the Wikipedia quotes, the Berry Primer quotes ("perfectly synchronous or zero delay model", "one and only one status", "causality cycle", "consistent way"), Esterel-in-Racket, Lustre `pre`, Lucid Synchrone `when`/clock/`current` quotes and URL. None in the notes (which hold Edwards' lecture notes and Palshikar). Verify, or swap in the notes' quotes.
- L134 (STM): `Control.Monad.STM` `retry` doc; PostgreSQL Repeatable Read quotes and `mvcc.sgml`; Datomic quotes. None in the notes.
- L142–151 (edge and level): every quote (epoll, kqueue `EV_CLEAR`, Wikipedia interrupts, genericirq `handle_edge_irq`, inotify `IN_Q_OVERFLOW`, ZooKeeper watch guarantees and zxid, Kubernetes "Level driven, not edge driven", `410 Gone`, "5 minutes", Reflector, API conventions, Debezium steps 2–3, Kafka core-concepts md, Binance `U`/`u`/`lastUpdateId` and "5000 levels", CME tag 369, Coinbase, FIX tag 263 and MsgType V). None in the notes. This is the entry the design leans on hardest, so verification matters more here than elsewhere; the numbers ("5 minutes", "5000", "369", "263") in particular.
- L157 (React): "React will restart the update from scratch … to ensure that every component on screen is reflecting the same version of the store", "React does not preserve any state for renders that got suspended…retry rendering the suspended tree from scratch" are in the notes (§4.3, §5). `getSnapshot` immutability/"same value" quotes, `ReactFiberThenable.js` `SuspenseException` text, Strict Mode "calls each component's function twice" are not (the notes mention Strict Mode only in passing). "The page does not use the word "tearing"" conflicts with notes §1.2, which has tearing quotes from react.dev-adjacent sources; check which page.
- L169 (RxJS): "an event stream of birthdays is a Subject, but the stream of a person's age would be a BehaviorSubject." Not in the notes (which have the "current value" line via learnrxjs). Verify.
- L181 (effects): Plotkin–Pretnar abstract line, OCaml 5.2 effects manual quotes, Koka book "tail-resumptive", Koka README. None in the notes.
- L187 (GHC): `-dcore-lint`, `-ddump-simpl`, `-ddump-bcos`, GHCi "roughly 10x faster", recursive_do quotes, STG notes "variant of administrative normal form". None in the notes.
- L193 (compilers): Lua 5.0 jucs05 quotes ("more than twice as fast"), WebAssembly rationale quotes, `bytegen.mli`, `closure.mli`, Real World OCaml, Cranelift README "four backends", cranelift-frontend, Crafting Interpreters "fundamentally the wrong design", Feeley–Lapalme 1987 citation (12(1):47–66), Rustlantis, LLVM DeveloperPolicy "not backwards compatible". None in the notes.
- L199 (Squeak): OOPSLA paper quotes and URL. Not in the notes.
- L203 (also consulted): Incremental `incremental_intf.ml` quotes, differential dataflow `advance_to` quote, Verilog/Nix/Bazel URLs, winit "must be created on the main thread". None in the notes.
- L7 and L89: "you spent an afternoon finding out which was right" / "the experience of every Reflex program with a list of identified items is that `simpleList` was the first thing tried" are rhetoric, not specifics; fine.

### Rule 11: code that cannot be Prism

- L509 (table): "`Cli.Exit` | | `!Code`". BIBLE: `Process.Exit`. Fix as under R9.
- L413: "`Answer Nat, Text ==> Text, Text        # status, headers, body`" and L437/L496 similar multi-payload alternatives (`Received Text` is fine; `Answer` with three payloads is not), against L272–274 "payload alternatives are newtypes" (one payload). Fix: either state in C that an alternative takes a tuple of payloads (`Answer (Nat, Text ==> Text, Text)`) or make `Answer` a record alternative; be consistent in C's Types section.
- L284: "`type TextInput = record: !changed: Text; $value: Text`". `;` as an inline field separator is not in the Lexical section (inline `:` opens an inline block of the line; nothing says `;` separates). Fix: write it on two lines, or document `;`.
- L237: "`@Button: fn $Text -> @!unit = \@widget: ...`" and L303 "`intercept: fn (@a, …)`" use `name: type = value` for a top-level definition while the Statements table uses `let pattern = expr`. Fix: say in C which spelling a top-level definition takes (the BIBLE: "top-level annotations required", form unspecified) and use one consistently.

### Rule 12: filler

None.

## Part 2: structural notes

1. **Appendix D will be read as the canonical tag list, and it is the chapter most out of date with the Decisions.** Every GUI row, `Fs.Watch`, `Time.After`, `Process.*`, `Args`/`Env`, `Cli.Exit`, `Cli.Stdout`/`Stderr`, and the missing `World.Tags`, `Gui.Scene`, `Gui.Metrics`, `Store.*`, `Error`/`ErrorKind` (all listed under R9). Because the lead at L3 promises "the names stay the same from chapter to chapter", a wrong row here propagates. Rebuild D from the Decisions list before anything else in this chapter.
2. **Appendix B's "Verified" label is unearned for most entries** (R10). The honest structure is three tiers: quoted from a source on disk (notes or rx/), quoted from a source with URL (fetched, but no local copy), and from memory. Say which tier each quote is in, or collapse to "quoted" and "from memory" and move the URLs to a sources list. Also drop the per-entry "this session" prose (R1).
3. **Appendix A's paragraphs duplicate chapter material.** "The behavior half is hidden" (L49–53) repeats the no-`current` argument of the dynamics chapter; "Only one switch" (L61–65) repeats the switching chapter; "There is no PostBuild" (L67–71) repeats `@started`'s definition; "`listWithKey` is `for`" repeats the `for` chapter. For a reference, keep the table and cut each paragraph to the one sentence that names the Reflex construct and the one that says what Prism does instead; put the argument in the owning chapter and point at it by name.
4. **Appendix C is missing syntax the Decisions fix:** `requests=` construction and its exclusion from equality; the `else`-terminates-inline-block and no-guards rules; "a trailing lambda as the last argument absorbs the rest of the line"; "interpolating an event in a text literal lifts"; "field access and indexing lift like application"; `+?`'s type; `Int`/`Nat` 64-bit under Primitives; `Kind` record; `$Distinct`; `!Updated`. Each is one line in the right table.
5. **Appendix C's test-script section does not say who runs it or what a path looks like.** Add the canonical path spelling (`0/#1/0`, `[k]`, `s/n`) since `Expect`/`Send` patterns and transcripts use it, and the Decision that `State` precedes the build for state tags created at build.
6. **Invariants in D without an enforcer.** L339 "nothing fires in the occurrence a slot is created" (enforced by the runtime, which has no response to deliver in n, and by the World obligation never to answer in the request's occurrence) and L465 "Responses to a torn-down slot are discarded by the World" (a World obligation) are stated without saying who is bound. L519's "A state tag the World does not accept is an error at the slot's creation" should name the kind (`UnsuppliedState`) and who raises it (the runtime, on the oracle's refusal). One clause each.
7. **The value-types block (L343–439) is incomplete for the rows it serves.** No `Error`/`ErrorKind`, no `WriteFailed`, no `Draw`, no `Stamp`, no `Frame`, no `Kind`, no `ProcessState` for `Process.Child(id)`; `Change` becomes orphaned once `Fs.Watch` is state. `Duration` is declared and then used only by `Time.Every` once `Time.After` takes `Nat`.
8. **`Net.Socket` has two identities in one row** (L486: path for requests/responses, value for state). Say explicitly that this is the general rule for any tag with both parts (responses by path, state by value, one slot, two identities) in the "Three things apply to every row" paragraph (L339), with an example, rather than letting the one row imply it.
9. **Appendix A would benefit from one worked example per differing row group**: the relabelled-button case at L65 is the only concrete one. The `MonadQuery` paragraph (L77) wants the three-line Reflex widget that starts with `mempty` beside the Prism `state:` element that has a value in occurrence zero; the `listWithKey` paragraph wants the insert-at-front row rebuild in both.
10. **The "Also consulted" section (L203) is a list of URLs without contributions.** Either give each one line of what it contributed (Incremental and differential dataflow already have it) or cut the rest; a reference the author reads in months needs the why, not the link.
