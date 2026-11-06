# The GUI benchmark

*Written by Claude on the claude-self-hosting branch; subject to change.*

js-framework-benchmark's operations on one table, written four ways, timed from input to
the frame on the screen. Prism's two targets are each all one language: the Prism table
compiled to C++ runs under the C++ canvas World, compiled to Rust under the Rust one.

```
bench/gui/run.sh                    # build everything, time everything, write RESULTS.md
bench/gui/run.sh prism-cpp gtk      # only those
RUNS=20 WARMUP=3 bench/gui/run.sh
```

Run it on a quiet machine: power plugged in, nothing building, no other app busy.
`RESULTS.md` records the machine, every version, and the load average before and after.

## The table

A row is an id, a label of three random words (js-framework-benchmark's words, from one
generator all four share, so all four make the same labels), and a remove button. Above
the rows: Create 1,000 rows, Create 10,000 rows, Append 1,000 rows, Update every 10th row,
Clear, Swap rows. Clicking a row's label selects it, marked with a leading `▶` in all four
(the window library has no live background colour). The window is 900 by 800.

## The operations

Each run of an operation is its setup (untimed), the timed step, and a clear (untimed), as
js-framework-benchmark sets them up:

| Operation | Setup | Timed |
|---|---|---|
| create1k | | create 1,000 rows |
| replace1k | create 1,000 | create 1,000 again, every row new |
| update10th | create 1,000 | " !!!" after every tenth label, from the first |
| select | create 1,000 | click row 2's label |
| swap | create 1,000 | rows 2 and 999 trade places |
| remove | create 1,000 | click row 2's remove button |
| create10k | | create 10,000 rows |
| append1k | create 1,000 | append 1,000 rows |
| clear | create 1,000 | clear |

## What is timed

From the input to the frame that shows its result, handed to the window system.

- **Prism** (`prism/`): `bench.prism`, with examples/chess's window library, compiled by
  prismc. The driver (`driver.cpp` for C++, `driver.rs` for Rust) runs the root under the
  canvas World with `Hooks`; it finds a button by its `Draw.Text` in the scene and queues
  a press just inside it, as a click would. The clock stops at the next
  `Hooks::settled`: every tick the press caused has run, the layout and the scene are
  recomputed, and the frame is painted and presented (`[CATransaction flush]` in C++,
  softbuffer's `present` in Rust). After each timed step the driver checks the screen
  against the table the script should have made: the rows from the top down to the
  window's bottom edge, each with its id, its `▶` when selected and its ` !!!` when
  updated; it fails the run otherwise. C++ is built at -O3 with clang; Rust as a cargo
  release build (opt-level 3).
- **GTK** (`gtk/`): C with GTK 3 over Quartz. `gtk_button_clicked`, then the window's
  next frame-clock `after-paint`, then `gdk_display_sync`. Rows are boxes of a label and
  two buttons in a scrolled window.
- **Cuis** (`cuis/`): Cuis Smalltalk 7.9 with Morphic, on the Cog JIT VM the Cuis
  repository ships (`CuisVM.app`, an OpenSmalltalk Cog build with the VectorEnginePlugin
  Cuis 7 draws with). The step performs its button's action, then
  `WorldMorph>>doOneCycleNow` (layout, drawing, damage to the Display). Rows are linear
  layouts of a label and two buttons in a scroll pane.

The median of RUNS timed runs, after WARMUP untimed ones, with the fastest and the
slowest. Peak memory is each process's maximum resident set size from `/usr/bin/time -l`.

## The checks

`prism/check.sh` (macOS) runs create1k, update10th, select, swap, remove and clear once
each under both drivers with `driver OP 1 0 check`, which counts work rather than time:

- **What the scene draws:** after every step, no `Draw.Text` in the scenes starts more
  than 8 outside the window, so the window library draws what the window shows, not the
  whole table.
- **What a small change costs:** the instructions the whole process retires (every
  thread's, `proc_pid_rusage`) from the press to the settled frame. Update10th, select,
  swap and remove must each retire under a tenth of what their setup's Create 1,000 rows
  did.

A failed check fails the run and says what it saw. Then `check.sh` prints a table of each
timed step's instructions by phase, millions, against the same target's create1k: the
drivers set `prism_rtl`'s probe hook (Rust `world::probe`, C++ `probe` in `world.hpp`),
the runtime marks propagation, the build walks, the patch pass and the diff inside a
tick, the canvas World marks performing, compacting and painting, and each thread's
instructions between its marks (macOS's `thread_selfcounts`) go to the phase it was in;
`other` is the process's other threads.

## The files

- `run.sh`: builds, runs, and writes `RESULTS.md`.
- `prism/`: `app.prism`, `driver.cpp`, `driver.rs`, the `Makefile` that builds both
  drivers (the emitted `main` renamed, the driver beside it), and `check.sh`.
- `gtk/`: `table.c` and its `Makefile`.
- `cuis/`: `table.st` (one expression, run with `-s`), `run.sh`, and `setup.sh`, which
  clones Cuis-Smalltalk-Dev into `target/bench/deps`.
- `RESULTS.md`: the last run's numbers.

## Doubts

- **Cuis's VM.** Jimmy chose the Cog VM built from source. The OpenSmalltalk release
  (202606270913, arm64) has no VectorEnginePlugin, and without it Cuis 7 draws in
  Smalltalk: create1k took 9.8 s, nearly all in the first World cycle. The VM the Cuis
  repository ships is Cog from the same source tree, newer (VMMaker.oscog-eem.3811), with
  the plugin; the benchmark uses it. Building opensmalltalk-vm from source with the plugin
  is not done.
- **Cuis's create1k** is still about 5 s, almost all of it in the first World cycle after
  the rows are added (14 ms to make them); not profiled to the method.
- **Where the clock stops differs by toolkit:** a frame handed to the window server in all
  three, but through three APIs, none of which waits for the display's vertical refresh.
- **Swap in Cuis** puts the column's rows back in order (Morphic lays the whole column out
  again on any change); GTK moves the two children; Prism moves two rows' values between
  their places.
- **No scrolling in Prism:** the window library has no scroll pane, so the Prism window
  lays out every row and draws those that start inside the window; GTK and Cuis lay out
  every row too, in a scroll pane.
