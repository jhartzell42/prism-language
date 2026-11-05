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
