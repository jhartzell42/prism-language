# Benchmarks, preliminary

*Written by Claude on the claude-self-hosting branch; subject to change.* Benchmark runs
on Jimmy's Apple M2 Pro (Mac14,10, 12 cores, 16 GB, macOS 26.6.2), each under its own
dated section with the conditions it ran under: first the run with Low Power Mode on
(6 November); then a section kept for the Linux cloud machine; then the 5 November run of
`bench/building/run.sh` and most of one of `bench/running/run.sh`, at `aa500e5` (after the
`Text`, header, `Rc` and string work; before nested modules, the array fix and regex),
**with agents building beside it** (load 4 to 20), and after it the GUI runs that measured
the incremental work, each on the same Mac under the load its section names. Wall times
on the busy runs are rough; instruction counts and peak memory are not. Each pipeline is
all one language: the Rust prismc emits Rust and cargo builds it (release, opt-level 3).

## On Jimmy's Mac with Low Power Mode on (6 November 2026)

The same Mac M2 Pro, Low Power Mode on: every absolute time is pessimistic.

### Building from clean, seconds (`bench/building/run.sh`)

| Subject | Rust: emit + build = total | binary |
|---|---|---|
| prismc | 65.9 + 100.1 = 166.0 | 25.9 MB |
| chess | 19.5 | |
| the table | 19.7 | |


### The GUI table, before `@PerKey`

js-framework-benchmark's operations on one table (`bench/gui`), median wall time in ms
over 10 runs, the Prism rows before the table's rows went over `@PerKey`; GTK 3 is the
toolkit written in C beside it. Prism runs under the canvas World.

| Operation | Prism Rust | GTK 3 |
|---|---|---|
| create 1,000 rows | 44 | 279 |
| replace 1,000 rows | 60 | 320 |
| update every 10th | 7.1 | 34.5 |
| select a row | 4.8 | 1.0 |
| swap two rows | 4.8 | 61 |
| remove a row | 10.2 | 5.9 |
| create 10,000 rows | 514 | 11,659 |
| append 1,000 rows | 48 | 390 |
| clear | 17 | 42 |

- Rust beats GTK 3 by five to twenty times on the creates, the replace
  and the append, and by two times on clear. GTK removes a row faster (5.9 ms).
- GTK 3 selects in
  1.0 ms.

### Instructions for one operation, after `@PerKey`

Instructions retired by one operation, millions, with the table's rows over `@PerKey`:

| Operation | Prism Rust |
|---|---|
| select a row | 30.5 |
| remove a row | 80 |

- About 11 M of every
  Rust frame is softbuffer presenting the whole frame on macOS (worlds/canvas/rust/src/app.rs).

## On the Linux cloud machine (6 November 2026)

The cloud VM the branch was finished on: an Intel Xeon with 2 cores and 8 GB, Ubuntu
24.04, rustc 1.97, nothing else running, no display (the GUI runs under
Xvfb) and no hardware counters (`perf` reports `instructions` as not supported), at
`7d68a86` (the `for` change in). None of these numbers compares with the Mac's above. The scripts' own files are `bench/building/RESULTS.md`,
`bench/running/RESULTS.md` and `bench/gui/RESULTS.md`.

### Building from clean, seconds (bench/building/run.sh)

| Subject | Rust: emit + build = total | rebuild | binary |
|---|---|---|---|
| prismc (33,506 lines of Prism; 115,103 of Rust) | 55.0 + 140.9 = 195.9 | 121.2 | 29.5 MB |
| chess (3,076 lines; 18,293 of Rust) | 1.5 + 72.9 = 74.4 | 19.4 | 7.4 MB |
| the table (2,133 lines; 14,391 of Rust) | 1.0 + 69.8 = 70.8 | 16.4 | 6.9 MB |

- **Rebuild** is the same build after the emitted program's own sources are touched: the
  program compiled again over the runtime library and the dependencies the clean build left,
  what an edit to a Prism program costs after its first build.
