# A host's `$doc` and a `for`'s gathered dynamic disagree in the prompt-cycle check

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
- **Through a `for`:** each row's result is a count of the `for`'s own gathered
  dynamic, so the gathered dynamic depends on itself, the same dependency on a
  structural dynamic:

```prism
@host: @unit
@host:
    let $names = $ {"a": unit, "b": unit}
    $rows <- for k, ignore <- $names:                             # 0/[k]
        Ui.Label(text=$"{$count} rows")                           # 0/[k]/0
        @ $count
    let $count = Size $rows
    Ui.Label(text=$"{$count}")                                    # 1
```

It first showed up in a Prism HTTP host whose listener reads the responses off the
host's `$doc`. There the cycle goes `$doc` to `$answers` to the child.

## Is it a bug?

Yes, but not the way it first looks. The two forms are the same dependency, a block
reading a structural dynamic that its own construction feeds, and the checker gives them
opposite answers. One of the two answers is wrong.

- **Which one:** the checker must not know which dynamics are structural (Jimmy: "don't
  you dare tell the checker which dynamics are structural"). Structurality is how the
  runtimes publish a snapshot (round 3 ST), a timing fact of the implementation; a check
  that relied on the lag would bake the lag into which programs are legal, and the lag
  could then never be removed.
- **So the host form's answer is the right one,** and the `for` form is the bug. A
  checker blind to structurality must take a structural dynamic's read as prompt, and
  then a row that reads its own `for`'s gathered dynamic is a prompt cycle, exactly as
  `$doc` to `$size` to the child is.
- **The HTTP host that found it** is correct to be refused: its listener reads the
  responses off `$doc`, and the loop needs a hold (which is what fixed it).

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
- **The result:** the host form gets an edge from `$doc` to `$size` (inherited from the
  child's result `x`, which is right: the child's document holds `@Shown`'s label, whose
  level reads `$size`) and one back through the `let`, a cycle; the `for` form gets the
  edge back through the `let` and none forward, so its real cycle goes unseen.

## The books

- **Semantics ch. 7** lists "a `for`'s consolidation" among the prompt edges, which is
  what the checker should do and the host form already does. The checker's `for`
  exemption ("construction is never an edge") is broader than R7: R7 says *building* a
  row is not prompt; it does not say that reading the rows' gathered results is not.
- **Round 3 ST** stays as it is for the runtimes. Nothing in the check depends on it.

## The fix

- **A `for`'s gathered dynamic is prompt in what its rows read.** In `PromptKids`, an
  `EFor` keeps no edge for *building* its rows (R7), but a binding of the `for`'s result
  gets edges to the block names its rows' bodies read promptly, as a `let` of the same
  expressions would. The same goes for `ERebuild`'s result.
- **The host forms stay as they are,** prompt in their arguments (the worst case, since
  the checker cannot see into the child).
- **Tests first,** committed failing before the fix:
  - `sem-ch07-for-own-gather`: the `for` program above, refused with `prompt-cycle`
    naming `$rows` and `$count`;
  - the same with the count held (`$count <- 0 then Size (!Updated $rows)`), accepted;
  - every existing program still checks. If one breaks, it has a real prompt cycle by
    this rule and gets a hold, named in its commit.

## A design direction, for Jimmy before any code

Jimmy: "If anything use this to get rid of delay ticks." With the check blind to
structurality and refusing every prompt cycle through a structural dynamic, the owed tick
that ST adds may not be needed:

- **The guarantee the check gives:** nothing that reads a structural dynamic feeds back
  into the propagation that produced it in the same tick.
- **What that allows:** after construction in tick n, run a second propagation over only
  the readers of structural dynamics, in the same tick, with the snapshot taken at the end
  of construction. Readers see one consistent state (Jimmy's DV2 objection to mixed state
  stays met), and the patch of tick n holds their results, with no owed tick.
- **Where a tick is still owed:** a structural read that reaches construction again (a
  `rebuild` or `for` whose key or arm depends on `$doc`), since building in the second
  phase would need another construction. Those stay at n+1, as now.
- **What it would cost:** the viewer's lag (VL) and chess's four owed ticks per move would
  shrink; the transcripts that pin today's owed ticks change; the runtimes gain a second
  phase per tick. A design round with Jimmy first; no code until he rules.
