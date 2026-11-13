# Red tests

The failing tests on `claude-self-hosting` at loop step 1 (12 November 2026), each run
once on the compiler bootstrapped from 939f598e (prismc-p1). One row per test: what it
is for, and how it fails. A test leaves this list when it passes.

| test | for | how it fails on p1 |
|---|---|---|
| doc-c1-never-field-wired-to-never | documents plan, step C1 | error[E0425]: cannot find type `Never` in module `crate::prelude` |
| doc-c3-role-indexed-associated-types | documents plan, step C3 | (no first error line kept; see its log) |
| doc-c4-seat-family-indexed | documents plan, step C4 | (no first error line kept; see its log) |
| doc-c5-widget-function-partly-applied | documents plan, step C5 | tests/conformance/doc-c5-widget-function-partly-applied/app.prism:24: error[mismatch]: `@Counter` takes two arguments; here it has one |
| sem-ch04-late-value-above-arm-with-rebuild-beside-inner-if | lowering bug | error[not-lowered]: src/program.rs: the plain result of an arm or a clause that is built before its block\'s node and numbered after it, read from a generat |
| sem-ch04-late-value-through-local-function-let-above-clause-widget | lowering bug | error[not-lowered]: src/program.rs: `@Wide m`, a line that takes a late result at its build, above the line that gives it, and whose places are counted as t |
| sem-ch08-blueprint-value-from-or-in-late-numbered-arm | lowering bug | error[not-lowered]: src/program.rs: a generation whose rows take their numbers as they are built (a blueprint value applied) in an arm or a clause that is b |
| sem-ch08-widget-value-generic-through-associated-type | lowering bug | error[internal]: src/program.rs: a prismc bug: the generic widget function @Named as a value in ?Frame, which reaches itself through a constraint an impl so |
| sem-ch08-widget-value-generic-through-local-function | lowering bug | error[not-lowered]: src/program.rs: a quantified type variable of `?Made`: no instantiation gives it a type |
| sem-ch09-host-dynamic-answer-below-building-if-host-below-if | lowering bug | error[not-lowered]: src/program.rs: a place that is read at its block\'s build (place h1_1) after a building `match`, `if` or widget of several clauses whos |
| sem-ch09-host-late-plain-part-taken-above-bundle-result | lowering bug | error[not-lowered]: src/program.rs: a late plain result in a bundle |
| ts-ch02-constructor-hold | the hold (single braces) and lifting constructors | (no first error line kept; see its log) |
| ts-ch02-constructor-lifts-over-dynamics | the hold (single braces) and lifting constructors | tests/conformance/ts-ch02-constructor-lifts-over-dynamics/app.prism:22: error[mismatch]: expected `$t10`, found `Pair $t15 $t21` |
| ts-ch02-constructor-lifts-over-event | the hold (single braces) and lifting constructors | tests/conformance/ts-ch02-constructor-lifts-over-event/app.prism:23: error[mismatch]: expected `[!t25]`, found `[Heard !Text]` |
| ts-ch04-hold-event-in-array | the hold (single braces) and lifting constructors | (no first error line kept; see its log) |
| parser | parser test module (`<:` and `< :`) | FAILED prismc/syntax/parser/tests.prism: the transcript differs: |
| tests/documents/c6-chess-by-hand | chess and hosting in design form | (no first error line kept; see its log) |
| tests/documents/library | chess and hosting in design form | (no first error line kept; see its log) |
| tests/documents/tally | chess and hosting in design form | (no first error line kept; see its log) |
| examples/chess | chess and hosting in design form | examples/chess/game/mod.prism:192: error[mismatch]: expected `[!t2386]`, found `[Heard !(Asked ChessGame Player (Responded Player ChessReason))]` |
