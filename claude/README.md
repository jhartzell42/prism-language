# claude

Everything here is written by Claude, not by Jimmy: notes for the work on this branch,
not documentation he has signed off on. Claude-written code can sit beside his code;
Claude-written prose lives here, under this name, and nowhere else.

- **`architecture.md`**: the pieces of this repository and how a program flows through
  them.
- **`backend.md`**: the contract between `prismc`'s emitted code and the runtimes.
- **`benchmarks-preliminary.md`**: the numbers of 5 November on a busy machine: building,
  running prismc, the GUI table.
- **`bugs/`**: bugs in prismc, investigated: the cause, the books' rule, the fix.
- **`decisions.md`**: the choices the books leave to this implementation, with reasons.
- **`handoff.md`**: everything still to do on the self-hosting branch, in order.
- **`incremental-host-document.md`**: what one select in the GUI table recomputes, tick by
  tick, and the routes to a small change costing a small amount: an incremental host
  document, per-child layout, a cached call, nested documents settled in one tick.
- **`regex-dialect.md`**: the regular-expression dialect as one procedure: every
  construct accepted or refused, each diagnostic's position and message, how each runtime
  spells a pattern to its library, and the doubts.
- **`modules-style-pass.md`**: modules nested as Rust's (`mod Name`: `name.prism`, or
  `name/mod.prism` with children), test modules as child modules (`test mod Tests`), and
  prismc's sources in that tree.
- **`todo-for-jimmy.md`**: Jimmy's list: things to try, making the code his, a shell
  World.
- **`test-world-in-prism.md`**: why the Test World is not a Prism program yet, and what
  would let it be.

The language's design is in the books of `jhartzell42/prism-notes`: a curated set of design prompts, not a specification.
