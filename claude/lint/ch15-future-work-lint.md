# Lint: ch15-future-work.md

Chapter: /home/claude/book/ch/ch15-future-work.md (16,629 words). Line numbers refer to that file.

## Part 1: rule violations

### Rule 1 — history / superseded designs (5)

- L128: "Inline `i64` and `f64` in the enum, replacing the `u32` and `i32` that are there because the first test programs used them." — explains the current enum by its history. Fix: "Inline `i64` and `f64` in the enum in place of the `u32` and `i32`."
- L346: "…which is what makes the occurrence abort of II.10 work unchanged under a native backend." — "II.10" is a section number from a previous document; no other chapter uses that numbering. Fix: "…which is what keeps the occurrence abort (the errors chapter) working unchanged under a native backend."
- L358: "A prototype has shown the form it takes: the register code prints almost directly as LLVM IR…" — argues from an earlier piece of work the reader has no access to. Fix: "The form it would take is clear: the register code prints almost directly as LLVM IR…"
- L515: "Closures regain `Send + Sync`, which the single-threaded runtime removed. Values go back to atomic counts…" — "regain"/"removed"/"go back" narrate a prior state. Fix: "Closures are `Send + Sync` in this variant, and values use atomic counts…"
- L577: "That is the construction-abort of II.10 doing its job for a subtree that was being reborn." — same "II.10" reference. Fix: "That is the construction abort from the errors chapter doing its job…"

### Rule 2 — status tracking (3)

- L151: "The runtime is single-threaded by decision, and the parallel variants later in this chapter are separate runtimes…" — "by decision" is status. Fix: "The runtime is single-threaded, and the parallel variants later in this chapter are separate runtimes…"
- L358: "It is not adopted." — status verdict. Fix: cut; the following sentence already gives the reasons ("The pin on an LLVM version… are costs that nothing yet justifies").
- L503: "That is a decision about the first implementation and not a claim about the model…" — "decision" as status. Fix: "That is a property of the first implementation and not a claim about the model…"

### Rule 3 — quoting / attributing Jimmy (0)

### Rule 4 — banned words (1)

- L175: "…so the word is checked against something obviously right." — "obviously". Fix: "…so the word is checked against something simple enough to trust."

### Rule 5 — clefts and non-default word order (7)

- L102: "Two layouts are the alternatives, and both are worth taking seriously before rejecting." — inverted order. Fix: "The alternatives are two other layouts."
- L206: "The point is not saving characters; it is that a function over a large value reads like a description of the value:" — "it is that" cleft. Fix: "The point is not saving characters. A function over a large value should read like a description of the value:"
- L308: "What a backend needs that the tree does not say flat is order:" — wh-cleft. Fix: "The tree does not say one thing flat, and a backend needs it: order."
- L354: "The thing to be clear about is what `rustc` checks." — pseudo-cleft. Fix: "`rustc` checks the emitter's Rust and nothing else."
- L364: "What that buys is worth saying precisely, because it is the reason a second backend is cheap to trust." — wh-cleft plus filler. Fix: "This is the reason a second backend is cheap to trust."
- L527: "What it preserves, and this is the part worth being exact about: the transcript." — wh-cleft. Fix: "It preserves the transcript."
- L547: "The thing you want is to change the dialog's code and see the running program pick it up with its state intact." — pseudo-cleft. Fix: "You want to change the dialog's code and see the running program pick it up with its state intact."

### Rule 6 — lecturing tone (0)

### Rule 7 — dry definition before motivation (2)

- L427 (Traits): "The language has three built-in traits, equality, ordering and numeric, which the checker resolves at each use…" — opens with mechanism; the lived case is at L444 ("a program with a `Vector` type writes `Add a b` where it would write `a + b`"). Fix: open with that: you have a `Money` or a `Vector` type and every sum is spelled `Add a, b`; then the three built-in traits and dictionary passing.
- L650 (A package system): "A module system says how one program is split into files. A package system says how code written by someone else gets into a program…" — definition first. Fix: open with the afternoon: a second repository wants the GUI library, copies the directory, and three weeks later the two copies disagree on `Ui.Button`'s fields; then what a package is.

### Rule 8 — presence or drop as a trigger (0)

### Rule 9 — conflicts with the BIBLE Decisions (8)

