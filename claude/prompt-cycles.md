# The prompt-cycle check: rows and function parameters

*Written by Claude (the prismc run), 11 November 2026. Subject to change; implementers
use discretion.*

The check is `prismc/check/passes/cycles.prism` (Semantics ch. 7, "There are no cycles").
This page says how it reads a `for`'s row and a widget function's function parameter.

## What the check is for

- A cycle of prompt edges among a block's bindings is `prompt-cycle`, an error with a line.
- A legal program is never refused: a loop that crosses a hold or a host's computed
  answer is a delayed loop (ch. 7; ch. 9, "A computed answer is the next tick's input").
- A wrong cut accepts a real cycle, which is a `Runtime.TwoWrites` at run time, so each cut
  errs toward reading more.

## A row is read by its result

- A `for`'s or a `rebuild`'s value is prompt in its rows' results (the `for`'s
  consolidation). The source is never read: building a row is never an edge (R7).
- `RowKids`: a row whose last line is an expression gives the `for` that line and the
  row's lines the result reaches (`RowReach`). A row with no result line gives every line.
- The example that needs it: a host's row holds `let $told = Read $state, seat, Whole`,
  read only by the supplier's lambda (`Count.Query(..): @ $told`), and the row's result is
  the requests the host's document asks. `$state` is not prompt in those requests.
  Reading every line of the row made it so, and the block that feeds `$state` from the
  requests was refused.
