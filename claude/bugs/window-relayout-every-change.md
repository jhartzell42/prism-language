# What a small change to the window still costs

*Written by Claude on the claude-self-hosting branch, 6 November 2026; subject to change.
A performance report on what the window library and the compiled hosts still do per
change, after the host document became `$(Path ==> $Elem c)`
(claude/incremental-host-document.md) and the canvas Worlds began painting only the damage
of a new scene: the programs are right, and a small change no longer lays out, documents
or paints the whole table, but some of it is still proportional to the table.*

## The scenario

`bench/gui/prism/check.sh` on Jimmy's Mac (`claude/benchmarks-preliminary.md`, "Painting
only the damage"), millions of instructions, one run each, with the canvas Worlds painting
only the damage of a new scene:

| target | op | ticks | build | patch | paint | total | ÷ create1k |
|---|---|---|---|---|---|---|---|
| C++ | create1k | 8 | 421.2 | 30.4 | 19.3 | 688.6 | 1.000 |
| C++ | update10th | 9 | 14.5 | 12.6 | 2.7 | 41.7 | 0.061 |
| C++ | select | 9 | 6.3 | 11.7 | 1.3 | 24.5 | 0.036 |
| C++ | swap | 9 | 7.3 | 11.7 | 1.3 | 25.4 | 0.037 |
| C++ | remove | 8 | 89.8 | 11.7 | 17.1 | 125.3 | 0.182 |
| Rust | create1k | 8 | 248.9 | 29.0 | 42.1 | 540.0 | 1.000 |
| Rust | update10th | 9 | 12.0 | 12.9 | 16.1 | 75.1 | 0.139 |
| Rust | select | 9 | 4.6 | 12.1 | 8.8 | 33.3 | 0.062 |
| Rust | swap | 9 | 5.6 | 11.9 | 8.7 | 36.0 | 0.067 |
| Rust | remove | 8 | 29.3 | 10.9 | 28.6 | 80.3 | 0.149 |

A select was 494 M (C++) and 436 M (Rust) with the flat document, and 166 M and 177 M with
the document as a map of dynamics while the Worlds painted the whole frame; no row is laid
out but the ones that moved, no document of the whole table is built, the patch pass (the
placing, and each moved place's entry) is 12 M, and the paint is the damaged boxes.

## What remains

- **Presenting a Rust frame** is about 11 M instructions whatever changed: softbuffer
  hands a new zeroed buffer each frame on macOS, the whole frame is copied in and becomes
  a new `CGImage` (worlds/canvas/rust/README.md, "Painting"). It is most of a Rust select's
  paint; C++ sets its layer's image from its own buffer.
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
- **Every row is listed for the patch pass on any write of the hold its `for` reads.**
  The emitted `Main::op` writes `$table` and marks every row of the `for` over
  `$table.rows` dirty, whether or not that row's entry moved; each marked row recomputes
  the elements it shows (the table's row: its label, its button, its row) and compares
  each with what it held. About half of the 11.6 M a select, swap or remove spends in the
  patch pass (a profile of repeated selects; the proportions are approximate, the profile
  also holds the setup). The rest: the window's placing over the column's 1,001 plans
  (about a quarter), the scene made a `Pod` (about a tenth), the moved rows' layout.
  The table's rows read the selection through `@Distinct` now, which stops the label's
  value moving and not the row being marked: a select's count did not move with it.
  `!Updated` over a row's entry firing in every row is the language's (pick UD,
  tests/conformance/sem-ch04-row-entry); skipping an unmoved row's patch would not touch
  it. Not done: Jimmy is deciding.
- **The routing of a press** (doubt SE was the labels, above), and the routing of a press through the column's 1,001
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
