# The promptness checker

*Written by Claude on the claude-self-hosting branch, 15 November 2026. Subject to change;
implementers use discretion.*

The checker is `prismc/check/passes/cycles.prism`. It enforces Semantics ch. 7, "There
are no cycles": in one tick, no event may cause itself. This page says what "prompt"
means, which graphs the program has, how the check reads each construct, and how
promptness is written in signatures with tick lifetimes.

## Goals

- **Refuse every real cycle.** A cycle the checker takes runs as `Runtime.TwoWrites` or
  loops inside a tick. A wrong cut is a soundness hole and gets a failing test.
- **Never refuse a legal program for its shape.** A loop through a hold, a host or the
  ownership tree is delayed and legal. The tally (`tests/documents/tally`) seats a
  document from its own results and must check with nothing but correct annotations.
- **Checking is local** (ruling PT). A call is read by its callee's signature, never its
  body. A body is checked against its own signature.
- **No special cases.** Hosts, documents, roles and seats get no rule of their own. Every
  rule here talks about events, dynamics, blueprints and functions.

## Prompt, in one sentence

- **`x` is prompt in `y` when a firing of `y` can make `x` fire in the same tick.**
- A dynamic is events plus behaviors: its value is read, its update event fires. Only
  update events take part in cycles. Reading a value is never a firing.
- A plain value is never prompt in anything. A function value is prompt in what it
  captures, because calling it reads those now.

## The graphs, and why most of the program is acyclic for free

The program has several graphs. Only one of them can close a cycle inside a tick.

- **Widget ownership is a tree.** A widget builds its children. Changing a dynamic of
  blueprints (`$seats <- ...` of `$(Text ==> $@(@unit))`) builds and ends guests at the
  end of the tick (Semantics ch. 5, construction). Ownership is never a prompt edge.
- **Element talk goes through hosts, and is never cyclic.** A guest's element goes up to
  its host. The host's computed answer comes back down a tick later (ruling HO, Language
  ch. 6, "Tick by tick"). A forwarded element reaches the World in the same tick, and
  the World's answer comes as the next tick opens. The tags form a DAG up the tree.
- **Events and dynamics form the only graph that can cycle.** An event computed from
  another, a dynamic lifted over another, a function given an event and giving one back.
- So a blueprint is a problem only when it outputs an event, or a dynamic, that is
  prompt in another event. A blueprint whose output holds no event (`@unit`, `@Text`)
  can never take part in a cycle, however it is built, held or hosted.

## Tick lifetimes

Promptness is written the way Rust writes borrows. An event that can be caused by
another event in the same tick borrows that event's tick.

- **A tick lifetime names the events a timed value may be caused by in its tick.**
  `!'a Nat` is an event that may fire in a tick because something with lifetime `'a`
  fired. An event with no lifetime is caused by nothing in the same tick: a World's
  input, a host's computed answer, a hold's read value, a built guest's first value.
- **A signature relates lifetimes.** `@Echo: fn !'a Nat -> @!'a Nat` says the output
  borrows the input's tick. `@Fresh: fn !'a Nat -> @!Nat` says it does not.
- **`prompt` is the elided form.** Ruling PT's `prompt` on a parameter is the lifetime
  `'p` given to that parameter and to every timed part of the result: an unannotated
  timed result borrows every `prompt` parameter. Every signature written so far keeps
  its meaning.
- **Spelled after Rust.** `!'a Nat`, `$'a Nat`, `@!'a Nat`: the lifetime sits right after
  the sigil, so it reads as a borrow and composes inside types (`$(k ==> !'a Nat)`).
  Alternatives: `!Nat by a`, `prompt(a)` on parameters and result parts.