- `RowReach` errs toward reaching:
  - a nested `for`'s or `rebuild`'s source is read (a nested `for` over `$doc` fires what
    `$doc`'s elements fire, so the line that binds `$doc` is on the path);
  - every line of a nested row is read, holds included;
  - a hold reaches what its step reads: a hold's update fires in the tick of its step (R3),
    so `@ $seen` with `$seen <- 0 then $seen + 1 on !step` reaches `!step`
    (`tests/checker/neg/sem-ch07-row-hold-cycle`).
- `RowReach` stops where the prompt walk stops: a lambda given to a prelude widget function
  (a supplier), a delayed widget's arguments, a nested widget block.
- A row's line that binds nothing and is not the result (a place) is not read: a place is
  a leaf, as in a widget function's summary, which reaches from the result alone.
- The alternative, every line of the row as read (what a block's own lines are), refuses
  the hosting library's role hosts.

## A function parameter is read by what each use gives

- A widget function's summary says which arguments its result reads promptly. A call of
  a function parameter inside a body has no summary of its own: nothing is known of it,
  and it is read as prompt in everything.
- `GivenAt`: where a use gives a program's widget function by name for the parameter
  (`@InMemory 0, @Seats`), a call of the parameter in the body goes by that function's
  summary.
  - In a summary: `?PromptSummaryAt` takes the call's arguments.
  - In the function's own block check: `ParameterUses` finds every application of the
    function in the program's definitions and tests, and the block is checked once for
    each. The first cycle found is reported at the body's line, as before.
- A use that gives a lambda, a local, a parameter of its own, or passes the function on
  as a value gives nothing: the worst is assumed for it. So is a function with no use.
- The example that needs it: `fn @InMemory initial, HostSeats` binds
  `((results, !asked)) <- HostSeats $state, !responses` and computes `!responses` from
  `!asked`. With `@Seats`, whose requests are not prompt in `!responses`, there is no
  cycle; with a function that gives `!responses` straight back there is one
  (`tests/checker/neg/sem-ch07-function-parameter-cycle`).
- The alternative, the worst for every parameter, refuses every document host that takes
  its seats as a function. Another alternative, a declared summary on the parameter's
  type, is a change to the language.
- The block check of a widget function now reads the program's other definitions, so a
  library's body is judged by the uses in the program it is compiled with.

## A loop through `!Updated` of a hold

- A hold's value is delayed; its update event is not: it fires in the tick of the hold's
  step (R3). So `$seen <- 0 then $seen + 1 on !step`, `let !step = (1 on !press) <|
  !Updated $total`, `let $total = $seen * 2` is a prompt cycle: a press steps `$seen`,
  which updates `$total`, which fires `!step`, which steps `$seen` again in the same tick
  (`tests/checker/neg/sem-ch07-block-hold-updated-cycle`; in a `for`'s row the same loop
  is `sem-ch07-row-hold-cycle`).
- The block's names keep the edges they had: a hold's name has none, and an event `let`
  samples the dynamics it reads. That walk runs first.
- `UpdateEdges` is a second walk, over update events. Each dynamic has an update node
  beside its name. `!Updated X` in a right side is an edge to the update of each dynamic
  `X` reads; a hold's update reads the events of its step and the updates under an
  `!Updated` there; the update of a `let`'s dynamic reads the updates of the dynamics it
  is lifted over (the change law). A cycle there is reported with the block's names in
  the order the block binds them.

## A widget function's flows

- The second walk crosses a line that applies a widget function by that function's
  flows (`ParameterFlow`, one for each parameter, `?FlowSummaryAt`). A flow says four
  things of the result and one argument:
  - the result is prompt in the argument as it is (`fn @Pass !passed: @ !passed`);
  - the result is prompt in the argument's update event (`@ (!Updated $given)`);
  - the result's update event is prompt in the argument (`@ $seen` beside
    `$seen <- 0 then $seen + 1 on !given`: a hold's update fires in the tick of its step);
  - the result's update event is prompt in the argument's update event (`@ ($given * 2)`).
- The flows are read off the body (`?BlockFlows`): the body's names and their update
  nodes, a summary's edges (a hold none, any other binding to all it reads) and each
  line's edges over update events (`UpdateLineEdges`, the same function the block's
  second walk uses). A name's sigil stands for its type there.
- Through a widget's line (`tests/checker/neg/sem-ch07-hold-updated-cycle-through-widget-line`):

  ```prism
  let !fed = (1 on !press) <| !Updated $total
  !step <- @Pass !fed                      # `fn @Pass !passed: @ !passed`
  $seen <- 0 then $seen + 1 on !step
  let $total = $seen * 2
  ```

  `@Pass`'s flow gives the line the edge `!step` → `!fed`, which closes
  `$seen` → `$total` → `!fed` → `!step` → `$seen`.
- Through a hold inside the function
  (`tests/checker/neg/sem-ch07-hold-updated-cycle-through-widget-summary`):

  ```prism
  fn @Echo !step:
      $seen <- 0 then $seen + 1 on !step
      @ (!Updated $seen)
  ...
  !back <- @Echo !step
  let !step = (1 on !press) <| !back
  ```

  `@Echo`'s result reads the update of `$seen`, which reads `!step`: the line gets
  `!back` → `!step`.
- Which lines get edges, and which get none:
  - a line whose right side is one call of a program's widget function by name (one
    clause, a block that ends in its result, not on the path of functions being read):
    by its flows;
  - a call of a function parameter: by the flows of the function each use gives by name
    (`FlowsGivenAt`, beside `GivenAt`); the block is checked once for each use;
  - none for a parameter a use gives a lambda, a local or a parameter of its own, for a
    function the program never uses, for a prelude widget function, for a call with a
    named argument, and for a right side that is not one call;
  - none for a `for`'s, a `rebuild`'s or a place's line: a row's result is read by the
    first walk (`RowReach`, which reaches a hold's step), and a place's answer comes as a
    tick opens (a World's) or a tick late (a host's computed answer).
- The first walk keeps the summaries it had (`?PromptSummaryAt`): a hold is a cut there.
  A loop that needs a flow is found by the second walk.
- The example that needs the cut for unknown functions: the hosting library's
  `@InMemoryOrdered` binds `!arrived <- HostSeats $state, !responses` beside
  `let !responses = !Updated $responded`, where `$responded` steps on what `!arrived`
  gives. `@InMemory` passes its own `HostSeats` on, which gives nothing; the programs'
  uses give `@Table` and its like by name, whose requests come from the hosted
  documents' places and read neither `!responses` nor `$state`'s update. With the worst
  assumed for an unknown `HostSeats`, the library would be refused.
- A loop the check takes: the same loop through a parameter given a lambda
  (`tests/checker/neg/sem-ch07-hold-updated-cycle-through-parameter-given-a-lambda`,
  red). It aborts at the build when run. A summary declared on a parameter's type
  (`HostSeats: fn $State, !Response -> @!Arrived delayed in 2`, spelling aside) would let
  the check assume the worst for an undeclared parameter and close it: a change to the
  language and to the hosting library's signatures.
- The alternative, the worst for every line and every call, refuses `@InMemoryOrdered`.

## A place whose position reads its own answer

- `BlockPlaceCycles` runs beside the prompt-cycle check, on the same block, and reports
  `place-cycle` at the place's line.
- What it is for: places are counted as they are built, in line order, and a place's
  supplier is handed the place's path. A line above a place that builds a different
  number of places by a plain part of that place's own answer makes the path read the
  answer the supplier gives at that path
  (`tests/checker/neg/sem-ch09-host-place-position-reads-own-answer`):

  ```prism
  w <- @Wide port                          # `fn @Wide 7:` two labels; `fn @Wide k:` one
  ((port, !rx, $peers)) <- Pad.Socket(address="chat:7000")
  ```

- A place line: a bind of an element tag's construction; its plain names are the ones
  read (`port`; `!rx` and `$peers` are wired, not read at the build).
- The lines above it that count, each a building choice that picks its arm by the plain
  name, read through the block's plain `let`s:
  - a call of a program's widget function of several clauses, where the name is read by
    an argument at a parameter some clause matches refutably (a literal, `nil`, `? x`, an
    array pattern, a constructor of a type with several);
  - a building `match` whose scrutinee reads it, a building `if` whose condition does.
- The error needs two arms whose widths are known and differ (`Width`):
  - a `let`, a hold, a declaration and the result line `@ value` take no place; an
    element tag's construction takes one; a call of a program's widget function takes its
    clauses' one known width;
  - a `for`, a `rebuild`, a nested building choice, a blueprint that arrives as a value or
    a parameter, a prelude widget function and a function that applies itself have no
    known width, and an arm with such a line never gives the error.
- With clauses that each build one place the program is legal
  (`tests/conformance/sem-ch09-host-late-plain-part-taken-above`).
- The alternative, refusing every building choice above a place by the place's plain
  part, refuses that legal program.
- The code `place-cycle` and its wording are Claude's: the books name no such error.
