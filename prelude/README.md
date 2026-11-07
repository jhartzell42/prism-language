# prelude: the Prism prelude and the standard tag families

The Prism source every program is checked against, a library tree
(claude/modules-style-pass.md): `lib.prism` declares the modules, each a file of its own:
`prelude.prism` (`Prelude`: types, traits and functions every module sees) and the
families the Worlds answer: `ui.prism`, `gui.prism`, `fs.prism`, `process.prism`
(`Process.Args`, `Process.Env`, `Process.Exit`), `time.prism`, `test.prism`.

- **The checker reads these files**; the Rust and C++ that implement the prelude's
  builtins are in `prismc/emit/rust/prelude/` and `prismc/emit/cpp/prelude/`.
- **Changing a signature here changes the language** a program sees; the books (Semantics
  Appendix B) describe the same prelude.
- **Widgets with no body** (`@Switch`, `@Flatten`, `@Distinct`, `@PerKey`, the hosts) are
  builtins of the lowering (prismc/lower/node/static.prism and switch.prism, with their
  state in the runtimes' `prism_rtl`); the checker reads only their signatures, so a
  `where` on one (`@PerKey`'s `v: ^Eq`) is checked like any other.