- **prismc emits about 2,100 lines of Rust a second on itself**, against
  12,000 to 14,000 on chess and the table: the emit time grows faster than the input.

### Running prismc (bench/running/run.sh)

| Configuration | prismc compiling itself: s | peak MB | normalize: s | peak MB |
|---|---|---|---|---|
| rust-release | 55.0 | 301 | 0.1 | 22 |

- Instructions are not counted here (no counters; valgrind is too slow for a run this long):
  to rerun on the Mac. On the clock the Rust prismc compiles itself in 55 s,
  and holds 301 MB.

### The GUI table under Xvfb (bench/gui/run.sh, 10 runs after 2)

Milliseconds, the median with the fastest and slowest; peak memory in MB. The frame goes
to an Xvfb server that shows nothing, so these are the programs' and the Worlds' own costs
without a compositor; the Rust World presents
the whole frame.

| Operation | Prism Rust | GTK 3 |
|---|---|---|
| create 1,000 rows | 58.6 (52.5–60.6), 94 MB | 328.5 (313.2–429.1), 100 MB |
| replace 1,000 rows | 94.8 (82.5–133.5) | 383.5 (376.9–441.4) |
| update every 10th | 7.6 (6.4–8.9) | 37.1 (36.6–37.4) |
| select a row | 5.3 (4.1–6.4) | 4.1 (4.0–4.4) |
| swap two rows | 5.2 (4.2–6.0) | 83.1 (82.8–83.4) |
| remove a row | 10.2 (8.9–11.1) | 16.9 (16.8–20.4) |
| create 10,000 rows | 1,065.0 (829.9–1,196.7), 598 MB | 26,976.3 (26,384.4–28,112.1), 311 MB |
| append 1,000 rows | 58.9 (52.7–63.5) | 807.0 (791.6–823.5) |
| clear | 36.6 (33.0–39.8) | 65.4 (64.1–65.7) |

- **GTK 3 selects faster** (4.1 ms against 5.3) and is three to five
  times slower on the creates, as on the Mac.

### The GUI check by phase (bench/gui/prism/check.sh)

Every operation's checks pass under Xvfb: no text drawn outside the window,
and the screen shows the table the operation should leave (a pixel comparison: the X11
paths, damage painting on X11 and the on-screen colours after the sRGB change are checked
by it). With no counters the phase table is CPU milliseconds, one run each; a select is
nine ticks, as on the Mac and as before the `for` change.

| target | op | ticks | propagate | build | patch | diff | perform | paint | loop | total | ÷ create1k |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Rust | create1k | 8 | 2.5 | 27.5 | 3.6 | 10.6 | 4.7 | 2.2 | 0.1 | 51.2 | 1.000 |
| Rust | update10th | 9 | 0.8 | 2.2 | 1.6 | 0.6 | 0.5 | 0.5 | 0.1 | 6.4 | 0.125 |
| Rust | select | 9 | 0.4 | 1.1 | 1.2 | 0.2 | 0.2 | 0.4 | 0.1 | 3.6 | 0.070 |
| Rust | swap | 9 | 0.4 | 1.2 | 1.2 | 0.2 | 0.2 | 0.4 | 0.1 | 3.8 | 0.074 |
| Rust | remove | 8 | 0.7 | 5.0 | 0.9 | 0.3 | 0.1 | 1.3 | 0.1 | 8.4 | 0.165 |
| Rust | clear | 8 | 0.6 | 7.4 | 3.3 | 11.9 | 4.3 | 0.7 | 0.1 | 28.4 | 0.555 |


### The cost pairs (tests/cost/run.sh, valgrind's cachegrind, instructions)

