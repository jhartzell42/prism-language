# bootstrap: building prismc from nothing but Rust

`prismc` is written in Prism, so building it needs a working `prismc`. The repository
keeps no binary; it keeps one archive.

- **`prismc-rust.tar.xz`** is the seed: the Rust crate `prismc` emitted for itself at its
  last fixed point (its `src/` and `Cargo.toml`), compressed. It is the only compiled
  code in the repository, kept apart from the source.
- **`bootstrap.sh`** starts from `$PRISMC`, else the installed `target/bootstrap/prismc`,
  else the seed unpacked and built with cargo (after a `cargo fetch` of its crates, the
  one step that needs the network; every build after it is offline). That compiler
  compiles `prismc/`; cargo
  builds the result; the new binary compiles `prismc/` again; the two emitted crates must
  be byte-equal (the fixed point). It installs the result as `target/bootstrap/prismc`,
  the binary `./prism` uses.
- **`bootstrap.sh --update`** also rewrites the seed from the new fixed point, after a
  change to the compiler.
- **`--from-seed`** (either script, with or without `--update`) starts from the committed
  seed, ignoring `$PRISMC` and the installed compiler. An installed compiler is older than
  the tree whenever the source's syntax moved past it (nested `mod` lines, regex
  literals): when the installed one fails to compile `prismc/`, the bootstrap stops with
  one line, "the installed target/bootstrap/prismc cannot compile this tree (it is older
  than the source's syntax); re-run with --from-seed", the compiler's diagnostics kept in
  `target/bootstrap/gen1-compile.log` (`cpp-gen1-compile.log`); it never retries from the
  seed on its own. **`--show-start`** only says which compiler it would start from.
  **`test-start.sh`** checks all of it under a scratch working directory
  (`PRISM_BOOTSTRAP`), with stand-in compilers. `make bootstrap` and `make bootstrap-cpp`
  are unchanged.
- **A seed is packed with no macOS extended attributes** (COPYFILE_DISABLE=1, and
  `--no-xattrs` where the tar takes it), so unpacking it on Linux prints no warnings.
- **`bootstrap.sh --seed-at DIR`** and **`bootstrap-cpp.sh --seed-at DIR`** only unpack
  and build a seed in DIR. **`test-seed.sh`** builds both three levels under the root in
  a directory that is a symlink out of the repository, as `target/` is in every worktree
  here, and checks a file with each: cargo is given the seed's crate by its path from
  the root (never run inside it, where `../../../runtime/rust` would resolve from the
  physical directory), and the C++ seed's Makefile takes `PRISM_ROOT`.
- **From the repository root:** `make bootstrap`, `make bootstrap-update`.

## The C++ target

The same bootstrap runs through C++ (`prismc compile --target cpp`, prismc/README.md,
"The C++ target"), with one check more.

- **`prismc-cpp.tar.xz`** is the C++ seed: the C++ program `prismc` emitted for itself at
  its last C++ fixed point (its `src/` and `Makefile`), compressed. It builds with `make`
  and a C++20 compiler (g++ 13 or clang++ 18) over `runtime/cpp` and `worlds/cli/cpp`.
- **`bootstrap-cpp.sh`** starts from `$PRISMC`, else the Rust bootstrap's
  `target/bootstrap/prismc`, else the C++ seed unpacked and built with make. That
  compiler compiles `prismc/` to C++ (`target/bootstrap/cpp-gen1`); make builds it; the
  new binary compiles `prismc/` to C++ again; the two programs must be byte-equal (the
  C++ fixed point). The C++ binary at the fixed point then compiles `prismc/` to Rust,
  which must be byte-equal to what `target/bootstrap/prismc` emits: one model, two
  printers, and either compiler gives the same output for either target. It installs
  the binary as `target/bootstrap/prismc-cpp`.
- **`bootstrap-cpp.sh --update`** also rewrites the C++ seed.
- **`CXX`, `OPT`, `JOBS`** choose the compiler (g++), its optimization (`-O3`) and make's
  jobs (1: the largest file, `lower.cpp`, needs most of a small machine's memory).
- **From the repository root:** `make bootstrap-cpp`, `make bootstrap-cpp-update`.
- **The last run** (5 November 2026, Apple M2 Pro, clang++ `-O3`, 6 jobs, other agents'
  builds on the machine): the Rust prismc compiled `prismc/` to C++ in 66 s; make built
  it in 168 s wall; the C++ prismc compiled itself in 83 s (185,385 lines of C++ in 63
  units, pick SF, byte-equal to what it was built from); its Rust (113,073 lines) was
  byte-equal to the Rust prismc's. On Linux (6 November 2026, a cloud VM with 2 cores and
  7 GB, Ubuntu 24.04, clang++ 18 `-O3`, 2 jobs, nothing else running; 8 November, the seeds' fixed point, jemalloc on both targets): the
  Rust bootstrap from the 7 November prismc compiled gen2 in 70 s, cargo built it in 149 s,
  and gen3 in 47 s (116,536 lines of Rust, gen2 and gen3 byte-equal); the Rust prismc
  compiled `prismc/` to C++ in 47 s; make built it in 1,129 s wall; the C++ prismc compiled
  itself in 82 s (190,255 lines of C++, byte-equal); its Rust was byte-equal to the Rust
  prismc's.
