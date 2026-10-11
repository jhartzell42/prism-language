# Handoff: the state of the Claude work on Prism, for the next session

Read this first. It is written by the session that produced everything under `claude/` (Claude
Fable 5.1, sessions ending 2026-10-11), for a session that cannot see that conversation. It says
what exists, what Jimmy's standing rules are, how to build and test, and what the next burst of
work is. Nothing here is a decision Jimmy has made unless it says so; "decided" below means he
said it in chat.

## What is where

- `claude/ch00-*.md` … `claude/ch17-*.md`: the Prism book, LLM-written, Jimmy's author credit
  revoked at his request. `claude/summaries.md` has one paragraph per chapter; read it before any
  chapter. `claude/CLAUDE.md` is the book's bible: the rules every chapter obeys (sigils, paths,
  laws, vocabulary). `claude/prism-book.pdf` is the built book; `claude/build/` has the pandoc
  template, filter, syntax file and `build.sh` (`./build.sh <dir-with-ch*.md> out.pdf`).
- `claude/prism-hs/`: a Haskell prototype of chapter 2's subset: parser, evaluator on a
  hand-rolled push graph, Test World, 53 passing scripts. `NOTES.md` is the build log, `TODO.md`
  the list of corners cut and the type checker's open questions, `test.sh` runs everything.
- The Rust runtime at the repo root is Jimmy's, with the on-fire fixes from this session already
  in it (occurrence state in `RuntimeInner`, `Blueprint`/`Widget`/`Rebuild<T>`/`DynBlueprint`
  renames, 37 tests). Doc comments there are in Jimmy's voice; do not rewrite them.
- Branch `claude` is PR #2, "claude work". PRs #3–#9 are one book fix each, based on `claude`,
  found by the prototype; Jimmy reviews and rewrites them himself, so do not merge them.
