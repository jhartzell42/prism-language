# Red tests

The tests red on the full run of build 2 (prismc-p2, claude-saved/step2, run 12-13
November 2026; the run stopped at its time limit after the conformance programs, before
the tests/documents trees, examples and module tests), with where each stands on
claude-saved/step3. A test leaves this list when it passes.

| test | how it fails on p2 | where it stands |
|---|---|---|
| assoc-params-mode-indexed |  | fixed in step3 (ok on p3) |
| doc-c1-never-field-wired-to-never | error[E0277]: the trait bound `Never: prism_rtl::FromPod` is not satisfied | fixed in step3 (ok on p3) |
| doc-c2-associated-type-reading | error[E0277]: the trait bound `G: Gauge_t` is not satisfied | deep walk lacked projection bounds; fixed after p3 |
| doc-c4-seat-family-indexed |  | indexed families: in p3; tag-variable fix after p3 |
| doc-c5-widget-function-partly-applied | tests/conformance/doc-c5-widget-function-partly-applied/app.prism:24: error[mismatch]: `@Counter` takes two arguments; here it has one | partial application: checker after p3, lowering not written |
| doc-c6-hosted-bundle-result | error[E0599]: no method named `n_d0` found for reference `&Page_w` in the current scope | lowering bug: `n_d0` on `Page_w` |
| doc-c8-derive-via |  | derive via: written after p3 |
| doc-seat-family-every-role |  | indexed families: in p3; tag-variable fix after p3 |
| doc-seats-record-of-rows |  | indexed families: in p3; tag-variable fix after p3 |
| doc-sk-existential-seat-key |  | needs a ruling: a `Role d` value refining `r` (question AF to Jimmy); existential lowering not written |
| doc-state-document-bound-and-default | tests/conformance/doc-state-document-bound-and-default/app.prism:25: error[mismatch]: a hold's step is an event; this is `State t7` | `State t7` ambiguous: the call names its document (ruling IN), test or checker to settle |
| heights-updated-of-mapped-dynamic-across-nodes | tests/conformance/heights-updated-of-mapped-dynamic-across-nodes/app.prism:22: error[hold-needed]: `Or` lifts over `($ 0)`, but its type variable here | test held per ruling HD (ok on p3) |
| promoted-constructors-unit-index |  | case read in an alias before cases were known; fixed after p3 |
| sem-ch04-building-match-no-arm-bound-live | error[E0599]: the variant, associated function, or constant `from_pod` exists for enum `Option<Never>`, but its trait bounds were not satisfied | fixed in step3 (ok on p3) |
| sem-ch04-building-match-no-arm | error[E0599]: the variant, associated function, or constant `from_pod` exists for enum `Option<Never>`, but its trait bounds were not satisfied | fixed in step3 (ok on p3) |
| sem-ch04-late-value-above-arm-with-rebuild-beside-inner-if | error[not-lowered]: src/program.rs: the plain result of an arm or a clause that is built before its block's node and numbered after it, read from a ge | lowering bug |
| sem-ch04-late-value-taken-by-arm-result-above | error[internal]: src/program.rs: a prismc bug: the name n, which nothing declares | lowering bug |
| sem-ch04-late-value-through-local-function-let-above-clause-widget | error[not-lowered]: src/program.rs: `@Wide m`, a line that takes a late result at its build, above the line that gives it, and whose places are counte | lowering bug |
| sem-ch07-while-match-for | FAILED tests/conformance/sem-ch07-while-match-for/tests.prism: the script did not parse: line 15: expected :, found Some(P("}")) | the test script does not parse (line 15) |
| sem-ch08-blueprint-value-from-or-in-late-numbered-arm | error[not-lowered]: src/program.rs: a generation whose rows take their numbers as they are built (a blueprint value applied) in an arm or a clause tha | lowering bug |
| sem-ch08-global-let-of-applied-widget-value | error[not-lowered]: src/program.rs: the widget function @Called as a value: a blueprint value is a supplier's answer that builds no place (`@ x`, `@El | lowering bug |
| sem-ch08-global-let-of-widget-value | error[not-lowered]: src/program.rs: the widget function @One as a value: a blueprint value is a supplier's answer that builds no place (`@ x`, `@Eleme | lowering bug |
| sem-ch08-widget-value-generic-through-associated-type | error[internal]: src/program.rs: a prismc bug: the generic widget function @Named as a value in ?Frame, which reaches itself through a constraint an i | lowering bug |
| sem-ch08-widget-value-generic-through-local-function | error[not-lowered]: src/program.rs: a quantified type variable of `?Made`: no instantiation gives it a type | lowering bug |
| sem-ch09-host-dynamic-answer-below-building-if-host-below-if | error[not-lowered]: src/program.rs: a place that is read at its block's build (place h1_1) after a building `match`, `if` or widget of several clauses | lowering bug |
| sem-ch09-host-late-plain-part-taken-above-bundle-result | error[not-lowered]: src/program.rs: a late plain result in a bundle | lowering bug |
| sem-ch09-tag-field-named-type-of-dynamic | tests/conformance/sem-ch09-tag-field-named-type-of-dynamic/app.prism:22: error[no-impl]: no impl of `^Num` for `$Nat` | checker: `no-impl` of `^Num` for `$Nat` |
| ts-ch04-plain-function-gives-whole-result-to-sigiled-parameter | tests/conformance/ts-ch04-plain-function-gives-whole-result-to-sigiled-parameter/app.prism:12: error[hold-needed]: `Or` lifts over `($ 0)`, but its ty | hold-needed: test held per ruling HD |
| ts-ch04-plain-function-lifts-over-whole-result | tests/conformance/ts-ch04-plain-function-lifts-over-whole-result/app.prism:15: error[hold-needed]: `Or` lifts over `($ 0)`, but its type variable here | hold-needed: test held per ruling HD |
| ts-ch04-plain-function-reads-whole-result-lifted | tests/conformance/ts-ch04-plain-function-reads-whole-result-lifted/app.prism:12: error[hold-needed]: `Or` lifts over `($ 0)`, but its type variable he | hold-needed: test held per ruling HD |
| ts-ch04-time-taken-whole | tests/conformance/ts-ch04-time-taken-whole/app.prism:28: error[hold-needed]: `Or` lifts over `($ 0)`, but its type variable here already holds a `$Nat | hold-needed: test held per ruling HD |
| ts-ch04-whole-anywhere | tests/conformance/ts-ch04-whole-anywhere/app.prism:14: error[hold-needed]: `Or` lifts over `($ 0)`, but its type variable here already holds a `$Nat` | hold-needed: test held per ruling HD |
| ts-ch04-whole-event-heard | tests/conformance/ts-ch04-whole-event-heard/app.prism:14: error[hold-needed]: `Or` lifts over `never`, but its type variable here already holds a `!Cl | hold-needed: test held per ruling HD |
| ts-ch04-whole-in-a-body | tests/conformance/ts-ch04-whole-in-a-body/app.prism:12: error[hold-needed]: `Or` lifts over `($ 0)`, but its type variable here already holds a `$Nat` | hold-needed: test held per ruling HD |
| ts-ch04-whole-lifts-many | tests/conformance/ts-ch04-whole-lifts-many/app.prism:23: error[hold-needed]: `Or` lifts over `($ 0)`, but its type variable here already holds a `$Nat | hold-needed: test held per ruling HD |
| ts-ch05-color-function-in-dynamic-through-lifted-call | error[not-lowered]: src/program.rs: a function value of its callee's own color inside the value a call returns, at a call that makes the color bounded | lowering bug |
