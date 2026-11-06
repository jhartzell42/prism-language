# A small change to the window: the host document as a map of dynamics

*Written by Claude on the claude-self-hosting branch; subject to change. A design note:
what a small change to the window costs and why, Jimmy's rulings on it, and the design of
the host document as `$(Path ==> $Elem c)`, which he ruled on 6 November 2026 and which is
designed here and not yet built. Jimmy's words are quoted and marked as his; every other
call is Claude's, with the alternative refused, and Jimmy overrules by editing. Book citations are to `prism-notes/claude/prism/`: Semantics (01), Compiler (07),
Platform (08). Line numbers in emitted code are of `prism compile bench/gui/prism
examples/chess/window` by the installed bootstrap prismc at 8127014 (`program.rs`, Rust;
`program/mod.cpp` has the same functions); the phase table is `bench/gui/prism/check.sh`'s,
on the merged tree with both seeds rebuilt (`885c89d`).*

## The question

- **The GUI table** (`bench/gui/`, js-framework-benchmark's operations over 1,000 rows,
  each an id label, a select button and a remove button) runs a small change in 32 to
  46 ms where GTK takes 1 to 60: update10th, select, swap and remove each retire 0.6 to
  0.85 times the instructions of Create 1,000 rows (C++ select 530 M against 719 M);
  `bench/gui/prism/check.sh` reports the ratios and fails on none
  (`claude/bugs/window-relayout-every-change.md`; `claude/benchmarks-preliminary.md`).
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
- **From his rulings of 5 and 6 November 2026** (as relayed by the coordinator):
  - no `Cached` prelude function and no hidden runtime cache: a cache, if any, is a hold
    in the window library in plain Prism;
  - no bar of a tenth of create1k ("never his"): the check reports the numbers;
  - one tick per level stands: every host level and every gathered `for` adds exactly one
    tick, and prismc is right as it is; route D (nested documents settled within a tick)
    is refused, and so, under the flat document, was route A (per-child `for` rows for
    layout);
  - the host document is `$(Path ==> $Elem c)`: "No, change everything for this. The
    other way is quite stupid." This drops route B, the flat document patched in place:
    with this type the owner never fuses its rows into one map.
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
    value after m is S(m−1)"): one tick per level, as Jimmy rules.
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

## The host document as `$(Path ==> $Elem c)` (Jimmy, 6 November 2026; designed, not built)

Jimmy, asked whether the document "should be `$(Path ==> $Elem c)`": "No, change
everything for this. The other way is quite stupid." The outer `$` moves when a place
comes or goes; each inner `$` moves when that one place's element changes. The flat
`$(Path ==> Elem c)` goes everywhere. What follows settles the design from the books and
the compiler as they stand; every call marked "(Claude's)" is a doubt below.

### 1. The signatures, and what an element holds

```prism
@HostDocument: fn @a emit c, (fn Path, Element c o -> @o) -> @((a, $(Path ==> $Elem c)))
@HostSome: fn @a emit c, (fn Path, Element c o -> ?@o) -> @((a, $(Path ==> $Elem c)))
@HostList: fn @a emit c, !(Path ==> Answer c), (fn Path, Element c o -> @o)
    -> @((a, $(Path ==> $Wired c)))
```

- **`type Elem r = any o: Element r o` is unchanged:** an element with its output
  forgotten, its `$` fields holding values. An entry `$Elem c` is the place's element
  over time, the dynamic the supplier's `e` already carries (the runtimes' `Live::level`).
- **A box's children take the document's shape:** the window library's boxes declare
  `children: $(Path ==> $Elem)`, and a box writes its document there unchanged
  (`Ui.Column(spacing=g, align=a, children=$elems)`). Sampled, the field is a plain map
  from each child's path to that child's dynamic; the field moves when a child comes or
  goes, not when one changes. So an `Elem` holds dynamics wherever it holds children.
