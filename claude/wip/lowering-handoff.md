# WIP handoff: claude-lowering, stopped 10 November 2026, 16:30 ET

Written by Claude (the coordinator) when Jimmy stopped every agent to hand off to a new
session. This branch, claude-lowering-wip, is claude-lowering plus unpushed and
uncommitted work. Move it onto claude-lowering only once it is green.

## What sits on top of origin/claude-lowering (b015a3d)
- 3b78553 Lower a builder in a rebuild generation over its block's names (DR, gap DY).
  Built, and Rust-tested by the third lowering agent; the C++ side was NOT verified.
- 2b7ba8d Failing test: one pattern over a live element and a description
  (sem-ch09-element-pattern-forms, LE).
- 645086f Failing test: a building match arm ending on a plain result
  (sem-ch04-building-match-plain).
- The WIP commit above them: uncommitted, unbuilt work in lower/, emit/ and
  claude/backend.md, most likely the start of LE option O1 (element descriptions).
  Treat it as a draft: read it, keep what is right, and verify it.
- Earlier failing tests already pushed: 08cce97 sem-ch04-element-value (LE b),
  0b51886 sem-ch09-live-element-field (LE a).

## Rulings since the handoff below was written
- LE(a): a plain function reading a live element's `$` field is whole: `e.$field`
  binds the projection Dynamic, and callers sample it.
- LE(b), Jimmy's ruling EV: elements are ordinary values. Option O1: an Element value is
  Live(Live<E>) or Described(D), where D is a per-tag struct emitted from the IR (no tag
  list) whose `$` fields hold Dynamics. Patterns match either form, binding a `$T`.
  Placing a description follows an inline element's birth and path rules.
- Jimmy's ruling AQ: `AnswerOf` goes. Each tag declaration gets a generated answer
  constructor typed by that tag's answer, and `Answer c` is their union. This is checker
  work (bug agent), then lowering.
- TP: a building match/if adds no path segment (book), ruled for now.
- The books moved: prism-notes claude/prism/... is now claude-old/prism/...

## Tooling
- The scripts in claude/wip/scripts/ were in the old session's scratchpad. Their paths
  inside may need editing.
- Binaries do not survive a new container. See claude/wip/bootstrap.md on
  claude-bugs-wip.

---

# Handoff: claude-lowering (worktree /home/claude/prism-lowering)

For a fresh lowering agent. The task is to lower every item of prismc/README.md "### Not
lowered yet" on both targets.
- Commit each failing test alone first, then the fix.
- No silent fallbacks, no hand-written name lists.
- Never edit README.md, notes/, TODO.md or LANGUAGE.md.
- Earlier notes are in target/HANDOFF.round2.md (the round before) and
  target/HANDOFF.old.md.

## State
- Pushed head: b015a3d on origin/claude-lowering.
- The field-sigil rename (SR) is applied (3a7bbe6) and claude-bugs fc00cb6 is merged
  (4cb5683). Write `$label: Text` fields and `$label=..` arguments from now on.
- Binaries:
  - target/probes/prismc-l19 is gen1 of b015a3d.
  - /home/claude/prism-bugs/target/probes/prismc-of2 also compiles the renamed source.
  - Pre-rename binaries (at9, l11, l12) cannot compile prismc any more.
- Suites on l19 (logs in target/probes/l19/):
  - Green: check, unused, driver, cpp3 (chess, lexer, parser, regex).
  - Rust: 219 ok. It fails only the pinned sem-ch04-local-function-clauses and the
    failing test sem-ch07-generation-rebuild (DR).
  - Checker: only the pinned local-function-clauses and supplier-block-body.
  - cpp1 was killed (exit 144) partway through, at sem-ch07-unbounded-lifted, whose
    "failure" is the kill (`Terminated`). Everything before it passed, apart from the two
    expected failures. Rerun cpp1 from there, then cpp2. Each new C++ test passed on its own.
- On l17 (the merge) everything else was green: check, unused, driver (chess-cpp
  included), and Rust apart from sem-ch07-paged, which was fixed in b015a3d and passes on
  l19 under both targets.

## Tooling (scratchpad /tmp/claude-0/-home-claude/5a4e5003-3931-59ff-932c-4878e7a04b3d/scratchpad)
- `lgen1rev.sh NAME BINARY REV`: gen1 of git REV's prismc/ and prelude/, built
  `--release`, at target/probes/prismc-NAME (about 10 min). Commit WIP first; it archives
  the REV.
- `lparts.sh NAME BINARY PART`: PART is one of
  - `rest`: check, unused, checker, driver, Rust;
  - `cpp1`, `cpp2`: the conformance halves;
  - `cpp3`: chess, prelude-maps, lexer, parser, regex.
  `lchain.sh NAME BINARY PART...` runs parts in turn with `PRISM_JOBS=1`. Each background
  run is cut at 2 h, so run one or two parts per job.
