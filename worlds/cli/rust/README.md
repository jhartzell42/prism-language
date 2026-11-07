# The CLI World

*Written by Claude, 1 November 2026, on the `claude-self-hosting` branch. Subject to
change: implementers use their discretion.*

The World compiled programs run under at the command line, over `prism_rtl`'s protocol
(`claude/backend.md`): a `Program` built and ticked by `prism_rtl::drive`, everything
crossing as a `Pod`. It answers the `Fs`, `Process` and `Time` families of `prelude/`, the
prelude's `Prelude.Unwrap` and `Prelude.Exceptions`, and one tag of its own, `Fs.List`.

```rust
prism_world_cli::run::<MyProgram>(args)            // the real file system, at the working directory
prism_rtl::drive::<MyProgram>(&mut CliWorld::new(MemFs::default(), args), &[], patience)
```

## What it answers

- **`Fs.Contents(path)`**, `$~Contents`
  - Its first value is the file's text, or an `Error` of kind `Fs.NotFound`, `Fs.Denied`
    or `Fs.Failed`.
  - Whenever this World writes a file, it pushes the new contents to every
    `Fs.Contents` of that path.
- **`Fs.Output(path, text=$Text)`**, `$WriteState`
  - Its first value is `Writing`.
  - When the place is added and whenever its text changes, the World writes the file and
    pushes `Written`, or `WriteFailed err`, keyed by the element, into the next tick.
  - A write makes the parent directories, writes beside the target and renames, so a
    reader never sees half a file.
- **`Fs.File(path, requests=!FileRequest)`**, `!~Contents`
  - Each `Read` and each `Write text` is answered in the next tick, with `~ Contents(..)`
    or an `Error`.
- **`Fs.List(dir)`**, `$[Text]`, this crate's own tag (below)
- **`Process.Args`**, `[Text]`: the arguments, plain, once.
- **`Process.Env(name)`**, `?Text`: the variable's value, plain, once; `nil` when it is
  unset or not UTF-8.
- **`Process.Exit(code=!U8)`**
  - It ends the program with `code` after every request of its tick is performed, in
    document order, so a write fired beside the exit lands first.
  - The first exit of a tick wins.
- **`Time.After(wakes=!Wake)`**, `!Wake`
  - Each wake is answered with the same `Wake`, no sooner than its `after`, in a later
    tick. `@Later` is the zero-delay case (Semantics Appendix B).
  - Wakes do not hold each other up: the World keeps a list of due times and sleeps only
    when nothing else is waiting and no tick is owed.
- **`Prelude.Unwrap(value=!~a)`**, `!a`, and **`Prelude.Exceptions`**, `!Exception`
  - Each `~ x` is answered with `x` in the next tick.
  - The first `Error` poisons the innermost subtree with a path of its own that holds the
    place: a `rebuild` generation, a `for` row, or the root. Nothing there is answered
    again, and no request from there is performed.
  - The report, an `Exception`, goes a tick later to the nearest `Prelude.Exceptions`
    place outside the subtree (Semantics ch. 10, "Exceptions: a widget giving up").
  - `prelude.rs` holds the policy alone (`Prelude`), and a wrapper (`UnwrapWorld`) that
    puts it around a World answering no prelude tag. The canvas World over `prism_rtl`
    (`worlds/canvas/rust`) runs inside that wrapper.

An answer to a place that has left goes nowhere (Semantics ch. 9, "A late result is
dropped"), and so does one to a poisoned place.

## Exit codes

From Semantics ch. 9, "The CLI World's policy", and ch. 10:

- **The code `Process.Exit` asked for.**
- **0** when nothing can cause another tick: no answer or push waiting, no wake in
  flight, and `pending` not set.
- **1** for any of these, each reported on standard error:
  - a poisoned root;
  - a failed tick (`World::failed`'s default);
  - a run that exhausts the World's patience (`RUN_PATIENCE`, ten million ticks, under
    `run`).
- **2** for a program that fails to check. That is `prismc`'s to report before tick 0;
  the crate re-exports `EXIT_UNCHECKED` so every World spells it alike.

## `Fs.List`: a directory listing, for the compiler's sake

```prism
tag Fs.List of Element $[Text]:
    dir: Text
```

- **What it answers.**
  - Its first value is every `.prism` and `.prismtest` file under `dir`, at any depth.
  - Each path is `dir` as written joined to the file's path inside it, so it can go
    straight to `Fs.Contents`. With `dir` written `.`, paths have no prefix.
  - The list is sorted as text.
  - Entries whose name starts with `.` are skipped, and so are symbolic links to
    directories, so a listing always ends.
  - A missing or unreadable directory lists nothing: the answer is `$[Text]`, with no `~`
    to carry an `Error`.
  - When this World writes a file, it pushes the new listing to every live `Fs.List`
    whose listing moved.
- **Why it is here.** `prism run DIR` must find a directory's sources and tests, and the
  compiler is a Prism program, so it needs a listing from its World.
- **The books exclude it.** prism-reflex ch. 6 lists "No directory listing in `Fs`" in its
  negative space, which buys "a file system language small enough to answer on every
  World, the Test World included", and the Semantics seed's TD-W1 rules "no directory
  listing".
  `prelude/fs.prism` says the same ("There is no directory listing (TD-W1)").
- **First ruling (Claude's), for the compiler's sake:** the CLI World answers `Fs.List`,
  and the tag is declared by the program that uses it (the compiler), not in
  `prelude/fs.prism`. The Test World does not answer it, so the negative space still
  holds for every program that does not declare it.
  - *If otherwise:* the driver that calls the compiler lists the directory and passes the
    paths as `Process.Args`, and `Fs` keeps no listing anywhere.

## Picks

- **The ready state reads the file system when a place asks** (`Fs.Contents`,
  `Fs.List`). That is the ready state materialised lazily, not the "no I/O in a query"
  of Semantics ch. 9 to the letter, as in prism_rt's CLI World.
- **The World keeps its own copy of the document,** since `prism_rtl`'s `Request` carries
  a path and no tag. A request's tag is looked up in the document as the tick opened (which
  still holds a place torn down in the tick), and then among the tick's newborns.
- **A `Process.Exit` code that is not a `U8`** cannot pass the checker. If one arrives
  anyway, the World exits 1.
- **Unknown tags are passed over in silence**, as in prism_rt's CLI World. The root's row
  check (`FAMILIES`) is what keeps them out.

## Tests

`cargo test -p prism_world_cli`. `tests/cli.rs` holds programs written by hand in the
shape `claude/backend.md` gives the templates, as `runtime/rust/tests/counter.rs` does:

- reading a file through `Fs.File` and `Prelude.Unwrap`, writing it upper-cased through
  `Fs.Output`, and exiting 42 on `Written`, over the in-memory and the real file system;
- the same with the input missing: the root is poisoned, and the exit is 1;
- `Fs.List` on a temporary tree, and a write under a listed directory pushing the new
  listing;
- `@Later` answered one tick later, and a wake with a delay waiting for it;
- a run that exhausts its patience exits 1, and a program with nothing left to do exits 0.