- L17: "The rest of the runtime sees `Value<T>`, a typed wrapper, and `AnyValue`, the erased form…" (and `AnyValue` throughout: L39, L63–71, L183, L318, L331–344). The Decisions name the boundary's plain value type `Value` and the typed wrapper `Typed<T>`. Fix: describe the current module's names once as what the source has today, then use `Value` (erased) and `Typed<T>` for the design: `pub struct Value { word: NonNull<()>, … }`, `fn int_add(a: &Value, b: &Value) -> Value`, `Leaf2 = … fn(env: *const Value, …)`, `prism_event_tag_map(e: *const Event<Value>, …)`.
- L54: "`Nat` is `usize` and `Int` is `isize`, overflow panics, and none of that changes." — Decisions: `Int` is 64-bit signed and `Nat` 64-bit unsigned on every target. Fix: "`Nat` is `u64` and `Int` is `i64` on every target, overflow panics, and none of that changes." Same correction at L56 ("holding the full `isize`" → "the full `i64`"), L74 (`isize::MIN / -1` → `i64::MIN / -1`), L137 and L180 ("every `isize` and `usize` edge value" → "every `i64` and `u64` edge value"). The wasm32 immediate ranges at L56 stay as they are; the boxed form holds 64 bits.
- L76: "Whether `Int` should be 64 bits on every World, so that a program means the same thing in a browser as on a desktop, is a language question and should be argued as one; this layout implements either answer…" — the question is answered (64-bit). Fix: "`Int` is 64 bits on every World, so a program means the same thing in a browser as on a desktop; on wasm32 the values past 2^28 box, and the cost is time, never meaning."
- L453: "import Gui.Draw (Rect, Color)" — draw commands are `Draw.*` values (`Draw.Rect`, …), not under `Gui`. Fix: `import Draw (Rect, Color)`.
- L459: "…so `tag Button of Element` inside `module Gui` declares `Gui.Button`, which makes the tag namespaces and the module namespaces one thing." — `Gui.*` is the five-tag primitive vocabulary; the button is `Ui.Button`. Fix: "`tag Button of Element` inside `module Ui` declares `Ui.Button`".
- L575: "Under a World without `Dev.Code` the oracle answers `Unavailable` (or the World reports an unknown state tag, which is the production answer) and the fallback is the widget compiled into the program." and L587: "under a World without the tag it is the fallback." — Decisions: an unknown state tag is an error at creation, so under a production World `@Reloadable` would abort the build, not fall back. Fix: either the production World core accepts `Dev.Code` and always answers `Unavailable` (then say so), or `@Reloadable` reads `World.Tags` and builds the fallback directly when `Dev.Code` is not accepted. Rewrite both sentences to one of these.
- L602–606: the `Store.Value` declaration with `requests: !Text` and `state: Unavailable | Known Text`. The Decisions make chapter 6's declaration the persistence vocabulary: `Store.Value` is state-only (`state: Stored`, `Absent | Present Bytes`) and writes are a separate `Store.Put(key, requests: !Bytes, responds: PutResult)`. Fix: drop the redeclaration, and rewrite `@Persisted` (L611–620) with two slots: `$stored <- @Element Store.Value(key=key)` and `!put <- @Element Store.Put(key=key, requests=!write)` where `!write = Encode !change` gives `Bytes`, with `Present b: Decode b` / `Absent: default` in the match. The Test World sentence at L630 and the creation-rule sentence at L642 then refer to `Store.Put` for the write.
- L622: "`$stored on !first` reads it through a tag" — consistent; no change. (Listed so the `Store.Value` fix does not lose it: the state half is still read this way.)

### Rule 10 — unverified specifics (16)

None of these appear in /home/claude/book/intro/research-notes.md or under /tmp/claude-0/rx/. Fix for each: verify, mark as from memory, or cut.

