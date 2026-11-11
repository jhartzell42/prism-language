# The refusals of prismc's back end

Written by Claude; subject to change. Every `Refusal` / `RefusalDo` message in
prismc/lower and prismc/emit, by file and line, with its class. Classified by reading; an
entry marked "unsure" was not tested.

- **G**: a guard no checked program reaches. It becomes an internal error, "a prismc
  bug: ..".
- **R**: a legal program the back end does not lower. It is lowered, failing test first.
- **C**: a wrong program. It becomes a checker error with a diagnostic, failing test first.

Besides these, `OutRefused` (a type the lowering does not spell) stands at 15 places in
lower/expr.prism, model.prism, patterns.prism and types.prism: twelve that make one, the
type's definition in model.prism and two matches on it. Two are reached (see R, Functions:
a generic `let` nothing reads, and a field of an `Error`); the others are under G, "Types the lowering does not
spell".

## R: legal programs not lowered

Hosting
- lower/expr.prism:658 `@ x` as a supplier's answer outside a tag arm. Program: a host's
  `let @seven = @ 7`, given by the arm that matched the tag. A supplier `\|p, e| @ 1`
  with no `match e` is a checker error (its `o` is rank-2). Test:
  sem-ch09-host-answer-without-arm.
- lower/expr.prism:716 a supplier's dynamic answer that has no value at the owner's
  build. Program: `@ $~real` where `$~real` is the host's own pushed place, and `@ $~d`
  where `$~d` is a `let` below the host's line. Test: sem-ch09-host-dynamic-answer-late.
- lower/expr.prism:696 a bundle answer with a `$` part or two event parts. Program:
  Semantics ch. 9's `@ ((never, $ "offline"))`. Test: sem-ch09-bundle-dynamic-part.
- lower/expr.prism:697 a bundle answer not written as a tuple. Program: `@ pair` under a
  bundle tag's arm, `pair` a `let`. The refusal is not reached: `pair` is lowered as a
  plain answer and rustc refuses the crate. Test: sem-ch09-host-bundle-answer-named.
- lower/node/static.prism:560 `@Element` in a block of an element value that is not a
  tag applied in place. Program: `let e = Ui.Label(..)`, then `@Element e` as a line.
  Test: sem-ch08-element-value-statement.
- lower/node/elements.prism:182 an element tag whose bundle output has a `$` part or two
  event parts. Program: a tag declared `((!Text, $Text))` placed in a block. Test:
  sem-ch09-bundle-dynamic-part.
- DQ (no refusal; a fault at the build, `Runtime.Answer`): an element value with an event
  field placed by a host that is itself hosted. Test: sem-ch09-host-nested-request.
- A tag's answer constructor as a function value (`Each cs, Ui.Button.Answer`): no
  refusal; rustc refuses the crate. Test: sem-ch09-answer-constructor-value.
- A widget function passed as an argument and applied (`fn @Frame Inner:` with the line
  `Inner 1`): refused as "the widget function @Row as a value" and at
  lower/node/static.prism:121. Test: sem-ch08-widget-parameter-applied.

- lower/expr.prism:764 an event read as a value. Program: a widget function's event
  parameter read in its supplier's arm, `fn @HostPlayer side, @guest, $state, !outcome:`
  with the arm `Play.Request(..): @ !Filter (?ResponseTo side, p, !outcome)`. Test:
  sem-ch09-hosting-per-role-event-parameter (hosting attempt 09 of the Bible's evidence,
  as written).
- lower/node/static.prism:651 a generic widget at a call whose types its parameters' do
  not match. Program: a widget function with a bounded type variable that also takes a
  blueprint `@a`, `@Framed: fn s, @a -> @a where s: ^Show` called as `@Framed 3, (@Guest
  "one")`; `a` is left unmatched. With the bound alone (`fn s, Text`) or the blueprint
  alone (`fn Nat, @a`) the call is lowered. Test:
  sem-ch08-generic-widget-blueprint-parameter. Hosting attempt 14 reaches it too, and is
  a wrong program for another reason: see C, a tag field's type variable.
- lower/patterns.prism:268 (see Dynamics and events) stops a host hearing a guest's
  request: a supplier's arm `Seat.Request(!requests=!r): @ "ok {!r}"`
  (sem-ch09-hosting-host-hears-request) and the host's body reading each entry of its
  document (sem-ch09-hosting-body-reads-request, which meets gap LF at `$evs` too);
  hosting attempts 01 and 02, as written.

