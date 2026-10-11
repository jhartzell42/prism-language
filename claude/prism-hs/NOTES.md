# Notes from building the Haskell prototype

Started 2026-10-11 07:00. Jimmy asleep. Local repo only.

- Hackage and ghcup are blocked by the proxy; Ubuntu's apt has GHC 9.4.7 and cabal 3.8 but no `reflex`.
  So the evaluator runs on a hand-rolled push graph in IO, built after Spider's shape (heights, one
  occurrence at a time, holds commit at the end). Chapter 14's "interpret on Reflex" is not what this is;
  it is closer to a second runtime in Haskell. The differential-test value is the same.
- 07:25 The counter passes with chapter 2's transcript, character for character, including the
  `2/#1/0` generation bump on the label. First build of the parser, evaluator and world compiled
  on the first try; the one bug was mine: application was parsed only at the top of an expression,
  so `@ !Switch !!clicks` read as `@ !Switch` followed by a new statement `!!clicks`. Application is
  now a postfix on an applicable head, which is what the book's "a paren-less call takes the rest of
  its expression" means once you try to write the grammar.
- Hold initial values are evaluated in a pre-pass before the block's other statements, so a hold
  can be referenced before its line (todos: `for $items` above `$items <- {} then ...`). The book
  says the resolver's binding groups handle this; the pre-pass is the cheap version. Limit: a
  hold's initial value cannot mention a non-hold block-local binding defined later.
- 07:50 Clicker, stopwatch (with `with`/`push` on a state element and an `errors` line for the
  unsupplied clock), todos (`for`, `!Union`, `Fold`, a self-referential hold through a `for`), and
  the viewer (`@started`, a request element, a state element under `rebuild`, `$Join` of a hold of
  dynamics, `@TextInput`) all pass.
- Construction in the middle of an occurrence was the hard part, and the book's rule ("built in n
  sees all of n, once") is right but says nothing about how. What made it work: (1) while nothing
  is propagating, a `fire` is queued and applied when propagation starts, so the whole build
  occurrence is built before anything fires, which is Spider's post-build frame; (2) `rebuild` and
  `for` are nodes that build their first generation when they first *run*, in height order, so a
  rebuild over a dynamic that changes in its build occurrence builds once, with the new value;
  (3) a derived dynamic or a `$Join` made mid-occurrence over a source that already changed is
  born with the new value and does not fire its change again. Reflex gets (2) from `networkView`'s
  `leftmost [updated d, tag (current d) postBuild]`; the book should say that this is the mechanism
  behind its sentence.
- Paths: chapter 2's counter prints `0/#0/0` for a `@Button` (the application adds no segment), but
  its viewer prints `1: Fs.File` (so `@started` took position 0) and `3/0`, `3/1/#0/0` for the parts
  of `@TextInput` (so the application took a position and its parts numbered under it). Those are
  two different rules. I follow the counter and the CLAUDE.md rule (applications are transparent),
  so my viewer prints `0: Fs.File`, `2: Ui.Focusable`, `3/#0/0: Ui.Label`. One of the two listings
  in the book is wrong; the regular rule (application takes a position) is the one I'd pick for the
  spec, because a function's insides then can't renumber its caller's slots.
- Names are keyed by sigil plus base: `!motd` and `$motd` are different names (the viewer has both),
  and the pronoun `_` resolves across sigils. My first version keyed by base alone and the viewer's
  label showed an event.
- The test language cannot address a slot by path, only by pattern, so two rows with identical
  buttons cannot be clicked individually (`click Ui.Button(label="Star")` with two rows is a script
  error, as chapter 10 says). Every todo test in the book has to star or toggle before adding the
  second row. A `click at 1/[1]/2/#0/0` form, or a key in the pattern, would fix it.
- `then` as hold and `on`/`|` as the rest of an application's argument parse cleanly; the only
  precedence surprise was mine (application at the top of an expression instead of as a postfix).
- Double-paren tuples collide with nested parentheses: `requests=(Read on !started))` lexes as a
  tuple close unless the lexer keeps a bracket stack. It does now. The book's lexer section should
  say that `))` closes a tuple only when a `((` is open.
- The equality-not-deduplicated rule shows: a row's `$_` fires on every change to the map, so every
  row's label rebuilds on every add (`1/[0]/3/#3/0` after one star, one toggle, one add). Chapter
  2's listing shows the same thing, so the prototype agrees with the book; it is the perf item.
- 08:10 Chapter 4's 14 law tests and chapter 10's 28 (events, dynamics, time, widgets, switching)
  all pass, ported by Sonnet agents from the book's own scripts. Fixes they found: a `with` on a
  turn line is that turn's push for the element (chapter 10 says so; I had only pushed for `push`);
  check patterns must not take commas (`requested Echo, ignore` read `Echo, ignore` as one
  constructor pattern); `for` over an array is a map keyed by index; a `for` whose rows are plain
  values consolidates to a dynamic map; the consolidated dynamic has to run after the for node so
  births are in it.