- L46: "Immediates are built with `NonNull::without_provenance` and retagged with `map_addr`, which is the strict-provenance style Miri checks…" — Rust API names and Miri behaviour.
- L50: "OCaml uses one tag bit and lets the static type say which immediate a word is."
- L82: "That is the same increment `Arc` does, with the same release decrement and acquire fence on drop."
- L98: "Go 1.4 changed interface values to always hold a pointer and accepted more allocations for scalars because most interface values were pointers already. Go's reason was its collector…"
- L100: "Pharo's 64-bit VM has immediate floats on this principle."
- L102: "NaN-boxing … narrows integers to 48 bits or fewer, ties pointers to a 48-bit address space…"
- L151: "Lean 4's approach, where an object uses a plain count until it is shared with another thread and is marked for atomic counting then…"
- L163: "HAMT maps and RRB vectors, as the `im` crate provides, give O(log n) updates…"
- L165: "…is `Arc::make_mut` and needs no compiler help. … Koka's Perceus and Lean 4 do the general form with compiler-inserted ownership…"
- L171: "…which is the inline cache Self and the JavaScript engines use."
- L294: "Haskell has the `lens` library, where a lens is a first-class value … and `over`, `set` and `view` are the three operations…"
- L326: "Its IR is SSA with block parameters instead of phi nodes … a structured `match` becomes a `brif` or `br_table` on the tag … The `cranelift-frontend` builder does the SSA construction…"
- L331–346: `extern "C-unwind"`, `#[unsafe(no_mangle)]`, "unwinds through the emitted frame under `C-unwind`, and is caught by the runtime's `catch_unwind`" — Rust ABI and attribute spellings.
- L348: "What Cranelift does not do is wasm: its backends are x86-64, aarch64, s390x and riscv64. … Cranelift also offers a JIT…"
- L358: "…since LLVM's wasm backend is how the runtime itself gets there." and "A prototype has shown…" (see rule 1).
- L481: "A query-based build, in the style of `salsa` or `rustc`'s query system…"

