# tests/checker/

The checker's tests (prismc/README.md, Checker), through a native prismc's `--check`;
`run.sh` runs them (its header says how).

- **`pos/`**: what checks clean, or gives exactly its `NAME.expected`: every corpus
  program (the conformance programs, chess, the prelude, prismc itself), each a tree, and
  the programs of its own, a loose `NAME.prism` or a tree `NAME/` (`mod-tree`,
  `lib-tree`); `NAME.colors` holds a program's resolved colors.
- **`neg/`**: one wrong program each, a loose `NAME.prism` or a tree `NAME/` (the module
  layout's refusals: `mod-lowercase`, `mod-both`, `mod-leaf`, `mod-missing`,
  `mod-stray`, `module-line`, `two-roots`, `no-root`, `test-outside`,
  `test-declares`, `test-only-reached`, `family-across-modules`, ...), giving exactly
  its `NAME.expected`. A `# args: ...` first line passes options.
- **`regex-neg.py`**: writes `neg/regex-NAME.prism` and its `.expected`, one program for
  each diagnostic of claude/regex-dialect.md: the first literal tests/regex/corpus.txt
  refuses with that message, its position moved into the line (`python3 -I
  tests/checker/regex-neg.py` after the corpus or the messages change). The type errors
  (`regex-not-replacement`, `regex-replacement-not-regex`, `regex-no-ord`) are written
  by hand.

This README stands for `pos/` and `neg/` and the trees under them.

Written by Claude on the claude-self-hosting branch; subject to change.
