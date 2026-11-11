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
- lower/expr.prism:796 a supplier's dynamic answer that reads what has no value at the
  owner's build: a structural dynamic (a `for`'s gathered result, a `rebuild`'s) or a
  name of the parent. No test. The host's own pushed place and a `let` below the host's
  line are lowered (sem-ch09-host-dynamic-answer-late).
- lower/expr.prism:703 `@ x` outside a tag arm where `x` is no plain data (a function).
  No test. Any other answer outside an arm is lowered: plain data in `Answer`'s case of
  no tag, read by the place that takes it (sem-ch09-host-answer-without-arm).
- lower/expr.prism (`WrittenPart`) a bundle answer's `$` part that reads what has no
  value at the owner's build (sem-ch09-host-bundle-answer-structural-part,
  -parent-part). Every other bundle answer is lowered, any mix of plain, `$` and event
  parts, written as a tuple or given as a value (`PartsBlueprint`:
  sem-ch09-host-bundle-two-events-answer, -two-dynamics-answer,
  -plain-beside-dynamic-answer, sem-ch09-host-bundle-live-dynamic-answer,
  sem-ch09-bundle-dynamic-part).
- An element tag whose output is a bundle is lowered for any mix of plain, `$` and
  event parts (`BundleOutputOf`: sem-ch09-bundle-two-events, -two-dynamics,
  -plain-beside-dynamic, sem-ch09-bundle-dynamic-part), in a hosted scope too
  (sem-ch09-host-bundle-dynamic-part-forwarded). A bundle nested in a tag's bundle
  output is read as a plain part: no test.
