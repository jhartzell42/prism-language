# Running Prism once compiled: results

*Written by bench/running/run.sh on 2026-11-06 19:03 EST; regenerate, do not edit.*

- **Machine:** Intel(R) Xeon(R) Processor, 2 cores, 8 GB, Ubuntu 24.04.5 LTS, Linux 6.18.44-fc-v260; load 1.05 1.01 1.08
- **prism-language:** d156742 (with uncommitted changes)
- **The binaries:** bench/building/run.sh's (bench/building/RESULTS.md says how each was built)

## prismc as the workload

Each binary is a prismc built at that configuration, compiling to its own language.
Instructions are n/a: no counters on this machine (perf (/usr/lib/linux-tools-6.8.0-149/perf) reports the instructions counter <not supported>: this machine exposes no hardware counters), and valgrind is too slow for
runs this long. To rerun on a machine with counters. The seconds and the memory stand;
memory is the peak resident set, MB (`/usr/bin/time -v`). normalize is `tests/conformance/sem-ch07-normalize`, the median of 5 runs.

| Configuration | prismc/: s | MB | G instr | normalize: s | MB | G instr |
|---|---|---|---|---|---|---|
| rust-release | 54.6 | 301 | n/a (no counters on this machine) | 0.1 | 22 | n/a (no counters on this machine) |
| cpp-O3 | 78.7 | 420 | n/a (no counters on this machine) | 0.2 | 36 | n/a (no counters on this machine) |

## The GUI table as the workload

bench/gui/RESULTS.md's timings (prism-cpp is the table compiled to C++ at -O3, prism-rust
compiled to Rust as a release build; GTK beside them for scale):

| Operation | prism-cpp | prism-rust | gtk |
|---|---|---|---|
| create1k | 76.5 (72.0–81.6) | 58.6 (52.5–60.6) | 328.5 (313.2–429.1) |
| replace1k | 128.0 (117.7–142.5) | 94.8 (82.5–133.5) | 383.5 (376.9–441.4) |
| update10th | 10.2 (8.9–11.1) | 7.6 (6.4–8.9) | 37.1 (36.6–37.4) |
| select | 7.1 (5.9–8.0) | 5.3 (4.1–6.4) | 4.1 (4.0–4.4) |
| swap | 6.8 (6.0–7.4) | 5.2 (4.2–6.0) | 83.1 (82.8–83.4) |
| remove | 15.6 (14.4–16.9) | 10.2 (8.9–11.1) | 16.9 (16.8–20.4) |
| create10k | 1200.5 (1139.1–1511.0) | 1065.0 (829.9–1196.7) | 26976.3 (26384.4–28112.1) |
| append1k | 78.2 (74.1–84.8) | 58.9 (52.7–63.5) | 807.0 (791.6–823.5) |
| clear | 48.1 (44.6–56.9) | 36.6 (33.0–39.8) | 65.4 (64.1–65.7) |

