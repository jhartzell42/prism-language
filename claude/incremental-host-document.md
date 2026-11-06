# A small change to the window: the host document as a map of dynamics

*Written by Claude on the claude-self-hosting branch; subject to change. A design note:
what a small change to the window costs and why, Jimmy's rulings on it, and the host
document as `$(Path ==> $Elem c)`, which he ruled on 6 November 2026 and which is built as
described here. Jimmy's words are quoted and marked as his; every other call is Claude's,
with the alternative refused, and Jimmy overrules by editing. Book citations are to
`prism-notes/claude/prism/`: Semantics (01), Type System (02), Platform (08). Numbers are
`bench/gui/prism/check.sh`'s phase table on Jimmy's Mac (`claude/benchmarks-preliminary.md`,
"The host document as a map of dynamics").*

## The question

- **The GUI table** (`bench/gui/`, js-framework-benchmark's operations over 1,000 rows,
  each an id label, a select button and a remove button) ran a small change in 0.6 to 0.85
  of the instructions of Create 1,000 rows (C++ select 494 M against 636 M), because each
  host document was one value of the whole tree, rebuilt when any place moved, and the
  window laid out the whole tree as one function of it, twice per change, with a word set
  walked over every label (`claude/bugs/window-relayout-every-change.md`).
- **With the document as `$(Path ==> $Elem c)`** a select costs 166 M in C++ and 177 M in
  Rust, of which painting the frame is 141 M and 155 M (doubt PB): 25 M and 22 M besides,
  where it was 352 M and 281 M.

## Jimmy's constraints

- **From his ruling on this bug** (as relayed by the coordinator): the layout engine
  "STAYS IN PRISM"; "bare minimum bone simple"; no general retained-mode layout system,
  no diffing World protocol, no damage regions in the World; the one new semantic piece
  allowed before this ruling is `@Distinct` (built: `$y <- @Distinct $x`, `^Eq`).
- **From his rulings of 5 and 6 November 2026** (as relayed by the coordinator):
  - no `Cached` prelude function and no hidden runtime cache;
  - no bar of a tenth of create1k ("never his"): the check reports the numbers;
  - one tick per level stands: every host level and every gathered `for` adds exactly one
    tick; route D (nested documents settled within a tick) is refused;
  - the host document is `$(Path ==> $Elem c)`: "No, change everything for this. The
    other way is quite stupid." The flat `$(Path ==> Elem c)` is gone everywhere.
- **From his earlier rulings** (Semantics RULINGS): on hosts, "hosting means inserting
  delay" (HO); on structural dynamics, "Inconsistent state shouldn't be visible. Either
  the entire thing fires in the next tick (reasonable actually) or it doesn't" (ST); an
  owed tick nobody reads is not skipped (TK); a World presents only settled states (PS).

## The document, as built

### Its type, and what an element holds

```prism
@HostDocument: fn @a emit c, (fn Path, Element c o -> @o) -> @((a, $(Path ==> $Elem c)))
@HostSome: fn @a emit c, (fn Path, Element c o -> ?@o) -> @((a, $(Path ==> $Elem c)))
@HostList: fn @a emit c, !(Path ==> Answer c), (fn Path, Element c o -> @o)
    -> @((a, $(Path ==> $Wired c)))
```

- **`type Elem r = any o: Element r o` is unchanged:** an element with its output
  forgotten, its `$` fields holding values. An entry `$Elem c` is the place's element over
  time.
- **A container's children take the document's shape:** the window library's boxes declare
  `children: $(Path ==> $Elem)` and write their document there unchanged
  (`Ui.Column(spacing=g, align=a, children=$elems)`). Sampled, the field is a map from each
  child's path to that child's dynamic: it moves when a child comes or goes, not when one
  changes. So an `Elem` holds dynamics wherever it holds children.
- **Neither compares nor prints in Prism** (doubt EQ): Type System ch. 3 says a value that
  holds a dynamic has none of `^Eq`, `^Ord`, `^Show`, and the checker now holds to it: a
  `derive` of one on a type a field of which holds a dynamic or an event is refused at its
  line (`derive-dynamic`), and an `Elem r` whose row has a tag holding dynamics has no
  built-in `^Eq` or `^Show` (`no-impl`).

### When each part fires

- **The outer is structural** (ch. 7, round 3 ST): sampled at n, the map of the child's
  places as the tick opened, each entry that place's dynamic; its value after n is fixed in
  n's construction step; its update fires at n+1, owing the tick, only when a place came or
  went. An entry is the same dynamic for as long as its place lives, so a value change
  never moves the outer (`sem-ch09-host-doc-entries`).
- **Each entry is published per place** (doubt EP): sampled at n it is the place's element
  after n-1; its update fires at n+1 when that element differs, owing the tick. A value
  change crosses a host level in one tick, as the flat document's did (HO: "hosting means
  inserting delay").
- **A program that wants the old flat value** writes the `for` that flattens each entry
  and pays its tick: `$flat <- for p, $$d <- $doc: $e <- @Flatten $$d; @ $e` gives
  `$(Path ==> Elem)`, structural, a tick after the entries (`sem-ch09-host-updated`,
  `sem-ch09-host-doc-in-tick`).
