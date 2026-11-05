# prelude: the Prism prelude and the standard tag families

The Prism source every program is checked against, a library tree
(docs/modules-style-pass.md): `lib.prism` declares the modules, each a directory:
`prelude/` (`Prelude`: types, traits and functions every module sees) and the families the
Worlds answer: `ui/`, `gui/`, `fs/`, `process/`, `time/`, `test/`.

- **The checker reads these files**; the Rust and C++ that implement the prelude's
  builtins are in `prismc/emit/rust/prelude/` and `prismc/emit/cpp/prelude/`.
- **Changing a signature here changes the language** a program sees; the books (Semantics
  Appendix B) describe the same prelude.
