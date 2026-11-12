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

3. **Rebuild.** The emitted program's own sources (every `.rs`, or `.cpp` and `.hpp`,
   under its `src/`; not the runtime's) are touched and the same build runs again, so
   cargo or make compiles and links the program once more over the runtime library and
   the dependencies the clean build left: what an edit to a Prism program costs after its
   first build, since prismc emits every file afresh.

Subjects: `prismc/` itself (the compiler), `examples/basic-chess`, and the GUI table
(`bench/gui/prism/bench.prism` with the window library). `RESULTS.md` has, for each: the
emit seconds and the emitted lines a second (every file under the emitted `src/`, the
nested modules' `mod.rs` and `mod.cpp` included), the build seconds, their total, the
rebuild seconds, and the binary's size; its header names the machine (`bench/lib.sh`
reads it from `sysctl` on macOS, from `/proc` and `/etc/os-release` on Linux) and the
load. The binaries stay under `target/bench/building/` for `bench/running/run.sh`.

```
bench/building/run.sh                        # every subject and configuration
CONFIGS="rust-release" bench/building/run.sh prismc   # one side
```

`prism run` and `prism test` build C++ at -O3 too (`PRISM_OPT` overrides for debugging).
