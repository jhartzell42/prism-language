# Lint: ch11-rust-lang.md

Chapter: /home/claude/book/ch/ch11-rust-lang.md (1390 lines). Line numbers refer to that file.

## Part 1: rule violations

### Rule 1 — History / superseded designs (0)

None. "the previous chapter" (L9, L886) is a cross-reference, not a superseded design. L926 "Today's `AnyValue` has inline `U32`, `I32` and `Unit`… and the representation is going to change" is a current-state reference the BIBLE permits; see structural note 9 for why it still reads badly in a reference.

### Rule 2 — Status tracking (0)

None.

### Rule 3 — Quoting or attributing Jimmy (0)

None.

### Rule 4 — Banned words (0)

None. `Shape` (L426, L686) is a type name in an example.

### Rule 5 — Clefts and non-default word order (2)

1. L696: "What the refinement buys is that an interceptor with one arm per tag it handles is typed at those tags, and an interceptor that forwards what it does not know is typed generically and checked at the slot, which is what the elements chapter asks for and no more."
   Pseudo-cleft. Fix: "The refinement buys this: an interceptor with one arm per tag it handles is typed at those tags, and one that forwards what it does not know is typed generically and checked at the slot."
2. L1365: "What the listing is good at, said plainly: it is readable as a work order, it can be hand-written as a fixture before any parser exists, it is checkable in one pass, and an SSA backend is a short step from it."
   Pseudo-cleft plus filler ("said plainly"). Fix: "The listing is readable as a work order, can be hand-written as a fixture before any parser exists, is checkable in one pass, and is a short step from an SSA backend."

### Rule 6 — Lecturing tone (1)

1. L190: "Rule by rule from the language chapter: the stack is rule 1; `pending` is rule 2; the three comparisons are rule 3; `Colon` for an inline block is rule 4; `depth` is rule 5; … end of file is rule 10."
   Refers to another chapter's rules by number (the BIBLE: refer by content, never by number), and the numbers will drift. Fix: name each rule by what it says ("the stack is the column rule; `pending` is the line-final-colon rule; …") or cut the mapping and keep only the two corpus cases that follow.

### Rule 7 — Dry definition before motivation (4)

1. L634 "### Literals" opens: "An integer-spelled literal is `Nat | Int` by inference: the elaborator gives it a fresh variable with a `Num` constraint and a *spelling restriction*…"
   Lead with the counter's `-1`, `+1` and `0` (the paragraph at L638 is the lived example) and derive the rule from how they get their types.
2. L642 "### `for`" opens with the node form and the consolidation table.
   Lead with the todo list: a row returning `!Delete` gives one `!(Id ==> Delete)`; a row returning a `$Text` gives one `$(Id ==> Text)`; then the table.
3. L686 "### Exhaustiveness and the catch-all" opens with the rule list.
   Lead with the forwarding interceptor whose `match e:` would silently stop forwarding when a new tag is declared; the open-set catch-all requirement falls out of that.
4. L690 "### The tag-indexed refinement" opens with the `Slot` type and a rule list.
   Lead with the sandbox arm `Fs.Contents(path=p): …` wanting `_.!request` at `!Text` and the catch-all wanting it generic, then the rule.

### Rule 8 — Presence or drop as a trigger (1)

1. L1237: "`!deletes` fires when a key appears, because its row's result is `Delete on !started`; a row that fires at birth is a prompt path from the collection"
   A diagnostic text that says an event fires because a key appeared. The cause is `@started`. Fix: "`!deletes` fires in the occurrence a row is built, because its result is `Delete on !started`; `@started` fires at build, so the collection's change reaches `!deletes` promptly".

### Rule 9 — Design conflicts with the BIBLE Decisions (9)

1. L934: "each `Bind` is one child node, in step order, with the next `Bind(i)` segment; each `Element` is one slot, sharing the counter with child binds" and "`0/#1/0` is the first bind's `dyn`'s second generation's first slot".
   BIBLE: a plain widget-function application makes a node but adds no segment; only slots, `dyn` generations, `for` keys and intercept children take segments (the runtime chapter's L825 says the same). Fix: "a plain `Bind` makes a node and adds no segment; a slot, a `dyn`, a `for` and a captured child each take the next position in the nearest addressable node; `0/#1/0` is the first `dyn`'s second generation's first slot". (L1331's paths are already consistent with the BIBLE rule.)
2. L102, L374, L426, L1122, L1134: `Gui.Button` with `label: Text` and `responds: Click` ("`tag Gui.Button of Element:` with `label: Text` and `responds: Click` becomes one row", L1134; "`tag Gui.Button of Element:` opens a block whose statements are fields (`label: Text`) and parts (`responds: Click`)", L374; `["Gui", "Button"]`, L1122; "`Gui.Button` is one token", L426; `// Gui.Button, Fs.File, Wrap.Add`, L102).
   BIBLE: `Gui.*` is only `Gui.Window`, `Gui.Scene`, `Gui.Focusable`, `Gui.Metrics`, `Gui.Frames`; a button is `Ui.Button`. Fix: `Ui.Button` in every one of these (L368, L1020, L1331 already use it). L1172's `Gui.Text.Input` is likewise not a tag; use a `Ui.*` example or say "a hypothetical `Ui.Text.Input`".
