# To do, for Jimmy

*Written by Claude on the claude-self-hosting branch; subject to change. Jimmy's list, to
reform as he likes; Claude's own list is `claude/handoff.md`. Every path here is in the
tree as it stands.*

## Things to try

- **Bootstrap from nothing:** `make bootstrap` on a fresh clone (`bootstrap/bootstrap.sh`:
  the seed `bootstrap/prismc-rust.tar.xz`, cargo, a fixed point installed as
  `target/bootstrap/prismc`), `make bootstrap-cpp` for the C++ one, then `make test`
  (bootstrap/README.md).
- **Chess in both Worlds:** `./prism run examples/chess` (or `make chess`; Rust, the winit
  canvas World) and `./prism run --target cpp examples/chess` (C++, the Cocoa loop on a
  Mac, Xlib on Linux). `./prism test examples/chess` runs its test modules under the Test
  World (examples/chess/README.md).
- **The benchmarks:** `bench/building/run.sh` (how long Prism takes to build), then
  `bench/running/run.sh` (how fast it runs, prismc on itself and the GUI table), each
  writing a `RESULTS.md` beside itself; `bench/gui/run.sh` times the table alone
  (bench/README.md, bench/gui/README.md). `make -C bench/gui/prism` builds the table's two
  drivers and `bench/gui/prism/check.sh` prints what each operation costs by phase. The
  drivers each run one operation; to click through the table yourself, run its root under
  the canvas World the way the Makefile compiles it, `./prism run bench/gui/prism
  examples/chess/window` (unrun here).
- **Break the compiler on purpose:** edit a module under `prismc/` (`prismc/syntax/lexer/`,
  `prismc/syntax/parser/`, `prismc/check/`, `prismc/lower/`, `prismc/emit/rust/`,
  `prismc/emit/cpp/`), run `make bootstrap`, and watch the fixed point refuse or the suites
  fail; `tests/compiled/run.sh NAME` runs one program (`tests/compiled/run.sh chess`), and
  `--target cpp` the same through C++.
- **Write a new example** as a tree under `examples/` (claude/modules-style-pass.md): a
  directory with its root `app.prism`, a `test mod Tests` in `tests.prism` beside it and
  its transcript `tests.expected`, as `tests/conformance/counter/` is laid out; run it with
  `./prism test examples/NAME` on both targets, and add it to `tests/compiled/run.sh` to
  pin it.
- **Handles against inline variants under jemalloc** (doubt CH1, prismc/README.md): under
  glibc the C++ choice as a handle (one shared block per value, `7dc334d^`) compiled prismc
  about 9% faster than the inline `std::variant`; jemalloc makes allocation cheaper, so the
  gap may have moved. To measure: bootstrap the C++ prismc from `7dc334d^` with the
  jemalloc link, then six interleaved self-compiles against the tip's
  (claude/benchmarks-preliminary.md, "Choices as inline variants (C++)", for the method).
- **TODO: jemalloc for the Rust target** (runtime/rust/README.md, pick AL). The Rust programs
  run on the system allocator. jemalloc took the Rust prismc's self-compile from 69.9 s to
  51.6 s on the Linux VM (claude/benchmarks-preliminary.md, "jemalloc (Rust)"), but
  through `tikv-jemallocator`, which compiles jemalloc's C sources on every clean build
  (chess's clean Rust build 73 s → 128 s there). Linking the system's jemalloc, as the C++
  target does through `pkg-config`, would keep the speed without the build: the
  `tikv-jemalloc-sys` crate's system-library mode (or a few lines of `#[link]` and a
  `GlobalAlloc` over `mallocx`), the `#[global_allocator]` line back in the emitted
  `main.rs` (prismc/emit/rust/templates/mod.prism; commit `10211d6` has it), and the same
  hard error as C++ when jemalloc is missing.
- **TODO: recursion past the stack** (prismc/README.md, IB3 and pick BS). Bounded functions
  recurse on the machine stack (1 GB in Rust, 4 GB in C++), so a walk of a structure
  millions deep overflows it: Rust prints "thread has overflowed its stack" and aborts, C++
  segfaults. Unbounded functions are not affected (each call is a cycle point with a heap
  frame). Left as it is; a depth limit is the same mechanism as gas (a counter threaded
  through calls, in a register), which Jimmy plans to design.

