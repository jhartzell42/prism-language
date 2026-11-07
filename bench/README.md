# Benchmarks

*Written by Claude on the claude-self-hosting branch; subject to change.*

Two questions, Jimmy's, each with its own benchmark, and the GUI table they share. Every
Prism pipeline is all one language: the Rust-built prismc emits Rust and cargo builds it;
the C++-built prismc emits C++ and make builds it. Nothing mixes the two.

- **`running/`: how fast Prism runs once compiled.** The key number. Workloads: prismc
  itself (compiling `prismc/`, compiling `sem-ch07-normalize`) at every configuration,
  with wall time, peak memory and instructions retired; and the GUI table under each
  target's canvas World.
- **`building/`: how long Prism takes to build.** From Prism source to a runnable binary,
  from clean: prismc's emit plus cargo's or make's build, for `prismc/`, chess and the
  table, at cargo release against clang -O3 (Jimmy: the shipping configurations only).
- **`gui/`: the table**, js-framework-benchmark's operations, written four ways: Prism
  (C++ and Rust), C with GTK 3, Cuis Smalltalk on the Cog VM. The scripted runs leave
  Cuis out (Jimmy: its creates take minutes a run); `bench/gui/run.sh cuis` still runs it.

```
make bootstrap bootstrap-cpp     # the two fixed points that do the emitting
bench/building/run.sh            # builds every configuration; writes building/RESULTS.md
bench/running/run.sh             # uses those binaries; runs gui/; writes running/RESULTS.md
```

Run them with nothing else running; each `RESULTS.md` records the machine, every
version and the load average.

## Measuring on each platform

`lib.sh`, sourced by every script here and by `tests/cost/run.sh`, chooses the platform's
tools by `uname -s`, so the scripts read alike and the macOS run is untouched:

| | macOS | Linux |
|---|---|---|
| machine, load | `sysctl`, `sw_vers` | `/proc/cpuinfo`, `/proc/meminfo`, `/etc/os-release`, `/proc/loadavg` |
| peak memory | `/usr/bin/time -l` (bytes) | GNU `/usr/bin/time -v` (kbytes) |
| instructions, long runs (`running/`) | `time -l`'s "instructions retired" | `perf stat -e instructions`; with no hardware counters, "n/a" and the reason |
| instructions, small runs (`tests/cost/`) | the same | `perf stat`, else valgrind's cachegrind ("I refs", exact, slow); the output names the counter |
| instructions inside the GUI drivers | `proc_pid_rusage`, `thread_selfcounts` | perf_event per process and per thread; else CPU nanoseconds, with `unit=ns` on every line |
| a display for `gui/` | the desktop | `DISPLAY`, else `xvfb-run -a` around the whole script |

A missing tool is a hard error that names it; no measurement is skipped and passed off as
a number. A machine with no hardware counters (most VMs) is said so in every result.
