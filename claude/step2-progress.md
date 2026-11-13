# Step 2: the compiler catching up to every failing test

The loop's step 2 (RULES.md, 13 November): every change to prismc/ for every test in
claude/red.md, in one pass, type-checked against prismc-p11 with the library before its
final form (shared/loop/chkc.sh, about a minute), and no build until the list is done.
Written by Claude on the claude-self-hosting branch; subject to change.

## Done (type-checked, not built)

- The saved brace-placeholder patch (TpBadEscape, the brace block's span).
- EI: an impl that is its header alone.
- SD: a sub-trait's default for a method above. `TraitInfo.aboveDefaults`, keyed
  `"{cls} default {method}"`; `AboveDefaults` (env/decls) gives each impl the lowest
  rung's default as `ImplInfo.aboveChosen`, `ambiguous` for two unrelated rungs; the
  default is checked with `AboveScheme` (check/mod); lowering: `ImplDefOf` writes the
  method from the default's clauses with the rung's parameter read as the head,
  `MethodTargets`/`ImplTargets` instantiate it, and inlined widget methods take it
  (`?ImplWidgetClauses`, `?WidgetDefaultSubst`, `InlineWidget` wrapping
  `InlineWidgetBody`).
- OS and UA: own names shadow `use X.*` and the prelude (`Shadowing`, env/names);
  `use X.Name as Y` (`Use.?alias`).
- VA: "variant" in diagnostics, `assoc-variant-missing`/`-duplicate`.
- HD: a map literal keeps its values whole (`ApplyRuleOf`'s `wholeNow`).
- OP: `Fixed<<r>>` against a variable (`FixedAgainstVariable`); a tag arm narrows its
  scrutinee to the tag's family (`Narrowed`).
- DE: a derive's equalities (`Bound.?equals`, `ViaDecl.bounds`, `BoundAt`/`TypeAt` in
  reading.prism, `ImplInfo.equalities` checked in `ImplsComplete`).
- `unit` as the empty row; an associated row's default (`TraitInfo.assocRowDefaults`,
  `ImplRowDefaults`).
- Utf8Bytes, ~Utf8Text in the Rust prelude (`utf8_bytes`, `utf8_text_try`; the names
  are a guess at the lowering's naming, to confirm at the build) and `Prelude.NotText`.
- Result-only calls: `?EitherSelf` (lower/expr) replaces an `Or` over optionals, which
  kept a `nil` and made five ts-ch03 tests and forall-three-jobs "a call at a type the
  checker did not record".
- FA inside types: `TyForall` (check/types/mod) through substitution, zonking, showing,
  unification (`OpenForall`), a fresh instance at each use of a local (`InferNameIn`);
  Rust holds its variables as `prism_rtl::Packed` (`OutTyOf`).
- RR: a type's parameter written only in a row's place takes a row
  (`TyInfo.rowParams`, `RowParamsOf`; the argument held as `TyElemAny rw`; `ParamInst`
  puts it in `rows` as `TlRow rw`, which `SubstRowQ` merges). `UsesScope` no longer
  reads a blueprint that writes its own `emit` as using the module's row.
- RT: rows of several variables (`TlUnion`, `UnionTail`) through `ConvRow`,
  `SubstRowQ`, `WalkRow`, `UnifyRowRest` and `DischargeOne`.
- Test slips found on the way: ts-ch02-regex-values (a map shows as `{{..}}`),
  ts-ch03-associated-row (a widget call's one element is at the call's place); library:
  a supplier's answer emits nothing, `emit unit`.

- Kinded type variables at run time: a function's hidden leading `{r}_variant: &Nat`
  parameters (`KindsOf`/`KindsOfVal`, lower/functions; `lx.kinded`), given at each
  declared call (`KindArgs`, lower/expr, from `?ParamMatch` of the callee's type there);
  `ValueOf<<r>>` (`?KindedVariant`, an if-chain from the number), `Fixed<<r>>`
  (`?KindedNumber`), and the `Fixed<<r>>` pattern (a guard). Not yet: a kinded function
  as a value or partly applied (no hidden arguments there), a widget function's.
- Checker reds: the ambiguous result-only call names `Step<<d>>`
  (`?OnlyThroughAssoc`); an escaping existential by its alias (`AliasOrShown`;
  `?MatchHead` reads associated types, roles and rigids); a constructor argument takes its
  parameter's type first (`Env.?expect`); a program that is its own Prelude is checked
  without the library's other modules.

## Asked of Jimmy

- The hold cycle through a parameter given a lambda (neg/sem-ch07-hold-updated-cycle-
  through-parameter-given-a-lambda): Claude recommends following the lambda at the call.

## Left

- FA call sites: a value of a `forall` type applied in Rust must pack its arguments and
  unpack its result (`Packed::of(..).packed()`, `.held::<T>()`); not written.
- Lowering: existentials (`any`), family associated types per instantiation (doc-c4,
  every-role), partial application (doc-c5; a widget function as a value), default
  widget methods (doc-state), doc-c6's bundle event part, doc-sk's not-exhaustive.
- The library's builtins with no body: `SeatPathOf`, `$Merge`, `SeatIdAt`, `$GuestsOf`,
  `@Answer`, `!Outgoing`, `?ResponseFrom`, `$?AnswerFrom`; the Net and File worlds.
- The checker red doc-c4-watcher-wires-request: the mismatch at a tag's sigiled field
  shows the peeled types (`Never`, wanted `!Never`) and has no note line; diagnostics have
  no notes yet.
- The ten old lowering reds (sem-ch04/08/09 late values, widget values).
- Then: the first check of the library, the tally, c6, c8 and chess with the new
  compiler, and their remaining errors (the prompt-cycle, the agent's two unconfirmed
  reports).
