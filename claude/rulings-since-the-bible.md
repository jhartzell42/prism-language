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
