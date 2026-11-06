# Running Prism once compiled: results

*Written by bench/running/run.sh on 2026-11-06 18:33 EST; regenerate, do not edit.*

- **Machine:** Intel(R) Xeon(R) Processor, 2 cores, 8 GB, Ubuntu 24.04.5 LTS, Linux 6.18.44-fc-v260; load 1.01 1.47 1.78
- **prism-language:** b8b7c44
- **The binaries:** bench/building/run.sh's (bench/building/RESULTS.md says how each was built)

## prismc as the workload

Each binary is a prismc built at that configuration, compiling to its own language.
Instructions are n/a: no counters on this machine (perf (/usr/lib/linux-tools-6.8.0-149/perf) reports the instructions counter <not supported>: this machine exposes no hardware counters), and valgrind is too slow for
runs this long. To rerun on a machine with counters. The seconds and the memory stand;
memory is the peak resident set, MB (`/usr/bin/time -v`). normalize is `tests/conformance/sem-ch07-normalize`, the median of 5 runs.

| Configuration | prismc/: s | MB | G instr | normalize: s | MB | G instr |
|---|---|---|---|---|---|---|
| rust-release | 55.0 | 301 | n/a (no counters on this machine) | 0.1 | 22 | n/a (no counters on this machine) |
| cpp-O3 | 79.2 | 420 | n/a (no counters on this machine) | 0.2 | 36 | n/a (no counters on this machine) |

