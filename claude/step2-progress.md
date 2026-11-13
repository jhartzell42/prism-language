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

- A widget method whose parameters hold only associated types finds its impl and its
  default by the callee's type at the call (`?WidgetMethodImpl`, lower/node/static).
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

- The CLI World answers `Net` over TCP (worlds/cli/rust/src/net.rs; `cargo check` clean).
- A `forall` value applied packs its arguments and unpacks its result (`PackedApply`,
  `prism_rtl::unpacked`). Not yet: making such a value from a generic function by name.

## Claude's calls that change no semantics (Jimmy, 13 November: "MB/LR decisions that
## don't influence semantics are yours, if they're dicey put them in the handover doc")

- MB, dicey: the Rust back end monomorphizes. Each generic definition becomes one Rust
  function per instantiation the program reaches (the fixed point `?DefInstances`
  already computes them), so every `d` is concrete in Rust, an indexed associated type at
  a known role is a known type, and an existential over `r: Role d` is opened by a match
  over `d`'s roles, each arm lowered at its role. Why dicey: it is the largest change of
  step 2, it grows the Rust code, and a program whose instantiations do not end is
  refused by the growth check already (ruling PG), so nothing new is refused. Refused
  alternative: erasing such values to `Rc<dyn Any>`, which fails at run time where the
  checker was right.

## Asked of Jimmy

- MB: the Rust representation of values whose type depends on a variable (existentials
  over role variables; indexed associated types at a variable). Claude recommends
  monomorphizing over the instantiation fixed point. Blocks: existential lowering,
  doc-c4 family associated types, doc-sk, the library's builtins.
- The hold cycle through a parameter given a lambda (neg/sem-ch07-hold-updated-cycle-
  through-parameter-given-a-lambda): Claude recommends following the lambda at the call.

## Left

- FA call sites: a value of a `forall` type applied in Rust must pack its arguments and
  unpack its result (`Packed::of(..).packed()`, `.held::<T>()`); not written.
- Lowering, waiting on MB: existentials (`any`), family associated types per
  instantiation (doc-c4, every-role), doc-sk.
- Lowering: partial application (doc-c5). Where a widget function parameter is given
  `@Counter 5` (`ApplyWidgetParam`, lower/node/static), the plan is to read it as a lambda
  of the missing parameters whose body is the full call; the obstacle is that the line's
  arguments and the partial call's own are in two files' environments and spans.
- Done after: doc-c6 (`BindHosted` binds a tuple pattern part by part). Its cause, in p11's Rust for the test (compiled in the scratch tree,
  `target/c6t`), the hosted child's `!inc` handler (`inc_e0_3`) sets its own hold and
  fires its own port, but the parent's `!clicked` (`clicked_e1`, `clicked_e1_v_port`) is
  a port nobody fires: the event part of a `@Host`'s result bundle is bound apart from
  the child's event (`BindParts`, lower/node/def.prism, handles the inlined-call case;
  the host's result is another path).
- The library's builtins with no body: `SeatPathOf`, `$Merge`, `SeatIdAt`, `$GuestsOf`,
  `@Answer`, `!Outgoing`, `?ResponseFrom`, `$?AnswerFrom` (their types depend on MB).
- The checker red doc-c4-watcher-wires-request: the mismatch at a tag's sigiled field
  shows the peeled types (`Never`, wanted `!Never`) and has no note line; diagnostics have
  no notes yet.
- The ten old lowering reds (sem-ch04/08/09 late values, widget values).
- Then: the first check of the library, the tally, c6, c8 and chess with the new
  compiler, and their remaining errors (the prompt-cycle, the agent's two unconfirmed
  reports).