- Fast type check: `PRISMC=target/probes/prismc-l19 ./prism check prismc/` (about 60 s).
- Runtime checks:
  - Rust: `CARGO_TARGET_DIR=<scratch>/rtl-target cargo build --offline --release -p prism_rtl`.
  - C++: `make -s -C runtime/cpp check CXX=clang++ CONFIG=clang`.
- Memory is 7 GB and there are 2 cores, shared with the bug agent. Run one heavy build or
  suite at a time, with `PRISM_JOBS=1`. Never edit runtime/ while a suite runs.

## Done this round
- FT (622033d) and FA (04a768c) are the tests; FV is 2757990.
  - `@Flatten` of any `$$a` (`HeldFlatten`). The outer's update port queues an owed
    adoption (`Adoption`, answer_later; doubt AL, accepted).
  - `Peel` takes one layer, so `$$a` is a dynamic of `Dynamic`s.
  - Whole functions by call site.
  - `$ $q` is a map to constants.
  - Holds of dynamics.
  - Live-dynamic ports fire once a tick.
- UP/SU: 1af0b25 is the test and 4447b7e the fix. `!Updated` in a generation over its
  block (`GenHears`: ports `{gen}_wrote_{m}_port`).
- `!MergeAll` of a plain list: ad54a80 is the test and 4447b7e the fix (runtime
  `MergeList`).
- ML: a24e63e is the test and b015a3d the fix.
  - `@Switch` of any dynamic of events (`HeldSwitch`, swap list, runtime `Listen`).
    Before this, it silently never fired.
  - `!MergeAll` lifted over a dynamic list (runtime `merge_all` / `MergeNode`).
- DR: fe0958a is the test and b015a3d the refusal. A builder in a generation over a block
  name that the outer source does not read is refused loudly (`UnheardGaps`).
- claude/backend.md describes each of these.

## Left
1. **DY remainder (DR).** A generation's own builders (a `rebuild` or `for` in it) over
   block names its outer `rebuild`'s source does not read are told nothing. They are
   refused today, and sem-ch07-generation-rebuild is the failing test.
   - The obvious fix is to tell the generation through `GenHears` for `RowHears`, not only
     `UpdatedHears`.
   - That breaks sem-ch07-paged: the old generation rebuilds its inner in the tick it is
     replaced.
   - So the fix must tell the builder only while its generation stays current after the
     tick: skip the build when the block replaced the generation, or order the block's
     build before the generation's and check staleness.
2. **LE** (README pick LE), in two parts:
   - A live element's `$` field read in a plain function. A draft probe is in
     target/scratch/le1, in the pre-rename syntax, and it needs `$Shown`.
   - An element applied as a plain value (`TagValue`, "a live element comes from its
     host").
   - Idea: treat a function with a live-element parameter as whole. In patterns.prism, the
     LE refusal in a whole body would then bind the `$` field to the projection `Dynamic`
     (`level.map`) instead of refusing, and callers sample it (`SampledObject`).
   - That is a design call: ask the coordinator first.
3. **A building match/if arm ending on a plain result** (`ArmRowOf`, "a building `match`
   or `if` whose arm ends on a plain result").
   - The coordinator's direction: split the block's build around that line, keeping births
     in source order (Semantics ch. 8).
   - Do not pass build-time arguments for every name the arm reads.
4. **Zombie and dead states** (`leave`, `clean_up`, `edges`). Do these last.

BO and the checker gaps pinned by f7a90e3 and c6df448 belong to the bug agent.

## Doubts raised to the coordinator
- AL: answer_later rather than the swap list for the flatten (accepted).
- DD: the FV tests are also the failing tests for the `$$` collapse (accepted).
- UP: DY in a generation was taken as UP/SU's remaining case (accepted).
- The generation and MergeAll fixes share one commit (4447b7e). Their tests came first.

## README entries now stale
For Jimmy, through the coordinator; the README itself was not edited.
- From the previous handoff: "lifted functions with time parameters" was already
  lowered, and all of these are lowered on this branch:
  - @HostList, NS, HO, CS, `Time.After`;
  - building match/if except an arm ending on a plain result;
  - an `@Machine` whose step is not a named function;
  - local functions as values or with pattern parameters;
  - an unbounded call lifted over `?`/`~`;
  - the driver's clause-plain-result case.
- "`@Flatten` of a dynamic that is not a `rebuild`'s (pick FV)" is lowered (2757990).
- "`!Updated` of a dynamic no sink of the node writes (pick UP), and in a row over a
  structural dynamic of its block (gap SU)". Every form found is lowered. The README
  pick UP text about refusing in a generation is stale since 4447b7e.
- "`!MergeAll` of a list that is not a literal" is lowered (4447b7e, b015a3d).
- New and not in the README:
  - `@Switch` of a non-`rebuild` dynamic of events is lowered (b015a3d).
  - A generation's builder over an unheard block name is refused (DR).
