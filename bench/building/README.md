# Building Prism

*Written by Claude on the claude-self-hosting branch; subject to change.*

Given Prism source, how long until a runnable binary. The whole loop, from clean, each
pipeline all one language:

1. **Emit.** The installed fixed point of that language compiles the source: the Rust
   prismc (`target/bootstrap/prismc`, a cargo release build) emits a Rust crate; the C++
   prismc (`target/bootstrap/prismc-cpp`, clang -O2) emits a C++ program. One emit per
   language per subject; its time is the same whatever level the build is at.
2. **Build.** cargo (debug, or release at opt-level 3; its default parallelism) or make
   (`-j` every core, clang, at -O0, -O1, -O2 or -O3), with the runtime library and the
   Worlds built from clean too.

Subjects: `prismc/` itself (the compiler), `examples/chess`, and the GUI table
(`bench/gui/prism/bench.prism` with the window library). `RESULTS.md` has emit, build and
total seconds and the binary's size for each; the binaries stay under
`target/bench/building/` for `bench/running/run.sh`.

```
bench/building/run.sh                        # every subject and configuration
CONFIGS="rust-release cpp-O2" bench/building/run.sh prismc
```

`cpp-O1` is what `prism test` builds C++ at, `cpp-O2` what `prism run` does.