- Book bugs the ports hit: `(Insert $keys, Size $keys, unit)` in chapter 10 parses as
  `Insert($keys, Size($keys, unit))` under the book's own rule that a paren-less call takes the
  rest of its expression, commas included; the test needs `(Size $keys)`. And `((Insert ..) on !add)`
  lexes as a tuple open: `((` is ambiguous with "a parenthesised expression that starts with a
  parenthesis". The double-paren tuple fails the book's own rule 13 (a loss caused by a token
  collision); the lexer here resolves `))` with a bracket stack but cannot resolve `((`.
- 08:40 Chapter 2's editor, chat and CLI programs pass (53 tests in all now). What the chat program
  needed: tuple-pattern binds `((!rx, $link)) <- @Element Net.Socket(..)` with an element that
  has both `responds` and `state` returning the pair; forward *dynamics* for a `$` name bound later
  in the block (the socket's `$link` is read, through a tag, by the `requests` expression of the
  very element that defines it: a cycle the book allows and RecursiveDo would give for free);
  tuples and arrays as plain containers (an array of events is plain, chapter 3 says so; a tuple
  is a bundle), while map literals lift (todos' `{$next: ..} on !add` needs it). That last split is a
  dynamic-typing stand-in for what the type checker decides from the expected type; it will be
  wrong for an array literal that is meant to be a lifted array of plain values.
- Requests fired by a slot torn down in the same turn (the chat's send-and-change-address in one
  occurrence) are reported too, as `(torn down)` lines, since the World has to perform them; the
  chapter 10 transcript format has no place for them and should.
- Char literals: the book writes `'\n'`; the prototype lexes a char as a one-character text.
- Recursion at top level needed every definition to be a thunk over the final environment; a
  function could not call itself in the two-pass version. Same answer as RecursiveDo, again.
- Things the agents reached for that the prelude lacked, now built in: JoinText, Count, Prefix,
  Suffix, Split, Reverse, First/Second/Third on tuples, First/Last on arrays, Upper/Lower,
  Prepend, Take/Drop/DropLast. The book's prelude list (chapter 8's assumption) needs writing down.
