# Lint: ch13-haskell.md

Chapter: `/home/claude/book/ch/ch13-haskell.md`. Checked against `/home/claude/book/BIBLE.md` (Clarifications and both Decisions sections), `/home/claude/book/intro/research-notes.md`, the Reflex sources under `/tmp/claude-0/rx/` (`Adj.hs`, `Basic.hs`, `Class.hs` = PostBuild class, `Network.hs`, `PostBuild.hs`, `Query.hs`, `QueryBase.hs`, `Requester.hs`, `Spider.hs`, `Xhr.hs`), and chapter 11's text form (`/home/claude/book/ch/ch11-rust-lang.md`, lines 959–1099).

Counts: rule 1: 0 · rule 2: 0 · rule 3: 0 · rule 4: 0 · rule 5: 7 · rule 6: 5 · rule 7: 2 · rule 8: 0 · rule 9: 6 · rule 10: 13 · rule 11: 0 · rule 12: 1. Total 34.

## Part 1: rule violations

### Rule 1 — history / superseded designs: 0

Nothing flagged. Borderline, not a violation: "a reflex-dom World that puts Prism programs on a screen before the Rust GUI World exists" (line 3) and "before the Rust rasteriser exists" (line 639) are plan ordering, not a superseded design; "Prism has removed" (line 522) and "`newTriggerEvent` is removed" (line 500) are relative to Reflex, not to an earlier Prism.

### Rule 2 — status tracking: 0

Nothing flagged. Borderline: "The third problem is the one that settles it." (line 52) uses "settles" as argument, not status; it is flagged under rule 5 instead.

### Rule 3 — quoting or attributing Jimmy: 0

### Rule 4 — banned words: 0

No carry/shape/compose/genuinely/honestly/straightforward/leverage in the chapter.

### Rule 5 — clefts and non-default word order: 7

1. Line 52: "The third problem is the one that settles it." — cleft. Fix: "The third problem settles it."
2. Line 56: "What the interpreter gives up is GHC checking the output and speed, and it needs neither:" — pseudo-cleft. Fix: "The interpreter gives up GHC checking the output, and speed, and needs neither:".
3. Line 94: "What is never done is bringing the Haskell frontend to the Rust frontend's behaviour without a sentence to point at." — pseudo-cleft. Fix: "The Haskell frontend is never brought to the Rust frontend's behaviour without a sentence to point at."
4. Line 510: "What the Haskell implementation does not do after an error is quarantine." — pseudo-cleft. Fix: "After an error the Haskell implementation does not quarantine."
5. Line 558: "The transcript printer is the one piece of the Haskell implementation that has to agree with the Rust one character for character, and it is defined by the suite's format rather than by either implementation:" — "X is the one piece that…" is a cleft with a heavy relative clause. Fix: "Only the transcript printer has to agree with the Rust one character for character, and the suite's format defines it, not either implementation:".
6. Line 609: "What is never done is resolving a divergence by making the Haskell implementation copy the Rust behaviour, or the reverse, without a sentence to point at, because that turns the second reading back into the first." — pseudo-cleft. Fix: "A divergence is never resolved by making one implementation copy the other without a sentence to point at; that turns the second reading back into the first."
7. Line 625: "What the Haskell implementation does catch is a *typed AST* that is wrong, because it executes the typed AST directly, …" — pseudo-cleft. Fix: "It does catch a wrong *typed AST*, because it executes the typed AST directly, …".

### Rule 6 — lecturing tone / section scaffolding: 5