Bundles
- lower/node/keyed.prism:165 a `for` whose rows end on a bundle. Test:
  sem-ch04-for-bundle-result.
- lower/node/switch.prism:488 a widget of several clauses whose clause ends on a bundle.
  Test: sem-ch08-clause-bundle-result (its caller's tuple pattern also meets
  lower/node/def.prism:687).
- lower/node/switch.prism:676 a `rebuild` whose block ends on a bundle. Refused first at
  lower/node/switch.prism:155. Test: sem-ch04-rebuild-bundle-result.
- lower/node/def.prism:695 a late plain result in a bundle. Program: a block that ends
  `@ ((n, !e))` where `n` is bound from a building `match`. Refused at
  lower/node/def.prism:219 ("a block split at a line whose result is not late") and
  lower/node/switch.prism:488. Test: sem-ch08-late-result-in-bundle.
- lower/expr.prism:765 a tuple pattern's binding as a value. Program: `both <- @counter`,
  `let same = both`, `@ same`. Test: sem-ch04-bundle-rebound.
- lower/node/def.prism:96 a bundle as the root's result. Program: `@main: @((!Nat,
  Text))` ending `@ ((($n + 1) on !go, "ready"))`; the World hears the plain part at the
  build and each firing of the event after. Test: sem-ch07-root-bundle-result.

Building `match`, clause widgets, `rebuild`
- lower/node/def.prism:553 a late plain result bound where its block is not split (LW).
  Program: `n <- @w` where `@w` is a widget parameter whose block ends late.
- lower/node/switch.prism:359 a `rebuild` whose generation is a widget of several
  clauses ending on a plain result. Program: `$n <- rebuild $x: @Pick _` with
  `fn @Pick 0: .. @ "zero"`. The refusal is not reached: rustc refuses the crate. Test:
  sem-ch04-rebuild-clause-plain-result.
- lower/node/switch.prism:155 a `rebuild` whose generation gives a dynamic it does not
  lower. Reached by a block ending on a bundle (sem-ch04-rebuild-bundle-result). A block
  ending on a constant dynamic is lowered (sem-ch04-rebuild-constant-dynamic passes).
- lower/node/switch.prism:430 a widget of several clauses given a dynamic its caller
  reads through its own parent. Program: inside a `for` row, `@Show (k == 1), $count`
  where `$count` is the block's hold. Test: sem-ch08-clause-dynamic-through-parent.
- lower/node/static.prism:79 a widget parameter applied to arguments: see Hosting, the
  widget function passed as an argument.
- A hold above a split that begins at the late result (`$gone <- n then ..` above `n <-
  match mode:`): legal, since the order of a group's lines decides positions and nothing
  else (Semantics ch. 4); refused today as "the name n, which nothing declares". Test:
  sem-ch04-building-match-hold-above.
- SP2, SP3 (tests: sem-ch04-building-match-plain-group; lines below a split reading a
  widget parameter or a local function).
- TC (tests: sem-ch08-places-counted-as-built, sem-ch04-building-match-mixed-arms), PR (a
  clause widget that calls itself).
- gap ID: a widget that calls itself deeper than sixteen levels.

Dynamics and events
- lower/node/finish.prism:518 `!Updated` of a dynamic no sink of the node writes (UP).
  Program: `$r <- @Pick 1`, the dynamic result of a widget of several clauses, then
  `!Updated $r`; a hosted child's dynamic result read in a row reaches it too. Test:
  sem-ch07-updated-clause-result. A constant `let` and a widget's dynamic parameter are
  not the case: sem-ch07-updated-constant and sem-ch07-updated-parameter pass.
- lower/node/finish.prism:528 the same in a row over a name of its block (SU). No
  program found: a row over its block's parameter passes
  (sem-ch07-updated-row-parameter), and over a clause widget's result, a global and a
  constant it compiles.
- lower/node/switch.prism:867 `@Flatten` of a dynamic of dynamics with no live dynamic
  at the build (LF). Program: `$shown <- @Flatten ($Pick $useB, $a, $b)`, the outer
  written in place. Test: sem-ch07-flatten-call. With the outer a hold bound below the
  `@Flatten` the refusal is not reached: rustc refuses the crate
  (sem-ch07-flatten-held-above).
- lower/expr.prism:354 a dynamic passed whole to a function where it has no live dynamic
  (LF). Programs: `Wrap $r` with `$r` a clause widget's result
  (ts-ch04-whole-clause-result), with `$g` a place's dynamic
  (ts-ch04-whole-place-dynamic), with a global `let $four = $ 4`
  (ts-ch04-whole-global-dynamic); a `for`'s `$v` over
  `@PerKey` and a host's `$doc` reach it too, with no test. A row passing its block's
  hold and a hosted child's result do not reach it: rustc refuses the crate
  (ts-ch04-whole-row-parent, ts-ch04-whole-hosted-result).
- lower/expr.prism:327 a dynamic a function gives whole, read in a plain function. No
  program found: `fn $Kth n = Or (Consts n)[1], ($ 0)` compiles.
- lower/node/dynamics.prism:192 `!MergeAll` of a list that is not plain: a G, the checker
  gives `two-layers` for `!MergeAll $evs` read as an event.
- lower/expr.prism:494 an event value of several events: a G, the checker gives
  `two-events`.
- lower/patterns.prism:268 an event field in an element pattern. Program:
  `match e: Pad.Bell(!rings=!r, ..): !r` over a description. Test:
  sem-ch09-element-pattern-event-field.
- lower/patterns.prism:323 a pattern inside an element's `$` field: see C
  (`Ui.Label($text=$"x")` passes the checker; `$text="x"` is a mismatch there).
- An interpolation lifted over `?` (`fn ?Greet ?name = "hello {?name}!"`): no refusal,
  rustc refuses the crate. Test: ts-ch04-interpolation-lifts-optional.

Functions
- lower/expr.prism:1893 ("a parameter") and the emitters' "a quantified type variable ..:
  no instantiation gives it a type": a generic `let` of a widget block that nothing
  reads. Program: `let Same = \|x| x`, `let empty = [[]]`, `let pair = ((nil, [[]]))`,
  `let blanks = Each [[1, 2]], ?Nothing`, none read. Test: ts-ch05-generic-let-unused.
- lower/expr.prism:1580 a field of an `Error` has no type in the back end: no refusal,
  and `"{e.message}"` prints the message as `Show` would, in quotes. Test:
  ts-ch02-field-reads.
- A fault kind as a value (`e.kind == Fs.NotFound`, a list of kinds, a record's field of
  type `Fault`, a generic call at `Fault`): no refusal; rustc refuses the crate. Test:
  sem-ch10-fault-kind-value.
- The prelude's `Turn b` built and matched by a program (`Done x`, `More`): no refusal;
  rustc refuses the crate (the runtime's `Turn` is the job's own). Test:
  sem-appB-turn-value.
- A trait method passed as a function value (`Each ns, Blank`, `Each xs, Twice`): no
  refusal; rustc refuses the crate. Test: ts-ch03-method-function-value.
- lower/expr.prism `StartsAsBounded`: a function value of its callee's own color inside
  the value a call returns, at a call that makes the color bounded, where a map, a named
  type or another function's type holds it. Through an array, an optional, a `~T` and a
  tuple it is lowered (ts-ch05-color-function-in-result). No test for the rest.
- lower/expr.prism:636 an existential of a program's own trait (`any ^Weighed`); `^A +
  ^B` is no syntax. Test: ts-ch03-existential-own-trait.
- lower/node/def.prism:687 a widget's plain result bound by a pattern: a G,
  `BindPatternResult` takes every other pattern (sem-ch08-plain-result-pattern passes).

Jobs and templates
- lower/jobs.prism:291, 292 a cycle-point `while` in another expression or statement:
  a G, no program reaches this: `StmtJob` is entered only for a `while`, `if`, `match`
  or `for` (`HoldsCycleWhile`), and a nested `update` holding the loop compiles.
- lower/templates.prism:372 a reference path this back end does not lower: a G, no
  program reaches this: the checker refuses any other left side of `<=` (mismatch) and
  a named lens (`unsupported`).
- lower/expr.prism:145 `?` of a place: a G, no program reaches this: every operand of
  `TryOf` is a call the back end itself generates.

Catch-alls
- lower/node/def.prism:579 a statement of a widget block not lowered yet. Program: a
  widget defined inside a widget block, `@line:` and its block. Test:
  sem-ch04-local-widget. Every other kind is a G: `require`, `forbid`, `while`, `<=`
  and `+=` are a checker mismatch there, `return` a parse error.
- lower/expr.prism:2048 a statement of an expression block not lowered yet: a G, the
  checker refuses a bind, `emit`, `require`, `while` there (mismatch).
- lower/node/def.prism:938 a building line not lowered yet: a G, the checker gives
  mismatch for a bare line that is no element or blueprint.
- lower/node/static.prism:121 a widget not lowered yet. Programs: a building `if` with
  no `else` (sem-ch04-building-if-no-else); a bare call of a widget defined inside the
  block (sem-ch04-local-widget); SP3.
- lower/node/static.prism:600 the builtin widget {name}. Program: `@Later !go`, the one
  prelude widget with no body and no case. Test: sem-appB-later.
- lower/expr.prism:621 {kind} as a value: see C for a `for` and a `rebuild` as a plain
  function's value; a suffixed operator, a monad block and a lens stop at the parser or
  the checker (`unsupported`).
- lower/expr.prism:515 a lift over a sigil of {kind}. No program found: the checker
  lifts only at applications, operators, fields, indexes and interpolations, each a
  direct operand.

Zombie and dead states: `leave`, `clean_up`, `edges` (no refusal; the README's entry).
What a program can see of teardown holds: a `for` row whose key leaves and a replaced
generation fire in their last tick and are silent afterwards, the new generation's holds
fresh at fresh paths (sem-ch04-row-teardown and sem-ch04-generation-teardown pass). The
first value of an element value placed in a hosted scope (DH2) holds through two hosts
(sem-ch09-host-described-first-value passes).

## rustc refuses prismc's output

No refusal: prismc writes a crate and rustc refuses it. Each is an R, critical. Entries
above that end this way: sem-ch09-host-bundle-answer-named,
sem-ch09-answer-constructor-value, sem-ch04-rebuild-clause-plain-result,
sem-ch07-flatten-held-above, ts-ch04-whole-row-parent, ts-ch04-whole-hosted-result,
ts-ch04-interpolation-lifts-optional, sem-ch10-fault-kind-value, sem-appB-turn-value,
ts-ch03-method-function-value. Found by the Bible's document hosting:

- `!Filter` of `!Updated $d` written in place: the crate calls
  `crate::prelude::filter_ev` and `crate::prelude::updated`, which do not exist (E0425),
  and reads `self` in a function without it (E0424). Program: a supplier's arm `@
  !Filter (?ResponseTo side, p, !Updated $state)`, `$state` a parameter of the widget
  function. Test: sem-ch09-hosting-per-role-inline-update (hosting attempt 10, as
  written). A plain `fn !Echoed $?l = !Filter (!Updated $?l)` gives the same two E0425
  with no supplier in sight. With the update named first, `let !stepped = !Updated
  $state`, the program runs: sem-ch09-hosting-per-role-named-update passes.
- A generic tag with a `$` field of its variable: `struct PadEcho_g` and `PadEcho_desc`
  have a field of type `R` and no `<R>` (E0425; an E0308 follows). Program: `tag Pad.Echo
  of Element !r:` with `$?latest: r`, placed at `Text`. Test:
  sem-ch09-generic-tag-live-field. Hosting attempt 15 met it with a tag whose variable is
  missing from its output, a wrong program: see C.

## C: wrong programs the checker lets through

- lower/node/def.prism:272 lines that read each other at the build: a cycle among values.
- lower/expr.prism:576 `{b}` outside a `for`. Test:
  tests/checker/neg/bound-word-outside-for.
- lower/expr.prism:1012 a fault kind applied. A fault kind is its tag alone (Semantics
  ch. 10: every kind is declared bare, and a row kind with no output has values that are
  its tags), so `Fs.NotFound()` is wrong, and so is a tag `of Fault` that declares a
  field, whose use `Disk.Full(left=n)` reaches the same refusal. Tests:
  tests/checker/neg/fault-kind-applied and tests/checker/neg/fault-kind-fields.
- A refutable pattern in a `let`, a lambda parameter, a `for` binder or a `<-` of a
  widget block (Type System ch. 2, "Patterns and exhaustiveness": these take irrefutable
  ones): the checker says nothing, and the back end refuses the names the pattern binds
  ("the name t, which nothing declares") or the `let` (lower/node/def.prism:883). Test:
  tests/checker/neg/refutable-binder.
- A `for` bound by a `let` in a widget block (`let m = for x <- xs: @ x * 2`): the
  checker types it as the plain map, and the back end refuses it at lower/expr.prism:621
  ("a `for` outside a widget block as a value"). Test: tests/checker/neg/for-in-let.
- A tag pattern that names a field twice (`Ui.Button($label=$a, $label=$b)`): the
  checker says nothing. Test: tests/checker/neg/tag-pattern-field-twice.
- A ring of trait `where` clauses (`trait ^Up a where a: ^Down` and back): the checker
  does not end. Test: tests/checker/neg/trait-where-ring (tests/checker/run.sh cuts a
  check at 600 seconds).
- `ignore <-` of a function that was not applied (`ignore <- @Row`, `ignore <-
  @HostDocument`): the checker says nothing, and the back end refuses the line at
  lower/node/def.prism:938. Test: tests/checker/neg/ignore-binds-a-function.
- A second bare block for a widget that takes no arguments (`@Twice:` twice): the checker
  says nothing and the first block is built. Test:
  tests/checker/neg/clause-widget-without-arguments.
- A supplier written by clauses (`fn @Supply p, (Pad.Num(..)) = @ 7`) is a legal program
  the checker refuses: a clause's tag pattern does not refine the output. Test:
  tests/checker/pos/supplier-clauses.prism.
- A tag field's type variable that the tag's output does not have (`tag Seat.Request of
  Element !Response:` with `$?latest: ((Nat, r))`): the checker says nothing, a guest
  places the tag with a `Text` and the host's function reads the field as a `Nat`. Type
  System ch. 2: a variable in a field must appear in the index, and a generic tag's
  output is a scheme, its variables shared with its fields. Tests:
  tests/checker/neg/hosting-generic-request-field (hosting attempt 15, as written; rustc
  refuses its crate) and tests/checker/neg/hosting-generic-implementation (attempt 14, as
  written; the back end refuses it at lower/node/static.prism:651).

## G: guards

Each with the nearest program a user could write and its test, or the reason no program
reaches it.

- lower/node/mod.prism:403 an event read in a node without a port. No program reaches
  this: it reports a mistake of the lowering's own wiring, whatever the program.
- lower/node/elements.prism:59 an element of an unknown tag. The checker refuses the
  name: tests/checker/neg/unknown-tag.
- lower/node/def.prism:227 a block split at a line that binds nothing. No program reaches
  this: `?LateLineAt` finds only binding lines. lower/node/def.prism:219 (a line whose
  result is not late) is reached: see R, Bundles.
- lower/node/def.prism:865 a name at an instantiation with no binding. No program
  reaches this: the instantiations are the checker's own list for the `let`.
- lower/node/static.prism:405, 446, 492, 546 `@Distinct`, `@PerKey`, `@Run`, `@Machine`
  without an argument. The checker refuses each: tests/checker/neg/
  builtin-widget-without-argument. lower/node/static.prism:216, a host without an
  argument, stands behind `ignore <- @HostDocument`: see C.
- lower/node/static.prism:613 a widget of several clauses that takes no arguments: see C,
  the second bare block.
- lower/node/static.prism:651 a generic widget at a call whose types its parameters' do
  not match: reached, see R, Hosting.
- lower/node/switch.prism:758 a `rebuild` with no generation. No program reaches this: a
  `rebuild` has a block, and a block is one case.
- lower/expr.prism:667 a supplier's dynamic answer of no known form. No program reaches
  this: `?DynamicAnswer` takes every dynamic but `$ plain`, which the line above takes.
  The nearest program, `@ $~Shout $motd`, passes: sem-ch09-host-dynamic-answer-call.
- lower/expr.prism:670 a supplier's answer under a tag the arm does not name. No program
  reaches this: `#tag` is bound only by `TagPat`, always as a path.
