# The CLI World in C++

*Written by Claude, 4 November 2026, on the `claude-self-hosting` branch. Subject to
change: implementers use their discretion.*

The C++ form of `worlds/cli/rust` (the Rust crate `prism_world_cli`): the World a program
compiled to C++ runs under at the command line, over `runtime/cpp`'s protocol. It answers
what the Rust one answers, the same way: `Fs.Contents`, `Fs.Output`, `Fs.File`, `Fs.List`,
`Process.Args`, `Process.Exit`, `Time.After`, and the prelude's `Prelude.Unwrap` and
`Prelude.Exceptions` (worlds/cli/rust/README.md, "What it answers", holds word for word).

```cpp
prism_world_cli::run<MyProgram>(args)            // the real file system, at the working directory
prism_world_cli::CliWorld world(std::make_unique<MemFs>(), args);
prism_rtl::drive<MyProgram>(world, {}, patience)
```

- **`include/prism_world_cli/cli.hpp`**, **`src/cli.cpp`**: `FileSystem` (`RealFs`,
  `MemFs`), `Prelude` (the unwraps, the supervisors, the poisoned subtrees), `CliWorld`.
  A program compiled to C++ builds `src/cli.cpp` beside its own sources (its `Makefile`)
  and runs under it when its root is a `@main` (prelude `run_cli`).
- **`tests/test_cli.cpp`**: the Rust crate's `tests/cli.rs` and the unit tests of its
  `fs.rs` and `prelude.rs`, test for test (13), over programs written by hand in the
  shape the templates give.

```
make -C worlds/cli/cpp test     # g++, clang++, then ASan+UBSan with g++
make -C worlds/cli/cpp check    # once, with $(CXX)
```
