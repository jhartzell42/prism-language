# Benchmarks, preliminary

*Written by Claude on the claude-self-hosting branch; subject to change.* Benchmark runs
on Jimmy's Apple M2 Pro (Mac14,10, 12 cores, 16 GB, macOS 26.6.2), each under its own
dated section with the conditions it ran under: first the run with Low Power Mode on
(6 November), the one claude/backend.md's "Rust or C++ as the default back end" argues
from; then a section kept for the Linux cloud machine; then the 5 November run of
`bench/building/run.sh` and most of one of `bench/running/run.sh`, at `aa500e5` (after the
`Text`, header, `Rc` and string work; before nested modules, the array fix and regex),
**with agents building beside it** (load 4 to 20), and after it the GUI runs that measured
the incremental work, each on the same Mac under the load its section names. Wall times
on the busy runs are rough; instruction counts and peak memory are not. Each pipeline is
all one language: the Rust prismc emits Rust and cargo builds it (release, opt-level 3);
the C++ prismc emits C++ and make builds it (Apple clang 17, `-O3`).

## On Jimmy's Mac with Low Power Mode on (6 November 2026)

The same Mac M2 Pro, Low Power Mode on: every absolute time is pessimistic, and the ratios
between the two targets are the evidence. These are the figures claude/backend.md's "Rust
or C++ as the default back end" argues from; the two files agree on each.

### Building from clean, seconds (`bench/building/run.sh`)

| Subject | Rust: emit + build = total | binary | C++: emit + build = total | binary |
|---|---|---|---|---|
| prismc | 65.9 + 100.1 = 166.0 | 25.9 MB | 86.2 + 134.6 = 220.8 | 63.8 MB |
| chess | 19.5 | | 23.2 | |
| the table | 19.7 | | 23.5 | |

- **prismc builds a third faster in Rust on the wall** (166.0 s against 220.8 s), and the
  binary is two fifths the size (25.9 MB against 63.8 MB). The emit itself favours Rust
  (65.9 s against 86.2 s): the C++ printer writes 185,385 lines across 63 translation units
  and 280 headers where the Rust printer writes about 115,000.
- **Small programs are close, Rust slightly ahead** (chess 19.5 s against 23.2 s, the table
  19.7 s against 23.5 s), most of either being the runtime library and the World built
  from clean. The 5 November run below read the other way (C++ about 10 s against Rust's
  13 s); that run had agents building beside it and a smaller C++ printer output.

### The GUI table, before `@PerKey`

js-framework-benchmark's operations on one table (`bench/gui`), median wall time in ms
over 10 runs, the Prism rows before the table's rows went over `@PerKey`; GTK 3 is the
toolkit written in C beside it. Prism runs under each target's canvas World.

| Operation | Prism C++ | Prism Rust | GTK 3 |
|---|---|---|---|
| create 1,000 rows | 50 | 44 | 279 |
| replace 1,000 rows | 67 | 60 | 320 |
| update every 10th | 5.4 | 7.1 | 34.5 |
| select a row | 3.1 | 4.8 | 1.0 |
| swap two rows | 3.4 | 4.8 | 61 |
| remove a row | 12.2 | 10.2 | 5.9 |
| create 10,000 rows | 585 | 514 | 11,659 |
| append 1,000 rows | 59 | 48 | 390 |
| clear | 20 | 17 | 42 |

- **Rust wins the bulk operations by a tenth to a fifth** (create 1,000 rows 44 ms against
  50, create 10,000 rows 514 against 585, append 48 against 59, clear 17 against 20, remove
  10.2 against 12.2); both beat GTK 3 by five to twenty times on the creates, the replace
  and the append, and by two times on clear. GTK removes a row faster (5.9 ms).
- **C++ wins the three smallest operations by a quarter to a third** (update every 10th
  5.4 ms against 7.1, select 3.1 against 4.8, swap 3.4 against 4.8); GTK 3 selects in
  1.0 ms, three to five times faster than either.

### Instructions for one operation, after `@PerKey`

Instructions retired by one operation, millions, with the table's rows over `@PerKey`:

| Operation | Prism C++ | Prism Rust |
|---|---|---|
| select a row | 20.5 | 30.5 |
| remove a row | 128 | 80 |

- **The select gap is the Rust World's frame, not the emitted code:** about 11 M of every
  Rust frame is softbuffer presenting the whole frame on macOS (worlds/canvas/rust/README.md,
  "Painting"); the program's own ticks are equal within 5% (the phase table under "Painting
  only the damage" below).
- **Remove runs the other way, C++ 1.6 times Rust:** the column's document is gathered an
  insert at a time, and C++'s persistent `Map` copies a path per insert where Rust's
  updates its own nodes in place (claude/bugs/window-relayout-every-change.md).

## On the Linux cloud machine (6 November 2026)

