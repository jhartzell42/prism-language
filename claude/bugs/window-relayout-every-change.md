# A small change to the window still costs most of a create1k

*Written by Claude on the claude-self-hosting branch, 5 November 2026; subject to change.
A performance bug in what the window library and the compiled hosts do per change: the
programs are right, and a change to one row costs work proportional to the table.*

## The scenario

The GUI table (`bench/gui/`) on Jimmy's Mac, `RUNS=5 WARMUP=1`, on the merged tree with
both seeds rebuilt (`885c89d` merged, branch `claude-host-cheap`), medians in ms
(`claude/benchmarks-preliminary.md`, "Instructions by phase, and the cheap runtime
fixes"; GTK from "After the window fix"):

| op | Prism C++ | Prism Rust | GTK |
|---|---|---|---|
| create1k | 50 | 44 | 277 |
| update10th | 45 | 39 | 36 |
| select | 42 | 37 | 1.1 |
| swap | 36 | 29 | 61 |
| remove | 38 | 34 | 6.7 |

- **`bench/gui/prism/check.sh` fails its second check** on update10th, select, swap and
  remove under both targets: each retires 0.6 to 0.85 times the instructions of create1k
  (C++ select 523 M against 650 M), where the check asks for under a tenth.
- **What is fixed** (select 797 ms in C++ and 1,651 in Rust before): the window draws only
  what shows; its word set goes through no `Union`; `$kids <- @Distinct (Claimed $elems)`
  drops the hosts' unchanged refires, so the window lays out once per change of its
  document where it laid out six times; a multiply-read `let` is cached; a `for`'s builder
  touches only the keys that came or left; `==` on maps and arrays answers at once for a
  shared node, and an equal snapshot keeps its value (neither moves these numbers).

## The mechanism, measured

**By phase** (`check.sh`'s table; millions of instructions, one select):

| target | propagate | build | patch | paint | the rest | total | create1k |
|---|---|---|---|---|---|---|---|
| C++ | 12.5 | 192.2 | 164.9 | 143.3 | 10.1 | 523.0 | 650.4 |
| Rust | 5.4 | 104.9 | 184.5 | 154.9 | 7.0 | 456.7 | 589.6 |

- **Painting** one 900 by 800 frame costs 143 M (C++) and 155 M (Rust) whatever changed,
  0.22 and 0.26 of create1k: more than twice the bar on its own (doubt PB).
- **The build walks** (192 M, 105 M): the 1,000 row documents rebuilt because every row
  reads the selection; the host documents rebuilt whole in several ticks; every builder of
  the root diffing its whole key set in each of them.
- **The patch pass** (165 M, 185 M): two whole layouts, one when the row's label reaches
  the window's document and one a tick later when the new word `▶`'s extent arrives
  through the Metrics `for` (`$sizes` moves; swap, with no new word, lays out once and
  spends half), plus the word set: `TextsIn $kids` walks every label.

One C++ select, the emitted code instrumented by hand (each line one computation), as
measured before the phase split:

- **Two whole layouts,** each over all 1,000 rows' sizes, about 5 ms; only the leaves that
  show build draw commands.
- **One whole word set** per change of `$kids`.
- **Five whole documents:** the window's host rebuilds its child's document
  (`host_1_doc_now`) in three ticks and the column's (`host_2_doc_now`) in two, a thousand
  places each, because a host's document is computed live from every place's level when
  any place below it moved (prismc/README.md, gap DM).

## What is left needs a design

Each of the costs above but painting is proportional to the table because the structure it
reads is one value of the whole tree, and making it proportional to the change is a
design, not an inefficiency:

- **Per-child layout** would lay out each child where it has a place (a `for` over the
  children), but a `for`'s gathered `$(K ==> T)` is a structural dynamic whose update
  fires at n+1 (Semantics ch. 7, round 3 ST), so a box combining its children's sizes
  would lag them a tick per level and every transcript would change. Doubt **PC**; the
  coordinator ruled to leave it unbuilt.
- **Per-child word sets** have the same lag.
- **An incremental host document** (the snapshot updated per place from the child's
  writes, each place's level recomputed only when its own reads moved, gap DM's "Next")
  changes what a host's `$doc` is computed from.

Within the bar Jimmy set (the layout engine in Prism, no new semantics but `@Distinct`)
the table is at the order of GTK's update10th and swap and two orders above its select.

## Doubts

- **PC** Whether per-child layout is worth a tick of lag per box (left unbuilt).
- **TX** `Text: fn Char -> Text` is in the prelude and not in Appendix B, which has no way
  to make a `Char` a text; `@TextInput` needs one.
- **PB** Painting the whole frame is 0.22 (C++) and 0.26 (Rust) of create1k by itself,
  so the check's tenth cannot pass while the canvas Worlds paint every pixel per settled
  state: count the program's phases only, or paint less (damage, which Jimmy refused in
  the World), or a different bar?
- **MT** A new word's metrics reach the window a tick after the word, so an input shows
  its new text at the old width (`h=8`) for one tick (Window.Tests'
  a_key_types_its_character pins it).

## Tests and measurement

- **`Window.Tests`** (`examples/chess/window/tests.prism`): culling, focus, typing.
- **`bench/gui/prism/check.sh`**: no text drawn outside the window (passes); a small
  change under a tenth of create1k's instructions (fails, as above); then the table of
  each step's instructions by phase.
- **`tests/cost/run.sh`**: a `let` read by ten places costs one computation (passes).
- **The bench:** `bench/gui/run.sh prism-cpp prism-rust`.