- Not in the repo: `/home/claude/book/` (the session's working copy of the book with `sync.sh`,
  the essay stage files under `essay-purity/`, lint and steelman files under `lint/`), and
  `/home/claude/jimmy-log/` (Jimmy's private calendar and notes repo). A new session on another
  machine has only what is in this repo.

## Jimmy's standing rules (decided, in chat)

- Answer his questions first, with your own take, then do repo work. He said "RESPOND WORK
  RESPOND RESPOND WORK RESPOND" and meant it.
- Put a distinct two-letter code on every bullet in a reply, including change reports, so he can
  answer by code; never reuse a code in one message; restate a proposal's context when referring
  back to it. Change reports are bullet points.
- The book argues every choice as "this is cool, and here's why". No status vocabulary:
  "settled", "open", "proposal", "future work", "TBD". No em dashes. No "genuinely", "honestly",
  "straightforward". Do not fix his grammar into slop ("They aren't" stays).
- Doc comments in the Rust runtime are his voice. If you draft one, show the old comment and
  give yours in German (and Spanish) only, never English; edit his for grammar only.
- "Nothing reaches past an interceptor unless Prism code sends it" is the whole sandbox law; a
  scoped intercept is sugar that obeys it. Never weaken it.
- Hold needs a widget for two reasons, birth (same-occurrence catch-up) and rollback (the
  journal); not for death.
- Vocabulary: blueprint = value of type `@a`; widget = the built thing; node = runtime struct;
  World = the external context; interceptor = an inner context. Hold keyword is `then`. `rebuild`
  (not `dyn`). Sigils are functors: `!` event, `$` dynamic, `@` widget monad, `?` optional. `~T`
  for faults is his idea, not designed.
- Rebuild the PDF and push on every book change; attach the PDF to him when a turn ends.
- Persuasive essays go through the staged workflow (`anthropic-skills:staged-persuasive-essay`),
  objections inside the text, no appended caveats; stage files are thinking aids, not for the
  repo.
- Do not vibe-code in his repo. The Haskell work lives under `claude/`; the Rust runtime is his.
- His own plan (TODO.md on `main`): read the book once as a resource, write his designs under
  `design/`, not write notes to Claude.

## How to build and test

Prototype, on any machine with GHC 9.4 or later (no packages beyond base, containers, mtl):

    cd claude/prism-hs
    ./test.sh                       # builds with ghc --make, runs each example file against the prelude
    ./prism-hs -v examples/prelude.prism examples/counter.prism   # -v prints the transcript

With Hackage reachable, `prism-hs.cabal` is there but unused; `cabal build` should work with a
`build-depends` of base, containers, mtl. Example files share one namespace, so run them one
at a time (chapter 2's programs reuse `Clock` and `Show`).

Book: `cd claude/build && ./build.sh .. ../prism-book.pdf` (pandoc + xelatex; the session had
them in its container). Commit the PDF with the chapters.

Rust runtime: `cargo test` at the root; 37 tests.

## What the prototype is and is not

It is an independent reading of chapters 2, 4 and 10 on a Spider-shaped graph, not chapter 14's
"interpret on Reflex" (Hackage was unreachable from the session's container). It has no type
checker, no `intercept`, no World but Test, no prompt-cycle check, no `update`/`&`. Chapter 17 of
the book is the report; `prism-hs/TODO.md` is the full list with the type checker's open
questions. Everything in `TODO.md` under "corners cut" makes a passing test conditional.

## The next burst (Jimmy's goal: a runnable chess demo on reflex-dom, plus a folder of test cases)

Decided direction: port the evaluator to Reflex first, type checker second. Reasons: the port fits
one burst (parser, Test World, examples and scripts carry over; replace `src/Prism/Runtime.hs`
and `src/Prism/Eval.hs`, about 1,100 lines), and it answers chapter 14's claim; typing is a
design job before a coding job (TODO.md, last section).

Steps:

1. On a machine with Hackage: clone this repo, `cd claude/prism-hs`, run `./test.sh`, confirm 53
   pass. If GHC is older than 9.4, the code uses nothing newer than `LambdaCase`.
2. Reflex port: keep `Syntax.hs`, `Lexer.hs`, `Parser.hs`, `Value.hs` (replace `Ev`/`Dyn`/`Node`
   with `Event t`/`Dynamic t` in a `PrismM t m` monad), `World.hs` (its `runTurn` becomes
   `fireEventsAndRead` over trigger refs, chapter 7's one run call), `Test.hs`, `Main.hs`. Rewrite
   `Eval.hs` on `MonadHold`, `Adjustable` (`runWithReplace` for `rebuild`, `traverseIntMapWithKeyWithAdjust`
   or `listHoldWithKey` for `for`), `switchDyn` for `!Switch` (never the prompt forms), `join` for
   `$Join`. Chapter 14 and appendix A are the map. Get the 53 scripts green; the transcripts are
   the oracle, character for character.
3. A `Dom` World over reflex-dom: elements to DOM (`Ui.Button`/`Label`/`Focusable`, plus an
   `Ui.Image(src, bg)` for chess), clicks and keys back as responses by path, state elements from
   a snapshot per frame. Run it with `jsaddle-warp` for a native build served to a browser, or
   inside Jimmy's existing Obelisk setup (see below); do not start with GHCJS.
4. Chess: Jimmy's `jhartzell42/reflex-chess` (Obelisk app; `common/src/Common/Chess.hs` is the
   272-line pure rule engine, `frontend/src/Frontend/Chess.hs` the 170-line UI). The rule engine
   ports to plain Prism functions mechanically (records, choices, lists, `match`). The UI is 64
   `Ui.Image` elements in a `for` over the board, a hold for the selected square, a hold for the
   game state fed by `move` on click, and labels; it uses `attachWith` on `current`, `foldDyn` and
   `rec`, all of which are non-prompt and map directly onto tags and `then`. Since the app is
   already Obelisk-built, the Prism interpreter plus the Dom World can live in its `frontend`
   package and `ob run` serves it, which sidesteps the toolchain problem entirely.
5. Test cases: chapter 10's scripts are the suite; add a `typecheck` form (a file expected to fail
   with a named error) and write the twelve pin files listed in `TODO.md` before the checker,
   so the checker is built to them.

What not to do: do not rebuild the push graph; do not use `switchPromptlyDyn`, `coincidence`,
`sample` or `performEvent` inside the interpreter (chapter 14 "What must not be used"); do not
touch the Rust runtime for this.

## Open questions only Jimmy can answer

- The chapter 1 steelman's points 3–5 (piecewise-constant chosen for implementability; the
  relabelling button losing a fast second click and focus; Safe Haskell as a counterexample to
  "it had to be a language"). The file is not in the repo; ask him for it or redo it.
- Whether `click at path` (PR #7) and `torn down:` (PR #6) are the spellings he wants.
- `~T` for faults, fault policy by contexts, and whether `update`/`&` stay in chapter 3.
