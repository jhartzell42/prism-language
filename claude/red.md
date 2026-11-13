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

Kinds: **gap** a compiler feature or fix to write in step 2; **Q3**/**Q4** waits on
Jimmy's questions of 13 November (one role per place; two row variables).

## Features, each with its test

| feature (ruling) | test(s) | on p11 |
|---|---|---|
| empty impl, header alone (EI) | conformance/subtrait-default-for-method-above, neg/default-from-two-rungs | parse error |
| sub-trait default for a method above (SD) | conformance/subtrait-default-for-method-above | parse error (EI first) |
| two rungs' defaults are `ambiguous` (SD) | neg/default-from-two-rungs | parse error (EI first) |
| own declaration shadows `use X.*` (OS) | conformance/own-declaration-shadows-use | `collision` on `Kind` |
| row parameter needs no kind (RR) | doc-seats-record-of-rows (checker and conformance); library `FileSeats`, tally, c6, c8 seats | "a record has no row of its own"; "`Ui.*` is a row, not a type" |
| associated row default, `row Lower = unit` in a trait | library ^Document (tally, c6, c8) | "a trait declares an associated row ...; an impl chooses it" |
| `type State = unit` default; `@Request`/`@Query` on the state (RS) | library ^Document/^StateDocument (tally, c6, c8) | `^StateDocument` has no method `@Request` |
| `forall o.` in a type alias body | library `Supplier` | "`o` is no parameter of `Supplier`" |
| a supplier lambda's arms checked at the refined `o` | neg/doc-seat-family-arm-answers-other-role; library `SupplierFrom`, `@OnDisk`, `@Over` | arm results mismatch `o` |
| binding-group cycle through a function value (book ch. 5: "the two bindings refer to each other, and that is legal") | library `@HostDocument` (every document test) | `prompt-cycle` `$guests`, `Supply` |
| Fixed narrowing at an associated kind (VO) | doc-sk-existential-seat-key; c6 `Stepping` | not-exhaustive; `SeatId ChessGame r` not narrowed to `Color` |
| `ValueOf` on a variable (VO) | (to write with Q3's answer) | |
| prelude `Utf8Bytes`, `~Utf8Text` in the runtime | library `@OnDisk` (tally's saved-game test, chess) | unknown name before 589eaa5e |
| Net and File worlds in the runtime | library `@Over`, `@Serve`, `@OnDisk`; chess (agent writing) | not reached |
| existential lowering | existential-unbounded-index-list | not-lowered |
| family associated types in lowering, a variant per instantiation | doc-c4-seat-family-indexed, doc-seat-family-every-role | not-lowered |
| partial application lowering (AF) | doc-c5-widget-function-partly-applied | not-lowered |
| default widget method inlined with its parameter substituted | doc-state-document-bound-and-default | internal: no value entry |
| `Step<<d>>` call recorded at its type | forall-three-jobs | internal |
| `{ $x }` placeholder escapes (wip-brace-placeholder.patch) | neg/brace-placeholder | wording/span |
| alias name kept in messages | neg/doc-sk-opened-role-escapes | message names the expansion |
| a map literal keeps its values whole, as array literals do (HD) | conformance/hd-map-literal-keeps-dynamic-whole; the seats of tally, c6, c8 | `expected Text ==> $Nat, found Text ==> t9` |
| `row-lacks` per family instantiation, not per family | library `@HostRole` (`Seat.* d r \| rest`) | "`rest \| Seat.*` promises `rest` has no `Seat` tags" |

## Waiting on Jimmy

| question | where it shows | on p11 |
|---|---|---|
| Q3: one role per place (the seat and the element at one `r`) | library `PendingAt`, `!HeardAt`, `LeftWith`, `?ResponseTo`, `$ReadLive` | `expected ((Fixed r, SeatId d r, Request d r)), found` the same text at another `r`; existential-escape |
| Q4: two row variables, `emit lower \| rest` (book ch. 5) against neg/row-two-tails | library `@HostDocument` | `row-two-tails` |

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
| neg/sem-ch07-hold-updated-cycle-through-parameter-given-a-lambda | gap | cycle through a parameter given a lambda |

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

## Document trees (p11 check)

- **tests/documents/tally**, **c6-chess-by-hand**, **c8-chess-via**: every error traces
  to a row above (the library's features, Q3, Q4, RR, OS, the map literal), and to one
  more gap: an associated type read through an impl whose head has a variable,
  `impl ^Document (Tallied o) where o: ^TallyOrder` with `type Answer r = Nat`, is not
  reduced at `Answer (Tallied o) Viewer` (tally app.prism:109, `expected ?Nat, found
  ?(Answer (Tallied o) Viewer)`). The tally's `no impl of ^TallyOrder for o` was its own
  slip: its participants now say `where o: ^TallyOrder`.
- **examples/chess**: being rewritten to the final form (step 1, an agent); listed when it lands.
