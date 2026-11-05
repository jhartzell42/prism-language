# Every array, however small, allocates whole 64-element chunks

*Written by Claude on the claude-self-hosting branch, 5 November 2026; subject to
change. A performance bug: the programs are right, and they use far more memory than
their data needs.*

## The scenario

prismc compiling itself on the cloud box (5 November, branch head `aa500e5`):

| | peak memory |
|---|---|
| the Rust-built prismc | 2.43 GB |
| the C++-built prismc | 1.01 GB |

Both take about 90 s, so the 2.4× is memory alone. Compiling `examples/chess` under
valgrind's massif shows the same shape on a small scale: a 103 MB peak heap for the Rust
binary, 56 MB for the C++ one.

## Where the memory goes

**Rust,** at the peak (massif, `examples/chess`), the biggest sites are `im_rc::Vector`'s:

- `Vector<check_types::Ty>::promote_front`: 18.0 MB
- `Vector<lower::Ex>::promote_front`: 16.5 MB
- `Vector<typed::ImplNote>::push_back`: 9.5 MB
- `Vector<lower::Do>::promote_front` and `Vector<ast::Stmt>::push_back`: 5.7 MB each
- `Rc<Chunk<ast::Arg>>::clone_from_ref_in`: 5.5 MB (a shared chunk copied whole on
  a write)

**C++,** at its peak, the biggest are `prism_rtl::Array`'s constructors:

- `Array<Text>::Array(std::vector)`: 10.9 MB, nearly all from `lex_qualified_path`
- `Array<ast::Arg>::Array(std::vector)`: 9.9 MB, from the parser's `commas`

## The cause

Both runtimes store an array in fixed chunks of 64 elements, and allocate a whole chunk
for the first element:

- **Rust:** `Array<T>` is `im_rc::Vector<T>` (`runtime/rust/src/collections.rs`), an RRB
  tree whose nodes are `sized_chunks::Chunk<T, 64>`. A vector that outgrows its inline
  head is "promoted" to a heap chunk of 64 slots (`promote_front`), and a write to a chunk
  another vector shares copies the whole chunk (`clone_from_ref_in`).
- **C++:** `Array<T>` is a view of a `Buffer` whose `push` allocates
  `std::make_unique<Slot[]>(CHUNK)` with `CHUNK = 64` when the size is a multiple of 64,
  including at 0 (`runtime/cpp/include/prism_rtl/collections.hpp`).
- **A compiler's arrays are almost all tiny:** a type's arguments, a call's arguments,
  a qualified name's segments, a block's statements. A two-element array of a 64-byte
  `Ty` holds 128 bytes in a 4 KB chunk.
- **Why Rust is worse:** `sizeof` of its enums is larger, `im_rc` copies shared chunks on
  write where the C++ view appends in place at the buffer's end, and prepending
  (`promote_front`) starts a new chunk.

## Is it a bug?

Yes. It costs memory proportional to the number of arrays rather than their contents,
and it is why the Rust binary needs 2.4 GB to compile 26k lines. It also likely costs
time (malloc and free were about 25% of the C++ binary's instructions in the earlier
callgrind run). Appendix B's cost promises are about time; nothing promises memory, but a
compiler that needs gigabytes for its own source is wrong.

## The fix

- **The first chunk sized to fit:** grow it geometrically (1, 2, 4, ..., 64 slots) and
  use 64-slot chunks only beyond that, in both runtimes. A copy for a write then copies
  only the elements there are.
- **One design for both runtimes:** port the C++ view-over-a-shared-buffer `Array` to
  Rust in place of `im_rc::Vector`, with the geometric first chunk, so the comparison
  between the targets stops measuring two collection libraries. (The same lesson as the
  rope `Text`.) `Map` (`im_rc::OrdMap` against the C++ AVL tree) deserves the same check.
- **Measure:** peak memory and wall time for prismc compiling itself in both modes,
  before and after, and massif's peak on `examples/chess`.
- **Tests:** the existing suites pin behavior; add runtime unit tests for the small-array
  growth path (empty, 1, 2, 63, 64, 65 elements; a shared view written at each size).
