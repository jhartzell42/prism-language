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