3. L545: `For { coll: Box<TExpr>, block: Box<TLambda>, kind: Consolidation, keyed: bool }` and L992: `'for' expr kind lambda`.
   BIBLE: the elaborator records "can fire at birth" on each `for` node and the text form prints it (the Haskell evaluator's one `coincidence` keys on it). Fix: add `birth_firing: bool` to the `For` node and print it (`'for' expr kind ('birth')? lambda`), and say in the causality section (L671) that the flag computed there is stored on the node.
4. L603: "elaborates as the binding `!click: !Click [build] = Switch { events: Dyn { … } }`".
   BIBLE and the chapter's own L656/L1084: a binding whose root is a graph node is `[occ]`, and `Switch` is a root lift. Fix: `[occ]`.
5. L869: "`event.tag_map(dyns, closure)`" and L886: "`tag_map` over a vector of dynamics".
   BIBLE names the n-ary target constructors `hold_first`, `map_n`, `tag_map_n` (the runtime chapter's `tag_map` is the binary one). Fix: `tag_map_n` in both places.
6. L786, L800, L804–L811, L895, L912, L751, L830: `AnyValue` as the value type (`pub constants: Vec<AnyValue>`, `captures: Rc<[AnyValue]>`, `impl PolymorphicWidget<AnyValue> for IrWidget`, `Rc<dyn Fn(&Frame) -> AnyValue>`, `Value<AnyValue>`).
   BIBLE: the boundary's plain value type is `Value`; the typed wrapper is `Typed<T>`; the runtime chapter writes `PolymorphicWidget<Value>`. Fix: `Value` throughout, `Typed<Value>` for the closure's typed argument at L912, and L751's "`Value<T>` downcast … about `AnyValue`" becomes "`Typed<T>` downcast". Keep L926's "today's `AnyValue`" only as the statement of the current representation behind `boxing.rs`.
7. L877: "`Started` | `Started` | the builder's done event filtered to `unit`: fires once at `height + 1` in the build occurrence".
   BIBLE: `@started` is a pure primitive, a `dyn` over a constant; the chapter's own L438 says so, and under the path rule a `dyn` takes a position (runtime chapter L825). Fix: "`Started` | `Dyn` over a constant | `dynamic_widget` over `Dynamic::constant(unit)`; its event is the `dyn`'s result event, firing once in the build occurrence; it takes a path position like any `dyn`".
8. L102, L426: `Fs.File` ("unknown tag or function `Fs.Fil`; did you mean `Fs.File`?").
   The file tags the BIBLE names are `Fs.Contents` and `Fs.Watch`. Fix: `Fs.Contents` / "did you mean `Fs.Contents`?".
9. L782–L788: `Program { functions, blocks, tags, constants, main }`.
   BIBLE: `Program { functions, blocks, tags, constants }`. Minor: either drop `main` (the root block is the `IrWidget`'s `block`) or add `main` to the decision; the chapter's own `IrWidget` already names the block, so `main` is redundant.

### Rule 10 — Unverified specifics (5)

1. L749: "The lint is Prism's `-dcore-lint`."
   GHC flag name, not in the research notes. Verify (it is GHC's Core lint flag) or write "the lint is the compiler's Core-lint" or mark from memory.
2. L1343: "Cranelift's input is SSA with block parameters".
   Not in the research notes. Verify or mark from memory or cut (the paragraph only needs "a native backend's input is flat SSA").
3. L1331: "with its transcript under `harness/goldens/counter.txt`".
   The runtime chapter does not name that path. Verify against ch10 or drop the path and say "the runtime chapter's counter transcript".
4. L739 and L761: "The runtime chapter lists the leak test that pins this" / "the leak test in the runtime chapter".
   ch10 has no sibling-capture leak test (its only "leak" is `trace`). Either add the test to ch10 or name it here as this chapter's test (`a_compiled_closure_captures_by_environment_index_and_not_by_rc`, L1335, already is).
5. L886: "Three of the runtime constructors in the table are ones the previous chapter gives the runtime for the compiler's sake: `hold_first`, `map_n` …, and `tag_map` over a vector of dynamics".
   ch10 defines `map2` and a binary `tag_map` and no `hold_first`. The BIBLE blesses the three as the target API, so the fix is "the runtime's target API has three constructors for the compiler's sake" and a matching line in ch10.

### Rule 11 — Code that cannot be Prism (1)

1. L682: "the counter's author who writes `$count <- 0 <= $count + 1` (no event: a derived dynamic reading itself promptly) learns about it at the first occurrence instead of at compile time".
   A hold's right side must be an event; `$count + 1` is a `$Int`, so this is a sigil/type error at the `<=`, not a cycle the causality check finds. Fix: use a real prompt cycle, `let $a = $b + 1` / `let $b = $a * 2` (the chapter's own L1229), or `$count <- 0 <= !Updated $count` (a hold fed promptly by its own change event).

### Rule 12 — Filler (4)

1. L25: "and it is worth listing what is absent, because a model handed this chapter will otherwise reach for each of these out of habit."
   Fix: "What is absent, listed because a model handed this chapter will otherwise reach for each of these out of habit:".
2. L733: "and that is fine, for a reason that is worth being exact about: the dynamic's node does not hold the closure."
   Fix: "and that is fine: the dynamic's node does not hold the closure."
3. L1142: "as the elaboration section describes" — scaffolding. Fix: cut.
4. L1172 "as above" and L1373 "*Register code and native backends.* Above." — scaffolding. Fix: L1172 cut; L1373 "A consumer of the converted typed AST; the typed AST does not change."

Counts: rule 1: 0; rule 2: 0; rule 3: 0; rule 4: 0; rule 5: 2; rule 6: 1; rule 7: 4; rule 8: 1; rule 9: 9; rule 10: 5; rule 11: 1; rule 12: 4. Total 27.

## Part 2: structural notes

1. **Marks and steps disagree about what `Occ` means.** L656 makes `Leftmost`, `Switch`, `Join`, `Updated`, `Distinct`, `HoldFirst` and an event/dynamic `For` `Occ` roots; L888 says an `Occ` binding "is a step that constructs a graph node holding a closure: `Map`, `Tag`, `DMap`, `Distinct`"; L875 says `Updated` builds no node at all. Say which of the two readings the lint's "marks are right" check (L755) enforces, and what `[occ]` means for a binding like `!delta = leftmost […]` that holds no closure.
2. **Bundles are used and never defined.** "a bundle" appears in the dynamic-of-widgets table (L611), the `for` table (L650) and as `Step::Project` (L857), but nothing says what a bundle is (a record whose fields are sigiled?), how a widget returns one, or how its projections print (`name.field`, L971). One example with its dump line would settle it.
3. **Sequential `let`s have no typed-AST node.** The parser has `LetIn` (L274) and the resolver has `Binding::LetSeq` (L406), but `TExpr` (L516–L551) has no let node and the leaf evaluator (L905) lists no let. Say where a lambda body's `let`s go (a `Let { pat, value, body }` leaf node, or inlined) and how they print.
4. **Globals.** `Operand::Global(ItemId)` (L825) is "a plain constant computed once per program"; nothing says when top-level non-lambda `let`s are evaluated, in what order, whether they may refer to each other, or what happens when one aborts. A reference reader will need this when the first top-level `let` with a `Fold` in it appears.
5. **Runtime names drift from the runtime chapter.** `Rc<Tags>` here (L785, L1134) vs ch10's `Rc<TagTable>`; `PolymorphicWidget<AnyValue>` vs ch10's `PolymorphicWidget<Value>`; `builder.element(elem_value, request)` and `with_span` are not shown in ch10's builder API. One of the two chapters should own each name and the other quote it.
6. **"A program cannot name a path" (L1151) contradicts the list type.** `$(Path ==> Slot)` (L1180) is a dynamic of a map keyed by `Path`, so programs do hold paths. State the invariant precisely (a program reads paths from the list it was given and can neither mint one nor address a slot by one; the World and the runtime are bound; the type system enforces it by giving `Path` no constructor) and say who is bound.
7. **Dictionary passing through lifts is unstated.** `Dict { Ord, a }` (L626) is a hidden first argument at a call site where `a` is concrete; nothing says what happens when the bounded call sits inside a `Tag` or `DMap` body in a generic widget function, nor how `dict[Ord a]` (grammar, L1002) prints in a real line. A dumped `Max<a: Ord>` would close both gaps.
8. **Missing dumps for the goldens the chapter promises.** L1313 lists `for` in each consolidation, a cyclic group cut at a switch, an interceptor with a tag-specific arm and a catch-all, and a generic function with a bound; the chapter dumps only the counter and the dynamic-of-widgets lift. A `for` dump (with the birth-firing flag) and an `intercept` dump (generic `element`, supplier capture list, the `$(Path ==> Slot)` result) are the two the differential test depends on most.
9. **Plan and date material in a reference chapter.** L1109 ("it stops changing before the Haskell frontend starts… a comment at its head records the date it was frozen"), L1331 ("the milestone that ends the compiler's first phase"), L926 ("Today's `AnyValue`… the representation is going to change"). State the frozen form as the form and the boxing seam as the design; the sequencing belongs to the plan chapter.
10. **Duplication.** "What is deferred" (L1369–L1389) restates the end of "Deep in" (L1365: packed encoding, register backends) and the traits negative space (L632 vs L1377) almost verbatim; the tag table's module-system paragraph (L1136) and L1379 say the same thing twice. Keep each once, in "What is deferred", and point there from the sections.
