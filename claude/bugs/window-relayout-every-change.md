# What a small change to the window still costs

*Written by Claude on the claude-self-hosting branch, 6 November 2026; subject to change.
A performance report on what the window library and the compiled hosts still do per
change, after the host document became `$(Path ==> $Elem c)`
(claude/incremental-host-document.md) and the canvas Worlds began painting only the damage
of a new scene, and the table's rows read their values through `@PerKey`: the programs are right, and a small change no longer lays out, documents
or paints the whole table, but some of it is still proportional to the table.*

## The scenario

`bench/gui/prism/check.sh` on Jimmy's Mac, millions of instructions, one run each, with the
canvas Worlds painting only the damage of a new scene and the table's rows over `@PerKey`
of its rows (`bench/gui/prism/app.prism`):

| target | op | ticks | build | patch | paint | total | ÷ create1k |
|---|---|---|---|---|---|---|---|
| C++ | create1k | 8 | 422.2 | 30.1 | 22.6 | 631.6 | 1.000 |
| C++ | update10th | 9 | 13.7 | 8.4 | 2.7 | 37.8 | 0.060 |
| C++ | select | 9 | 6.3 | 7.1 | 1.4 | 20.6 | 0.033 |
| C++ | swap | 9 | 6.5 | 7.1 | 1.3 | 20.9 | 0.033 |
| C++ | remove | 8 | 90.4 | 11.4 | 18.4 | 130.7 | 0.207 |
| Rust | create1k | 8 | 248.6 | 28.4 | 38.4 | 532.9 | 1.000 |
| Rust | update10th | 9 | 11.4 | 9.3 | 14.2 | 49.7 | 0.093 |
| Rust | select | 9 | 4.6 | 8.0 | 12.7 | 31.1 | 0.058 |
| Rust | swap | 9 | 4.8 | 8.0 | 12.8 | 31.7 | 0.060 |
| Rust | remove | 8 | 29.3 | 10.7 | 32.7 | 81.3 | 0.152 |

No row is laid out but the ones that moved, no document of the whole table is built, a
write of `$table` that leaves a row's value alone does not mark that row for the patch
pass, and the paint is the damaged boxes.

## What remains

- **Presenting a Rust frame** is about 11 M instructions whatever changed: softbuffer
  hands a new zeroed buffer each frame on macOS, the whole frame is copied in and becomes
  a new `CGImage` (worlds/canvas/rust/README.md, "Painting"). It is most of a Rust select's
  paint; C++ sets its layer's image from its own buffer.
- **A structural change** (remove, create, clear) gathers the outer of the column's
  document from every place's entry (3,003 entries), once, and the window re-stacks the
  column's children: per change, a pass over every row's entry and every row's plan, not
  over its layout. In C++ it is three times Rust's (remove's build 90 M against 29 M):
  `HostDoc::insert` puts the entries in one at a time (a profile of swap, while it still
  gathered the outer, showed `Map::put` of the outer's two maps on top). Both runtimes'
  maps change their own nodes in place and copy only a node another map shares; with
  that, on the Linux machine, remove's build is 7.5 ms of CPU in C++ against 8.8 before
  (one run each), so the path copies were a part of the gap, not its body: what remains
  is to profile.
- **One re-stack per level of the moved path:** the column's result is computed again from
  its 1,001 children's plans (each reused), and the placing pass visits each plan to cull
  what does not show: linear, a few million instructions.
- **Every row still hears a write of `$table`** through its selected flag, `$mine <-
  @Distinct ($table.?selected == (? slot))`: 1,000 small comparisons a write, and two rows'
  labels move on a select. A row's value reaches it through `@PerKey` and is marked only
  when it changed.
- **A structural change walks every key of the table's `for`** (remove, create, clear): a
  key that came or went writes `@PerKey`'s outer, and the `for`'s builder, walking the keys
  on that write, compares each kept key's dynamic with the one its row was built from and
  tells only a row whose dynamic was replaced (ruling FV; under `@PerKey` a kept key's
  dynamic is the same one for as long as the key stays, so none is). The kept rows are not
  marked for the patch pass; what remains is the walk itself, one comparison per key, for
  every map of dynamics alike, with nothing special for `@PerKey` (Jimmy refused that).
- **The rest of a select's 7.1 M patch pass:** the window's placing over the column's
  1,001 plans, the scene made a `Pod`, the two moved rows' layout.
- **The routing of a press** through the column's 1,001 filters (doubt RF).
- **MT** A new word's extent reaches its leaf's layout a tick after the word (each leaf
  measures its own texts, in a gathered `for`), so a layout with a new word runs twice and
  an input shows its new text at the old width (`h=8`) for one owed tick
  (BasicWindow.Tests' `a_key_types_its_character` pins it).

## Doubts

- **TX** `Text: fn Char -> Text` is in the prelude and not in Appendix B, which has no way
  to make a `Char` a text; `@TextInput` needs one.

## Tests and measurement

- **`BasicWindow.Tests`** (`examples/basic-chess/basic_window/tests.prism`): culling, focus, typing.
- **`bench/gui/prism/check.sh`**: no text drawn outside the window and the screen showing
  the table (both pass); then the table of each step's instructions by phase, with the
  ratios, reported.
- **`tests/cost/run.sh`**: a `let` read by ten places costs one computation; a builder over
  the source it last built from costs nothing; forty one-key edits of a map under
  `@PerKey` cost one row's work each (all pass).
- **The bench:** `bench/gui/run.sh prism-cpp prism-rust`.
