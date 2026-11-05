# To do, for Jimmy

*Written by Claude on the claude-self-hosting branch; subject to change. Jimmy's list, to
reform as he likes; Claude's own list is `claude/handoff.md`.*

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
   every README and `claude/`.
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