- lower/expr.prism:723 `@Element` without its element. The checker refuses it:
  tests/checker/neg/element-without-its-element.
- lower/expr.prism:764 an event read as a value: reached, see R, Hosting. A `let !again =
  !go` in a block is not the case: sem-ch04-event-let passes.
- lower/expr.prism:765 a tuple pattern's binding as a value: reached, see R, Bundles.
- lower/expr.prism:766 a widget parameter read as a plain value. The nearest program, a
  widget parameter passed on to another widget, passes: sem-ch08-widget-parameter-passed.
- lower/expr.prism:777, 868 a name or a call nothing declares. The checker refuses the
  name: tests/checker/neg/unknown-name.
- lower/expr.prism:816, 1134 a global or a call with no lowering of its kind. No program
  reaches this: every name the checker resolves has a value entry.
- lower/expr.prism:1031 an answer constructor of no tag or with no argument. The nearest
  program, the constructor as a function value, takes another path and rustc refuses the
  crate: see R, Hosting.
- lower/expr.prism:1290 a constructor of an unknown owner. No program found: a
  constructor's owner is a choice or a record by its declaration.
- lower/expr.prism:1115 an element value without one of its fields. The checker refuses
  it ("`Ui.Label` needs `text`"): tests/checker/neg/element-value-without-field.
