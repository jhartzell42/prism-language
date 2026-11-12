# Running Prism once compiled: results

*Written by bench/running/run.sh on 2026-11-08 11:29 EST; regenerate, do not edit.*

- **Machine:** Intel(R) Xeon(R) Processor, 2 cores, 8 GB, Ubuntu 24.04.5 LTS, Linux 6.18.44-fc-v310; load 1.00 1.03 1.30
- **prism-language:** 83183b7 (with uncommitted changes)
- **The binaries:** bench/building/run.sh's (bench/building/RESULTS.md says how each was built)

## prismc as the workload

Each binary is a prismc built at that configuration, compiling to its own language.
Instructions are n/a: no counters on this machine (perf (/usr/lib/linux-tools-6.8.0-149/perf) reports the instructions counter <not supported>: this machine exposes no hardware counters), and valgrind is too slow for
runs this long. To rerun on a machine with counters. The seconds and the memory stand;
memory is the peak resident set, MB (`/usr/bin/time -v`). normalize is `tests/conformance/sem-ch07-normalize`, the median of 5 runs.

| Configuration | prismc/: s | MB | G instr | normalize: s | MB | G instr |
|---|---|---|---|---|---|---|
| rust-release | 37.8 | 307 | n/a (no counters on this machine) | 0.1 | 23 | n/a (no counters on this machine) |

## The GUI table as the workload

bench/gui/RESULTS.md's timings (prism-rust is the table
compiled to Rust as a release build; GTK beside them for scale):

| Operation | prism-rust | gtk |
|---|---|---|
| create1k | 49.3 (46.5–59.9) | 382.9 (359.3–521.0) |
| replace1k | 85.3 (69.9–114.2) | 469.8 (439.2–587.3) |
| update10th | 9.7 (7.9–11.3) | 48.3 (43.1–80.2) |
| select | 7.4 (4.4–10.5) | 5.9 (5.1–7.8) |
| swap | 7.0 (5.2–9.4) | 120.0 (95.3–157.8) |
| remove | 11.9 (8.2–14.5) | 31.5 (26.0–36.8) |
| create10k | 626.1 (567.2–683.0) | 37146.3 (34290.2–44179.8) |
| append1k | 53.1 (49.9–79.1) | 1033.2 (941.2–1487.1) |
| clear | 23.6 (19.8–27.5) | 93.8 (83.3–106.4) |

