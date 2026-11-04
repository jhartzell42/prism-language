# To do, for Jimmy

*Written by Claude on the claude-self-hosting branch; subject to change. Jimmy's list, to
reform as he likes; Claude's own list is `docs/handoff.md`.*

## Things to try

- **Bootstrap from nothing:** `bootstrap/bootstrap.sh` on a fresh clone (the seed, cargo,
  a fixed point), then `make test`.
- **Chess in both Worlds:** `./prism run examples/chess` (Rust) and
  `./prism run --target cpp examples/chess` (C++, the Cocoa loop on a Mac).
- **The benchmark:** `bench/run.sh`, then read `bench/RESULTS.md`; click through the
  table app yourself (bench/README.md says how to open each version by hand).
- **Break the compiler on purpose:** edit `prismc/`, run `make bootstrap`, and watch the
  fixed point refuse or the suites fail; `tests/compiled/run.sh NAME` runs one program.
- **Write a new example** under `examples/` with a `.prismtest` beside it, and run it
  with `./prism test` on both targets.

## Making the code yours

Everything Claude wrote blames as "Jimmy's Claude", so `git blame` shows what is left.
A path that keeps the tests green at every step:

1. **Start where the tests are thickest:** `prismc/lexer.prism` and `prismc/parser.prism`
   (lexer-checks and parser-checks pin them), then the checker (`tests/checker`).
2. **Then lowering and the printers** (`prismc/lower*.prism`, `emit.prism`,
   `emit_cpp.prism`, `templates/`): the fixed point and both compiled suites
   (`tests/compiled/run.sh`, `--target cpp`) catch any change in behavior. Rewrite a
   function at a time; `make bootstrap` after each batch.
3. **Then the runtimes and the Worlds** (`runtime/`, `worlds/`): their unit tests, then
   the compiled suites over them.
4. **Then the scripts and the docs:** `prism`, `bootstrap/`, `tests/*/run.sh`, `bench/`,
   every README and `docs/`.
5. **Last, the seeds:** `bootstrap/bootstrap.sh --update` and `make bootstrap-cpp-update`
   from your own compiler, so the archives are yours too.

`git blame --line-porcelain FILE | grep -c "^author Jimmy's Claude"` counts what is left
in a file.

## A shell World

Replace the `prism` shell script (and in time `bootstrap.sh`, `tests/*/run.sh`,
`bench/run.sh`) with Prism: a World for coordinating Unix processes, so that a Prism
program reads like a shell script.

- Tags for running a process (argv, environment, working directory; its exit code,
  standard output and error as answers), pipes between processes, files, and timing.
- A program's holds and `for` rows as the script's state and its parallel jobs: a build
  of N modules is a `for` over them, each row a process.
- The shell World's own policy on failure (the `set -e` question), as a wrapper like
  `Prelude.Unwrap`.
- A `#!` line: `prism run` with a cache, so a script starts as fast as a shell's.
- Then `prism` itself as a Prism script under this World, compiled by prismc.
