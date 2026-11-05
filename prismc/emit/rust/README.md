# emit/rust/

The module `Emit.Rust` (claude/modules-style-pass.md): Emission: the model lowering decided (lower.prism) spelled as a Rust crate through the templates (templates/rust.prism, templates/prelude.prism): `Cargo.toml`, `src/main.rs`, `src/prelude.rs` and one `src/NAME.rs` per Prism module.

- `mod.prism`: the main file: the header; types, expressions, patterns and statements.
- `items.prism`: a section; items, nodes and programs, the crate.
- Submodules: `prelude/` (`Emit.Rust.Prelude`), `templates/` (`Emit.Rust.Templates`).

Written by Claude on the claude-self-hosting branch; subject to change.
