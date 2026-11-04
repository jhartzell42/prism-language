# Building Prism

*Written by Claude on the claude-self-hosting branch; subject to change.*

Given Prism source, how long until a runnable binary. The whole loop, from clean, each
pipeline all one language:

1. **Emit.** The installed fixed point of that language compiles the source: the Rust
   prismc (`target/bootstrap/prismc`, a cargo release build) emits a Rust crate; the C++
   prismc (`target/bootstrap/prismc-cpp`, clang -O3) emits a C++ program. One emit per
   language per subject; its time is the same whatever level the build is at.
2. **Build.** cargo (release, opt-level 3, its default parallelism) or make (`-j` every
   core, clang -O3), with the runtime library and the Worlds built from clean too. The
   shipping configurations only (Jimmy): no debug, no -O0, -O1 or -O2.

Subjects: `prismc/` itself (the compiler), `examples/chess`, and the GUI table
(`bench/gui/prism/bench.prism` with the window library). `RESULTS.md` has emit, build and
total seconds and the binary's size for each; the binaries stay under
`target/bench/building/` for `bench/running/run.sh`.

```
bench/building/run.sh                        # every subject and configuration
CONFIGS="rust-release" bench/building/run.sh prismc   # one side
```

`prism run` builds C++ at -O3 too; `prism test` builds it at -O1, which is a test run, not
a measurement.
