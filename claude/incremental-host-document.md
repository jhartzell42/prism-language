# A small change to the window: an incremental host document, and what else it needs

*Written by Claude on the claude-self-hosting branch; subject to change. A design note for
Jimmy to choose from: no code is changed by it. Jimmy's words are quoted and marked as
his; every other call is Claude's, with the alternative refused, and Jimmy overrules by
editing. Book citations are to `prism-notes/claude/prism/`: Semantics (01), Compiler (07),
Platform (08). Line numbers in emitted code are of `prism compile bench/gui/prism
examples/chess/window` by the installed bootstrap prismc at 8127014 (`program.rs`, Rust;
`program/mod.cpp` has the same functions); the phase table is `bench/gui/prism/check.sh`'s,
on the merged tree with both seeds rebuilt (`885c89d`).*

## The question

- **The GUI table** (`bench/gui/`, js-framework-benchmark's operations over 1,000 rows,
  each an id label, a select button and a remove button) runs a small change in 32 to
  46 ms where GTK takes 1 to 60, and `bench/gui/prism/check.sh`'s second check fails:
  update10th, select, swap and remove each retire 0.6 to 0.85 times the instructions of
  Create 1,000 rows (C++ select 523 M against 650 M), where the bar is under a tenth
  (`claude/bugs/window-relayout-every-change.md`; `claude/benchmarks-preliminary.md`,
  "Instructions by phase, and the cheap runtime fixes").
- **The bug note's measurement of one C++ select:** two whole-window layouts (about 5 ms
  each), one walk of every label for the word set, five whole host-document rebuilds
  (`host_1_doc_now` in three ticks, `host_2_doc_now` in two, about 1,000 places each),
  and painting (about a seventh of the samples).
- **Each reads a structure that is one value of the whole tree**, so a change of one row
  recomputes over all rows. This note traces why, then designs the routes.

## Jimmy's constraints

- **From his ruling on this bug** (as relayed by the coordinator): the layout engine
  "STAYS IN PRISM"; "bare minimum bone simple"; no general retained-mode layout system,
  no diffing World protocol, no damage regions in the World; the one new semantic piece
  allowed so far is `@Distinct` (built: `$y <- @Distinct $x`, `^Eq`).
- **From his earlier rulings** (Semantics RULINGS): on hosts, "hosting means inserting
  delay" (HO); on structural dynamics, "Inconsistent state shouldn't be visible. Either
  the entire thing fires in the next tick (reasonable actually) or it doesn't" (ST); an
  owed tick nobody reads is not skipped (TK); a World presents only settled states (PS);
  on caching, "of course we want to cache multiply-read lets" (tests/cost/README.md).

## The cost today: one select, tick by tick

**Measured by phase** (`check.sh`'s table, millions of instructions; the merged tree with
both seeds rebuilt and the runtime fixes below in):

| target | ticks | propagate | build | patch | diff | perform | paint | the rest | total | create1k |
|---|---|---|---|---|---|---|---|---|---|---|
| C++ | 8 | 12.5 | 192.2 | 164.9 | 3.8 | 0.8 | 143.3 | 5.5 | 523.0 | 650.4 |
| Rust | 8 | 5.4 | 104.9 | 184.5 | 2.5 | 2.2 | 154.9 | 2.2 | 456.7 | 589.6 |

