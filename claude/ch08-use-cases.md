# Chapter 8 — Use cases in depth

Eight programs that are not toys, each taken far enough that the hard moment shows up: the file that changes on disk in the occurrence the user presses undo, the fill that lands with the book update that would have re-quoted it, the emergency stop that arrives with the tick, the member who leaves the room in the moment someone speaks to it, the source file that changes while its compile is finishing, the window that quits under the click that raised it, the jump pressed on the frame boundary, the time range that changes while the old range's data is arriving. For each: the element vocabulary it needs, declared as tags; the widget structure, as code that could compile; the walkthrough of that one moment at the occurrence level; what the goals buy that you would otherwise pay for by hand; and the part of the problem where Prism is slow, awkward, or the wrong tool, said plainly. The chapter ends with the programs Prism is the wrong language for, and what a Prism program does with them instead of being them.

## Conventions

Occurrences are numbered from zero; occurrence zero is the build. Paths are written as in the worked examples: bind positions from the root separated by `/`, `#n` for a `dyn` generation, `[k]` for a `for` key, and `s/n` for a slot that an interceptor's supplier built. Element lists are written one slot per line as `path: element`, with `requested: [..]` after a slot that fired its request in that occurrence.

The GUI vocabulary is the two-level one. `Gui.*` is what a World that draws pixels accepts: `Gui.Window(title)` responds with pointer, key and resize events and has the window's size as state; `Gui.Scene(window, ops)` is the draw list for a window, a presence element replaced whenever the picture changes; `Gui.Focusable(name)` responds with keys; `Gui.Metrics(text, font)` is text measurement as state; `Gui.Frames` responds once per frame while a slot exists. `Ui.*` is the Prism GUI library's own vocabulary (`Ui.Button`, `Ui.Label`, `Ui.TextInput`, `Ui.VBox`, …), claimed by the library's interceptors on the way out and reaching a World only when that World accepts them, which the Test World does. Programs in this chapter are written against `Ui.*` and assume `@Window` from the GUI library around them where a screen is involved; `@Button`, `@Label` and `@TextInput` are the library widgets over `Ui.Button`, `Ui.Label` and `Ui.Focusable`, and `Ui.Button` responds `Click`, a unit type.

```prism
type Click = click
type Tick = tick

tag Ui.Label of Element:
    text: Text

tag Ui.Button of Element:
    label: Text
    responds: Click

tag Ui.Focusable of Element:
    name: Text
    responds: Key

@Label: fn $Text -> @unit = \@widget:
    @Element Ui.Label(text=$_)
    @ unit

@Button: fn $Text -> @!Click = \@widget:
    !click <- @Element Ui.Button(label=$_)
    @ !click
```

A dynamic where a field goes lifts the element into a `dyn`, a fresh slot per change, and a bound event is the `!Switch` over the generations; a bound state is a hold of dynamics joined with `$Join`, starting from the first build's value. The chapter uses that shorthand throughout and spells the `dyn` out only where the generations matter to the walkthrough.

The prelude of plain functions from the worked examples is assumed (`!Filter`, `!Union`, `!Leftmost`, `Insert`, `Remove`, `Fold`, `MapValues`, `Keys`, `Values`, `Concat`, `Len`, and the rest; comparisons are the infix operators `lt`, `le`, `gt`, `ge`, `==` and `!=`), plus two that this chapter leans on: `!Updated: fn $a -> !a`, the change event of a dynamic, firing with the new value in every occurrence the dynamic changes and never at build; and `$Distinct: fn $a -> $a`, which suppresses a change to an equal value, since derived dynamics and holds never do that on their own. Every other function a program uses is defined before its first use, or is so plainly a pure function of its arguments (`Mid`, `Pid`, `Render`) that its body would say nothing about time.

Every deep dive below hits one rule from the worked examples: an operation over one event and any number of dynamics fires with the event and reads each dynamic through a tag, as it stood before the occurrence; an operation over two events is a compile error. When a program wants two things that can land together to both count, it says so, with `!Union` over maps or with one event of a record, and the walkthroughs below are mostly about moments where that choice is the whole bug.
## 1. A text editor

You have used an editor that lost an undo step because the file changed on disk at the wrong moment, and one that marked a buffer clean after a save that had not finished, and one that asked "reload from disk?" about a change it had made itself. Every one of those is a state with more than one writer and no rule about which write wins. An editor is a good first deep dive because it has exactly the states a GUI framework claims to handle for you (the text, the cursor, the undo stack, the dirty flag, the search) and exactly the outside events that it does not (a file changing under you, a save completing), and because the hard moment is where the two meet.

### The vocabulary

```prism
type Key = choice:
    Char Char
    Left
    Right
    Backspace
    Enter

type Instant = Instant Int

type Stamp = choice:
    Missing
    Modified Instant

type FsRequest = choice:
    Read
    Write Text

type FsResponse = choice:
    Contents Text
    Written
    Missing
    Denied

tag Fs.Watch of Element:          # state: the file's modification stamp
    path: Text
    state: Stamp

tag Fs.File of Element:           # request: reads and writes, answered later
    path: Text
    requests: !FsRequest
    responds: FsResponse
```

Three elements talk to the outside for each buffer. `Ui.Focusable(name)` hears keys when it is focused; which buffer is focused is the window interceptor's business, and the buffer never knows. `Fs.Watch(path)` is the watcher, as *state*: its value is the file's modification stamp, pushed when the file changes. `Fs.File(path)` is the one-shot conversation: a `Read` or a `Write` fires, and `Contents`, `Written`, `Missing` or `Denied` comes back addressed to this slot, later. The watcher is state and not a stream for the reason the Worlds chapter gives: a slot built in the occurrence of a change sees the push, while a slot built in the occurrence of a stream's firing would miss it. An editor that opens a file in the moment it changes wants the change.

### The structure

A document is a plain record, and editing is a plain function from a document and a key to a document. The undo stack is a plain record too, and everything the buffer can do to it is one closed choice:

```prism
type Doc = record:
    text: Text
    cursor: Nat

type History = record:
    past: [Doc]
    now: Doc
    future: [Doc]

type Cmd = choice:
    Reload Text
    Undo
    Redo
    FindNext
    Edit Key

let Apply: fn Doc, Key -> Doc = ...          # the text input of chapter 2, plus Enter

let Step: fn History, Text, Cmd -> History = \|h, query, c| match c:
    Reload t: History [[]], (Doc t, (Min h.now.cursor, (Len t))), [[]]
    Undo: match (Last h.past):
        Some d: History (Init h.past), d, (Prepend h.now, h.future)
        Nil: h
    Redo: match h.future[0]:
        Some d: History (Append h.past, h.now), d, (Tail h.future)
        Nil: h
    FindNext: match (NextMatch h.now.text, query, h.now.cursor):
        Some i: History h.past, (Doc h.now.text, i), h.future
        Nil: h
    Edit k: History (Append h.past, h.now), (Apply h.now, k), [[]]
```

`Step` is where every decision about the document lives, and it has no time in it: it can be tested with a list of commands and nothing else. The buffer widget is the time around it:

```prism
let ContentsOf: fn FsResponse -> ?Text = \|r| match r:
    Contents t: Some t
    ignore: Nil

let Unless: fn Text, Text -> ?Text = \|saved, t|
    if saved == t: Nil else: Some t

let Title: fn Text, Bool -> Text = \|path, dirty|
    if dirty: Concat [[path, " *"]] else: path

@Buffer: fn Text -> @!unit = \|path| @widget:
    !started <- @started
    !key <- @Element Ui.Focusable(name=path)
    !undo <- @Button $"Undo"
    !redo <- @Button $"Redo"
    !save <- @Button $"Save"
    !close <- @Button $"Close"
    $query <- @TextInput ""
    !next <- @Button $"Find next"

    $stamp <- @Element Fs.Watch(path=path)
    !resp <- @Element Fs.File(path=path, requests=!io)
    let !io = (Read on (!started | !Updated $stamp)) | (Write $doc.text on !save)
    let !loaded = !Filter (ContentsOf !resp)
    let !reload = !Filter (Unless $saved, !loaded)
    $saved <- "" <= ($doc.text on !save) | !reload

    let !cmd = (Reload !reload) | Undo on !undo | Redo on !redo | FindNext on !next | (Edit !key)
    $hist <- History [[]], (Doc "", 0), [[]] <= Step $hist, $query, !cmd
    let $doc = $hist.now
    $dirty <- False <= True on !key | False on (!save | !reload)

    @Label (Title path, $dirty)
    @Label (Render $doc)
    @ !close
```

The holds are the whole specification of what can happen to a buffer. `$hist` moves on `!cmd` and nothing else, and `!cmd` is a merge whose order is the program's answer to every "what if two of these land together": a reload beats an undo, an undo beats a redo, a search beats a keystroke. `$saved` is the text the buffer last knows to be on disk: what it wrote, or what it loaded. `$dirty` is set by a keystroke and cleared by a save or a reload, and the merge there matters: a keystroke and a save in one occurrence leave the buffer dirty, which is right, because `Write $doc.text on !save` wrote the text *before* the keystroke by the tag law, and the document after the occurrence has one more character than the disk.

`!io` is the only way the buffer talks to the file, and it has three sources. `@started` reads the file once when the buffer is built; a change of the stamp reads it again; a click on Save writes it. The watcher's value at creation comes from a real stat, so `@started` reads the file once and the stamp's first push is a real change.

`Unless` is the line that stops the editor asking about its own saves. A `Write` lands on disk; the watcher pushes a new stamp; the buffer reads; the contents come back equal to `$saved`; the reload is dropped. A change by another program comes back different and is applied. There is no "ignore the next watch event" flag, which is the fix you have written and the fix that has a race in it, because the next event is not always yours. Comparing contents is the test that cannot be wrong about whose change it is.

Buffers are a map keyed by path, so opening a path twice is one buffer, and closing one tears its node down with its history, its watcher and its pending reads:

```prism
type Change = choice Open | Close

let ApplyChanges: fn Text ==> unit, Text ==> Change -> Text ==> unit = \|bs, cs|
    Fold cs, bs, \|acc, p, c| match c:
        Open: Insert acc, p, unit
        Close: Remove acc, p

@editor:
    $path <- @TextInput ""
    !open <- @Button $"Open"
    let !opens = {$path: Open} on !open
    !closes <- for $buffers:
        !closed <- @Buffer key
        @ Close on !closed
    $buffers <- {} <= ApplyChanges $buffers, (!Union !opens, !closes)
    @ unit
```

The cycle through `$buffers` is the todo list's: `ApplyChanges` reads the map through a tag, and a row's close is a click, which cannot arrive in the occurrence the row was born. `!Union` rather than `|` so that opening one file and closing another in the same occurrence both count, and if the same path is opened and closed at once the left side, the open, wins, which the program chose by writing `!opens` first.

### The hard moment

The buffer has `"v1"` loaded and saved. The user types `x`, so the document is `"v1x"`, dirty, with one step of history. Another program writes `"v2"` to the file.

**Occurrence 10.** The World pushes `Modified (Instant 7000)` for `Fs.Watch(path="notes.txt")`. `$stamp` changes; `!Updated $stamp` fires; `Read on ...` fires, which is the `Fs.File` slot's request. Nothing else happens. The World reads the file after the turn.

**Occurrence 11.** The read completes, and the user presses Undo in the same frame; the World delivers both: `Contents "v2"` to the `Fs.File` slot and `Click` to the Undo button. The two are responses to different slots, so the World is entitled to put them in one occurrence, and it has. `!loaded` fires with `"v2"`; `$saved` is `"v1"` through a tag; `Unless` passes it; `!reload` fires. `!undo` fires too. Now `!cmd`: its leftmost firing operand is `Reload "v2"`, so `!cmd` fires with that, and the undo is gone. `Step` replaces the history: no past, `Doc "v2", 2`, no future. `$saved` fires with `"v2"` (its left side, the save, is quiet). `$dirty` clears. Commit: the buffer shows `"v2"`, clean, with nothing to undo.

That is the program's answer and it is defensible: a change on disk is a fact, and the local history was about a text that no longer exists. Three other answers are each one line away, and this is the point of writing them down.

Swap the merge, `Undo on !undo | Reload !reload`, and in occurrence 11 the undo applies to `"v1x"`, giving `"v1"`, and the reload is lost from `!cmd`. But `$saved` has its own definition and still takes `"v2"`, and `$dirty` still clears on `!reload`. The buffer now shows `"v1"`, marked clean, with `"v2"` on disk. Two holds listened to overlapping events and each decided alone; the program said so, and the bug is in its text, not in a race.

Use `!Union` instead, so both count: make `!cmd` a map and fold it, `{0: Reload !reload}` unioned with `{1: Undo on !undo}`, applied in key order. The reload runs first and empties the past, so the undo is a no-op. Same screen as the first answer, by a route that generalises: a program with many simultaneous commands folds them all and never loses one.

Or decide that a reload should not throw history away: `Reload t` keeps `h.past` and makes the reload one more undoable step. Then an undo after a reload restores `"v1x"`, dirty, and a save writes it over `"v2"`. Some editors do exactly this. It is a product decision, and the language's contribution is that it is one `match` arm, in the function with no time in it, rather than a flag checked in two handlers.

### What the goals buy

One way to change anything, written next to the thing: `$hist`'s line is the complete list of what can move the document, and the merge on `!cmd` is the complete ordering of simultaneous commands. There is no handler for the watcher that reaches into the history and no handler for undo that checks a reloading flag.

Simultaneity defined by the program: occurrence 11 is the whole of this section, and its outcome is read off one line.

