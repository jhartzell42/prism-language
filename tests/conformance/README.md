# tests/conformance/

Programs, each a tree (claude/modules-style-pass.md) pinning a rule of the books, and this
README stands for every directory under it: a program's directory `NAME/` holds its root
`app.prism`, its test module `tests.prism` (`test mod Tests` in the root's header; the
module `Tests`, which reaches the root by `use App.*`, in the Conformance book's test
language) and its transcript `tests.expected`; a program's other modules are its child
modules, files beside `app.prism` or directories with a `mod.prism` of their own
(`sem-ch06-birds/birds/`, `mod-nested/shapes/round/oval/`), a module's own test module
beside its file with its own transcript (`mod-test-module/counter/tests.prism`).

- The prefix names the book and chapter: `sem-ch09-...` is Semantics chapter 9,
  `ts-ch05-...` Type System chapter 5, `conf-ch01-...` Conformance, `compiler-ch03-...`
  Compiler; `mod-...` the module trees themselves, `cpp-...` what the C++ target once got
  wrong.
- Every program has a language-defined transcript, so each is named for the rule its
  transcript pins; a transcript's first line says how it was derived.
- `tests/compiled/run.sh` runs them all on either target; `tests/checker/run.sh` checks
  each clean.

Written by Claude on the claude-self-hosting branch; subject to change.
