# A cycle through a host's own `$doc` is refused as a prompt cycle

*Written by Claude on the claude-self-hosting branch, 4 November 2026; subject to
change.*

## The scenario

A host shows its child a count computed from the host's own view of the child's
document:

```prism
emit Ui.*

@Shown: fn @a, $Nat -> @a
fn @Shown @c, $n:
    Ui.Label(text=$"{$n} places")                                  # 0
    @c

@child: @unit
@child:
    Ui.Label(text=$"inner")                                       # 0

@host: @unit
@host:
    ((x, $doc)) <- @HostDocument (@Shown @child, $size), \|p, e| @Element e   # 0/[p]
    let $size = Size $doc
    @ x
```

prismc refuses it:

```
cyc.prism:15: error[prompt-cycle]: a prompt cycle: `$doc`, `$size`
```

Two neighbours of the same program pass the checker:

- **Through a hold:** `$size <- 0 then Size (!Updated $doc)` in place of the `let`.
- **Through a `for`:** each row of a `for` reads a count of the `for`'s own gathered
  dynamic, which is the same dependency on a structural dynamic:

```prism
@host: @unit
@host:
    let $names = $ {"a": unit, "b": unit}
    $rows <- for k, ignore <- $names:                             # 0/[k]
        Ui.Label(text=$"{$count} rows")                           # 0/[k]/0
        @ $ k
    let $count = Size $rows
    Ui.Label(text=$"{$count}")                                    # 1
```

It first showed up in a Prism HTTP host whose listener reads the responses off the
host's `$doc`. There the cycle goes `$doc` to `$answers` to the child.

## Is it a bug?

Yes. The program has no prompt cycle under the books' rules:

- **`$doc` is structural, so it is lagged.** Semantics round 3 ST: a host document's
  dynamic "fix[es] its value after n as the last act of construction and fire[s] its
  update event at n+1 with that value". A read of `$doc` in tick m sees the snapshot
  taken at the end of tick m-1's construction. That is a delayed edge, like a sample or
  a hold's own value, and a cycle through a delayed edge is not prompt.
- **The checker already agrees for a `for`.** A `for`'s gathered dynamic is structural
  by the same ruling, and the `for` form passes. A host document is a `for` over the
  child's document (Semantics ch. 9, "A host document is a `for`"), so the two must be
  judged the same.

## Why the checker refuses it

Hosting is not special-cased in the prompt-cycle check. The host forms fall through
to the generic rule for prelude widget functions:

- **A widget binding's names all get the same edges.** `PromptEdges`
  (`prismc/check-passes.prism`, the `SBind` case) gives every name the pattern binds
  (`PatternNames`: both `x` and `$doc` of `((x, $doc))`) an edge to every block name its
  right side reads promptly.
- **A prelude widget function is prompt in its arguments.** `PromptCallKids` reads a
  prelude `@` function's arguments, its lambdas excepted, as prompt. Only `@Defer`,
  `@Later`, `@Run`, `@Machine` and `@started` are known to be delayed
  (`IsDelayedWidget`). So `@HostDocument (@Shown @child, $size), ...` reads `$size`
  promptly.
- **A `for` is excluded from edges altogether.** `PromptKids` returns no children for
  `EFor` and `ERebuild`, under the comment "construction is never an edge", so the `for`
  form never forms the edge.
- **The result:** an edge from `$doc` to `$size`, and from `$size` to `$doc` through
  the `let`. That cycle exists only because `$doc` inherited `x`'s edges.

## What is prompt and what is not

The host's two outputs differ:

- **`x`, the child's result, can be prompt in the arguments.** A child's result crosses
  out of the host in the same tick (`sem-ch09-host-param`: `@Boxed` returns `r`), and
  `@Shown` might pass `$n` straight into its result. The checker cannot see into the
  child, so the worst case is right here (Semantics ch. 7: "where a blueprint arrives as
  a parameter, the worst is assumed").
- **`$doc` is never prompt in anything.** It is structural, so by ST every read of it is
  one construction late.

## A doubt in the books

Semantics ch. 7's list of prompt edges still names "a `for`'s consolidation" as
prompt. That predates round 3 ST, which made the gathered dynamic lagged. The checker
follows ST (no edge for a `for`). The chapter's list should say that a structural
dynamic's read is a delayed edge, beside a sample, a hold's own value and an adoption.

## The fix

- **Edges per name for the host forms.** For a binding whose right side is
  `@HostDocument`, `@HostSome` or `@HostList`, the result's first component (`x`) keeps
  the edges it has now, and the document component (`$doc`, `$slots`) gets none.
  - Where it goes: the `SBind` case of `PromptEdges` needs the pattern's components
    matched against the call's result tuple, not `PatternNames` flattened.
- **Structural dynamics as a class, not a list of names.** The cleaner form of the same
  fix is to mark structural results in the checker's types or summaries. Lowering
  already knows which dynamics are structural (pick ST in `lower.prism`; `Snapshot` in
  both runtimes), and the checker should know it too, so that any
  structural dynamic, including one lifted from `$doc`, reads as a delayed edge. That
  also covers a user's widget function that returns its host's `$doc`.
- **Tests first:**
  - `sem-ch07-host-doc-cycle`: the program above, accepted, with a transcript showing
    the label one owed tick behind the document, as ST predicts;
  - a negative test where the cycle runs through `x` instead (the child's result fed
    back as the child's argument), still refused as a prompt cycle.