- **`Claimed` goes (Claude's, doubt CL):** it kept the library's elements by testing each
  one's tag, and a plain function cannot sample a `$Elem` to see its tag (the tag is
  identity, Semantics ch. 8, "A plain field is identity", but the type does not say so).
  A box's children are its whole document; layout lays an element that is not the
  library's as nothing, as `Lay`'s `otherwise: unlaid` does today.
- **What a World reads (Claude's, doubt WD):** an element holding dynamics is not `^Pod`
  (Type System ch. 2: "A function, dynamic or event refuses the derive"), and the root
  document goes to a World. The World reads a place deep: its level with every nested
  entry sampled, printed as today, reported changed in the tick any nested entry moves.
  That keeps every Test World transcript's printed elements as they are.

### 2. What the document is sampled as, and when each part fires

- **The outer is structural** (ch. 7, round 3 ST): sampled at n, the map of the child's
  places as the tick opened, paths relative to the child's root, each entry that place's
  dynamic; its value after n fixed as the last act of n's construction; its update at
  n+1, owing the tick, only when a place came or went. An entry is the same dynamic for
  as long as its place lives, so a value change cannot move the outer.
- **Each entry is published per place (Claude's, doubt EP):** sampled at n it is the
  place's element after n-1; its value after n is fixed in n's construction; its update
  fires at n+1 when that element differs (elements have `^Eq`), owing the tick. This is
  today's whole-document rule applied to one place, so a value change crosses a host
  level in exactly the one tick it does today (HO: "hosting means inserting delay"), and a
  box's element, whose children are its own host's structural document, is lifted from a
  structural dynamic and so structural by ST anyway. *Refused:* the entry as the place's
  live level, prompt for a plain one: a value change would cross a host in no tick while a
  structural change still costs one, which breaks Jimmy's "every host level adds exactly
  one tick".
- **Chapter 9's sentence becomes** "sampled at n it is the map of the child's places as
  the tick opened, paths relative to the child's root, each the place's element as a
  dynamic, `$Elem c`, whose sample at n is the element with its `$` fields' values after
  n-1. Both levels are structural (chapter 7): the map's value after n is fixed as the
  last act of n's construction and `!Updated $doc` fires at n+1 when a place came or
  went; each entry's value after n is fixed there too and its update fires at n+1 when
  that place's element changed."
- **A World paints only settled states** (PS): unchanged; the owed ticks are the ones
  above.

### 3. How the window lays out a map of dynamics

- **A plain function cannot sample a dynamic,** and `Lay` is plain. The window reads a
  child only inside a place that holds its dynamic: a `for p, $d <- $children` row, whose
  entry `$d` is `$$Elem` (the row's entry is the source's value at its key, Semantics
  ch. 4), followed with `$e <- @Flatten $d`, laid out as `$(LayOne ...)`.
- **What a gathered `for` gives today:** each row ends on `$T`, the `for` gives `$(K ==>
  T)`, structural (ST): a row's result that moves at n is in the gathered map published
  at n+1. So a `for` per level of boxes adds a tick per level, as Jimmy's story says every
  gathered `for` does. Under the flat document this was route A, which he refused; under
  `$(Path ==> $Elem)` it is the only way the window hears one child change without
  flattening the tree back into one value (part 4), so the window's layout goes per child,
  one `for` per box level, through one recursive widget (Semantics ch. 4: "widget
  functions recurse as deep as their data"; prismc lowers it, `sem-ch04-widget-recursion`,
  a row's entry being a part of the source for the recursion checker).
- **Route A's refusal is withdrawn** (the coordinator's, made under the flat type, against
  Jimmy's "change everything for this"): the window is one child per `for` row.
- **What a row returns:** plain data with `^Eq` (a child's size, a leaf's lines and ink,
  its kind and chain), not `Laid`, whose closure has no `^Eq`, so an equal take keeps the
  gathered map and nothing above it moves. The placing (draw commands and hits for what
  shows) is one plain pass over the gathered data at the window, as `PlaceAt` is today.
- **Text extents per leaf (Claude's, doubt MW):** each leaf row asks for its own texts'
  extents (`rebuild $text: Gui.Metrics(text=$text)`, since `text` is an identity field),
  answered synchronously in the birth tick; the window's word set and its Metrics `for`
  go. The other way, a word set gathered per level into one Metrics `for`, adds the
  gathered ticks twice (once for the word, once for its extent) and recomputes every leaf
  when the extents map moves.
- **The hold cache (job 2) is not built (Claude's, by design):** each `for` row already
  keeps its child's last layout while its entry does not move, and recomputes only when
  it does, so the window no longer lays out unchanged children and a hold would cache
  nothing. That cannot be measured until the type change is built.

One select under the new type, derived from the books (not run). The press is tick n; the
routing ticks are today's. L1 is the window's row for the column, L2 the column's rows,
L3 a row's leaves.

| Tick | What fires | What is recomputed |
|---|---|---|
| n to n+2 | the press, the window's and the column's deliveries | as today |
| n+3 | row k's `!sel`, the block's `!op`, `$table` written | 1,000 buttons' labels (each reads `$table.?selected`); row k's and the old row's button levels move; their entries are taken in row k's host |
| n+4 | the two entries published | in the window: the two L3 rows' `@Flatten` follow; each `rebuild $text` builds a new Metrics place (its first value synchronous); the rebuilds' results are taken |
| n+5 | the rebuilds' results published | the two L3 rows' leaf data; the L3 gathered maps taken |
| n+6 | L3 gathered published | the two L2 rows re-stack their three children; the L2 gathered map taken |
| n+7 | L2 gathered published | the L1 row re-stacks the column's 1,001 sizes; the L1 gathered map taken |
| n+8 | L1 gathered published | the placing pass over the window's data (the rows that show), then the paint |

- **Nine ticks where today there are eight,** with no second layout: the new word's extent
  arrives with the word. Per tick the work is the rows that moved plus one re-stack per
  level of the moved path (the column's 1,001 sizes once); painting stays a whole frame.
  The 1,000 button labels recomputed at n+3 remain: every row reads the selection
  (doubt SE).
- **No outer document fires** in the select: the column's and row k's elements are the
  same values (their children fields hold the same dynamics).

### 4. Reading a map of dynamics as one value, and `@Flatten`

- **The only way is a `for` with a flatten per row:** `for p, $d <- $doc: @Flatten $d`
  gives `$(Path ==> Elem)`, the flat document, one gathered tick after the entries and
  structural, firing whenever any entry moves. Each row's `@Flatten` adopts its outer's
  new inner at the end of a tick in which the outer moved (ch. 7; ruling VL) and owes that
  tick (ruling TK), so a structural change costs one tick more per level read this way.
- **The window needs it nowhere as one value;** it needs `@Flatten` in each row of its
  `for`s, to read the row's `$$Elem` entry as `$Elem`. That flatten moves only when the
  outer moved (a child came or went), owing a tick then.
- **A program that wants the old flat value** (sem-ch09-host-updated's `Texts`) writes the
  `for` above and pays its tick.

### What the change touches, and where it stands

- **Built so far:** the failing conformance test `sem-ch09-host-doc-entries` (a value
  change of a place leaves the outer's count where it was; a new place moves it), and
  what the per-child window needs of the compiler: a widget recursing over a `for` row's
  entry (`sem-ch04-widget-recursion`) and a `for` over a field of a row's entry rebuilding
  when the block writes it (`sem-ch04-for-nested-entry`).
- **Still to build:** the prelude's three signatures; the checker (the hosting forms'
  typing and the supplier's `e`, a field of type `$(Path ==> $Elem)` inside a tag, the
  World's deep read); lowering (`HostDocGetter`, `DocumentBody`, `Row::document`: an entry
  per place holding the place's published level; the host's rows over the outer); both
  printers' templates; both runtimes (a per-place snapshot, the World's deep `to_pod`);
  the window library (per-child rows, per-leaf metrics, no `Claimed`, no word set); chess;
  the GUI benchmark program; every hosting test (`sem-ch09-*`, `sem-ch07-structural*`,
  `sem-ch04-row-source`, `ts-ch05-supplier-rank2`, the checker's hosting tests) and every
  transcript that prints a window's ticks (`examples/chess/tests.expected`,
  `examples/chess/window/tests.expected`) or counts `!Updated $doc`.

## Routes refused or dropped

- **Route B, the flat document updated in place:** dropped with the flat type.
- **Route C, a `Cached` prelude function:** refused by Jimmy: no new prelude piece and
  no hidden runtime cache; a cache would be a hold in the window library.
- **Route D, nested structural dynamics settled within a tick:** refused by Jimmy: one
  tick per level stands. Doubt LG (whether ST meant a nested document settles in its
  tick) is withdrawn: his story is one tick per level, and the second layout of a select
  comes from the Metrics `for` publishing a tick after the new word arrives.
- **Route A under the flat type,** per-child `for` rows for layout over one whole-tree
  document: refused by Jimmy. Doubt RA is withdrawn with it. Under `$(Path ==> $Elem)`
  per-child rows are the design above.

## Without a route: values that keep their sharing (no transcript changes)

- **Built, both runtimes:** `Snapshot::take_compared` keeps the snapshot that stood when
  the new one is equal to it, so an unchanged structure keeps its nodes across ticks;
  `==` on maps and arrays answers at once for a node both sides share at the same place,
  once the node knows its values equal themselves (a shared NaN stays unequal: `Float`
  `==` is IEEE equality, Type System ch. 2). Safe by Semantics ch. 11: "sharing is
  invisible". Measured, they move nothing by themselves (`claude/benchmarks-preliminary.md`).
- **Built, both printers:** a `for`'s builder (`ForBuild`) keeps the source it last built
  from and does nothing when the new source is that very map (`Map::same`, one root node,
  in both runtimes); a host's own rows do the same over the document. `tests/cost`'s
  builder pair pins it: forty writes of a 20,000-key map with the value it has cost under
  twice one write.

## The books' sentences that change

Listed in Claude's report to the coordinator, who edits the books: Semantics ch. 7 (ST's
list of structural dynamics), ch. 8 ("The element tree is a dynamic"), ch. 9 (the forms,
"The result"), Appendix B's hosting entry; Type System ch. 2 (`Elem r`, the primitives);
RULINGS (the ruling itself, LG and RA withdrawn).

## Doubts

- **CL** (Claude's) `Claimed` goes and a box's children are its whole document, a
  non-library element laid as nothing; or a typed way to read a `$Elem`'s tag without a
  sample (a new piece, so not taken).
- **WD** (Claude's) A World reads an element holding dynamics deep (each nested entry
  sampled, a change reported when any moves), so printed transcripts keep their shape;
  or elements with children never go to a World (chess's board test emits a `Ui.Grid` to
  the Test World directly).
- **EP** (Claude's) Each entry published per place, structural, one tick per host level
  for a value change as today; or the entry as the place's live level, prompt.
- **MW** (Claude's) Text extents per leaf through `rebuild $text: Gui.Metrics(...)`, the
  word set gone; or the word set gathered per level into one Metrics `for`.
- **EQ** (Claude's) An element whose tag holds children holds dynamics, and a dynamic has
  no `^Eq`, so `Elem` of the window's row loses `^Eq` (Type System ch. 2's derive refuses a
  dynamic); the per-place publication compares deep-sampled levels in the runtimes, where a
  spurious firing is allowed (round 3 DH) and nothing in Prism sees the comparison. Or
  dynamics compared by identity in Prism, which would make identity observable (refused).
- **PB** Painting one frame is 143 M instructions in C++ and 155 M in Rust whatever
  changed, a fifth to a quarter of create1k; a small change stays above that floor.
- **SE** (Claude's) The table's rows each read `$table.?selected`, so a select recomputes
  1,000 small labels: left in js-framework-benchmark's natural spelling.
- **RF** (Claude's) Routing visits every child of a box through its own filter (1,001 at
  the column per press): left, linear and small.