(L76's "current Unix time is past 1.79 billion" and "2^30 is about 1.07 billion" are arithmetic and check out; not flagged.)

### Rule 11 — code that cannot be Prism (4)

- L398: "$size <- @Child $positions[0]" — a lifted index returns `$?Pos`, so `@Child` would have to take `$?Pos`. Fix: `$size <- @Child (First $positions)` with `First: fn [Pos] -> Pos` as a library function, or give `@Child` a `$?Pos` argument and say so.
- L430–436: "trait Show A:", "Show: fn A -> Text", "impl <A: Show> Show [A]:" — type variables are lowercase (`fn a -> b`, `[a]`); uppercase names denote functions and types. Fix: `trait Show a:` / `Show: fn a -> Text` / `impl <a: Show> Show [a]:`.
- L437: "Show = \|xs| Concat ["[", JoinText ", ", (for xs: Show _), "]"]" — inside an array literal a multi-argument call must be parenthesised (chapter 2: `Concat [(Prefix e.text, e.cursor), ToText c]` is two elements); as written this is four array elements, the second being `JoinText ", "`. Fix: `Concat ["[", (JoinText ", ", (for xs: Show _)), "]"]`.
- L444: "a program with a `Vector` type writes `Add a b` where it would write `a + b`." — application is juxtaposition with comma-separated arguments. Fix: "writes `Add a, b` where it would write `a + b`."

### Rule 12 — filler (4)

- L98: "The third is what the alternatives charge, which is the subject of the next paragraph but one." — scaffolding. Fix: "The third is what the alternatives charge, below."
- L155: "This is already assumed by step 7 and is listed so it is not skipped." — Fix: "Step 7 assumes it."
- L284: "The hold case is the one the reuse entry above describes and is worth repeating from this side." — announces a restatement. Fix: cut the sentence and keep the paragraph's content, which adds the count-of-2 detail; or cut the paragraph and move that detail to L165.
- L683: "Every item above waits on a fact. The value representation waits on a profile. `update` blocks wait on a fold… Packages wait on a second repository." — restates every Trigger subsection. Fix: keep the first and last sentences of the paragraph and cut the per-item list.

### Counts

| Rule | Count |
|---|---|
| 1 History | 5 |
| 2 Status | 3 |
| 3 Attribution | 0 |
| 4 Banned words | 1 |
| 5 Clefts / word order | 7 |
| 6 Lecturing | 0 |
| 7 Definition before motivation | 2 |
| 8 Presence as trigger | 0 |
| 9 BIBLE conflicts | 8 |
| 10 Unverified specifics | 16 |
| 11 Non-Prism code | 4 |
| 12 Filler | 4 |
| Total | 50 |

## Part 2: structural notes

1. **`Store.*` duplicates and contradicts chapter 6.** Chapter 6 (L1184–1197) declares `Store.Value` (state) and `Store.Put` (request) and sketches the same hold-from-store idiom. Chapter 15's persistence section should import that declaration by name and spend its length on what chapter 6 does not have: the `@Persisted` widget, the three laws against a tree snapshot, and whole-program snapshots.
2. **`@HBox` has two signatures in the book.** Chapter 5 (L758) defines `@HBox: fn @a -> @a` over an `Ui.HBox` element; the causality section (L376, L389, L397) uses `@HBox: fn [$Size] -> @$[Pos]`. Either name the layout-library widget something else (`@Row`, `@Pack`) or write the cycle in terms of chapter 5's `@HBox`, where the child's size comes back through `Ui.HBox`'s state and the cycle runs through an element rather than two function types. The second is the more interesting case for the annotation, since the element boundary is where the type would have to say something.
3. **The closure compiler is described differently from chapter 11.** L302 calls it "a tree of pre-resolved steps whose `build` walks the tree once … whose leaves are closures over an environment vector" and L477 says "one `IrWidget` constructor per widget function and one closure per leaf"; the Decisions have `IrWidget { program, block, captures, span }` over `Program { functions, blocks, tags, constants }`, which is a table of blocks and function indices, not a tree of Rust closures. One sentence aligning the description with chapter 11's form (register code is then a flattening of a block, not of a closure tree) is enough.
4. **Who enforces the package exclusions.** L665 states "No native code … And no build scripts" without saying who checks. Say: the loader rejects a package containing anything but Prism source and the manifest, and both implementations' loaders run the same check so the differential test covers it.
5. **Causality: the soundness claim needs its conservative cases.** L415 says "the annotation only adds prompt edges that exist." The Decisions already have one conservative rule the inference would inherit (`for` over a widget parameter whose body is not visible is treated as able to fire at birth), which adds an edge that may not exist and therefore can reject a program that would never have aborted. Say which edges are over-approximated (widget parameters, `for` over an opaque body) and that in those cases the annotation on the parameter is what the author writes to get the program accepted.
6. **Hot reload: the production path.** Beyond the rule-9 fix, the section would be complete with two sentences on what the production World does with `Dev.Code` (accepts the tag through `World.Tags` and answers `Unavailable`, or the library checks `World.Tags`) and on paths: a reload changes every slot path under the `dyn`, so a World holding a native widget keyed by path sees a teardown and a fresh slot, which is the same as any rebuild and should be said once here.
7. **`@Persisted` and occurrence zero.** L622 claims "nothing outside the program sees `default`" because the World reads after commit. A `dyn` over `$value` inside occurrence zero builds with `default`, then rebuilds in the same occurrence when the hold commits? No: the hold commits at the end, so the `dyn` rebuilds in occurrence one with a fresh path, and the slot built under `default` existed in the list the World read after occurrence zero. Either state that one-occurrence hiccup, or use the cache idiom the Clarifications name (hold of dynamics joined with `$Join`) and say why it has no hiccup here.
8. **Values: closures and descriptors.** L84 says closures are "a code id plus captures, one descriptor per closure site" and L80 says descriptors are `&'static`. Where a per-site descriptor comes from (generated by the closure compiler at load into a leaked table? emitted as a static by the Rust backend?) and whether a bytecode-loaded program can have `'static` descriptors is a gap a reader implementing step 2 hits immediately.
9. **"What the list has in common" restates the Trigger subsections.** Beyond the rule-12 cut, this closing section could instead say the one thing no Trigger subsection says: the order in which the triggers are expected to arrive and which items block which (persistence needs the world core; `update` reuse needs the value representation; packages need modules; fork-join needs the non-atomic-count feature flag decided first).
10. **Debug/release gap table.** L116 says "the tests that pin the gap are listed below" and the table at L177–190 has two rows for it. A reader will want the within-kind confusions enumerated once (payload-free choice swap, newtype as base, `?Nat` as `Nat`, boxed `Int` read as boxed `Nat`) with the release behaviour of each (wrong answer, bounds-checked index, descriptor type-id mismatch), since L116 and L122 each give a partial list.
