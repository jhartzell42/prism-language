# The prompt for the cloud session that finishes this branch

*Written by Claude on the claude-self-hosting branch; subject to change. Jimmy asked for
it on 6 November 2026 to move the work from his Mac to a cloud session. Delete this file
in the last commit of the handoff.*

You are taking over Jimmy Hartzell's (jhartzell42) Prism work from a session that ran on
his Mac. The goal is to finish the branch and hand the repository to him. Work in
`jhartzell42/prism-language` on branch `claude-self-hosting` (draft PR #20, "do not
merge"). The language's books are in `jhartzell42/prism-notes`, branch `main`, under
`claude/prism/`. Read `claude/handoff.md`, `claude/README.md`, `prismc/README.md`,
`tests/README.md`, `bootstrap/README.md`, `claude/incremental-host-document.md` and every
file in `claude/bugs/` before changing anything.

## Where the branch stands

- `prismc` is a self-hosting compiler written in Prism that emits Rust and C++. Both
  bootstraps reach a fixed point; the C++ compiler emits Rust byte-equal to the Rust one's.
- The tip was green on the Mac for: both bootstraps, `cargo test --workspace --release`,
  the three C++ `make test`s, `tests/compiled/run.sh` on both targets, the checker, the
  driver, the cost tests on both targets, `./prism check --unused prismc/`. The sanitizer
  runs and `bench/gui/prism/check.sh` were last run one merge earlier.
- Landed today: the host document as `$(Path ==> $Elem c)`; damage painting in both
  canvas Worlds; the chess interface (settings dialog, move list, Back, Forward, Reset);
  `@PerKey: fn $(k ==> v) -> @$(k ==> $v) where v: ^Eq`; no transcript over 10 KB in the
  repository (chess's board and window transcripts are deliberately absent).

## Benchmarks and this machine

It is Linux with no display. Jimmy: "we have Linux-side support for some benchmarks, tell
it to figure out how to run at least the compilation time benchmarks."

- **The building benchmark runs here, and you run it.** Read `bench/README.md` and
  `bench/building/README.md`, work out what `bench/building/run.sh` needs on Linux, fix
  what stops it (root cause, in the script), and run it for both shipping configurations
  (cargo release, clang -O3) with nothing else building. Commit its `RESULTS.md` with the
  machine, the core count and the load named in it.
- **Then the others, as far as they go.** `bench/running/run.sh`, `tests/cost/run.sh` and
  `bench/gui/prism/check.sh` read macOS's `/usr/bin/time -l` for instructions and peak
  memory. Linux has the same facts (`/usr/bin/time -v` for memory, `perf stat` for
  instructions where the machine allows it). `bench/gui/prism/Makefile` already has an
  X11 branch, and `bench/gui/run.sh` already reads `/proc/loadavg`. Find out what each
  needs, make the scripts choose by platform without changing what they do on macOS, and
  run what then runs (the GUI ones under Xvfb). Where a script cannot run here, say
  exactly what stopped it.
- **Label every number with its machine.** Numbers from here are not comparable with the
  Mac's below; keep the two sets apart, each with its units and what it is compared to.
  Anything you could not run goes in the report as "to rerun on the Mac".
- The canvas Worlds' Linux (X11) paths, damage painting on Linux, and on-screen colours
  after the sRGB change have never been checked. If you can build and run them headless
  (Xvfb), do, and report; otherwise list them as unverified.
- `tests/driver/run.sh` and the C++ suites need `clang++` and `libre2-dev`; the sanitizer
  runs need clang's runtime. Install what is missing; a missing tool is a hard error to
  report, never a skipped test passed off as green.

## Work owed, in order

### 1. A `for` over a map of dynamics binds `$v` (Jimmy's ruling FV, RULINGS round 9)

Today `for k, $$d <- $each:` over `$(k ==> $v)` binds a dynamic of a dynamic and every
row in the tree writes `$v <- @Flatten $$d` on its next line and uses `$$d` for nothing
else. Jimmy: "Do you need `$$k`?" then "Yes, bind $v".

- **The form:** `for k, $v <- $each:` over `$(k ==> $v)` binds `$v` directly, meaning
  exactly today's two lines: the value of whatever dynamic is under the row's key,
  followed if the outer later holds another one there. `for k, $$d <- ...` is refused by
  the checker with an error that says to write `$v` (a `neg` test). Convert every use:
  `examples/chess/window/widgets.prism` (twice), `bench/gui/prism/app.prism`,
  `tests/conformance/sem-ch09-host-doc-in-tick`, `sem-ch09-host-updated`,
  `sem-appB-per-key`, `sem-ch04-bind-read-above`, `tests/cost/per-key-*`,
  `tests/checker/pos/bind-read-above.prism`, and the comments in `prelude/prelude.prism`.
  `@Flatten` stays for `rebuild` results and other `$$a` values.
- **The cost:** today every row is woken on every write of the outer to ask whether its
  dynamic is still the same one. Instead the `for`'s builder, which already walks the
  keys on such a write, notices a replaced dynamic under a kept key in that walk and
  tells only that row. This holds for every map of dynamics. Nothing in the compiler or
  runtime may know about `@PerKey` specially: Jimmy rejected that.
- **Forbidden:** rebuilding the row when its dynamic is replaced. That would let a
  program tell whether two dynamics are the same one (Semantics ch. 11). The row keeps
  its holds and paths and follows.
- **Timing:** keep `@Flatten`'s timing exactly (ch. 4). A select of the benchmark table
  is nine ticks. Transcripts of converted tests should not move beyond what the
  conversion itself changes; say what moved and why.
- **First, confirm in the compiler source that the key walk can carry the check.** If it
  cannot, write up what you found with a recommendation and stop that item.
- Failing tests first: a conformance test with an outer that replaces the dynamic under
  a kept key, and a cost pair (one key removed from 2,000 costs no wake per kept row;
  run it here if the cost script runs here; if not, commit it and say it is unrun).
- Then the books: Semantics ch. 4 (the `for` forms), ch. 9 (the flattening `for`),
  appendix B (`@PerKey`), Platform ch. 3, and close FV in RULINGS.

### 2. Small owed items

- `examples/chess/window/hosting.prism`: its header still says an unrecognised element
  is "laid out as nothing (README, doubt CL)". The book's rule (Semantics ch. 9) is: a
  recognised element is swallowed, an unrecognised one is passed up or refused. Reword;
  CL is withdrawn.
- `claude/incremental-host-document.md`: bring to Jimmy's rulings. WD: a foreign World
  has no dynamics and reads the document flattened through; a Prism World is a host and
  holds the nested dynamics. MK, EQ, MW ruled correct. CL withdrawn. EP (each entry
  published per place) is still Claude's pick, his to overrule.
- `bench/building/run.sh`: the "emitted lines" count for prismc is wrong (it printed
  7,080; the Rust fixed point is about 115,000 lines). Fix the count, and add emit lines
  per second and a second-build time that reuses the runtime library, then run it
  (see "Benchmarks and this machine").
- `claude/todo-for-jimmy.md` is stale (it names files that no longer exist). Rewrite it
  against the tree as it is.
- `claude/benchmarks-preliminary.md`: fold in the building numbers and the table in item 3
  below (they are recorded nowhere else in the tree), each with units and what it is
  compared to.

### 3. The recommendation Jimmy asked for: Rust or C++ as the default back end

Write it in `claude/backend.md` from the recorded numbers (Mac M2 Pro, Low Power Mode
on, so absolute times are pessimistic; ratios are the evidence):

- Building prismc from clean: Rust emit 65.9 s + build 100.1 s = 166.0 s, 25.9 MB
  binary. C++ emit 86.2 s + build 134.6 s = 220.8 s, 63.8 MB. Chess: 19.5 s against
  23.2 s. The table: 19.7 s against 23.5 s.
- The GUI table, median wall time in ms over 10 runs (before `@PerKey`):

  | Operation | Prism C++ | Prism Rust | GTK 3 |
  |---|---|---|---|
  | create 1,000 rows | 50 | 44 | 279 |
  | replace 1,000 rows | 67 | 60 | 320 |
  | update every 10th | 5.4 | 7.1 | 34.5 |
  | select a row | 3.1 | 4.8 | 1.0 |
  | swap two rows | 3.4 | 4.8 | 61 |
  | remove a row | 12.2 | 10.2 | 5.9 |
  | create 10,000 rows | 585 | 514 | 11,659 |
  | append 1,000 rows | 59 | 48 | 390 |
  | clear | 20 | 17 | 42 |

- Instructions for a select after `@PerKey`: C++ 20.5 million, Rust 30.5 million; a
  remove: C++ 128 million, Rust 80 million (C++'s persistent map copies a path per
  insert).
- Give one recommendation with its reasons and what would change it. Cuis Smalltalk is
  out of the scripted runs (Jimmy); do not bring it back.

### 4. The pull request and the handoff

- Rewrite PR #20's description to cover the whole branch: the compiler and its two
  targets, the bootstraps and seeds, modules, the regex lexer, arrays, the runtimes, the
  Worlds and damage painting, the host document type, `@Distinct`, `@PerKey`, the `for`
  change, the chess interface, the test suites, the benchmarks with the table above.
  Then every open doubt and gap, each with whose pick it is and a recommendation, and
  everything unverified (the Linux paths, colours after sRGB, what was not rerun here).
  End the description with: `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.
  Keep it a draft titled "do not merge". Never enable auto-merge.
- Open doubts to carry, at least: EP (entry published per place); SL (`Show (? 3)`
  lifting); ET, BT, IX, SH, CT, RF, MT, PB in the READMEs; the chess doubts (slider
  drag DG, 13 ticks per press HT, transcript size TS, notation NT); remove still
  rebuilding the row list (C++ three times Rust); prismc's emit time growing faster
  than its input (about 500 lines a second on itself against 1,900 on small programs,
  cause unknown); gap DG in the checker half closed; `claude/bugs/`.
- A final current-state pass over `claude/handoff.md`: what exists, how to build and
  test it from a fresh clone, what is open, what to rerun on the Mac.
- List the stale remote branches (every `origin/claude-*` except `claude-self-hosting`,
  all merged) in the report for Jimmy to delete. Do not delete remote branches yourself.
- Last commit: delete this file.

## Jimmy's rules

- **Commits:** commit green work outright; a failing test is committed before its fix;
  batch, test once, bisect on failure. No history rewrite, no force push. Fetch before
  starting and before pushing. Every commit message ends with
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` (use your own model's name)
  and `Claude-Session: ` followed by your session's URL.
- **No silent fallbacks:** a hard error and a root-cause fix. No interpreter and no
  second implementation as an oracle. A compiler bug gets a failing test and a fix, not
  a workaround in the program.
- **Never touch** `notes/`, `TODO.md`, `LANGUAGE.md`, or Jimmy's README text (append
  only). Never remove his comments. Never claim something is absent after a narrow
  search.
- **Prose:** Claude's prose lives under `claude/` or in READMEs, never in a `docs/`
  directory and never called "docs". Current-state wording, not history. Every directory
  has a README. Head each file "Written by Claude on the claude-self-hosting branch;
  subject to change." Bug reports go in `claude/bugs/`, one per bug, deleted when fixed.
- **Builds:** only `--release` and `-O3`. Reason about undefined behaviour in C++.
- **Size:** no test transcript over 10 KB (`tests/compiled/run.sh` refuses one). No
  build output in the tree. Rebuild and commit the two seed archives once, at the end of
  a compiler change, never per commit: each adds a megabyte to history.
- **Books:** edit `claude/prism/` in `prism-notes` and push to `main` fast-forward,
  merging what others pushed. Never touch
  `claude/prism/01-semantics/01-seed/VOCABULARY.md`. Record every ruling in the book's
  `RULINGS.md` with Jimmy's words. If you have no access to that repository, put the
  owed book edits in the PR description.
- **Check the books before raising a problem.** Jimmy: "You keep on rediscovering
  problems we've already litigated".
- **Subagents** do bounded pieces well; give each the rules above. Review what they
  hand back before merging it.

## How to write to Jimmy

He reads only your latest message. Every bullet starts with a bold two-letter tag, never
reused in one message. Reply to what he asked first, then work. State his question
before answering it. Raise a disagreement as a doubt, and give every doubt a
recommendation, never a bare choice. Pose a design question as: what the book requires,
how you want to do it, why. Plain English, no jargon, no made-up labels, and say
"requirement" with where it comes from. Do not imply there is no alternative. Give
numbers with units and something to compare them to. Do not stop to ask permission for
what he already asked for; if conditions are imperfect, do it, label the caveat, and
offer a rerun.

## Done means

The `for` change is merged and green on everything this machine can run; the small
items, the recommendation, the PR description and the handoff note are written and
pushed; the books match; and your last message tells Jimmy what is his now, what is
unverified, and exactly what to rerun on his Mac.
