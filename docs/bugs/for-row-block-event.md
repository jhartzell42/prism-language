# A `for` row that reacts to its block's event reacts to the wrong event

*Written by Claude on the claude-self-hosting branch, 4 November 2026; subject to
change.*

## The scenario

A block's button, and a `for` whose rows count its clicks and each have a button of their
own:

```prism
emit Ui.*

@host: @unit
@host:
    !go <- Ui.Button(label=$"go")                                 # 0
    let $names = $ {"a": unit}
    !heard <- for k, ignore <- $names:                            # 1/[k]
        $c <- 0 then $c + 1 on !go
        Ui.Label(text=$"row {$c}")                                # 1/[k]/0
        !b <- Ui.Button(label=$"row button")                      # 1/[k]/1
        @ ((1 on !go) <| (5 on !b))
    $n <- 0 then $n + 1 on !heard
    Ui.Label(text=$"{$n}")                                        # 2
```

```
test go_once, @host:
    build
    click at 0
    compute
    click at 1/["a"]/1
    compute
```

The transcript, the same under the Rust and the C++ target:

```
tick 1
  in:
    0: click

tick 2

tick 3
  in:
    1/["a"]/1: click
  changed:
    1/["a"]/0: Ui.Label(text="row 1")
    2: Ui.Label(text="1")
```

- **The click on `go` does nothing.** The row's `$c` should become 1 and `!heard` should
  fire 1, so `$n` should become 1, in tick 1.
- **The click on the row's own button moves `$c`,** which counts `!go`, not `!b`.
- **Without `!b`** (the row only `@ (1 on !go)`), the click on `go` is lost with no
  trace at all.

The checker accepts the program; the compiled code is silently wrong.

## Is it a bug?

Yes. A row reads its block's names (Semantics ch. 4, `for`), and `$c <- 0 then $c + 1
on !go` in a row is a hold over the block's event, which fires the tick `go` is clicked.
Nothing in the books makes a row deaf to its block's events.

## The cause

Lowering resolves the block's event in the row's own event table:

- **A `for` row's environment comes from `RowReads`** (`prismc/lower.prism`, called by
  `ForLine` as `RowReads menv, parentRef`). `RowReads` rewrites the block's holds and
  dynamic `let`s into reads through the parent, and keeps every other binding as it is
  (`otherwise: en`).
- **So the block's `!go` reaches the row as the block's own `BnEv er`,** whose `er.node`
  is the index of `!go` among the block's event nodes (`gns`).
- **In the row, that index means the row's event node with the same index.** `!go` is
  the block's node 0, and `!b` is the row's node 0, so the row's reactions to `!go` are
  wired to `!b`. In the emitted C++, `HostRow::b` runs `c.set(c + 1)` and
  `anon_left.set(1)`, which are the row's reactions to `!go`; there is no subscription to
  the block's `go_v_port` anywhere in the row. With no row event at index 0, the
  reactions have no node and vanish.
- **The machinery for doing it right exists and is unused here.** `RowEnv` gives a
  `rebuild` generation (`GenRowOf`) a port per block event it names: a fresh port node
  in the row, the row subscribed to the block's port (`subs`), and the block's event
  connected to that port (`SkPort`). `ForLine` never calls it. The record `PortDef` ("a
  port a row subscribes to") is one of the ten unreferenced definitions in the handoff's
  dead-code list.

## Why the suite never caught it

The conformance programs' `for` rows react only to their own elements' events (each row's
button, as `todos` and `sem-ch04-labels` do), and read the block's holds and dynamics,
which `RowReads` handles. No program has a row react to a block event.

## The fix

- **A `for` row gets ports for the block events it names, as a generation does:** call
  `RowEnv` in `ForLine` and bind its port environment into the row's build and method
  environments, as `GenRowOf` does (`Bind ports, ...`).
- **A hard error for the class of bug:** lowering must refuse an event reference whose
  node is not in the node being lowered, naming the binding. An `EvRef` crossing a node
  boundary unconverted is always this bug, and the error would have caught it.
- **Check `RowReads`' other pass-through cases** (`otherwise: en`) for the same
  confusion: any binding that names a node of the block by index.
- **Tests first,** committed failing before the fix:
  - `sem-ch04-row-block-event`: the program above, with the transcript the rules give
    (tick 1: `1/["a"]/0` is `row 1` and `2` is `1`; tick 3: `2` is `2`);
  - the same with no row event of its own (`@ (1 on !go)`), where the click is lost
    today;
  - a row of a row reacting to the outer block's event, through two ports.