- lower/expr.prism:1355 a method call at a type the checker did not record. The nearest
  program, a method whose type is only in its result called from an argument's type,
  under a generic function's bound, as a function value, in a returned lambda and at a
  generic record's impl, passes: ts-ch03-result-only-method.
- lower/expr.prism:1417 an unknown operator. The nearest program, every operator the
  parser reads at `Nat`, `Int`, `Float`, texts, records, tuples, arrays, a generic type
  and under `?`, passes: ts-ch03-operators.
- lower/patterns.prism:164, 165, 207 a pattern of an unknown constructor. The checker
  refuses the name: tests/checker/neg/unknown-constructor-pattern.
- lower/jobs.prism:570 a call a frame cannot enter. The nearest program, an unbounded
  function value called from a record's field, a `let`, a lambda, a list under `Fold`, a
  tuple pattern's part and an optional's payload, passes: sem-ch07-frame-callees.
- lower/templates.prism:90, 105 a template call that names no function or fills no
  argument. The checker refuses each: tests/checker/neg/template-unknown and
  tests/checker/neg/template-missing-argument.
- emit/cpp/expr.prism:332, 347, 567, 575 a constructor, record or pattern of no type the
  C++ emitter knows. Not probed: the C++ target is not run in this phase.

Types the lowering does not spell (`OutRefused`)
- lower/expr.prism:1162, 1163 a type argument of a call that its type at the call does
  not give. The nearest program, a generic function whose type parameter is in no
  parameter's type, called from an annotation, from the caller's own type parameter, in
  a lambda, as a function value and in a `let` read at one type, passes:
  ts-ch05-type-arguments-at-call.
