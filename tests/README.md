# tests

- **`conformance/`**: programs, each a tree (docs/modules-style-pass.md): a directory
  `NAME/` with its root `app.prism`, its test module `tests.prism` (in the Conformance
  book's test language) and the transcript `tests.expected`, each pinning a rule of the
  books; a program's other modules are its submodules (`sem-ch06-birds/birds/`). The
  prefix names the book and chapter (`sem-ch09-...` is Semantics chapter 9, `ts-ch05-...`
  Type System chapter 5, `conf-ch01-...` Conformance, `compiler-ch03-...` Compiler; `mod-`
  the trees themselves). Every one of them is a program with a language-defined
  transcript, so every one is named for the rule its transcript pins.
- **`compiled/run.sh`**: every program above, plus chess and prismc's lexer and parser
  checks (the test modules of prismc/syntax/lexer and prismc/syntax/parser), compiled by
  `prismc` and run under the Test World, byte-equal to `.expected`.
- **`checker/`**: the checker's own tests: every corpus program checks clean, every
  program under `neg/` gives exactly its `.expected` diagnostic; `run.sh` runs them. A
  directory under `pos/` or `neg/` is one program, a tree, its `.expected` beside it.
- **`driver/run.sh`**: which World `./prism run` picks for a root.
- **`make test`** from the repository root runs all of them, cargo's tests and the
  bootstrap.
