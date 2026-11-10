# WIP handoff: claude-bugs, stopped 10 November 2026, 16:30 ET

Written by Claude (the coordinator) when Jimmy stopped every agent. This branch,
claude-bugs-wip, is origin/claude-bugs (d4d4afc) plus one WIP commit holding the bug
agent's uncommitted work. Move it onto claude-bugs only once it is green.
target/HANDOFF.md in the old worktree was stale (9 November), so this replaces it.

## Pushed on claude-bugs, green on of3 (check, unused, checker, Rust, C++, driver)
- fc00cb6: field sigils on names (SF/SR), records and non-element tags lifting at plain
  fields (LC, TL). The compiler refuses old `label: $Text` with `field-sigils` and its fix.
- d4d4afc: an element's identity field lifts the construction to `$Element o` (WT): a
  `let` holds it, and a widget statement or binding refuses it at the field with the
  `rebuild` fix (the hold fix for an event). Local fn clauses group into one function
  (LC2). A cycle among plain values is an error (BO, Semantics ch. 4 "Binding groups and
  cycles"; a cycle of functions alone is allowed).
- 70f3000: FD, a lambda as a generic fold's initial value (rustc used to refuse it).
- Earlier: MS, SH (GHC's MonoLocalBinds), block signature lines checked
  (`no-definition`), stdNames deleted, Error-in-data, `_w`/`_t`/`_g` names.

## In the WIP commit (unbuilt or unverified)
- AK: each associated type is a type-level name of its trait's module (`G.assocNames`),
  resolved by ResolveType; duplicates are `duplicate-name`; the four-step lookup and
  Cx.ownTrait/ownAssocs/bounds are deleted. Its failing tests are a15d23d. The bug agent
  reported this in its tree; whether it passed suites is unknown.
- The supplier block-body fix: a named widget function's block body checks its last-line
  match against `@o` (failing test d106a2d).
- CK, started: lexer and parser edits for `[a]+`, `[k: v]+` and `as`. Its failing tests
  are d524eff (ts-ch02-non-empty, checker non-empty-literal). The checker and lowering
  halves are not written.

## Queue, in order, each failing test first
1. Build a gen1 of this WIP (with claude/wip/bin/prismc-of3), and get AK and the supplier
   fix green on both targets. Then commit them properly and push to claude-bugs.
2. CK (Type System RULINGS round 7): `[a]+` and `[k: v]+`, `xs as [a]+?` giving nil on an
   empty collection, `e as T` as ascription.
3. AQ (Jimmy's new ruling, Semantics RULINGS round 12): delete `AnswerOf`. Each tag
   declaration gets a generated answer constructor typed by that tag's answer, and
   `Answer c` is their union, so a wrong answer is a type error. No hand-written tag
   list. The lowering half follows in claude-lowering.
4. A final hunt round across the conformance suite and examples.
