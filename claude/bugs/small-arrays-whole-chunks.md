# Every Rust array, however small, allocates a whole 64-element chunk

*Written by Claude on the claude-self-hosting branch, 5 November 2026; subject to
change. A performance bug: the programs are right, and the Rust ones use far more memory
than their data needs. The C++ half is fixed, and so is the same waste in Rust's `Map`;
the Rust `Array` waits for a ruling on Semantics Appendix B's promises.*

## The scenario

prismc compiling itself (Apple M2 Pro, 5 November; the tables are in prismc/README.md,
"Arrays and maps: what they hold"): the Rust-built prismc's peak memory footprint is
about 2.7 GB, the C++-built one's about 1.0 GB, in about the same instructions.

## The cause

`Array<T>` in Rust is `im_rc::Vector<T>` (`runtime/rust/src/collections.rs`), an RRB
tree whose nodes are `sized_chunks::Chunk<T, 64>`. A vector that outgrows the few
elements its inline head holds (none, for a 64-byte element) is promoted to a heap chunk
of 64 slots, and a write to a chunk another vector shares copies the whole chunk. A
compiler's arrays are almost all tiny (a type's arguments, a call's, a name's segments),
so a two-element array of a 64-byte `Ty` holds 4 KB. `runtime/rust/tests/arrays.rs`'s
ignored `an_array_holds_storage_linear_in_its_length` shows it: one 64-byte element
pushed onto a shared empty array holds 4,128 bytes.

## What is fixed

- **C++ `Array`:** a buffer's first chunks hold 1, 2, 4, ..., 64 slots
  (runtime/cpp/README.md, "Arrays and maps: the costs").
- **Rust `Map`:** the C++ AVL tree, ported (`runtime/rust/src/collections/map.rs`);
  `im_rc::OrdMap` allocated a 64-entry node for every map, an empty one included.

## The design question under the Rust fix

The proposed fix was to port the C++ view-over-a-shared-buffer `Array` to Rust in place
of `im_rc::Vector`, so that both targets measure one collection. That design does not
keep two of Appendix B's promises, which the C++ runtime already breaks:

- **`Append: fn [a], [a] -> [a]`, log:** the view copies the second array onto the
  first (linear in it), and the first too unless it is the newest view of its buffer.
- **`Push: fn [a], a -> [a]`, amortised constant:** constant onto the newest view only;
  a second push onto one array, or a push onto a `Take` or `Drop` that does not end its
  buffer, copies the view, linear.

`im_rc` keeps both. The choices:

- **A persistent RRB tree with small leaves, in both runtimes** (Claude's
  recommendation): every promise kept, one design for both targets; the only leaf and
  the tail grow 1, 2, 4, ..., 64 as the C++ buffer's first chunks do. The most work.
- **The view in both runtimes, with the books relaxed:** `Append` linear, `Push`
  constant on the newest view only. Jimmy's call; the C++ runtime is there today.
- **A small form in front of `im_rc` in Rust:** up to 64 elements a buffer of its own,
  then `im_rc::Vector`; every promise kept (a bounded array's operations are constant)
  and the waste gone, but the targets still measure two collection libraries.

## Tests

`runtime/cpp/tests/test_arrays.cpp`, `runtime/rust/tests/arrays.rs` and
`runtime/rust/tests/maps.rs`: the sizes around a chunk's edges (empty, 1, 2, 63, 64, 65,
...), a shared value written at each, and the storage each holds.