- **Lifetimes ride inside types.** A type variable carries the lifetimes of the type it
  is instantiated at. `@Seated: fn $(k ==> $@(@a emit r)) -> @$(k ==> a)` needs no mark:
  whatever `a` borrows, the result's entries borrow, and the map's own update (ownership,
  built at the tick's end) borrows nothing.
- **Records and choices take lifetime parameters, written, where a field borrows.**
  `type Results 't = record: $teller: $'t (..), !tellersLeft: ![Text]` says the tellers
  map may move in `'t`'s tick and the departures move in no one's. Written like Rust's:
  the record's lifetimes are part of its promise, so a library says them rather than
  letting a field's body decide them.
- **Holds cut values, not updates.** `$x <- init then step` gives a value read as the
  tick opened and an update event that borrows the step's events (R3).
- **Hosts are primitives.** `@Host`'s child result borrows what the child's blueprint
  output borrows. Its element tree and its computed answers borrow nothing (HO).

### The tally, checked

```prism
@HostDocument: fn Implementation d lower, Seats d rest -> @(Results d) emit lower | rest
@Table: fn TallyResults 't -> @(TallySeats 't o Ui.*) emit Ui.* where o: ^TallyOrder

@table:
    seats <- @Table<<OneEach>> results                 # seats borrow results' 't
    results <- @HostDocument (@InMemory .. empty), seats
```

- The seats are `$@(@unit emit ..)`: their outputs borrow nothing, and changing the map is
  ownership. So `@HostDocument`'s result borrows nothing from `seats`.
- `!tellersLeft` comes off the guests' element trees through `@HostRole`'s host: it
  borrows nothing.
- `@Table`'s seats borrow `results.!tellersLeft`'s tick, which is empty. No cycle, no
  `@Defer`, and every annotation says only what the function does.

### A cycle it still refuses

```prism
@Echo: fn !'a Nat -> @!'a Nat emit Ui.*
@room:
    let $@!echo = $ (@Echo !said)      # the blueprint captures !said
    $!back <- rebuild $@!echo:
        _
    !said <- @Switch $!back
```

- `@Echo`'s output borrows `!said`, and the blueprint captured `!said`, so `$!back`
  borrows `!said`. `!said` borrows `$!back`. A cycle
  (`tests/checker/neg/prompt-cycle-through-hosted-blueprint-output`).
- A map of blueprints read as data still borrows: `let $seats = Grown (Keys $seats)` reads
  the map's keys, which is a data edge, not ownership
  (`tests/checker/neg/prompt-cycle-through-blueprint-map-keys`).

## How the check reads a block today

These are the walks in `cycles.prism`. The tick lifetimes above are what they compute.

- **`BlockPromptCycles`**: one graph per widget block. Each binding gets an edge to each
  of the block's names its right side reads promptly (`PromptEdges`, `PromptReads`). A
  cycle is `prompt-cycle`, named in binding order.
- **Calls** (`PromptCallKids`, `ByFlags`): a widget function's argument is read when its
  parameter is marked `prompt` (ruling PT). A plain function lifts, so every argument is
  read. A lambda given at an unmarked parameter is not read (`CapturingLambdas`).
- **Captures** (`CaptureReads`, `MarkedArgKids`): a function value inside a record, a list
  or a constructor brings along what it reads.
- **Rows** (`RowKids`, `RowReach`): a `for`'s or a `rebuild`'s result is prompt in what
  its rows' results read. Building a row is ownership, never an edge.
- **Holds**: a hold's name has no edge. Its update node does, to its step
  (`UpdateEdges`, the second walk over update events).
- **Places** (`IsPlaceOf`): a place's answer is the next tick's input (HO). A
  `!a <- Count.Ask(!by=!a)` line has no edge.
- **Bodies against signatures** (`BodyMarked`, `?BlockFlows`): a function's body is
  summarized per parameter (`ParameterFlow`). A parameter the result reads in the same
  tick must be marked `prompt` (`prompt-unmarked`).
- **Time in types** (`CarriesTime`): a parameter whose type holds no event and no dynamic
  needs no mark.

## Negative space

- **No reading of callee bodies at call sites.** Buys local checking, and a library
  whose signatures are its promises.
- **No special type for "came off an element tree".** Buys hosts that are primitives:
  their lifetimes come from their signatures like any function's.
- **No per-document or per-role rule.** Buys a checker that knows events, dynamics,
  blueprints and functions, and nothing else.
- **No runtime check.** Buys programs that cannot loop inside a tick at all.