The build walks (ticks n+3 to n+6 below: the row documents, the host documents, the
builders' diffs) and the patch pass (the two layouts and the word set) are two thirds of
the step; painting one frame is the rest but a few percent, and is 0.22 (C++) and 0.26
(Rust) of create1k by itself (doubt PB).

The press is tick n. Names are the emitted ones: `Main` is the root node (the window, the
table's block and the column, all inlined), `MainRow` a row of the table's `for`
(`row_ops_2`), `Main1Row` a row of the window's Metrics `for`. Hosts: `host_1` is the
window's (its child is `@table`), `host_2` the column's (`@Column` inside `@table`),
`host_3` the top button row's; each `MainRow` has its own `host_1` (its `@Row`).

| Tick | What fires | What is recomputed over the whole table |
|---|---|---|
| n | the World's press at the window: `incoming`, `hit` (`placed.hits`, cached), `clicks`, `route` | nothing; the column's supplier queues its answer (`later`, host.rs:129) |
| n+1 | `host_1` delivers to the column's `!in` | the port fires to 1,001 subscribers, each row's `?InnerTo` filter (hosting.prism:43, cells.rs:383) |
| n+2 | `host_2` delivers to row k's `!in` | nothing (3 subscribers) |
| n+3 | row k's `sel`, its `acts`, the block's `!op`: `$table` written (`op`, program.rs:578) | 1,000 rows marked; 1,000 row documents rebuilt and compared; every builder of `Main` diffs its whole key set; `host_2_doc_now`; `host_1_doc_now`; the patch pass visits about 5,000 dirtied nodes |
| n+4 | row k's document published (`document_moved` to `host_2_moved`, program.rs:969) | every builder again; `host_2_doc_now` (moves); `host_1_doc_now` (does not) |
| n+5 | `host_2`'s document published | every builder; `host_1_doc_now` (moves) |
| n+6 | `host_1`'s document published: `!Updated` decides `@Distinct` (`updated`, program.rs:719) | `Claimed` and `==` over the window's children; every builder; `TextsIn $kids` walks every label; the new word `▶` gets a `Main1Row` and a `Gui.Metrics` place; layout one, in the patch |
| n+7 | the Metrics `for`'s gathered `$sizes` published (`rows_all_publish`) | layout two, in the patch |

Then `pending` is unset and the canvas World paints once (ruling PS; `app.rs:216` renders
the scene's draws).

- **Tick n+3, the marks** (`op`): every `MainRow` reads `$table` (its button's label is
  `Marked $entry, slot, $table.?selected`, bench app.prism:130), so for each of the 1,000
  rows the write calls `cx.dirty(row)`, `parent_wrote_row_ops_2_source` and
  `parent_wrote_table` (each marking the row's `host_1_doc_snap`, listing its build, and
  dirtying its three `HostRow`s), plus `cx.dirty` on the 1,001 rows of `host_2`. The
  lowering's `RowWritten` (prismc/lower/node/finish.prism:518) emits these loops.
- **Tick n+3, the row documents:** each `MainRow::build` (program.rs:1623) computes
  `host_1_doc_now` (program.rs:1511: the id label, `IdText` formatting a new text, the
  button's `Marked`, the `x` button, a map of three) and `take_compared`s it with the last;
  one row differs and queues its publication.
- **Every build of `Main`, ticks n+3 to n+6** (`Main::build`, program.rs:1335): the
  lowering's `BuildSteps` (finish.prism:346) runs every builder of the node, not only the
  marked ones, and each builder (`ForBuild`, prismc/emit/rust/templates/items.prism:236)
  walks the old rows checking `contains_key` in the new source and the new source checking
  the old rows: `row_ops_2_build` over 1,000 keys and `host_2_rows_build` over 1,001, each
  both ways, four times.
- **`host_2_doc_now`** (program.rs:821, lowered by `HostDocGetter`, prismc/lower/node/static.prism:288,
  body `DocumentBody`, finish.prism:408): a new map; for each of the 1,000 `MainRow`s,
  `row.document(cx)` (program.rs:1666) builds a vector holding the row's `Ui.Row` element
  (`children = Claimed` of the row's own lagged document) and the path
  `[2]/[k]/1` is made from `k.to_pod()` and joined; each entry is an AVL insert.
- **`host_1_doc_now`** (program.rs:838): one entry, the column's element, whose
  `children = Claimed (host_2_doc)` folds `IsUi` over 1,001 values (hosting.prism:21),
  plus a visit of `host_2`'s 1,001 `HostRow`s, none of which forwards.
- **Each snapshot is then compared whole** (`Snapshot::take_compared`): `==` on a `Map` or
  an `Array` answers at once for a node both sides share (once its values are known to
  equal themselves, a NaN staying unequal) and walks the rest in order. A host document
  is a new map each tick, no node of it shared with the last snapshot, so its comparison
  walks every entry; each entry's children, shared with the row's kept snapshot, answer at
  once. An equal take keeps the snapshot it had, so an unchanged structure keeps its
  sharing with the last tick.
- **Tick n+6, the window** (examples/chess/window/widgets.prism:120): `$kids <- @Distinct
  (Claimed $elems)` compares the new children with the held ones (one `==` of the column
  element, its 1,001-entry children map walked to the first difference); `$sizes <- for
  text <- TextsIn $kids` recomputes the word set (layout.prism:154: every label's `Words`,
  built a character at a time by `"{word}{c}"`, layout.prism:128); and `$placed =
  PlaceAt (LayRoot ...)` (widgets.prism:128) lays out every row: `LayStack`'s passes over
  1,001 children, and per row a `LayStack` of three leaves whose labels call `Words` and
  `WordsW` again. Only what shows builds draw commands.
- **Tick n+7, the second layout:** the new word's metrics are not late from the World. The
  canvas World answers `Gui.Metrics`' first value synchronously from its font
  (worlds/canvas/rust/src/world.rs:138), so `▶`'s extent is in the new `Main1Row` at n+6.
  The lag is the Metrics `for`'s: its gathered `$(Text ==> Extent)` is a structural
  dynamic, published at n+7 (`rows_all_publish`), so layout one used `▶` at zero width
  and layout two corrects it (the bug note's doubt MT).
- **Painting:** one 900 by 800 frame per settled state, whatever the change: 143 M
  instructions in C++, 155 M in Rust, as much for clearing the table as for a select.

What this adds to the bug note's list (derived from the code; the phase table measures
them only as a group): the 1,000-row marking and row documents at n+3, the whole-key-set
diffs of every builder in four ticks, and the whole-map comparisons are the build phase;
the routing fan-out at n+1 is in propagation (under 3% of the step); the patch pass over
the dirtied rows is in the patch phase with the layouts.

### Why each is proportional to the table

- **Inherent in the semantics as the books state them:**
  - A host's document is one value: "**Its type is `$(Path ==> Elem r)`,** a map from
    paths to elements" (Semantics ch. 8, "The element tree is a dynamic"); "The result is
    the child's own result and its document" (ch. 9, the forms). A reader of `$doc` gets
    the whole map; nothing in the language says which entries moved.
  - It is a structural dynamic: "Its value after n is fixed once, as the last act of
    construction" and "Its update event fires at n+1" (ch. 7, round 3 ST), so each
    publication owes a tick, and owed ticks are not skipped (ruling TK).
  - A computed answer is the next tick's input (ch. 9, "How a supplier answers"), so the
    three routing ticks n to n+3 are one per box (window, column, row).
  - A dynamic recomputes when an input it depends on fires (ch. 7, R3; Platform I): every
    row's label reads the selection, so a select recomputes 1,000 labels. This is the
    program's structure (the js-framework-benchmark spelling), not the library's.
  - Values are persistent and an update copies one path (Semantics ch. 5, "Cost
    promises": "A statement on a shared part copies the path ... once per block"; Appendix
    B, Maps: `Insert` log): the sharing that would let a reader skip an unchanged subtree
    exists in the representation, but Prism "exposes no identity operation" (ch. 11), so a
    Prism reader cannot use it.
- **Implementation choices in the lowering and runtimes:**
  - The whole document rebuilt from every place when any place moved (gap DM,
    prismc/README.md: "*Next, if it shows in a profile:* the snapshot updated per place").
  - A tick per nesting level: an outer document's live computation reads the inner `$doc`
    through its lagged getter (`host_2_doc`, `Snapshot::get`, S(n−1)), so the window's
    document moves two ticks after the column's (prismc/README.md, ST: "The dynamic's
    value after m is S(m−1)"). Doubt LG asks whether the books mean that.
  - Every builder of a node runs on every build of the node, and diffs whole key sets.
  - A host document is a new map each tick, so `==` on it walks every entry.
  - `Port::fire` copies the subscriber list before calling it (cells.rs:385).
- **The window library's own structure:**
  - Layout is one function of the whole tree (`$placed`); so is the word set
    (`TextsIn`); Platform I says so in its scenario: "Today's window library throws this
    away by laying out the whole tree as one function."
  - `Words` builds each word a character at a time.
  - The Metrics `for` adds a tick before the layout is right.
  - `?Boxed` answers each child with its own filter over the box's one `!in`, so a routed
    input visits every child of the box.

## Route B: an incremental host document

**What "the places that changed" means:** the owner already knows, per tick, which child
places' levels may have moved, because it marks them: a row's document published
(`document_moved`), a row built or torn down (its builder), a hold written behind one of
the child scope's own places (`HoldWritten`, finish.prism:164). B threads that knowledge
into the snapshot instead of discarding it.

- **Data structures, per host k in its owner** (Claude's):
  - `host_k_doc_snap` as today: a `Snapshot<Map<Path, Elem>>`, persistent.
  - `host_k_own_moved: Cell<bool>`: a hold behind one of the child scope's own places
    was written (the scope's own places are few, statically known: re-read all of them).
  - `{for}_touched: RefCell<Set<K>>` per `for` in the child's scope: the keys whose row
    published its document, came, or left this tick.
  - Per hosted row, `doc_paths: RefCell<Vec<Path>>`: the paths it contributed last time,
    so its old entries can be removed exactly (a row with forwarded places under it
    contributes several).
- **What lowering emits:**
  - `host_k_doc_next(prev) -> (Map, moved, came, left)` in place of `host_k_doc_now`
    (`HostDocGetter`): start from the newest snapshot; if `own_moved`, re-insert each own
    place's level; for each touched key, remove its row's `doc_paths` and insert its
    current `document()`; keys that left are removed. `moved` is whether any re-inserted
    entry differs from the one it replaced, or any key came or left: a comparison per
    touched entry, not per document.
  - `document_moved` (finish.prism:401) passes the row's key: `parent.host_k_moved_row_m(key)`
    records it, then marks as `host_k_moved` does today.
  - `HostMoved` (finish.prism:335) sets `own_moved` when the write is behind an own place.
  - A child `for`'s builder in a hosted scope adds the keys it builds and tears down.
  - The host's own `for` (`host_k_rows_build`) builds and ends rows from `came` and `left`
    instead of diffing the document's keys.
  - The take is `take_moved(v, moved)`, publishing exactly when `take_compared` would.
- **What the runtimes hold:** nothing new but `Snapshot::take_moved` (about 15 lines each)
  and a `Row` method for the paths (`doc_paths`); `Map`'s `insert` and `remove` are
  enough.
- **What the window library writes:** nothing.
- **What the books would say:** Semantics is unchanged (the document's value is the same).
  Appendix B, Widgets: "**`@HostDocument`, `@HostSome`, `@HostList`:** chapter 9's
  signatures; constant per child place, plus the supplier" gains: "a tick costs each
  child place whose level may have moved, logarithmic in the document, and nothing for
  the places that did not". prismc/README.md gap DM becomes the design.
- **Transcripts:** none change. The snapshot is the map today's rebuild makes, published
  in the same ticks.
- **Cost per tick afterwards,** with c touched places at a level of n and depth d:
  c log n per level, d levels, in the ticks that already exist. A select touches one row
  per level. The 1,000 row documents of n+3 remain (the rows read the selection) at about
  a map of three each.
- **What B does not fix:** any reader of `$doc` that walks it: `Claimed` (one `IsUi` per
  child each time the column's entry is re-read), `@Distinct`'s `==` unless equality
  skips shared nodes, `TextsIn`, both layouts, the routing fan-out, the tick per nesting
  level, painting. B makes readers proportional to the change *possible*; it makes none
  so. *Refused:* a change set carried beside `$doc` for Prism readers (a `!Changed` of
  paths): a new semantic piece, and a reader would still need somewhere to keep per-child
  results, which is route A or C.

## Route A: per-child layout and word sets in the window library

- **The design:** the window splits its children with `for p, $e <- $kids` and lays each
  out in its row, `@ $(LayKid $e ...)`; the gathered `$(Path ==> Laid)` feeds the
  window's stack. A box's children are a value inside its element, not places the window
  hosts, so a second level needs a `for` over a dynamic lifted from the row's entry
  (`let $cs = ChildrenOf $e`), and so on per level.
- **The lag is real, with or without `@Distinct`.** A gathered `$(K ==> T)` is structural,
  and so is any dynamic lifted from it (ch. 7, ST: "and every dynamic lifted from one of
  these"); `@Distinct` drops equal refires and adds no promptness. Two levels, a label in
  a row in the column, under ST as prismc implements it:

  | Tick | Level 2 (the row's `for` over its leaves) | Level 1 (the window's `for` over rows) | `$placed` |
  |---|---|---|---|
  | m | the label's `$(LayLeaf $c)` moves; gathered S2(m) taken, published m+1 | its row reads S2(m−1): no change | old |
  | m+1 | | the row's `$(Stack $inner)` moves; gathered S1(m+1) taken, published m+2 | old |
  | m+2 | | | new, in the patch |

  A tick per level after the document's own lag; a World that presents only settled
  states (PS) shows no torn frame, but every window transcript (Window.Tests, chess)
  gains the depth in owed ticks per change.
- **`Laid` holds a closure and has no `^Eq`,** so a gathered `$(Path ==> Laid)` cannot be
  compared: every take moves and every change owes a publication per level, unchanged
  rows included.
- **What it fixes:** layout and the word set proportional to the rows that changed, plus
  the stacks' combining passes. **What it does not:** the documents (B), the routing,
  painting.
- *Refused (Claude's):* per-box layout carried in the elements (`Ui.Column(laid=$laid)`,
  each box widget laying out its own children): it removes the window's `for`s but is a
  retained layout spread over every box, which Jimmy refused.

## Route C: a cached call (the third route, and the one Claude recommends with B)

Layout and the word set need somewhere to keep each child's last result. A `for` row is
such a place (route A) and costs a tick per level. A cache keyed by position is the other,
and costs nothing in the semantics: Prism "exposes no identity operation", so "sharing is
invisible" (Semantics ch. 11), and a cached pure call is sharing.

- **The prelude function** (Claude's spelling): `Cached: fn (fn k, a -> b), k, a -> b
  where k: ^Ord, a: ^Eq`. `Cached F, k, x` is `F k, x`: no program can tell them apart.
  The cost promise: when the last `Cached F, k, _` call with this `k` had an argument
  `==` to `x`, the call costs one lookup (log in the keys) and one `==`; otherwise it
  costs `F k, x` and keeps the result under `k`. One slot per key.
  - `F` is a named top-level function (`cached-needs-name` for a lambda, whose captures
    would be part of its meaning); `k` is where the result belongs (a chain of paths, a
    text), `x` everything else `F` reads.
  - *Refused:* `Memo F, x` keyed by the whole argument (needs `^Ord` on records of
    `Float`s, which I did not check the printers derive, and a table that grows with every
    distinct argument); memoising by the lowering's choice with no mark in the text (a
    program's cost would stop being a property of its text, Appendix B TD-V2); a
    `let`-cache generalised to calls inside plain functions (a plain function has no node
    to hold it).
- **What lowering emits:** a builtin, beside `DistinctLine` (static.prism:337): a call
  `Cached F, k, x` becomes a lookup in a table for `F` (a thread-local in Rust, a function
  `static` in C++), `F`'s ordinary call on a miss. The checker types it from its prelude
  signature.
- **What the runtimes hold:** `prism_rtl::Cached<K, A, B>`: an ordered map from `k` to
  `(x, result)`, and a sweep. *Eviction (Claude's):* each slot records whether it was used
  since the last sweep; a sweep runs when the calls since the last one reach twice the
  table's size, dropping the unused slots; so the table holds at most about twice what
  one pass over the tree uses, and nothing is swept between two layouts with no calls in
  between. The results are plain values (a plain function makes no node or port).
- **What the window library writes** (Prism, layout.prism):
  - `Words` through the cache, keyed by the text: `fn CachedWords t = Cached WordsOf, t,
    unit`; `TextsOf`, `LayLabel` and `Wrap` call it. The word-set fold stays one walk of
    the tree, at a few `Insert`s per label.
  - A label's laying out through the cache, keyed by its chain: `LayLabel` computes the
    extents of its own words and of `" "` (the only part of `env` it reads) and calls
    `Cached LabelAt, chain, ((t, room, extents))`, where `LabelAt` is today's `LayLabel`
    body over those extents instead of `env`. A new word elsewhere (`▶`) changes no other
    label's key, so layout two re-lays only the selected row's label.
  - Containers are not cached: a stack's passes over its children stay, at a few
    operations per child once the leaves hit.
- **What the books would say:** Appendix B, a new entry beside `@Distinct`: "**`Cached:
  fn (fn k, a -> b), k, a -> b where k: ^Ord, a: ^Eq`:** `F k, x`. When the last call of
  `F` through `Cached` with key `k` had an argument equal to `x`, one lookup and one
  `==`; otherwise `F k, x`, kept under `k`." Semantics ch. 11 gains one sentence: a
  cached call is sharing, so no program sees it. Platform I's terms refuse "memoization
  (caching, not propagation)" as a word for incrementality; `Cached` is caching and is
  not called incrementality.
- **Transcripts:** none change.
- **Cost per layout afterwards,** n rows, c changed, v visible: the stacks' passes, a few
  operations per row; c leaf layouts; the word walk at a few inserts per row; draw
  commands for v. Still a pass over the rows, with a small constant per row.
- **What C does not fix:** the documents (B), the tick per level, the second layout's
  existence (it becomes cheap, not gone), routing, painting.

## Route D: settle nested structural dynamics within the tick (needs a reading, doubt LG)

- **The reading (Claude's):** ST fixes a structural dynamic's "value after n ... as the
  last act of construction", and "a builder in n's construction step and the tick's
  outputs read the value after n" (ch. 7). An outer document whose entry's `$` field is
  lifted from the inner `$doc` would then include the inner document after n. prismc
  takes the value after m to be S(m−1), and reads the inner through it; ruling VL already
  accepted one such extra tick ("following ST as `prismc` implements it").
- **The design:** inner snapshots are taken first (they are: `BuildSteps` runs scope by
  scope, innermost first; a row's snapshot is taken in its own build), an outer live
  computation reads the inner's S(n) (`Snapshot::current`), a row whose snapshot moved
  lists its owner's build again in the same construction step, and the patch reads S(n).
- **What it buys:** a select's documents in one tick (two rebuilds, not five, without B;
  one entry per level with B); one layout, with `▶` measured (doubt MT goes); four fewer
  owed ticks.
- **What it costs:** every transcript with a nested host or a gathered `for` read by a
  place moves (fewer ticks); the books say which reading holds. Not needed for the bar if
  C is built.

## Without a route: values that keep their sharing (no transcript changes)

- **Built, both runtimes:** `Snapshot::take_compared` keeps the snapshot that stood when
  the new one is equal to it, so an unchanged row's document and every element built from
  it keep their nodes across ticks; `==` on maps and arrays answers at once for a node
  both sides share at the same place, at any depth, once the node knows its values equal
  themselves (a shared NaN stays unequal: `Float` `==` is IEEE equality, Type System ch.
  2, so pointer equality is never the answer). Safe by Semantics ch. 11: "sharing is
  invisible"; the same values, the same moves, the same firings.
- **Measured, they move nothing** (`claude/benchmarks-preliminary.md`: every instruction
  ratio within 1% of 1): the host documents are new maps each tick, so the comparisons
  still walk every entry. With B they make a document compare cost its changed paths.
- **Not built: a builder skips when its source is the node it last built from**
  (`ForBuild`, both printers): `row_ops_2_build` in ticks n+4 to n+6 reads the cached
  source, unchanged. It is emitted code, in prismc's templates.

## Recommendation

**B and C (the runtime fixes above are in); D held for Jimmy's reading; A refused.**

- **Why both:** the bug note's own figures put one whole layout at about 5 ms against
  create1k's 55 ms, so a single whole layout is already about the bar; the layout reader
  must stop re-laying unchanged rows, which B cannot do, since B changes what `$doc` is
  computed from, not what reads it. And the documents are five whole rebuilds plus the
  comparisons and builder diffs around them, which C cannot touch. Each is the smallest
  piece for its half: B is the gap DM's own next step, invisible to programs; C is one
  prelude function, invisible to programs.
- **Why not A:** a tick per box level on top of the documents' own, every window
  transcript moved, and a `Laid` that cannot be compared. Under D's reading A's lag at
  outputs would go, and A would be the books' own route (Compiler ch. 1: "A translator
  hosted as a document recomputes its output where its input changed, by the semantics
  of `for` and of live fields"); it would still owe a tick per level.
- **The strongest argument against:** C is a second new semantic piece where Jimmy has
  allowed one, and its efficiency rests on a sweep policy no book states, so how often a
  cache hits is the implementation's, not the text's; and neither B nor C delivers
  Platform I's "a tick's work is proportional to the dynamics reachable from the inputs
  that changed": a pass over the rows with a small constant remains (the stacks, the word
  walk, the 1,000 labels that read the selection, the routing fan-out, the patch visits).
  The bar is a ratio, so a small constant per row may pass; "a change costs the change"
  it is not.
- **The first step, done:** `check.sh` splits each step's instructions by phase (the table
  in "The cost today"). Painting alone is 0.22 to 0.26 of create1k, so the bar is not
  reachable while the World paints the whole frame (doubt PB); C and B together could
  bring the build and patch phases down, not painting.
- **Size (Claude's estimates, not counted):**
  - C: the prelude line; about 80 lines of lowering (the builtin, the checker's refusal);
    about 80 lines per runtime; about 30 lines of window library.
  - B: about 250 lines of lowering (`static.prism`'s getter, `finish.prism`'s
    `HostMoved`, `DocumentMoved`, `HoldWritten`, the host rows' builder), about 80 lines
    per printer's templates, about 30 per runtime.

## The books' sentences that change

- **For B:** Appendix B, Widgets: the `@HostDocument` line gains the per-tick promise
  above. Nothing in Semantics ch. 8 or 9: the document's value and its ticks are unchanged.
- **For C:** Appendix B gains `Cached` beside `@Distinct`; ch. 11 gains the sentence on a
  cached call; Appendix B's `@Distinct` entry ("Nothing dedupes unasked (chapter 7); this
  is the opt-in") is the model for the wording.
- **For D:** ch. 7's ST paragraph gains: a structural dynamic lifted from another's value
  reads its value after n in n's construction step, so nested documents settle in one
  tick; RULINGS VL is revisited.
- **For A:** none; it is Prism code under today's rules.

## Failing tests first

- **`tests/cost/host-one-row-100` and `-1000`** (the Test World, a `@HostSome` box over a
  `for` of N labels, one label changed): the thousand must cost under twice the hundred.
  Fails today; passes with B.
- **`tests/cost/window-one-label-100` and `-1000`** (the window library under the Test
  World, as `Window.Tests` runs it; N rows, one label changed): under twice. Fails today;
  needs C and B.
- **`tests/conformance/prelude-cached`** (both targets): `Cached F, k, x` equals `F k, x`
  across ticks, through an argument that changes under a kept key, two keys, and a sweep.
  Checker `neg/cached-needs-name` (a lambda), `neg/cached-needs-ord`.
- **`tests/conformance/host-doc-row-returns`:** a key that leaves and comes back in one
  tick, with forwarded places under a nested row, the delta's hard case; today's host
  tests (`sem-ch09-hosts`, `sem-ch09-host-nested`, `sem-ch04-row-source`) pin B's values
  unchanged. *Refused:* checking B's document against a whole rebuild in a debug mode, a
  second implementation as the oracle.
- **`bench/gui/prism/check.sh`, the second check:** the gate.

## Migration order

1. The phase split in the check: done.
2. The runtime fixes (but the builder's skip, which is prismc's): done, no measurable gain.
3. C: Appendix B and ch. 11 first, then the builtin, the runtimes, the window library.
4. B: gap DM's next step, then its cost test.
5. The check rerun; the bug note updated or removed.
6. D, if Jimmy rules the reading; then every moved transcript, derived by hand.
7. Painting, by Jimmy's answer to PB.

## Doubts

- **PB** (Claude's) Painting one frame is 143 M instructions in C++ and 155 M in Rust,
  0.22 and 0.26 of create1k (measured by phase), so the bar is unreachable while the World
  paints the whole frame. Count the program's phases only (propagate, build, patch, diff),
  or the whole process as today?
- **CA** `Cached` as the second new prelude piece after `@Distinct`: yes, or no and live
  with a whole layout per change?
- **CK** (Claude's) `Cached F, k, x`, one slot per key; or `Memo F, x`, keyed by the whole
  argument? Taken: the key.
- **EV** (Claude's) The sweep (unused slots dropped when calls reach twice the table) is
  the implementation's and no book's; or should Appendix B state it?
- **LG** Does ST mean a nested document settles in the tick (Claude's reading of ch. 7:
  the value after n read in n's construction step), or a tick per level (prismc, and
  ruling VL for `@Flatten`)?
- **AB** Appendix B's "constant per child place" for `@HostDocument`: is today's whole
  rebuild per tick outside that promise (then B is a fix, not an optimisation)?
- **RA** (Claude's) Route A refused for its tick per level and its transcripts: agree?
- **SE** (Claude's) The table's rows each read `$table.?selected`, so a select recomputes
  1,000 small row documents: leave the benchmark in js-framework-benchmark's natural
  spelling? Taken: leave it.
- **WS** (Claude's) `Words` built a character at a time: take `SplitOn` (prelude.prism:186,
  its pieces views of the text) as well, a constant factor for every miss?
- **RF** (Claude's) Routing visits every child of a box through its own filter (1,001
  at the column per press): leave it, linear and small; or route by path, a runtime
  piece?
