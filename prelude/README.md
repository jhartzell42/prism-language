# prelude: the Prism prelude and the standard tag families

The Prism source every program is checked against: `prelude.prism` (types, traits and
functions every module sees) and the families the Worlds answer: `ui.prism`, `gui.prism`,
`fs.prism`, `process.prism`, `time.prism`, `test.prism`.

- **The checker reads these files**; the Rust and C++ that implement the prelude's
  builtins are in `prismc/templates/` (`prelude.prism` holds the Rust).
- **Changing a signature here changes the language** a program sees; the books (Semantics
  Appendix B) describe the same prelude.
