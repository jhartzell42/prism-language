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
- **`Array`** (`src/collections/array.rs`) is a persistent balanced tree of chunks with a
  tail, the same design as the C++ runtime's `Array`. The tail holds the last elements,
  at most 64, in a vector that grows 1, 2, 4, ..., 64 slots while the array is small; a
  full tail goes into the tree whole, as a chunk. The tree's leaves are chunks of 1 to 64
  elements and its nodes have two children, each with its length and height, balanced
  as an AVL tree is; two trees are concatenated by the AVL join and cut by joins along
  the path to the cut. An update goes through `Rc::make_mut` on the path to its element.
  It is a binary tree of chunks rather than a radix tree: Appendix B asks for `Append` in
  log "(a relaxed radix balanced tree or the like)", and the AVL join keeps its balance
  by a local rule (no node fill to restore after a cut). It has `im_rc::Vector`'s
  methods that emitted code and the runtime call (`new`, `unit`, `len`, `get`,
  `get_mut`, indexing, `push_back`, `push_front`, `append`, `take`, `skip`, `set`,
  `update`, `iter`, `iter_mut`, `into_iter`, `collect`, `extend`, `From<Vec>`, `Eq`,
  `Ord`, `Hash`, `Debug`), so emitted code is unchanged; `take` and `skip` past the end
  panic, as `im_rc`'s did. No `unsafe`. `im-rc` is no longer a dependency.
- **The costs against Semantics Appendix B**, `n` the size of the collection and `m` of
  the second; the book's words first:
  - `Map`: `Insert`, `Remove`, `?Get` and indexing, `?First`: "log", kept (the height is
    at most about 1.44 log n and an update copies one path at most); `Size`: "constant",
    kept; `Keys`, `Values`: "linear, in key order", kept; `Map` of pairs: "`n` log `n`,
    the later pair winning", kept; `Union`: "`m` log `(n+m)`", kept (the smaller map's
    entries go into the larger).
  - `Array`, `Len`: "constant", kept (a count).
  - Indexing `xs[i]`: "constant or log", kept: constant in the tail (an array of 64
    elements or fewer is all tail), log in the tree (its height, about 1.44 log2 of the
    chunks, then an index into a chunk).
  - `Append`: "log (a relaxed radix balanced tree or the like)", kept: the AVL join of
    the two trees, with at most two chunks copied (a tail that fits beside its
    neighbour, a shared one cloned).
  - `Push`: "amortised constant", kept: at most the tail (64 elements) copied when
    another array shares it, and once every 64 pushes the full tail joined to the tree
    (one path, log) with no element copied.
  - `Each`, `Fold`: "`n·t`"; `Sum`: "linear", kept (the iterator walks the chunks).
  - `Take`, `Drop` and an element's update (`set`, `update`, `&xs[i] <= v` in an `&`
    block): Appendix B lists no cost; Semantics ch. 5 asks that a statement on a shared
    part copy "the path from `&_` to the part once per block". Here each is log and
    copies one path and at most one chunk (64 elements), the path once: after it the
    nodes are the array's own and later updates are in place.
  - Storage: at most `2n + 1` slots while the array is small, `n + 64` beyond, and a
    node per chunk; an array from a `Vec` holds exactly `n` (chunks of 64 moved out).
  - `==` on a `Map` or an `Array`: Appendix B states no cost. Linear at worst; a node
    both sides share at the same place answers at once once its values are known to
    equal themselves, which the node remembers (`SelfEqual` in `map.rs`, in a map's nodes
    and an array's joins; a chunk, 64 elements at most, is compared with itself; any
    change through `Rc::make_mut` forgets it), so a value compared with a clone, or with
    an update of itself, costs about the path the update copied after the first
    comparison. A shared node holding a NaN (`Float` `==` is IEEE equality, Type System
    ch. 2) stays unequal, as the element-by-element comparison has it: pointer equality is
    never the answer, so sharing stays invisible (Semantics ch. 11). The tests count the
    values compared.
  - `tests/arrays.rs` and `tests/maps.rs` check the behaviour at the sizes around a
    chunk's edges, the storage each holds (`tests/storage` counts the test binary's
    allocations) and the costs (elements cloned, counted by the element type).
- **`Upper` and `Lower`** (Semantics Appendix B, Text) are the prelude's, not this
  crate's: the Rust prelude (prismc/emit/rust/prelude.prism) answers
  `Text::from(t.as_str().to_uppercase())` and `.to_lowercase()`. `std`'s mapping is
  the statement both targets meet (`tests/text/README.md`): Unicode's default full case
  mapping (`SpecialCasing.txt`'s unconditional mappings over `UnicodeData.txt`'s simple
  ones: `ß` → `SS`, `ﬁ` → `FI`, `İ` → `i̇`), Final_Sigma in `Lower` (`Σ` → `ς` at the end
  of a word, the one conditional mapping `std` applies), no locale's tailoring (no
  Turkish dotless `ı`), no normalization; linear, one pass, with a copy into the
  counted buffer even when nothing changes. Its tables are Unicode 17.0.0's in Rust 1.98
  (`char::UNICODE_VERSION`), the version the C++ runtime's tables are generated from;
  `tests/case.rs` runs the prelude's bodies over the shared `tests/text/case.txt` and
  checks the version.
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
