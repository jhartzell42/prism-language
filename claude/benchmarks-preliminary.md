# Benchmarks, preliminary (5 November 2026)

*Written by Claude on the claude-self-hosting branch; subject to change.* One run of
`bench/building/run.sh` and most of one of `bench/running/run.sh`, at `aa500e5` (after the
`Text`, header, `Rc` and string work; before nested modules, the array fix and regex), on
Jimmy's Apple M2 Pro (Mac14,10, 12 cores, 16 GB, macOS 26.6.2) **with agents building
beside it** (load 4 to 20). Jimmy asked for numbers on a noisy machine; the run on a quiet
one replaces this file with the scripts' own `RESULTS.md`. Wall times here are rough;
instruction counts and peak memory are not. Each pipeline is all one language: the Rust
prismc emits Rust and cargo builds it (release, opt-level 3); the C++ prismc emits C++ and
make builds it (Apple clang 17, `-O3`).

## Building Prism: source to a runnable binary, from clean, seconds

| Subject | Rust: emit + build = total | binary | C++: emit + build = total | binary |
|---|---|---|---|---|
| prismc (24,390 lines of Prism with the prelude; 103,850 of Rust, 154,462 of C++) | 70.9 + 97.2 = 168.1 | 25.5 MB | 63.4 + 143.5 = 206.9 | 58.2 MB |
| chess (1,786 lines; 6,327 of Rust, 8,334 of C++) | 1.3 + 13.1 = 14.4 | 4.5 MB | 1.3 + 10.1 = 11.4 | 3.3 MB |
| the table (1,538 lines; 5,745 of Rust, 7,336 of C++) | 1.0 + 12.8 = 13.8 | 4.3 MB | 0.9 + 10.0 = 10.9 | 2.9 MB |

- cargo ran 12 jobs and make `-j12`; the load average was 20 during the prismc builds.
- **Small programs build faster in C++** (about 10 s against 13 s, most of either being the
  runtime and the World); **the compiler builds faster in Rust** (97 s against 144 s), and
  on CPU time by far (agent E's quieter measurement: 255 s against 1,003 s).

## Running Prism once compiled: prismc as the workload

| Run | Rust wall | instructions | peak footprint | C++ wall | instructions | peak footprint |
|---|---|---|---|---|---|---|
| prismc compiling itself | 74.9 s | 724.2 G | 3,020 MB | 65.5 s | 729.2 G | 1,039 MB |
| compiling `sem-ch07-normalize` | 0.13 s | 1.38 G | 20 MB | 0.14 s | 1.52 G | 15 MB |

- **Instructions are equal within 1% on the self-compile**; C++ is about 13% faster on the
  clock and uses a third of the memory.
- **The memory gap is the array bug** (`claude/bugs/small-arrays-whole-chunks.md`): on
  branch `claude-arrays` the C++ footprint is already 293 MB, and the Rust `Map` port took
  the Rust wall time to about 57 s; the Rust `Array` is in progress.

## Running Prism once compiled: the GUI table

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

## After the window fix

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

## Instructions by phase, and the cheap runtime fixes

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

## A builder over the source it last built from

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

## The host document as a map of dynamics

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
