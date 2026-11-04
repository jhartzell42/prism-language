# tests

- **`conformance/`**: programs with `.prismtest` scripts and `.expected` transcripts, each
  pinning a rule of the books. The prefix names the book and chapter (`sem-ch09-...` is
  Semantics chapter 9, `ts-ch05-...` Type System chapter 5, `conf-ch01-...` Conformance,
  `compiler-ch03-...` Compiler). A directory of a program's name joins it.
- **`backend/`**: transcripts only `prismc`'s emission can get wrong (names, inference
  rustc needs, printing), with no book rule behind them.
- **`compiled/run.sh`**: every program above, plus chess and prismc's lexer and parser
  checks, compiled by `prismc` and run under the Test World, byte-equal to `.expected`.
- **`checker/`**: the checker's own tests: every corpus program checks clean, every
  program under `neg/` gives exactly its `.expected` diagnostic; `run.sh` runs them.
- **`prismc/`**: prismc's lexer and parser checked through its modules.
- **`driver/run.sh`**: which World `./prism run` picks for a root.
- **`make test`** from the repository root runs all of them, cargo's tests and the
  bootstrap.