"Not yet" as a type: a buffer under a slow disk shows an empty document and `Missing` from the watcher until the World has looked, and the `match` arms for that exist because the types made the program write them. There is no spinner state that has to be cleared on three different paths.

The language is enough: the undo stack, the dirty flag, the echo suppression and the search are plain data and plain functions; the only things native are the focusable region, the file and the watcher.

The spec is the thing: the Test World runs this editor with `Push Fs.Watch(path="notes.txt"), Modified (Instant 7000)` and `Send Fs.File(path="notes.txt"), Contents "v2"` inside a `Together` with the Undo click, and the test for occurrence 11 is four lines that a second implementation must agree with.

### Where it is awkward

The text is a `Text`, a plain value, and `Apply` builds a new one per keystroke. For a file of a few thousand lines that is a copy per keystroke and nobody notices; for a hundred-megabyte log it is the whole problem. The answer is a rope or a piece table, which is a plain data structure the prelude can provide and `Apply` can be written over, with no change to anything that has time in it. Until there is an `update` block (future work), writing that structure in Prism is the same functional-update ceremony as in any pure language, and the rope is native code the program calls, like text shaping.

Search is a derived dynamic over the whole text and the whole query, recomputed on every change of either. That is right for a query box and wrong for a hundred-megabyte log, and the fix is the same: an index that is a plain value, updated incrementally by a plain function, with the derived dynamic reading the index. Nothing about occurrences changes.

Syntax highlighting is the same story told the other way: `Highlight $doc.text` is a derived dynamic, prompt, glitch-free, and the whole document is re-highlighted per keystroke. The incremental version is a plain function with a memo keyed on unchanged regions, which is a computation, which is the kind of thing Prism calls out to.

And the cursor blink. A blink is a `Time.Every` and a hold that toggles, and a buffer that is not focused should not blink, so the timer is inside a `dyn` over whether it is focused, which the buffer learns from the window interceptor as a state element or a response. That is right and it is slightly more ceremony than a toolkit's `cursor_visible` property; the ceremony is what makes the blink testable with `Push`.
## 2. A market-data consumer

A trading system is the program that builds the occurrence-as-transaction rule by hand, every time, badly. It holds a book, applies updates in sequence order, decides on the book it has, sends an order, and then has to answer a question no framework helps with: when the fill comes back, which book was the decision made on, and has anything happened to the position since? The teams that get this right keep a single-threaded loop, a sequence number on everything, and a rule that nothing reads the book while it is being updated. That is an occurrence with a sequence number, written in C++. Here it is as the language's own rule, with a plain paragraph about speed at the end.

### The vocabulary

Three things come from outside and one goes out. The feed is a stream of sequenced increments; the snapshot is a request answered with a full book and the sequence number it is good through; the book is a state element whose type says whether it is live; and an order is a request element keyed by a client-side id.

```prism
type Side = choice Bid | Ask

type Increment = record:
    seq: Nat
    side: Side
    price: Int                   # ticks
    qty: Nat                     # 0 removes the level

type Snapshot = record:
    seq: Nat
    bids: Int ==> Nat
    asks: Int ==> Nat

type Book = record:
    seq: Nat
    bids: Int ==> Nat
    asks: Int ==> Nat

type BookState = choice:
    Buffering [Increment]        # no snapshot yet
    Live Book
    Resyncing ((Book, [Increment]))   # a gap was seen; the last good book stays visible

tag Md.Feed of Element:
    symbol: Text
    responds: Increment

tag Md.Snapshot of Element:
    symbol: Text
    requests: !unit
    responds: Snapshot

tag Md.Book of Element:
    symbol: Text
    state: BookState

type NewOrder = record:
    side: Side
    price: Int
    qty: Nat
    on_seq: Nat                  # the book sequence the decision was made on

type OrderRequest = choice:
    Submit NewOrder
    Cancel

type OrderEvent = choice:
    Acked
    Rejected Text
    Filled ((Nat, Int))          # quantity, price
    Cancelled

tag Om.Order of Element:
    id: Nat
    requests: !OrderRequest
    responds: OrderEvent
```

`Md.Book` is supplied either natively, by a World with a book builder next to the exchange socket, or by the `@BookBuilder` interceptor of the Worlds chapter, which holds `Md.Feed` increments, asks `Md.Snapshot` from `@started` and again on a gap, folds the exchange's own recovery procedure into a `BookState`, and supplies that hold to every `Md.Book(symbol)` slot beneath it. The strategy below cannot tell which it is under, and that is the substitutability test from the other direction. It can tell whether the book is `Live`, because the type makes it say so; a strategy with no arm for `Resyncing` does not compile.

### The structure

The whole strategy is one state machine, one hold, and one `for`. Everything that can happen to it arrives as one event of a map, so that a book update and a fill landing together are both applied, in an order the program wrote down:

```prism
type Msg = choice:
    BookChanged BookState
    OrderEvents (Nat ==> OrderEvent)

type Action = choice:
    Place ((Nat, NewOrder))      # a fresh id and the order
    Pull Nat                     # cancel by id

type Strategy = record:
    position: Int
    working: Nat ==> NewOrder    # live orders by id
    next_id: Nat
    book: BookState

let Desired: fn BookState, Int, Params -> ?((NewOrder, NewOrder)) = ...   # the bid and ask we want, if the book is Live
let Reconcile: fn Strategy, Params -> ((Strategy, [Action])) = ...        # diff desired against working

let ApplyMsg: fn Strategy, Msg -> Strategy = \|s, m| match m:
    BookChanged b: Strategy s.position, s.working, s.next_id, b
    OrderEvents evs: Fold evs, s, \|acc, id, ev| match ev:
        Acked: acc
        Rejected ignore: Strategy acc.position, (Remove acc.working, id), acc.next_id, acc.book
        Cancelled: Strategy acc.position, (Remove acc.working, id), acc.next_id, acc.book
        Filled ((q, ignore)): match acc.working[id]:
            Some o: Strategy (acc.position + (Signed o.side, q)), (Remove acc.working, id), acc.next_id, acc.book
            Nil: acc

let Step: fn Strategy, Params, Nat ==> Msg -> ((Strategy, [Action])) = \|s, params, msgs|
    Reconcile (Fold msgs, s, \|acc, ignore, m| ApplyMsg acc, m), params

@MarketMaker: fn Text, Params -> @$Int = \|sym, params| @widget:
    $book <- @Element Md.Book(symbol=sym)
    let !msgs = !Union {0: BookChanged (!Updated $book)}, {1: OrderEvents !events}
    let !stepped = Step $state, params, !msgs
    $state <- Strategy 0, {}, 0, Buffering [[]] <= First !stepped
    let !actions = Second !stepped

    !events <- for $state.working:
        !started <- @started
        let !place = Submit $_ on !started
        let !pull = Cancel on !Filter (PullOf key, !actions)
        !ev <- @Element Om.Order(id=key, requests=(!place | !pull))
        @ !ev

    @ $state.position
```

`!msgs` is a `!Union` of two one-entry maps, so a book change and a batch of order events in one occurrence make one firing with both entries, folded in key order: book first, then order events, then `Reconcile`. Had it been `(BookChanged (!Updated $book)) | (OrderEvents !events)`, a fill landing with a book update would be lost, and the position would be wrong for the rest of the session; the compiler does not catch that, but it does refuse `Step $state, params, (!Updated $book), !events`, two events in one operation, and the refusal is what makes you write the map.

`!stepped` is one event whose value is a pair, and the hold and the actions are its two halves. The alternative, a hold for the state and a separate lifted call for the actions, would read the state twice through tags and could not see the fold's result in the same occurrence. One expression, used twice: there is no "after" inside an occurrence.

`for $state.working` keys an `Om.Order` slot on each live order's id. The order is submitted at the block's birth, from `@started`, in the occurrence the state decided it; the slot lives while the id is in `working`; a partial fill that leaves the id in the map with a smaller quantity changes `$_` and rebuilds nothing; a terminal event removes the id and the slot goes at commit. A late `Cancelled` for an order the program already saw filled is addressed to a path that is gone, and the World discards it, which is right: the conversation ended.

`!pull` reads `!actions` from the enclosing scope, and that is a prompt edge from `!actions` into the block. There is no cycle, because the block's result is `!ev`, the slot's response, and a slot's response has no edge from its request: the creation rule is also a fact the compiler knows. The only prompt edge from a `for`'s collection to its consolidated event is what a new block fires at birth, and here a block fires only its request at birth, never its result.

### What "act on exactly the state you saw" means

An order fired in occurrence n is a pure function of the state before n and the inputs of n, and nothing else. `on_seq` in the order is the book's sequence as `Reconcile` saw it, and `Reconcile` saw a book that was whole, because derived dynamics are glitch-free and the fold inside `Step` runs to completion before anything reads its result. The World reads the request after n commits. Whatever comes back, an ack, a fill, a reject, lands in an occurrence after n, addressed to the slot that asked, and anything that changed the program's view of the world in between was itself an occurrence the program applied. There is no window in which the strategy reads a book with three of five increments applied, no window in which the position is updated but the working set is not, and no way for a response to be handled against a state the request did not see, because the response is a `Msg` folded into the same state machine.

That is the thing the hand-built loop is for, and it is cheaper here because the language refuses the alternatives: no sampling means no reading a half-built book; no callbacks means a fill cannot run code that mutates the working set while a book update is halfway through; one firing per event per occurrence means the fold sees each thing once.

### The hard moment

The book is `Live` at sequence 1041. The strategy has one working bid, id 7, for 100 at 9995, placed on sequence 1040. The exchange fills it, and in the same instant the book moves.

**Occurrence 20.** The World delivers two things it had waiting: a push for `Md.Book(symbol="XYZ")` with `Live (Book 1042, ..)`, and `Filled ((100, 9995))` to `Om.Order(id=7)` at path `1/[7]/1`. They address different slots, so one occurrence is legal, and under a World whose book builder and order gateway share a loop it is the common case.

`$book` changes; `!Updated $book` fires with the new state; `{0: BookChanged ..}` fires. Block `[7]`'s `!ev` fires with the fill; `!events` fires with `{7: Filled ((100, 9995))}`; `{1: OrderEvents ..}` fires. `!Union` fires with both entries. `Step` reads `$state` through a tag, the state before 20: position 0, working `{7: ..}`, book at 1041. It folds: the book becomes 1042; then the fill: position becomes +100 and 7 leaves `working`. `Reconcile` now wants a bid at the new best less one tick, say 9993, and an ask at 9998, and finds nothing working, so it returns `[[Place ((8, (NewOrder Bid, 9993, 100, 1042))), Place ((9, (NewOrder Ask, 9998, 100, 1042)))]]` and a state with `next_id` 10 and both orders in `working`.

`$state`'s change event fires with that state. `$state.working` changes promptly, so the `for` sees key 7 gone and keys 8 and 9 appear. Blocks `[8]` and `[9]` are built now, in 20; each `@started` fires; each `Submit` fires at birth. Block `[7]` is torn down at commit. `!actions` fires with the two `Place`s, which nothing in `[8]` or `[9]` listens to (`PullOf` wants `Pull`). Commit. After the turn the World sees:

```
1/[8]/1:  Om.Order(id=8)    requested: [Submit (NewOrder Bid, 9993, 100, 1042)]
1/[9]/1:  Om.Order(id=9)    requested: [Submit (NewOrder Ask, 9998, 100, 1042)]
```

and sends both, each stamped with the sequence it was decided on.

**Occurrence 21.** `Acked` to `1/[8]/1` and `Acked` to `1/[9]/1`. `ApplyMsg` leaves the state alone; `Reconcile` finds desired and working equal and returns no actions. `!stepped` fires, the hold changes to an equal value, `$state.working` changes to an equal map, the `for` rebuilds nothing because no key appeared or vanished. The position is 100, from occurrence 20, and nothing was ever 100 with order 7 still working.

Now suppose the World had delivered the fill and the book update as two occurrences, fill first. In 20 the fill removes 7, position goes to 100, `Reconcile` places replacements on the 1041 book. In 21 the book moves to 1042, `Reconcile` finds the working bid one tick off from desired, returns `[[Pull 8, Place ((10, ..))]]`, and block `[8]`'s `!pull` fires `Cancel` while block `[10]` is born. One more cancel than before, on a book the program fully applied. The program did not depend on the batching; the World's policy decided how many occurrences the instant took and the program's text said what each one meant. A test under the Test World writes both orders, `Together` and not, and the second implementation must agree on each.

The merge version is what gets written first, and the walkthrough of 20 under it is short: `(BookChanged ..) | (OrderEvents ..)` fires with the book, the fill is never applied, `Reconcile` sees 7 still working at 9995 on a book whose best bid has moved to 9994, pulls it, and places a new bid. The cancel reaches the exchange for an order that no longer exists and is rejected; the position is 0 in the program and 100 at the clearing house. Nothing crashed. That is the bug every one of these systems has had once, and the reason the `!Union` line is the most important one in the program.

### What the goals buy

You can see what moves: `Step $state, params, !msgs` says, before you know what any of them are, that one thing happens and two things are read as they stood. The risk reviewer reads the sigils.

Simultaneity is the program's: the `!Union` and the fold order are the specification of what a fill-and-update instant means, and the two orders of folding (book first, then events) are a visible choice that could be reversed in one line.

"Not yet" as a type: `Buffering`, `Live` and `Resyncing` are arms the strategy must write, so there is no strategy that quotes on a book that is being rebuilt, and nothing that returns an empty book while waiting for the snapshot.

The spec is the thing: the entire session is a transcript of pushes and responses, and a transcript replays exactly. A bad fill from last Tuesday is a test, with the `Together` written where the World batched.

