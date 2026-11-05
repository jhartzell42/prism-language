# An absolute `-o` points the Rust crate at `/runtime/rust`

*Written by Claude on the claude-self-hosting branch, 5 November 2026; subject to
change.*

## The scenario

On the cloud box, from a scratch directory, with the repository at
`/home/claude/prism-language`:

```
target/bootstrap/prismc compile /home/claude/prism-language/prismc \
    --prelude /home/claude/prism-language/prelude --name prismc \
    -o /home/claude/prism-language/target/bench/r2r
```

The emitted `Cargo.toml` says:

```toml
prism_rtl = { path = "../../../../../../runtime/rust" }
prism_world_cli = { path = "../../../../../../worlds/cli/rust" }
```

From `target/bench/r2r` that is `/runtime/rust`, and cargo fails: "failed to read
`/runtime/rust/Cargo.toml`". prismc exits 0 and says nothing. The same compile with
`--target cpp` builds, because the C++ Makefile takes `PRISM_ROOT` from the environment
first, and the run had it set.

## The cause

`RuntimeFor` (`prismc/front.prism`) is the default for `--runtime`. It puts one `..` for
each segment of OUT:

```prism
fn RuntimeFor out:
    let parts = Filter (Split out, "/"), \|p| p != "" and p != "."
    if (Len parts) == 0: "." else: Join (Each parts, \|ignore| ".."), "/"
```

That is right only when OUT is a relative path from the repository root, which is how
`./prism` and the bootstraps call it (the comment above `@CompileMain` says so: "so OUT
is under the repository root"). It is silently wrong in two cases:

- **an absolute OUT,** where it climbs to `/`, the case above;
- **a relative OUT from any other directory,** where it lands on that directory's
  ancestors, not the repository's.

`./prism` hides the first case by passing `--runtime` itself for an absolute output.

## The fix

- **Same as C++:** the default for `--runtime` is `PRISM_ROOT` when it is set, as
  `templates/cpp.prism`'s Makefile already does (`PRISM_ROOT ?= ...`). The two targets
  then find the runtime the same way.
- **No silent fallback:** with no `--runtime` and no `PRISM_ROOT`, keep `RuntimeFor`
  only for a relative OUT, and refuse an absolute OUT: "an absolute -o needs --runtime
  PATH or PRISM_ROOT". The relative-from-elsewhere case can't be detected without a
  file-system check. Better still: prismc checks that `{runtime}/runtime/rust/Cargo.toml`
  exists, through `Fs`, and refuses with the path it tried.
- **Tests first,** in `tests/driver` (the driver test already runs prismc as a process):
  - an absolute `-o` with `PRISM_ROOT` set: the crate builds;
  - an absolute `-o` with neither: refused, naming `--runtime`;
  - the bootstraps and `./prism` unchanged.
