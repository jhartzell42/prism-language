# The refusals of prismc's back end

Written by Claude; subject to change. Every `Refusal` / `RefusalDo` message in
prismc/lower and prismc/emit, by file and line, with its class. Classified by reading; an
entry marked "unsure" was not tested. The last section, "Every refusal site", is the
whole list as a table; the sections before it say what each construct is.

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
- Lowered: a place of a tag whose answer has a plain part beside a `$` part, in a
  hosted scope whose host answers with a dynamic that has no value while its node is
  being made: the plain part is a late name, set where the place asks its first value
  (`LatePlaceParts`; sem-ch09-host-late-bundle-plain-part). A place below it with a
  `$` output or a bundle's state, whose constant field reads the part, asks its first
  value with the element read from the node (`ElementPlaced`,
  sem-ch09-host-late-plain-part-in-place-field). What stays of it
  (lower/node/def.prism `LatePlaceParts`: "the plain part .. of a late place's answer
  read at its block's build"): a place with a PLAIN output whose constant field reads
  the part (its first value is asked in the build, before the node; the output would
  have to be a late name as the part is), and any such place written above the place
  that gives the part. A supplier's dynamic answer, or a bundle answer's `$` part, that reads a
  structural dynamic (a `for`'s gathered result, a `rebuild`'s): the child's `$` places
  ask their first values where the construction step reaches the host's line
  (`LateFirstPlace`; sem-ch09-host-dynamic-answer-structural,
  sem-ch09-host-bundle-answer-structural-part); the host's own pushed place and a `let`
  below the host's line (sem-ch09-host-dynamic-answer-late); a name of the parent, read
  by a host in a row (sem-ch09-host-dynamic-answer-reads-parent,
  sem-ch09-host-bundle-answer-parent-part); an answer through a blueprint a `let` names
  (sem-ch09-host-dynamic-answer-let, sem-ch09-host-event-answer-let).
  lower/expr.prism (`DynamicBlueprint`, `WrittenPart`): a dynamic answer whose line
  made no getter for it and that reads a reader of the node with no live dynamic
  where it is written. G, an internal error: `DynamicAnswers` makes a getter for every
  dynamic answer in the supplier's text, a blueprint `let`, a blueprint parameter's
  argument and a function of the block the supplier calls
  (sem-ch09-host-local-supplier-dynamic-answer,
  sem-ch09-host-local-supplier-bundle-answer), except a held answer (one that reads
  a name the supplier's text binds, or a parameter of that function), and
  `HeldAnswerReaders` makes the live dynamic of every reader of the node a held
  answer names (host-bug-01, host-bug-02, host-bug-09,
  sem-ch09-host-local-supplier-dynamic-parameter and -clauses). lower/expr.prism
  (`?LiveArgument`): a `$` argument of a function of the host's block with no live
  dynamic in the supplier's place: G, the same reasoning. A supplier cannot be handed on as a parameter
  of a widget (its type is of every output `o`; the checker has no such parameter),
  and a method or a named function reads no name of the block.
- `@ x` outside a tag arm is lowered: plain data in `Answer`'s case of no tag, read by
  the place that takes it (sem-ch09-host-answer-without-arm); a value that is no plain
  data (a function) held as it is and read at the type of the line that binds it
  (`TaggedOrHeld`: sem-ch08-blueprint-of-function-outside-tag-arm,
  ts-ch05-color-function-in-blueprint-result).
  A bundle with such a part outside an arm is held whole, its events and dynamics as
  values (`PartsBlueprint`: sem-ch08-blueprint-of-bundle-with-function-outside-tag-arm).
  Given to a host's place by the arm that matched its tag, the held bundle is answered
  part by part, as the bundle written under the arm is (`?HeldPartsOf`, prism_rtl's
  `AnswerOf::held_parts`: sem-ch09-host-held-bundle-answer).
- A tag whose output holds a part that is no plain data (a function) is answered by a
  host, under the tag's arm part by part (sem-ch09-host-bundle-function-part-answer);
  `crate::Answer` reads such a part from its own case or from the case held whole
  (the emitters' `OutsideArm`), and a World's answer for the tag is a fault of the run.
- Every other bundle answer is lowered, any mix of plain, `$` and event
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
  a blueprint value is lowered whatever it gives back
  (sem-ch09-host-widget-answer, sem-ch08-blueprint-out-of-value,
  sem-ch08-widget-value-dynamic-parameter, -event-parameter, -blueprint-parameter,
  -event-result, -let, -dynamic-result, -bundle-result, -function-result), one whose `$` parameter holds a dynamic below its top
  (sem-ch08-widget-value-dynamic-of-dynamics-parameter), one written by `<=` in an `update`
  (sem-ch08-widget-value-written-in-update), one with a widget function parameter, a function
  value of its row, its argument a widget named or a lambda
  (sem-ch08-widget-value-widget-function-parameter), a generic
  one at each set of types its readings give it (sem-ch08-widget-value-generic), one
  that gives a bundle back under a tag's arm (sem-ch09-host-widget-bundle-answer), one
  whose block ends on a host's line (sem-ch08-widget-value-ends-on-host), a generic one
  read in a generic function's body, given that function's own type parameter
  (`?OpenWidgetBlueprintValue`: sem-ch08-widget-value-generic-in-function), however
  many generic functions deep (sem-ch08-widget-value-generic-ten-functions-deep), in a
  function that calls itself at other types
  (sem-ch08-widget-value-generic-function-at-other-type), in a generalized `let` or local
  function of a block (sem-ch08-widget-value-generic-in-generalized-let), in an impl's
  method at the impl's type parameter (sem-ch08-widget-value-generic-in-method) and at the trait
  method's own (sem-ch08-widget-value-generic-at-method-type-parameter). Left
  refused (`OpenWidgetValueWords`): a generic one read where a type parameter stays
  open and the program gives the definition it stands in no finite set of types
  the lowering can find (`DefInstancesAnswer`, `?GrowsFrom`: a line through a
  constraint an impl solves, or through a use whose types hold an associated type,
  that holds one definition three times, each one's types embedded in the next one's:
  `EndlessThroughImpl`, G, an internal error, since the checker's walk takes both
  kinds of step and gives `grows-without-end` for a line that grows
  (tests/checker/neg/impl-at-ever-larger-types, neg/associated-type-at-ever-larger-types,
  neg/derived-impl-at-ever-larger-types, neg/default-method-at-ever-larger-types); a
  line that settles is followed, sem-ch08-widget-value-generic-in-method-at-settling-types
  and sem-ch08-widget-value-generic-through-associated-type), or a use of the
  definition leaves a type parameter undecided (`UndecidedParameter`). A line of
  uses by name that gives a type parameter a larger type holding itself is the
  checker's `grows-without-end` (ruling PG: tests/checker/neg/function-at-ever-larger-types,
  neg/widget-value-function-at-ever-larger-types), so the lowering's stop at one
  (`EndlessByName`) is G, an internal error; types that grow by uses by name and then
  stay are followed (sem-ch08-widget-value-generic-function-grows-once). Also left: one read in a place `ValuedWidgets` does not look (a record's field read back and
  given on is a value already). One built in a hosted scope builds its hosted row
  (sem-ch09-host-widget-answer-hosted).
- lower/node/static.prism (`BlueprintValuesAt`) a widget defined in a block read as a
  value is lowered (sem-ch08-local-widget-passed-as-value, sem-ch08-local-widget-in-record).
  One whose places read a dynamic of the block is lowered
  (sem-ch08-local-widget-value-reads-dynamic), and one in a hosted scope and one that
  gives a plain value, an event, a dynamic, a bundle or a function back
  (sem-ch08-local-widget-value-gives-back, -gives-event, -gives-dynamic, -gives-bundle,
  -gives-function). One a
  `let` names is the same value (sem-ch08-local-widget-value-let). lower/expr.prism
  (`NameValueOf`) "a widget
  defined in a block read as a blueprint value" stands for a reading outside a line's
  value places; a reading in the body of a function of the block is one of the line
  that calls it (sem-ch08-local-widget-value-in-local-function), and no program is
  known that reaches the refusal.
- lower/node/static.prism (`AppliedBlueprintLine`) a blueprint value applied by a block is
  lowered, and so is `@Element` of any element value (a parameter, a call's result;
  sem-ch08-element-parameter-statement): a line of unit, a line that gives an event
  back, which hears the blueprint's answers, and a line that binds a plain result
  (sem-ch08-blueprint-value-gives-back), in a hosted scope too
  (sem-ch09-blueprint-value-hosted), and a line that binds a dynamic
  (sem-ch08-element-value-dynamic-statement), and a line that binds a bundle
  (`AppliedBundleLine`: sem-ch08-element-value-bundle-statement,
  sem-ch08-widget-value-bundle-result). A bundle answered as a generic tag's is read
  through no tag (a fault of the run: no test).
- lower/node/static.prism (`ApplyWidgetParam`) a widget function parameter applied to
  arguments is lowered whatever its argument: a named widget function
  (sem-ch08-widget-parameter-applied), a lambda
  (sem-ch08-widget-parameter-lambda-argument), any other function value
  (sem-ch08-widget-parameter-call-argument).
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
  The name is bound before the block's lines (`PreBindWhole`), so a line above the split
  reads it (sem-ch08-late-part-bundle-named-read-above), and a `let` of a bundle written
  out over a late name is late (`LateBundleLet`, sem-ch04-late-let-bundle-above).
  A `let` whose tuple pattern takes a late bundle apart names its plain parts late
  (`LatePartNames`, sem-ch08-late-part-bundle-let-pattern-above); a late bundle read
  as one value from the node's methods is a getter over its parts (`LateBundleValue`,
  sem-ch08-late-bundle-read-as-value-above); a `let` of a bundle read by a line above
  it is lowered above that line (`BundleLetsEarly`, sem-ch04-let-bundle-read-above,
  sem-ch04-late-let-bundle-read-above).
  A pattern inside the `let`'s tuple pattern over a late plain part names its plain
  names late too (`let ((((step, noun)), !go)) = both`,
  sem-ch08-late-part-bundle-let-nested-pattern-above).
  A pattern there that binds no name (`let ((((ignore, ignore)), !go)) = both`) reads
  nothing (sem-ch08-late-part-bundle-let-nested-pattern-above-nameless).
  A bundle bound to one name and passed whole to a function (`pair <- @Two`, then
  `$First pair`) is one value made at the line that passes it, a field of the node
  (`BundleArgumentsAt`, ts-ch02-named-bundle-as-argument).
  Left, no test: a late bundle with a bundle inside read as one value.
  Left, R: the build-side markers of a late name (lower/node/def.prism `BindLate`,
  `MarkLateLets`: "the late result .. read at its block's build"; `PreBindWhole`,
  `MarkLateBundle`: "read as one value at its block's build"; `PreBindWholeParts`,
  `LateBundleParts`: "a late plain part of the bundle ..") are reached by one kind of
  line: a place whose first value is asked in the block's build (a plain output, or a
  `$` output outside a late host's scope) and whose constant field reads the late name
  (`c <- Pad.Count(name="count {n}")` above `n <- match ..`). Test, red:
  sem-ch04-late-value-above-in-plain-place-field. An element's field with no output
  asked at the build is live, a `let` over the name late, a hold that starts from it
  set late, and a building line that takes it a deferred row
  (sem-ch09-host-late-bundle-answer-above).
  A dynamic part of a named bundle read by a row of a row is read through a getter of
  the row between (`ThroughGetter`, sem-ch04-named-bundle-in-row-of-row).
- A `rebuild` whose block ends on a bundle a line named whole (`@ both`) is lowered
  (`WholeNamedOf`, sem-ch04-rebuild-named-bundle-result), a bundle inside that bundle
  too (`WholePartsOf`, sem-ch04-rebuild-nested-bundle-result), a named bundle inside a
  tuple written out (sem-ch04-rebuild-bundle-in-tuple-result) and a block that ends on
  the line that gives the bundle (`WholeOfResults`, sem-ch04-rebuild-ends-on-widget-bundle).
- A widget's parameter pattern is lowered: a tuple pattern over a bundle written out
  or named, any pattern over a plain value (sem-ch08-widget-parameter-pattern), a tuple
  pattern over a bundle that is a value (sem-ch08-widget-parameter-pattern-bundle-value),
  a pattern that binds an event or a dynamic out of a value that is no bundle
  (sem-ch08-widget-parameter-pattern-record-dynamic), and a pattern that names a bundle
  of its own inside a value (`fn @W ((id, both)):` given a function's result,
  `PreBindBundleBinder`; sem-ch08-widget-parameter-pattern-bundle-inside-value).
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
  `let` (of one name, or of a pattern whose names are plain:
  sem-ch04-late-let-pattern-above) and a hold's first value are late in turn
  (sem-ch04-building-match-read-above, sem-ch04-building-match-hold-above). A `let` of
  a bundle written out that reads the late name is late (`LateBundleLet`,
  sem-ch04-late-let-bundle-above). A building `match` or `if` above that reads the
  plain result of a building `match` or `if` whose arms give it from nothing their own
  lines build takes it as a value of the build (`IsPureChoiceAt`, `PureChoiceValue`:
  sem-ch04-building-if-on-late-value-above, sem-ch04-building-match-on-late-value-above).
  The same holds for any other line above that takes the result at its build (a
  widget's plain argument, a builder's rows: `IsPureChoiceTakenBy`,
  sem-ch04-late-value-above-clause-widget-result). A World's place with a plain answer
  bound to a name whose constant field reads the late name asks its first value where
  the row that gives the result is built (`LateFieldPlaces`,
  sem-ch04-late-value-above-in-plain-place-field). A line that takes the late name
  and binds a plain result of its own to a name gives a late name in turn
  (`LateTakerResults`).
  Left refused: a `let` of a type with time inside that is no bundle and reads the late
  name (the `unbuilt` refusals of `BindLate`, `MarkLateLets`); a place whose constant
  field reads the late name and whose answer is a bundle, is bound by a pattern, or
  stands in a hosted scope (`BindLate`'s `unbuilt`: R, no test; a `$` answer is asked
  late as a plain one is, `LateAskedPlace`: sem-ch04-late-value-above-in-dynamic-place-field); the late
  names of `LateFieldPlaces` and `LateTakerResults` read at the build by a line that
  takes a value there and is not deferred (their `unbuilt`: unsure G, as
  `MarkLateLets`'); and in `DeferredLine`, a line that takes the late name and whose
  places are counted as built, where the late result is no pure choice's and is not
  given by a building `match` or `if` of the block's own scope the block is split at
  (that one is lowered: its rows built first with no `base`, numbered at their place,
  `LateGiving`; sem-ch04-late-value-above-clause-widget-result-from-arm-lines): a late
  `let`, a clause widget's late result, a hosted scope, a row that builds its
  generations at its build (R, no test), and one whose plain result no name binds (a
  pattern's, or the block's own result. R, no test).
  A line whose late names are all plain parts of a late place's bundle
  (`{name}#lateplace`) is neither deferred nor refused: it is built in its place's
  order, and one with a plain result splits the block (`?LatePlaceTakerAt`:
  sem-ch09-host-late-plain-part-clause-widget-result,
  sem-ch09-host-late-plain-part-building-match-result), or, written above the place
  whose part it takes, binds its result to a late name
  (sem-ch09-host-late-plain-part-taken-above). A late place's plain part under a
  pattern of its own binds the pattern's names late
  (sem-ch09-host-late-plain-part-nested-pattern). Not lowered and not a refusal of the
  back end's: such a line written above the place, whose clauses differ in width, so
  that the place's own path reads its own answer (a host's supplier is handed the
  path); it stops at `PlaceNumber`'s refusal below. Unsure: C.
- lower/jobs.prism: none added. A bare `for` in a job is lowered (`ForJob`), no test of
  the job form; the plain form is sem-ch05-update-for-bare.
- lower/node/finish.prism (`PlaceNumber`) a first-value query at its block's build,
  before the node exists, after a building `match`, `if` or widget of several clauses
  whose rows differ in width and are built after the node. R, no test. A host there is
  lowered: made at the root and placed once the node stands (`PlaceUnknownAtBuild`,
  prism_rtl's `Host::place_at`). The `row_base` of a line built early there
  (`EarlyChoiceLine`, `EarlyClauseLine`) is lowered: the row is numbered after the node
  (`NumberedLate`, sem-ch04-building-match-on-built-late-value-above).
- In a row numbered after its block's node (`Wx.baseLate`): a host in the arm is
  lowered (sem-ch04-building-match-on-built-late-value-above-host-in-arm), and so is
  the row's plain result read from a building `match` inside the arm (`Wx.lateGens`,
  sem-ch04-building-match-on-built-late-value-above-inner-match). Left refused, all
  R with no test: lower/node/finish.prism (`PlaceNumber`) a hosted node's own
  first-value query spelled before its node in such a row;
  lower/node/elements.prism (`ElementOutputOf`) a hosted place with a plain answer
  that no plain name binds, under a host placed after its node (bound to a name it is
  lowered: `LateHostedAnswer`,
  sem-ch04-building-match-on-built-late-value-above-host-answers-in-arm);
  lower/node/switch.prism (`ArmRowOf`) the row's
  late plain result beside a `for`, a `rebuild` or a widget that calls itself in the
  arm, and a widget that calls itself in a row that builds its generations before it
  is numbered.
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
- lower/node/static.prism (`LocalFunctionLine`): no refusal. A function of a block that
  gives a blueprint is applied as a line, and where a widget it is handed to applies it
  (sem-ch08-block-function-applied-as-line, host-bug-15); one of several clauses as a
  building `match` over its plain arguments
  (sem-ch08-block-function-clauses-applied-as-line); one that applies itself level by
  level, sixteen deep, a build fault past that as for a widget that calls itself
  (sem-ch08-block-function-applies-itself).
- lower/node/static.prism (`WidgetCall`) a widget this back end does not lower yet: no
  program found since a blueprint taken out of a value by a pattern is applied
  (`ValueBlueprintLine`, sem-ch08-blueprint-out-of-value) and a function value a `let`
  of the block names is applied by a line (`AppliedFunctionLine`,
  sem-ch08-function-value-let-applied), an unbounded function's value too
  (sem-ch08-unbounded-function-value-gives-blueprint). A blueprint a `let` names is
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
  at the build (LF). No program found: an outer written in place, in the block or in a
  row, and an outer bound below are lowered (sem-ch07-flatten-call,
  sem-ch07-flatten-expression-in-row, sem-ch07-flatten-held-above).
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
- A widget block's `let` of a pattern that is no name and no tuple of names is
  lowered: with plain names (sem-ch08-let-pattern-in-widget), with dynamics inside it
  and with events inside it (`LetPatternDynamics`,
  sem-ch08-let-pattern-dynamic-in-widget, sem-ch08-let-pattern-event-in-record). Left:
  the value taken apart is a nested bundle a line named whole (`let ((ten, ((!more,
  $guests)))) = all`, sem-ch08-let-pattern-event-in-tuple: lower/expr.prism, "a tuple
  pattern's binding as a value").
- lower/expr.prism `Recolored`: a function value of its callee's own color inside the
  value a call returns, at a call that makes the color bounded, is made again through
  every type that can hold it (ts-ch05-color-function-in-result, -in-map-result,
  -in-named-result, -in-choice-result, -in-nested-named-result, -in-named-in-tuple-result,
  -in-optional-result, -in-result-result, -in-function-parameter-result), beside a
  function that stays unbounded (ts-ch05-color-function-beside-unbounded-function-result)
  and in the result of a function that stays unbounded
  (ts-ch05-color-function-in-unbounded-function-result). A type alias is its target in
  every type the checker gives (`ConvNamedType`), so `NamedRecolored` meets none (G, an
  internal error). `Recolored`'s refusal of a type that is none of those is reached
  only where the call's type does not match the callee's signature (no program found).
  `FunctionRecolored`'s refusal is of a function whose color is a generalized `let`'s
  that no use decides (`OutQuantified`, refused where its type is written). A function under an
  event or a `$` inside the result is mapped (ts-ch05-color-function-in-event-result).
  A function held by a blueprint inside the result is made again through the
  blueprint's answers (`?Made F = ? @ F`, the prelude's `held_answers_mapped`:
  ts-ch05-color-function-in-blueprint-result).
- lower/node/def.prism (`RefusePlainResult`) a widget's plain result bound by a pattern
  that left a name without its field: an internal error; `BindPatternResult` takes every
  pattern (sem-ch08-plain-result-pattern passes).

Jobs and templates
- lower/jobs.prism:291, 292 a cycle-point `while` in another expression or statement:
  a G, no program reaches this: `StmtJob` is entered only for a `while`, `if`, `match`
  or `for` (`HoldsCycleWhile`), and a nested `update` holding the loop compiles.
- lower/templates.prism (`PlaceOf`) a reference path this back end does not lower: a G, no
  program reaches this: the checker refuses any other left side of `<=` (mismatch); a
  named lens applied is lowered there (`LensCallEx`). Beside it, "the place of a lens that
  ends at an index": a G, since such a lens's part is `?a`, which the checker gives no
  `+=`, no `for` and no lens over it, and a set and a read go whole (`set_part`, the
  lens's read function). In `LensDefs`, "a lens whose body is a block" and "a lens
  without a clause": Gs, the checker's `lens-body` and `no-definition`.
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
  function's value; a suffixed operator, a monad block and a lens value (`&.score.home`) stop at
  the parser or the checker (`unsupported`).
- lower/expr.prism:515 a lift over a sigil of {kind}. No program found: the checker
  lifts only at applications, operators, fields, indexes and interpolations, each a
  direct operand.

Zombie and dead states: `leave`, `clean_up`, `edges` (no refusal).
They are the runtime's names (claude/backend.md, "Leaving"); a program writes nothing
for them (Semantics ch. 4: "no lifecycle events"). What a program can see is Semantics
ch. 7, "Holds die with their widget": a kept dynamic of a widget that left reads its
holds' last values. A `for` row whose key leaves and a replaced generation fire in their
last tick and are silent afterwards (sem-ch04-row-teardown, sem-ch04-generation-teardown
pass), and a kept hold of a row that left shows its last value while nothing reads it
again; a kept combinator over that hold and a live input outside the row reads the
hold's last value and the input live: a live dynamic of a node's hold reads `{name}_last`,
written by the node's `leave` (sem-ch07-hold-dies-with-its-row passes).
The first value of an element value placed in a hosted scope (DH2) holds through two
hosts (sem-ch09-host-described-first-value passes).

## rustc refuses prismc's output

No refusal: prismc writes a crate and rustc refuses it. Each is an R, critical. None is
known: a tag's plain field that holds dynamics below its top is lowered
(sem-ch09-tag-field-array-of-dynamics), and so is a no-arm building `match` bound as a
dynamic or an event (sem-ch04-building-match-no-arm-bound-live). Every `compile_error!`
the emitters write begins "prismc does not lower", so `CompileProgram` reports it.

## C: wrong programs the checker lets through

- lower/node/def.prism (`BuildInOrder`) lines that read each other at the build: an
  internal error; the checker gives `prompt-cycle` (tests/checker/neg/build-cycle).
- lower/expr.prism:576 `{b}` outside a `for`. Test:
  tests/checker/neg/bound-word-outside-for.
- A fault kind with no field applied, `Fs.NotFound()`: a checker error (its value is the
  tag). Test: tests/checker/neg/fault-kind-applied. The back end has no refusal here: a
  kind with fields is lowered (`fault_kind_of`, claude/backend.md, "A fault kind as a
  value").
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
- A method that needs one dictionary for a whole collection or for no value at all, asked
  of `any ^Trait` (the trait's type inside another type, `fn [a] -> Nat`, or in no
  parameter, `Empty: a`; the trait's type inside its result, `Zero: fn a -> Unit a`
  under an associated type, tests/checker/neg/ts-ch03-existential-method-gives-
  associated-type; a type parameter of its own): the checker's
  `existential-method` at the call (ruling EX; tests/checker/neg/
  existential-method-needs-one-dictionary). Packing the trait is legal and is lowered:
  such a method is no entry of the dictionary and has a body no program runs in the
  trait's impl for the packed value (`PackedDef.unasked`;
  ts-ch03-existential-other-method-not-asked). A trait with an associated type is packed
  too: only an unasked method names one, so the impl for the packed value chooses the
  unit type for each (`PackedDef.assocs`; ts-ch03-existential-associated-type). Every
  trait of the prelude is packed (ts-ch03-existential-num-packed, ts-ch03-existential-pod-packed).
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
  widgets; a longer one would reach it). `@Element e` of an element value that answers
  plain data is late (sem-ch08-element-parameter-plain-result reached the guard).
- lower/expr.prism:667 a supplier's dynamic answer of no known form. No program reaches
  this: `?DynamicAnswer` takes every dynamic but `$ plain`, which the line above takes.
  The nearest program, `@ $~Shout $motd`, passes: sem-ch09-host-dynamic-answer-call.
- lower/expr.prism:670 a supplier's answer under a tag the arm does not name. No program
  reaches this: `#tag` is bound only by `TagPat`, always as a path.
- lower/expr.prism:723 `@Element` without its element. The checker refuses it:
  tests/checker/neg/element-without-its-element.
- lower/expr.prism (`NameValueOf`) an event read as a value: an event's name with no
  value port. Every event a block binds has one (`PreBindName`, `BindEventParam`), a
  row reads its block's (`WholeOfBlock`), and an event parameter of a widget that calls
  itself is the `Event` its row holds (sem-ch08-widget-recursion-event-parameter-read-as-value).
  No program found for what is left (a bundle's event part bound whole, by a pattern, or
  as a widget's parameter, in a block and in a row, each passes); not proved unreachable,
  so it stays a refusal.
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


## Every refusal site

Every `Refusal` / `RefusalDo` call in prismc/lower, by the function it stands in. Class
R: a legal program reaches it or is thought to. G: an internal error ("a prismc bug:
.."), with the reason no checked program reaches it. U: no program is known that reaches
it and no argument is written that none does; it stays a refusal until one or the other
is found. "Test" is the test that reaches the site today, red, or none.

| site | class | test | state |
|---|---|---|---|
| node/elements.prism `ElementLine`, an element of an unknown tag | G | - | internal error: the checker's `unknown-name` (tests/checker/neg/unknown-tag) |
| node/elements.prism `ElementOutputOf`, a hosted place with a plain answer no plain name binds, under a host placed after its node | R | none | refused |
| node/finish.prism `UpdatedGaps`, `!Updated` of a dynamic no sink of the node writes (UP) | G | sem-ch07-updated-rebuild-dynamic-above reached it | internal error: every dynamic a node binds reads a hold, a getter over its holds, a structural dynamic or a kept result; the program that reached it (a `rebuild`'s dynamic read above its line) was bound to a field the node did not have, lowered by `GivesResult` |
| node/finish.prism `UpdatedGaps`, the same in a row over a name of its block (SU) | G | the same test's second widget reached it | internal error, by the same argument |
| node/finish.prism `PlaceNumber`, two sites: a place read at its row's or block's build before its node, after rows that differ in width | R | none | refused |
| node/switch.prism `RebuildDynamic`, a `rebuild` that hands back a dynamic whose generation gives no dynamic, value or bundle | G | sem-ch04-rebuild-ends-on-host reached it | internal error: the one result left out, a host's line last in the generation, is lowered as the host's bundle (`GenRowOf`); `GivesDynamic` keeps events and unit away |
| node/switch.prism `ArmRowOf`, two sites: a late plain result beside a `for`, a `rebuild` or a widget that calls itself, in a row numbered after its node | R | none | refused |
| node/switch.prism, `@Flatten` with no live dynamic at the build (LF) | R | none | no program found: a global and a name of the block are lowered (`{name}_outer`; sem-ch07-flatten-global-dynamic, sem-ch07-flatten-block-dynamic-in-row); an outer written in place in a row is the block's live dynamic when it reads the block's names alone, else the row's own (sem-ch07-flatten-expression-in-row) |
| node/def.prism `SplitBlockInto`, a block split at a line whose result is not late | U | none | refused |
| node/def.prism, the build-side markers of a late name (`PreBindWhole`, `PreBindWholeParts`, `BindLate`, `MarkLateLets`, `LateTakerResults`, `LateFieldPlaces`, `LatePlaceParts`, `LateHostedAnswer`, `MarkLateBundle`, `LateBundleParts`: twelve sites) | R | none (sem-ch04-late-value-above-in-plain-place-field passes) | refused: a place with a `$` or bundle answer, bound by a pattern or in a hosted scope, whose constant field reads the late name; a `let` of a type with time inside that is no bundle |
| node/def.prism, a late plain result in a bundle (`WrLate`) | U | none | refused |
| node/def.prism `DeferredLine`, two sites: a line that takes a late result above the line that gives it, with counted places (where the giver is no building `match` or `if` of the block's own scope, `LateGiving`) or a plain result no name binds | R | sem-ch04-late-value-above-clause-widget-result-from-arm-lines (the lowered case) | narrowed |
| node/static.prism `WidgetCall`, a widget line whose name has no value entry | G | - | internal error: the checker's `unknown-name`; a `for`'s binder that is a blueprint (`for @each <- [[@One, @Two]]:`) applied as a line of the row is a value of the node (`WidgetLine`'s `held`; sem-ch08-for-over-blueprints) |
| node/static.prism `HostLine`, a host without its child or its supplier | G | - | internal error: the checker's `mismatch` (tests/checker/neg/ignore-binds-a-function, neg/builtin-widget-without-argument) |
| node/static.prism `InlineWidget`, a widget of several clauses that takes no arguments | G | - | internal error: the checker's `collision` (ruling ZD; tests/checker/neg/clause-widget-without-arguments) |
| node/static.prism `InlineWidget`, a generic widget at a call whose types its parameters' do not match | U | none | refused; the program that reached it is the checker's `field-variable` (tests/checker/neg/hosting-generic-implementation) |
| expr.prism `SampledObject`, a dynamic a function gives whole, read in a plain function (LE) | U | none | refused |
| expr.prism, a dynamic passed whole where it has no live dynamic (LF) | U | none | refused |
| expr.prism `LiftedValue`, a lift whose noted operands are not its own | G | - | internal error: the checker lifts only at applications, operators, fields, indexes and interpolations, each a direct operand |
| expr.prism `PlainValueByKind`, `{b}` outside a `for` | G | - | internal error: the checker's `unknown-name` (tests/checker/neg/bound-word-outside-for) |
| expr.prism `PlainValueByKind`, an expression of no value kind | G | - | internal error: `mismatch` for a `for` or a `rebuild` as a plain value (ruling BP; tests/checker/neg/for-in-let), `unsupported` for a suffixed operator, a monadic block and a lens value; a topic, `..` and an absent value are read by an `update` block, a pattern and an element's fields. Not argued: a widget block as a value |
| expr.prism `PartsBlueprint`, two sites (`PartRefused`) | G | - | internal error by its text: `DynamicAnswers` makes a getter for every dynamic answer |
| expr.prism `NameValueOf`, an event read as a value | U | none | refused |
| expr.prism `NameValueOf`, a tuple pattern's binding as a value | R | none | refused where a bundle bound to one name is read as one value in no function's argument, no generation's result and no `let` of it (a tuple or an array that holds it); passed to a function it is one value (`BundleArgumentsAt`, ts-ch02-named-bundle-as-argument) |
| expr.prism `NameValueOf`, a widget defined in a block read as a blueprint value outside a line's value places | R | none | refused; a lambda's body that is the widget is a value place (`BlueprintValuesAt`; sem-ch08-block-widget-given-by-lambda), one whose body chooses it (`\|c| if c: @Inner else: @Other`) is not |
| expr.prism `WidgetValueWords`, a widget as a value where `ValuedWidgets` does not look (two calls) | R (one call), U (the other) | none | refused; a widget defined as one expression that is another widget (`fn @main = @page`) is an alias and no longer reaches it (sem-ch08-widget-is-another-widget) |
| expr.prism `OpenWidgetValueWords`: `EndlessByName`, `EndlessThroughImpl` | G | - | internal errors: the checker's `grows-without-end` |
| expr.prism `OpenWidgetValueWords`: `UndecidedParameter` | U | none | refused |
| expr.prism `Recolored`, a function value held by a type that is none of those it maps through | U | none | refused |
| expr.prism `FunctionRecolored`, a function whose color no use of its `let` decides | U | none | refused |
| patterns.prism, a pattern inside an element's `$` field | G | - | internal error: the checker's `live-field-pattern` (ruling LP; tests/checker/neg/live-field-pattern) |

Totals: 36 `Refusal` / `RefusalDo` calls stand in prismc/lower. 2 are internal errors
by their text (`PartsBlueprint`); 4 are R with a red test (`AnyValue`, the named
bundle, the block widget, one `WidgetValueWords` call); 20 are R with no test (`@Flatten`
LF, which no program found reaches, the 12 late-name markers, `DeferredLine` 2, `PlaceNumber` 2, `ArmRowOf`
2, `ElementOutputOf`);
10 are U (`SplitBlockInto`, `WrLate`, the generic widget, LE, LF of expr.prism, an event
read as a value, the other `WidgetValueWords` call and `UndecidedParameter`, `Recolored`,
`FunctionRecolored`). Eleven guards are internal errors and no longer refusals.

Refusals outside `Refusal` / `RefusalDo`, which the table above does not hold:

| site | class | test | state |
|---|---|---|---|
| emit/rust/items.prism `DeepPodOf`: a level field's walk at a type that holds no time | G | - | internal error ("a prismc bug: a level field's walk met .."): arrays, maps, options, `~T`s, tuples, `Rc`s, dynamics and generic program types are walked (sem-ch09-tag-field-array-of-dynamics, -named-type-of-dynamic, -fault-of-dynamic) |
| emit/cpp/mod.prism `PrinterRefusal`: a constructor, record or pattern of no type the C++ printer's tables know | G | - | internal error ("a prismc bug: the C++ printer met ..") |
| lower/types.prism, `OutQuantified` (two sites): a quantified type variable no instantiation gives a type | U | none | refused; `Len [[]]`, `Size {}` and an unused `let empty = [[]]` are lowered |
| lower/expr.prism, `OutRefused "a parameter"`: a typed parameter pattern with no type | U | none | refused |

## Red tests of the depth sweep

Each is lowered (the transcripts by hand, one corrected by Type System ch. 4,
sem-ch07-flatten-constant-of-dynamic). sem-ch07-flatten-block-dynamic-in-row and
sem-ch09-tag-field-array-of-dynamics have their last fix written and type-checked, unrun.

Lowered in the sweep, unrun: sem-ch04-rebuild-ends-on-host (`GenRowOf`),
sem-ch07-updated-rebuild-dynamic-above and sem-ch04-rebuild-dynamic-read-above
(`GivesResult` takes a `rebuild`; before it rustc refused a plain read above the line),
sem-ch04-building-match-no-arm (`NoArmLine`; a dynamic or an event result too,
sem-ch04-building-match-no-arm-bound-live).

## Programs tried that reach no refusal

Under target/fx/q of the sweep's worktree, compiled by prismc-r21, each lowered with no
refusal. They are the attempts behind the R rows with no test and the U rows.

- The late-name markers, `DeferredLine`: a late plain result `n <- match mode:` read
  above its line by a place's constant field, a `for`'s source, a hold's first value, a
  `rebuild` over `$ n`, a `let`, a widget's plain argument, a clause widget's argument,
  that widget's plain result handed on, a tagged place's answer and its use, a mixed
  `((plain, $live))` answer, a `let` of a type with time inside, a local function and a
  lambda that close over it, a blueprint argument and a hosted child that read it, a
  widget of the block, `$ n` and `@Distinct ($ n)`, `n on !x`, a row's and a
  generation's hold starting from it (l1, l2, l3); a widget with a building `if` given
  it, and a widget whose plain pair a pattern binds (d1).
- `ElementOutputOf`, `PlaceNumber`, `ArmRowOf`: a hosted child with a plain answer under
  a host below a building `match`, in each arm of one that gives a late result read
  above, in a `for` row, and in a clause of a widget applied twice (h1).
- LE, LF of expr.prism: a whole-taking function given a hold, a dynamic `let`, a hosted
  answer, a clause widget's result, a bundle's parts, a `rebuild`'s and a `for`'s
  dynamic, `@Distinct`'s, a parameter and a global, in a block, in a row and in a
  supplier's arm (g1); the same inside a lambda and a local function (g2).
- An event read as a value: `!MergeAll` over a block's events, a widget's event
  parameter, a generation's and a row's own events and a switched one, in a block, a
  generation and a row (a3).
- Not tried: `SplitBlockInto`, `WrLate`, the generic widget, `UndecidedParameter`,
  `Recolored`, `FunctionRecolored`, and the four sites of the second table.
