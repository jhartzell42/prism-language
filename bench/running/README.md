# Running Prism once compiled

*Written by Claude on the claude-self-hosting branch; subject to change.*

How fast compiled Prism code runs, all-Rust against all-C++: the key number. Two workloads.

- **prismc itself.** Each binary `bench/building/run.sh` made (a prismc at cargo release, and
  one at clang -O3) compiles `prismc/` to its own language and compiles
  `tests/conformance/sem-ch07-normalize` (the median of `NORM` runs). For each:
  wall time, peak resident memory, and instructions retired (macOS's `time -l`, the
  processor's count, which does not move with the machine's load as time does).
- **The GUI table** (`bench/gui/`): js-framework-benchmark's operations on the same table
  compiled to C++ (-O3) and to Rust (release), under each target's own canvas World,
  timed from the press to the frame on the screen; GTK beside them for scale.

```
bench/building/run.sh            # first: the binaries
bench/running/run.sh             # then this; writes RESULTS.md
CONFIGS="rust-release" GUI=0 bench/running/run.sh
```

A caveat on the self-compile: the C++ prismc emits about half again as many lines as the
Rust one for the same source, so its job is larger; normalize and the table are the same
work on both sides.