| Pair | one | forty | Rust |
|---|---|---|---|
| a `let` read by ten | 2,907 M | 2,916 M | ok |
| a builder over its last source | 252.7 M | 253.5 M | ok |
| a row per key changed (`@PerKey`) | 32.1 M | 66.0 M | ok |
| no kept row woken by a removed key (ruling FV) | 1,239 M | 1,616 M | ok |

- **The per-key pairs' bar is marginal:** each extra edit (0.87 M here in Rust)
  must cost under a twentieth of the one-edit run, which builds the 2,000 rows; doubling the
  whole run made the bar turn on how large the build counts under a machine's counter.
- **A removed key costs about 9.4 M in Rust** with 2,000 kept rows, none
  of them woken: the `@PerKey` walk, the `for`'s key walk and the removed row's leaving.

### jemalloc (Rust), 8 November 2026

Every Rust program prismc emits declares jemalloc its global allocator (`tikv-jemallocator`
0.7.0, jemalloc 5.3.1), where the system's `malloc`
(glibc 2.39) served it. The same VM and night as the sections above (compare only within
this section), load 0.25 at the start; the before is the Rust prismc at `e8b0aa2` (glibc),
the after the branch's fixed point at `10211d6`: the same program but for the three lines
of the allocator in its `main.rs`.

- **The Rust prismc compiling itself to Rust** (`prismc compile prismc --target rust`,
  `/usr/bin/time -v`, six runs each, interleaved):

  | Allocator | wall, s (median; runs) | peak RSS |
  |---|---|---|
  | glibc (before) | 69.9 (64.7, 65.2, 67.7, 72.1, 73.7, 75.5) | 310.8 MB |
  | jemalloc (after) | 51.6 (49.4, 50.1, 50.7, 52.5, 53.2, 54.0) | 314.6 MB |

  26% less wall time, 4 MB more peak memory. The glibc runs fell from 75.5 s to 64.7 s
  over the twelve (the jemalloc runs held between 49 and 54 s), so the gap is between 11
  and 25 s run for run.
- **The GUI table by phase** (`bench/gui/prism/check.sh prism-rust`, CPU milliseconds, one
  run each, the glibc driver emitted by the before prismc, every check passing on both):

  | allocator | op | propagate | build | patch | diff | perform | paint | total |
  |---|---|---|---|---|---|---|---|---|
  | glibc | create1k | 3.4 | 42.1 | 6.3 | 18.1 | 10.8 | 3.8 | 84.7 |
  | jemalloc | create1k | 2.1 | 34.6 | 3.0 | 11.2 | 4.5 | 2.5 | 58.1 |
  | glibc | update10th | 1.3 | 3.0 | 2.1 | 0.7 | 0.7 | 0.7 | 8.6 |
  | jemalloc | update10th | 1.1 | 3.6 | 1.8 | 0.9 | 0.7 | 0.7 | 8.9 |
  | glibc | select | 0.9 | 1.7 | 1.6 | 0.3 | 0.2 | 0.6 | 5.4 |
  | jemalloc | select | 0.6 | 1.3 | 1.2 | 0.2 | 0.2 | 0.4 | 4.0 |
  | glibc | swap | 1.0 | 1.9 | 1.6 | 0.3 | 0.3 | 0.6 | 5.8 |
  | jemalloc | swap | 0.7 | 1.7 | 1.4 | 0.2 | 0.2 | 0.6 | 4.9 |
  | glibc | remove | 2.5 | 10.1 | 1.9 | 0.5 | 0.2 | 2.2 | 17.6 |
  | jemalloc | remove | 1.4 | 6.9 | 1.1 | 0.3 | 0.1 | 1.9 | 11.9 |
  | glibc | clear | 1.8 | 16.1 | 5.4 | 28.5 | 8.5 | 1.3 | 61.9 |
  | jemalloc | clear | 1.0 | 7.8 | 0.9 | 7.3 | 2.6 | 0.9 | 20.7 |

  The bulk operations fall the most (create1k 84.7 against 58.1 ms, remove 17.6 against
  11.9, clear 61.9 against 20.7, its diff 28.5 against 7.3); update10th is equal within one
  run's noise.
