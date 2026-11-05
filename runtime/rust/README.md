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
- **`Map`** (`src/collections/map.rs`) is the C++ runtime's persistent AVL tree,
  ported: one counted node per entry, holding its height and the entries under it; an
  update goes through `Rc::make_mut` on the path to its key, so a node this map alone
  holds changes in place and a shared one is copied first. An empty map allocates
  nothing. It has `im_rc::OrdMap`'s methods that emitted code and the runtime call, with
  one difference: `a.union(b)` keeps `b`'s value on a shared key (as `extend` does),
  where `im_rc` kept the larger map's, so the prelude's `Union a, b` (`b.union(a)`) now
  has `a` winning, as Semantics Appendix B and the C++ prelude say. No `unsafe`.
- **`Array`** is `im_rc::Vector`, an RRB tree of 64-element chunks: a vector that
  outgrows the few elements held inline takes a whole chunk, however small
  (claude/bugs/small-arrays-whole-chunks.md, open for Rust).
- **The costs against Semantics Appendix B**, `n` the size of the collection and `m` of
  the second:
  - `Map`: `Insert`, `Remove`, `?Get` and indexing, `?First`: log, kept (the height is at
    most about 1.44 log n and an update copies one path at most); `Size`: constant, kept;
    `Keys`, `Values`: linear in key order, kept; `Map` of pairs: `n log n`, the later
    winning, kept; `Union`: `m log (n + m)`, kept (the smaller map's entries go into the
    larger).
  - `Array` (`im_rc`'s promises): `Len` constant; indexing log (base 64); `Append` log;
    `Push` amortised constant, on a shared vector too (it copies at most a chunk and
    one path); `Each`, `Fold`, `Sum` linear. All kept.
  - `tests/arrays.rs` and `tests/maps.rs` check the behaviour at the sizes around a
    chunk's edges and the storage each holds (`tests/storage` counts the test binary's
    allocations); the `Array` storage test is ignored while the Rust half of the bug is
    open.
- **Build and test:** `cargo test -p prism_rtl` from the repository root.
- **Its C++ twin** is `runtime/cpp`, module for module.
