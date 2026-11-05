# A small change to the window still costs more than building it

*Written by Claude on the claude-self-hosting branch, 5 November 2026; subject to change.
A performance bug in the window library (`examples/chess/window/`) and in what the
compiled hosts do each tick: the programs are right, and a change to one row still costs
about as much as creating the table.*

## The scenario

The GUI table (`bench/gui/`) on Jimmy's Mac, `RUNS=5 WARMUP=1`, medians in ms
(`claude/benchmarks-preliminary.md`, "After the window fix"):

| op | Prism C++ | Prism Rust | GTK |
|---|---|---|---|
| create1k | 53 | 53 | 277 |
| update10th | 78 | 91 | 36 |
| select | 76 | 88 | 1.1 |
| swap | 70 | 79 | 61 |
| remove | 63 | 73 | 6.7 |

- **Selecting one row costs more than creating a thousand.** `bench/gui/prism/check.sh`
  fails on update10th, select, swap and remove under both targets: each retires more
  instructions than its setup's create1k (select 0.97 G against 0.69 G in C++), where the
  check asks for under a tenth.
- **It was ten to twenty times worse** (select 797 ms in C++, 1,651 in Rust) before the
  window drew only what shows and stopped joining word sets with `Union`; that part is
  fixed, and pinned by `Window.Tests` and the check's first half.

## The mechanism

One select is about eight ticks: the press, then a tick per host layer (the window
answers the column, the column the row, the row the button; Semantics ch. 9) and the
owed ticks of the structural dynamics that moved (ch. 7, round 3 ST). Measured on the
C++ select (the emitted code instrumented by hand):

- **The window lays the whole tree out in six of those ticks, about 5 ms each, though its
  document changes in one.** In every other tick `$kids` is equal to the tick before;
  its update event fires anyway, as R3 allows ("may fire in a tick where it was
  recomputed to the same value"), and `$placed` and the scene follow it.
- **`$placed` is recomputed at every read.** It is a `let`, and the lowering caches only a
  `for`'s source, so the scene, the hit test and (when something has focus) the focus
  ring each lay the window out again. The window now reads it once per tick where it can
  (the focus reuses the hit; the ring is laid out only when focused).
- **About half of the samples are the hosts' own work:** each tick a host rebuilds its
  child's document whole (`host_N_doc_now`) and its rows' map key by key
  (`host_N_rows_build`), and the Metrics `for` recomputes `TextsIn` over the tree, all
  proportional to the table.
- **Routing a click is a filter per child:** `?Boxed` gives every child of a box a filter
  of the box's one `!in` event, so a click runs a thousand filters at the column. Cheap,
  but proportional to the table.

## What would fix the rest, and why this branch stops here

Jimmy's ruling stands: "bare minimum bone simple" refactoring, the layout engine stays
in Prism, no new semantics except one possible combinator that drops spurious refires of
a dynamic holding an `^Eq` value.

- **That combinator is what the six spurious layouts need, and it cannot be written in
  Prism.** The books give dynamics no deduping: "nothing dedupes by default" (Semantics
  ch. 7, "A dynamic is a sample and an update event"), and Appendix B rules it out of the
  prelude (D-DT, round 47: "No `$Distinct`: a distinct-until-changed keeps state, so it
  would be a widget binding, `$y <- @Distinct $x` with `^Eq`, and the prelude does not
  have one"). As a widget in Prism it needs a hold whose first value is `$x`'s at the
  build, and a hold's initial value is plain (`$kept <- $x then ...` is refused,
  mark-stack and two-layers). So it is a builtin, which means lowering in both printers,
  under `prismc/`, which another branch owns. Doubt **DQ** below.
- **Per-child dynamics** (a `for` over the children, each child's layout and draw list
  its own dynamic) would add a tick: a `for`'s gathered `$(K ==> T)` is a structural
  dynamic whose update event fires at n+1 (ch. 7, "Structural dynamics"), so the parent
  that combines its children's sizes would lag them, and chess's and the window's
  transcripts, which print every tick, would change. Doubt **PC**.
- **The hosts' per-tick rebuilds and the uncached `let`** are the lowering's and the
  runtime's (`runtime/*/host`), not the window's.

## Doubts

- **DQ** Whether to add `@Distinct: fn $a -> @$a where a: ^Eq` as a builtin (both
  runtimes and the lowering) against D-DT; it would remove five of the six layouts per
  select and the spurious Metrics recomputes.
- **PC** Whether per-child layout is worth a tick of lag per box, or whether the window
  keeps one plain function of the tree.
- **CQ** `@TextInput`'s `Apply` types `"{c}"` for a `Char c`, which interpolates its
  `Show` form: pressing `q` in an input inserts `'q'`, quotes included (seen in a
  Window.Tests draft; no test pins it).
- **AT** The C++ printer writes every `path.at(n)` in a node as `path->at(n)` when the node
  has a dynamic named `at` (`let $?at = ...` in `@Window` did not compile; it is `$?ring`).

## Tests and measurement

- **`Window.Tests`** (`examples/chess/window/tests.prism`): a column taller than its
  window draws only the leaves that start inside it, and a change below the edge leaves
  the scene alone; the focus moves by press and by Tab.
- **`bench/gui/prism/check.sh`**: no text drawn outside the window (passes); a small
  change under a tenth of create1k's instructions (fails, as above).
- **The bench:** `bench/gui/run.sh prism-cpp prism-rust`.
