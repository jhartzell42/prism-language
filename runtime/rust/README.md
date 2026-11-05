# runtime/rust: `prism_rtl`

The runtime library every Rust program `prismc` emits links. Emitted code is typed Rust,
one type per Prism type, with each widget's graph emitted from code templates; this crate
is the part that is not generated per block, written once and shared.

- **Modules:** numbers (`num`), persistent collections, `Pod` and paths, the transcript's
  printer and reader, the node primitives (`cells`: `Hold`, `Stamped`, `Cache`, `Port`,
  `Event`, `Dynamic`, `Defer`, `Job`), the tick (`cx`), the document and patch (`doc`),
  the zombie collector (`gc`), hosting (`host`), and the World protocol with the Test
  World (`world`). `src/lib.rs` describes each.
- **The contract** between this crate and the emitted code is `claude/backend.md`.
- **`Text`** (Semantics Appendix B, Text; round 4 TX) is UTF-8: an `Rc<str>` and a byte
  range of it, so a clone is a count and a `Cursor`'s `before` and `after` are views of
  the same buffer, constant. `len` is bytes; order and equality are bytewise.
  Concatenation copies both sides once. A `Cursor` is the text and a byte index on a
  character boundary; it steps one `char` either way and compares by its index. It
  shows, and leaves as plain data, as `Cursor(at=N)`; nothing reads one back. A `Text`
  is made only from a `str`, so it is always UTF-8; the CLI World answers a file that is
  not with `Fs.NotText` (doubt TXU in prismc/README.md).
- **Build and test:** `cargo test -p prism_rtl` from the repository root.
- **Its C++ twin** is `runtime/cpp`, module for module.
- **Regular expressions** (Semantics Appendix B, "Regular expressions"): `src/regex.rs`
  has `Regex`, `RegexReplacement`, the `Group` and `Match` records and the searches;
  `src/regex_dialect.rs` checks a pattern, its flags and a replacement against the
  dialect (`claude/regex-dialect.md`, the same procedure as the C++ runtime's) and spells
  the pattern for `regex-automata`'s meta engine, which never sees the pattern as
  written. A `Regex` is an `Rc` (a clone is a count); every matched text is a view of the
  searched text and every cursor one into it. The shared corpus
  `tests/regex/corpus.txt` is `tests/regex.rs`'s. The costs, against Appendix B's
  promises (`n` the text's bytes, `p` the pattern's expanded size, `k` the matches):
  - `Regex::new` (`?Regex`, and each literal once): the check is linear in the pattern;
    the library's compilation is linear in `p`. At the dialect's bound (10,000 atoms of
    `(?i)\pL`) it takes about 2 s and 1.3 GB, the NFA size limit being lifted so that
    every pattern in the dialect compiles.
  - `match_at` (`?Match`): one anchored search, `p` times the bytes it reads, plus one
    allocation for the captures and three counted views per group taking part.
  - `find` (`?Find`): one search, `p·n`.
  - `matches`, `split`, `replace` (`Matches`, `Split`, `Replace`): one search per match,
    `p·n·(k+1)` at worst (doubt QM); `replace` adds the length of the result and gives
    back the text itself, uncopied, when nothing matches.
- **Its crates:** `regex-automata` 0.4 with its defaults, which bring `regex-syntax`,
  `aho-corasick` and `memchr` (already in the lock file); `cargo fetch` gets them for an
  offline build.
