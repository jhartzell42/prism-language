# The refusals of prismc's back end

Written by Claude; subject to change. Every `Refusal` / `RefusalDo` message in
prismc/lower and prismc/emit, by file and line, with its class. Classified by reading; an
entry marked "unsure" was not tested.

- **G**: a guard no checked program reaches. It becomes an internal error, "a prismc
  bug: ..".
- **R**: a legal program the back end does not lower. It is lowered, failing test first.
- **C**: a wrong program. It becomes a checker error with a diagnostic, failing test first.

Besides these, `OutRefused` (a type the lowering does not spell) stands at 15 places in
lower/expr.prism, model.prism, patterns.prism and types.prism; each is a G unless a
program below reaches it.

## R: legal programs not lowered

Hosting
- lower/expr.prism:658 `@ x` as a supplier's answer outside a tag arm. Program: a supplier
  `\|p, e| @ 1` with no `match e`. Ruling: wrapped by the place's tag at run time.
- lower/expr.prism:667 a supplier's dynamic answer of no known form. Program: `@ $f x`
  where the dynamic is a call's result, not a name, a hold or `$ literal`.
- lower/expr.prism:716 a supplier's dynamic answer that has no value at the owner's
  build. Program: `@ $d` where `$d` is a `let` after the host's line or a pushed place.
- lower/expr.prism:696, 697 a bundle answer with a `$` part, two event parts, or not
  written as a tuple. Program: `@ pair` under a bundle tag's arm, `pair` a name.
- lower/node/static.prism:560 `@Element` in a block of an element value that is not a
  tag applied in place. Program: `let e = Ui.Label(..)`, then `@Element e` as a line.
  Unsure: the description lowering (LE) may cover it and leave this a G.
- lower/node/elements.prism:182 an element tag whose bundle output has a `$` part or two
  event parts. Program: a tag declared `-> (($Nat, !Click))` placed in a block.
- DQ (no refusal; a fault at the build): a described place with an event field in a
  hosted scope.

Bundles
- lower/node/keyed.prism:165 a `for` whose rows end on a bundle.
- lower/node/switch.prism:488 a widget of several clauses whose clause ends on a bundle.
- lower/node/switch.prism:676 a `rebuild` whose block ends on a bundle.
- lower/node/def.prism:695 a late plain result in a bundle. Program: a block that ends
  `@ ((n, !e))` where `n` is bound from a building `match`.
- lower/node/def.prism:96 a bundle as the root's result: see C.

Building `match`, clause widgets, `rebuild`
- lower/node/def.prism:553 a late plain result bound where its block is not split (LW).
  Program: `n <- @w` where `@w` is a widget parameter whose block ends late.
- lower/node/switch.prism:595 a building `match` or `if` whose arms do not all end on a
  plain result. Unsure: arms have one type, so this may be a G.
- lower/node/switch.prism:359 a `rebuild` whose generation is a widget of several
  clauses ending on a plain result. Program: `$n <- rebuild $x: @Pick _` with
  `fn @Pick 0: @ 1`.
- lower/node/switch.prism:155 a `rebuild` whose generation gives a dynamic it does not
  lower. Program: `$$d <- rebuild $x:` whose block ends `@ $ 1` (a constant dynamic).
  Unsure of the exact form.
- lower/node/switch.prism:430 a widget of several clauses given a dynamic its caller
  reads through its own parent. Program: inside a `for` row, `@Clause $outer` where
  `$outer` is the block's hold.
- lower/node/static.prism:79 a widget parameter applied to arguments. Program:
  `fn @Frame @inner: @inner 1`.
- SP2, SP3 (tests: sem-ch04-building-match-plain-group; lines below a split reading a
  widget parameter or a local function).
- TC (test: sem-ch08-places-counted-as-built), PR (a clause widget that calls itself).
- gap ID: a widget that calls itself deeper than sixteen levels.