- lower/types.prism:107 a type variable the checker left undecided. The same program
  passes; an unread generic `let` in a plain function is lowered, and in a widget block
  it takes another path: see R, Functions.
- lower/expr.prism:1580 a field of an `Error`: reached, see R, Functions.
- lower/expr.prism:1589, 1590, 1591 a field of a type with no constructor, of a
  non-record. The checker refuses a field of a row's value, a choice, a tuple or a number
  (tests/checker/neg/field-of-no-record); fields of an alias of a generic record, under
  `?`, `~` and `$` and in an `update` block are lowered (ts-ch02-field-reads, which fails
  only on the `Error`'s message).
- lower/patterns.prism:186 a payload behind a reference. Nested constructor patterns
  through a recursive choice are lowered (ts-ch02-field-reads): the refused type is the
  place's own and is never printed.
- lower/types.prism:113 a type the checker did not find. No program reaches this: the
  checker makes it only beside an error, and a program with an error is not lowered.
- lower/types.prism:182 a type the checker did not declare. The nearest program, the
  prelude's `Ordering`, `Turn`, `Span`, `Duration`, `Fault` and `Exception` as fields and
  as the types of generic calls, passes: ts-ch02-prelude-types-named.
- lower/types.prism:469 an associated type no trait declares. No program reaches this:
  the checker makes an associated type only from a name a trait declared.
- lower/expr.prism:1764, lower/types.prism:272 and lower/model.prism:169 are the type's
  own matches and definition.