- **The cost pairs** (`tests/cost/run.sh`, cachegrind) after: a `let` read by ten 2,393 M /
  2,394 M; a builder over its last source 184.3 M / 185.2 M; a row per key changed 23.3 M /
  57.1 M (0.87 M per extra edit); no kept row woken 1,228 M / 1,545 M (8.1 M per extra
  edit); all four within their bars. No glibc run of the pairs at this commit was made;
  the 6 November row above (2,907 M, 252.7 M, 32.1 M, 1,239 M for the one-run sides) is
  an older program and runtime, so the difference is not the allocator's alone.

### Float equality as bits, 8 November 2026

`Float`'s `==` is bit identity and its ordering IEEE's totalOrder (runtime/rust/src/float.rs;
Type System round 6, ruling FB): `prism_rtl::Float`, `Floaty` and
`TyDef.floaty` deleted, the maps' and arrays' self-equality caches gone, an `Rc`'s `==`
comparing pointers first. A fresh boot of a VM of the same kind (Intel Xeon, 2 cores,
8 GB, rustc 1.97, jemalloc), nothing else running, load
under 0.3 at the start of each pair; this is not the machine or the boot of the sections
above, so compare only within this section. The before is the prismc at
`3a5ff1f` (the branch point, the seeds' fixed point), the after the fixed points at
`01935cb`.

- **The Rust prismc compiling itself to Rust** (`prismc compile prismc --prelude prelude
  --target rust`, `/usr/bin/time -v`, six runs each, interleaved; load 0.21 at the start):

  | Float | wall, s (median; runs) | peak RSS |
  |---|---|---|
  | IEEE `==`, `Floaty` (before) | 49.9 (48.7, 48.9, 49.5, 50.3, 50.4, 51.3) | 314.1 MB |
  | bits (after) | 39.6 (37.6, 38.9, 39.5, 39.7, 40.1, 40.2) | 314.3 MB |

  21% less wall time, the same memory; every after run is faster than every before run.
- **Where the time went** (the Rust pair, one run each under `perf record -e cpu-clock -F
  499`, self time): 24,695 samples before, 18,764 after. `floaty` itself held 419 (1.7%);
  the prelude's folds 1,681 against 897, `has_text` 207 against 67 and `Text`'s `==` 752
  against 411, which is what `floaty` ran per declared type (a fold over its fields and
  arguments, each step a frame and a job, and `HasText seen` over the types it had
  passed); the allocator and copying shrank with them. The pointer-first `Rc` and the
  maps' and arrays' shared nodes add nothing visible here: prismc compares few values that
  share structure. So the gain is the deleted pass, not the new comparisons.
- **The GUI table by phase** (`bench/gui/prism/check.sh prism-rust`, CPU
  milliseconds, one run each, the after only: the before prismc emits `f64` and partial
  derives, which the after runtime refuses, so no before driver builds on this tree;
  every check passing):

  | impl | op | propagate | build | patch | diff | perform | paint | total |
  |---|---|---|---|---|---|---|---|---|
  | prism-rust | create1k | 3.0 | 39.2 | 4.1 | 17.2 | 7.6 | 3.0 | 74.3 |
  | prism-rust | update10th | 1.2 | 3.1 | 1.7 | 1.0 | 0.8 | 0.6 | 8.5 |
  | prism-rust | select | 0.6 | 1.4 | 1.1 | 0.2 | 0.2 | 0.5 | 4.0 |
  | prism-rust | swap | 0.5 | 1.3 | 1.2 | 0.2 | 0.2 | 0.4 | 3.8 |
  | prism-rust | remove | 1.5 | 4.5 | 1.2 | 0.3 | 0.1 | 1.7 | 9.4 |
  | prism-rust | clear | 1.3 | 8.6 | 1.5 | 13.5 | 4.1 | 1.1 | 30.3 |

  The Rust run started at load 0.66, just after the profiles. A
  record of the after, not a pair; with no before on this boot, nothing here says whether
  the table moved.
- **The cost pairs** after (`tests/cost/run.sh`, cachegrind): Rust: a `let` read by ten
  2,392 M / 2,394 M; a builder over its last source 182.1 M / 183.0 M; a row per key
  changed 23.2 M / 56.9 M (0.87 M per extra edit); no kept row woken 1,228 M / 1,545 M
  (8.1 M per extra edit). All within their bars. Instruction counts do not drift with the machine, so
  they compare with "jemalloc (Rust)" above: each within 2.5% of its value there, the builder 1 to 2.5%
  lower.

## On Jimmy's Mac with agents building beside it (5 November 2026, and the GUI runs after it)

The sections from here on are the 5 November run at `aa500e5` and the GUI runs that
followed it on the same Mac, each naming the load it ran under.

### Building Prism: source to a runnable binary, from clean, seconds

| Subject | Rust: emit + build = total | binary |
|---|---|---|
| prismc (24,390 lines of Prism with the prelude; 103,850 of Rust) | 70.9 + 97.2 = 168.1 | 25.5 MB |
| chess (1,786 lines; 6,327 of Rust) | 1.3 + 13.1 = 14.4 | 4.5 MB |
| the table (1,538 lines; 5,745 of Rust) | 1.0 + 12.8 = 13.8 | 4.3 MB |

- cargo ran 12 jobs; the load average was 20 during the prismc builds.

### Running Prism once compiled: prismc as the workload

| Run | Rust wall | instructions | peak footprint |
|---|---|---|---|
| prismc compiling itself | 74.9 s | 724.2 G | 3,020 MB |
| compiling `sem-ch07-normalize` | 0.13 s | 1.38 G | 20 MB |

- **The memory is the array bug** (`claude/bugs/small-arrays-whole-chunks.md`): on
  branch `claude-arrays` the Rust `Map` port took
  the Rust wall time to about 57 s; the Rust `Array` is in progress.

### Running Prism once compiled: the GUI table

js-framework-benchmark's operations; the median of 10 runs after 2 warm-ups, one process
per operation, milliseconds; peak footprint where it was read. Prism runs under the
canvas World. The Cuis run was stopped before it finished.

| Operation | Prism Rust | GTK 3 (C) | Cuis (Cog VM) |
|---|---|---|---|
| create 1,000 rows | 532 (203 MB) | 277 (75 MB) | 5,000 (125 MB) |
| replace 1,000 rows | 1,484 (274 MB) | 327 (76 MB) | 11,591 (131 MB) |
| update every 10th row | 1,716 (235 MB) | 36 (76 MB) | 2.3 (125 MB) |
| select a row | 1,685 (226 MB) | 1.1 (76 MB) | no figure |
| swap two rows | 1,465 (235 MB) | 61 (79 MB) | 69 (120 MB) |
| remove a row | 1,449 (220 MB) | 6.7 (76 MB) | 6,018 (123 MB) |
| create 10,000 rows | 7,900 (1,209 MB) | 11,529 (282 MB) | not run |
| append 1,000 rows | 2,353 (277 MB) | 393 (98 MB) | not run |
| clear | 947 (197 MB) | 42 (75 MB) | not run |

What the table shows, none of it traced to a cause yet (each becomes a report under
`claude/bugs/` once it is):

- **Every small operation in Prism costs about what building the table costs** (GTK selects in 1.1 ms). The work per change is
  proportional to the table, not to the change.
- **Clear** is 947 ms in Rust.
- **The Cuis figures are not to be leaned on**: 5 s to create and 2.3 ms to update do not
  sit together, select printed nothing, and the run was cut.
- **A GUI run needs the display awake.** The first attempt hung in GTK for two and a half
  hours at 0% CPU when the display slept; `bench/gui/run.sh` should hold it awake itself.

### After the window fix

`bench/gui/run.sh prism-rust` with `RUNS=5 WARMUP=1`, on the same Mac with other
agents building (load average 7 to 26 before, about 8 after): before at `3fb9362`, after
at `2c6d8be` (branch `claude-incremental`: the window draws only what shows and holds its
document with `@Distinct`; the lowering caches multiply-read `let`s and a `for`'s builder
touches only the keys that changed; the arrays work of `claude-self-hosting` merged in).
Medians in ms, peak resident memory in MB; Prism only.

| Operation | Rust before | Rust after |
|---|---|---|
| create1k | 515 (289 MB) | 47 (218 MB) |
| replace1k | 1,428 (322 MB) | 54 (220 MB) |
| update10th | 1,642 (305 MB) | 40 (208 MB) |
| select | 1,651 (304 MB) | 38 (208 MB) |
| swap | 1,426 (304 MB) | 32 (208 MB) |
| remove | 1,413 (292 MB) | 36 (208 MB) |
| create10k | 29,046 (1,236 MB) | 349 (397 MB) |
| append1k | 2,287 (381 MB) | 56 (228 MB) |
| clear | 937 (288 MB) | 20 (218 MB) |

- **Fifteen to forty times faster on small changes, and create10k is linear again.**
- **A small change still costs most of a create1k**, so `bench/gui/prism/check.sh`'s second check still fails;
  GTK selects in 1.1 ms. What is left needs a design:
  `claude/bugs/window-relayout-every-change.md`.

### Instructions by phase, and the cheap runtime fixes

`bench/gui/prism/check.sh` now prints each timed step's instructions by phase,
through `prism_rtl`'s probe (`world::probe`):
the runtime marks propagation, the build/swap/leave walks, the patch pass and the
document diff inside a tick, and the canvas World marks performing the outputs, the
compact round and painting. Each thread's instructions between its marks go to the phase
it was in (macOS's `thread_selfcounts`); `other` is the rest of the process's (`proc_pid_rusage`), its
other threads. One run each, on Jimmy's Mac with agents building (load 8 to 16), at the
merged tree with the seeds rebuilt (`885c89d` merged), the fixes below in. Millions of
instructions:

| target | op | ticks | propagate | build | patch | diff | perform | paint | loop + other | total | ÷ create1k | paint ÷ create1k |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Rust | create1k | 7 | 47.3 | 152.0 | 194.2 | 20.3 | 13.3 | 157.9 | 4.5 | 589.6 | 1.000 | 0.268 |
| Rust | update10th | 8 | 23.8 | 107.7 | 185.1 | 3.5 | 3.0 | 155.2 | 2.1 | 480.4 | 0.815 | 0.263 |
| Rust | select | 8 | 5.4 | 104.9 | 184.5 | 2.5 | 2.2 | 154.9 | 2.2 | 456.7 | 0.775 | 0.263 |
| Rust | swap | 7 | 5.6 | 105.7 | 93.9 | 2.1 | 1.1 | 154.9 | 1.6 | 364.9 | 0.619 | 0.263 |
| Rust | remove | 7 | 5.2 | 88.5 | 183.9 | 2.2 | 1.2 | 155.5 | 1.2 | 437.7 | 0.742 | 0.264 |
| Rust | clear | 7 | 5.1 | 39.4 | 6.0 | 21.2 | 6.4 | 139.5 | 1.0 | 218.6 | 0.371 | 0.237 |

The compact round is under 0.05 M in every step and is left out.

- **Painting alone is a fifth to a quarter of create1k** (Rust 155 M of
  590 M), whatever the step: clearing the table paints as much as selecting a row. The
  canvas Worlds paint the whole 900 by 800 window (at the backing scale) once per settled
  state, so while they do, no small change can come under a tenth of create1k: the bar is
  out of reach by painting alone, by a factor of 2.6 in Rust (doubt PB).
- **The rest of a select is the build walks and the patch pass:** Rust spends 105 M in the
  builds and 185 M in the patch. Propagation, the diff and the World's
  perform are each under 3% of the step.
- **Swap's patch is half the others'** (94 M): one layout where the others lay out
  twice (`claude/bugs/window-relayout-every-change.md`, doubt MT: no new word, so no second
  layout).

**The cheap runtime fixes** (claude/incremental-host-document.md, "Without a route"): `==`
on a `Map` or an `Array` answers at once for a node both sides share once its values are
known to equal themselves, a NaN staying unequal; and a structural dynamic's equal snapshot
keeps the value it had. Before is the same tree with those two runtime changes reverted
(the probe in it). Instructions from `check.sh`, one run each; times from
`bench/gui/run.sh prism-rust`, `RUNS=5 WARMUP=1`, medians in ms (load 7 to 9
before, 12 to 17 after):

| op | Rust before | Rust after | Rust ratio | Rust ms before → after |
|---|---|---|---|---|
| create1k | 590.9 M | 589.6 M | 0.998 | 43.9 → 44.2 |
| update10th | 483.9 M | 480.4 M | 0.993 | 38.0 → 38.5 |
| select | 459.3 M | 456.7 M | 0.994 | 36.8 → 36.7 |
| swap | 367.8 M | 364.9 M | 0.992 | 29.8 → 29.4 |
| remove | 436.9 M | 437.7 M | 1.002 | 35.2 → 34.4 |
| clear | 217.1 M | 218.6 M | 1.007 | 17.9 → 17.8 |

(replace1k 52.8 → 52.1, create10k 345 → 348, append1k
55.1 → 55.4 ms; peak memory unchanged within 3 MB.)

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
in the printer, `Map::same` in the runtime; a host's rows over its document included).
Millions of instructions; the transcripts are unchanged.

| target | op | ticks | build before | build after | patch | paint | total before | total after | after ÷ create1k |
|---|---|---|---|---|---|---|---|---|---|
| Rust | create1k | 7 | 152.7 | 144.9 | 194.0 | 155.4 | 476.1 | 581.3 | 1.000 |
| Rust | update10th | 8 | 108.1 | 87.8 | 184.9 | 155.2 | 608.6 | 461.5 | 0.794 |
| Rust | select | 8 | 105.4 | 84.7 | 184.3 | 155.0 | 587.4 | 437.4 | 0.752 |
| Rust | swap | 7 | 105.9 | 85.5 | 93.7 | 154.9 | 497.7 | 345.5 | 0.594 |
| Rust | remove | 7 | 88.5 | 73.8 | 183.3 | 154.8 | 449.4 | 423.4 | 0.728 |

- **The build phase falls by a tenth to a fifth** (Rust select 105 M
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
| Rust | create1k | 7 → 8 | 145.0 → 246.8 | 193.6 → 28.2 | 155.3 | 580.9 → 646.6 | 1.000 |
| Rust | update10th | 8 → 9 | 88.3 → 11.7 | 184.9 → 12.8 | 155.1 | 461.9 → 193.5 | 0.299 |
| Rust | select | 8 → 9 | 85.6 → 4.5 | 184.0 → 11.9 | 155.0 | 436.1 → 176.9 | 0.274 |
| Rust | swap | 7 → 9 | 86.1 → 5.3 | 93.6 → 11.9 | 155.0 | 346.0 → 177.9 | 0.275 |
| Rust | remove | 7 → 8 | 73.9 → 29.0 | 183.2 → 10.9 | 154.9 | 423.5 → 202.9 | 0.314 |
| Rust | clear | 7 → 8 | 31.5 → 39.3 | 5.7 → 9.0 | 149.0 | 207.2 → 293.0 | 0.453 |

- **A small change is the paint and a little:** no row is laid out but the
  moved ones and no document of the table is built. Painting is now four fifths of a
  select.
- **Building the table costs more:** create1k's build is 247 M (Rust)
  against 145 M, a node per element of the window's layout and per measured
  label, and clear's diff and build grow with them (tearing those down).
- **A tick more per press** (eight to nine, seven to nine for swap): the window lays out
  a gathered `for` per level of the tree (Jimmy's story: every gathered `for` adds one).
- On the way: the layout no longer builds an array of (path, result) pairs per container
  per change (Rust select 260 M before that), a map of a map's entries is built
  at once, and a hosted builder gathers its host's outer
  only when its rows' keys changed.

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
| Rust | create1k | 8 | 248.9 | 29.0 | 32.9 → 42.1 | 100.8 → 7.0 | 631.0 → 540.0 | 1.000 |
| Rust | update10th | 9 | 12.0 | 12.9 | 33.6 → 16.1 | 4.4 → 18.2 | 77.0 → 75.1 | 0.139 |
| Rust | select | 9 | 4.6 | 12.1 | 32.6 → 8.8 | 0.3 → 2.3 | 55.1 → 33.3 | 0.062 |
| Rust | swap | 9 | 5.6 | 11.9 | 33.7 → 8.7 | 4.4 → 4.4 | 61.1 → 36.0 | 0.067 |
| Rust | remove | 8 | 29.3 | 10.9 | 33.6 → 28.6 | 4.4 → 5.1 | 84.4 → 80.3 | 0.149 |
| Rust | clear | 8 | 39.8 | 9.0 | 144.0 → 10.8 | 138.5 → 5.1 | 431.0 → 156.1 | 0.289 |

Wall time, `driver OP 20 3` run by hand (not `bench/gui/run.sh`), medians of 20 runs in ms
with the fastest and slowest, before → after:

| op | Rust |
|---|---|
| select | 14.9 (14.1–16.0) → 5.3 (3.7–7.4) |
| update10th | 17.6 (16.4–19.6) → 7.6 (6.5–8.9) |
| remove | 18.1 (17.0–19.9) → 10.1 (9.1–11.3) |
| create1k | 50.8 (40.1–60.5) → 43.9 (41.7–45.3) |

- **Most of the frame's cost was not painting but colour matching:** Core Animation converting every pixel of the frame from sRGB to the
  screen's colour space on the CPU, in this process, when the frame was committed. With the window's colour space set to sRGB, the
  frame's own, the conversion is gone (the window server matches colours as it
  composites). The same conversion is the Rust table's 100 to 140 M bursts in "loop" and
  "paint" (create1k and clear before): softbuffer's frame is committed with the run loop's
  transaction, which falls inside the timed step or after it from run to run, so the
  Rust before row is low in paint for most steps only because the conversion was counted
  elsewhere or not at all.
- **A select now paints 8.8 M (Rust):** two labels' boxes. In Rust about
  11 M of each frame is presenting it through softbuffer, whatever the damage: its macOS
  buffer is a new zeroed allocation per frame, the whole frame is copied in, and it
  becomes a new `CGImage` (worlds/canvas/rust/src/app.rs). Rust's update10th
  and select vary by a few M from run to run in "loop" for the same reason as before, the
  commit's timing.
- **A remove, a create and a clear damage most of the frame** (every row below the removed
  one moves; every row comes or goes): Rust 29 M, 42 M and 11 M,
  about what painting the whole frame costs, with the comparison's cost in it.
  The Prism window has no scroll pane; a scroll would move every row and cost as a remove.
- **A small change is now the program's own work.** What is left of the window's per-change
  cost is `claude/bugs/window-relayout-every-change.md`.
- **The wall times fall by four to five times on the small steps**: a select is
  5.3 ms in Rust (GTK's is about 1.1 ms in the table above). The instruction
  counts miss what the window server does with a frame after the flush; the clock here
  stops at the flush, as before.