## Making the code yours

Claude's commits carry the trailer `Co-Authored-By: Claude ...` (those from the cloud are
authored `Claude` too; those from the Mac are authored by you), so the trailer, not the
author, says which lines are Claude's:

```
git log --format=%H --grep='Co-Authored-By: Claude' > /tmp/claude-commits
git blame --line-porcelain FILE | grep -E '^[0-9a-f]{40} ' | cut -d' ' -f1 | grep -cFf /tmp/claude-commits
```

counts the lines of `FILE` that a Claude commit last touched. A path that keeps the tests
green at every step:

1. **Start where the tests are thickest:** `prismc/syntax/lexer/` and
   `prismc/syntax/parser/` (their test modules, `tests.prism` with `tests.expected`, pin
   them, and `tests/compiled/run.sh` runs them), then the checker, `prismc/check/`
   (`tests/checker/run.sh`: every corpus program clean, every `tests/checker/neg/` program
   its `.expected` diagnostic).
2. **Then lowering and the printers** (`prismc/lower/`, `prismc/emit/rust/`,
   `prismc/emit/cpp/`, the templates under each `emit/*/templates/`): the fixed point and
   both compiled suites (`tests/compiled/run.sh`, `--target cpp`) catch any change in
   behavior. Rewrite a function at a time; `make bootstrap` and `make bootstrap-cpp` after
   each batch.
3. **Then the runtimes and the Worlds** (`runtime/rust`, `runtime/cpp`, `worlds/cli`,
   `worlds/canvas`, each in both languages): their unit tests (`cargo test --workspace`,
   `make -C runtime/cpp test`, `make -C worlds/cli/cpp test`, `make -C worlds/canvas/cpp
   test`), then the compiled suites over them.
4. **Then the scripts and the prose:** `prism`, `Makefile`, `bootstrap/`,
   `tests/*/run.sh`, `bench/*/run.sh`, every README and `claude/`.
5. **Last, the seeds:** `make bootstrap-update` and `make bootstrap-cpp-update` from your
   own compiler, so `bootstrap/prismc-rust.tar.xz` and `bootstrap/prismc-cpp.tar.xz` are
   yours too.

## A shell World

Replace the `prism` shell script (and in time `bootstrap/bootstrap.sh`,
`bootstrap/bootstrap-cpp.sh`, `tests/*/run.sh`, `bench/*/run.sh`) with Prism: a World for
coordinating Unix processes, so that a Prism program reads like a shell script.

- Tags for running a process (argv, environment, working directory; its exit code,
  standard output and error as answers), pipes between processes, files, and timing.
- A program's holds and `for` rows as the script's state and its parallel jobs: a build
  of N modules is a `for` over them, each row a process.
- The shell World's own policy on failure (the `set -e` question), as a wrapper like
  `Prelude.Unwrap`.
- A `#!` line: `prism run` with a cache, so a script starts as fast as a shell's.
- Then `prism` itself as a Prism script under this World, compiled by prismc.

## A design round: owed ticks after the prompt check

Jimmy: "If anything use this to get rid of delay ticks." The checker refuses every prompt
cycle through a structural dynamic, a host's `$doc` and a `for`'s or `rebuild`'s
gathered result alike, without knowing which dynamics are structural. Claude's proposal,
for a round before any code:

- **The guarantee:** nothing that reads a structural dynamic feeds back into the
  propagation that produced it in the same tick.
- **What it allows:** after construction in tick n, a second propagation over only the
  readers of structural dynamics, in the same tick, from the snapshot taken at the end of
  construction. Readers see one consistent state (DV2 stays met), and tick n's patch
  holds their results, with no owed tick.
- **Where a tick is still owed:** a structural read that reaches construction again (a
  `rebuild` or `for` whose key or arm depends on `$doc`).
- **Cost:** the viewer's lag (VL) and chess's owed ticks per move shrink; the transcripts
  pinning today's owed ticks change; the runtimes gain a second phase per tick.
