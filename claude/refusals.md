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

Late names read at the build
- A late name (`{name}#late`) has a marker at the build where a build-side reader would
  read it. Three readers reach the markers, each with a red test per marker it reaches
  (the table, "Every refusal site"): a `let` of a lambda used at two types, which
  `LateLetNames` leaves out (`?ManyInstances`) and `LetLine` builds at the build; a
  World's place whose constant field reads the late name in a hosted scope, or with its
  bundle's state parts all `ignore`, which `LateFieldPlaces` does not take; a late bundle
  with a bundle inside passed whole to a function, which has no `{name}#value`
  (`LateBundleValue`), so `BundleArgumentsAt` reads its parts. Designs: the last section.
- A line that takes a late result above its giver, over a split line that is no pure
  choice: a taker counted as built that reads a taker's result, and a taker whose plain
  result a tuple pattern or `ignore` binds (`DeferredLine`'s two refusals); an arm
  numbered late that builds a `for` beside its own late result (`ArmRowOf`); a hosted
  `$` answer below a building line of its child (`PlaceNumber`). Each has a red test.

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
  where it is written. G, an internal error: `DynamicOutputs` makes a getter for every
  dynamic answer in the supplier's text, a blueprint `let`, a blueprint parameter's
  argument and a function of the block the supplier calls
  (sem-ch09-host-local-supplier-dynamic-answer,
  sem-ch09-host-local-supplier-bundle-answer), except a held answer (one that reads
  a name the supplier's text binds, or a parameter of that function), and
  `HeldOutputReaders` makes the live dynamic of every reader of the node a held
  answer names (host-bug-01, host-bug-02, host-bug-09,
  sem-ch09-host-local-supplier-dynamic-parameter and -clauses). lower/expr.prism
  (`?LiveArgument`): a `$` argument of a function of the host's block with no live
  dynamic in the supplier's place: G, the same reasoning. A supplier cannot be handed on as a parameter
  of a widget (its type is of every output `o`; the checker has no such parameter),
  and a method or a named function reads no name of the block.
- `@ x` outside a tag arm is lowered: plain data in `Output`'s case of no tag, read by
  the place that takes it (sem-ch09-host-answer-without-arm); a value that is no plain
  data (a function) held as it is and read at the type of the line that binds it
  (`TaggedOrHeld`: sem-ch08-blueprint-of-function-outside-tag-arm,
  ts-ch05-color-function-in-blueprint-result).
  A bundle with such a part outside an arm is held whole, its events and dynamics as
  values (`PartsBlueprint`: sem-ch08-blueprint-of-bundle-with-function-outside-tag-arm).
  Given to a host's place by the arm that matched its tag, the held bundle is answered
  part by part, as the bundle written under the arm is (`?HeldPartsOf`, prism_rtl's
  `OutputOf::held_parts`: sem-ch09-host-held-bundle-answer).
- A tag whose output holds a part that is no plain data (a function) is answered by a
  host, under the tag's arm part by part (sem-ch09-host-bundle-function-part-answer);
  `crate::Output` reads such a part from its own case or from the case held whole
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
  the lowering can find (`DefInstancesOutput`, `?GrowsFrom`: a line through a
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
- lower/node/static.prism (`InlineWidget`) a generic widget at a call whose types its
  parameters' do not match: a parameter whose type is an associated type at the widget's
  type variable (`@Shown: fn r, Move r -> @unit where r: ^Game`), whose argument's type
  is the impl's (`Step`), matched by nothing (ts-ch03-generic-widget-associated-type-
  parameter). `?ParamMatch` takes an associated type as deciding nothing, as
  `?InstanceMatch` does; a blueprint parameter and a function parameter are matched
  through (sem-ch08-generic-widget-blueprint-parameter).
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
  given by a line that numbers its rows late (lowered: `LateGiving`, the split line a
  building `match` or `if`, or a one-clause widget ending on one, in any scope built in
  the order of its places, read directly or through plain `let`s, in a row that builds
  its generations at its build too; sem-ch04-late-value-above-clause-widget-result-from-arm-lines,
  -late-let-from-arm-lines, -widget-result-from-arm-lines, -in-late-numbered-row,
  sem-ch09-host-late-value-above-from-arm-lines): the split line a recursing widget, a
  blueprint value or a bundle, the scope one built at its host's line, a late name of
  another kind (R, no test), and one whose plain result no name binds (a pattern's, or
  the block's own result. R, no test).
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
  lowered: `LateHostedOutput`,
  sem-ch04-building-match-on-built-late-value-above-host-answers-in-arm);
  lower/node/switch.prism (`ArmRowOf`) the row's
  late plain result beside a `for` or a `rebuild` in the arm, and a blueprint value
  applied in a row that builds its generations before it is numbered. A widget that
  calls itself there is lowered: its shared rows are numbered late too (`GivesLate`,
  sem-ch04-late-value-above-recursing-widget-result).
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
- lower/expr.prism:371 a dynamic passed whole to a function where it has no live dynamic
  (LF). No program found: a clause widget's result, a place's dynamic, a global, a row's
  parent's hold and a hosted child's result are read through the node's link
  (ts-ch04-whole-clause-result, -place-dynamic, -global-dynamic, -row-parent,
  -hosted-result); a `for`'s `$v` over `@PerKey` has no test.
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
  blueprint's answers (`?Made F = ? @ F`, the prelude's `held_outputs_mapped`:
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
dynamic or an event (sem-ch04-building-match-no-arm-bound-live), a lifted function's `$`
parameter given what a whole function gives (ts-ch04-plain-function-gives-whole-result-to-
sigiled-parameter, -lifts-over-whole-result), a time-taking lambda's own `$x`
(ts-ch04-whole-lambda-reads-hold-in-lifted-call), and a supplier's answer that holds
dynamics (sem-ch09-host-answer-holding-dynamics). Every `compile_error!`
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
  this: `?DynamicOutput` takes every dynamic but `$ plain`, which the line above takes.
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
- lower/expr.prism:1031 an output constructor of no tag or with no argument. The nearest
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
| node/elements.prism `ElementOutputOf`, a hosted place with a plain answer no plain name binds, under a host placed after its node | G | - | internal error: such a place's line binds one name (`?BindingOf`), and `LateHostedOutput`, run over every line of the block in `LinesInto` before `PreBind`, makes it a late hold (`LatePlainBn`) under the same host (`HostLine` puts the host in `unplacedHosts` before it lowers the child), unless the answer is a `$`, an event or a bundle (no `BnLocal` binding) or the name is late already (`LatePlaceParts`, a `LatePlainBn` too); a `$` name over a plain answer is the checker's `mark-stack`. Probes eo1-eo5, m4 (the child's result is the answer) lower; sem-ch09-host-late-answer-holding-dynamics is ok on prismc-fd1 |
| node/finish.prism `UpdatedGaps`, `!Updated` of a dynamic no sink of the node writes (UP) | G | sem-ch07-updated-rebuild-dynamic-above reached it | internal error: every dynamic a node binds reads a hold, a getter over its holds, a structural dynamic or a kept result; the program that reached it (a `rebuild`'s dynamic read above its line) was bound to a field the node did not have, lowered by `GivesResult` |
| node/finish.prism `UpdatedGaps`, the same in a row over a name of its block (SU) | G | the same test's second widget reached it | internal error, by the same argument |
| node/finish.prism `PlaceNumber`, a place read at its block's build after rows built after the node that differ in width | R | sem-ch09-host-dynamic-answer-below-building-if | refused: a hosted child's place with a `$` answer (or a bundle answer, m8b) asking its host its first value below a building line of the child, under a host placed at the build; a plain answer bound to a name asks once the node stands (`BelowGappyLine`; sem-ch09-host-place-asks-below-building-if) |
| node/finish.prism `PlaceNumber`, the same at its row's build in a row numbered late | G | - | internal error: the callers with no receiver are `HostLine` under `PlaceUnknownAtBuild`, the early clause rows under `NumberedLate` (both true for such a row's scope-0 places) and `FirstValueLet` in a hosted scope, whose positions have no `?base` |
| node/switch.prism `RebuildDynamic`, a `rebuild` that hands back a dynamic whose generation gives no dynamic, value or bundle | G | sem-ch04-rebuild-ends-on-host reached it | internal error: the one result left out, a host's line last in the generation, is lowered as the host's bundle (`GenRowOf`); `GivesDynamic` keeps events and unit away |
| node/switch.prism `ArmRowOf`, a late plain result beside a `for` or a `rebuild` in a row numbered after its node | R | sem-ch04-late-value-above-arm-with-for-beside-inner-if | refused: a widget of several clauses above a building `if` whose arm builds a `for` beside an inner building `if` it gives; a widget that calls itself there builds shared rows numbered late (`GivesLate`, `SharedNumbered`; sem-ch04-late-value-above-recursing-widget-result) |
| node/switch.prism `ArmRowOf`, a blueprint value applied in a row numbered after its node | R | none | refused where a gappy generation of the arm is neither early nor numbered by the parent (`UnnumberedSteps`): of the gappy ones only `AppliedBlueprintLine`'s qualifies. Probes a2, a2b-a2f (a `let` blueprint, a blueprint chosen by `if`, an inner gappy `if` beside the arm's split, an impure inner split) lower; a blueprint value out of `Or` (a2d) meets `BuildLineNow`'s internal error first (sem-ch08-blueprint-value-from-or) |
| node/switch.prism `HeldFlatten`, `@Flatten` with no live dynamic where the line reads it (LF) | G | none | internal error: every widget line goes through `WholeArgsAt` before its `@Flatten` is lowered, and it makes a live dynamic for every outer (a hold or dynamic of the node, a dynamic `let` of a row by its expression, an outer in place); a row's outer of its block is the block's through the parent, a global its constant, a shared row's parameter its `#whole#{name}` |
| node/def.prism `SplitBlockInto`, a block split at a line whose result is not late | G | - | internal error: a block is split at a line with a plain result or a bundle with a plain part (`IsPlainResult`, asked by `IsLateExpr` and `?LatePlaceTakerAt`); `BuildingChoicesLine` (a building `match` or `if`, a `DeferredLine`) gives such a result late (`WrLate`) or as parts, `AppliedBlueprintLine` (a blueprint or element value applied) late or as parts, a widget that calls itself plain, and an inlined block or widget gives its last line's; a host's line is never split at. Probes sp1, sp3, sp5, sp6 (a block, an `if`, a widget ending on a host's line, a late place's taker that is a host's line) lower |
| node/def.prism, the build-side markers of a late name in `BindLate`, `MarkLateLets`, `LateFieldPlaces` (`AnswerUnbuilt`), `PreBindWhole` (four sites) | R | `BindLate`: sem-ch09-host-place-field-reads-late-result, sem-ch04-late-value-above-in-ignored-bundle-place-field, sem-ch04-late-value-in-generalized-let; `MarkLateLets`: sem-ch04-late-let-in-generalized-let; `AnswerUnbuilt`: sem-ch04-late-place-answer-in-generalized-let; `PreBindWhole`: sem-ch04-late-bundle-whole-in-generalized-let | refused: three readers reach them. A `let` of a lambda used at two types (`?ManyInstances`) is no late `let` (`LateLetNames`), so `LetLine` builds the closure at the build over the marker; a World's place in a hosted scope, and one whose state parts are all `ignore`, whose constant field reads the late name (`LateFieldPlaces` takes neither). Lowered: the bundle place with named parts (sem-ch04-late-value-above-in-bundle-place-field and its three kin) |
| node/def.prism, the markers of `PreBindWholeParts`, `LateTakerResults`, `LatePlaceParts` (`LatePlainPartOf`, two), `LateHostedOutput`, `LateBundleParts` (six sites) | R | sem-ch04-late-bundle-whole-with-bundle-inside-as-argument, sem-ch04-late-taker-result-in-generalized-let, sem-ch09-host-late-plain-part-in-generalized-let, sem-ch09-host-late-plain-part-pattern-in-generalized-let, sem-ch09-host-late-plain-answer-in-generalized-let, sem-ch04-late-bundle-let-with-bundle-inside-as-argument | refused: the generalized `let` of a lambda reaches each late name of a node; a late bundle with a bundle inside has no `{name}#value` (`LateBundleValue` makes one only of simple parts), so a function that takes it whole reads its parts at the build (`BundleArgumentsAt`, `WholeNamedOf`) |
| node/def.prism `MarkLateBundle`, two markers (a late bundle `let`, another name of one) | G | - | internal error: each stands under `{name}#late` (and `{name}#latebundle`) alone, keys asked with `HasKey` and never read for a value; the walks that copy a block's names into a row's (`WholeOfBlock`, `BuildSpelling`, `RowReads`, `OverEnv`) keep each under its key; a read of the bundle as a value goes to the name (its parts, `LateBundleLet`; `NameValueOf`'s tuple-pattern refusal, gl4) or to `{name}#value` |
| node/def.prism, a late plain result in a bundle (`WrLate`) | R | sem-ch09-host-late-plain-part-taken-above-bundle-result | refused: a widget giving a bundle with a plain part, bound by a tuple pattern, above the place line whose late plain part it takes, in a hosted child under a late host (`BindParts` binds the part by `BundleBn`); without the host the same lines lower (sp8) |
| node/def.prism `DeferredLine`, a line that takes a late result above the line that gives it, with counted places, where the giver numbers no rows late | R | sem-ch04-late-taker-result-taken-by-clause-widget-above | refused for a late name no split line gives (a taker's result, `@Wide w` over `w <- @Twice n`; a `let` calling a local function, d2), a blueprint value the node holds, a late place's answer. Lowered: sem-ch08-late-value-above-blueprint-parameter-result, sem-ch04-late-value-above-bundle-plain-part, the `-from-arm-lines` tests, -in-late-numbered-row, sem-ch09-host-late-value-above-from-arm-lines; a hosted giver under a host below a building `if` (g4b) |
| node/def.prism `DeferredLine`, a taker's plain result no name binds | R | sem-ch04-late-value-above-impure-taker-result-tuple | refused over a split line that is no pure choice (its arm gives what its own widget gives): a tuple pattern (d4) and `ignore <-` (d6). Over a pure choice the taker is no late taker and lowers (sem-ch04-late-value-above-taker-result-tuple, dl2, g7) |
| node/static.prism `WidgetCall`, a widget line whose name has no value entry | G | - | internal error: the checker's `unknown-name`; a `for`'s binder that is a blueprint (`for @each <- [[@One, @Two]]:`) applied as a line of the row is a value of the node (`WidgetLine`'s `held`; sem-ch08-for-over-blueprints) |
| node/static.prism `HostLine`, a host without its child or its supplier | G | - | internal error: the checker's `mismatch` (tests/checker/neg/ignore-binds-a-function, neg/builtin-widget-without-argument) |
| node/static.prism `InlineWidget`, a widget of several clauses that takes no arguments | G | - | internal error: the checker's `collision` (ruling ZD; tests/checker/neg/clause-widget-without-arguments) |
| node/static.prism `InlineWidget`, a generic widget at a call whose types its parameters' do not match | R | ts-ch03-generic-widget-associated-type-parameter | fix written (`?ParamMatch` takes an associated type as deciding nothing), unrun; another program that reached it is the checker's `field-variable` (tests/checker/neg/hosting-generic-implementation) |
| expr.prism, a dynamic passed whole where it has no live dynamic (LF) | R | none; lowered: ts-ch04-whole-lambda-reads-hold-in-lifted-call (`WholeArgsAt` takes a time-taking lambda's reads, and the lambda's sigiled binder is the `Dynamic` it is given; ok on prismc-fd1) | refused where no line made a live dynamic for the name; no program found (the probes g1, g2 below) |
| expr.prism `LiftedValue`, a lift whose noted operands are not its own | G | - | internal error: the checker lifts only at applications, operators, fields, indexes and interpolations, each a direct operand |
| expr.prism `PlainValueByKind`, `{b}` outside a `for` | G | - | internal error: the checker's `unknown-name` (tests/checker/neg/bound-word-outside-for) |
| expr.prism `PlainValueByKind`, an expression of no value kind | G | - | internal error: `mismatch` for a `for` or a `rebuild` as a plain value (ruling BP; tests/checker/neg/for-in-let), `unsupported` for a suffixed operator, a monadic block and a lens value; a topic, `..` and an absent value are read by an `update` block, a pattern and an element's fields. Not argued: a widget block as a value |
| expr.prism `PartsBlueprint`, two sites (`PartInternalBug`) | G | - | internal error: `DynamicOutputs` makes a getter for every dynamic answer |
| expr.prism `NameValueOf`, an event read as a value | G | - | internal error: every name of an event node is bound beside its port (`PreBindName`, `BindEventParam`; a pattern's part, a host's child's result and a `let` over a bundle bind a prebound name), a row's through the parent (`RowEnvNames`, `EventsOf`, `WholeOfBlock`), a shared row's event parameter is its caller's (`LiveParams`). Probes a3, ev1-ev5 lower |
| expr.prism `NameValueOf`, a tuple pattern's binding as a value | R | none | refused where a bundle bound to one name is read as one value in no function's argument, no generation's result and no `let` of it (a tuple or an array that holds it); passed to a function it is one value (`BundleArgumentsAt`, ts-ch02-named-bundle-as-argument) |
| expr.prism `NameValueOf`, a widget defined in a block read as a blueprint value outside a line's value places | R | none | refused; a lambda's body that is the widget is a value place (`BlueprintValuesAt`; sem-ch08-block-widget-given-by-lambda), one whose body chooses it (`\|c| if c: @Inner else: @Other`) is not |
| expr.prism `WidgetNamed` and `GlobalCall` (`DeclaredCall`), a widget, named or applied, as a value where `ValuedWidgets` does not look (two calls, through `OpenWidgetValueWords`) | R | sem-ch08-widget-value-as-method-argument (`WidgetNamed`), sem-ch08-applied-widget-value-as-method-argument (`GlobalCall`) | a method call's argument (`loud.?Pick(@One)`); fix written, unrun: `ValueWidgetsIn` reads a method call as its function's call with the receiver first. Lowered: sem-ch08-widget-value-in-map-literal; a widget chosen by `if` in an argument (w3) and one in a record's `update` block (w5) lower. A widget defined as one expression that is another widget (`fn @main = @page`) is an alias (sem-ch08-widget-is-another-widget) |
| expr.prism `WidgetBlueprintValueFrom`, a reading with no rows | G | - | internal error: `WidgetNamed` and `GlobalCall` ask `Lx.valued` first, `?OpenWidgetBlueprintValue` asks it for every instantiation (`rowed`) |
| expr.prism `OpenWidgetValueWords`: `EndlessByName`, `EndlessThroughImpl` | G | - | internal errors: the checker's `grows-without-end` |
| expr.prism `OpenWidgetValueWords`: `UndecidedParameter` | G | - | internal error by its text: `?UseTargets` gives no answer only where a use's types name a type parameter once the user's instantiation's types are put in, and those are closed over every type parameter of the user's signature (`?ClosedTypes`, a method's own beside its impl's); a type left open at a use (`Frame [[]]`, ud3) is a variable, taken as it is; a local generic function's own variable meets `OutQuantified` first (ud1, ud4) |
| expr.prism `Recolored`, a function value held by a type that is none of those it maps through | R | none; lowered: ts-ch05-color-function-through-associated-type (an associated type in the signature's result at a head an impl knows, `KnownAssoc`; ok on prismc-s6) | with `?ParamMatch` matching every other pair has one shape; left: a call whose type does not give its callee's type parameters (`?calleeTypes` nil), not met |
| expr.prism `FunctionRecolored`, a function whose color no use of its `let` decides | U | none | refused; reached only where one side is `OutQuantified` (`OutTyOf` of a function type is `OutFn`, `OutJobFn` or that), the `OutQuantified` gap: a call making its callee's color bounded inside a generalized `let` or local function whose color no use decides. fr1-fr5 (a `let` of `Held (..)`, used or not, a generalized lambda over it, a widget's local function calling it) lower: such colors default bounded |
| patterns.prism, a pattern inside an element's `$` field | G | - | internal error: the checker's `live-field-pattern` (ruling LP; tests/checker/neg/live-field-pattern) |

Totals: 25 `Refusal` / `RefusalDo` calls stand in prismc/lower. 21 are R with a red
test (`AnyValue`, the named bundle, the block widget, `WrLate`, the generic widget, the
ten late-name markers of `BindLate`, `MarkLateLets`, `AnswerUnbuilt`, `PreBindWhole`,
`PreBindWholeParts`, `LateTakerResults`, `LatePlainPartOf` two, `LateHostedOutput` and
`LateBundleParts`, `ArmRowOf`'s `for` site, `PlaceNumber`'s block site and
`DeferredLine`'s two, the two widget-value calls); 3 are R with no test (`ArmRowOf`'s
blueprint-value site, `Recolored`, whose test is ok on prismc-s6, and LF, whose test is ok
on prismc-fd1); 1 is U (`FunctionRecolored`). Twenty-two guards are internal errors
and no longer refusals. One internal error is reached by a checked program and is an R
with a red test: `BuildLineNow`'s "a building line that is no element, blueprint or
builder" (sem-ch08-blueprint-value-from-or).

Refusals outside `Refusal` / `RefusalDo`, which the table above does not hold:

| site | class | test | state |
|---|---|---|---|
| emit/rust/items.prism `DeepPodOf`: a level field's walk at a type that holds no time | G | - | internal error ("a prismc bug: a level field's walk met .."): arrays, maps, options, `~T`s, tuples, `Rc`s, dynamics and generic program types are walked (sem-ch09-tag-field-array-of-dynamics, -named-type-of-dynamic, -fault-of-dynamic) |
| emit/cpp/mod.prism `PrinterRefusal`: a constructor, record or pattern of no type the C++ printer's tables know | G | - | internal error ("a prismc bug: the C++ printer met ..") |
| lower/types.prism, `OutQuantified` (two sites): a quantified type variable no instantiation gives a type | R | sem-ch08-widget-value-generic-through-local-function | refused: a widget's local generic function (`fn ?Made y, label = ?Framed ((y, y)), label`) calling a generic function that reads a generic widget as a value; when the local function reads the widget itself it lowers (sem-ch08-widget-value-generic-in-generalized-let). `Len [[]]`, `Size {}`, an unused `let empty = [[]]`, an unused generalized `let` of a lambda and one handed to a function of its callee's color (oq1-oq3) are lowered |
| lower/expr.prism `ClosureParams`, a typed lambda parameter with no type | G | - | internal error: a lambda's type has as many parameters as it has (the checker's arity, check/expr/calls.prism), so its target type gives one per parameter: `OutFn` (`PlainLambda`), a `Start` (`JobLambda`), or `OutQuantified`, now given to each parameter and refused as the type is |

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
- Under target/fx/s4 of the fourth sweep's worktree, compiled by keep-B: a taker above a
  hosted giver under a host below a building `if` (g4b), `ignore <- @Wide n` above the
  giver (g7), a grouped taker `w <- (@Wide n)` and one over a `let` (m4, m5), a `for`
  over a late place's answer (m6), a hold starting from a function of a late bundle
  bound whole (b1), a hosted place's plain answer bound by `(a)`, by a plain pair's
  tuple pattern, read by another place's field, and a bundle place reading a late plain
  part, under a host below a building `if` (e1b, e3, e4, e5, e6, v1321, v1376), a
  bundle place reading a taker's result (v1248): each lowered.
- Under target/fx/s5 of the fifth sweep's worktree, compiled by prismc-s4: a split line
  that is a widget block, a building `if`, or a widget of one clause, each ending on a
  host's line whose child gives a plain result, read above (sp1, sp3, sp5); a late
  place's taker above its place that is a host's line (sp6); a bundle giver above a
  building `if` it takes (sp8): each lowered. A bundle giver over a late place's part
  (sp7) reached `WrLate`; a generic widget with an associated type parameter (gw5) the
  generic widget's refusal; a widget's local generic function calling a generic
  function that reads a generic widget as a value (ud1) `OutQuantified`. Lowered: a
  method with a type parameter of its own calling that generic function (ud2); an event
  part of a bundle taken apart by a `let` from a bundle bound whole, and from another
  name of it, read by `!MergeAll` (ev1, ev2); an unused generalized `let` of a lambda,
  one handed to `Applied: u fn (u fn Nat -> Nat), Nat -> Nat`, one kept in an array by a
  function of that color (oq1, oq2, oq3); a taker above its giver bound by `(w)` (dl1)
  and by a record pattern, `Bounds(low=a, high=b) <- @Wide n` (dl2);
  a hosted place's plain answer in a widget inlined in the child, in an arm of a building
  `if` of the child, and under a host inside the child, each under a host below a
  building `if` (eo1, eo2, eo3).
- Under target/fx/s6 of the sixth sweep's worktree, compiled by prismc-s4: an event
  read as a value through a `let` alias, a bundle's event part taken apart above a `for`
  and inside it, a late event read above its building `if`, a hosted child's event
  result (ev3, ev4, ev5); a generic widget read as a value in a generic function used at
  `[[]]` and through a function's local function (ud3 lowers, ud4 meets `OutQuantified`);
  a call's function of its callee's color kept by a `let`, used or not, under a
  generalized lambda and in a widget's local function (fr1-fr5); a hosted place's
  answer bound by a record pattern, and one whose field reads a late result, under a
  host below a building `if` (eo4, eo5); a lambda with a `$` parameter given to `Each`
  over an array of dynamics (lf1's second widget): each lowered.
- Compiled by prismc-s6 (the sixth sweep's bootstrap of 6805634c): sweep-4's marker
  probes v839, v1207, v1248, v1292, v1321, v1376, v1446, v1450, v1486: each lowered.
- Under target/fx/s7 of the seventh sweep's worktree, compiled by prismc-fd1: a late
  taker's result read by a generalized `let` over a pure choice (gl2), a late bundle
  `let` read as one value by one (gl4, `NameValueOf`'s tuple-pattern refusal first), the
  child's own result that is a plain answer under a late host (m4), a `let` blueprint
  applied beside an arm's late result (a2), a blueprint chosen by `if` there (a2b, a2c),
  a gappy inner `if` and an impure inner split in a late-numbered arm (a2e, a2f), a
  widget as a value chosen by `if` in an argument (w3), a function of its callee's color
  through a type alias (rc1), a widget value in a record's `update` block (w5): each
  lowered.

## Designs for the red R tests with no fix

- A widget's local generic function calling a generic function that reads a generic
  widget as a value (sem-ch08-widget-value-generic-through-local-function): the uses
  in the local function are read at its quantified type (`?Made`'s `y`), which no
  instantiation of the enclosing widget gives. Read the local function's uses at each of
  its instances (the instances `WithInstances` already lists for a local function that
  reads the widget itself), so `?Framed` has the instances `((Nat, Nat))` and
  `((Text, Text))`, and lower the local function once per instance as a generalized
  `let` is.
- A bundle giver above a late place's plain part it takes, in a hosted child under a late
  host (sem-ch09-host-late-plain-part-taken-above-bundle-result, the `WrLate` site): the
  line `((w, !b)) <- @Pair port` is built after the node (`DeferredLine`, its late names
  all a late place's parts), so its bundle's plain part comes back late (`WrParts` with a
  `WrLate`), and `BindParts` binds the name by `BundleBn`, the marker. A plain name there
  is a late taker's result (`LateTakerResults`, `{name}#taker`, its getter defined where
  the line is lowered). Mark the plain names of a bundle's tuple pattern the same way when
  the line takes only late place parts (`ReadsLateOfPlaces`): each a getter over its
  part's read, defined in `WidgetStmt`'s `WrParts` arm by `LateGetters` with the part's
  read, the part then handed to `BindParts` as `WrUnit`. Not for a taker above a split
  line (sp8): there the part comes back as a field of the node (`WrPlain`), bound as one.
- A `let` of a lambda used at two types that reads a late name (the seven
  `-in-generalized-let` tests: `BindLate`'s, `MarkLateLets`', `AnswerUnbuilt`'s,
  `PreBindWhole`'s, `LateTakerResults`', `LatePlainPartOf`'s two and `LateHostedOutput`'s
  markers): `LateLetNames` leaves out a `let` with several instances
  (`?ManyInstances`), so `LetLine` builds the closure at the build, over the marker. A
  lambda's value reads nothing until it is called, so make such a `let` late with the
  others: a getter of the node per instance (`late_{name}_{instance}`, the instances
  `WithInstances` lists), each building the closure over the node's getters, bound under
  `{name}#late` as a late `let` is (`LateLetGetters`).
- A World's place whose constant field reads a late name, in a hosted scope
  (sem-ch09-host-place-field-reads-late-result): `LateFieldPlaces` takes scope 0 alone,
  where the World answers. In a hosted scope the host answers: mark the place's plain
  answer as `LateHostedOutput` marks one under a host placed late (a late hold,
  `LatePlainBn`), and ask its first value in the scope's `lateFirsts` slot of
  `ScopeSteps`, which runs after the scope's deferred generations, the giver's row among
  them.
- A World's bundle place whose state parts are all `ignore`
  (sem-ch04-late-value-above-in-ignored-bundle-place-field): `IsAskedBundle` finds the
  place by a named part's `{name}#asked`. Give `LateFieldPlaces` a marker per place
  (`{place}#asked`, by the line's span) so the place asks its whole first value where the
  late result is built with no part bound.
- A late bundle with a bundle inside, passed whole to a function
  (sem-ch04-late-bundle-whole-with-bundle-inside-as-argument, -let-with-bundle-inside-as-
  argument): `LateBundleValue` makes `{name}#value` only of simple parts, so
  `BundleArgumentsAt` reads the parts at the build (`WholeNamedOf`). Let it build a
  bundle inside as the tuple of its own parts' values, recursively, so every late bundle
  has its `{name}#value`.
- A `for` or a `rebuild` beside a late result in an arm numbered after its node
  (sem-ch04-late-value-above-arm-with-for-beside-inner-if, `ArmRowOf`): the row builds
  its generations at its build (`Wx.lateGens`), but a builder's rows are numbered as they
  are built (`UnnumberedSteps`). Number them in the row's `number_places` step as the
  late-numbered generations are (`NumberedByParent`): a builder's width is known once
  built, so phase 2 can place it.
- A hosted place's `$` or bundle answer below a building line of its child, under a host
  placed at the build (sem-ch09-host-dynamic-answer-below-building-if, `PlaceNumber`):
  the place asks at the build, before its number is known. Mark it as `LateFirstPlace`
  marks a `$` place under a late host (`PreBind`'s `latePlace`, extended by
  `BelowGappyLine`), and ask in the same `ScopeSteps` slot as the plain answer's.
- A widget of several clauses above, taking a late taker's result
  (sem-ch04-late-taker-result-taken-by-clause-widget-above, `DeferredLine`): `FromSplit`
  follows `let`s to the split names, not takers. Let it follow a taker's result to the
  split names its line reads: the taker is built in phase 1 after the split line's rows
  (`LateTaking`), so the counted line above can ask the split line's rows numbered late
  as for a direct read.
- A taker's plain result bound by a tuple pattern or `ignore`, over a split line that is
  no pure choice (sem-ch04-late-value-above-impure-taker-result-tuple, `DeferredLine`):
  `LateTakerResults` marks a name alone. Mark a pattern's names too, each a getter over
  the row's kept result matched by the pattern (as `LateName.within` does for a late
  plain part), and let `ignore <-` keep no result (`WrUnit`).
- A blueprint value taken out of an optional by `Or` and applied
  (sem-ch08-blueprint-value-from-or, `BuildLineNow`'s internal error): the line's type is
  none of `TyElem`, `TyAt` or a builder's where `BuildLineNow` matches it, though the
  checker gives `Or` the type `@unit` there. Find the type `TypeAt` has for the line
  (likely `Or`'s type variable, bound at the call but not resolved by `Peel`) and
  resolve it before the match; the line is then `AppliedBlueprintLine`'s.
