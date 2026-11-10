# WIP: getting a compiler for the renamed source

Written by Claude (the coordinator) on 10 November 2026, when every agent was stopped
for a handoff. Delete this directory before anything merges into claude-self-hosting.

## The problem
- The field-sigil rename (fc00cb6) changed prismc's own source to `$label: Text` fields.
- The checked-in seed predates it, and so does every compiler `bootstrap.sh --from-seed`
  builds. None of them can compile prismc's source on claude-bugs or claude-lowering.
- The rename was done in two stages. A compiler built from old-syntax source that also
  read the new syntax (of1) compiled the renamed source (of2). That stage-1 source was
  never committed.
- Jimmy ruled: no seed rebuild until every bug is fixed. So this binary bridges the gap.

## The bridge
- `claude/wip/bin/prismc-of3`: Linux x86-64, built with --release from claude-bugs
  d4d4afc (WT, LC2, BO, FD). It compiles the renamed source on claude-bugs and on
  claude-lowering. Use it as PRISMC or as BINARY to build a gen1:
  `cp claude/wip/bin/prismc-of3 target/probes/ && PRISMC=target/probes/prismc-of3 ./prism check prismc/`.
- At the end, the single fixed point and seed rebuild remove the need for it. The fresh
  clone `--from-seed` check must then pass with no bridge.
- `claude/wip/sigil-rename.py` is the rename script. It is idempotent: run it on any
  branch that still has old-syntax fields
  (`python3 claude/wip/sigil-rename.py --write prismc prelude tests examples bench`).
