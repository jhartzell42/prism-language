# Using `prism`

*Written by Claude on the claude-self-hosting branch, 9 November 2026; subject to change.
How the `prism` command works as the tree stands; `bin/prism` is the source of truth.*

## Getting the commands

- **With direnv** ([direnv.net](https://direnv.net), `brew install direnv`, then its
  hook in your shell's rc file): `cd` into the repository and run `direnv allow` once.
  The `.envrc` puts `bin/` on `PATH`, so `prism`, `chess` and `basic-chess` work from anywhere inside
  the repository.
- **Without it:** `bin/prism` from the root, or `./prism` (a symlink to it).
- **A compiler first.** `prism` runs an installed prismc. On a fresh clone, or after
  pulling a compiler change your installed one predates:

  ```
  cargo fetch                                   # once; every later build is offline
  bootstrap/bootstrap.sh --from-seed            # installs target/bootstrap/prismc
  ```

  `prism` uses `$PRISMC` when it is set, else the newest of `target/bootstrap/prismc` and
  `target/keep/prismc-*`.

## The commands

Each takes one or more sources. A source is a directory, a tree whose root is its
`app.prism`, `lib.prism` or `mod.prism` (claude/modules-style-pass.md), or a loose
`.prism` file. Paths are relative to where you are, and must be inside the repository.

- **`prism check [--world FAMS] [--unused] SRC...`** prints the checker's report (`no
  errors`, or each diagnostic with its line and fix) and exits nonzero on an error. It
  reads the test modules too. `--unused` adds a warning at each definition nothing uses.
  `--world` names the tag families the World answers, when the root's row does not
  decide it.
- **`prism test SRC...`** compiles the program with its test modules,
  builds it, and runs every test under the Test World. Each test's transcript must equal
  the `.expected` file of its module's name (`tests.expected` for `tests.prism`) byte for
  byte; a difference is printed as a diff.
- **`prism run [--world cli|canvas] [--print-world] SRC... [-- ARGS...]`**
  compiles and builds the program without its tests and runs its `@main` root. The World
  comes from the root module's `emit` row: `Gui.*` runs in a window (the canvas World),
  anything else on the terminal (the CLI World). `--world` overrides. A row with `Ui.*`
  and no `Gui.*` is refused, since no World answers `Ui` elements alone. `--print-world`
  prints the World it would use and stops. Everything after `--`, or after the first
  argument that is not a source, goes to the program.
- **`prism compile SRC... -o OUT`** writes the program to `OUT` and
  stops: a Rust crate over `runtime/rust`.

## Builds

- The crate goes to `target/prism/NAME` and is built
  with cargo, release, into `target/prism-build`.
- `NAME` is the first source's file or directory name, with anything but letters,
  digits and `_` made `_`.

## Environment

| Variable | Effect |
|---|---|
| `PRISMC` | the prismc binary to run, instead of the installed one |

## `chess` and `basic-chess`

`chess` is `prism run examples/chess`: chess the document way, the hot seat in a window
(claude/document-chess.md). `basic-chess` is `prism run examples/basic-chess`: the basic
chess game in a window. Options go to `prism run`, so `basic-chess --print-world` says `canvas`.

## Examples

```
prism check examples/basic-chess          # the checker's report on the basic chess tree
prism test examples/basic-chess           # basic chess's rules and board tests, Rust
prism run bench/gui/prism                 # the GUI benchmark table in a window
prism compile examples/basic-chess -o /tmp/chess-crate   # the Rust crate, to read
```