The cloud VM the branch was finished on: an Intel Xeon with 2 cores and 8 GB, Ubuntu
24.04, clang++ 18 `-O3`, rustc 1.97, nothing else running, no display (the GUI runs under
Xvfb) and no hardware counters (`perf` reports `instructions` as not supported), at
`7d68a86` (the `for` change in). None of these numbers compares with the Mac's above; what
compares is each pair's ratio. The scripts' own files are `bench/building/RESULTS.md`,
`bench/running/RESULTS.md` and `bench/gui/RESULTS.md`.

### Building from clean, seconds (bench/building/run.sh)

| Subject | Rust: emit + build = total | rebuild | binary | C++: emit + build = total | rebuild | binary |
|---|---|---|---|---|---|---|
| prismc (33,506 lines of Prism; 115,103 of Rust, 188,493 of C++) | 55.0 + 140.9 = 195.9 | 121.2 | 29.5 MB | 79.9 + 789.8 = 869.7 | 765.2 | 50.8 MB |
| chess (3,076 lines; 18,293 of Rust, 23,698 of C++) | 1.5 + 72.9 = 74.4 | 19.4 | 7.4 MB | 2.2 + 72.9 = 75.1 | 47.4 | 6.3 MB |
| the table (2,133 lines; 14,391 of Rust, 18,711 of C++) | 1.0 + 69.8 = 70.8 | 16.4 | 6.9 MB | 1.6 + 63.4 = 65.0 | 42.6 | 5.3 MB |

- **Rebuild** is the same build after the emitted program's own sources are touched: the
  program compiled again over the runtime library and the dependencies the clean build left,
  what an edit to a Prism program costs after its first build.
- **On two cores the C++ compiler build is 5.6 times the Rust one** (790 s against 141 s);
  on the Mac's twelve it was a third longer. The C++ build's CPU is the number that shows
  on a small machine (claude/backend.md).
- **prismc emits about 2,100 lines of Rust a second and 2,360 of C++ on itself**, against
  12,000 to 14,000 on chess and the table: the emit time grows faster than the input.
- **Small programs are a tie from clean** (73 s both for chess; most of it the runtime and
  the World), and a Rust rebuild is 16 to 19 s against 43 to 47 s in C++.

### Running prismc (bench/running/run.sh)

| Configuration | prismc compiling itself: s | peak MB | normalize: s | peak MB |
|---|---|---|---|---|
| rust-release | 55.0 | 301 | 0.1 | 22 |
| cpp-O3 | 79.2 | 420 | 0.2 | 36 |

- Instructions are not counted here (no counters; valgrind is too slow for a run this long):
  to rerun on the Mac. On the clock the Rust prismc compiles itself in 55 s against 79 s,
  and holds 301 MB against 420 MB.

### The GUI table under Xvfb (bench/gui/run.sh, 10 runs after 2)

Milliseconds, the median with the fastest and slowest; peak memory in MB. The frame goes
to an Xvfb server that shows nothing, so these are the programs' and the Worlds' own costs
without a compositor; the C++ World presents only the damaged boxes on X11, the Rust World
the whole frame.

| Operation | Prism C++ | Prism Rust | GTK 3 |
|---|---|---|---|
| create 1,000 rows | 76.5 (72.0–81.6), 69 MB | 58.6 (52.5–60.6), 94 MB | 328.5 (313.2–429.1), 100 MB |
| replace 1,000 rows | 128.0 (117.7–142.5) | 94.8 (82.5–133.5) | 383.5 (376.9–441.4) |
| update every 10th | 10.2 (8.9–11.1) | 7.6 (6.4–8.9) | 37.1 (36.6–37.4) |
| select a row | 7.1 (5.9–8.0) | 5.3 (4.1–6.4) | 4.1 (4.0–4.4) |
| swap two rows | 6.8 (6.0–7.4) | 5.2 (4.2–6.0) | 83.1 (82.8–83.4) |
| remove a row | 15.6 (14.4–16.9) | 10.2 (8.9–11.1) | 16.9 (16.8–20.4) |
| create 10,000 rows | 1,200.5 (1,139.1–1,511.0), 555 MB | 1,065.0 (829.9–1,196.7), 598 MB | 26,976.3 (26,384.4–28,112.1), 311 MB |
| append 1,000 rows | 78.2 (74.1–84.8) | 58.9 (52.7–63.5) | 807.0 (791.6–823.5) |
| clear | 48.1 (44.6–56.9) | 36.6 (33.0–39.8) | 65.4 (64.1–65.7) |

- **Rust is ahead on every operation here, by a fifth to a third**, where the Mac had C++
  ahead on the three smallest. The difference is the frame: on the Mac the Rust World's
  softbuffer present cost about 11 M instructions a frame against the C++ World's layer
  image; under Xvfb a whole frame is a copy to a server that shows nothing, and the C++
  World's damage-only present buys less. The Mac's ordering stands for the Mac.
- **GTK 3 selects faster than either** (4.1 ms against 5.3 and 7.1) and is three to five
  times slower on the creates, as on the Mac.

### The GUI check by phase (bench/gui/prism/check.sh)

