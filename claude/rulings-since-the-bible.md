# Rulings made during the prismc run, after the books moved

*Written by Claude (the prismc coordinator), 10 November 2026. Subject to change;
implementers use discretion.*

The Semantics and Type System books moved into the Prism Bible (prism-notes, `claude/`),
which another session writes. Rulings Jimmy makes during the prismc run after that move
are kept here, with his words, until they are woven into the Bible. Nothing is removed.

## 10 November 2026

- **AN** The answer constructor that ruling AQ generates for a tag is written as the
  tag's name with a last segment `Answer`: `Ui.Button.Answer click`,
  `Fs.Contents.Answer (~ c)`. Jimmy: "AN seems good!" Refused: a suffix on the tag's own
  name, `Ui.ButtonAnswer`, which collides with a tag named `FooAnswer`. Where: Semantics
  chapter 9 and Appendix B.
- **RM** A README gets no edits from Claude except deleting what went stale. Jimmy: "stop
  modifying `README.md`. If they're stale entries, delete them though. Delete stale
  things." Whoever lowers a construct deletes its "Not lowered yet" entry in the same
  commit.
- **EL** The finished branch lowers everything the compiler knows it refuses. Jimmy: "I'd
  like the main Claude branch to contain proper lowering when you're done of everything
  you know about and not to have lies in the `README.md`."
- **IB** An inner block may rebind a name written outside it: a lambda's parameter, a
  clause's parameter, an arm's pattern, a `let` or a bind in an inner block each hide the
  outer name inside that block. Jimmy: "Inner block rebinding is fine, I don't know why
  it's forbidden". This overrules Semantics chapter 4's "an inner block does not rebind a
  written name from outside" and its negative-space line "no rebinding of a written name".
  A name bound twice in one block stays an error (`collision`), since a block's lines are
  an unordered group. Refused: enforcing the book's sentence, which breaks 14 places (10
  in prismc, 4 in two conformance programs).
- **TC** Places are counted as they are built. After a building `match` or `if`, or a
  widget of several clauses, the block's later places take the next numbers after the
  places the chosen arm or clause built, so their positions differ between instances
  that took different arms, and no position is left unborn. Jimmy: "count things as they
  are built then!" This settles the question TP kept alive (Semantics chapter 8, "takes
  the next number"). Refused: prismc's pick CS, the widest arm's positions reserved,
  which has no answer for a blueprint parameter or for a clause widget that calls itself
  in a static position (prismc overflowed its stack on one).
- **CL** `match x as [a]+?:` reads without parentheses: the lexer is fixed so that the
  checked cast's `+?` before a block's `:` is the non-empty mark and the option. Jimmy:
  "fix the lexer". Refused: leaving the book's lexing rule and writing
  `match (x as [a]+?):`. Where: Semantics chapter 2; Type System chapter 2, "Collections".
- **AD** A trait's default for an associated type is read: an impl that does not state
  that associated type takes the default, and `impl-lacks` is reported only for one with
  no default. Jimmy: "fix it". Where: Type System chapter 3.
- **NS** The conversion from a non-empty collection to its plain type is symmetric at an
  operator: `[Nat]+ == [Nat]` checks as `[Nat] == [Nat]+` does. Jimmy: "change it".
  Refused: the conversion only at the right operand, the old rule as written. Where: Type
  System chapter 2, "Collections".
- **RA** Every lowering refusal is fixed by its class. A legal program the back end does
  not lower is lowered ("fix these all"); a program that is wrong gets a checker error; a
  guard no checked program reaches becomes an internal error. Jimmy: "apply appropriate
  fixes based on category".
- **BT** Work in bigger batches, one build per batch, and C++ last. Jimmy: "fix bugs in
  bigger batches rather than doing full builds after each one and save fixing C++ until
  the end when you make one big commit in the branch." During the run a fix is verified
  on Rust; the C++ fixes land as one commit after one C++ pass at the end.
- **TF** Tests first for everything, then the fixes, then the run. Jimmy: "Make failing
  tests for all the bugs, then try to fix all of them, then run the tests." A lowering
  gap is called a bug.
- **EP** An event field in an element pattern binds the event the description was made
  with, a `!T`, as a `$` field binds a `$T`. Claude's first ruling; Jimmy: "your rulings
  seem right". Where: Type System chapters 2 and 4.
- **FM** `for &v` over a map is legal and visits each value by reference in key order.
  Claude's first ruling; Jimmy: "your rulings seem right". Where: Semantics chapter 5.
- **SC** A supplier written by clauses refines its output at a clause's tag pattern as a
  `match` arm does: `fn @Supply p, (Pad.Num(..)) = @ 7` checks. The checker refusing it is
  a bug. Claude's first ruling; Jimmy: "your rulings seem right". Where: Type System
  chapter 5, "Refinement at tag matches".
- **IE** An `if` needs its `else`, a building `if` included: `if loud:` and a block with
  no `else` is a checker error. Jimmy: "no helll no if needs else". Refused: an else-less
  building `if` that builds its arm or nothing.