Dynamics and events
- lower/node/finish.prism:518, 528 `!Updated` of a dynamic no sink of the node (or of
  the row's block) writes (UP, SU). Program: `!Updated $c` with `let $c = $ 1`, and with
  `$c` a caller's parameter. Ruling: a constant never fires; a caller's is a port.
- lower/node/switch.prism:867 `@Flatten` of a dynamic of dynamics with no live dynamic
  at the build (LF).
- lower/expr.prism:354 a dynamic passed whole to a function where it has no live dynamic
  (LF).
- lower/expr.prism:327 a dynamic a function gives whole, read in a plain function.
- lower/node/dynamics.prism:192 `!MergeAll` of a list that is not plain. Unsure a
  program reaches it.
- lower/expr.prism:494 an event value of several events. Unsure; read as R.
- lower/patterns.prism:268 an event field in an element pattern. Program:
  `match e: Ui.Button(!press=p, ..): ..`.
- lower/patterns.prism:323 a pattern inside an element's `$` field. Program:
  `match e: Ui.Label($text="x"): ..`.

Functions
- lower/expr.prism:886 a local function that calls itself.
- lower/expr.prism:939 a local function of several clauses as a value.
- lower/expr.prism:940 a generic local function as a value.
- lower/expr.prism:1203 a function value of its callee's own color inside the value a
  call returns, at a call that makes the color bounded.
- lower/expr.prism:636 an existential of several traits (`^A + ^B`). Unsure of the form.
- lower/node/static.prism:491 `@Run` of a function value that is not a name. Program:
  `@Run (\|| 1)`.
- lower/node/def.prism:883 a widget block's `let` of a pattern that is no name or tuple
  of plain names. Program: `let Extent(w=w, ..) = e` in a widget block.
- lower/node/def.prism:687 a widget's plain result bound by a pattern. Unsure:
  `BindPatternResult` may cover every pattern and leave this a G.

Jobs and templates
- lower/jobs.prism:289, 291, 292 a cycle-point `while` in a `for &` of an `update`
  block, in another expression, in another statement (gap LP).
- lower/templates.prism:231 a `while` whose pattern matches through an `Rc`.
- lower/templates.prism:262 a `for` by reference over what is not an array. Program:
  `for &x in map` in an `update` block. Unsure whether the checker allows it.
- lower/templates.prism:372 a reference path this back end does not lower.
- lower/expr.prism:145 `?` of a place, which C++'s TRY would move out of.

Catch-alls (each is a G for the kinds that have a case, an R for any kind left; to be
enumerated against the syntax tree's kinds)
- lower/node/def.prism:579 and lower/expr.prism:2048 a statement not lowered yet.
- lower/node/def.prism:938 a building line not lowered yet.
- lower/node/static.prism:121 a widget not lowered yet.
- lower/node/static.prism:600 the builtin widget {name}.
- lower/expr.prism:621 {kind} as a value.
- lower/expr.prism:515 a lift over a sigil of {kind}.

Zombie and dead states: `leave`, `clean_up`, `edges` (no refusal; the README's entry).

## C: wrong programs the checker lets through

- lower/node/def.prism:272 lines that read each other at the build: a cycle among values.
- lower/node/def.prism:96 a bundle as the root's result (a World hears a plain value, a
  dynamic or an event). Unsure: the book may let a root give a bundle; then R.
- lower/expr.prism:576 `{b}` outside a `for`.
- lower/expr.prism:1012 a fault kind applied. Unsure; read as C.
- lower/expr.prism:1115 an element value without one of its fields. Unsure: G if the
  checker already demands every field.
- A hold above a split that needs the late result at its build (SP2): both lines named.

## G: guards

- lower/node/mod.prism:403 an event read in a node without a port.
- lower/node/elements.prism:59 an element of an unknown tag.
- lower/node/def.prism:219, 227 a block split at a line whose result is not late, or
  that binds nothing (`?LateLineAt` finds only such lines).
- lower/node/def.prism:865 a name at an instantiation with no binding.
- lower/node/static.prism:216, 405, 446, 492, 546 a host, `@Distinct`, `@PerKey`, `@Run`,
  `@Machine` without an argument (the checker counts arguments).
- lower/node/static.prism:613 a widget of several clauses that takes no arguments.
- lower/node/static.prism:651 a generic widget at a call whose types its parameters' do
  not match.
- lower/node/switch.prism:758 a `rebuild` with no generation.
- lower/expr.prism:670 a supplier's answer under a tag the arm does not name.
- lower/expr.prism:723 `@Element` without its element.
- lower/expr.prism:764, 765, 766 an event, a tuple pattern's binding, a widget
  parameter read as a plain value (the checker's layers refuse each).
- lower/expr.prism:777, 868 a name or a call nothing declares.
- lower/expr.prism:816, 1134 a global or a call with no lowering of its kind.
- lower/expr.prism:1031 an answer constructor of no tag or with no argument.
- lower/expr.prism:1290 a constructor of an unknown owner.
- lower/expr.prism:1355 a method call at a type the checker did not record.
- lower/expr.prism:1417 an unknown operator.
- lower/patterns.prism:164, 165, 207 a pattern of an unknown constructor.
- lower/jobs.prism:570 a call a frame cannot enter.
- lower/templates.prism:90, 105 a template call that names no function or fills no
  argument.
- emit/cpp/expr.prism:332, 347, 567, 575 a constructor, record or pattern of no type the
  C++ emitter knows.