### Latency, as an implementation question

The specification does not claim a latency, and here is what the Rust runtime as described costs. An occurrence is one synchronous run call: fire the inputs, propagate by height through a height-ordered action map, commit the holds, rebuild the element list, return. Values are boxed except small integers; the state record is cloned on every step (a persistent map inside it makes that cheap, a plain array does not); the element list is recomputed in full after each occurrence in the first implementation; and before any of that, the feed's bytes crossed from the socket thread to the World's loop through a queue. For a graph the size of the one above that is on the order of microseconds to tens of microseconds per increment on ordinary hardware, which is three orders of magnitude slower than a tuned kernel-bypass loop and one or two orders faster than it needs to be for a strategy that thinks in milliseconds (an estimate, not a measurement; the runtime chapter's measurements replace it). Every increment is its own occurrence, because increments are responses and responses to one slot never share an occurrence; a World that offers the book as native state instead coalesces, and a strategy that reads `Md.Book` gets one occurrence per batch the World chose. The knobs that would move the number are all in the implementation chapters and none in the language: values representation, an incremental element list, arena allocation per node, a runtime per symbol on its own thread. A system whose whole budget is five microseconds from packet to wire is not this program; it is a loop in C++ that this program supervises, through a state element for the loop's status and a request element for its parameters, with the strategy's decisions taken out of the hot path on purpose.
## 3. A control loop

A controller is the oldest reactive program there is: read the sensors, compute, drive the actuators, wait for the next tick. The synchronous languages (Esterel, Lustre, Lucid Synchrone) were built for exactly this and got the law right decades ago: an instant is atomic, every signal has one value per instant, and `pre` is the only way to see the previous one. Prism has the same law with a different clock: the tick is an occurrence the World opens, `pre` is the tag law, and the controller is a hold whose change event is the control law applied on the tick. Prism adds that the structure can change while the loop runs, and it owes the synchronous tradition a plain statement about timing, which the last part of this section gives.

### The vocabulary

Sensors are state: the latest reading as the occurrence opened. Actuators are requests: a firing is a command, answered with whether it was applied. The tick is a stream, and the emergency stop is state, because a late-built widget that asks "are we stopped" needs an answer in the occurrence it asks.

```prism
type Imu = record:
    gyro: ((Float, Float, Float))
    accel: ((Float, Float, Float))

type ImuState = choice:
    NoReading
    Reading ((Instant, Imu))

tag Sensor.Imu of Element:
    bus: Text
    state: ImuState

tag Sensor.Encoder of Element:
    wheel: Text
    state: Nat                  # ticks since power-on; wraps, and the program knows it

type MotorEvent = choice:
    Applied
    Fault Text

tag Actuator.Motor of Element:
    id: Text
    requests: !Float            # torque, normalised
    responds: MotorEvent

tag Safety.Stop of Element:
    state: Bool                 # True while the stop is asserted

tag Time.Every of Element:
    period: Nat                 # milliseconds
    responds: Tick

tag Clock of Element:
    state: Instant
```

Why the sensor is state and not a stream of samples: a stream delivers at the sensor's rate, each sample its own occurrence, and a 1 kHz IMU under a 100 Hz loop would run ten occurrences per tick, nine of which do nothing. As state, the World coalesces (state is a level, and the last value is the truth), the loop reads the latest reading through a tag on the tick, and the sample-and-hold every controller does by hand is the tag law. A program that wants every sample, for logging or for a filter that needs them all, asks for a stream element beside it, and a World offers both.

### The structure

```prism
type Gains = record:
    kp: Float
    ki: Float
    kd: Float

type Loop = record:
    integral: Float
    last_err: Float
    last_t: Int

type Sample = record:
    setpoint: Float
    measured: Float
    now: Int

let Pitch: fn ImuState -> ?Float = ...        # Nil while NoReading

let Control: fn Gains, Loop, Sample -> ((Loop, Float)) = \|g, l, s|
    let err = s.setpoint - s.measured
    let dt = Or (ToFloat s.now - l.last_t) / 1000.0, 0.01
    let integral = l.integral + err * dt
    let deriv = Or (err - l.last_err) / dt, 0.0
    (((Loop integral, err, s.now), g.kp * err + g.ki * integral + g.kd * deriv))

let IfTrue: fn Bool -> ?unit = \|b| if b: Some unit else: Nil

let Unless: fn Bool, Float -> ?Float = \|stopped, t| if stopped: Nil else: Some t

@Balance: fn Gains, $Float -> @$Bool = \|gains, $setpoint| @widget:
    !tick <- @Element Time.Every(period=10)
    $clock <- @Element Clock
    $imu <- @Element Sensor.Imu(bus="i2c-1")
    $stop <- @Element Safety.Stop

    let !sample = !Filter (SampleOf $setpoint, (Pitch $imu), $clock) on !tick
    let !stepped = Control gains, $loop, !sample
    $loop <- Loop 0.0, 0.0, 0 <= First !stepped
    let !torque = Second !stepped

    let !halt = !Filter (IfTrue (!Updated $stop))
    let !fault = !Filter (IsFault (!left | !right))
    let !cmd = 0.0 on (!halt | !fault) | !Filter (Unless $stop, !torque)
    !left <- @Element Actuator.Motor(id="left", requests=!cmd)
    !right <- @Element Actuator.Motor(id="right", requests=!cmd)
    @ $stop
```

`SampleOf` is a plain function that returns `Nil` while the IMU has no reading, so `!sample` fires on a tick only when there is something to control on, and the loop's state is untouched on the ticks before the first reading. `Control` is the whole control law and has no time in it beyond the timestamps it is handed: `dt` comes from the clock the World pushed, not from the nominal period, because a World skips missed ticks rather than delivering two at once, and a loop that assumed ten milliseconds per tick would integrate wrongly after every hiccup. Division returns an optional and the `Or` says what to do on the first tick, when `last_t` is zero and `dt` is nonsense: the `?` on `/` made the program say so rather than divide by a stale interval and hope.

`!cmd` is the only thing that reaches the motors and its merge is the safety policy in one line: a stop asserted or a fault reported drives zero torque, and that wins over the control law in the same occurrence; otherwise the law's output goes out, unless the stop was already asserted as the occurrence opened. Read it as three cases and you have read the program's answer to every ordering of tick, stop and fault.

### The synchronous view

Here is the Lustre you would write for the integral term (Lustre syntax from memory), with `inc` the increment computed on the same clock, and the Prism beside it:

```
integral = 0.0 -> pre integral + inc

$integral <- 0.0 <= $integral + !inc
```

`->` is the hold's initial value; `pre` is the tag; the clock on which the equation runs is the event the hold's change is lifted over. A Lustre node is a widget; a Lustre program's static causality analysis, which rejects `x = x + 1` with no `pre` (from memory), is the static cycle check, which rejects a hold whose change event reads the hold promptly. The two are the same model with two differences.

The first is the clock. In Lustre the base clock is the program's and every instant is a tick. In Prism an occurrence is whatever the World opened, and a push with no tick, say the stop being asserted between ticks, is an instant in which `on !tick` expressions are silent and everything else runs. That is why `!halt` is derived from `!Updated $stop` rather than from reading `$stop` on the tick: the stop acts in the occurrence it arrives, tick or no tick, and under a World that gives it its own turn the motors get their zero before the next sample. A synchronous program would have to poll the stop on its clock and wait up to a period.

The second is structure. A Lustre program's topology is fixed at compile time. Here the gain schedule can be a `dyn` over a mode, and a mode change tears down one controller's node and builds another, in the occurrence of the change, with the new one seeing all of it:

```prism
    !$halted <- dyn $mode:
        match _:
            Hover: @Balance hover_gains, $setpoint
            Carry: @Balance carry_gains, $setpoint
```

The new node's `$loop` starts from zero, which is the bumpless-transfer problem in a sentence: a controller that must keep its integral across a mode change keeps the hold outside the `dyn` and passes it in, and the program's text says which it does.

### The hard moment

The loop is running. The stop button is pressed, and the World, which batches what arrived together between turns, delivers the stop's push in the same occurrence as a tick.

**Occurrence 500.** Inputs: `Tick` to the `Time.Every` slot, a push of `Instant 5000` for `Clock`, a push of `Reading (..)` for `Sensor.Imu(bus="i2c-1")`, and a push of `True` for `Safety.Stop`. `!tick` fires. `SampleOf` reads `$setpoint`, `$imu` and `$clock` through tags: the setpoint, the IMU reading and the clock *before* the occurrence, the ones the previous tick also saw as "latest" plus whatever was pushed in occurrences since. `Control` fires with a torque, say `0.31`. `$stop` changes to `True`; `!Updated $stop` fires with `True`; `!halt` fires. Now `!cmd`: the left side, `0.0 on (!halt | !fault)`, fires, so `!cmd` fires with `0.0`, and the `0.31` from the right side is not heard. Both motor slots fire `0.0`. Commit: `$loop` has integrated the error (the law ran; its output was overridden), `$stop` is `True`.

```
4:  Actuator.Motor(id="left")     requested: [0.0]
5:  Actuator.Motor(id="right")    requested: [0.0]
```

**Occurrence 501.** The next tick, with the usual pushes. `Control` fires `0.29`. `Unless $stop, !torque` reads `$stop` through a tag, `True`, and returns `Nil`; `!Filter` drops it; the left side of the merge is quiet (`!Updated $stop` did not fire; the stop did not change). `!cmd` is quiet. Nothing is sent, and the motors hold the zero they were given. The loop keeps integrating, which is the wind-up problem and a product decision: a program that wants the integral frozen while stopped puts `$stop` into `Sample` and lets `Control` see it.

**Occurrence 530.** The stop is released: a push of `False` for `Safety.Stop`, with a tick. `!Updated $stop` fires with `False`; `IfTrue` drops it; `!halt` is quiet. `Unless $stop, !torque` reads `$stop` through a tag, still `True` before this occurrence, and drops the torque. `!cmd` is quiet. The motors stay at zero for this tick, and from 531 on the tag reads `False` and torque flows. One tick of delay on release, by the tag law, and none on assertion, by `!Updated`. A program that wants release to act in its own occurrence derives the torque from `!Updated $stop` as well, and the asymmetry is the right default: stopping is urgent and starting is not.

Now the same stop under a World that delivers each input in its own turn. Occurrence 500 has the push alone: `!tick` is quiet, `!torque` is quiet, `!halt` fires, `!cmd` fires `0.0`. The motors stop one turn earlier than the next tick. Occurrence 501 has the tick: as above. The program's text covered both without a case split.

Had the program written `!cmd = !Filter (Unless $stop, !torque)` alone, trusting the tag, the stop would act one tick late in both Worlds, and under the batching World the torque computed in 500 would go out with the stop already asserted. Ten milliseconds, in a loop that may be the only thing between a motor and a person. The merge with `!Updated` is one line, and it is readable as "a change of the stop wins in the moment it changes."

### What the goals buy

One way to change anything: `$loop` moves on `!stepped` and nothing else; `!cmd` is the complete list of what can reach a motor, with its priority order. There is no interrupt handler that writes a torque behind the loop's back.

Simultaneity: the stop-and-tick instant is a line of the program, under either World policy.

No clock: `dt` is from a state element the test sets. The step response of the controller is a test with `Push Clock` and `Push Sensor.Imu(..)` per tick and `Requested Actuator.Motor(id="left"), 0.31` as the assertion, and it runs in no time at all, because there is no timer in it.

The language is enough: the control law, the safety policy, the mode switch and the anti-windup are all Prism. The native things are the bus, the motor driver and the timer, which is the list of things that touch hardware.

### Where it is awkward, and where it is wrong

Prism promises nothing about *when* an occurrence runs. The tick is a `Time.Every` the World delivers when its loop gets to it; the run call takes as long as the graph takes; a `dyn` rebuild in the tick path (the mode switch above, in the same occurrence as a tick) takes as long as the build takes. A controller with a deadline needs the World to run the loop on a thread with real-time priority, to keep its own IO off that thread, and to measure the occurrence; the language gives it a deterministic amount of work per occurrence and nothing more. Structural changes belong outside the tick path or inside the jitter budget, and the advice is to measure the rebuild under the Test World and decide.

A World skips missed ticks rather than delivering them late, which is the right policy for a controller and the wrong one for a program counting ticks; such a program counts the clock instead.

Estimation is a computation. A Kalman filter over a six-by-six covariance is a plain function called on the tick, and writing it in Prism means writing matrix code in a language with no mutable arrays and no `update` block yet; it is native code the prelude exposes, like text shaping, and the Prism program decides when to call it and what to do with the result.

And the innermost loop is not here. A motor driver's current loop at twenty kilohertz, with a deadline measured in microseconds, lives in firmware, and `Actuator.Motor` is the interface to it: the Prism program sets a torque target at a hundred hertz and the driver closes the fast loop itself. The synchronous-language view is the whole justification for Prism in this domain, and the synchronous languages themselves were never used for the fastest loop either; they were used for the layer that decides.
## 4. A chat server

You have written the server where a connection object is kept in a map by hand, removed in a close callback that sometimes runs after the next read callback, so the map has a dead entry that the next broadcast crashes on. Or the one where a user who left the room still got three messages because the membership set was read before the leave was applied, by a handler that ran first for no reason anyone wrote down. A chat server is "maintain a model, fan out its changes" with nothing else in it, which makes it the cleanest place to see what a `for` keyed on connection ids and a single state machine buy, and the right place to see what happens to a message addressed to someone who is leaving in the same moment.

### The vocabulary

```prism
type Accepted = Accepted Nat

type ConnRequest = choice:
    Send Text
    Close

type SocketEvent = choice:
    Received Text
    Closed

type Link = choice:
    Connected Nat               # bytes queued and not yet written
    Disconnected Text

tag Net.Listener of Element:
    address: Text
    responds: Accepted

tag Net.Conn of Element:
    id: Nat
    requests: ![ConnRequest]
    responds: SocketEvent
    state: Link

type Stored = choice:
    Absent
    Present [Text]

type PutResult = choice:
    Stored
    Failed Text

tag Store.Value of Element:
    key: Text
    state: Stored

tag Store.Append of Element:
    key: Text
    requests: ![Text]
    responds: PutResult
```

The listener is a stream of accepted connections with ids the World allocated. A connection is the three-part element: requests are what to send, responses are what arrived and the close, and the link is state shared by value, which here means nothing, since each id is one connection. The store is a key-value component: a value as state, with `Absent` in the type, and an append as a request answered later. A World that has no store answers `Unsupported` to the append and errors at the creation of the state slot, and a program that must run without one checks `World.Tags` first; this one assumes a store.

### The structure

The model is one record, and every command any connection can issue is one closed choice:

```prism
type Cmd = choice:
    Nick Text
    Join Text
    Say Text
    Who
    Leave                       # synthesised from Closed

type Model = record:
    nicks: Nat ==> Text
    rooms: Text ==> (Nat ==> unit)      # room name to its members
    member_of: Nat ==> Text             # one room per connection

# Apply every command that landed in this occurrence, in key order, and say what
# each connection should be sent as a result. One function, one result, no "after".
let StepAndRoute: fn Model, Nat ==> [Cmd] -> ((Model, Nat ==> [Text], Text ==> [Text])) = ...

let Presence: fn Model -> Text ==> [Text] = \|m|
    MapValues m.rooms, \|members| Sorted (Names m.nicks, members)
```

`StepAndRoute` is the server. It folds the commands of an occurrence into the model and, as it goes, accumulates the lines each connection is owed: the room's members get `"<nick> hi"` on a `Say`, `"nick joined"` on a `Join`, `"nick left"` on a `Leave`; the asker gets the member list on `Who`. It is a pure function over a map of command lists, and every policy question (does a leaver see their own leave notice, does a join to a full room fail, is `Who` answered before or after the joins in the same occurrence) is an arm of it, testable with no sockets.

The widget around it:

```prism
let IdOf: fn Accepted -> Nat = \|a| match a: Accepted n: n

let TextOf: fn SocketEvent -> ?Text = \|ev| match ev:
    Received t: Some t
    Closed: Nil

let IsClosed: fn SocketEvent -> ?unit = \|ev| match ev:
    Closed: Some unit
    ignore: Nil

let SendAll: fn [Text] -> [ConnRequest] = \|lines| for lines: Send (Concat [[_, "\r\n"]])

let HasLeave: fn [Cmd] -> Bool = ...

@server:
    !accepted <- @Element Net.Listener(address="tcp://0.0.0.0:6667")
    let !opens = {IdOf !accepted: Opened}

    !cmds <- for $conns:
        ((!ev, $link)) <- @Element Net.Conn(id=key, requests=!out)
        let !split = SplitLines (Concat [[$rest, !Filter (TextOf !ev)]])
        $rest <- "" <= Second !split
        let !parsed = ParseAll (First !split)
        let !gone = [[Leave]] on !Filter (IsClosed !ev)
        let !out = SendAll (!Filter (!owed[key]))
        @ !parsed | !gone

    let !stepped = StepAndRoute $model, !cmds
    $model <- Model {}, {}, {} <= First !stepped
    let !owed = Second !stepped
    let !said = Third !stepped

    let !closes = MapValues (Filter !cmds, HasLeave), \|ignore| Shut
    $conns <- {} <= ApplyDeltas $conns, (!Union !opens, !closes)

    let $presence = Presence $model
    @ $presence
```

Each connection's block does framing (the `$rest` hold and `SplitLines`, as in the Worlds chapter), parses complete lines into commands, turns a close into a `Leave`, and sends whatever the model owes it. `!owed[key]` is an event of a map indexed by a plain key, which lifts to an event of an optional; `!Filter` keeps the occurrences in which this connection is owed something. The block's result is its commands, consolidated by `for` into `!(Nat ==> [Cmd])`, keyed by connection id, with exactly the connections that said something in this occurrence.

The cycle runs through the whole program and is legal at every edge. `$conns` is held from `!closes`, which comes from `!cmds`, the result of `for $conns`; that passes through the tag on `$conns` in `ApplyDeltas` and through the fact that a block's commands are responses, which never arrive in the occurrence the block was born. `!out` inside a block reads `!owed`, which is derived from `!cmds`, the consolidated result of the very `for` the block is in; that is a prompt edge from the collection's result into every block, and it is fine because the edge ends at a request field, and a slot's response has no edge from its request. The compiler sees a `for` whose blocks fire nothing at birth, which is the only prompt edge from a collection to its consolidated event it has to worry about.

A `Say` and a `Leave` from two different connections in one occurrence are two entries of `!cmds`. A `Say` and a `Leave` from *one* connection cannot share an occurrence, because they are both responses to one slot (`Received` and `Closed`), and a World delivers at most one response per path per occurrence. That is the one place the model is simpler than the sockets it describes, and it is simpler because the World promised it.

### Presence as a derived dynamic

`$presence` is the room-to-members map, derived from `$model`, prompt and glitch-free: in any occurrence it is the model after that occurrence's commands, never a membership set with the join applied and the nick change not. It is the widget's result, so a program that wraps `@server` in an admin page shows it, and a `Who` command is answered inside `StepAndRoute` from the same model, so the two can never disagree. A derived dynamic changes whenever its operand changes, including to an equal value, so a dashboard that redraws on `!Updated $presence` wraps it in `$Distinct` first; a message said in a room changes `$model` and therefore fires `$presence`'s change event with the same membership, and the dashboard would redraw per message otherwise. Equality on a map of arrays of text exists, so `$Distinct` is available.

### Persistence

A room's history is a state element per room, and an append is a request per line said:

```prism
    $histories <- for $model.rooms:
        $h <- @Element Store.Value(key=(Concat [["room:", key]]))
        @Element Store.Append(key=(Concat [["room:", key]]), requests=(!Filter (!said[key])))
        @ $h
```

`for $model.rooms` is keyed by room name, so a room's history slot and append slot live as long as the room has members and go when it empties; `!said` is the third output of `StepAndRoute`, the lines to persist per room. `$histories` is `$(Text ==> Stored)`, with a value in the occurrence each room appears, from the oracle, which is `Absent` until the World has loaded the key and `Present` after; a join that wants to send the last twenty lines reads `$histories` through a tag in `StepAndRoute` and gets what was known as the occurrence opened. A `Failed` from the append is a response the block can hold and retry behind a `Time.After`, and the retry policy is the block's text.

The store is also the sharding seam. Two rooms share nothing but the listener, so a World that owns the listener can route each accepted connection to one of several runtimes, each running this program, with the store as the only thing they share; the Worlds chapter has the router. Nothing in `@server` changes between one runtime and ten.

### The hard moment

Connections 5, 9 and 12 are in `lobby`. Connection 5's client disconnects, and in the same instant connection 9 says "hi".

**Occurrence 300.** The World delivers `Closed` to `1/[5]/0` and `Received "hi\r\n"` to `1/[9]/0`: two slots, one occurrence, which the World may do and a busy server often will. In block `[5]`, `!gone` fires with `[[Leave]]`. In block `[9]`, the chunk is split, `ParseAll` gives `[[Say "hi"]]`, `!parsed` fires. `!cmds` fires with `{5: [[Leave]], 9: [[Say "hi"]]}`. `StepAndRoute` reads `$model` through a tag and folds in key order: 5 first. `Leave` removes 5 from `lobby` and from `member_of`, and owes 9 and 12 the line `"five left"`. Then 9: `Say "hi"` is routed to `lobby`'s members *as the fold now has them*, 9 and 12, so each is owed `"<nine> hi"` as well. `!owed` fires with `{9: [["five left", "<nine> hi"]], 12: [["five left", "<nine> hi"]]}`. Block `[9]`'s `!out` fires with two `Send`s; block `[12]`'s likewise; block `[5]`'s `!owed[5]` is `Nil` and nothing fires there. `!closes` fires with `{5: Shut}`; `$conns` loses key 5; block `[5]` is torn down at commit. After the turn the World writes two lines to 9 and two to 12 and sees the `Net.Conn(id=5)` element gone, which it already knew.

Nothing was sent to 5, and the reason is that the leave and the say were folded by one function in one pass with one model, so the say saw the leave. Now the other way to write it, which is the way the first version always gets written: a `Step` that updates the model and a separate `Route` lifted over `$model` and `!cmds`:

```prism
    $model <- ... <= Step $model, !cmds
    let !owed = Route $model, !cmds
```

`Route` reads `$model` through a tag, the model before 300, in which 5 is still in `lobby`. `!owed` fires with 5 owed `"<nine> hi"`. Block `[5]`'s `!out` fires `[[Send "<nine> hi\r\n"]]` in the occurrence block `[5]` is torn down. The runtime reports that firing to the World with its path, `1/[5]/0`, because the program asked; the World performs it if it can, which for a closed socket it cannot, and discards any response, since the slot is gone. The user never sees the message and nothing crashes. The program paid one wasted send and one line in the World's diagnostics; the callback version would have paid a write on a closed descriptor from a handler that still held the connection object, and the exception that goes with it.

Key order is doing work in 300. Within an occurrence there is no order of arrival; the leave and the say are simultaneous, and the program chose to apply them in key order, which here means 5 before 9. Had the leaver been 12, the fold would have routed the say to 12 first and removed 12 after, and 12's block would have fired a send on its own teardown. A program that wants leaves applied before says regardless of id sorts the occurrence's commands by kind inside `StepAndRoute` before folding, and that is one line and a stated policy. The point is that the policy is in the program's text, and that in the callback version it was in the order two sockets happened to become readable.

### What the goals buy

One way to change anything: `$model` has one change event, and `StepAndRoute` is the complete list of what any command does to it. Membership cannot drift from `member_of`, because both are written by one function in one place.

Simultaneity: occurrence 300 is a `!Union` and a fold order, both visible.

No node values: there is no connection object to keep in a map and forget to remove. A dead connection has no slot, and a write to it is impossible to express.

The spec is the thing: the Test World accepts `Net.Listener` and `Net.Conn`, so the whole server, including the race above, is a script of `Send Net.Conn(id=5), Closed` inside a `Together` with `Send Net.Conn(id=9), Received "hi\r\n"` and a `Requested` step per recipient. The server you tested is the server you shipped.

### Where it is awkward

Every message is an occurrence, and every occurrence recomputes the element list in the first implementation, which with a hundred thousand connections is a hundred thousand entries per message. The incremental element list is the fix and it is an implementation item, not a language one; until it exists, the server that needs that scale is several runtimes with fewer connections each, which the sharding seam allows anyway.

Fan-out to a room of five thousand is a map with five thousand entries in `!owed`, five thousand request firings, and five thousand performs by the World after the turn. That is the right amount of work and it is done on the World's side, where the write buffers are; the program cannot stream it, because a request firing is atomic per occurrence. A room that large wants its fan-out in the World, as a `Net.Multicast(group)` element the program sends to once, and a World can offer that without the language knowing.

Backpressure is `$link`'s `Connected n`, readable through a tag at the moment of sending, and a block that drops or queues when `n` is large has a policy in its text. A block that ignores it works until a slow client fills the World's buffer, which is the same as every other server, and the difference is that the fix is one `match` in the block rather than a writable-callback dance.

Parsing a wire protocol is string work in a pure language with no mutable buffers. It is fine for lines and tedious for anything with length prefixes and binary fields; the prelude should offer the usual codec functions natively, and the program should call them.

TLS is the World's: `tls://` in the address and a `Disconnected "certificate .."` in the link type is all the program sees.
## 5. A build system

Every build tool is an incremental computation with a file watcher bolted on afterwards, and the bolt is where it leaks: the rebuild that ran twice because the editor wrote the file in two syscalls, the stale object that survived because the watcher fired while the compiler was still writing, the dependency the tool did not know about because it was discovered inside a compile. A dynamic *is* an incrementally maintained value, and a `for` over a map *is* an incrementally maintained collection, so a build system in Prism is the thing itself rather than a graph the tool maintains by hand. It is not a fast compiler, and the end of this section says where the line is.

### The vocabulary

```prism
type Stamp = choice:
    Missing
    Modified Instant

tag Fs.Watch of Element:
    path: Text
    state: Stamp

type Command = record:
    program: Text
    args: [Text]
    cwd: ?Text

type SpawnResponse = choice:
    Spawned Nat
    NotFound
    Denied

tag Process.Spawn of Element:
    requests: !Command
    responds: SpawnResponse

type ChildInput = choice:
    Stdin Text
    CloseStdin
    Signal Sig

type ChildOutput = choice:
    Stdout Text
    Stderr Text
    Exited Int

type ChildStatus = choice:
    Running
    Exited Int
    Gone

tag Process.Child of Element:
    id: Nat
    requests: !ChildInput
    responds: ChildOutput
    state: ChildStatus

tag Cli.Stdout of Element:
    requests: !Text
    responds: WriteFailed
```

Files are state: a target's inputs are `Fs.Watch` slots whose stamps are the facts the build is a function of. A compiler is a process: spawning is a request (a one-shot effect with a result, an id the World allocated), and the child is a state-and-stream element keyed by that id, whose `Exited` arrives both as a response (an edge the target counts) and as state (a level a late-built widget can read). When a `Process.Child` slot is torn down, the World's cancel-on-teardown obligation kills the child; a target that stops wanting a compile stops having the slot, and the compile stops.

### The structure

The dependency graph is a map from target to rule. Each target is a `for` block over that map, and a target's dependencies on other targets are a read of the `for`'s own consolidated result:

```prism
type Rule = record:
    inputs: [Text]              # source files
    deps: [Text]                # other targets
    output: Text
    command: Command

type Job = record:
    id: Nat
    on: Text ==> Stamp          # input stamps as they stood when the job was spawned

let AllKnown: fn Text ==> Stamp -> Bool = ...     # no input Missing
let Newer: fn Text ==> Stamp, Text ==> Stamp -> Bool = ...

@Target: fn Rule, $(Text ==> Nat) -> @$Nat = \|rule, $versions| @widget:
    $stamps <- for $ (KeyBy rule.inputs):
        $s <- @Element Fs.Watch(path=key)
        @ $s
    let $deps = Project $versions, rule.deps           # dependency versions, as a map
    let $want = $Distinct (Wanted $stamps, $deps)      # the (stamps, dep versions) this build is a function of

    !sp <- @Element Process.Spawn(requests=(rule.command on !Filter (ReadyFor $job, (!Updated $want))))
    $job <- Nil <= (Some (Job (!Filter (IdOf !sp)), $stamps)) | Nil on !done
    !child <- dyn $job:
        match _:
            Some j:
                ((!out, $st)) <- @Element Process.Child(id=j.id, requests=!never)
                @ ((!Filter (ExitOf !out), $st))
            Nil:
                @ ((!never, $ Gone))
    let !done = !Switch (First !child)
    $version <- 0 <= $version + 1 on !Filter (IsZero !done)
    @ $version

@build: fn Text ==> Rule -> @unit = \|graph| @widget:
    $versions <- for $ graph:
        $v <- match graph[key]:
            Some rule: @Target rule, $versions
            Nil: @ $ 0
        @ $v
    @Element Cli.Stdout(requests=(Report (!Updated $versions)))
    @ unit
```

Take it from the bottom. `$versions` is `$(Text ==> Nat)`, one entry per target, each target's count of successful builds, and it is passed back into every `@Target` as a dynamic. A target derives `$want` from its input stamps and its dependencies' versions; when `$want` changes to a new value (`$Distinct`, because a stamp push with an equal value is not a reason to rebuild), the target spawns its command. The spawn's response is an id; the `dyn` over `$job` builds a `Process.Child(id)` slot for it; the child's `Exited 0` bumps `$version`; and the bump reaches every target that depends on this one through `$versions`, promptly, in the same occurrence. A chain of three targets whose root input changed rebuilds in three turns separated by two compiles, because each spawn is a request answered later, and never rebuilds a target before its dependency has finished, because `$deps` cannot change until the dependency's `$version` does.

The read of `$versions` inside the `for` that produces `$versions` is a cycle, and it is legal. `$version` in a target is a hold; its change event fires on `!done`, which is a response from a `Process.Child` slot, and a slot's response has no edge from anything the program did. So the loop from `$versions` into a target and back out to `$versions` passes through the World, which is the delay that makes every build system's dependency graph acyclic in time even when the tool's own data structure is not: a dependency's new version arrives in an occurrence after the dependency was spawned, which is after the dependent read the old version. Had `$version` been a derived dynamic over `$versions` with no response in between, the cycle would be prompt and the compiler would refuse it, which is the spreadsheet case below.

`Job` records the input stamps as they stood when the job was spawned. That is the line that makes the hard moment come out right.

### The hard moment

Target `lib` has one input, `lib.c`, and target `app` depends on `lib`. The user saves `lib.c`; `lib` is compiling; the user saves `lib.c` again just as the compile finishes.

**Occurrence 40.** The World pushes `Modified (Instant 1000)` for `Fs.Watch(path="lib.c")`. In `lib`, `$stamps` changes, `$want` changes, `!Updated $want` fires, `ReadyFor` passes it, `rule.command on ..` fires: the `Process.Spawn` slot's request. The World spawns after the turn.

**Occurrence 41.** `Spawned 77` to the spawn slot. `$job` becomes `Some (Job 77, {lib.c: Modified 1000})`; the `dyn` rebuilds with a `Process.Child(id=77)` slot, whose oracle answer is `Running`.

**Occurrence 42.** The compile finishes and the user's second save lands in the same instant. The World delivers `Exited 0` to the child slot at `0/[lib]/2/#1/0` and pushes `Modified (Instant 1300)` for `lib.c`. In `lib`: `!out` fires; `ExitOf` passes `0`; `!child`'s current generation fires its exit; `!Switch` delivers `!done` with `0`; `$version + 1 on ..` fires with `1`; `$version` changes. In the same occurrence `$stamps` changes to `{lib.c: Modified 1300}`, `$want` changes, `!Updated $want` fires. `ReadyFor` is the function that decides whether to spawn now, and it reads `$job` through a tag: `Some (Job 77, {lib.c: Modified 1000})`, a job still running as the occurrence opened. The program has to say what a change during a running job means, and this program's `ReadyFor` says: spawn, since the stamp the job was spawned on is older than the stamp now. So the spawn request fires again in 42. `$job`'s change event has two sides: the left is lifted over `!sp`, the spawn's *response*, which did not fire in 42 (the request did; the answer comes later), and the right is `Nil on !done`, which did. `$job` becomes `Nil`; the `dyn` rebuilds to the `Nil` arm; the child slot for 77 is torn down at commit, which is fine because 77 has exited.

Meanwhile `$versions` changed, so `app`'s `$deps` changed, `app`'s `$want` changed, and `app` fires its own spawn in 42, against the `lib` that was just built from the 1000 stamp. That is correct and wasteful: `app` will be built from the old `lib`, then `lib` will be rebuilt from the 1300 stamp, then `app` again. The program could avoid the first `app` build by having `lib` not bump `$version` when its stamps have moved past the job's: `IsZero !done` becomes `Fresh $job, $stamps, !done`, reading both through tags, and the job's recorded stamps are exactly what that test needs. The decision to record the stamps at spawn time, by the tag law in `Some (Job .., $stamps) on !sp`, is what makes either policy expressible; a build tool that records nothing cannot tell a compile that finished on time from one that finished stale.

**Occurrence 43.** `Spawned 78` to `lib`'s spawn slot. `$job` becomes `Some (Job 78, {lib.c: Modified 1300})`. Everything proceeds. Had the second save and the exit been separate occurrences, the walkthrough is the same with the steps in two places, and the program never cared which.

The version with a watcher-as-stream and a mutable dirty set does, in occurrence 42, the bug from the first paragraph: the exit handler clears the dirty flag, the watch handler sets it, and whichever ran second decides whether `lib` is rebuilt. Here there is no second; there is one occurrence, and `ReadyFor` saw both facts.

### A spreadsheet is the same thing, until it is not

A spreadsheet is cells with formulas that reference other cells, and a change to one cell recomputes the cells that depend on it. Write it as the build system above with cells for targets and formulas for commands and it fails to compile, and the failure is instructive. A cell's value would be a derived dynamic over the consolidated `$values` of the `for`, with no hold and no World response in the loop; that is a prompt cycle through a `for`'s result, and the static check refuses it. It is right to refuse it. The build system's loop was legal because every dependency crosses the World and comes back later; a spreadsheet's dependencies are between values in one occurrence, and a cycle among them is the thing the spreadsheet must detect, not the thing the language can wave through.

So the sheet's values are one derived dynamic over the sheet's formulas:

```prism
@sheet:
    !edits <- for $ (Grid 26, 100):       # one block per cell reference, "A1" to "Z100"
        $text <- @TextInput ""
        @ !Updated $text
    $formulas <- {} <= Union $formulas, !edits
    let $values = Evaluate $formulas      # topological order, cycle detection, all inside
    for $values:
        @Label (Show $_)
    @ unit
```

`Evaluate` is a plain function from the whole map of formulas to the whole map of values. It sorts, detects cycles, and reports them as a value (`Cycle [Ref]` in the cell's result type) rather than as an abort. Every keystroke in any cell recomputes the whole sheet, and for a thousand cells that is nothing and for a million it is the problem. The incremental evaluator, which recomputes only the cells downstream of the edit, is a plain function too: it takes the old values and the edit and returns the new values, and a hold that keeps the old values gives it its argument through a tag. That is incremental computation written as a fold, which is what a build system's dependency graph is once you stop pretending the tool's data structure is the graph.

The division of labour is the one this chapter keeps finding. Between files and processes, where the edges cross the World and the facts arrive at different times, the dynamics and the `for` are the dependency tracker and the program writes none of it. Between cells in one occurrence, where all the facts are present at once, the dependencies are data, and data is a plain function's business.

### What the goals buy

One way to change anything: a target's `$version` moves on one event, its child's exit, and nothing can bump it because a file was touched or a sibling finished.

No request on creation: a target built in occurrence zero spawns nothing until `$want` changes, and a `$Distinct` over it makes "changes" mean what the build means by it. The build that runs on startup because the tool was restarted is not a thing here, and the one that runs because an input is `Missing` is a line the program wrote (`AllKnown`).

"Not yet" as a type: a target whose input is `Missing` is a target the program had to write an arm for; there is no rebuild loop against a file that does not exist.

The language is enough: the dependency graph, the dirty tracking, the stale-job detection and the scheduling are all the hold-and-`for` structure of the program. The native things are the watcher, the spawn and the child.

The spec is the thing: the build system's race in occurrence 42 is a `Together` of a `Send Process.Child(id=77), Exited 0` and a `Push Fs.Watch(path="lib.c"), Modified (Instant 1300)`, and the expected `Requested Process.Spawn, ..` after it.

### Where it is awkward, and where it is wrong

The compile is a computation and it lives in a process, which is right, and it means the fine-grained incrementality everyone wants from a build system (rebuild this function, not this file) is not something the build system can offer; it is something the compiler must. Prism tracks dependencies at the granularity of what crosses the World: files and processes. A compiler that reports the headers it actually included (a depfile) is one more `Fs.File` read after the exit and a `$discovered` hold that joins the rule's inputs, which is a few lines, and dynamic dependencies turn out to be ordinary.

A build with ten thousand targets is a `for` with ten thousand blocks, each with its own `Fs.Watch` slots, and the World has ten thousand watches, which on most platforms is fine and on some is not; aggregating them is an interceptor (one watch per directory, fanned out by value) and an afternoon.

`Evaluate` for a million cells, or any evaluator that wants a mutable worklist, is the kind of code Prism is wrong for, and the right answer is the same as for the rope: a native function the program calls. A spreadsheet in Prism is a thin layer of time over an evaluator written in something else, and the thin layer is where the bugs were.
## 6. A shell and a window compositor

A shell is a World written in Prism that hosts programs; a compositor is the same thing with a screen. The worked examples have the small version: two programs, one shown at a time, clicks routed to the visible one. This section takes it to a desktop: any number of programs launched and quit at runtime, overlapping windows with a stacking order, focus, and the one invariant a compositor must keep that a toolkit never states, which is that a click lands on what the user saw. The hard moment is the window that quits under the click that raised it.

### The vocabulary

The compositor speaks the primitive GUI vocabulary downward to the World and, upward to the applications it hosts, the same vocabulary again. From an application's side the compositor *is* a World that accepts `Gui.Window` and `Gui.Scene`; from the real World's side the compositor is one program with one window.

```prism
type Size = record:
    w: Int
    h: Int

type Point = record:
    x: Int
    y: Int

type Rect = record:
    x: Int
    y: Int
    w: Int
    h: Int

type WindowInput = choice:
    Pointer Point
    Pressed Key
    Released Key

tag Gui.Window of Element:
    title: Text
    responds: WindowInput
    state: Size

tag Gui.Scene of Element:
    window: Text
    ops: [Op]

type Code = Code Int

tag Process.Exit of Element:
    requests: !Code

tag Process.Spawn of Element:
    requests: !Command
    responds: SpawnResponse

tag Process.Child of Element:
    id: Nat
    requests: !ChildInput
    responds: ChildOutput
    state: ChildStatus
```

`Process.Exit` is a request. An application that wants to end fires `Code 0` on it; nothing ends because a window closed or a widget was dropped. That is the rule the compositor leans on hardest, because it is what lets the compositor decide what an application's exit means (remove its frame) rather than having the World decide (end the process the compositor is running in).

### Processes as state: the shell half

Before the screen, the shell. A process table is a keyed collection of state elements, and the shell's own job list is a derived dynamic over it:

```prism
type Entry = record:
    id: Nat
    command: Command

@Shell: @widget:
    !line <- @Lines                                   # standard input, one line per firing
    let !spawn = !Filter (ParseCommand !line)
    !sp <- @Element Process.Spawn(requests=!spawn)
    let !opens = {!Filter (IdOf !sp): Opened}
    $table <- for $jobs:
        ((!out, $status)) <- @Element Process.Child(id=key, requests=!never)
        @Element Cli.Stdout(requests=(!Filter (StdoutOf !out)))
        @ $status
    let !closes = MapValues (Filter (!Updated $table), IsExited), \|ignore| Shut
    $jobs <- {} <= ApplyDeltas $jobs, (!Union !opens, !closes)
    @Element Cli.Stdout(requests=(Render (!Updated $table)))
    @ unit
```

`$table` is `$(Nat ==> ChildStatus)`: one entry per child, each a state element the World answers `Running` for from the oracle and pushes `Exited n` on. `jobs` is a rendering of `$table` on each change; a child leaves the table when its status says it exited, which is a `for` whose membership is a hold moved by the consolidated state's own change, legal because `ApplyDeltas` reads `$jobs` through a tag and a child's exit is a push. Piping one child's output into another's input is the `Defer`-per-chunk pattern in the Worlds chapter, and a `Signal Term` on a `Process.Child` is a request like any other. A shell that spawns a child and is itself torn down kills the child, because the World's cancel-on-teardown obligation covers `Process.Child` slots.

### The compositor

Applications are a catalogue, so that a frame can be keyed by an id and hold its application as plain data:

```prism
type App = choice Counter | Stopwatch | Editor

let Launch: fn App -> @unit = \|a| match a:
    Counter: @counter
    Stopwatch: @stopwatch
    Editor: @editor

type Frame = record:
    app: App
    rect: Rect

type Desk = record:
    frames: Nat ==> Frame
    order: [Nat]                # bottom to top
    focus: ?Nat
    next: Nat

type DeskCmd = choice:
    Open App
    Raise Nat
    Quit Nat
    Move ((Nat, Rect))

let StepDesk: fn Desk, Text ==> DeskCmd -> Desk = ...      # fold in key order; a Raise or Move of a missing id is a no-op

type FrameOut = record:
    !quit: DeskCmd
    $ops: [Op]
```

A frame is an interceptor around one application. It supplies the application's window its size from the frame's rectangle, holds the application's scene back and offers it upward as a dynamic, holds `Process.Exit` back and reports the exit as a result, delivers the pointer and key events the compositor addresses to it, and forwards everything else:

```prism
let IsFramed: fn Element -> Bool = \|e| match e:
    Gui.Window(..): True
    Gui.Scene(..): True
    Process.Exit(..): True
    ignore: False

@Frame: fn App, $Rect, !WindowInput -> @((!Code, $[Op])) = \|app, $rect, !input| @widget:
    ((ignore, $slots)) <- intercept (Launch app), !down, $ supply
    let supply = \|e| match e:
        Gui.Window(..): @ (SizeOf $rect)
        ignore: @Element e
    !held <- for $slots:
        dyn $_:
            match _.element:
                Process.Exit(..): @ _.!request
                ignore: @ !never
    let !exit = !Filter (AnyValue !held)
    let $ops = SceneOf $slots
    !fwd <- @Forward (Filter $slots, \Not (IsFramed _.element))
    let !to_window = !Filter (Addressed $slots, !input)
    let !down = !Union !to_window, !fwd
    @ ((!exit, $ops))
```

`Addressed` is the plain function that turns an input into a one-entry map keyed by the application's `Gui.Window` slot path, read from `$slots` through a tag, or `Nil` if the application has no window yet; `SceneOf` is the plain function that finds the application's `Gui.Scene` element in the list and takes its `ops`, or `[[]]`; both read the application's list, and an application that draws two windows is an application the frame draws as one, which is a policy the frame could change. `AnyValue` takes the first value of a map, if there is one, so `!exit` fires with the code of whichever exit slot fired; the frame does not know its own id, so it reports a code and the desktop turns it into a `Quit` of the right frame.

The desktop is a `for` over frames, a hit-test, and a fold:

```prism
@Desktop: @widget:
    ((!raw, $size)) <- @Element Gui.Window(title="Desktop")
    let !hit = !Filter (HitFrame $desk, (!Filter (PointerOf !raw)))      # ((id, local point)), topmost first
    let !keys = !Filter (KeyOf !raw)

    frames <- for $desk.frames:
        let !own = (Pointer (!Filter (ForFrame key, !hit))) | (Pressed (!Filter (WhenFocused $desk, key, !keys)))
        !gen <- dyn $_.app:
            out <- @Frame _, $_.rect, !own
            @ out
        let !quit = (Quit key) on (!Switch (First !gen))
        $$ops <- $ [[]] <= Second !gen
        @ FrameOut !quit, ($Join $$ops)

    let !system = !Union ({"open": Open Counter} on !Filter (IsLaunchChord !keys)), {"raise": RaiseOf !hit}
    let !cmds = !Union !system, (Prefixed "quit:", frames.!quit)
    $desk <- Desk {}, [[]], Nil, 0 <= StepDesk $desk, !cmds

    let $scene = Compose $desk, frames.$ops
    @Element Gui.Scene(window="Desktop", ops=$scene)
    @ unit
```

`dyn $_.app` builds the frame once per key, since an application never changes for a key, and is there only to get `_` as a plain `App`; its result crosses the `dyn` by the usual two routes, `!Switch` for the quit and a hold joined for the ops. `frames` is a plain record of consolidated results: `frames.!quit` is `!(Nat ==> DeskCmd)`, firing with exactly the frames that quit in an occurrence, and `frames.$ops` is `$(Nat ==> [Op])`, every frame's picture. And `!cmds` is a `!Union` of everything that can change the desk, keyed by text so that a frame's quit and a raise from the same click are two entries folded in key order, never one winning.

Focus and layering are nothing but `$desk.focus` and `$desk.order`, read through tags. `HitFrame` walks `order` from the top and finds the first frame whose rectangle contains the point, against the desk as the occurrence opened: the user clicked on the stacking they saw. `WhenFocused` passes a key to the frame whose id is `focus` as the occurrence opened. `Compose` translates each frame's ops by its rectangle and concatenates them in `order`, bottom to top, so the scene the World draws after the occurrence is the stacking after the occurrence. There is no z-buffer, no focus manager and no event capture: three fields of one record, one function that moves them, and the tag law.

### The hard moment

Frames 5 and 9 overlap, 9 on top, 5 focused. The user clicks the visible corner of 5, on the spot where 5's application has drawn its Quit button.

**Occurrence 70.** The World delivers `Pointer (Point 40, 300)` to `0`, the desktop window. `HitFrame` reads `$desk` through a tag: `order` is `[[5, 9]]`, 9's rectangle does not contain the point, 5's does, so `!hit` fires with `((5, (Point 12, 20)))`, the point in 5's coordinates. Two things follow from one firing. `RaiseOf !hit` fires `Raise 5`, so `!system` fires with `{"raise": Raise 5}`. And block `[5]`'s `!own` fires with `Pointer (Point 12, 20)`, so `@Frame` for 5 fires `!to_window` with `{w: Pointer (Point 12, 20)}` where `w` is the application's window path, and `intercept` delivers it to the application's `Gui.Window` slot against the application's list as the occurrence opened.

Inside application 5, the library's `@Window` hit-tests the point against its layout as the occurrence opened, finds the Quit button, and fires its click; the application's `Process.Exit(requests=(Code 0 on !quit))` fires `Code 0`. Back in the frame, `!held`'s block for the exit slot sees `_.!request` fire; `!held` fires with `{p: Code 0}`; `!exit` fires; it crosses the `dyn` through `!Switch`, which has been listening to this generation since the frame was built; block `[5]`'s `!quit` fires with `Quit 5`; in the desktop, `frames.!quit` fires with `{5: Quit 5}`, and `!cmds` fires with `{"quit:5": Quit 5, "raise": Raise 5}`.

`StepDesk` reads `$desk` through a tag and folds in key order. `"quit:5"` first: frame 5 leaves `frames`, `order` becomes `[[9]]`, `focus` becomes `Nil`. Then `"raise"`: `Raise 5` on a desk with no frame 5 is a no-op, as `StepDesk` was written. `$desk` changes; `$desk.frames` changes promptly; the `for` sees key 5 gone and tears block `[5]` down at commit, with the whole application under it: its holds, its timers, its sockets, every slot, each cancelled by the World under its obligations when the next list arrives without them. `frames.$ops` changes promptly to a map without 5; `Compose $desk, frames.$ops` is a derived dynamic over the new desk and the new ops, and it is glitch-free: there is no instant in which the scene has 5's ops with 5 missing from `order`, or `order` naming a frame with no ops. The scene rebuilds, the World draws a desktop with one window. The application's own `Gui.Scene` also rebuilt in 70 (its button drew pressed), in a node that is dropped at commit, and nobody saw it. Nothing about the exit reached the World below, because the frame held `Process.Exit` back and it was never in the compositor's list.

```
0:        Gui.Window(title="Desktop")
2/#41/0:  Gui.Scene(window="Desktop", ops=[[... frame 9 only ...]])
```

Now the two ways to get this wrong, each a line. If `!cmds` were `!system | (Prefixed ..)`, the raise would win and the quit would be lost: application 5 fired its exit, nothing happened, and it cannot fire again, since the firing was a click's consequence and the click is over. A program with a dead application in a frame that cannot quit. If `StepDesk` had applied `"raise"` before `"quit:5"`, by sorting its commands the other way, the result would be the same desk, because the fold applies both; the only order that matters is that both are applied, and that is what `!Union` promises.

And the click that *raised* 9 while 9's application was quitting on a timer: `Raise 9` and `Quit 9` in one occurrence, from different sources, folded the same way, same answer. The compositor never has a frame that was raised and then found missing, because it never has a "then."

### What the goals buy

The language is enough: a window manager is an interceptor, a process table is a `for`, focus is a field. The World under the desktop is the software rasteriser, or the X11-over-a-socket interceptor from the Worlds chapter, and it knows nothing about frames.

Nothing happens because an element appeared: an application appears on the desk because `Open` was folded into `$desk`, and leaves because `Quit` was. The World below never sees an application's exit, and the compositor's own run ends only if the compositor fires `Process.Exit` itself.

No node values: an application cannot keep a reference to its frame, and the compositor cannot keep a reference to an application. When frame 5 is removed from the map, everything under it is gone, and there is no listener somewhere holding the dead subtree alive.

The spec is the thing: the hard moment is a Test World script with `Send Gui.Window(title="Desktop"), Pointer (Point 40, 300)` and an `ExpectTopNone` for frame 5's label afterwards, and the same script runs with the desktop under a `@VBox` or under nothing.

### Where it is awkward, and where it is wrong

Every application runs in the compositor's runtime, in the same occurrence. One application's prompt cycle or overflow aborts the occurrence for all of them, and the Errors chapter's quarantine is the answer within one runtime: the World quarantines the failing subtree on its next run call, and the desk shows a dead frame. That is containment, not isolation. Real isolation means real processes, and then the application is a `Process.Spawn`ed program speaking the GUI vocabulary over a socket, with the compositor as the X11 interceptor's mirror image: an interceptor that turns `Net.Conn` bytes into a `Gui.Scene` and pointer events back into bytes. That is the design for untrusted applications, and this section's design is for trusted ones, which is most of what a desktop runs.

`Compose` concatenates every frame's ops on every change of any frame, and the World rasterises the whole scene. There are no damage regions, no retained layers, and no GPU textures the compositor could blit instead of redrawing. A scene of ten thousand ops at sixty frames a second is fine; a browser window's worth of content behind a terminal is not, and the fix is in the World (retained layers keyed by frame id, which the ops can name) and in the vocabulary (an `Op` that references an image the World already holds), not in the language.

Images and video are opaque handles the World allocates (`ImageId`), which means an application that decodes a video frame does so natively and hands the compositor a handle, and the compositor never sees pixels. That is right, and it means the compositor cannot do a pixel effect without asking the World for one.
## 7. A game

A game loop is the one program every programmer has written as a mutable loop and knows is wrong in the same place: input is polled at the top, physics is integrated in the middle, rendering reads the result at the bottom, and the press that arrived between the poll and the render is either dropped, applied a frame late, or applied to half-integrated state, depending on where the input callback happened to run. A tick is an occurrence, the world state is a dynamic, and the press on the frame boundary is the hard moment. A game also shows, better than anything else, that rendering is a description: the element list is the draw list, and the World draws it after the turn, so there is no render call anywhere in the program.

### The vocabulary

```prism
type Frame = frame

tag Gui.Window of Element:
    title: Text
    responds: WindowInput              # Pointer, Pressed, Released
    state: Size

tag Gui.Scene of Element:
    window: Text
    ops: [Op]

tag Gui.Frames of Element:
    responds: Frame                    # once per frame the World is about to draw, while a slot exists

tag Clock of Element:
    state: Instant

type Sound = choice Jump | Land | Hit

tag Audio.Play of Element:
    requests: !Sound
    responds: Played
```

`Gui.Frames` is a stream that exists only while a slot for it does; a paused game puts the slot inside a `dyn` over the pause flag and the World stops asking for frames. `Audio.Play` is a request: a sound happens because the program fired it, after the turn, which is the right time, since the turn is the frame.

### The structure

```prism
type Entity = record:
    kind: Kind
    pos: ((Float, Float))
    vel: ((Float, Float))
    grounded: Bool

type Held = record:
    left: Bool
    right: Bool
    jump: Bool

type Game = record:
    entities: Nat ==> Entity
    held: Held
    score: Int
    last_t: Int

type Msg = choice:
    Down Key
    Up Key
    Tick Instant

type Effect = choice:
    Play Sound

let Step: fn Game, Nat ==> Msg -> ((Game, [Effect])) = \|g, msgs|
    Fold msgs, ((g, [[]])), \|st, ignore, m| match m:
        Down k: (((Press (First st), k), Second st))
        Up k: (((Release (First st), k), Second st))
        Tick now: Integrate (First st), now, (Second st)     # physics, collisions, scoring; appends effects

@game:
    ((!raw, $size)) <- @Element Gui.Window(title="Jump")
    $clock <- @Element Clock
    !!frames <- dyn $paused:
        if _: @ !never
        else: @Element Gui.Frames
    let !frame = !Switch !!frames
    let !key = !Filter (KeyOf !raw)
    $paused <- False <= (Not $paused) on !Filter (IsPause !key)

    let !msgs = !Union {0: !Filter (KeyMsg !key)}, {1: Tick ($clock on !frame)}
    let !stepped = Step $game, !msgs
    $game <- Initial <= First !stepped
    @Element Audio.Play(requests=(!Filter (FirstSound (Second !stepped))))

    $sprites <- for $game.entities:
        $phase <- 0 <= ($phase + 1) on !frame
        @ Draw $_, $phase
    @Element Gui.Scene(window="Jump", ops=(Concat (Values $sprites)))
    @Element Ui.Label(text=$"Score: ${$game.score}")
    @ unit
```

`Step` is the game. It folds the occurrence's messages in key order: key edges first (key `0`), then the tick (key `1`), so a press that lands with a frame is applied before the frame integrates. `Press` and `Release` set the held flags; `Integrate` advances every entity by the time since `last_t`, resolves collisions, scores, and appends an `Effect` for each thing that should make a sound. It has no time in it beyond the timestamps it is handed, and it is the function a replay runs.

The tick's timestamp is `$clock` read through a tag on `!frame`: the clock as the occurrence opened. A World that pushes the clock in the same occurrence as the frame gives the step the previous frame's time, one frame behind, constantly; the differences between consecutive ticks are still the frame intervals, so `Integrate` is right, and a program that wants the frame's own time asks for `Gui.Frames` to respond with it instead. The tag law is not a bug here; it is a fixed, stated lag, which is more than a game loop's `now()` ever promised.

`$sprites` is a `for` over the entities, which looks like more than a draw loop needs, and is: `Draw` over `$game.entities` as one derived dynamic would do for a picture. The `for` earns its keep when an entity has *widgets*: a `$phase` of its own that survives other entities spawning and dying, a `Time.After` for a death animation that must outlive the entity's removal from physics, a particle emitter with holds. A key is an identity, and an entity's local state belongs to its key. The consolidated `$(Nat ==> [Op])` is concatenated in key order into one `Gui.Scene`, and that scene is the entire output of the program: a presence element, replaced every frame, drawn by the World after the turn. There is no `render()`; there is a dynamic whose value is the picture.

Sounds are requests. `Integrate` cannot play a sound, because nothing in Prism can; it returns `Play Jump` in its effects, and the `Audio.Play` slot fires it. One request per occurrence per slot, so a frame with two sounds either fires an array (make the request type `[Sound]`) or picks the first, as `FirstSound` does here; the program said which.

### The hard moment

The player is on the ground. They press Space in the instant the next frame is due.

**Occurrence 1200.** The World, which batches between frames, delivers `Pressed Space` to the window slot and `Frame` to the frames slot in one occurrence, with a push of the clock. `!key` fires with `Space`; `KeyMsg` turns it into `Down Space`; `{0: Down Space}` fires. `!frame` fires; `Tick ($clock on !frame)` fires with the clock before the occurrence; `{1: Tick ..}` fires. `!Union` fires with both. `Step` reads `$game` through a tag and folds: `Down Space` sets `held.jump`; then `Tick` integrates, and `Integrate` sees `held.jump` true with the player grounded, sets a vertical velocity, clears `grounded`, and appends `Play Jump`. `$game` changes; the player's `$_` in the `for` changes; `Draw` draws the player one step into the air; the scene rebuilds; `Audio.Play` fires `Jump`. After the turn the World draws the frame with the player already rising and plays the sound. The press was applied in the frame it arrived in.

Here is the version everyone writes first:

```prism
    $held <- Held False, False, False <= (Press $held, (!Filter (DownOf !key))) | (Release $held, (!Filter (UpOf !key)))
    let !stepped = Integrate $game, $held, ($clock on !frame)
```

A hold for the keys, updated by key events, read by the frame step through a tag. In occurrence 1200, `$held` changes to `jump = True` and `Integrate` reads `$held` through a tag, *before* the occurrence: `jump = False`. The player does not jump this frame. In 1201 the tag reads `True` and the jump happens, one frame late, sixteen milliseconds after the press. That is the input lag that game programmers measure with high-speed cameras (from memory), and here it is a consequence of one line that could be read: the step reads the keys as they stood before this frame. The `!Union` version reads them as the fold has them, with this occurrence's edges applied, and the difference between the two programs is visible in their text rather than in a frame counter.

And the version with `|`: `(KeyMsg !key) | (Tick ..)` loses the tick when a key lands with it, which is a dropped frame on every keypress, or loses the key, which is a stuck or missed input depending on whether it was a press or a release. Neither is a race; both are the program saying the wrong thing, and the two-events-in-one-operation error is what stops the third version, `Integrate $game, !key, !frame`, from ever being written.

Now the same press under a World that gives every input its own turn. Occurrence 1200 has the key alone: `!msgs` fires with `{0: Down Space}`, `Step` sets `held.jump`, nothing integrates, the scene rebuilds with the same picture (derived dynamics do not dedupe, and the World diffs or redraws; a `$Distinct` on the ops would spare it). Occurrence 1201 has the frame: `Integrate` sees `jump` held and the player rises. Same result, one occurrence later in numbering and the same frame on screen. The program covered both policies because it said what a key means and what a frame means, separately, and let `!Union` say what both together mean.

### Replay, for free

The sequence of inputs the World delivered, with their occurrence numbers and the clock pushes, is the entire session. Given the same transcript, the program's sequence of scenes is bit-identical, because there is no `now()`, no random source that is not a state element, no scheduling, and no float that depends on which thread ran first. A replay file is a transcript; a demo is a replay; a test of a level is a replay with an `Expect` at the end. Lockstep networking is two peers running the same `Step` over the same transcript, exchanging inputs rather than state, and the thing lockstep netcode spends its life fighting, nondeterminism between the two simulations, cannot happen between two Prism runtimes running one program, and the differential test between the two implementations is the guarantee that it cannot happen between a Rust client and a Haskell server either.

### What the goals buy

Simultaneity is the program's: a key and a frame in one instant is the `!Union` and the fold order, which is the first thing a game programmer wants to be able to state and the last thing a loop lets them.

No clock: the step is a function of the clock the World pushed, so a slow-motion mode is a World that pushes a slower clock, and a test runs a thousand frames in no time.

You can see what moves: `Step $game, !msgs` is the whole loop, and `$sprites` reads as a picture derived from state, which is what rendering is.

Nothing is chosen for the implementer's comfort: the `for` over entities with per-key state is the same construct as the todo list, and a dying entity's animation outliving its physics is a `Time.After` in its block, not a "pending removals" list.

### Where it is awkward, and where it is wrong

`Integrate` is a computation over every entity every frame, and it builds a new map each time. A persistent map makes an unchanged entity free and a changed one a path copy, which for a few thousand entities at sixty frames a second is fine. Fifty thousand bullets is not this; it is an array of structs updated in place by a loop, which is the kind of code that wants an `update` block and, until then, native code the program calls with the map and gets a map back. The physics engine, the navigation mesh, the particle system: computations, called out to, with the Prism program owning when they run and what their results mean.

Fifty thousand entities is also fifty thousand `for` blocks, each rebuilding a sprite per frame, and the first implementation recomputes the element list per occurrence. For a picture alone, do not use the `for`; derive the ops from the map in one plain function and keep the `for` for the entities that have widgets, which is a few.

The GPU is the World's. An `Op` can name a mesh or a texture by a handle the World allocated, and a shader is a World component with a request element for its parameters; a program that wants to write a shader in Prism is writing a computation in the wrong language. The vocabulary of `Op` for a real game is a retained scene graph with handles, not a list of rectangles, and that vocabulary is a World's to offer; the language only fixes that it is plain data and that it is drawn after the turn.

Fixed-timestep integration with an accumulator, the standard advice for stable physics, is a loop inside `Integrate`, in the plain function, where it belongs. Audio latency is the turn: a sound requested in a frame plays after it, which at sixty frames a second is under a frame of lag, and a rhythm game that wants better schedules sounds ahead through a request with a timestamp, which is a World policy.

And a thousand-player simulation is a server, and the sharding seam is the one from the Worlds chapter: one runtime per region, the World routing players between them, and the handoff of an entity between regions a teardown in one `for` and a birth in another, through a store.
## 8. A dashboard over HTTP

Twelve charts on a page, each asking a server for a time series every few seconds. You have built this, and you remember the three things that went wrong: the chart that flashed the old range's data under the new range's title because the fetch for the old range landed after the user dragged; the twelve identical requests per tick because each chart had its own timer; and the cache that was "simple" until it had a TTL, a stale indicator and a race with the in-flight request. All three are about where state lives and who may change it, and the answer here is that a chart asks for a query *as state*, something between the chart and the network supplies it, and whether that something polls, pushes or caches is a choice of interceptor that the chart never sees.

### The vocabulary

```prism
type HttpRequest = record:
    method: Text
    url: Text
    headers: Text ==> Text
    body: Text

type HttpAnswer = record:
    status: Nat
    headers: Text ==> Text
    body: Text

type HttpResult = choice:
    Answered HttpAnswer
    Failed Text

tag Http.Fetch of Element:
    requests: !HttpRequest
    responds: HttpResult

type Range = record:
    from: Int
    to: Int

type Query = record:
    metric: Text
    range: Range
    step: Int

type QueryState = choice:
    Unavailable
    Known ((Instant, [((Int, Float))]))   # fetched when, and the rows
    Errored Text

tag Query.Result of Element:
    query: Query
    state: QueryState

tag Time.Every of Element:
    period: Nat
    responds: Tick

tag Clock of Element:
    state: Instant
```

`Http.Fetch` is the World's: fire a request, get an answer later. `Query.Result` is the dashboard's own tag, a state element whose value is the latest result of a query, and the point of the section is that nothing in the chart says who supplies it. A World with a push channel to the server can accept it natively and push each new result; a World that has only HTTP needs an interceptor to supply it by polling; a test supplies it with `State`. State is identified by value, so two charts with equal queries observe one dynamic wherever it comes from.

### The chart

```prism
let IsKnown: fn QueryState -> ?QueryState = \|s| match s:
    Known ignore: Some s
    ignore: Nil

let Loading: fn QueryState -> Bool = \|s| match s:
    Unavailable: True
    ignore: False

@Chart: fn Text, $Query -> @unit = \|title, $query| @widget:
    $live <- @Element Query.Result(query=$query)
    $shown <- Unavailable <= !Filter (IsKnown (!Updated $live))
    @Element Gui.Scene(window=title, ops=(Plot title, $shown, (Loading $live)))
    @ unit
```

`$query` is a dynamic and `query` is a field, so the element is inside a `dyn`: a new query is a new slot, and `$live` is the hold of the generations' dynamics joined, starting from the first build's value. `$shown` is the last `Known` the chart has seen, across every change of query: it moves only when `$live` changes to a `Known`, so a fresh slot's `Unavailable` leaves the old rows on screen with a loading badge drawn from `Loading $live`. That is the whole "show stale data while loading" policy, two lines, and it is in the chart because it is the chart's policy; a chart that would rather go blank omits `$shown`.

A dashboard is charts over a shared range:

```prism
@dashboard:
    $range <- @RangePicker
    let q = \|m| Query m, $range, 60
    @Chart "cpu", (q "cpu")
    @Chart "cpu again", (q "cpu")
    @Chart "mem", (q "mem")
    @Chart "disk", (q "disk")
    @ unit
```

`q "cpu"` applies a lambda whose body reads a dynamic, which lifts: a `$Query` that changes with the range. The first two charts ask for equal elements and get one state. Four charts, three queries, however many suppliers there are below.

### Polling as an interceptor

```prism
let Encode: fn Query -> HttpRequest = ...
let Decode: fn Instant, HttpResult -> QueryState = \|now, r| match r:
    Answered a: if a.status == 200: Known ((now, ParseRows a.body)) else: Errored (ToText a.status)
    Failed why: Errored why

@Poller: fn Nat, @a -> @a = \|period, @dash| @widget:
    ((result, $slots)) <- intercept @dash, !down, $ supply
    !down <- @Forward $slots
    let supply = \|e| match e:
        Query.Result(query=q): @widget:
            !started <- @started
            !tick <- @Element Time.Every(period=period)
            $clock <- @Element Clock
            !res <- @Element Http.Fetch(requests=((Encode q) on (!started | !tick)))
            $latest <- Unavailable <= Decode $clock, !res
            @ $latest
        ignore: @Element e
    @ result
```

The supplier returns a widget for each `Query.Result` element, and the runtime builds it once per distinct element value, in the poller's node, when the first slot with that value is created, and keeps it until the last such slot is gone. So the two `cpu` charts share one timer and one fetch per period, by value identity, without the poller counting anything. The widget fires a fetch at birth and on every tick, decodes the answer with the clock as it stood, and holds the result; the hold is the slot's dynamic, with `Unavailable` as its value in the occurrence the first slot was created, which is the truth: nobody has asked yet.

Under a World that accepts `Query.Result` natively, pushing a new value whenever the server sends one, the program is `@dashboard` with no `@Poller` around it, and nothing in `@Chart` changes. Polling versus pushes is, from the chart's side, no difference at all; from the World's side it is the difference between a request element and a state element, and the interceptor is the adapter between them.

### Caching as an interceptor

The poller's supplied widget dies when the last slot for its query goes, so dragging the range away and back refetches. A cache that survives that lives outside the poller, where the poller's `Http.Fetch` elements are visible, and it is the HTTP cache of the Worlds chapter with a TTL in its lookup:

```prism
@main:
    @HttpCache (@Poller 5000, @dashboard)
```

The order matters and is readable: the cache is outermost, so it sees the poller's fetches, and the poller sees the dashboard's queries. A GET the cache has answered within the TTL is answered from the cache's hold through a `Defer`, in the next occurrence of the same turn; anything else is forwarded. The cache does not know what a query is, and the poller does not know there is a cache. Each is a few lines and each is tested alone.

### The hard moment

The dashboard shows the last hour of `cpu`. A poll for that range is in flight. The user drags the range picker to the last day, and the poll's answer arrives in the same instant.

**Occurrence 90.** The World delivers `Answered ..` to the poller's `Http.Fetch` slot for the hour query, at `s/0/3`, and a key to the range picker's focusable, in one occurrence. The range picker's hold changes; `$range` changes; both `cpu` charts' `$q "cpu"` change promptly to the day query; each chart's `dyn` rebuilds: generation `#1` is built with a `Query.Result(query=(Query "cpu", (Range day), 60))` slot. The runtime asks the poller's supplier for that element, finds no supplied widget for it, applies `supply`, and builds one now: `@started` fires, the fetch for the day query fires at birth, `$latest` is `Unavailable`. Both charts' new slots get that `$latest`, since the second chart's slot, created in the same occurrence with an equal element, joins the widget already built. In each chart, `$live`'s hold of dynamics changes to the new generation's dynamic, and `$Join` changes to `Unavailable`; `!Updated $live` fires with `Unavailable`; `IsKnown` drops it; `$shown` keeps the hour's rows. The scenes rebuild with the hour's rows and the loading badge.

Meanwhile the answer. It was addressed to `s/0/3`, the hour widget's fetch slot, which existed as the occurrence opened, so it is routed: `!res` fires, `Decode` fires, the hour widget's `$latest` changes to `Known`. Whose dynamic is that? The two `#0` generations' slots, which are being torn down at commit, and nothing else. `$Join` in each chart has already switched to the `#1` inner; its change event fires on the outer change with the new inner's value, and the old inner's change in the same occurrence is not heard, because the outer changed. So `!Updated $live` fired once, with `Unavailable`, and the hour's fresh rows reached no chart. At commit, the `#0` generations go, the hour's slots go, the hour's supplied widget has no slot left and goes too, with its fetch slot and its timer.

The dashboard's list, as the poller sees it, with the poller's own supplied slot beside it:

```
1/0/#1/0:   Query.Result(query=(Query "cpu", (Range day), 60))
2/0/#1/0:   Query.Result(query=(Query "cpu", (Range day), 60))
s/1/3:      Http.Fetch                                  requested: [GET .../cpu?range=day]
```

The old range's fresh data did not flash under the new range's title, and the reason is not a guard; it is that the chart's `dyn` rebuilt in the occurrence the query changed, and a widget built in an occurrence sees the occurrence from the start, so the new generation's `$live` was never the old slot's dynamic. There is no instant in which the chart has the day's query and the hour's rows as its live value. It has the hour's rows as `$shown`, which it chose to keep, labelled as loading.

**Occurrence 91.** The cache. The day fetch fired at `s/1/3` in 90, and the `@HttpCache` around the poller has a block for that slot, built in 90 when the slot appeared, which saw the request fire in 90 under the mid-occurrence construction rule. If the user had looked at the day range in the last TTL, the cache's `$cache` has it: `!hit` fired in 90, the cache's `Defer` fired in 90, and 91 is the second occurrence of the same turn, delivering the `Defer`'s answer; the cache fires the cached answer down to `s/1/3`; `!res` fires; `$latest` becomes `Known`; both charts' `$Join` change; `!Updated $live` fires with a `Known`; `$shown` moves to the day's rows; the badge clears. The user sees the day's chart before the World has drawn the loading state, because the turn was not over. If the cache had nothing, 91 does not happen; the World performs the fetch after the turn, and the answer lands in a later turn by the ordinary route.

Had the answer for the hour arrived one turn later instead of in 90, its path `s/0/3` would not be in the list the World last received, and the World would discard it. Same screen either way. The hour's rows are gone from the poller and present in the cache, which saw them pass through in 90 as an answer to a GET, and a drag back to the hour is a cache hit.

### What the goals buy

The boundary nests: polling, caching and the push World are three suppliers of one tag, stacked or swapped without touching a chart. The chart you tested with `State Query.Result(..), Known ..` is the chart you ship under the poller, and the poller you tested with `Send Http.Fetch, Answered ..` is the poller you ship under the cache.

"Not yet" as a type: `Unavailable` is a case the chart matched on, and `$shown` is the chart's explicit decision about what to draw then. There is no undefined series and no chart that renders an empty array as a flat line.

Identity by value: twelve charts of one query are one fetch, with no registry of subscribers and no reference count, because the runtime builds a supplier's widget once per distinct element.

Routing against the opened list: the answer that arrived with the drag reached the slot it was for and nothing else, and the slot's teardown at commit is what made the rest of the walkthrough unnecessary.

### Where it is awkward

A poll's phase restarts whenever the supplied widget is rebuilt, which happens whenever a query's last slot goes and a slot with that value comes back; a dashboard whose charts toggle in and out polls on an irregular schedule, and a World that wanted phase continuity for `Time.Every` could key it by tree position, which under a supplier it cannot see. The cache is the answer: rebuilds are cheap when the data is a hit.

Decoding a large body is a computation on the runtime's thread, inside the occurrence: `ParseRows` over ten megabytes of JSON is ten megabytes of JSON parsed between the response arriving and the scene being drawn, with the whole program waiting. A World that decodes on its IO thread and hands the program rows instead of text has a request type to match, and should; the language offers no way to parse in the background, because there is no background.

Two hundred charts rebuilding two hundred `Gui.Scene` slots on every push of a shared clock, because `Decode` stamps the result with the clock and the stamp changed: `$Distinct` on the rows, not on the whole `QueryState`, or the stamp out of the equality. Derived dynamics do not dedupe and the program has to say where equality matters.

Authentication is a header, and the right place for it is an interceptor around the fetches that rewrites the request with the current token from a state element, which is the path-rewriting sandbox with a different `match` arm. Token refresh on a `401` is a response the interceptor matches, a request to the token endpoint, and a replay of the original, all later, through the ordinary rules; it is a page of Prism and the page is where the bugs in every auth layer have lived.

A websocket for live updates is the World's component, and under it `Query.Result` is native state. The poller is the thing you write when the backend is a plain HTTP API, and the chapter's point is that it is written once, outside the charts, and deleted when the backend grows up.
## Where Prism is the wrong language

Eight programs, and in every one of them the same seam: the part that is "state, things happen, respond" was shorter and safer than you have ever written it, and the part that was a computation was called out to, awkward, or sent elsewhere. The claim of the language is narrow and this section draws its edge, by kind of program, with what a Prism program does with each instead of being it. None of this is a weakness to be fixed later; it is the price of a language whose only subject is time, and a language that tried to be good at these too would stop being good at the thing it is for.

### Batch and throughput work

A compiler, a numerical solver, a log cruncher, a video encoder. There is no time in these: the input is all there at the start, the output is all there at the end, and the only question is how fast. Every mechanism Prism has is overhead here. An occurrence is a transaction with buffered commits, which is pointless when nothing is observing; a dynamic is a value with a change event nobody subscribes to; the dependency graph is built once and used once. The build system of this chapter said this plainly: the compile is a process, the Prism program is the thing that decides when to run it and what its exit means.

What a Prism program does with batch work: it fires it as a request on a process or a native component and receives the result as a response, or it calls a plain function that is native. `Shape` for text, `Rasterise` for a draw list, `ParseRows` for a body, `Evaluate` for a spreadsheet: each is a plain function from plain data to plain data with no occurrence between asking and knowing, and that is the foreign function interface, which is also the whole of it. A plain function has no way to touch time and so needs no rules about it; the program is still deterministic, still a transcript, and the function is a black box the transcript records the results of.

The cost is latency inside the occurrence: a plain function runs on the runtime's thread, between the inputs arriving and the holds committing, and a slow one stalls every input behind it. The dashboard's ten-megabyte parse and the game's fifty thousand entities were the examples. A computation that takes longer than the program can afford to pause belongs behind a request, in a process or a World thread, with "not yet" in the response type and a turn of latency.

### Sequential algorithms

Anything naturally written as a loop with a cursor: a parser over a token stream, a graph search, a sort you wanted to write yourself, a state machine that walks through a protocol one step at a time with no outside input between steps. These can be written in Prism and the ways of writing them are instructive, because each is wrong in a way that shows what occurrences are for.

The first way is as a recursive plain function, which is fine and is not Prism; it is the functional language Prism has as its value layer, and the closure compiler's recursion depth guard is the only thing the implementation adds. A backtracking search with a mutable visited set wants an `update` block, which is future work, and until then it is the same ceremony as in any pure language. Use the plain function when the algorithm fits in one; it has no time in it and should not pretend to.

The second way is as a chain of `Defer`s: each step of the loop an occurrence, the cursor a hold, the loop body the hold's change event lifted over the `Defer`'s answer. It runs, one occurrence per step, inside one turn, and it is wrong twice over. Each `Defer` slot is answered at most once per turn, so the loop needs a fresh `Defer` per step, which means a `dyn` rebuilding per step, which the world core treats as the same `Defer` reborn, so the second firing in the turn exceeds the once-per-turn bound and is an error. That rule exists to stop exactly this, because a loop through the turn mechanism is a loop that freezes the program's output while it spins, and the right place for an iteration to a fixed point is a plain function.

The third way is as a chain of `Time.After`s, one step per turn, so that the World gets to run in between. That is not wrong; it is an animation, or a protocol handshake, or a retry schedule, and those are things that happen over time and belong here. The test is whether a step waits for something outside the program. A handshake waits for the peer; an animation waits for the frame; a sort waits for nothing, and a sort written this way is a sort that takes a frame per comparison.

### Data-parallel pipelines

Map a function over a billion elements, reduce, repeat. The parallelism Prism has is between occurrences and across heights within one, and neither is across the elements of one big array. A `for` over a map with a million keys builds a million nodes, and the runtime visits each one per occurrence that touches it; a lifted `MapValues` over a million-entry dynamic recomputes the map in one closure on one thread. The game's bullets and the chat server's fan-out were the small versions of this. The place for data parallelism is a native function that uses the machine's cores, or a World component that runs the pipeline and offers its progress as a state element; the Prism program is the thing that knows when the pipeline should start and what to show while it runs.

### Programs that are one pure function

A request-response handler with no state between requests: a function from an HTTP request to an HTTP response, a lambda in a cloud, a stateless RPC. The Worlds chapter shows that a WASI handler can be a World whose life is one turn, and it can, and the exercise shows why it should not be: the program builds `@main`, receives one response, fires one request, and is dropped, and every mechanism of the language ran once to compute what a function would have computed with none of them. If there is no state between requests, there is no "what can change this," and the question Prism answers has no subject. A language that has only the request is the right one, and if the handler grows state (a session, a cache, a rate limit), it has grown into the first bucket and the chat server is the model.

### Hard real time

The control loop said this plainly. Prism gives a deterministic amount of work per occurrence and no bound on how long it takes; a `dyn` rebuild in the tick path is a build of unknown size, an occurrence runs on whatever thread the World chose at whatever priority the platform gave it, and the runtime has no notion of a deadline. A controller with a deadline measured in milliseconds runs under a World that pins the loop to a real-time thread and measures the occurrence, and keeps structural changes out of the tick. A controller with a deadline measured in microseconds is firmware, and Prism is the layer that sets its targets.

### Cryptography and secrets

TLS is on the World's side of the boundary, and so is anything else that wants randomness, constant-time operations, or a secret that should not sit in a value. Prism has no effects, and randomness is an effect; a random number as a state element the World pushes is a value in a hold, visible to the transcript, which is the wrong place for key material and the right place for a dice roll in a game. A cipher is a computation. The program sees `tls://` in an address and `Disconnected "certificate .."` in a link type and nothing else, and that is the correct amount of cryptography for a program whose subject is time.

### Large mutable structures

A rope, a matrix, a spatial index, a B-tree. The value layer is immutable and eager, and functional update of a large structure is the one place a pure language is worse to write than an imperative one, which is why `update` blocks and `&` references are the first item of future work. Until they exist, a large structure that changes a little per occurrence is either a persistent data structure the prelude provides (maps already are) or native code the program calls with the old value and gets the new value back. The editor's text, the controller's covariance and the game's entities were the three examples, and in each the answer was the same: the structure is a plain value from the program's side, whatever it is inside.

### Programs whose structure is a call stack

Recursive descent, backtracking, coroutines, anything where "where am I" is a position in code rather than a value in state. Prism has no call stack at occurrence time; a widget block is built once, and what runs afterwards is a graph. An algorithm whose natural state is a stack frame has to make the stack a value, which is the continuation-passing transformation done by hand, and the result is correct and unreadable. Write it as a plain function, where the value layer's ordinary recursion gives the stack back.

### The first bucket

Interactive interfaces, market data and order flow, control, stateful services, incremental computation over things that arrive at different times, simulations and games, shells and compositors. Each of the eight deep dives found the same structure under its problem: a model, a closed set of things that can happen to it, and a boundary where the outside sends and receives plain data. In each, the hard moment was two of those things in one instant, and the answer was a line of the program. And in each, the parts that were not that structure were sent out through a plain function or a request, which is the foreign function interface and the only one the language needs.

The industry-changing claim is that the first bucket is most of the programs that get written, that the tools for it are bad in a way that is structural, and that a language with time in its types fixes the structural problem. The second bucket is where the claim stops, and the right thing to do at its edge is to call out, cleanly, with "not yet" in the type and the transcript still true. A Prism program is a thin layer of time over whatever computations it needs, and the thin layer is where the bugs were.
