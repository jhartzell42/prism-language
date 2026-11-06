# What a small change to the window still costs

*Written by Claude on the claude-self-hosting branch, 6 November 2026; subject to change.
A performance report on what the window library and the compiled hosts still do per
change, after the host document became `$(Path ==> $Elem c)`
(claude/incremental-host-document.md): the programs are right, and a small change no longer
lays out or documents the whole table, but some of it is still proportional to the
table.*

## The scenario

`bench/gui/prism/check.sh` on Jimmy's Mac (`claude/benchmarks-preliminary.md`, "The host
document as a map of dynamics"), millions of instructions, one run each:

| target | op | ticks | build | patch | paint | total | ÷ create1k |
|---|---|---|---|---|---|---|---|
| C++ | create1k | 8 | 420.9 | 30.4 | 141.7 | 747.1 | 1.000 |
| C++ | update10th | 9 | 14.5 | 12.6 | 141.2 | 181.8 | 0.243 |
| C++ | select | 9 | 6.3 | 11.7 | 141.3 | 165.9 | 0.222 |
| C++ | swap | 9 | 7.3 | 11.7 | 141.1 | 166.8 | 0.223 |
| C++ | remove | 8 | 90.0 | 11.6 | 141.1 | 250.7 | 0.336 |
| Rust | create1k | 8 | 246.8 | 28.2 | 155.3 | 646.6 | 1.000 |
| Rust | update10th | 9 | 11.7 | 12.8 | 155.1 | 193.5 | 0.299 |
| Rust | select | 9 | 4.5 | 11.9 | 155.0 | 176.9 | 0.274 |
| Rust | swap | 9 | 5.3 | 11.9 | 155.0 | 177.9 | 0.275 |
| Rust | remove | 8 | 29.0 | 10.9 | 154.9 | 202.9 | 0.314 |

A select was 494 M (C++) and 436 M (Rust) with the flat document; no row is laid out but
the ones that moved, no document of the whole table is built, and the patch pass (the
placing, and each moved place's entry) is 12 M.

## What remains

- **Painting one frame** (doubt PB): 141 M in C++, 155 M in Rust, whatever changed, the
  floor under every small change while the canvas Worlds paint every pixel per settled
  state (damage regions in the World are refused).
- **A structural change** (remove, create, clear) gathers the outer of the column's
  document from every place's entry (3,003 entries), once, and the window re-stacks the
  column's children: per change, a pass over every row's entry and every row's plan, not
  over its layout. In C++ it is three times Rust's (remove's build 90 M against 29 M):
  `HostDoc::insert` puts the entries in one at a time, and C++'s persistent map copies a
  path per insert where Rust's updates its own nodes in place (a profile of swap, while
  it still gathered the outer, showed `Map::put` of the outer's two maps on top).
- **One re-stack per level of the moved path:** the column's result is computed again from
  its 1,001 children's plans (each reused), and the placing pass visits each plan to cull
  what does not show: linear, a few million instructions.
- **The 1,000 button labels** recomputed when the selection moves, each reading
  `$table.?selected` (doubt SE), and the routing of a press through the column's 1,001
  filters (doubt RF).
- **MT** A new word's extent reaches its leaf's layout a tick after the word (each leaf
  measures its own texts, in a gathered `for`), so a layout with a new word runs twice and
  an input shows its new text at the old width (`h=8`) for one owed tick
  (Window.Tests' `a_key_types_its_character` pins it).

## Doubts

- **TX** `Text: fn Char -> Text` is in the prelude and not in Appendix B, which has no way
  to make a `Char` a text; `@TextInput` needs one.

## Tests and measurement

- **`Window.Tests`** (`examples/chess/window/tests.prism`): culling, focus, typing.
- **`bench/gui/prism/check.sh`**: no text drawn outside the window and the screen showing
  the table (both pass); then the table of each step's instructions by phase, with the
  ratios, reported.
- **`tests/cost/run.sh`**: a `let` read by ten places costs one computation; a builder over
  the source it last built from costs nothing (both pass).
- **The bench:** `bench/gui/run.sh prism-cpp prism-rust`.