- **A World paints only settled states** (PS), unchanged.

### The compiler and the runtimes

- **Each hosted place is an `Entry`** (`prism_rtl::Entry`, both runtimes), a field of the
  node the place is in: its own snapshot, taken at the node's build in its scope's turn
  and again in the patch pass when a write behind the place dirtied the node, as a World
  place is sent. A take compares the level (the derived `PartialEq` of the program's `Elem`,
  a dynamic equal only to itself); one that moved dirties the host rows reading the place
  as it stands (a forwarded element re-emits in the tick) and queues the publication of
  the entry's dynamic for the next tick.
- **The outer is the owner's snapshot of a `HostDoc`**: each place's dynamic (what the
  program's `$doc` reads) and each place's entry (what the host's own rows read). It is
  gathered from the entries, and only in a tick in which a builder or generation of the
  child changed its rows: they tell the host (`host_k_moved`, `document_moved`), and
  nothing a place reads marks it. Two compare equal when they hold the same entries at the
  same paths.
- **A forwarded element is the same place** for an enclosing host: a host row's own
  document is the child's entry (`HostRow::entries`), so nested hosts share it.
- **A level that holds dynamics goes to a World deep** (doubt WD): `Elem` implements
  `prism_rtl::Level`, each dynamic it holds sampled at the read point, and a World place
  whose tag holds dynamics keeps a `prism_rtl::Watch` on them, sent again when any nested
  entry moves. Chess's board test, which sends a `Ui.Grid` to the Test World, prints as it
  did.
- **`@Flatten` of a `for` row's entry** is lowered: the inner is set at the row's
  construction and followed (`prism_rtl::Follow`), and an entry that becomes another
  dynamic is adopted the tick after, owed (Semantics ch. 7; rulings VL, TK).
- prismc/README.md, picks HO and ST and gaps DM, ID and SU, and `claude/backend.md`,
  "Hosting", say how each is lowered.

### How the window lays out a map of dynamics

- **One row per element:** the window's `for p, $$d <- $elems` lays each top element out,
  and `@Laid $e, $room, chain` (widgets.prism) lays an element out in the room its parent
  gives it, with a `for` row per child over `KidsOf $e` (the container's document), each
  flattening its entry and recursing (Semantics ch. 4: "widget functions recurse as deep
  as their data"; the shrink check reads `@Flatten` as no larger than its argument).
- **What a row ends on is plain data with `^Eq`:** the element's `Res` (layout.prism): its
  kind, the room it was given, its chain of paths, its children's results, a leaf's
  measured texts, and its `Plan`, the layout with its placing as data. A container reuses
  each child's plan where the room it lays the child in is the room the child's row was
  given (`PlanIn`; `KidRoom` is that room), and lays the child again from its `Res` where
  it is not (flex, stretch, a grid's cells). An equal result keeps the gathered map, and
  nothing above it moves. The room is a hold of the row's own (`@Distinct`), so the result
  is a cached `let`, computed once per write behind it.
- **One plain placing pass at the window** (`PlaceAt`, over the root's plan): draw commands
  and hits for what shows, with the focus, so a focus change lays out nothing.
- **Each leaf measures its own texts** (doubt MW): a `for` over its texts in its row, a
  `Gui.Metrics` each at `.../1/[text]/0`; the window's word set, its `Metrics` `for` and
  `Claimed` (doubt CL) are gone.
- **No hold cache is built:** each `for` row keeps its child's last result while its entry
  does not move, which is what a cache would have kept.
- **Every element nests about a dozen containers deep at most** below the window: the
  lowering inlines a recursive widget sixteen levels (gap ID); a deeper tree faults at its
  build, loudly.

### One select, tick by tick

Derived from the emitted code; the tick count, nine, is the check's. The press is tick n;
three hosts stand between the window and the button (the window's, the column's, the
row's), each a computed answer a tick late.

| Tick | What fires | What is recomputed |
|---|---|---|
| n to n+2 | the press; the window's, then the column's delivery | each box's `?Boxed` filters (the column's 1,001) |
| n+3 | row k's `!sel`, the block's `!op`, `$table` written | 1,000 buttons' labels (each reads `$table.?selected`), their entries taken; row k's and the old row's moved |
| n+4 | the two entries published | the two buttons' `@Laid` rows: their `@Flatten`s move, each new word's `Gui.Metrics` row built, their results taken; the two rows' gathered results taken |
| n+5 | the leaves' measured texts and the rows' gathered results published | the two buttons' results again (now with the new word's extent); the two rows' results; the column's gathered results taken |
| n+6 | the column's gathered results (first wave) and the rows' (second) | the column's result: one re-stack of 1,001 plans; the window's tops taken |
| n+7 | the window's tops (first wave), the column's (second) | the window's placing; the column again |
| n+8 | the window's tops (second wave) | the window's placing, then the paint |

- **Nine ticks where there were eight;** the frame at n+7 is not painted (the tick is
  owed, PS). The new word's extent reaches its leaf a tick after the word (doubt MT), so a
  layout with a new word runs twice; one without (swap, a removed word) runs once.
- **Per tick the work is the rows that moved plus one re-stack per level of the moved
  path** (the column's 1,001 plans, reused), and the placing pass, which visits each
  row's plan to cull what does not show. No document of the whole table is built and no
  row is laid out but the moved ones.
- **No outer document fires** in a select: the column's and the rows' elements hold the
  same dynamics.

### What a small change still costs

- **Painting one frame** (doubt PB): 141 M in C++, 155 M in Rust, whatever changed.
- **The 1,000 button labels** recomputed at n+3, each reading the selection (doubt SE).
- **A structural change** (remove, create, clear) gathers the outer of its host's document
  from every place's entry, once, and the window re-stacks the column's children.
- **A structural change costs C++ three times Rust's** (remove's build 90 M against 29 M):
  the outer's maps are built an insert at a time, and C++'s persistent map copies a path
  per insert (claude/bugs/window-relayout-every-change.md).

## Routes refused or dropped

- **Route B, the flat document updated in place:** dropped with the flat type.
- **Route C, a `Cached` prelude function:** refused by Jimmy: no new prelude piece and
  no hidden runtime cache.
- **Route D, nested structural dynamics settled within a tick:** refused by Jimmy: one
  tick per level stands.
- **Route A under the flat type,** per-child `for` rows over one whole-tree document:
  refused by Jimmy; under `$(Path ==> $Elem)` per-child rows are what is built.

## Values that keep their sharing

- **Both runtimes:** `Snapshot::take_compared` keeps the snapshot that stood when the new
  one is equal to it; `==` on maps and arrays answers at once for a node both sides share
  at the same place, once the node knows its values equal themselves (a shared NaN stays
  unequal). Safe by Semantics ch. 11: "sharing is invisible".
- **Both printers:** a `for`'s builder does nothing when its source is the very map it last
  built from (`Map::same`); a host's rows do the same over the places.
- **Both runtimes:** a map collected from entries in ascending key order (a `for`'s
  gathered results, by the rows' keys) is built balanced at once.
- **Both printers:** a builder in a hosted scope tells its host that places came or went
  only when its rows' keys changed (`same_keys`), so a swap or an update of values gathers
  no outer.

## The books' sentences that change

Listed in Claude's report to the coordinator, who edits the books: Semantics ch. 2 (the
type-syntax example), ch. 7 (ST's list of structural dynamics), ch. 8 ("The element tree
is a dynamic"), ch. 9 (the forms, "The result"), RULINGS (the ruling itself); Type System
ch. 1 (`@Grid`, doubt GR), ch. 2 (`Elem r`, the primitives), RULINGS (WL); the Pitch's ch.
4 and ch. 5.

## Doubts

- **CL** (Claude's) `Claimed` goes and a box's children are its whole document, a
  non-library element laid as nothing; or a typed way to read a `$Elem`'s tag without a
  sample (a new piece, not taken).
- **WD** (Claude's) A World reads an element holding dynamics deep (each nested entry
  sampled, sent again when one moves), so transcripts keep their shape; or elements with
  children never go to a World.
- **EP** (Claude's) Each entry published per place, structural, one tick per host level
  for a value change as before; or the entry as the place's live level, prompt (a value
  change would cross a host in no tick while a structural change still costs one).
- **MW** (Claude's) Each leaf measures its own texts in a `for` of its row (a label's words
  and the space), the word set gone. The note's first spelling, `rebuild $text:
  Gui.Metrics(...)`, measures a label whole (so it cannot wrap by words) and its
  `@Flatten` adds two ticks where the `for` adds one; or a word set gathered per level
  into one Metrics `for` (the gathered ticks twice).
- **EQ** (Claude's) The runtime compares levels with a dynamic equal only to itself (the
  same read and update port), invisible to Prism, which compares no dynamic. The note's
  first spelling compared deep-sampled levels: a take would then sample the dynamics an
  old level held, whose places may have left, and walk a container's whole subtree; the
  same-dynamic comparison never samples and costs nothing, and differs from the deep one
  only where a place's dynamic is replaced by another with equal samples, where it fires
  (allowed, round 3 DH). In Prism the checker refuses `^Eq` and `^Show` on what holds a
  dynamic (Type System ch. 3).
- **ET** (Claude's) An `Elem r` with an open row is judged by the tags it names: the
  checker refuses its `^Eq` only when one of those holds dynamics; or refused whenever a
  tail could stand for such a tag.
- **PB** Painting one frame is 141 M instructions in C++ and 155 M in Rust whatever
  changed: the floor under every small change while the canvas Worlds paint every pixel
  per settled state (damage regions are refused).
- **SE** (Claude's) The table's rows each read `$table.?selected`, so a select recomputes
  1,000 small labels: left in js-framework-benchmark's natural spelling.
- **RF** (Claude's) Routing visits every child of a box through its own filter (1,001 at
  the column per press): left, linear and small.
