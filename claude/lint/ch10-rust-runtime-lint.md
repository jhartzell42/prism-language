# Lint: ch10-rust-runtime.md

Chapter: `/home/claude/book/ch/ch10-rust-runtime.md` (1202 lines). Line numbers below refer to that file.

Verification notes used for rule 10: the crate at `/home/claude/jhartzell42/prism-language/prism_runtime` (branch `on-fire-repros`, tip `c03a6fd`, edition 2024, deps `log`/`serde_json`/`thiserror`, dev-dep `test-log`, 38 `#[test]`s) matches the chapter; every test name in "The 38 tests as the laws they pin" exists; the `*10`/70, 500 swaps, `< 10` weak entries over 1000 occurrences, 5-second timeouts, every-16 consolidation, `Some(vague)` typo, `IndexMap` comment, `__rebuilt__`, `children[0]`, `max(current + 1)`, `usize::MAX` scheduling, `mem::forget` in `trace`, the `schedule` panic text and the `behavior cycle` panic text are all in the source. `subscribeAndRead` is in `/tmp/claude-0/rx/Spider.hs`. The crate-vs-target structure of the chapter is what the BIBLE's "Current Rust runtime facts" paragraph asks for, so descriptions of the current code are not treated as rule-1 history.

## Part 1: rule violations

### Rule 1 — history / superseded designs (1)

- L902: "This replaces the delegate callback." — "replaces the" aimed at the current code's mechanism rather than describing the target on its own terms. Fix: cut the sentence; the next one ("The World never sees an event; it sees a list of what fired.") already says it. (Borderline: the current/target split is sanctioned, but this is the one place the target is argued as a replacement rather than described.)

### Rule 2 — status tracking (0)

### Rule 3 — quoting or attributing Jimmy (0)

### Rule 4 — banned words (0)

### Rule 5 — clefts and non-default word order (8)

- L25: "What remains is small." — pseudo-cleft. Fix: "The remainder is small."
- L148: "What the panic path does not do is turn the panic into a value or drop the nodes a `dyn` built before the panic; the target's `run` does both." — pseudo-cleft. Fix: "The panic path does not turn the panic into a value and does not drop the nodes a `dyn` built before the panic; the target's `run` does both."
- L688: "What the runtime does with it is under the occurrence, below." — pseudo-cleft plus scaffolding. Fix: "The occurrence section says what the runtime does with it." (or cut; the quarantine paragraph is two screens down).
- L708: "What rollback cannot undo is a closure's side effect outside the graph, and there are none in compiled code, which is pure by construction." — pseudo-cleft. Fix: "Rollback cannot undo a closure's side effect outside the graph, and compiled code has none, because it is pure by construction."
- L858: "That the list's *value* is still the old list until commit, while its change event has the new one, is the ordinary dynamic contract, and it is the delayed edge that routing reads." — "it is X that" cleft in the second clause, heavy subject clause in the first. Fix: "The list's value stays the old list until commit while its change event has the new one; that is the ordinary dynamic contract, and routing reads exactly this delayed edge."
- L1003: "what the specification requires is only that a conforming implementation does not hang or die." — pseudo-cleft. Fix: "the specification requires only that a conforming implementation neither hangs nor dies."
- L1133: "What it was for, telling a backend what a node means, is the element list." — pseudo-cleft. Fix: "Its purpose, telling a backend what a node means, is served by the element list."
- L1195: "What the variant needs is `Sync` graph types, a concurrent subscriber-list consolidation and a concurrent action map, which is a second runtime compiled from the same source with the pointer and cell types swapped, not a change to this one; the single-threaded variant stays the reference the other is tested against." — pseudo-cleft. Fix: "The variant needs `Sync` graph types, a concurrent subscriber-list consolidation and a concurrent action map: a second runtime compiled from the same source with the pointer and cell types swapped, not a change to this one. The single-threaded variant stays the reference the other is tested against."

### Rule 6 — lecturing tone (1)

- L3: "This chapter is the runtime twice over. First the crate as it is on disk: module by module and type by type, with the real signatures and the mechanisms behind them, which are […]. Then the target: […]. Between the two sits an ordered list of steps, each with what done means; after them, every loose end in the current code with its fix; and at the end, what costs what and the three kinds of parallelism the structure allows without any program noticing." — the lead is a table of contents ("In this chapter we will…" in disguise), and the second sentence is a 90-word relative-clause chain. Fix: keep the first two sentences of the lead (what the runtime is) and one sentence of the point ("The whole of it is a height-ordered action map, a commit list, weak subscriber lists and one synchronous call; everything else here is either the code that exists or the consequences of that boundary."); cut the roadmap.

