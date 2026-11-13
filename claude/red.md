# Red tests

Step 1 of the loop (RULES.md, 13 November): every known issue and every wanted change has a
failing test before prismc/ is touched. This lists them on prismc-p11 (the fixed point
through 60ad099d) with what each needs. A test leaves this list when it passes. The full
run on p11 stopped at `ts-ch02-constructor-hold`; from 8bf86dd1 on, the library's final
form makes every program red on p11 (each loads Documents), so the rest of the run goes on
a tree at 20004f18, before the library, to find the remaining lowering reds
(shared/loop/full-p11-oldlib.summary); they are added as they land. The nine p2 reds past
that point (ts-ch04 whole-result tests, ts-ch05-color-function-in-dynamic-through-lifted-call)
check clean on p11 there.

Kinds: **gap** a compiler feature or fix to write in step 2; Q3 and Q4 are answered (rulings OP, RT).

## Features, each with its test

| feature (ruling) | test(s) | on p11 |
|---|---|---|
| empty impl, header alone (EI) | conformance/subtrait-default-for-method-above, neg/default-from-two-rungs | parse error |
| sub-trait default for a method above (SD) | conformance/subtrait-default-for-method-above | parse error (EI first) |
| two rungs' defaults are `ambiguous` (SD) | neg/default-from-two-rungs | parse error (EI first) |
| own declaration shadows `use X.*` (OS) | conformance/own-declaration-shadows-use | `collision` on `Kind` |
| row parameter needs no kind (RR) | doc-seats-record-of-rows (checker and conformance); library `FileSeats`, tally, c6, c8 seats | "a record has no row of its own"; "`Ui.*` is a row, not a type" |
| associated row default, `row Lower = unit` in a trait | conformance/ts-ch03-associated-row-default; library ^Document (tally, c6, c8) | "a trait declares an associated row ...; an impl chooses it" |
| `@Request`/`@Query` on the state, their defaults in ^StateDocument (RS) | library ^Document/^StateDocument (tally, c6, c8); the default is SD's, and an associated type's default already works (ts-ch03-assoc-default, green) | `^StateDocument` has no method `@Request` |
| `forall o.` in a type alias body | conformance/ts-ch06-forall-in-alias-body; library `Supplier` | "`o` is no parameter of `Supplier`" |
| a supplier lambda's arms checked at the refined `o` | neg/doc-seat-family-arm-answers-other-role; library `SupplierFrom`, `@OnDisk`, `@Over` | arm results mismatch `o` |
| binding-group cycle through a function value (book ch. 5: "the two bindings refer to each other, and that is legal") | library `@HostDocument` (every document test); conformance/sem-ch05-binding-group-cycle-through-function-value checks clean on p11 (its lowering is untested), so the library's `prompt-cycle` likely comes from its `Supplier` error; rechecked once the library checks | `prompt-cycle` `$guests`, `Supply` |
| Fixed narrowing at an associated kind (VO) | doc-sk-existential-seat-key; c6 `Stepping` | not-exhaustive; `SeatId ChessGame r` not narrowed to `Color` |
| `ValueOf` on a variable (VO) | covered by the `Fixed<<r>>` match below (library `?ResponseTo`) | |
| prelude `Utf8Bytes`, `~Utf8Text` in the runtime | conformance/sem-appb-utf8-bytes; library `@OnDisk` | unknown name (at 20004f18) |
| Net and File worlds in the runtime | library `@Over`, `@Serve`, `@OnDisk`; chess (agent writing) | not reached |
| existential lowering | existential-unbounded-index-list | not-lowered |
| family associated types in lowering, a variant per instantiation | doc-c4-seat-family-indexed, doc-seat-family-every-role | not-lowered |
| partial application lowering (AF) | doc-c5-widget-function-partly-applied | not-lowered |
| default widget method inlined with its parameter substituted | doc-state-document-bound-and-default | internal: no value entry |
| `Step<<d>>` call recorded at its type | forall-three-jobs | internal |
| `{ $x }` placeholder escapes (wip-brace-placeholder.patch) | neg/brace-placeholder | wording/span |
| alias name kept in messages | neg/doc-sk-opened-role-escapes | message names the expansion |
| a map literal keeps its values whole, as array literals do (HD) | conformance/hd-map-literal-keeps-dynamic-whole; the seats of tally, c6, c8 | `expected Text ==> $Nat, found Text ==> t9` |
| own declaration shadows the prelude (OS) | conformance/own-declaration-shadows-prelude | `collision` on `Len` |
| `use X.Name as Other` (UA) | conformance/use-name-as | parse error at `as` |
| a row with two variables (RT) | checker/pos/row-two-variables; library `@HostDocument` | `row-two-tails` |
| derive asks equalities of associated types (DE) | library via derive (c8, chess) | parse error at `where` |
| `Fixed s` matched against `Fixed<<r>>`, `r` a variable (OP, VO) | library `?ResponseTo` | (behind the library's parse error) |
| a tag arm narrows the matched element to its family's row (OP) | library `@HostRole` | (behind the parse error) |
| library builtins with no body, each needing the runtime: `SeatPathOf`, `$Merge`, `SeatIdAt`, `$GuestsOf`, `@Answer`, `!Outgoing`, `?ResponseFrom`, `$?AnswerFrom` | every document test | not reached |
| `!Filter` over a `!??`, Seat.Leave's last request (chess agent) | examples/chess | not confirmed |
| clause widgets whose rows differ (chess agent) | conformance/sem-ch08-clauses-emit-parts-of-the-row: checks clean on p11; its lowering and hand-derived transcript are unverified | |
| `$` over an applied `@Serve<<ChessGame>>` blueprint, and `!Filter` over Seat.Leave's `!??` (chess agent, unconfirmed) | examples/chess: rechecked once the library checks | |
| a derive asks equalities (DE), alone | conformance/doc-c8-derive-via-asks-equalities | parse error at `where` |
| `Fixed s` against `Fixed<<r>>` (OP), alone | conformance/vo-fixed-matched-against-variable | `kind`: `r` is a type |
| a tag arm narrows its element (OP), alone | checker/pos/tag-arm-narrows-element-to-family | `expected Element Count.* t10, found Element (rest \| Count.*) o` |
| `use Prelude.X as Y` (UA) | conformance/use-name-as | parse error at `as` |
| the seats that left, told to the program (SL) | tally (`!tellersLeft`, `@Table results`); library `!LeavesAt` | behind the library's errors |
| `row-lacks` per family instantiation, not per family | library `@HostRole` (`Seat.* d r \| rest`) | "`rest \| Seat.*` promises `rest` has no `Seat` tags" |
| `prompt` in a function's type, checked locally (PT) | neg/prompt-unmarked-widget-function, neg/prompt-function-where-not-prompt, neg/prompt-lambda-where-not-prompt, the three neg/sem-ch07-hold-updated-cycle-through-* (now annotated), pos/prompt-parameter-not-prompt; every annotated signature in the prelude, library, examples and tests | no errors; parse error at `prompt` |

## Waiting on Jimmy

| question | where it shows |
|---|---|
| PT is whole-function: `@Table` (prompt in `results` by its holds' updates) and `@HostDocument` (prompt in `seats` by the blueprint rule) make a prompt cycle in the tally's `@table`, though the hosted requests come from the participants' own buttons; defer in `@Table`, or something finer? | tests/documents/tally `@table` |

## Checker suite mismatches on p11 (tests/checker/run.sh)

| test | kind | why |
|---|---|---|
| doc-seats-record-of-rows | gap (RR) | row parameter read as a type |
| doc-sk-existential-seat-key | gap (VO) | Fixed at an associated kind |
| neg/brace-placeholder | gap | the saved patch: TpBadEscape and the BraceBlock span |
| neg/doc-c4-watcher-wires-request | gap | the outer `!` layer, and the note |
| neg/doc-seat-family-arm-answers-other-role | gap | supplier lambda arm not checked at the refined output |
| neg/doc-sk-opened-role-escapes | gap | alias name `SeatKey d` not kept |
| neg/forall-ambiguous-trait-method | gap | wording |
| neg/hold-needed-constructor-at-expected-type | gap | hold-needed not raised for a constructor at its expected type |
| neg/prelude-module | gap | library diagnostics when a program replaces Prelude |
| neg/sem-ch07-hold-updated-cycle-through-parameter-given-a-lambda | gap (PT) | parse error at `prompt \|..|`; the parameter's type says it is prompt |

## Conformance reds on p11 (lowering, older)

| test | why |
|---|---|
| sem-ch04-late-value-above-arm-with-rebuild-beside-inner-if | not-lowered: late plain result of an arm |
| sem-ch04-late-value-taken-by-arm-result-above | internal: name `n` undeclared |
| sem-ch04-late-value-through-local-function-let-above-clause-widget | not-lowered: late result above its giver |
| sem-ch08-blueprint-value-from-or-in-late-numbered-arm | not-lowered |
| sem-ch08-global-let-of-applied-widget-value | not-lowered: widget function as a value |
| sem-ch08-global-let-of-widget-value | not-lowered: widget function as a value |
| sem-ch08-widget-value-generic-through-associated-type | internal |
| sem-ch08-widget-value-generic-through-local-function | not-lowered: quantified variable without instantiation |
| sem-ch09-host-dynamic-answer-below-building-if-host-below-if | not-lowered |
| sem-ch09-host-late-plain-part-taken-above-bundle-result | not-lowered: late plain result in a bundle |
| doc-c6-hosted-bundle-result | transcript differs: the event part of the host bundle |
| doc-seats-record-of-rows | checker (RR), above |
| doc-sk-existential-seat-key | checker (VO), above |

## The rest of the p11 run (at 20004f18, before the library)

Regressions since p2 (green there):

| test | how it fails on p11 | needs |
|---|---|---|
| ts-ch02-regex-values | checks 4 and 5 compare a `Match` and fail; the transcript prints the match | equality of a `Match` (its `named` map, or a group list of optionals) |
| ts-ch03-assoc-in-impl-match | internal: a call of `Make` at a type the checker did not record | the result-only method change (20e2fb97) records no type at these calls; forall-three-jobs is the same |
| ts-ch03-impl-calls-at-its-part | internal: `?Zero` at an unrecorded type | the same |
| ts-ch03-impl-collection-head | internal: `Blank` at an unrecorded type | the same |
| ts-ch03-result-only-method | internal: `Blank` at an unrecorded type | the same |
| ts-ch03-result-only-self | internal: `?Read` at an unrecorded type | the same |

New tests, red:

| test | how it fails | needs |
|---|---|---|
| ts-ch03-associated-row | the widget method's label is at `0`; expected `0/0` | an inlined widget method still a node of its own (a448fdc5 flattens it) |
| value-of-promoted-case | not-lowered: `ValueOf` of a type variable | VO's lowering |
| ts-ch05-color-function-in-dynamic-through-lifted-call | not-lowered (red on p2 too) | a function value of its callee's color inside a call's result |

## Document trees (p11 check)

- **tests/documents/tally**, **c6-chess-by-hand**, **c8-chess-via**: every error traces
  to a row above (the library's features, OP, RT, RR, OS, the map literal). One
  error has no test of its own yet: `expected ?Nat, found ?(Answer (Tallied o) Viewer)`
  (tally app.prism:109). An associated type read through `impl ^Document (Tallied o)
  where o: ^TallyOrder`, by case equations, reduces on p11 in a small program at
  20004f18, so it is likely a cascade from the library's own errors; it is rechecked
  once the library checks. The tally's `no impl of ^TallyOrder for o` was its own
  slip: its participants now say `where o: ^TallyOrder`.
- **examples/chess**: in its final form (6378d7f1, then OP's supplier and `@Serve`); on p11 it stops on the kinds of error c8 has, and on the two unconfirmed rows above.
