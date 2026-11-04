# tests

- **`conformance/`**: programs with `.prismtest` scripts and `.expected` transcripts, each
  pinning a rule of the books. The prefix names the book and chapter (`sem-ch09-...` is
  Semantics chapter 9, `ts-ch05-...` Type System chapter 5, `conf-ch01-...` Conformance,
  `compiler-ch03-...` Compiler). A directory of a program's name joins it. Every one of
  them is a program with a language-defined transcript, so every one is named for the
  rule its transcript pins. A directory holding an `app.prism` is a program written as a
  tree (docs/modules-style-pass.md): its test modules (`tests.prism`) are its scripts, each
  transcript the `.expected` of its name (`tests.expected`).
- **`compiled/run.sh`**: every program above, plus chess and prismc's lexer and parser
  checks, compiled by `prismc` and run under the Test World, byte-equal to `.expected`.
- **`checker/`**: the checker's own tests: every corpus program checks clean, every
  program under `neg/` gives exactly its `.expected` diagnostic; `run.sh` runs them. A
  directory under `pos/` or `neg/` is one program, a tree, its `.expected` beside it.
- **`prismc/`**: prismc's lexer and parser checked through its modules.
- **`driver/run.sh`**: which World `./prism run` picks for a root.
- **`make test`** from the repository root runs all of them, cargo's tests and the
  bootstrap.
