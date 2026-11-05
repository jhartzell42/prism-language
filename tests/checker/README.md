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

This README stands for `pos/` and `neg/` and the trees under them.

Written by Claude on the claude-self-hosting branch; subject to change.