- lower/expr.prism (`WidgetBlueprintValue`, `WidgetValueWords`) a widget with places as
  a blueprint value is lowered where its parameters are plain, it has no type parameter
  and it gives nothing or a plain value back (sem-ch09-host-widget-answer,
  sem-ch08-blueprint-out-of-value). Left refused, no tests: one with a `$`, event or
  blueprint parameter, a type parameter, or an event, dynamic or bundle result; one read
  in a place `ValuedWidgets` does not look (a record's field read back and given on is
  a value already; a `let` in a plain function's body is not looked at). One built in a
  hosted scope builds its hosted row (sem-ch09-host-widget-answer-hosted).
- lower/node/static.prism (`BlueprintValuesAt`) a widget defined in a block read as a
  value is lowered (sem-ch08-local-widget-passed-as-value, sem-ch08-local-widget-in-record).
  One whose places read a dynamic of the block is lowered
  (sem-ch08-local-widget-value-reads-dynamic), and one in a hosted scope and one that
  gives a plain value back. Left refused: one that gives an event, a dynamic, a bundle
  or a value that is no plain data back. lower/expr.prism (`NameValueOf`) "a widget
  defined in a block read as a blueprint value" stands for a reading outside a line's
  value places (inside a lambda's own block).
- lower/node/static.prism (`AppliedBlueprintLine`) a blueprint value applied by a block is
  lowered, and so is `@Element` of any element value (a parameter, a call's result;
  sem-ch08-element-parameter-statement): a line of unit, a line that gives an event
  back, which hears the blueprint's answers, and a line that binds a plain result
  (sem-ch08-blueprint-value-gives-back), in a hosted scope too
  (sem-ch09-blueprint-value-hosted), and a line that binds a dynamic
  (sem-ch08-element-value-dynamic-statement). Left refused: one that gives a bundle
  back (sem-ch08-widget-value-bundle-result reaches it, behind the widget's own
  refusal as a value).
- lower/node/static.prism:104 a widget function parameter applied to arguments whose
  argument is a function value that is no name (a lambda). No test. A named widget
  function passed and applied is lowered (sem-ch08-widget-parameter-applied).
- lower/node/static.prism:778 a generic widget at a call whose types its parameters' do
  not match. No program found: a blueprint parameter and a function parameter are
  matched through (sem-ch08-generic-widget-blueprint-parameter).
- A generic tag's field read in a host's arm at a written type (`let x: r = ..` over
  `Pad.Echo($?latest=$l)`): the tag's variables are plain data in its structs
  (`TagFieldOut`), and a type the arm writes with the variable is spelled as a type
  parameter no item declares. No refusal, no test.

Bundles
  A root's bundle of plain parts, event parts and one dynamic part is lowered
  (sem-ch07-root-bundle-result).
- A bundle with a late plain part bound whole to a name (`both <- @Choose mode`) is its
  parts (`BindWhole`, sem-ch08-late-part-bundle-named), and a late plain part taken
  apart by a pattern of its own is read by that pattern (sem-ch08-late-part-nested-pattern).
  Left: a line of the block's node below the split that reads the whole name (`let same =
  both`) finds no binding ("the name both, which nothing declares"); a bundle inside the
  bundle whose late part a nested tuple pattern binds is not bound late; a dynamic part of
  a named bundle read by a row of a row (lower/node/switch.prism, `RowPartBn`). No tests.
- A `rebuild` whose block ends on a bundle a line named whole (`@ both`) is lowered
  (`WholeNamedOf`, sem-ch04-rebuild-named-bundle-result); a bundle inside that bundle is
  refused there. No test.
- A root's bundle with several dynamic parts is told part by part
  (sem-ch07-root-bundle-two-dynamics).
- lower/node/switch.prism:216 a `rebuild` whose generation gives a dynamic it does not
  lower: no program found since the bundle is lowered.
- Lowered, no refusal left: a `for` whose rows end on a bundle
  (sem-ch04-for-bundle-result); a widget of several clauses whose clause ends on a bundle
  (sem-ch08-clause-bundle-result); a bundle given another name (sem-ch04-bundle-rebound).

Building `match`, clause widgets, `rebuild`
- lower/node/def.prism (`BindLate`, `MarkLateLets`) a late result, or a `let` that reads
  one, read at its block's build by a line of the block's node. A building line that
  reads one is built after the row that gives it (`DeferredLine`:
  sem-ch04-building-match-value-above-arm-reads-block, sem-ch04-building-if-value-above,
  sem-ch04-late-widget-result-value-above, sem-ch04-late-let-value-above); a building
  `match` with no guard whose arms read nothing of the block is built early
  (`EarlyChoiceLine`, sem-ch04-building-match-value-above); a place's field, a plain
  `let` of one name and a hold's first value are late in turn
  (sem-ch04-building-match-read-above, sem-ch04-building-match-hold-above). Left
  refused, no tests: a `let` of a pattern that is no one name, or of a bundle or a type
  with time inside, that reads the late name (the three `unbuilt` refusals of
  `BindLate`, `MarkLateLets` and `BindWhole`); and in `DeferredLine`, a line that takes
  the late name and whose places are counted as built, and one that gives a plain
  result of its own.
- lower/jobs.prism: none added. A bare `for` in a job is lowered (`ForJob`), no test of
  the job form; the plain form is sem-ch05-update-for-bare.
- lower/node/finish.prism:699 a place read at its block's build before the node exists
  (a first-value query, a host made in the build) after a building `match`, `if` or
  widget of several clauses whose rows differ in width and are built after the node. No
  test.
- gap ID, the part left: a widget that calls itself and has a `$` or event parameter
  beside a result or with time below its top, a
  blueprint parameter whose argument is no blueprint value the lowering makes (a widget
  of a block, a widget with a `$` parameter) or beside a result, an event, dynamic or
  bundle result, or a type parameter is lowered level by level
  and faults at its build past sixteen levels (`TooDeep`). No test. A `$` or an event
  parameter is as deep as its
  data (sem-ch08-widget-recursion-deep-dynamic-parameter, -event-parameter), and so are
  a blueprint parameter given a widget the program reads as a value
  (sem-ch08-widget-recursion-deep-blueprint-parameter) and a widget in a hosted scope
  (sem-ch09-widget-recursion-deep-hosted). A
  widget of one clause or of several, with plain parameters and nothing or a plain value
  given back, is as deep as its data, to 2^20 places a call
  (sem-ch08-clause-widget-recursion, sem-ch08-widget-recursion-deep,
  sem-ch08-widget-recursion-deep-one-clause, sem-ch08-widget-recursion-deep-result).
- lower/node/static.prism (`WidgetCall`) a widget this back end does not lower yet: no
  program found since a blueprint taken out of a value by a pattern is applied
  (`ValueBlueprintLine`, sem-ch08-blueprint-out-of-value). A blueprint a `let` names is
  lowered (sem-ch08-blueprint-let), and a blueprint parameter of a widget of several
  clauses (sem-ch08-clause-widget-blueprint-parameter).
- Lowered, no refusal left: LW (sem-ch08-late-result-widget-parameter); a `rebuild` of a
  clause widget with a plain result (sem-ch04-rebuild-clause-plain-result); a clause
  widget's dynamic read through its caller's parent
  (sem-ch08-clause-dynamic-through-parent); SP2 and SP3
  (sem-ch04-building-match-plain-group, sem-ch04-building-match-plain-reads); TC
  (sem-ch08-places-counted-as-built, sem-ch04-building-match-mixed-arms); a `for` row
  that calls its block's function (sem-ch04-for-local-function-reads-block).

Dynamics and events
- lower/node/finish.prism:523 `!Updated` of a dynamic no sink of the node writes (UP).
  No program found: a generation's dynamic result the block keeps is lowered
  (sem-ch07-updated-clause-result), as a constant `let` and a widget's dynamic parameter
  are (sem-ch07-updated-constant, sem-ch07-updated-parameter).
- lower/node/finish.prism:533 the same in a row over a name of its block (SU). No
  program found: a row over its block's parameter passes
  (sem-ch07-updated-row-parameter).
- lower/node/switch.prism:1328 `@Flatten` of a dynamic of dynamics with no live dynamic
  at the build (LF). No program found: an outer written in place and an outer bound
  below are lowered (sem-ch07-flatten-call, sem-ch07-flatten-held-above).
- lower/expr.prism:371 a dynamic passed whole to a function where it has no live dynamic
  (LF). No program found: a clause widget's result, a place's dynamic, a global, a row's
  parent's hold and a hosted child's result are read through the node's link
  (ts-ch04-whole-clause-result, -place-dynamic, -global-dynamic, -row-parent,
  -hosted-result); a `for`'s `$v` over `@PerKey` has no test.
- lower/expr.prism:341 a dynamic a function gives whole, read in a plain function. No
  program found: `fn $Kth n = Or (Consts n)[1], ($ 0)` compiles.
- lower/expr.prism:549 a lift over a sigil of {kind}. No program found.
- lower/patterns.prism:332 a pattern inside an element's `$` field: see C
  (`Ui.Label($text=$"x")` passes the checker; `$text="x"` is a mismatch there).

Functions
- lower/node/def.prism (`LetLine`) a widget block's `let` of a pattern that is no name
  and no tuple of names and binds an event inside it. No test. With plain names it is
  lowered (sem-ch08-let-pattern-in-widget), and with dynamics inside it
  (`LetPatternDynamics`, sem-ch08-let-pattern-dynamic-in-widget).
- lower/expr.prism `StartsAsBounded`: a function value of its callee's own color inside
  the value a call returns, at a call that makes the color bounded, where another
  function's parameter holds it, or a named type inside a tuple, an optional, a `~T` or
  a map.
  No test. A record or a choice the call returns, and an array of either, is made again
  (`?TypeStartsAsBounded`, ts-ch05-color-function-in-named-result,
  ts-ch05-color-function-in-choice-result,
  ts-ch05-color-function-in-nested-named-result). Through an array, an optional, a `~T`, a tuple, a map's values and a
  function's result it is lowered (ts-ch05-color-function-in-result,
  ts-ch05-color-function-in-map-result).
- lower/node/def.prism (`RefusePlainResult`) a widget's plain result bound by a pattern
  that left a name without its field: an internal error; `BindPatternResult` takes every
  pattern (sem-ch08-plain-result-pattern passes).

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
- lower/node/def.prism (`WidgetStmt`) a statement of a widget block that is none of its
  kinds: an internal error. A widget defined inside a widget block is lowered
  (sem-ch04-local-widget); `require`, `forbid`, `while`, `<=` and `+=` are a checker
  mismatch there, `return` a parse error.
- lower/expr.prism:2048 a statement of an expression block not lowered yet: a G, the
  checker refuses a bind, `emit`, `require`, `while` there (mismatch).
- lower/node/def.prism (`BuildLine`) a building line that is no element, blueprint or
  builder: an internal error; the checker gives mismatch for such a bare line.
- lower/node/static.prism (`WidgetCall`) a widget not lowered yet: see Building `match`.
  A building `if` with no `else`
  (sem-ch04-building-if-no-else) and a bare call of a widget defined inside the block
  (sem-ch04-local-widget) are lowered.
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

No refusal: prismc writes a crate and rustc refuses it. Each is an R, critical. None is
known that has no lowering written; none of the lowerings written has been run.

## C: wrong programs the checker lets through

- lower/node/def.prism (`BuildInOrder`) lines that read each other at the build: an
  internal error; the checker gives `prompt-cycle` (tests/checker/neg/build-cycle).
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
- lower/expr.prism (`AnyValue`) an existential of a program's trait with a method that
  needs one dictionary for a whole collection or for no value at all (`?PackedDefOf`):
  the trait's type inside another type (`fn [a] -> Nat`) or in no parameter (`Empty:
  a`), a type parameter of its own, an associated type. The dictionary belongs to the
  packing, one a packed value, so such a method cannot be asked of `any ^Trait` (the
  coordinator's first ruling): the checker says nothing and the back end refuses it.
  Tests: tests/checker/neg/existential-method-needs-one-dictionary;
  tests/conformance/ts-ch03-existential-trait-inside-array is that wrong program with a
  transcript. A prelude trait other than `^Show`, `^Eq`, `^Ord` as a bound is refused
  there too (an R, no test).
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

- lower/node/mod.prism (`Crossed`) an event read in a node without a port: an internal
  error. It was reached by a widget parameter's event read in a row, which is lowered
  (sem-ch08-widget-parameter-event-in-row).
- lower/node/elements.prism:59 an element of an unknown tag. The checker refuses the
  name: tests/checker/neg/unknown-tag.
- lower/node/def.prism (`SplitBlockInto`) a block split at a line that binds nothing: an
  internal error; `?LateLineAt` finds only binding lines. "A block split at a line whose
  result is not late" (lower/node/def.prism:383) stays a refusal: no program found since
  a bundle with no plain part splits nothing.
- lower/node/def.prism (`LetLine`) a name at an instantiation with no binding: an
  internal error; the instantiations are the checker's own list for the `let`.
- lower/node/static.prism `@Distinct`, `@PerKey`, `@Run`, `@Machine` without an argument:
  internal errors. The checker refuses each: tests/checker/neg/
  builtin-widget-without-argument. lower/node/static.prism:282, a host without its child
  or its supplier, stands behind `ignore <- @HostDocument`: see C.
- lower/node/static.prism:740 a widget of several clauses that takes no arguments: see C,
  the second bare block.
- lower/node/static.prism:651 a generic widget at a call whose types its parameters' do
  not match: reached, see R, Hosting.
- lower/node/switch.prism (`GenSelect`) a `rebuild` with no generation: an internal
  error; a `rebuild` has a block, and a block is one case. So are a building `match` or
  `if` whose arms do not all end on a plain result, a `rebuild` that hands back no
  dynamic whose generation is a clause widget ending on a plain result, and a late plain
  result bound where its block is not split (`IsLateExpr` follows a chain of sixteen
  widgets; a longer one would reach it).
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