1. Line 18: "the last section of this chapter lists what that costs and what it buys." — forward-reference scaffolding ("In this section we will" in reverse). Fix: cut the clause; the last section stands on its own.
2. Line 500: "Everything in the correspondence table's last rows is now visible in one place." — announces that the section has done something rather than saying it. Fix: cut; start the paragraph at "An occurrence is a Spider frame, …".
3. Line 522: "This section is the checklist with the evidence for each line, so that whoever reviews the evaluator can see why the line is there." — section describing itself. Fix: "Each line below has the evidence for it, so that whoever reviews the evaluator can see why the line is there."
4. Line 530: "Covered above under `@started`: its frame is "determined by the instance", and its purpose is safe sampling, which Prism does not have." — "as noted above" scaffolding. Fix: "Its frame is "determined by the instance", and its purpose is safe sampling, which Prism does not have."
5. Lines 146, 169, 368, 409 (one item): "How it does that is the subject of the `for` section, because `for` needs the same thing and borrows the same mechanism." / "`envSupply` is the nearest supplier context, which is the heart of state supply and gets its own section." / "Its state dynamic, if the tag has state, comes from the supplier context, which is the next section." / "The partition in the middle is state supply's lifetime mechanism, and it needs the supply context to explain it." — four "see the next section" pointers. Fix: keep at most the first; drop the others (the subsection headings already do the pointing).

### Rule 7 — dry definition before motivation: 2