- 08:10 Stopped here. 53 tests across 10 example files pass (`./test.sh`, one file at a time: the top level is one namespace, and chapter 2's programs reuse names like `Clock` and `Show`, so loading all files at once makes three tests fail on the wrong definition). Left as is: PRISM_TRACE debugging in Runtime.hs; the unused cabal file (Hackage is unreachable from this machine, so the build is `ghc --make` over the apt GHC 9.4.7); dynamic typing throughout, so the places where the book's type checker decides a lift (array literals, result kinds of `for`) are guesses from the last statement. The experience report sent to Jimmy summarises the book findings above.

## The Reflex port (branch `claude-reflex`, 2026-10-11, afternoon)

A second session, on Jimmy's Mac with Hackage reachable. GHC 9.10.3 and cabal 3.16.1 through
ghcup; `reflex` 0.9.4.1 from Hackage. `Runtime.hs` is gone; `Eval.hs` and `World.hs` are rewritten
on Reflex, `Value.hs` holds `Event t`/`Dynamic t` values; parser, lexer, syntax, `Test.hs`,
`Main.hs`, the examples and `test.sh` are as they were. The 53 scripts pass in 1.4 s, and the
transcripts are character for character those of the push-graph prototype, with two exceptions
noted at the end.

- 12:50 `./test.sh` on the old prototype, GHC 9.10.3: 53 pass, with `-Wx-partial` warnings on
  `head`. The branch is `claude-reflex`, not `claude/reflex`: git refuses a branch `claude/reflex`
  while a branch `claude` exists (one is a file, the other a directory under `refs/heads`).
- 13:05 The shape of the port, decided from the Spider source before writing code. Chapter 14
  puts `PerformEventT` at the bottom of the stack "for its `Adjustable` instance" and says a
  `rebuild` built in n sees n through `subscribeAndRead`. The instance does not do that:
  `PerformEventT`'s `runWithReplace` is `requestingIdentity` over the replacement event, and
  `hostPerformEventT` performs the requested builds between frames and fires their results as a
  follow-up frame. A block replaced in n would be built in n+1, where the event that caused it
  is not firing, and `a_new_block_sees_the_event_that_made_it`, `a_parent_sees_a_new_key_at_once`
  and `a_nested_dyn_builds_once_with_the_new_value` would fail. There is no `Adjustable` instance
  for `SpiderHostFrame` or `PushM`. So the evaluator uses no `Adjustable` at all: a `rebuild`
  is `push build (leftmost [updated d, tag (current d) now])`, `networkView`'s trigger with the
  block built inside the push closure, in the frame of the change; the generation's list is a
  `holdDyn` of the generations' list dynamics, joined; its requests are `switch (current held)`
  for the members as the occurrence opened plus one `coincidence` for the newborn. The root is
  the same thing over the build trigger. `for` is the same with a map of rows diffed by key
  inside the push, `joinDynThroughMap` for the list and for `$`/plain results, `mergeMap` under
  `switch` for event results. `PushM` has `MonadHold`, `MonadFix`, `MonadIO` and `now`, which is
  all a block needs. This is a finding for chapter 14: the stack it describes would not pass the
  chapter 10 laws on Reflex 0.9.4.1.
- 13:20 Forward references are `mdo` over the block's environment, as chapter 14 says, with one
  wrinkle. The evaluator is dynamically typed, so it pattern-matches a value to decide whether
  to lift; a name bound later in the block has to have a known constructor before its line.
  The pre-pass binds each later `!` name to `VEvent (switch (constant real))` and each `$` name
  to `VDyn (join (constDyn real))`, with `real` the knot. The wrapper matters: `leftmost` is
  `mergeInt` over a strict `IntMap`, which forces each event to weak head normal form, and a bare
  knot thunk there is `<<loop>>`; a `switch` over a constant behavior is already a value and
  Spider reads the behavior only when it subscribes, after the block has returned. Plain names
  bound later are the bare thunk; forcing one at build time is `FixIOException`, reported as
  `Cycle`.
- 13:25 Lifting is pure. One event and some dynamics is `attachWith` over the dynamics'
  behaviors (the tag law); dynamics only is `distributeListOverDynWith`; both take a closure
  that runs the evaluator in `Identity`, where a hold or a `!Switch` is an error. The evaluator
  is written once over a `Build t m` dictionary that says how a statement reaches the push
  monad. `!Switch` is `switchHold never`, `$Join` is `join`, `!Updated` is `updated`,
  `$Distinct` is `holdUniqDyn` (the prototype returned the dynamic unchanged), `@started` is `now`.
- 13:30 First full build: counter, clicker, editor, viewer, chat, cli, stopwatch and all 28
  chapter 10 laws pass at once. Two files hang.
- 13:40 Both hangs are Spider recursing without bound while subscribing a prompt cycle. The todo
  list was mine: the `for` birth term (`coincidence` over the patch event) makes the
  consolidated event prompt on the collection's change, and the collection is a hold fed by the
  consolidated event. Chapter 14 says the elaborator leaves the birth term out when the block
  cannot fire at birth; without an elaborator, `canFireAtBirth` reads it off the syntax (the
  block mentions `@started`, a `rebuild`, `!Updated`, or an event bound outside it). The other
  hang is `cycle_is_an_error`, which is meant to hang: `!out <- @Echo !out`. Chapter 14 says
  Spider throws `EventLoopException` for a prompt cycle; a standalone experiment
  (`rec e = leftmost [fmap (+1) (switch (constant e)), 0 <$ now]`, held and fired) shows it
  does not, it recurses until the stack is gone. The binary runs with `-K64m` and a
  `StackOverflow` escaping the frame is reported as `Cycle`. That is a bounded-stack detector,
  not a check; the check is the type checker's (TODO.md). Named `!` bindings are pinned with a
  throwaway `hold` at the end of a block so that an unused cyclic event is subscribed at all,
  since Spider builds nothing nobody subscribes to.
- 13:45 Every test runs on its own timeline (`withSpiderTimeline`), so an aborted frame cannot
  leak into the next test. After an error the timeline is not trusted and the test ends;
  chapter 14's rollback by replay is not written, and no script needs it.
- 13:50 Transcript diff against the old binary, all ten files: identical except two lines.
  `cycle_is_an_error` prints a different `detail` (the prototype named its own graph node).
  `state_survives_a_neighbours_removal`, turn 2, result: the prototype printed
  `{"a": unit, "b": unit}` after dropping `a`, because its consolidated dynamic ran before the
  row was dropped at commit; this build prints `{"b": unit}`, which is chapter 14's rule for
  `for` with plain results (changes promptly when a key disappears). The old line was a bug.
- 14:30 `test/Chapter14Claims.hs`, a cabal test-suite `chapter14-claims` that `test.sh` does not
  run: chapter 14's two claims about Reflex, measured, plus the control the port relies on.
  Claim 1 (`runWithReplace` under `PerformEventT` builds in the frame of the change): the run
  call takes 2 frames and the newborn's hold over the click reads 0. Claim 2 (Spider throws
  `EventLoopException` on a prompt cycle): it raises a stack overflow under `-K16m`. Control (the
  block built inside `push`): one frame, the newborn reads 1. The suite fails on purpose; it is
  the record for the book.