Every operation's checks pass on both Worlds under Xvfb: no text drawn outside the window,
and the screen shows the table the operation should leave (a pixel comparison: the X11
paths, damage painting on X11 and the on-screen colours after the sRGB change are checked
by it). With no counters the phase table is CPU milliseconds, one run each; a select is
nine ticks, as on the Mac and as before the `for` change.

| target | op | ticks | propagate | build | patch | diff | perform | paint | loop | total | ÷ create1k |
|---|---|---|---|---|---|---|---|---|---|---|---|
| C++ | create1k | 8 | 2.1 | 40.7 | 3.1 | 11.2 | 3.9 | 1.6 | 0.7 | 63.5 | 1.000 |
| C++ | update10th | 9 | 1.4 | 2.8 | 2.5 | 0.8 | 0.4 | 0.2 | 0.5 | 8.5 | 0.134 |
| C++ | select | 9 | 0.8 | 1.4 | 2.1 | 0.4 | 0.2 | 0.2 | 0.4 | 5.5 | 0.087 |
| C++ | swap | 9 | 0.6 | 1.2 | 1.8 | 0.4 | 0.2 | 0.2 | 0.4 | 4.7 | 0.075 |
| C++ | remove | 8 | 1.4 | 8.8 | 1.5 | 0.4 | 0.1 | 0.7 | 0.3 | 13.2 | 0.207 |
| C++ | clear | 8 | 1.0 | 11.1 | 6.6 | 11.2 | 3.4 | 0.3 | 2.0 | 35.5 | 0.559 |
| Rust | create1k | 8 | 2.5 | 27.5 | 3.6 | 10.6 | 4.7 | 2.2 | 0.1 | 51.2 | 1.000 |
| Rust | update10th | 9 | 0.8 | 2.2 | 1.6 | 0.6 | 0.5 | 0.5 | 0.1 | 6.4 | 0.125 |
| Rust | select | 9 | 0.4 | 1.1 | 1.2 | 0.2 | 0.2 | 0.4 | 0.1 | 3.6 | 0.070 |
| Rust | swap | 9 | 0.4 | 1.2 | 1.2 | 0.2 | 0.2 | 0.4 | 0.1 | 3.8 | 0.074 |
| Rust | remove | 8 | 0.7 | 5.0 | 0.9 | 0.3 | 0.1 | 1.3 | 0.1 | 8.4 | 0.165 |
| Rust | clear | 8 | 0.6 | 7.4 | 3.3 | 11.9 | 4.3 | 0.7 | 0.1 | 28.4 | 0.555 |

- **Remove's build is still C++ 8.8 ms against Rust 5.0 ms**, the shape of the Mac's 90 M
  against 29 M instructions (claude/bugs/window-relayout-every-change.md), at a smaller
  ratio on the clock.

### The cost pairs (tests/cost/run.sh, valgrind's cachegrind, instructions)

| Pair | one | forty | Rust | one | forty | C++ |
|---|---|---|---|---|---|---|
| a `let` read by ten | 2,907 M | 2,916 M | ok | 4,613 M | 4,674 M | ok |
| a builder over its last source | 252.7 M | 253.5 M | ok | 488.5 M | 489.5 M | ok |
| a row per key changed (`@PerKey`) | 32.1 M | 66.0 M | ok | 55.6 M | 78.8 M | ok |
| no kept row woken by a removed key (ruling FV) | 1,239 M | 1,616 M | ok | 678 M | 1,165 M | ok |

- **The per-key pairs' bar is marginal:** each extra edit (0.87 M here in Rust, 0.59 M in C++)
  must cost under a twentieth of the one-edit run, which builds the 2,000 rows; doubling the
  whole run made the bar turn on how large the build counts under a machine's counter.
- **A removed key costs about 9.4 M in Rust and 12 M in C++** with 2,000 kept rows, none
  of them woken: the `@PerKey` walk, the `for`'s key walk and the removed row's leaving.

## On Jimmy's Mac with agents building beside it (5 November 2026, and the GUI runs after it)

The sections from here on are the 5 November run at `aa500e5` and the GUI runs that
followed it on the same Mac, each naming the load it ran under.

### Building Prism: source to a runnable binary, from clean, seconds

| Subject | Rust: emit + build = total | binary | C++: emit + build = total | binary |
|---|---|---|---|---|
| prismc (24,390 lines of Prism with the prelude; 103,850 of Rust, 154,462 of C++) | 70.9 + 97.2 = 168.1 | 25.5 MB | 63.4 + 143.5 = 206.9 | 58.2 MB |
| chess (1,786 lines; 6,327 of Rust, 8,334 of C++) | 1.3 + 13.1 = 14.4 | 4.5 MB | 1.3 + 10.1 = 11.4 | 3.3 MB |
| the table (1,538 lines; 5,745 of Rust, 7,336 of C++) | 1.0 + 12.8 = 13.8 | 4.3 MB | 0.9 + 10.0 = 10.9 | 2.9 MB |

