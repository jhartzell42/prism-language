# tests

- **`conformance/`**: programs, each a tree (claude/modules-style-pass.md): a directory
  `NAME/` with its root `app.prism`, its test module `tests.prism` (`Tests`, in the
  Conformance book's test language) and the transcript `tests.expected`, each pinning a
  rule of the books; a program's other modules are its child modules
  (`sem-ch06-birds/birds/`, `mod-nested/shapes/`). Its README says how they are named.
- **`compiled/run.sh`**: every program above, plus chess and prismc's lexer and parser
  checks (the test modules of prismc/syntax/lexer and prismc/syntax/parser), compiled by
  `prismc` and run under the Test World, byte-equal to `.expected`. No transcript over
  10 KB is kept in the repository (Jimmy; `run.sh` refuses one): a test module without
  its `.expected` prints its transcript and passes on its own checks, as chess's board
  and window library do.
- **`checker/`**: the checker's own tests: every corpus program checks clean, every
  program under `neg/` gives exactly its `.expected` diagnostic; `run.sh` runs them. A
  directory under `pos/` or `neg/` is one program, a tree, its `.expected` beside it.
- **`text/`**: `case.txt`, what `Upper` and `Lower` answer, which both runtimes' tests
  read (its README states the mapping).
- **`driver/run.sh`**: which World `./prism run` picks for a root.
- **`cost/run.sh`**: work counted, not time (instructions retired, by bench/lib.sh:
  macOS's `time -l`, `perf stat` on Linux with hardware counters, else valgrind's
  cachegrind): a costly `let` read by ten places costs under twice one reader's
  instructions, and the other pairs tests/cost/README.md names.
- **`make test`** from the repository root runs all of them, cargo's tests and the
  bootstrap.