### Rule 7 — dry definition before motivation (15)

The "crate as it stands" subsections are framed at L29 as a reference, but each still opens cold with a struct dump. One sentence of the lived problem per subsection is enough; the sentence usually exists later in the section and can move up.

- L90–115 "The runtime core" opens with the `Runtime`/`RuntimeInner`/`Action` code block. Fix: lead with the sentence at L117 ("This is the whole engine… pop the lowest height, run everything at it, repeat") and then the code.
- L152–165 "Events" opens with `pub struct Event<T: ValueType>(pub(crate) Arc<dyn EventImpl<T>>);`. Fix: lead with L167's "An event is a handle to an implementation with exactly two obligations…" and the laziness point (L171), then the code.
- L308–316 "Behaviors" opens with the `Behavior`/`BehaviorImpl` code. Fix: lead with L318 "A behavior is the pull half of push-pull FRP: a value you ask for…", then the code.
- L368–381 "Dynamics" opens with the `Dynamic<T>` code. Fix: lead with "A dynamic is the pair, with the invariant that the event fires exactly when the behavior changes" (L383) and the hold-is-its-own-input point, then the code.
- L387–398 "Widgets" opens with the `Widget`/`PolymorphicWidget` code. Fix: lead with L400 "A widget is a description with a `build` method; a node is what `build` produces…".
- L613: "`Rc`, `Weak`, `RefCell`, `Cell`." — the target's first section opens with four type names. Fix: open with the one-thread argument from L645 ("the locks are uncontended by construction") or with the question the thread-locals answer (L639), then the type list.
- L649–682 "The run call" opens with the `Inputs`/`Outcome`/`ErrorKind`/`RuntimeError` code. Fix: lead with the alternative at L690 ("A World could keep today's form: hold triggers… a World that holds a trigger for a slot that has since been torn down holds a pointer into a tree that has moved on"), then the types.
- L694: "`run` has five phases, and every one of them is short." then the phase list. Fix: lead with what goes wrong without them (an input delivered against a list that changed under it; a hold written before a tag read it), which L698 and L702 contain.
- L720: "For `outer: Dynamic<Dynamic<A>>`, `join` is `$Join`: at occurrence n its value before n is the value before n of the inner current before n…" — the join law stated formally first. Fix: lead with the hard case at L722 ("the outer switches to `d2` in the same occurrence that `d2` changes… the two halves disagree"), then the law.
- L770: "`for $m:` is a widget over a dynamic map:" then the `list_widget` signature. Fix: lead with the todo-list row that must keep its state when a neighbour is inserted (L780's "a key that stays is never torn down"), then the signature.
- L798–819 "Elements, slots and paths" opens with five type definitions. Fix: lead with L821's "An `ElementValue` is plain data… so that a World can key a map by it" and the counter's `0/#0/0` from L825, then the types.
- L849: "A node's element list is a derived dynamic:" then the `ElementList` type. Fix: lead with the stale-label problem at L858 ("A list that waited for commit to change would be a list one occurrence stale, and the counter's transcript would show the old label for a turn").
- L870: "Routing goes through a keyed fan, one per interceptor and one at the root:" then the `ResponseFan` struct. Fix: lead with the relabelling-button click that must land on the old slot (L1101), then the fan.
- L934–944 "`InterceptWidget`" opens with the struct and impl. Fix: lead with L948 "forwarding is manual, so the interceptor has no policy to implement" and the two alternatives at L952, then the code.
- L969: "The compiler's output for a widget block is an `IrWidget`, a Rust value implementing `PolymorphicWidget<Value>`…" then the `Step` enum. Fix: one sentence first on why the three occurrence-time build sites need one Rust type (L993), then the enum.

### Rule 8 — presence or drop as a trigger (0)

Teardown, `KeepUntilOccurrenceEnds`, memo expiry on the last slot's drop, and "write the last value into a cell on teardown" (L1145) are ownership and bookkeeping, not program-visible actions; none flagged.

### Rule 9 — design conflicts with the BIBLE Decisions (10)

- L806: `pub name: &'static str,                // "Gui.Button"` — `Gui.*` is the five-tag primitive vocabulary; a button is `Ui.Button`. Fix: `// "Ui.Button"`.
- L963: "`@Element Gui.Label(text=$t)` is a `DynamicWidget` over `t.map(|t| element_widget(Gui.Label { text: t }))`, and its generation's node has one slot at position 0, which is why the label is `2/#g/0`." Fix: `Ui.Label` in both places.
- L1010–1012: the list `0/#0/0: Gui.Button(label="Increment")` / `1/#0/0: Gui.Button(label="Decrement")` / `2/#0/0: Gui.Label(text="Count: 0")`. Fix: `Ui.Button` / `Ui.Button` / `Ui.Label` (worked examples under the Test World use `Ui.Button` responding `Click`).
- L1017: "the tag `Gui.Button` responds with `Click`" Fix: "the tag `Ui.Button` responds with `Click`".
- L1019: "the block's one step is `element(Gui.Label { text: "Count: 1" }, None)`" Fix: `Ui.Label`.
- L1021: "the root list is now the three entries with `2/#1/0: Gui.Label(text="Count: 1")`." Fix: `Ui.Label`.
- L972: `pub struct IrWidget { steps: Rc<[Step]>, captures: Vec<Value>, span: Span }` — the Decision is `IrWidget { program, block, captures, span }` with `Program { functions, blocks, tags, constants }`, and chapter 11 (L797) defines it that way. Fix: `pub struct IrWidget { program: Rc<Program>, block: BlockId, captures: Rc<[Value]>, span: Span }`, with the `Step` list living in `Program.blocks[block]`; keep the `Step` enum but say it is the block's, not the widget's.
- L827: "`Supplied(n)` counts, per interceptor node, every slot created by a widget the interceptor's supplier built, in creation order, and never resets, so a supplied widget that contains a `dyn` still gets fresh paths per generation; the supplied widget's own structure is hidden behind the one segment because only its slots' existence is a law." and L1113: "a supplied widget's elements appear in the interceptor's list at `s/n`" — the Decision is that supplier-built slots print `s/n/...` with the supplied widget's own segments after the prefix. Fix: "`Supplied(n)` numbers, per interceptor node, each widget the supplier built, in creation order, never reset; the slots inside it take their own segments under that prefix, so a supplied widget with a `dyn` prints `s/n/0/#g/0` and gets fresh paths per generation like any other." The `Segment::Supplied(u32)` at L815 then names a supplied *widget*, and the sentence about hiding structure goes.
- L963: "For a widget whose result is a dynamic, step 2 is `Dynamic::hold(first, dyn_event.map(|out| out.dynamic)).join()`… For a bundle, field by field by the kind of each field." — the Decision that binding a widget application whose result is plain gives a `$` holding the first build's value has no clause here, and the construct the compiler emits for it (`hold_first`) is a Decision-named part of the target API that this chapter never defines, although chapter 11 (L886) says "the previous chapter gives the runtime" `hold_first`, `map_n` and `tag_map_n`. Fix: add the plain case ("for a plain result, `Dynamic::hold_first(dyn_event.map(|out| out.value))`, a hold whose initial value is the first firing") and define `hold_first`, `map_n`, `tag_map_n` in the target's combinator list with signatures.
- L86 and L1179: `AnyValueInner { Arc(..), U32(u32), I32(i32), Unit }` and "Every value in the graph is a 24-byte enum" — current-code fact, correctly reported, but the target never says the enum gains `I64`/`U64` (`Int` is 64-bit signed and `Nat` 64-bit unsigned). Fix: one sentence under "The run call" or "Values" in the target: "`Value` holds `I64` and `U64` immediates; the 32-bit variants go in step 1."

### Rule 10 — unverified specifics (2)

- L223: "A randomised version of the same check, a hundred thousand random graphs over four million occurrences, found no violation; it is not in the crate, and belongs with the invariant checker of the target." — numbers for a test that is not in the crate and not in the research notes. Fix: verify or mark from memory or cut; if kept, say where the harness lives.
- L86: "The enum is 24 bytes, and the representation is deliberately behind the trait so that the rest of the runtime does not notice when it changes." — no `size_of` assertion in the crate. Fix: verify with a `const _: () = assert!(size_of::<AnyValue>() == 24)` in the crate, or write "two words plus a tag".

### Rule 11 — code that cannot be Prism (0)

The Prism fragments (`!x <- @Button $label`, `\|e| @Element e`, `@Element (Rewrite e)`, `@ Delete on !started`, `Read on !started`, `$"Count: $count"`) match the BIBLE and chapters 1 and 4. The `Gui.Label` cases are counted under rule 9.

### Rule 12 — filler (5)

- L7: "…this section is the same negative space seen from the implementation, and it is worth going through, because each item is a mechanism that does not have to be written, tested, or debugged at three in the morning." Fix: "…this section is the same negative space seen from the implementation: each item is a mechanism that does not have to be written, tested, or debugged at three in the morning."
- L251: "(its `Inner` type is `()` and its `handle_*` methods are `unreachable!()`, a stub discussed under known issues)" — "discussed under" scaffolding. Fix: "(its `Inner` type is `()` and its `handle_*` methods are `unreachable!()`)".
- L338: "*Hold* is how a hold commits, and it is worth reading in full because it is the transaction:" Fix: "*Hold* is the transaction:".
- L548: "That is the second instance of the stub pattern noted under known issues." Fix: cut (the known-issues entry lists all three).
- L952: "Two alternatives are worth putting beside this to see what it buys." Fix: "Two alternatives show what this buys."

## Part 2: structural notes

1. **Target combinator list is missing.** The target section defines `Dynamic::join`, `!Updated`, `list_widget` and the boundary types, but never gives the target's `Event`/`Dynamic` API as a whole. `hold_first`, `map_n`, `tag_map_n` (named in the BIBLE and relied on by chapter 11), `switch_hold_seeded` (used at L729 with no signature), and the `pub(crate)` fate of `external`/`schedule_trigger`/`build_root`/`propagate` should sit in one signature block like the one at L228 for the current crate.
2. **The plain-result lift and `hold_first` in the canonical node structure** (L954–965): the section covers dynamic and bundle results but not plain ones, and a reader writing a harness fixture for `@Label $text` (plain result) has nothing to match. Add the case and its path consequence.
3. **`Outcome.requests` ordering is underspecified** (L704, L898): "grouped per slot in list order" — which list? A slot torn down in the occurrence is in the opened list and not in the committed one. Say "in the order of the opened list, with slots created and torn down in this occurrence after it in creation order" or whatever the conformance chapter's transcript assumes, and name the transcript test that pins it.
4. **The fan's `outputs` map has no cleanup rule** (L873–881): `outputs: RefCell<BTreeMap<Path, Weak<FanOutput>>>` grows with every slot ever created under the fan unless something removes dead entries. Say whether it is pruned at slot drop (the slot record owns the output, so a `Drop` on `SlotRec` can remove it) or consolidated like a subscriber list, and which invariant holds the count bounded.
5. **State supply and failed supplier builds** (L924–930): the memo is written when? If the supplied widget's build fails under the nested transaction, the memo must not hold a `Weak` to a half-built `Supplied`, and the slot that asked must get `UnsuppliedState` or the build error. One sentence on the order (build, then memoise) and on who sees the error closes it.
6. **"A node's own slots are fixed for its life" (L856) states an invariant without a binder.** The compiler is bound (an `Element` step runs only at build) and the builder enforces it (`element` is a builder method, callable only inside `build`). Say so in the Invariants section or at L856.
7. **The diode's World half (L1042)** says "by obligation" with nothing named as the enforcer. The Test World and the harness's two boundary checks (L1062) are the enforcement in practice; name them, and say the Rust World core's loop structure makes it impossible by construction if that is true.
8. **`Dynamic::join` deserves a traced occurrence** like the two at L596–606. The height-dynamic firing of `JoinOuterEvent` (L749) is the subtlest mechanism in the chapter and is described in prose only; a six-line trace of the swap-and-change case in the "above" height order, with heights, would let a reader implement it without rediscovering the "`heard` set below, push above" split.
9. **The scenario table (L1066–1092) and chapter 9's law tests overlap.** Several rows (join law tests, `@started` inside a rebuilt `dyn`, equal-valued state slots sharing a supplier, request from a torn-down slot) are also conformance-suite tests in chapter 9. Mark in the table which rows are harness-only (need Rust-defined tags or injected panics) and which are the suite's, so the two chapters do not maintain the same list twice.
10. **Known issues and The gap overlap** (L1105–1117 vs L1119–1165): step 1 and the `Arc`/`Mutex` entry, step 2 and the `usize::MAX` entry, step 5 and the `WidgetDelegate`/test-backend entries, step 8 and `DWAction` index 0 say the same thing twice. Keep the known-issues list as the authoritative one and have each gap step cite its entries by title instead of restating the fix.

## Counts

| Rule | Count |
| --- | --- |
| 1 History | 1 |
| 2 Status | 0 |
| 3 Attribution | 0 |
| 4 Banned words | 0 |
| 5 Clefts / word order | 8 |
| 6 Lecturing | 1 |
| 7 Definition before motivation | 15 |
| 8 Presence / drop trigger | 0 |
| 9 BIBLE conflicts | 10 |
| 10 Unverified specifics | 2 |
| 11 Non-Prism code | 0 |
| 12 Filler | 5 |
| **Total** | **42** |
