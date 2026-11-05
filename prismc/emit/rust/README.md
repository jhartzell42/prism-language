# prismc/emit/rust/

The module `Emit.Rust` (claude/modules-style-pass.md): Emission: the model lowering decided (lower.prism) spelled as a Rust crate through the templates (templates/rust.prism, templates/prelude.prism): `Cargo.toml`, `src/main.rs`, `src/prelude.rs` and one `src/NAME.rs` per Prism module.

- `mod.prism`: `Emit.Rust` itself: Types; Expressions; Patterns and statements.
- `items.prism`: `Emit.Rust.Items`, a child module: Items; Nodes and programs; The crate.
- `prelude.prism`: `Emit.Rust.Prelude`, a child module: The prelude of every compiled program (Semantics Appendix B; prelude/prelude.prism): its builtins and the functions it writes in Prism, as Rust over `prism_rtl`, emitted as `src/prelude.rs` word for word.
- `templates/`: `Emit.Rust.Templates`, a child module with children of its own (its README).

Written by Claude on the claude-self-hosting branch; subject to change.
