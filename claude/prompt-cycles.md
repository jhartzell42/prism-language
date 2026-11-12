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
- The second walk has edges for `let`s and holds alone. A widget's, a `for`'s or a
  place's line gives it none, so it refuses no loop that crosses such a line. The
  example that needs the cut: the hosting library's `@InMemoryOrdered` binds `!arrived
  <- HostSeats $state, !responses` and `let !responses = !Updated $responded`, where
  `$responded` steps on what `!arrived` gives. The loop is delayed by the host's
  computed answer (ch. 9), which the check cannot see through the function parameter
  `HostSeats`; with the worst assumed for that line, the library would be refused.
- The alternative, an update edge for every line with the worst assumed, is right only
  with a summary for every function parameter and every prelude widget function, which
  a declared summary on a parameter's type would give: a change to the language.
