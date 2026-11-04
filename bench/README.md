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
  (C++ and Rust), C with GTK 3, Cuis Smalltalk on the Cog VM.

```
make bootstrap bootstrap-cpp     # the two fixed points that do the emitting
bench/building/run.sh            # builds every configuration; writes building/RESULTS.md
bench/running/run.sh             # uses those binaries; runs gui/; writes running/RESULTS.md
```

Run them with nothing else running; each `RESULTS.md` records the machine, every
version and the load average.
