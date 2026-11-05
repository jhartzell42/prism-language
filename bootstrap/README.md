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
- **The last run** (5 November 2026, Apple M2 Pro, clang++ `-O3`, 8 jobs, other work on
  the machine): the Rust prismc compiled `prismc/` to C++ in about 88 s; make built it in
  about 189 s wall; the C++ prismc compiled itself in about 75 s (181,483 lines of C++ in
  one unit per module, byte-equal to what it was built from); its Rust (109,349 lines) was
  byte-equal to the Rust prismc's.
