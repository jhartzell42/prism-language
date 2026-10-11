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