1. "The monad stack" (line 129–140) opens with "A widget block runs in `PrismM t`, which is what `@A` corresponds to:" and a three-layer transformer type, then "Each layer is there for one law." Fix: lead with the question the stack answers (a block has to produce an element list that is itself a dynamic, collect request firings including a newborn's, and be rebuildable by `dyn`; each of those is a writer or an `Adjustable`), then the type.
2. "The environment" (line 154–167) opens with a bare record definition and no sentence. Fix: one or two lines first: "Every slot needs a path, every response needs a fan to select from, every state slot needs a supplier to ask. The environment is where a widget block finds those:" then the record.

### Rule 8 — presence or drop as a trigger: 0

Nothing flagged. Checked: "`requestAnimationFrame` re-armed after each turn while a `Gui.Frames` slot is in the list" (line 593) is the World answering a responds-only element it accepted, which the Decisions allow ("responds per frame if asked"); "the supplied widget's nodes become garbage" (line 440) is lifetime, not an action; line 597 says the scene is description, not a trigger.

### Rule 9 — conflicts with the BIBLE Decisions: 6

1. Lines 78–90, the dump: "`@counter : @unit … @1:1-6:31`" / "`g0  !increment : !Click   build  = bind @Button (const $Text "Increment")   @2:5-2:38`" / "`g3  $count : $Int         placeholder`" / "`g3  close $count`". Decision: "The typed-AST text form is as chapter 11 defines it" (`name: sigil type [mark] group = expr @span`, `[build]`/`[occ]`, placeholder line `name: type [build] gN = placeholder`, close line `close $count $count.hold`, literals `0:Int`, trait ops `Num.Add[Int]`, `dmap`, `cat`, `ToText[Int]`, groups from `g1`, spans `@12:5`). Chapter 11's own counter dump (its lines 1051–1058) is the corrected form; replace the block with it verbatim, and update the sentence at line 92 that reads group numbers and line names off this dump ("line `g3` differs", "the `let` on line `g2`").
2. Line 69: "`Prism.Typed`: the typed AST as a Haskell data type, one constructor per node kind the text form has: `map`, `map2`, `tag`, `leftmost`, `hold`, `const`, `bind`, `slot`, `dyn`, `switch`, `join`, `for`, `intercept`, `updated`, `distinct`, `placeholder`, `close`, the literal nodes with their numeric types, the trait-resolved operators (`add.Int`, `show.Int`), and the closure form with its binders." — the node-kind names must be chapter 11's (`dmap`, `tag … [$count]`, `hold_first`, `element … responds`, `Num.Add[Int]`, `ToText[Int]`, `switch`, `join`, `dyn`, `for`, and the `can fire at birth` flag on `for`). Fix: list chapter 11's keywords, or say "one constructor per keyword in chapter 11's grammar table" and name none.
3. Line 575: "An aborted occurrence prints `error: Error.Cycle(spans=...)` and no list." — Decision: one tag `Error` with `kind: ErrorKind`, `spans`, `occurrence` (boundary type `RuntimeError { kind, spans, occurrence }`). Fix: `error: Error(kind=Cycle, spans=[...], occurrence=1)`. Note: chapter 9 (line 1967) spells it the same wrong way; the two should change together.
4. Line 590: "a draw entry per element: `Rect` to an absolutely positioned `div` with a background, `Text` to a `span` at its origin, `Image` to an `img`, `Clip` to a `div` with `overflow: hidden` and the clipped entries inside it." — Decision: the entries are `Draw.Rect`, `Draw.Text`, `Draw.Image`, `Draw.Clip` values in `Gui.Scene`'s `draw` field. Fix: spell them `Draw.*` and name the `draw` field.
5. Line 593: "`Gui.Frames` | responds per frame while a slot exists | `requestAnimationFrame` re-armed after each turn …; a tick is a turn." — Decision: `Gui.Frames` responds `Frame(now: Instant)`. Fix: "responds `Frame(now: Instant)` per frame while a slot exists … a tick is a turn delivering `Frame(now)` to the slot's path."
6. Line 127: "Records compare structurally, and `Ord (Val t)` is derived so that a `VMap` can be keyed by any plain value, which is what `for` over a `Text ==> Item` needs." — Decision: records have equality and no ordering, so a record cannot be a map key. Fix: "keyed by any ordered plain value (text, numbers, tuples, arrays); the elaborator has already rejected a record key, so the derived `Ord` on `VRecord` is never reached."

### Rule 10 — unverified specifics: 13

Each: verify, or mark from memory, or cut. The Reflex files on disk do not include `DynamicWriter.hs`, `EventWriter.hs`, `PerformEvent/Class.hs`, `PerformEvent/Base.hs`, `Host/Class.hs`, `Dynamic.hs`, `Collection.hs` or `Dynamic/Uniq.hs`; quotes and names from those are unverified here even where they are probably right.

1. Line 46: "GHC type-checks the output, so a printer that emits a `Dynamic` where an `Event` was meant is caught by a compiler that has been catching that for thirty years." — the number is from memory.
2. Line 144: "Reflex's `MonadDynamicWriter` documentation says it "efficiently collects 'Dynamic' values using 'tellDyn' and combines them monoidally to provide a 'Dynamic' result"" — quote not in the sources on disk or the research notes.
3. Line 146: "`EventWriterT`'s `runWithReplace` instance includes a newborn's tells in the frame of its birth." and line 343: "Reflex's own `EventWriterT` does the same thing in its `runWithReplace` instance, telling `coincidence` over the replacement event so that a newborn child's tells are heard in its birth frame, with a comment that the coincidence case must come first so that it has precedence" — `EventWriter.hs` not on disk; the comment is quoted from memory.
4. Line 215: "Its definition in Reflex is `attachWithMaybe` over `current d` and `updated d`, holding the result" — `holdUniqDyn`'s definition is not on disk.
5. Line 266: "`dyn $x: block` is Reflex's `networkView`, which reflex-dom exports as `dyn`" — `networkView` is verified (`Network.hs` 23–27); the reflex-dom export name is from memory.
6. Line 305: "`$Join` is `join` on `Dynamic`, which exists because `Monad (Dynamic t)` is a superclass of `Reflex`." — `Reflex/Class.hs` is not on disk (the `Class.hs` present is the PostBuild class).
7. Line 311: "`listWithKey :: (Ord k, Adjustable t m, MonadFix m, MonadHold t m, Eq v) => Dynamic t (Map k v) -> (k -> Dynamic t v -> m a) -> m (Dynamic t (Map k a))`" and line 317/322: "`listHoldWithKeyAdjust`" — the signature is from memory (only a use of `listWithKey` is on disk, `Basic.hs` 306); `listHoldWithKeyAdjust` is not a Reflex name I can find anywhere (Reflex has `listHoldWithKey` and `traverseDMapWithKeyWithAdjust`, the latter verified in `Adj.hs`). Fix: name the real function or say "a helper over `traverseDMapWithKeyWithAdjust`".
8. Line 322: "The per-key dynamic is the map's entry projected with `uniqDyn`-style equality on the value" and line 347: "as it does under `simpleList`" — both names from memory.
9. Lines 467–498: "`newEventWithTriggerRef`", "`hostPerformEventT`", "`runFireCommand`", "`FireCommand`", "`fireEventsAndRead` takes a list of triggers with their values and a read phase" — host-side names from `Reflex.Host.Class` and `Reflex.PerformEvent.Base`, not on disk (`subscribeEvent`, `readEvent`, `runSpiderHost`, `withSpiderTimeline` are verified in `Spider.hs`).
10. Line 526: "**`switchHoldPromptly`, `switchPromptlyDyn`, `switchPromptOnly`, `coincidence`, except the birth term.**" — `switchPromptOnly` is not a Reflex name; the doc quoted in the same paragraph says `switchHoldPromptOnly`. Fix: `switchHoldPromptOnly`.
11. Line 532: "The documentation says "the output 'Event' will generally occur later than the input 'Event', since most 'Performable' actions cannot be performed during 'Event' propagation"" — `PerformEvent/Class.hs` not on disk.
12. Line 601: "The World runs under GHC's JavaScript or WebAssembly backend through jsaddle, or natively through WebKitGTK with the same code." — from memory.
13. Line 615: "the way Rustlantis runs random MIR on several backends" — attribution from memory.

Verified and not flagged: the `hold` doc (line 209), the Spider `hold` comment and `defer $ SomeHoldInit h` (line 237), "causality loop detected" (line 239), `getHoldEventSubscription` reading `occ` through `subscribeAndRead` (line 260), `networkView`'s body and `tagCheap` (lines 269–274), the PostBuild doc and `now`'s `IORef`/scheduled clear (line 299), the `switchHold`, `coincidence`, `switchHoldPromptly`, `switchPromptlyDyn` and `tagPromptlyDyn` docs (lines 303, 341, 343, 526, 528), `defer $ updateMerge …` and `runFrame` running `_someMergeUpdate_update` after `clearOccurrences` (line 329), `eventEnvAssignments` and `checkCycle` (line 504), `widgetHold = networkHold` with `holdDyn` inside (line 307), the read phase running inside the frame before assignments (line 498; `runHostFrameFireAndRead`, `Spider.hs` 2827–2834).

### Rule 11 — code that cannot be Prism: 0

The Prism in the chapter (lines 27–32, `\|e| @Element e`, `@ Delete on !started`, `intercept @child, !responses, $supplier`) is consistent with the BIBLE. The Haskell sketches have type errors; see Part 2.

### Rule 12 — filler: 1

1. Line 343: "This is the one place `coincidence` appears in the evaluator, and it is worth saying why it is right here and nowhere else." — "it is worth saying". Fix: "This is the one place `coincidence` appears in the evaluator, and here is why it is right here and nowhere else."

## Part 2: structural notes

1. **The Haskell sketches do not type-check, and the chapter does not say whether they are pseudocode.** `VFun ![Val t] -> Val t` (line 119) needs parentheses; `Ord (Val t)` "is derived" (line 127) cannot be derived over `VFun`, `VEv`, `VDyn`, `VWidget`, which the same paragraph says have no `Eq`; `evalIntercept` (lines 390–402) uses `ix` and `runWidget'` unbound and applies `Map.partitionWithKey` to a `Dynamic` without `fmap`; `consolidate` for `$` results (line 326) joins a map of `Val` without `asDyn`. Either mark the blocks once as sketches ("the types are elided where they would hide the point") or fix them; a reader handing this chapter to a model will otherwise get a module that fails at the first compile for reasons the chapter does not anticipate. The cleanest fix for `Ord` is a separate `Plain` type (the values with `Eq`/`Ord`) nested inside `Val`, which also makes "the elaborator guarantees they are never compared" structural rather than a runtime failure.

2. **Rollback by replay logs the wrong thing, or the chapter does not say what it logs.** Line 506: "the host keeps the log of every committed run call's inputs". A run call's inputs include the oracle, which is a function the World supplies from its snapshot; replaying needs either the snapshots per turn or the oracle's answers as they were asked. Say which is logged (the answers as asked is smaller and is what determinism needs) and who is bound: the world core, since the evaluator cannot see the oracle's calls. Also say what bounds the log (nothing, and why that is acceptable for a test oracle), since "the cost is the length of the history" is only true if the history is kept whole.

3. **Path uniqueness is asserted without its enforcer.** Line 144: "with paths unique so union never has to choose." Uniqueness depends on `envBindIx` restarting at zero inside every rebuilt `dyn` block and `for` block (fresh `#g` or `[k]` segment, then positions from zero), and on `nextSupplied` being per-interceptor. State that the environment's counters are the enforcer and that `evalDyn`/`evalFor`/`supplyState` each create a fresh `envBindIx`; the sketches at lines 279–285 and 314–319 do not show it.

4. **Replay and `IORef`s.** Line 152 puts generation counters, bind positions and memo tables in `IORef`s "in the reader". Replay on a fresh timeline (line 506) must also recreate those refs, or the replayed program's generations and supplied-slot numbers (`#g`, `s/n`) will not match the committed transcript. One sentence: `loadProgram` allocates every ref, so a rebuild starts them all at zero.

5. **The reflex-dom World section is thin on everything but the five `Gui.*` tags.** It does not say which other components the World offers (`Time.*`, `Process.Exit`, `Net`?) or that it offers none; how `Gui.Metrics`'s oracle answers before fonts load (the `Unavailable`-style case the design elsewhere insists on); or that the World's own Reflex network and the Prism runtime live on different timelines and how a DOM event becomes a run call (an `IO` callback on the global timeline calling `run` on the local one). The "a turn in this World" paragraph (line 597) is the right example and should be twice as long.

6. **State supply's hit case has an unexplained term.** Line 427: "`Just (d, l) | ev `suppliedIn` opened || builtThisFrame ev`". `builtThisFrame` is never defined, and it is the one fact that distinguishes a second newborn with the same value from a slot for a value whose widget died in an earlier occurrence. Say where it comes from (an `IORef` set of values built since the frame opened, cleared by the host at the start of `run`) and who clears it.

7. **`Defer` twice in a turn.** Line 544: "a `Defer` answered twice in one turn is a `DeferBackwards` error". The BIBLE's `DeferBackwards` is deferring backwards (a `Defer` request after its answer in the same turn); a second answer to one slot is the "at most once per slot per turn" bound. Say whether the Haskell world core reports both under `DeferBackwards` and that chapter 6 is the authority.

8. **Duplicates.** The turn-loop paragraph (line 544) restates chapter 6's world core and chapter 9's suite steps nearly in full; one sentence ("the loop is chapter 6's, unchanged, and twenty lines shorter") and the Haskell-specific facts would do. "What must not be used" (lines 520–538) overlaps the correspondence appendix's Reflex-doc quotations (chapter 16); keep the checklist and the one-line reason per item here, and let the appendix hold the full quotations, or the reverse.

9. **An example that would help.** A `for` with a birth firing, traced through Spider the way the counter is traced (line 512–518): the patch fires in n, `coincidence` subscribes to the newborn's result event mid-frame, the merge's membership update is deferred to frame end, and the parent sees the entry in n. That trace is the chapter's most Spider-specific claim and the one most likely to be wrong, and it is the one with no worked occurrence.

10. **The frontend section is a module list without its hard cases.** Lines 62–71 name the modules; the goldens and layout corpus are named but the chapter does not say which sentences of chapters 2 and 11 the Haskell frontend is most likely to read differently (where generalisation happens is named at line 94; the comma rule, the trailing-lambda rule, `else` terminating an inline block, and literal pinning across a merge are not). A short list of "the five sentences to read twice" would make this section usable by whoever writes `Prism.Elab` in months.
