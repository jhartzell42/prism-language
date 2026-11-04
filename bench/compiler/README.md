# The compiler benchmark

*Written by Claude on the claude-self-hosting branch; subject to change.*

Rust against C++, each pipeline all one language: the Rust-built prismc emits Rust and
cargo builds it; the C++-built prismc emits C++ and make builds it. Never one binary
emitting the other target. Two numbers, for each build of prismc (Rust debug and release;
C++ at -O0, -O1, -O2, -O3):

- **The compile loop:** `prismc/` from source to a runnable binary, from a clean build
  directory: emitting (the installed fixed point of that language) plus building, the
  runtime library included. cargo uses its default parallelism; make runs with every core.
- **The compiled code's speed:** that binary compiling `prismc/` to its own language (wall
  time, peak memory, instructions retired), and compiling
  `tests/conformance/sem-ch07-normalize.prism` (the median of `NORM` runs).

```
make bootstrap bootstrap-cpp     # the two fixed points the emitting is done with
bench/compiler/run.sh            # every configuration; writes RESULTS.md
bench/compiler/run.sh cpp-O2 rust-release
```

Instructions are macOS's `time -l` "instructions retired", the processor's own count;
memory is its maximum resident set size. The GUI benchmark (`bench/README.md`) is the
other measure of compiled Prism's speed, through each target's canvas World.

- `run.sh`: builds, runs, writes `RESULTS.md`; the raw logs go to
  `target/bench/compiler/CONFIG/`.
- `RESULTS.md`: the last run's numbers.