- cargo ran 12 jobs and make `-j12`; the load average was 20 during the prismc builds.
- **Small programs build faster in C++** (about 10 s against 13 s, most of either being the
  runtime and the World); **the compiler builds faster in Rust** (97 s against 144 s), and
  on CPU time by far (agent E's quieter measurement: 255 s against 1,003 s).

### Running Prism once compiled: prismc as the workload

| Run | Rust wall | instructions | peak footprint | C++ wall | instructions | peak footprint |
|---|---|---|---|---|---|---|
| prismc compiling itself | 74.9 s | 724.2 G | 3,020 MB | 65.5 s | 729.2 G | 1,039 MB |
| compiling `sem-ch07-normalize` | 0.13 s | 1.38 G | 20 MB | 0.14 s | 1.52 G | 15 MB |

- **Instructions are equal within 1% on the self-compile**; C++ is about 13% faster on the
  clock and uses a third of the memory.
- **The memory gap is the array bug** (`claude/bugs/small-arrays-whole-chunks.md`): on
  branch `claude-arrays` the C++ footprint is already 293 MB, and the Rust `Map` port took
  the Rust wall time to about 57 s; the Rust `Array` is in progress.

### Running Prism once compiled: the GUI table

js-framework-benchmark's operations; the median of 10 runs after 2 warm-ups, one process
per operation, milliseconds; peak footprint where it was read. Prism runs under each
target's canvas World. The Cuis run was stopped before it finished.

| Operation | Prism C++ | Prism Rust | GTK 3 (C) | Cuis (Cog VM) |
|---|---|---|---|---|
| create 1,000 rows | 700 (115 MB) | 532 (203 MB) | 277 (75 MB) | 5,000 (125 MB) |
| replace 1,000 rows | 791 (128 MB) | 1,484 (274 MB) | 327 (76 MB) | 11,591 (131 MB) |
| update every 10th row | 820 (148 MB) | 1,716 (235 MB) | 36 (76 MB) | 2.3 (125 MB) |
| select a row | 821 (149 MB) | 1,685 (226 MB) | 1.1 (76 MB) | no figure |
| swap two rows | 789 (120 MB) | 1,465 (235 MB) | 61 (79 MB) | 69 (120 MB) |
| remove a row | 781 (150 MB) | 1,449 (220 MB) | 6.7 (76 MB) | 6,018 (123 MB) |
| create 10,000 rows | 57,563 (678 MB) | 7,900 (1,209 MB) | 11,529 (282 MB) | not run |
| append 1,000 rows | 2,717 (140 MB) | 2,353 (277 MB) | 393 (98 MB) | not run |
| clear | 96 (115 MB) | 947 (197 MB) | 42 (75 MB) | not run |

What the table shows, none of it traced to a cause yet (each becomes a report under
`claude/bugs/` once it is):

- **Every small operation in Prism costs about what building the table costs** (select:
  821 ms in C++ against 700 ms for create; GTK selects in 1.1 ms). The work per change is
  proportional to the table, not to the change.
- **C++ is about twice as fast as Rust on those operations and uses about half the
  memory**, but Rust is faster on the two creates.
- **Create 10,000 rows in C++ takes 57.6 s against Rust's 7.9 s**: ten times the rows for
  eighty times the time, so something there is quadratic in C++ alone.
- **Clear** is 96 ms in C++ and 947 ms in Rust.
- **The Cuis figures are not to be leaned on**: 5 s to create and 2.3 ms to update do not
  sit together, select printed nothing, and the run was cut.
- **A GUI run needs the display awake.** The first attempt hung in GTK for two and a half
  hours at 0% CPU when the display slept; `bench/gui/run.sh` should hold it awake itself.

### After the window fix

`bench/gui/run.sh prism-cpp prism-rust` with `RUNS=5 WARMUP=1`, on the same Mac with other
agents building (load average 7 to 26 before, about 8 after): before at `3fb9362`, after
at `2c6d8be` (branch `claude-incremental`: the window draws only what shows and holds its
document with `@Distinct`; the lowering caches multiply-read `let`s and a `for`'s builder
touches only the keys that changed; the arrays work of `claude-self-hosting` merged in).
Medians in ms, peak resident memory in MB; Prism only.

| Operation | C++ before | C++ after | Rust before | Rust after |
|---|---|---|---|---|
| create1k | 702 (197 MB) | 55 (171 MB) | 515 (289 MB) | 47 (218 MB) |
| replace1k | 798 (211 MB) | 65 (184 MB) | 1,428 (322 MB) | 54 (220 MB) |
| update10th | 822 (199 MB) | 48 (173 MB) | 1,642 (305 MB) | 40 (208 MB) |
| select | 797 (198 MB) | 46 (173 MB) | 1,651 (304 MB) | 38 (208 MB) |
| swap | 783 (197 MB) | 39 (172 MB) | 1,426 (304 MB) | 32 (208 MB) |
| remove | 773 (197 MB) | 41 (172 MB) | 1,413 (292 MB) | 36 (208 MB) |
| create10k | 57,638 (692 MB) | 423 (361 MB) | 29,046 (1,236 MB) | 349 (397 MB) |
| append1k | 2,718 (254 MB) | 68 (193 MB) | 2,287 (381 MB) | 56 (228 MB) |
| clear | 102 (198 MB) | 26 (171 MB) | 937 (288 MB) | 20 (218 MB) |

- **Fifteen to forty times faster on small changes, and create10k is linear again.**
- **A small change still costs most of a create1k** (select 46 ms against 55 in C++, 0.53 G
  instructions against 0.71 G), so `bench/gui/prism/check.sh`'s second check still fails;
  GTK selects in 1.1 ms. What is left needs a design:
  `claude/bugs/window-relayout-every-change.md`.

### Instructions by phase, and the cheap runtime fixes

`bench/gui/prism/check.sh` now prints each timed step's instructions by phase on both
targets, through `prism_rtl`'s probe (`world::probe` in Rust, `probe` in `world.hpp`):
the runtime marks propagation, the build/swap/leave walks, the patch pass and the
document diff inside a tick, and the canvas World marks performing the outputs, the
compact round and painting. Each thread's instructions between its marks go to the phase
it was in (macOS's `thread_selfcounts`; the C++ World ticks on its big stack's thread and
paints on the main one); `other` is the rest of the process's (`proc_pid_rusage`), its
other threads. One run each, on Jimmy's Mac with agents building (load 8 to 16), at the
merged tree with both seeds rebuilt (`885c89d` merged), the fixes below in. Millions of
instructions:

| target | op | ticks | propagate | build | patch | diff | perform | paint | loop + other | total | ÷ create1k | paint ÷ create1k |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| C++ | create1k | 7 | 19.5 | 270.1 | 179.4 | 24.6 | 7.9 | 141.6 | 7.3 | 650.4 | 1.000 | 0.218 |
| C++ | update10th | 8 | 33.2 | 195.3 | 165.3 | 4.9 | 1.3 | 143.2 | 4.8 | 548.0 | 0.843 | 0.220 |
| C++ | select | 8 | 12.5 | 192.2 | 164.9 | 3.8 | 0.8 | 143.3 | 5.5 | 523.0 | 0.804 | 0.220 |
| C++ | swap | 7 | 12.7 | 192.3 | 83.8 | 3.0 | 0.4 | 143.3 | 4.4 | 440.0 | 0.677 | 0.220 |
| C++ | remove | 7 | 11.9 | 143.8 | 163.6 | 2.9 | 0.5 | 143.5 | 5.2 | 471.3 | 0.725 | 0.221 |
| C++ | clear | 7 | 11.5 | 79.5 | 4.7 | 26.9 | 4.5 | 138.5 | 7.2 | 272.8 | 0.419 | 0.213 |
| Rust | create1k | 7 | 47.3 | 152.0 | 194.2 | 20.3 | 13.3 | 157.9 | 4.5 | 589.6 | 1.000 | 0.268 |
| Rust | update10th | 8 | 23.8 | 107.7 | 185.1 | 3.5 | 3.0 | 155.2 | 2.1 | 480.4 | 0.815 | 0.263 |
| Rust | select | 8 | 5.4 | 104.9 | 184.5 | 2.5 | 2.2 | 154.9 | 2.2 | 456.7 | 0.775 | 0.263 |
| Rust | swap | 7 | 5.6 | 105.7 | 93.9 | 2.1 | 1.1 | 154.9 | 1.6 | 364.9 | 0.619 | 0.263 |
| Rust | remove | 7 | 5.2 | 88.5 | 183.9 | 2.2 | 1.2 | 155.5 | 1.2 | 437.7 | 0.742 | 0.264 |
| Rust | clear | 7 | 5.1 | 39.4 | 6.0 | 21.2 | 6.4 | 139.5 | 1.0 | 218.6 | 0.371 | 0.237 |

The compact round is under 0.05 M in every step and is left out.

- **Painting alone is a fifth to a quarter of create1k** (C++ 143 M of 650 M, Rust 155 M of
  590 M), whatever the step: clearing the table paints as much as selecting a row. The
  canvas Worlds paint the whole 900 by 800 window (at the backing scale) once per settled
  state, so while they do, no small change can come under a tenth of create1k: the bar is
  out of reach by painting alone, by a factor of 2.2 in C++ and 2.6 in Rust (doubt PB).
- **The rest of a select is the build walks and the patch pass:** in C++ 192 M of build
  (the 1,000 row documents and the host documents rebuilt, and every builder's walk) and
  165 M of patch (the layouts), against create1k's 270 M and 179 M; Rust spends less in the
  builds (105 M) and more in the patch (185 M). Propagation, the diff and the World's
  perform are each under 3% of the step.
- **Swap's patch is half the others'** (84 M, 94 M): one layout where the others lay out
  twice (`claude/bugs/window-relayout-every-change.md`, doubt MT: no new word, so no second
  layout).

**The cheap runtime fixes** (claude/incremental-host-document.md, "Without a route"): `==`
on a `Map` or an `Array` answers at once for a node both sides share once its values are
known to equal themselves, a NaN staying unequal; and a structural dynamic's equal snapshot
keeps the value it had. Before is the same tree with those two runtime changes reverted
(the probe in both). Instructions from `check.sh`, one run each; times from
`bench/gui/run.sh prism-cpp prism-rust`, `RUNS=5 WARMUP=1`, medians in ms (load 7 to 9
before, 12 to 17 after):

| op | C++ before | C++ after | C++ ratio | Rust before | Rust after | Rust ratio | C++ ms before → after | Rust ms before → after |
|---|---|---|---|---|---|---|---|---|
| create1k | 649.2 M | 650.4 M | 1.002 | 590.9 M | 589.6 M | 0.998 | 53.3 → 49.5 | 43.9 → 44.2 |
| update10th | 549.9 M | 548.0 M | 0.997 | 483.9 M | 480.4 M | 0.993 | 46.1 → 45.3 | 38.0 → 38.5 |
| select | 517.4 M | 523.0 M | 1.011 | 459.3 M | 456.7 M | 0.994 | 43.2 → 41.8 | 36.8 → 36.7 |
| swap | 440.4 M | 440.0 M | 0.999 | 367.8 M | 364.9 M | 0.992 | 37.8 → 36.2 | 29.8 → 29.4 |
| remove | 469.9 M | 471.3 M | 1.003 | 436.9 M | 437.7 M | 1.002 | 40.7 → 38.3 | 35.2 → 34.4 |
| clear | 275.0 M | 272.8 M | 0.992 | 217.1 M | 218.6 M | 1.007 | 25.1 → 23.6 | 17.9 → 17.8 |

(replace1k 63.2 → 61.0 and 52.8 → 52.1, create10k 423 → 407 and 345 → 348, append1k 66.1 →
63.5 and 55.1 → 55.4 ms; peak memory unchanged within 3 MB.)

- **They change nothing measurable here:** every ratio is within 1% of 1, and the times
  move within the runs' spread. The comparisons they make cheap are not where the steps
  spend: each host document is rebuilt as a new map every tick (no node of it shared with
  the last snapshot), so its comparison still walks every entry; what the fixes save is
  the walk into each entry's shared children, a small part of the build phase. They are
  the precondition the note gives them (route B's per-place snapshot keeps the map's
  nodes, and then a comparison costs the changed paths), not a gain of their own.
- **Not done, because it is not the runtimes':** a builder skipping when its source is the
  node it last built from is emitted code (`ForBuild`, prismc's printer templates).

### A builder over the source it last built from

`bench/gui/prism/check.sh`, one run each, on Jimmy's Mac with the tree's own seeds rebuilt
(branch `claude-window-cache`, from `d30829d`): before is that tree, after adds a `for`'s
builder that does nothing when its source is the very map it last built from (`ForBuild`
in both printers, `Map::same` in both runtimes; a host's rows over its document included).
Millions of instructions; the transcripts are unchanged.

| target | op | ticks | build before | build after | patch | paint | total before | total after | after ÷ create1k |
|---|---|---|---|---|---|---|---|---|---|
| C++ | create1k | 7 | 269.8 | 260.4 | 178.4 | 141.8 | 718.5 | 636.2 | 1.000 |
| C++ | update10th | 8 | 195.8 | 173.1 | 165.2 | 141.2 | 555.4 | 521.4 | 0.820 |
| C++ | select | 8 | 192.1 | 168.6 | 164.3 | 141.1 | 530.0 | 493.3 | 0.775 |
| C++ | swap | 7 | 192.6 | 170.4 | 83.7 | 141.1 | 440.1 | 412.8 | 0.649 |
| C++ | remove | 7 | 143.8 | 127.9 | 163.5 | 141.1 | 472.2 | 449.8 | 0.707 |
| Rust | create1k | 7 | 152.7 | 144.9 | 194.0 | 155.4 | 476.1 | 581.3 | 1.000 |
| Rust | update10th | 8 | 108.1 | 87.8 | 184.9 | 155.2 | 608.6 | 461.5 | 0.794 |
| Rust | select | 8 | 105.4 | 84.7 | 184.3 | 155.0 | 587.4 | 437.4 | 0.752 |
| Rust | swap | 7 | 105.9 | 85.5 | 93.7 | 154.9 | 497.7 | 345.5 | 0.594 |
| Rust | remove | 7 | 88.5 | 73.8 | 183.3 | 154.8 | 449.4 | 423.4 | 0.728 |

- **The build phase falls by a tenth to a fifth** (C++ select 192 M to 169 M, Rust 105 M
  to 85 M): in the ticks after the table's write the builders of the root node, and the
  hosts' rows over an unchanged snapshot, no longer walk their key sets. Propagation,
  patch, diff and paint are unchanged within 1%.
- **The before run's Rust totals are not comparable:** its frames' painting fell partly
  into the loop phase (127 M of "loop" in update10th, select and swap, and create1k's paint
  at 33 M), so compare the build column; the after run's Rust paint is 155 M throughout,
  as in every earlier run.
- **What is left is the flat host document** (`claude/incremental-host-document.md`): the
  1,000 row documents and the hosts' whole documents in the build phase, two whole
  layouts in the patch phase, and a whole frame painted.

### The host document as a map of dynamics

`bench/gui/prism/check.sh`, one run each, on Jimmy's Mac (machine shared with nothing
heavy but this agent's builds), branch `claude-window-cache`: before is `ebaf3cf`, the flat
document `$(Path ==> Elem c)` with the window laying out the whole tree as one function;
after is the document as `$(Path ==> $Elem c)` with the window laying out a row per
element (claude/incremental-host-document.md). Millions of instructions; the compact round
is under 0.05 M and left out.

| target | op | ticks before → after | build before → after | patch before → after | paint | total before → after | ÷ create1k after |
|---|---|---|---|---|---|---|---|
| C++ | create1k | 7 → 8 | 260.5 → 420.9 | 178.5 → 30.4 | 141.7 | 636.4 → 747.1 | 1.000 |
| C++ | update10th | 8 → 9 | 173.8 → 14.5 | 165.6 → 12.6 | 141.2 | 522.2 → 181.8 | 0.243 |
| C++ | select | 8 → 9 | 169.0 → 6.3 | 164.5 → 11.7 | 141.3 | 493.7 → 165.9 | 0.222 |
| C++ | swap | 7 → 9 | 170.7 → 7.3 | 84.0 → 11.7 | 141.1 | 414.2 → 166.8 | 0.223 |
| C++ | remove | 7 → 8 | 127.3 → 90.0 | 163.8 → 11.6 | 141.1 | 449.1 → 250.7 | 0.336 |
| C++ | clear | 7 → 8 | 71.9 → 82.0 | 4.6 → 8.3 | 140.5 | 258.8 → 330.8 | 0.443 |
| Rust | create1k | 7 → 8 | 145.0 → 246.8 | 193.6 → 28.2 | 155.3 | 580.9 → 646.6 | 1.000 |
| Rust | update10th | 8 → 9 | 88.3 → 11.7 | 184.9 → 12.8 | 155.1 | 461.9 → 193.5 | 0.299 |
| Rust | select | 8 → 9 | 85.6 → 4.5 | 184.0 → 11.9 | 155.0 | 436.1 → 176.9 | 0.274 |
| Rust | swap | 7 → 9 | 86.1 → 5.3 | 93.6 → 11.9 | 155.0 | 346.0 → 177.9 | 0.275 |
| Rust | remove | 7 → 8 | 73.9 → 29.0 | 183.2 → 10.9 | 154.9 | 423.5 → 202.9 | 0.314 |
| Rust | clear | 7 → 8 | 31.5 → 39.3 | 5.7 → 9.0 | 149.0 | 207.2 → 293.0 | 0.453 |

- **A small change is the paint and a little:** select, swap and update10th spend 18 to
  33 M besides painting (C++) where they spent 250 to 380 M; no row is laid out but the
  moved ones and no document of the table is built. Painting is now four fifths of a
  select.
- **Building the table costs more:** create1k's build is 421 M (C++) and 247 M (Rust)
  against 261 M and 145 M, a node per element of the window's layout and per measured
  label, and clear's diff and build grow with them (tearing those down); create1k's
  patch falls from 179 M to 30 M (one placing pass, no whole layout).
- **A tick more per press** (eight to nine, seven to nine for swap): the window lays out
  a gathered `for` per level of the tree (Jimmy's story: every gathered `for` adds one).
- **Remove still costs C++ three times Rust** (90 M against 29 M of build): the outer of
  the column's document is gathered from its 3,003 entries an insert at a time
  (claude/bugs/window-relayout-every-change.md).
- On the way: the layout no longer builds an array of (path, result) pairs per container
  per change (C++ select 287 M before that, Rust 260 M), a map of a map's entries is built
  at once (C++ select 230 M before that), and a hosted builder gathers its host's outer
  only when its rows' keys changed (C++ swap 237 M before that).

### Painting only the damage

`bench/gui/prism/check.sh`, one run each, on Jimmy's Mac (on mains power, load about 3,
nothing else building), branch `claude-damage`: before is `f6c16a2`, the canvas Worlds
painting the whole frame for every settled scene; after, each World paints only the
boxes a new scene's draw commands damage (worlds/canvas/rust/src/damage.rs), and each
window's colour space is sRGB. Millions of instructions; the compact round is under
0.05 M and left out; "loop" is the loop's own marks, where Core Animation's commit of a
Rust frame can fall.

| target | op | ticks | build | patch | paint before → after | loop before → after | total before → after | ÷ create1k after |
|---|---|---|---|---|---|---|---|---|
| C++ | create1k | 8 | 421.2 | 30.4 | 145.7 → 19.3 | 58.2 → 62.4 | 814.1 → 688.6 | 1.000 |
| C++ | update10th | 9 | 14.5 | 12.6 | 143.8 → 2.7 | 0.8 → 0.7 | 183.7 → 41.7 | 0.061 |
| C++ | select | 9 | 6.3 | 11.7 | 144.7 → 1.3 | 0.6 → 0.6 | 168.2 → 24.5 | 0.036 |
| C++ | swap | 9 | 7.3 | 11.7 | 145.0 → 1.3 | 0.6 → 0.6 | 169.5 → 25.4 | 0.037 |
| C++ | remove | 8 | 89.8 | 11.7 | 146.1 → 17.1 | 0.5 → 0.5 | 255.2 → 125.3 | 0.182 |
| C++ | clear | 8 | 82.0 | 8.3 | 146.2 → 9.4 | 13.2 → 5.5 | 340.6 → 197.9 | 0.287 |
| Rust | create1k | 8 | 248.9 | 29.0 | 32.9 → 42.1 | 100.8 → 7.0 | 631.0 → 540.0 | 1.000 |
| Rust | update10th | 9 | 12.0 | 12.9 | 33.6 → 16.1 | 4.4 → 18.2 | 77.0 → 75.1 | 0.139 |
| Rust | select | 9 | 4.6 | 12.1 | 32.6 → 8.8 | 0.3 → 2.3 | 55.1 → 33.3 | 0.062 |
| Rust | swap | 9 | 5.6 | 11.9 | 33.7 → 8.7 | 4.4 → 4.4 | 61.1 → 36.0 | 0.067 |
| Rust | remove | 8 | 29.3 | 10.9 | 33.6 → 28.6 | 4.4 → 5.1 | 84.4 → 80.3 | 0.149 |
| Rust | clear | 8 | 39.8 | 9.0 | 144.0 → 10.8 | 138.5 → 5.1 | 431.0 → 156.1 | 0.289 |

Wall time, `driver OP 20 3` run by hand (not `bench/gui/run.sh`), medians of 20 runs in ms
with the fastest and slowest, before → after:

| op | C++ | Rust |
|---|---|---|
| select | 15.8 (14.7–16.8) → 3.2 (2.6–3.8) | 14.9 (14.1–16.0) → 5.3 (3.7–7.4) |
| update10th | 17.9 (16.8–19.0) → 5.5 (4.8–6.1) | 17.6 (16.4–19.6) → 7.6 (6.5–8.9) |
| remove | 22.4 (21.4–23.6) → 12.3 (11.6–13.2) | 18.1 (17.0–19.9) → 10.1 (9.1–11.3) |
| create1k | 60.0 (58.9–62.1) → 50.3 (49.0–51.1) | 50.8 (40.1–60.5) → 43.9 (41.7–45.3) |

- **Most of the frame's cost was not painting but colour matching.** Of C++'s 145 M paint,
  about 125 M was Core Animation converting every pixel of the frame from sRGB to the
  screen's colour space on the CPU, in this process, when the frame was committed; cairo
  painting the whole frame was about 20 M. With the window's colour space set to sRGB, the
  frame's own, the conversion is gone (the window server matches colours as it
  composites). The same conversion is the Rust table's 100 to 140 M bursts in "loop" and
  "paint" (create1k and clear before): softbuffer's frame is committed with the run loop's
  transaction, which falls inside the timed step or after it from run to run, so the
  Rust before row is low in paint for most steps only because the conversion was counted
  elsewhere or not at all.
- **A select now paints 1.3 M (C++) and 8.8 M (Rust):** two labels' boxes. In Rust about
  11 M of each frame is presenting it through softbuffer, whatever the damage: its macOS
  buffer is a new zeroed allocation per frame, the whole frame is copied in, and it
  becomes a new `CGImage` (worlds/canvas/rust/README.md, "Painting"). Rust's update10th
  and select vary by a few M from run to run in "loop" for the same reason as before, the
  commit's timing.
- **A remove, a create and a clear damage most of the frame** (every row below the removed
  one moves; every row comes or goes): C++ 17 M, 19 M and 9 M, Rust 29 M, 42 M and 11 M,
  about what painting the whole frame costs in each, with the comparison's cost in it.
  The Prism window has no scroll pane; a scroll would move every row and cost as a remove.
- **A small change is now the program's own work:** a C++ select is 24.5 M, 22 M of it the
  ticks (build, patch, propagate, diff, perform). What is left of the window's per-change
  cost is `claude/bugs/window-relayout-every-change.md`.
- **The wall times fall by four to five times on the small steps**: a select is 3.2 ms in
  C++ and 5.3 ms in Rust (GTK's is about 1.1 ms in the table above). The instruction
  counts miss what the window server does with a frame after the flush; the clock here
  stops at the flush, as before.
