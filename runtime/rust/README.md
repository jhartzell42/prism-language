# runtime/rust: `prism_rtl`

The runtime library every Rust program `prismc` emits links. Emitted code is typed Rust,
one type per Prism type, with each widget's graph emitted from code templates; this crate
is the part that is not generated per block, written once and shared.

- **Modules:** numbers (`num`), persistent collections, `Pod` and paths, the transcript's
  printer and reader, the node primitives (`cells`: `Hold`, `Stamped`, `Cache`, `Port`,
  `Event`, `Dynamic`, `Defer`, `Job`), the tick (`cx`), the document and patch (`doc`),
  the zombie collector (`gc`), hosting (`host`), and the World protocol with the Test
  World (`world`). `src/lib.rs` describes each.
- **The contract** between this crate and the emitted code is `docs/backend.md`.
- **Build and test:** `cargo test -p prism_rtl` from the repository root.
- **Its C++ twin** is `runtime/cpp`, module for module.
