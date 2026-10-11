# Chapter 2 — Worked examples

Fourteen programs, from a counter to a shell that hosts other programs, each one followed by what happens occurrence by occurrence: what the World sends in, what fires and in what order, what commits, and what the element list looks like afterwards. The programs are the language; the walkthroughs are the laws applied to them. If you have read the specification once and want to know whether you understood it, read the programs, predict the walkthroughs, and check. If you are coming to the language cold, the walkthroughs are where every rule in the later chapters is first seen doing work, and each example ends by saying which afternoon it prevents: the checkbox that un-checks itself, the fetch on mount that fires twice, the native text box that loses its cursor, the click that lands on a button that has already been replaced.

## Conventions used in every walkthrough

Occurrences are numbered from zero. Occurrence 0 is the build of the program; nothing is delivered to it. From then on every occurrence is opened by the World with a set of inputs, each one addressed to a slot by its path, all simultaneous. A walkthrough says, for each occurrence: the inputs; the events that fire and the derived dynamics that change, in dependency order, which is the only order the language promises; the holds that commit at the end; and the element list the World reads afterwards.

Paths are opaque to a World and the chapter needs a way to write them down, so here is one. A path is the chain of bind positions from the root, written with `/`: the first widget bound in `@main` is `0`, the second is `1`; a `rebuild` adds its generation as `#n`, starting at `#0` for the first build; a `for` adds its key as `[k]`. The Increment button of the counter below ends up at `0/#0/0`: the first bind in `@counter`, generation zero of the `rebuild` inside `@Button`, the first slot in that generation. Holds and `let`s do not take a bind position; only widgets do, and `@started` is a widget, so it takes one even though it creates no slot. A widget block bound at position `p` numbers its own binds under `p`, which is why `@TextInput`'s two parts below are `3/0` and `3/1/...`; a `rebuild`, a `for` or an `@Element` bound at `p` is one segment, `p/#0/0` or `p/[k]/...` or `p`, not two, because the position is the thing itself and not a block around it. So a widget function's insides can never renumber its caller's slots, and an interceptor's routing by path prefix is stable across any change inside a child. A slot created by a widget that an interceptor's supplier built is written `s/n`, with `n` counting the supplier-built widgets in creation order in the interceptor's widget, followed by the supplied widget's own segments; its position in the widget is the runtime's business, and only its existence is a law.

Element lists are written one slot per line as `path: element`. A tag with no `responds` and no `state` is a presence element, there to be seen and nothing else. A slot whose request fired in the occurrence has `requested:` after it, then the fired values in firing order, each written as the Prism literal it is and separated by `; `, with no brackets of the transcript's own: `requested: Read`, `requested: [[Send "ok", Close]]`, `requested: "a"; "b"`. An array request is already an array literal, so wrapping the list in brackets would stack three of them; the separator keeps every bracket in the line meaning an array.

The chapter assumes a small prelude of plain functions. None of them does anything with time; they are listed so that nothing in a program is a mystery:

```prism
!Filter: fn !?a -> !a                                   # fires x when the input fires Some x
!Union:  fn !(k ==> a), !(k ==> a) -> !(k ==> a)        # the union of the maps that fired; the left wins a shared key
!Leftmost: fn [!a] -> !a
First:  fn ((a, b)) -> a
Second: fn ((a, b)) -> b
Len:    fn Text -> Nat
Prefix: fn Text, Nat -> Text                            # the first n characters
Suffix: fn Text, Nat -> Text                            # everything from character n on
Concat: fn [Text] -> Text
JoinText: fn Text, [Text] -> Text                       # separator first
Count:  fn Text, Char -> Nat
StartsWith: fn Text, Text -> Bool
ToText: fn a -> Text                                    # a: a primitive with a canonical rendering
Append: fn [a], a -> [a]
Insert: fn k ==> v, k, v -> k ==> v
Remove: fn k ==> v, k -> k ==> v
Size:   fn (k ==> v) -> Nat
Keys:   fn (k ==> v) -> [k]
Filter: fn k ==> v, (fn v -> Bool) -> k ==> v
MapValues: fn k ==> v, (fn v -> w) -> k ==> w
Fold:   fn k ==> v, a, (fn a, k, v -> a) -> a           # in key order
```

Every operation over these lifts by the rule of the sigils: a plain function applied to a dynamic is a dynamic, applied to a dynamic and an event is an event with the dynamic read through a tag, applied to two events is a compile error. Comparisons are the infix operators `==`, `!=`, `<`, `<=`, `>` and `>=`, through the equality and ordering traits, and lift the same way. Boolean logic is the keyword operators `not`, `and` and `or`, looser than comparison and tighter than `on` and `|`, and they lift the same way too. So `Append $log, !line` is an event of arrays, and `(Insert $items, $next, todo) on !add` is an event that fires with the click and reads both dynamics as they stood before it.

### Shared declarations

The GUI vocabulary in this chapter is deliberately small and deliberately not the language's. `Element` is the extensible variant every element is a value of; the tags under `Ui` are the GUI library's own, which the Test World accepts directly and a GUI World sees only through the library's interceptors, and later examples add tags as they need them, each declared before its first use.

```prism
type Element = choice ..

type Click = click

tag Ui.Label of Element:
    text: Text

tag Ui.Button of Element:
    label: Text
    responds: Click
```

A label has a text and does nothing; a button has a label and responds with a click. Both have plain fields, fixed for the life of the slot, which is the rule for every element: a label whose text changes is, to the World, a new label at the same spot. The two library widgets below are how a program with a dynamic label gets one anyway. The dynamic lifts the element into a `rebuild`, which builds a fresh slot per change, and the click a button returns is a `!Switch` over the generations:

```prism
@Label: fn $Text -> @unit = \@Element Ui.Label(text=$_)

@Button: fn $Text -> @!Click = \@Element Ui.Button(label=$_)
```

A dynamic where a plain field goes lifts the element into a `rebuild`: a fresh slot per change, and for a button a `!Switch` over the generations' clicks. Written out, that is what each of these means:

```prism
@Label: fn $Text -> @unit = \@widget:
    rebuild $_:
        @Element Ui.Label(text=_)
    @ unit

@Button: fn $Text -> @!Click = \@widget:
    !!clicks <- rebuild $_:
        @Element Ui.Button(label=_)
    @ !Switch !!clicks
```

Inside the `rebuild` block `_` is the label as a plain value, because inside one build it never changes; a change rebuilds the block. The expansions are here so that nothing in the walkthroughs is hidden, and the library uses the one-line forms, which matters for paths: the shorthand is one bind and one position, so a `@Button` at position `0` prints `0/#0/0`, while the written-out block, bound yourself, would be a widget block at `0` with a `rebuild` as its first bind, `0/0/#0/0`. Example 4 is the same construction unrolled with a cycle through it.

## 1. The counter

```prism
@counter:
    !increment <- @Button $"Increment"
    !decrement <- @Button $"Decrement"
    let !delta = -1 on !decrement | +1 on !increment
    $count <- 0 then $count + !delta
    @Label "Count: {$count}"
```

You have written this program in every framework you have used and in each one the interesting question was never the increment; it was what happens when two things land at once. Here that question has an answer you can read off the third line, and it is the only thing the program has to say about it.

Nothing in the five lines says which numeric type `$count` is. `-1` is a signed literal, so `!delta` is an event of `Int`, so `$count + !delta` is `Int` addition, so the `0` that starts the hold is an `Int`. Had the deltas been `+1` and `+2`, nothing would pin the type, and an integer literal nothing constrains is an `Int`, so the program would mean the same thing; a `Nat` counter says so with an annotation.

**Occurrence 0 (build).** The World calls the run call with no inputs. `@counter` is built. The first `@Button` builds its `rebuild`, whose block builds generation `#0` with one slot, `Ui.Button(label="Increment")` at `0/#0/0`; the `rebuild`'s event `!!clicks` fires in this occurrence with the slot's click event, and `!Switch` records it as the inner event to listen to from occurrence 1. The second button does the same at `1/#0/0`. `$count` has its initial value, `0`. `"Count: {$count}"` is a derived dynamic whose value is `"Count: 0"`, and `@Label` builds generation `#0` of its `rebuild` with `Ui.Label(text="Count: 0")` at `2/#0/0`. Nothing fires at `!increment`, `!decrement` or `!delta`: a `!Switch` is subscribed to nothing in the occurrence it is given its first inner event. The occurrence commits: `$count` is `0`, and the two `!Switch` holds now point at their slots' events.

```
0/#0/0: Ui.Button(label="Increment")
1/#0/0: Ui.Button(label="Decrement")
2/#0/0: Ui.Label(text="Count: 0")
```

**Occurrence 1.** The World sends `click` to `0/#0/0`. Routing looks the path up in the list as the occurrence opened and finds the Increment slot; its click event fires. `!Switch` is listening to exactly that event, so `!increment` fires. `!delta` is a merge; its left side is quiet and its right side fires with `+1`, so `!delta` fires with `+1`. `$count + !delta` is an operation over a dynamic and an event: it fires with `0 + 1`, where `0` is `$count` read through a tag, the value before this occurrence. That is the hold's change event, so the derived `"Count: {$count}"` changes promptly to `"Count: 1"`, which is the dynamic the label's `rebuild` watches, so the `rebuild` builds anew: generation `#0` of the label is torn down at commit and generation `#1` is built now, with a new slot. Commit: `$count` is `1`.

```
0/#0/0: Ui.Button(label="Increment")
1/#0/0: Ui.Button(label="Decrement")
2/#1/0: Ui.Label(text="Count: 1")
```

The World diffs the two lists, sees a label gone and a label arrived at the same tree position, and redraws. Nothing told it to. The list is a description of what the program wants on the screen; reading it is the World's whole job.

**Occurrence 2.** The World sends `click` to `0/#0/0` and `click` to `1/#0/0` in one occurrence, because the user mashed both and the World chose to deliver them together. Both slots' click events fire; both `!Switch`es deliver; `!increment` and `!decrement` both fire. Now `!delta` has both sides firing, and `|` takes the left: `!delta` fires with `-1`. `$count + !delta` fires with `1 + -1 = 0`. The label rebuilds to generation `#2`. Commit: `$count` is `0`.

```
0/#0/0: Ui.Button(label="Increment")
1/#0/0: Ui.Button(label="Decrement")
2/#2/0: Ui.Label(text="Count: 0")
```

Between occurrences 1 and 2 the World did what Worlds do: it read the list, drew it, and waited. Nothing in the program ran. There is no render loop inside a Prism program and no idle callback; an occurrence happens because the World opened one, and between them the program is a data structure. That is why the same program can be driven by a test World with no screen (example 13), and why the World can batch two clicks into one occurrence or deliver them as two, and the program's text says what each choice means.

Suppose the program wanted a count per button as well as the total. The hold is the only state, so there would be two more holds, `$ups <- 0 then $ups + 1 on !increment` and `$downs <- 0 then $downs + 1 on !decrement`, each listing its one event, and in occurrence 2 both would move while `$count` moved by the leftmost. Nothing about the first hold changes, and nothing anywhere else has to know the new ones exist.

The tag law decides that `$count` on the right of `then` means the count *before* the click, which is what turns a line that looks like an equation into a definition: there is no "which version of `$count` did the handler see" because there is only one version at an event. The merge decides simultaneity: two clicks in one moment are one `!delta`, the leftmost, and if you want the other answer you write `+1 on !increment | -1 on !decrement` and the program says so. And the hold's definition is the complete list of what can move the count. There is no setter, no second file, no reset handler registered somewhere else that wins or loses depending on registration order. That is the goal of one way to change anything, written next to the thing, in five lines.

## 2. A text input with a cursor

```prism
type Key = choice:
    Char Char
    Left
    Right
    Backspace

tag Ui.Focusable of Element:
    name: Text
    responds: Key

type Edit = record:
    text: Text
    cursor: Nat

let Apply: fn Edit, Key -> Edit = \|e, k| match k:
    Char c:
        Edit (Concat [[(Prefix e.text, e.cursor), (ToText c), (Suffix e.text, e.cursor)]]), e.cursor + 1
    Left:
        if (e.cursor == 0): e
        else: Edit e.text, e.cursor - 1
    Right:
        if (e.cursor == Len e.text): e
        else: Edit e.text, e.cursor + 1
    Backspace:
        if (e.cursor == 0): e
        else: Edit (Concat [[(Prefix e.text, e.cursor - 1), (Suffix e.text, e.cursor)]]), e.cursor - 1

let Render: fn Edit -> Text = \|e|
    Concat [[(Prefix e.text, e.cursor), "|", (Suffix e.text, e.cursor)]]

@TextInput: fn Text, Text -> @$Text = \|name, initial| @widget:
    !key <- @Element Ui.Focusable(name=name)
    $edit <- Edit initial, (Len initial) then Apply $edit, !key
    @Label (Render $edit)
    @ $edit.text
```

Everyone who has wrapped a native text box has lost the cursor. The box is a widget the toolkit owns; its value is a property you set; you set it from your model on every change; and the toolkit, seeing a new value, puts the cursor at the end. The fix is some version of "only set it if it differs", and that fix has a race in it. Here the text box is not native and the cursor is not the toolkit's. It is a hold, and it lives outside any `rebuild`.

The element that receives keys is `Ui.Focusable(name=name)`: a named region that can have focus and responds with keys. It has no dynamic fields, so its slot is created once and lives as long as the widget, and whoever delivers keystrokes has one stable path to send them to. Everything that changes per keystroke, the text and the cursor, is in `$edit`, and `Apply` is a plain function from the old state and a key to the new state. The label is a presence element: a fresh `Ui.Label` slot per keystroke, which is right, because nothing is in flight on a label and whatever draws the list diffs it.

**Occurrence 0 (build).** `@TextInput "input", "ab"` is built. The focusable's slot is at `0`; `$edit` is `Edit "ab", 2`; `Render $edit` is `"ab|"`; the label's `rebuild` builds generation `#0` at `1/#0/0`. The result dynamic `$edit.text` is a derived dynamic with value `"ab"`.

```
0:      Ui.Focusable(name="input")
1/#0/0: Ui.Label(text="ab|")
```

**Occurrence 1.** The user presses Left. The World sends `Left` to `0`. `!key` fires with `Left`; `Apply $edit, !key` fires with `Apply (Edit "ab", 2), Left`, with `$edit` read through a tag, so the cursor moves from 2 to 1. The hold's change event is that firing; `Render $edit` changes promptly to `"a|b"`; the label rebuilds to `#1`. Commit: `$edit` is `Edit "ab", 1`. The result dynamic `$edit.text` did not change in this occurrence, because `"ab"` is `"ab"`; a derived dynamic fires its change event when an operand changes, and the operand here is the record, so the change event fires with `"ab"` again. Whoever holds the result sees a change event with an equal value, which is fine: a dynamic's change event is allowed to fire with the value it already had, and nothing downstream can tell the difference except a tag, which gives the same answer either way.

```
0:      Ui.Focusable(name="input")
1/#1/0: Ui.Label(text="a|b")
```

**Occurrence 2.** The user types `x`. The World sends `Char 'x'` to `0`. `Apply` fires with `Edit "axb", 2`. The label rebuilds to `#2` with `"ax|b"`. Commit: `$edit` is `Edit "axb", 2`. The result dynamic changes to `"axb"`.

```
0:      Ui.Focusable(name="input")
1/#2/0: Ui.Label(text="ax|b")
```

**Occurrence 3.** The World sends `Backspace` to `0`. `Apply` deletes the character before the cursor: `Edit "ab", 1`. The label is `"a|b"` again at generation `#3`.

Nothing in these four occurrences involved a World knowing what a cursor is. Under a GUI World the library's window interceptor turns the label into a `Draw.Text` in the window's `Gui.Scene` and delivers the window's key events to the focusable's slot; the World sees draw commands and keys, never a cursor. Under the Test World a script sends `Left` to the focusable directly. If a World wraps a native toolkit and wants to use the toolkit's own text box, it can offer a `Gtk.Entry` tag and a different `@TextInput` targets it; but the one above runs under any World that can draw and deliver a key, and its cursor cannot be lost because there is no second copy of it anywhere. This is the goal that the language is enough: text editing is not a native capability, it is a hold and a function.

Focus is not in this program either. A form with three inputs has three `Ui.Focusable` slots, and something has to decide which one a keystroke is for. That is the library's window interceptor, not the World. It holds the focused name in a hold moved by Tab and by clicks (hit-tested against the layout the library computed), announces each focusable to the World as a `Gui.Focusable(name=...)`, so that a World with a notion of focus of its own, such as an input-method layer, knows the names, and delivers each key from `Gui.Window` to the slot whose name is focused. Under that interceptor the form above is unchanged. Focus is by name, which is why `@TextInput` takes one: two inputs in one form with the same name would be one focus target with two slots.

One thing the program does *not* do is react to its own label. The label is a fresh slot per keystroke, and if slots appearing were events, the program would have a loop in it. They are not; the diode holds: inputs cause list changes and list changes never cause inputs.

## 3. A todo list

```prism
type Todo = record:
    label: Text
    done: Bool

type Change = choice:
    Add Text
    Toggle
    Delete

let Step: fn Nat ==> Todo, Nat, Change -> Nat ==> Todo = \|items, id, change| match change:
    Add label: Insert items, id, (Todo label, False)
    Toggle: match items[id]:
        Some t: Insert items, id, (Todo t.label, not t.done)
        Nil: items
    Delete: Remove items, id

let StepAll: fn Nat ==> Todo, Nat ==> Change -> Nat ==> Todo = \|items, changes|
    Fold changes, items, \|acc, id, c| Step acc, id, c

let Describe: fn Todo, Bool -> Text = \|t, starred|
    Concat [[(if t.done: "[x] " else: "[ ] "), t.label, (if starred: " *" else: "")]]

@todos:
    !add <- @Button $"Add"
    $next <- 0 then $next + 1 on !add
    let !adds = {$next: Add "item {$next}"} on !add
    !rows <- for $items:
        !toggle <- @Button $"Toggle"
        !delete <- @Button $"Delete"
        !star <- @Button $"Star"
        $starred <- False then not $starred on !star
        @Label (Describe $_, $starred)
        @ Toggle on !toggle | Delete on !delete
    let !changes = !Union !adds, !rows
    $items <- {} then StepAll $items, !changes
```

The list is a map from an allocated id to a todo, not an array. The id is the identity: it decides which row survives a change, which paths stay stable, and which `$starred` keeps its value when the row above it is deleted. An array keyed by position would rebuild every row after an insertion at the front and reset every star; that is why `for` over an array is second class and why the ids are allocated from `$next` rather than taken from a position.

`$starred` is local to the row on purpose. It is not part of the model, and that is the demonstration: state inside a `for` block belongs to the key's widget and lives exactly as long as the key does, through every change to its neighbours.

Each row's block returns an event of `Change`, and `for` consolidates those into `!rows : !(Nat ==> Change)`, a map of the changes that fired in an occurrence, keyed by the row's id. The Add button is outside the `for`, so its change is made into a one-entry map by hand, `{$next: Add "item 3"} on !add`, reading `$next` through a tag. `!Union` merges the two maps when both fire. Had the program written `!adds | !rows` instead, an add and a toggle in one occurrence would lose the toggle, and that would be a bug the program chose; the union is the program saying that both count.

**Occurrence 0 (build).** `$items` is `{}`; `for $items` builds no rows; `$next` is `0`. The Add button is at `0/#0/0`.

```
0/#0/0: Ui.Button(label="Add")
```

**Occurrence 1.** `click` to `0/#0/0`. `!add` fires. Two things read `$next` through a tag, and both read `0`: `$next + 1 on !add` fires with `1`, and `!adds` fires with `{0: Add "item 0"}`. `!Union` fires with the same map, since `!rows` is quiet. `StepAll $items, !changes` fires with `{0: Todo "item 0", False}`, reading `$items` as `{}`. That is `$items`'s change event, so `for $items` sees a key appear in this occurrence and builds row `[0]` now: three buttons at `1/[0]/0/#0/0`, `1/[0]/1/#0/0` and `1/[0]/2/#0/0`, a hold `$starred` at `False`, and a label at `1/[0]/3/#0/0` with `"[ ] item 0"`. Commit: `$next` is `1`, `$items` has one entry.

```
0/#0/0:        Ui.Button(label="Add")
1/[0]/0/#0/0:  Ui.Button(label="Toggle")
1/[0]/1/#0/0:  Ui.Button(label="Delete")
1/[0]/2/#0/0:  Ui.Button(label="Star")
1/[0]/3/#0/0:  Ui.Label(text="[ ] item 0")
```

**Occurrence 2.** `click` to `0/#0/0` again. The same steps with `$next` read as `1`; row `[1]` is built. Keys are ordered, so the list has row `[0]` before row `[1]`.

**Occurrence 3.** `click` to `1/[0]/2/#0/0`, the Star button of row 0. Inside that row's block, `!star` fires, `not $starred on !star` fires with `True`, and `Describe $_, $starred` changes to `"[ ] item 0 *"`, rebuilding the row's label to `#1`. The row's result event is quiet, so `!rows` is quiet, `!changes` is quiet, and `$items` does not change. Commit: row 0's `$starred` is `True`.

**Occurrence 4.** `click` to `1/[0]/0/#0/0` and `click` to `0/#0/0` in one occurrence: toggle row 0 and add at once. Row 0's block fires `Toggle`; `!rows` fires with `{0: Toggle}`. `!adds` fires with `{2: Add "item 2"}`. `!Union` fires with `{0: Toggle, 2: Add "item 2"}`. `StepAll` folds in key order: row 0 becomes done, row 2 is inserted. `for` sees key 2 appear and builds its block; row 0's `$_` changes, which is the row's entry *as a dynamic*, so its label rebuilds to `"[x] item 0 *"` without the row being rebuilt, and `$starred` stays `True`. Commit.

```
0/#0/0:        Ui.Button(label="Add")
1/[0]/0/#0/0:  Ui.Button(label="Toggle")
1/[0]/1/#0/0:  Ui.Button(label="Delete")
1/[0]/2/#0/0:  Ui.Button(label="Star")
1/[0]/3/#2/0:  Ui.Label(text="[x] item 0 *")
1/[1]/...      (row 1, unchanged)
1/[2]/0/#0/0:  Ui.Button(label="Toggle")
1/[2]/1/#0/0:  Ui.Button(label="Delete")
1/[2]/2/#0/0:  Ui.Button(label="Star")
1/[2]/3/#0/0:  Ui.Label(text="[ ] item 2")
```

**Occurrence 5.** `click` to `1/[1]/1/#0/0`, Delete on row 1. `!rows` fires with `{1: Delete}`; `$items` loses key 1; `for` tears row `[1]` down at commit. Row 0's star is still set and row 2's paths are what they were. Nothing was renumbered.

The program is a cycle: `$items` is defined from `!changes`, `!changes` comes from `!rows`, and `!rows` is the result of `for $items`. It is legal because it passes through delayed edges. `StepAll $items, !changes` reads `$items` through a tag, and the consolidated event of a `for` merges the blocks that existed as the occurrence opened, the way `!Switch` picks its inner event: a block born in an occurrence has its own events heard from the next one. Nothing is lost by that, because a row's events are clicks, and a click cannot arrive at a slot in the occurrence the slot was created.

What this prevents: the afternoon where deleting item 3 cleared the expanded state of item 4, because the list was an array and the framework matched rows by position. Keys are identities, `for` keeps a widget per key, and the state that belongs to a key is torn down with it and only with it.

## 4. A button that relabels itself

```prism
@clicker:
    !!clicks <- rebuild $label:
        @Element Ui.Button(label=_)
    let !click = !Switch !!clicks
    !reset <- @Button $"Reset"
    $count <- 0 then $count + 1 on !click | 0 on !reset
    let $label = "Clicked {$count} times"
    @Label $label
```

This is `@Button` unrolled, with the label depending on the click; it illustrates the fresh-slot rule and is not how a program normally labels a button, since the library's buttons take a dynamic label (`@Button $label`) and the fresh slot per change stays inside the library's interceptor. The `rebuild` watches `$label`; every change to the label tears the button's slot down and builds a new one; `!!clicks` fires in every build with the new slot's click event; and `!Switch` turns that into the one `!click` the rest of the program uses. The cycle is the thing to look at: `$count` depends on `!click`, `!click` on `!!clicks`, `!!clicks` on the `rebuild`, the `rebuild` on `$label`, and `$label` on `$count`. The `rebuild` is not a delay: its event fires in the occurrence of the rebuild. `!Switch` makes the cycle legal: it chooses its inner event from before the occurrence, and the tag on `$count` in `$count + 1` does the rest.

**Occurrence 0 (build).** `$count` is `0`; `$label` is `"Clicked 0 times"`; the `rebuild` builds generation `#0` with a button at `0/#0/0`; `!!clicks` fires with its click event; `!Switch` has it from occurrence 1. Reset is at `1/#0/0`; the label at `2/#0/0`.

```
0/#0/0: Ui.Button(label="Clicked 0 times")
1/#0/0: Ui.Button(label="Reset")
2/#0/0: Ui.Label(text="Clicked 0 times")
```

**Occurrence 1.** `click` to `0/#0/0`. The generation-`#0` slot's event fires; `!Switch` delivers it as `!click`; `$count + 1 on !click` fires with `1`; `$label` changes promptly to `"Clicked 1 times"`; the `rebuild` builds anew: generation `#1` is built with a slot at `0/#1/0`, and `!!clicks` fires with the new slot's click event. `!Switch` does not deliver anything from the new slot in this occurrence, and could not anyway, since no input was addressed to a slot that did not exist as the occurrence opened. At commit `!Switch` adopts the new inner event, generation `#0` is dropped, and `$count` is `1`. The label rebuilds too.

```
0/#1/0: Ui.Button(label="Clicked 1 times")
1/#0/0: Ui.Button(label="Reset")
2/#1/0: Ui.Label(text="Clicked 1 times")
```

**Occurrence 2.** `click` to `0/#1/0` and `click` to `1/#0/0` together: the user clicked the button and Reset in the same frame. Both slots' events fire. `$count + 1 on !click | 0 on !reset` takes the left: `$count` becomes `2`, and the reset is lost. The program said so; if reset should win, the merge is written the other way round. `$label` changes, the `rebuild` builds anew to `#2`.

**Occurrence 3.** Suppose the World has a click for `0/#1/0` queued from before it read the list after occurrence 2. It must not send it: `0/#1/0` is not in the list it last received, and sending to a path not in the opened list is an error of kind `UnknownPath`. A conforming World discards it. The click was for a button that no longer exists.

**Occurrence 4.** `click` to `1/#0/0` alone: Reset. `0 on !reset` fires; the merge's left side is quiet; `$count` becomes `0`; `$label` changes to `"Clicked 0 times"`; the `rebuild` builds anew to generation `#3` with a button whose label is the same text generation `#0` had. It is a different slot with a different path. A World that cached something about the first button by its path would find the cache useless, which is right: it is a new button. A World that keys native widgets by tree position and tag can reuse what it had, and the program cannot tell.

The question this example answers is what a click means in the occurrence the button is replaced. Routing says: the click is delivered to the slot at that path in the list as the occurrence opened, which is the old button, under the old label. The old widget keeps propagating until commit, so its click reaches `!Switch`, which is still listening to it. Then the new button exists, and from the next occurrence its clicks are the ones heard. There is no moment in which the program has a button on screen that nothing is listening to, and no moment in which two buttons are listened to at once.

`!Switch` not being prompt costs nothing here. A freshly built button cannot be clicked in the occurrence it was built, because the World addresses only paths it has seen. The only thing a new generation could fire in its birth occurrence is something pure, like `@started`, and if a block wants to act on its own birth it uses `@started` inside the block, where no switch stands in the way. This is why the language can refuse prompt switching and lose nothing anyone wanted, while keeping every node's height fixed by its sources.

## 5. A file viewer

```prism
type FsRequest = choice Read | Write Text

type FsError = choice:
    NotFound
    Denied
    TooLarge
    Other Text

type FsResponse = choice:
    Contents Text
    Unavailable FsError

type Contents = choice:
    Unavailable FsError
    Known Text

tag Fs.File of Element:
    path: Text
    requests: !FsRequest
    responds: FsResponse

tag Fs.Contents of Element:
    path: Text
    state: Contents

let ShowResponse: fn ?FsResponse -> Text = \|r| match r:
    Nil: "(reading...)"
    Some (Contents t): t
    Some (Unavailable why): ShowError why

let ShowError: fn FsError -> Text = \|e| match e:
    NotFound: "(no such file)"
    Denied: "(permission denied)"
    TooLarge: "(too large)"
    Other t: "({t})"

let ShowContents: fn Contents -> Text = \|c| match c:
    Unavailable why: ShowError why
    Known t: t

@viewer:
    !started <- @started
    !motd <- @Element Fs.File(path="/etc/motd", requests=(Read on !started))
    $motd <- Nil then Some !motd
    @Label (ShowResponse $motd)

    $path <- @TextInput "path", "/etc/hostname"
    !$contents <- rebuild $path:
        @Element Fs.Contents(path=_)
    $$current <- $ (Unavailable NotFound) then !$contents
    @Label (ShowContents ($Join $$current))
```

Two ways to get a file, because there are two things you might mean. The top half wants the message of the day once, now, and is content to wait: it fires a `Read` request from `@started` and holds the response when it comes. The bottom half wants whatever file the user has typed the path of, as a value that is simply true: it asks for `Fs.Contents` state, which has a value in the occurrence it is created and is pushed when the file changes.

The top half is where "no request on creation" shows. Creating the `Fs.File` slot does nothing; the `Read` fires because `!started` fired, and `!started` fires exactly once, in the build occurrence, and it is pure. Compare the fetch-on-mount you have written, where every re-mount was a surprise network call and every test needed the mock before the first render. Here, if `@viewer` is rebuilt by some `rebuild` around it, the read happens again because `@started` fires again, and that is visible in the program's text rather than in a lifecycle table. Until the response arrives `$motd` is `Nil`, and its type says so: the `match` in `ShowResponse` has an arm for not-yet, and leaving it out is a compile error.

The bottom half is where dynamic fields show. `Fs.Contents(path=_)` has a plain path, but the `rebuild` over `$path` means a fresh slot per path: a new path is a new file, and the new slot's state is sampled in the occurrence of the change. The `rebuild`'s result is `!$Contents`, an event of dynamics, one per build; a hold with a constant initial dynamic and a `$Join` turn that into the current file's contents as one dynamic. The specification lets you write `@Element Fs.Contents(path=$path)` for exactly this, and this chapter spells it out once.

**Occurrence 0 (build).** `!started` fires. `Read on !started` fires, which is the `Fs.File` slot's request; the World will see it after the occurrence. `!motd` does not fire: nothing addressed to a slot arrives in the occurrence the slot was created. `$motd` is `Nil`, so the first label is `"(reading...)"`. `@TextInput` builds with name `"path"` and text `"/etc/hostname"`; the `rebuild` builds generation `#0` with an `Fs.Contents(path="/etc/hostname")` slot, and the runtime asks the World's oracle for that element's value as this occurrence opened. The World has never been asked about it; this is the first ask for a local file, so it reads the file once under its size cap, records the result in its snapshot, and answers `Known "box"`. `!$contents` fires with the new slot's dynamic; `$$current` changes to it in this same occurrence (a hold adopts a firing in its build occurrence); `$Join $$current` is `Known "box"`; the second label is `"box"`. Commit.

```
1:          Fs.File(path="/etc/motd")             requested: Read
2/#0/0:     Ui.Label(text="(reading...)")
3/0:        Ui.Focusable(name="path")
3/1/#0/0:   Ui.Label(text="/etc/hostname|")
4/#0/0:     Fs.Contents(path="/etc/hostname")
5/#0/0:     Ui.Label(text="box")
```

The World reads the list, sees a `Read` fired on slot `1`, and starts reading `/etc/motd`. It also sees an `Fs.Contents` slot for `/etc/hostname`, which it already holds, and starts watching that file.

**Occurrence 1.** The machine is renamed. The World's watch sees `/etc/hostname` change, so it has a new snapshot in which `Fs.Contents(path="/etc/hostname")` is `Known "crate"`, and the difference from the previous snapshot is one push. It opens an occurrence with that push and no responses. The slot at `4/#0/0` is identified for state by its element value, not its path, so the push reaches it: its dynamic changes to `Known "crate"`; `$Join $$current` changes promptly; the label at `5` rebuilds to `#1` with `"crate"`.

**Occurrence 2.** The motd read finishes. The World sends `Contents "Welcome"` to path `1`. `!motd` fires; `Some !motd` is `$motd`'s change event; the label at `2` rebuilds to `"Welcome"`. Commit: `$motd` is `Some (Contents "Welcome")`.

**Occurrence 3.** The user presses Backspace in the path input: `Backspace` to `3/0`. `$path` changes to `"/etc/hostnam"`; the `rebuild` builds anew: generation `#1` is built with `Fs.Contents(path="/etc/hostnam")`, and the oracle is asked for it. The World has never seen that path, reads it once under its cap, and answers `Unavailable NotFound`. `!$contents` fires with the new slot's dynamic; `$$current` changes; the join changes to `Unavailable NotFound`; the label rebuilds to `"(no such file)"`. At commit generation `#0` is dropped, and with it the only slot for `/etc/hostname`; the World stops watching that file.

```
1:          Fs.File(path="/etc/motd")
2/#2/0:     Ui.Label(text="Welcome")
3/0:        Ui.Focusable(name="path")
3/1/#1/0:   Ui.Label(text="/etc/hostnam|")
4/#1/0:     Fs.Contents(path="/etc/hostnam")
5/#2/0:     Ui.Label(text="(no such file)")
```

**Occurrence 4.** Nothing arrives: the World's snapshot for `/etc/hostnam` is `Unavailable NotFound` and stays that way, so there is no push, and no occurrence happens at all. If the file ever appeared, the push would be `Known`. The program showed the truth the whole time.

If two widgets in a program ask for `Fs.Contents(path="/etc/hostname")`, there is one state, supplied once, pushed to both, and both see the same value in every occurrence: that is the equality law across the boundary, and it is why state is identified by value and responses by path. A response is a conversation with one slot; a state is a fact about the world.

This is the "not yet is a type" goal in both of its forms. The request form has a `?` that the program must match on. The state form has `Unavailable` with a reason as a case of the state's type, so a file that cannot be had says why in the type, and in no case is there a placeholder, a `Pending`, a retry loop, or a value that is wrong for one occurrence.

## 6. A stopwatch

```prism
type Instant = Instant Int                 # milliseconds since the World's epoch

type Tick = tick

tag Clock of Element:
    state: Instant

tag Time.Every of Element:
    period: Int
    responds: Tick

type Run = choice:
    Stopped
    Running Instant                        # since when

type Watch = record:
    base: Int                              # milliseconds banked by earlier runs
    run: Run

type Action = choice Start | Stop | Reset

let Since: fn Run, Instant -> Int = \|run, now| match run:
    Stopped: 0
    Running from: now.value - from.value

let Step: fn Watch, Instant, Action -> Watch = \|w, now, a| match a:
    Start: match w.run:
        Running ignore: w
        Stopped: Watch w.base, (Running now)
    Stop: Watch (w.base + Since w.run, now), Stopped
    Reset: Watch 0, Stopped

let Total: fn Watch, Instant -> Int = \|w, now| w.base + Since w.run, now

@stopwatch:
    $clock <- @Element Clock
    @Element Time.Every(period=100)
    !start <- @Button $"Start"
    !stop <- @Button $"Stop"
    !reset <- @Button $"Reset"
    let !action = Start on !start | Stop on !stop | Reset on !reset
    $watch <- Watch 0, Stopped then Step $watch, $clock, !action
    @Label "{Total $watch, $clock} ms"
```

There is no `now()`. The clock is a state element like the contents of a file: it has a value in the occurrence the slot is created, and the World pushes a new one when it feels like it, which for a CLI or GUI World is at most once per turn. `Time.Every(period=100)` is bound as a statement and its tick event is discarded; the element exists to make the World come back every hundred milliseconds, and each time it comes back it opens an occurrence and pushes the clock. The label is derived from `$clock`, so it changes in that occurrence, and the stopwatch runs.

The state machine is one hold and one plain function. `Step` takes the watch, the clock, and what the user did, and returns the new watch; every input it needs is in its arguments, so it is testable without a World. `$clock` in `Step $watch, $clock, !action` is read through a tag: the time *before* the occurrence, which is the time the World pushed at the start of this turn or an earlier one. That is the right time to stamp a Start with, because it is the only one the program can know.

**Occurrence 0 (build).** `@Element Clock` creates a slot and asks the oracle for `Clock`'s value, say `Instant 5000`. `Time.Every` creates a slot. `$watch` is `Watch 0, Stopped`. `Total $watch, $clock` is `0`; the label is `"0 ms"`.

```
0:       Clock
1:       Time.Every(period=100)
2/#0/0:  Ui.Button(label="Start")
3/#0/0:  Ui.Button(label="Stop")
4/#0/0:  Ui.Button(label="Reset")
5/#0/0:  Ui.Label(text="0 ms")
```

**Occurrence 1.** A hundred milliseconds pass. The World opens an occurrence with a `tick` to `1` and a push for `Clock` of `Instant 5100`. The tick's event fires and nothing listens. `$clock` changes to `5100`; `Total $watch, $clock` is still `0`, since the watch is stopped; the derived label text changes, with the same value, and the label's `rebuild` builds anew to generation `#1` with the same text. A World that diffs lists sees no difference.

Rebuild a label every tick with the same text is wasteful and the program could avoid it, by deriving the label from `$watch` alone while stopped; it stays because correctness does not depend on it. The rebuild is a description, not a draw call.

**Occurrence 2.** `click` to `2/#0/0`, together with the tick and a push of `5200`: the World delivered the click and the tick in one occurrence because both were waiting. `!start` fires; `!action` fires with `Start`; `Step $watch, $clock, !action` reads `$watch` as `Watch 0, Stopped` and `$clock` as `5100`, the value before the occurrence, and fires with `Watch 0, (Running (Instant 5100))`. `$clock` changes to `5200`. The label's text is `Total (Watch 0, Running 5100), 5200`, which is `"100 ms"`. Commit.

The stamp is `5100` and not `5200`, by the tag law, and the push in the same occurrence did not change that. If a program wanted the stamp to be the pushed value, it could not have it, because the push is the occurrence's input and the tag reads before the occurrence; the hundred milliseconds between the two is the price of a readable rule. A World that cares about it pushes the clock in an occurrence of its own before delivering the click, and the program sees no difference in its text.

**Occurrences 3 through 11.** Nine ticks, nine pushes, the label counts up by hundreds.

**Occurrence 12.** `click` to `3/#0/0` with the tick and a push of `6200`. `!action` fires with `Stop`; `Step` reads `$clock` as `6100` and banks `1000` ms: `Watch 1000, Stopped`. The label shows `"1000 ms"` and stays there through the following ticks.

**Occurrence 20.** `click` to `4/#0/0`: Reset. `Watch 0, Stopped`; `"0 ms"`.

To test this program, a script sets the clock: `with Clock = Instant 5000`, a click, `push Clock = Instant 5100`, an assertion. Anyone who has tried to unit-test a debounce with a real timer knows what that is worth, and it falls out of the clock being a state element rather than a function. The goal served is "no clock": time is data the World supplies, the program cannot ask for a finer one, and a transcript of pushes replays it exactly.

## 7. A sandbox

```prism
tag Defer of Element:
    requests: !unit
    responds: unit

# Map a path under the sandbox root, or refuse it.
let Confine: fn Text, Text -> ?Text = \|root, p|
    if (StartsWith p, "/home/"): Some (Concat [[root, p]])
    else: Nil

@Forward: fn $(Path ==> Slot) -> @!(Path ==> Response) = \|$slots| @widget:
    for $slots:
        rebuild $_:
            ((!resp, ignore)) <- @Element _.element
            @ resp

@Sandbox: fn Text, @a -> @a = \|root, @child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ supply
    !down <- for $slots:
        rebuild $_:
            match _.element:
                Fs.File(path=p):
                    match Confine root, p:
                        Some q:
                            !resp <- @Element Fs.File(path=q, requests=_.!request)
                            @ resp
                        Nil:
                            !later <- @Element Defer(requests=(unit on _.!request))
                            @ Unavailable Denied on !later
                ignore:
                    ((!resp, ignore)) <- @Element _.element
                    @ resp
    let supply = \|e| match e:
        Fs.Contents(path=p): match Confine root, p:
            Some q: @Element Fs.Contents(path=q)
            Nil: @ $ Unavailable Denied
        ignore: @Element e
    @ result
```

You want to run a widget you did not write and be sure it cannot read your files. In most languages that is a question about the operating system, because the widget has the same ambient access you do. Here it is a question about the interceptor around it, because the widget has no ambient anything: every file it wants is an element in its list, and the only way that element reaches the World is if `@Sandbox` makes one.

`intercept @child, !down, $ supply` builds the child and captures its element list as `$slots`, a map from the child's paths to its slots. Nothing in `$slots` goes further out by itself. The two other arguments are the two directions down: `!down` is the responses the sandbox delivers to the child's slots, by path; `supply` is how the sandbox answers the child's state elements.

The response loop is a `for` over the child's slots. A slot never changes, so the `rebuild` inside builds once per slot and `_` is the plain slot. An `Fs.File` under `/home/` is forwarded with its path rewritten: the sandbox makes its *own* `Fs.File` element, with the child's request event wired straight into it, and the response the outer World sends back is this block's result, which `for` consolidates into `!(Path ==> Response)` keyed by the child's path, which is `!down`. Any other `Fs.File` is denied, and denial is the interesting case. The sandbox cannot answer a request in the occurrence it was made, and it cannot start an occurrence of its own. So it fires a `Defer`, which the World answers in a later occurrence of the same turn, and answers `Denied` then. The catch-all arm forwards everything else unchanged.

`supply` is a function from an element to a blueprint that returns the state's dynamic. When the child creates an `Fs.Contents` slot, the runtime applies `supply` to the element and builds the returned blueprint as a child of the sandbox's widget, in the same build pass, so the slot has its value in its creation occurrence. For a confined path the blueprint is the sandbox's own `Fs.Contents` element with the rewritten path, which asks the outer World; for a refused one it is the constant `$ Unavailable Denied`. A sandbox that wants to deny must say what denial is in the state's type, and `Unavailable FsError` is a case of it.

Take the child to be the viewer of example 5, with its motd path changed to `/home/me/motd` and its default input path to `/home/me/hostname`, under `@Sandbox "/jail", @viewer`.

**Occurrence 0 (build).** The sandbox's widget builds the child. The child's `Fs.File(path="/home/me/motd")` slot is created at child path `1`; the child's `Fs.Contents(path="/home/me/hostname")` slot at child path `4/#0/0`, and the runtime asks `supply` for it, which returns `@Element Fs.Contents(path="/jail/home/me/hostname")`, built in the sandbox's widget; the outer World's oracle answers `Unavailable NotFound`, since nothing is at that path under the jail. The child's `!started` fires, so its `Read` request fires. Now the sandbox's `for $slots` builds a block per child slot; the `Fs.File` block makes `Fs.File(path="/jail/home/me/motd", requests=...)` at a sandbox path, and since the child's request event fired in this occurrence and a widget built in an occurrence sees all of it, the rewritten element's request fires too. The child's other slots, the focusable and the labels, go through the catch-all arm and are re-emitted as elements of the sandbox. What the outer World sees:

```
1/[1]/#0/0:        Fs.File(path="/jail/home/me/motd")             requested: Read
1/[2/#0/0]/#0/0:   Ui.Label(text="(reading...)")
1/[3/0]/#0/0:      Ui.Focusable(name="path")
1/[3/1/#0/0]/#0/0: Ui.Label(text="/home/me/hostname|")
1/[5/#0/0]/#0/0:   Ui.Label(text="(no such file)")
s/0:               Fs.Contents(path="/jail/home/me/hostname")
```

The path prefix `1/[child path]/#0/0` is the sandbox's `for` (its second bind, after the intercept), keyed by the child's path, then the `rebuild` generation, then the slot. The child's own paths are not visible outside; the outer World addresses the sandbox's slots, and the sandbox routes inward.

**Occurrence 1.** The outer World answers the motd read: `Contents "hi"` to `1/[1]/#0/0`. That slot's response event fires; it is the `for` block's result for key `1`; `!down` fires with `{1: Contents "hi"}`; `intercept` routes it to the child's slot at path `1`, against the child's list as the occurrence opened. The child's `!motd` fires; its label rebuilds. The sandbox re-emits the new label in the same occurrence: the child's list changed, `$slots` changed, the `for` built a block for the new label's path and tore down the old one.

**Occurrence 2.** The user, inside the child, edits the path input to `/etc/passwd`. The child's `rebuild` builds anew its `Fs.Contents` slot with that path; the runtime applies `supply`, which returns `@ $ Unavailable Denied`, so the slot's dynamic is constantly `Unavailable Denied` and no element reaches the World. The old supplied widget is torn down at commit when its last slot goes.

**Occurrence 3.** Suppose the child, in some other part of it, builds `Fs.File(path="/etc/shadow", requests=(Read on !started))`. Its `@started` fires and so does its request, in the occurrence the slot is created. The sandbox's `for` builds a block for it: the `Nil` arm of `Confine`. `unit on _.!request` fires, so the `Defer` slot's request fires. Nothing else happens; from the child's view, it asked and is waiting, as it would be for a real read.

**Occurrence 4, same turn.** The World, after occurrence 3, found a fired `Defer` request and opened another occurrence delivering `unit` to it. `!later` fires; `Unavailable Denied on !later` fires; the `for` consolidates it as `{p: Unavailable Denied}`; `!down` fires; the child's slot receives `Unavailable Denied` and matches on it. From the outer World's view, occurrences 3 and 4 were one turn, and nothing it did was involved except answering a `Defer`.

The child cannot tell the difference between the sandbox and a World that happens to have a filesystem rooted at `/jail`. That is the test of the primitive: from inside, an interceptor is indistinguishable from the real thing. It is also why forwarding is manual. Suppose `intercept` took a table of tags to forward and did it itself; then the rewrite would be a mode of the table, the denial another, the `Defer` another, and before long the table is a second language. Here forwarding is the shortest program an interceptor can write, `@Forward` above, and rewriting, denying and holding back are longer programs in the same language. Nothing the child makes reaches the outside unless some Prism code chose to send it; there is no channel the sandbox does not see, because the language has no ambient anything.

## 8. A vertical box

```prism
type ChildInput = record:
    path: Path
    response: Response

tag Ui.VBox of Element:
    children: Path ==> Element
    responds: ChildInput

let IsUi: fn Element -> Bool = \|e| match e:
    Ui.*(..): True
    ignore: False

let UiOnly: fn (Path ==> Slot) -> Path ==> Element = \|slots|
    MapValues (Filter slots, \IsUi _.element), \_.element

let NonUi: fn (Path ==> Slot) -> Path ==> Slot = \|slots|
    Filter slots, \not IsUi _.element

let ToMap: fn ChildInput -> Path ==> Response = \|c|
    {c.path: c.response}

@VBox: fn @a -> @a = \|@child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ \|e| @Element e
    !child_in <- @Element Ui.VBox(children=(UiOnly $slots))
    !fwd <- @Forward (NonUi $slots)
    let !down = !Union (ToMap !child_in), !fwd
    @ result
```

Layout is not something the language knows about, and this is the smallest demonstration of why it does not need to. A VBox is an interceptor: it captures its child's elements, holds back the library's (`Ui.*`), and puts out a single `Ui.VBox` whose `children` field holds them as data, in key order, which is tree order, which is top to bottom. The box does no geometry, and neither does any World. Further out, the library's window interceptor reads the `children` data, computes the layout, draws it as `Draw.*` values in a `Gui.Scene`, and hit-tests each pointer event from `Gui.Window` against the rectangles it computed. A pointer that lands on a row becomes a `ChildInput` holding that child's path and a `click`, delivered to the box's slot. The box turns it into a one-entry response map and `intercept` routes it. Everything the child made that is not a library element is forwarded untouched, by the `@Forward` of example 7.

The `children` field is a map that changes whenever the child's list changes, and a field is plain data. So the `Ui.VBox` element is rebuilt, a fresh slot per change of the child's list, and the input the box listens to is a `!Switch` over the generations: that is what binding an element with a dynamic field means. A layer that diffs lists sees a VBox replaced by a VBox at the same position with different children.

Routing falls out of the tag law. `intercept` delivers `!down` against the child's list as the occurrence opened. The user clicked a row of a layout drawn from the `children` field of the VBox slot that existed as the occurrence opened, and that field was built from exactly that list. The path in the `ChildInput` therefore names a slot in the list `intercept` routes against, and there is no way for a hit-test to pick a row the user has not seen.

Take the child to be the counter of example 1 wrapped as `@VBox @counter`. The counter's own paths are `0/#0/0`, `1/#0/0` and `2/#0/0`.

**Occurrence 0 (build).** The child is built under the intercept; `$slots` has three entries, all library elements. The box's bind builds generation `#0` with one `Ui.VBox` whose children are the three; `!child_in` fires with its input event, adopted by `!Switch` for occurrence 1. `NonUi $slots` is empty, so `@Forward` builds nothing. What the library above sees is one element:

```
1/#0/0: Ui.VBox(children={0/#0/0: Ui.Button(label="Increment"),
                           1/#0/0: Ui.Button(label="Decrement"),
                           2/#0/0: Ui.Label(text="Count: 0")})
```

**Occurrence 1.** The user clicks the first row. The library above hit-tests the pointer against the layout it drew from that list, finds the first child, and sends `ChildInput(path=0/#0/0, response=click)` to `1/#0/0`. (Under the Test World, `click Ui.Button(label="Increment")` produces the same input, by the convention at the end of this example.) The slot's event fires; `!Switch` delivers `!child_in`; `ToMap` makes `{0/#0/0: click}`; `!down` fires with the same, since `!fwd` is quiet. `intercept` routes `click` to the child's slot at `0/#0/0`; from there it is occurrence 1 of example 1: `!increment`, `!delta`, the count, the label rebuilt to `2/#1/0`. The child's list changes, so `$slots` changes promptly, the box's `rebuild` builds anew to generation `#1` with the new children, and the new slot's input event is adopted at commit.

```
1/#1/0: Ui.VBox(children={0/#0/0: Ui.Button(label="Increment"),
                           1/#0/0: Ui.Button(label="Decrement"),
                           2/#1/0: Ui.Label(text="Count: 1")})
```

**Occurrence 2.** The user clicks the third row. The library finds a label, which responds to nothing, and sends the box nothing. No occurrence happens, which is what clicking a label does.

**Occurrence 3.** The user clicks row 0 and, in the same frame, the child's list changes for some other reason (say, a second counter wrapped in the same VBox whose button was also clicked; or just suppose it). The `ChildInput` is delivered to the VBox slot that existed as the occurrence opened, and the click goes to the button at that path in the list as it opened. The new VBox slot built in this occurrence takes over from the next one. A click can never land on a row the user had not seen.

Had the child created a `Net.Listener` or an `Fs.File`, `NonUi $slots` would have had it, `@Forward` would have re-emitted it in the VBox's list beside the box, and its responses would have come back through `!fwd` under the child's path. A container and a sandbox are the same program with different arms.

The convention of a `children` field holding elements as data, and a `ChildInput` holding a path and a response, is the library's, not the language's. A Test World that wants `expect Ui.Button(label="Increment")` to find a button inside a box looks inside `children` fields, and its `send` wraps the response in one `ChildInput` per container, because the library and the test suite agree to; a different library could nest differently. The language fixes only three things: leaves are elements, containment is interception, and routing is against the opened list.

## 9. A cache

```prism
let Freeze: fn a -> $a = \|v| $ v

@Cached: fn @a -> @a = \|@child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ supply
    !down <- @Forward $slots
    !refresh <- @Button $"Refresh"
    let supply = \|e| match e:
        Fs.Contents(path=p): @widget:
            $live <- @Element Fs.Contents(path=p)
            !started <- @started
            $$shown <- $live then (Freeze $live) on (!started | !refresh)
            @ $Join $$shown
        ignore: @Element e
    @ result
```

A child that watches a file sees every change the World pushes. Sometimes that is not what you want: a config file the program should read at start and again only when the user asks. This interceptor forwards every response and every element unchanged, and supplies `Fs.Contents` state through a hold of dynamics.

The supplier returns a blueprint, and that is the point: a widget can have state. For an `Fs.Contents` element it builds, in the interceptor's widget, an `Fs.Contents` slot of its own asking the outer World for the live value, and a hold `$$shown` whose value is a dynamic: initially `$live` itself, then a frozen constant of the live value (`Freeze $live`, the constant dynamic of whatever `$live` holds) each time the supplied widget starts or the Refresh button is clicked. `(Freeze $live) on (!started | !refresh)` reads `$live` through a tag, the value before the occurrence. The child's slot gets `$Join $$shown`: the value of whichever dynamic the hold currently has.

Shared by value: the runtime builds a supplied widget once per distinct element value per interceptor. If the child has two slots for `Fs.Contents(path="/etc/app.conf")`, in two different widgets that have never heard of each other, there is one supplied widget, one live slot, one hold, and both child slots observe it. That is the equality law inside an interceptor for free, and it is also a cache in the ordinary sense: the outer World is asked once.

Take a child with two readers of `/etc/app.conf` and one of `/etc/motd`, each a label over the state.

**Occurrence 0 (build).** The child's first `Fs.Contents(path="/etc/app.conf")` slot is created; the runtime applies `supply` and builds the widget: a live slot in the cache's widget, whose oracle answer is, say, `Known "v1"`, and `$$shown`, whose initial value is `$live`. `!started` fires; `(Freeze $live) on !started` fires with the constant dynamic of `$live`'s value before the occurrence, which is what the oracle answered, `$ Known "v1"`; the hold adopts it in this occurrence. `$Join $$shown` has `Known "v1"` before the adoption, from `$live`, and `Known "v1"` after it, from the frozen constant. The child's slot's dynamic is that join, so the child's label is `"v1"`. The child's second `app.conf` slot is created; the runtime finds a supplied widget for that value already built and hands it the same join. The motd slot gets its own supplied widget. The World sees two `Fs.Contents` elements and a Refresh button, plus the forwarded labels.

**Occurrence 1.** The file changes on disk; the World pushes `Known "v2"` for `Fs.Contents(path="/etc/app.conf")`. The cache's live slot is identified by that value, so `$live` changes. `$$shown` holds the frozen constant and nothing fired at it, so the join does not change. Both child labels still say `"v1"`. Commit.

**Occurrence 2.** `click` on Refresh. `!refresh` fires; `(Freeze $live) on (!started | !refresh)` fires with the constant dynamic of `Known "v2"`; `$$shown` changes; the join changes promptly to `Known "v2"`; both child labels rebuild to `"v2"` in this occurrence.

**Occurrence 3.** The World pushes `Known "v3"` and, in the same occurrence, the user clicks Refresh. `$live` changes to `"v3"` in this occurrence, but `Freeze $live` on `!refresh` reads it through a tag and gets `"v2"`. The cache shows `"v2"`; the next refresh will show `"v3"`. This is the tag law doing what it always does, and a cache that wants the pushed value in the same occurrence as the click cannot have it, because the only window onto a dynamic at an event is the value before.

The child's slot has its value in its creation occurrence and nothing less, which is the law for state elements. Suppose the supplier held plain values instead, `$cached <- Unavailable NotFound then $live on (!started | !refresh)`. Then `$cached` would be `Unavailable NotFound` before the build occurrence and `Known "v1"` after it, and a tag in the child reading it in that occurrence would see `Unavailable NotFound`, a value the oracle never gave. The hold of dynamics has no such gap, because `$Join` switches to the new inner's current value in the occurrence of the switch instead of waiting for the new inner to fire, and the hold starts as `$live`, which has the oracle's value from the first instant.

The goal served is the one that says the language is enough. A cache is not a feature of the runtime, a flag on the element, or a mode of `intercept`. It is a hold, in a widget, returned by a function, and anything else you want between a child and the World is the same construction with a different body.

## 10. A chat client

```prism
type SocketEvent = choice:
    Received Text
    Closed

type Link = choice:
    Connecting
    Connected
    Disconnected Text

tag Net.Socket of Element:
    address: Text
    requests: !Text
    responds: SocketEvent
    state: Link

let WhenConnected: fn Link, Text -> ?Text = \|link, msg| match link:
    Connected: Some msg
    ignore: Nil

let Line: fn SocketEvent -> ?Text = \|ev| match ev:
    Received t: Some t
    Closed: Some "(closed)"

let Status: fn Link -> Text = \|link| match link:
    Connecting: "connecting..."
    Connected: "connected"
    Disconnected why: Concat [["disconnected: ", why]]

@chat:
    $address <- @TextInput "address", "chat.example:7000"
    $draft <- @TextInput "draft", ""
    !send <- @Button $"Send"
    !conns <- rebuild $address:
        let !out = !Filter (WhenConnected $link, $draft on !send)
        ((!rx, $link)) <- @Element Net.Socket(address=_, requests=!out)
        @ ((!rx, $link))
    let !rx = !Switch (First !conns)
    $$link <- $ Connecting then Second !conns
    let $link = $Join $$link
    $log <- [[]] then Append $log, !Filter (Line !rx)
    @Label (Status $link)
    @Label (JoinText "\n", $log)
```

One element, three parts. `Net.Socket` takes a request event, the text to send; responds with what arrives; and has state, whether the connection is up. The address is a dynamic, so the whole element lives inside a `rebuild`: a new address is a new socket, and reconnecting is typing. The `rebuild` returns a tuple, an event and a dynamic, and the two halves cross the `rebuild` boundary differently because they are different kinds of thing: the event through `!Switch`, which is not prompt, and the dynamic through a hold and `$Join`, which is.

`!out` is the send, filtered by the link state: `WhenConnected $link, $draft on !send` reads both dynamics through tags and fires only when the link was `Connected` before the occurrence. `$link` inside the `rebuild` block is the socket's own state, the one from this generation, and `!out` is bound before the element that produces it, which is fine in a widget block: bindings are recursive, and there is no prompt edge from a request event to a response event or to the state, since those come from outside in later occurrences.

**Occurrence 0 (build).** Two text inputs and a button are built. The `rebuild` builds generation `#0`: an `Net.Socket(address="chat.example:7000")` slot. The oracle is asked for `Link` of that element; the World has no such connection and answers `Connecting`. `!conns` fires with the tuple; `$$link` changes to the new `$link` in this occurrence; the join is `Connecting`; the status label says `"connecting..."`. `!send` did not fire, so nothing was requested.

```
0/0:        Ui.Focusable(name="address")
0/1/#0/0:   Ui.Label(text="chat.example:7000|")
1/0:        Ui.Focusable(name="draft")
1/1/#0/0:   Ui.Label(text="|")
2/#0/0:     Ui.Button(label="Send")
3/#0/0:     Net.Socket(address="chat.example:7000")
4/#0/0:     Ui.Label(text="connecting...")
5/#0/0:     Ui.Label(text="")
```

The World reads the list, sees a socket element for an address it has no connection to, and opens one. Whether it keeps one OS socket per address or per slot is its business.

**Occurrence 1.** The connection comes up. The World pushes `Connected` for `Net.Socket(address="chat.example:7000")`. The slot's `$link` changes; the join changes; the status label rebuilds to `"connected"`.

**Occurrence 2.** The user has typed `hello` into the draft (several occurrences, each a keystroke to `1/0`, omitted) and clicks Send. `click` to `2/#0/0`. `!send` fires; `$draft on !send` fires with `"hello"`; `WhenConnected` reads `$link` as `Connected` and passes it; `!out` fires, which is the slot's request. The World sees `requested: "hello"` on `3/#0/0` after the occurrence and writes it to the connection.

**Occurrence 3.** A line arrives. The World sends `Received "hi back"` to `3/#0/0`. The slot's response event fires; `!Switch` delivers it as `!rx`; `Line` passes it; `Append $log, !line` fires with `[["hi back"]]`; the log label rebuilds.

**Occurrence 4.** The user edits the address to `chat.example:7001` and, in the same frame, clicks Send with `"bye"` in the draft. The keystroke goes to `0/0` and the click to `2/#0/0`. `$address` changes; the `rebuild` builds anew: generation `#1` is built with a slot for the new address, whose oracle answer is `Connecting`, since the World has no such connection. Now both generations propagate in this occurrence. In generation `#0`, `$draft on !send` fires with `"bye"`, `WhenConnected` reads that generation's `$link` as `Connected`, and the old slot's request fires. In generation `#1`, which sees all of the occurrence including the click, `$draft on !send` fires too, but `WhenConnected` reads the new slot's `$link` as `Connecting` and drops it. Exactly one send, on the old connection. `!conns` fires with the new tuple; `$$link` changes to the new `$link`; the join is `Connecting`; the status label says `"connecting..."`. At commit generation `#0` is dropped. `!Switch` adopts the new `!rx`.

```
3/#1/0:     Net.Socket(address="chat.example:7001")
```

The World sees the old socket element gone and the new one present. It closes the old connection, after writing `"bye"`, and opens the new one. The old slot's request fired in this occurrence and its path is not in the list after it; the World performs it or not by its own policy for an operation whose slot is gone, and whatever it does, no response for it will ever reach the program, because there is no slot to address.

**Occurrence 5.** Suppose the World had a `Received` queued for `3/#0/0`, read from the old connection before it closed. It discards it: that path is not in the list it last received.

A second `Net.Socket` element for `chat.example:7000` somewhere else in the program would share the first's `Link` state, because state is identified by value, and its sends and receives would be its own, because responses are identified by path. One connection, two conversations. If the user had typed the old address back, a second slot with that value would keep the connection alive through the change, which is either a feature or a surprise depending on what the program wanted, and either way it is in the program's text.

What this prevents: the socket handler that writes to a connection object captured by a closure that outlived the reconnect. There is no connection object. There is an element with an address, a request event that is the program's only way to send, and a slot that lives exactly as long as the `rebuild` generation does.

## 11. A line counter

```prism
type StdinEvent = choice:
    Chunk Text
    End

type Code = Code Int

type WriteFailed = choice:
    Closed
    Failed Text

tag Cli.Stdin of Element:
    responds: StdinEvent

tag Cli.Stdout of Element:
    requests: !Text
    responds: WriteFailed

tag Process.Exit of Element:
    requests: !Code

let ChunkOf: fn StdinEvent -> ?Text = \|ev| match ev:
    Chunk t: Some t
    End: Nil

let IsEnd: fn StdinEvent -> ?unit = \|ev| match ev:
    End: Some unit
    ignore: Nil

@wc:
    !in <- @Element Cli.Stdin
    let !chunk = !Filter (ChunkOf !in)
    let !end = !Filter (IsEnd !in)
    $lines <- 0 then $lines + Count !chunk, '\n'
    !werr <- @Element Cli.Stdout(requests=("{$lines}\n" on !end))
    @Element Process.Exit(requests=(Code 0 on !end))
    @ unit
```

`wc -l`, under a World with no screen. Standard input is a stream element: it responds with chunks and then with an end, and the program never asks it for anything. Standard output and exit are request elements: nothing happens until their request events fire, and the firing is the whole instruction. There is no `print`, and there is no `exit()`; there is an element whose request fired, read by the World after the occurrence.

Exit is a request and not a presence. If exiting were "a `Process.Exit` element is in the list", the program would have to arrange for the element to appear at the right moment, with a `rebuild` over some flag, and the World would have to watch for appearances, which is a trigger by existence and the language has none. Instead the element is always there and fires `Code 0` when the input ends.

**Occurrence 0 (build).** Three slots. `$lines` is `0`. Nothing fires.

```
0: Cli.Stdin
1: Cli.Stdout
2: Process.Exit
```

The World sees a `Cli.Stdin` slot and starts reading standard input.

**Occurrence 1.** The World sends `Chunk "one\ntwo\nthr"` to `0`. `!in` fires; `ChunkOf` passes it; `Count !chunk, '\n'` is `2`; `$lines + 2` fires with `2`. `IsEnd` gives `Nil` and `!end` is quiet. Commit: `$lines` is `2`.

**Occurrence 2.** `Chunk "ee\n"` to `0`. `$lines` becomes `3`.

**Occurrence 3.** `End` to `0`. `!end` fires; `"{$lines}\n" on !end` fires with `"3\n"`, reading `$lines` through a tag as `3`; `Code 0 on !end` fires. After the occurrence the World sees `requested: "3\n"` on `1` and `requested: Code 0` on `2`. It performs request firings in list order, so the write happens, then the exit.

`$lines on !end` is the count *before* the occurrence in which the end arrived. That is right because the end arrives alone: a slot receives at most one response per occurrence, so the last chunk and the end are always in different occurrences, and by the time `End` arrives every chunk has been counted and committed. A World that put the last chunk and the end into one occurrence could not, because they are both responses to the same slot. Had the program instead read a stream where the final chunk and the end were different slots, it would have had to count the chunk in the same expression as the output, and the type system would not have warned it; the walkthrough is the warning.

A real `wc` takes a file name on the command line, and that is where `@main` taking no arguments shows its reason. Arguments are a state element:

```prism
tag Process.Args of Element:
    state: [Text]

@wc_file:
    $args <- @Element Process.Args
    !!resps <- rebuild $args:
        !started <- @started
        match _[0]:
            Some path:
                !r <- @Element Fs.File(path=path, requests=(Read on !started))
                @ r
            Nil:
                @ !never
    let !resp = !Switch !!resps
    ...
```

The `match` inside the `rebuild` chooses between two blueprints by a plain value, which is the one form a `match` with blueprint arms may take.

`$args` has a value in occurrence 0, from the oracle, and the `rebuild` over it builds once with that value; a World that cannot observe a change to the arguments never pushes one and the `rebuild` never rebuilds. Arguments are not parameters of `@main` because of the sandbox of example 7: an interceptor can supply a child a different `Process.Args`, and a test can set them with `with Process.Args = [["notes.txt"]]`, and neither needs a second way to start a program. A `Process.Env` is the same again.

The World that runs this is the same one that runs example 5's viewer, and `Fs.File` is not a CLI tag: a GUI program reads files the same way. The CLI World adds only standard input, standard output, standard error and signals, four tags; exit, like the arguments, is a process tag.

## 12. A tiny HTTP server

```prism
type Accepted = Accepted Nat

type ConnEvent = choice:
    Received Text
    Closed

type ConnRequest = choice:
    Send Text
    Close

tag Net.Listener of Element:
    port: Nat
    responds: Accepted

tag Net.Conn of Element:
    id: Nat
    requests: ![ConnRequest]
    responds: ConnEvent

type Delta = choice Opened | Shut

let IdOf: fn Accepted -> Nat = \|a| a.value

let RequestLine: fn ConnEvent -> ?Text = \|ev| match ev:
    Received t: Some t
    Closed: Nil

let IsClosed: fn ConnEvent -> ?unit = \|ev| match ev:
    Closed: Some unit
    ignore: Nil

let Respond: fn Text -> Text = \|request_line|
    Concat [["HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\nHello, world\n"]]

let ApplyDeltas: fn Nat ==> unit, Nat ==> Delta -> Nat ==> unit = \|conns, deltas|
    Fold deltas, conns, \|acc, id, d| match d:
        Opened: Insert acc, id, unit
        Shut: Remove acc, id

@server:
    !accepted <- @Element Net.Listener(port=8080)
    let !opens = {IdOf !accepted: Opened}
    !closes <- for $conns:
        !ev <- @Element Net.Conn(id=key, requests=!reply)
        let !reply = [[Send (Respond !Filter (RequestLine !ev)), Close]]
        @ Shut on !Filter (IsClosed !ev)
    $conns <- {} then ApplyDeltas $conns, (!Union !opens, !closes)
    @ unit
```

A listener is a stream element: it responds with accepted connections, each with an id the World allocated, and asks for nothing. A connection is a request element keyed by that id: the program sends on it and hears from it. The program's model is the set of open connections, `$conns`, a map from id to nothing; the `for` over it builds one `Net.Conn` slot per open connection, and the connection's block answers the first thing it receives with a fixed response and a close. When the World reports the connection closed, the block fires `Shut`, the consolidated event removes the key, and the block is torn down.

`!reply` is an array literal with an event inside it, so it is an event of arrays, firing when the request line arrives, with two requests in order. A request event fires at most once per occurrence, which is why a request type that wants two things in one moment is an array.

The cycle is the todo list's again: `$conns` is held from `!closes`, which is the result of `for $conns`. It passes through the tag on `$conns` in `ApplyDeltas` and through the delayed membership of the `for`'s merge, and a connection's events are responses, which never arrive in the occurrence the slot was created.

**Occurrence 0 (build).** One slot, the listener; `$conns` is `{}`; the `for` builds nothing.

```
0: Net.Listener(port=8080)
```

The World binds port 8080 because a listener element exists in the list, and starts accepting.

**Occurrence 1.** A client connects. The World sends `Accepted 7` to `0`. `!accepted` fires; `!opens` fires with `{7: Opened}`; `!Union` fires with it; `ApplyDeltas` fires with `{7: unit}`; `$conns` changes; the `for` builds block `[7]` in this occurrence, creating `Net.Conn(id=7)` at `1/[7]/0`. Nothing else fires: the new slot's `!ev` cannot fire in the occurrence it was created. Commit.

```
0:        Net.Listener(port=8080)
1/[7]/0:  Net.Conn(id=7)
```

The World sees a connection element for id 7, which it has, and will deliver its bytes to `1/[7]/0`.

**Occurrence 2.** The request arrives. `Received "GET / HTTP/1.1\r\n..."` to `1/[7]/0`. `!ev` fires in block `[7]`; `RequestLine` passes it; `Respond` builds the response; `!reply` fires with `[[Send "HTTP/1.1 200 OK...", Close]]`, which is the slot's request. `IsClosed` is `Nil`; the block's result is quiet; `$conns` is unchanged. After the occurrence the World sees `requested: [[Send "...", Close]]` on `1/[7]/0`, writes the response and closes the connection.

**Occurrence 3.** Two things at once: the World has closed connection 7 and reports it, and a new client has connected. `Closed` to `1/[7]/0` and `Accepted 8` to `0`, together. Block `[7]` fires `Shut`; `!closes` fires with `{7: Shut}`. `!opens` fires with `{8: Opened}`. `!Union` fires with `{7: Shut, 8: Opened}`; `ApplyDeltas` removes 7 and inserts 8; `$conns` changes; the `for` builds `[8]` now and tears `[7]` down at commit. The World sees one connection element replaced by another and, because it already closed 7, has nothing to cancel.

```
0:        Net.Listener(port=8080)
1/[8]/0:  Net.Conn(id=8)
```

**Occurrence 4.** Suppose the World has bytes queued for `1/[7]/0` that arrived before the close. It must discard them: the path is not in the list it last received. A late result for a slot that is gone is the World's problem, and the rule that it must drop them is what lets the program never have a slot it did not ask for.

A server that read real requests would need to collect the bytes until the blank line that ends the headers, and that is a hold per connection, inside the `for` block, next to the slot it serves:

```prism
    !closes <- for $conns:
        !ev <- @Element Net.Conn(id=key, requests=!reply)
        $buf <- "" then Concat [[$buf, !Filter (RequestLine !ev)]]
        let !head = !Filter (CompleteHead (Concat [[$buf, !Filter (RequestLine !ev)]]))
        let !reply = [[Send (Respond !head), Close]]
        @ Shut on !Filter (IsClosed !ev)
```

`$buf` lives as long as key `key` does and is torn down with it; two connections have two buffers that cannot see each other; and `Concat [[$buf, !chunk]]` reads the buffer before the chunk by the tag law, which is the right buffer to append to. `CompleteHead: fn Text -> ?Text` is a plain function that returns the head once the blank line is in. The second `Concat` repeats the first rather than reading `$buf` after it changed, because there is no "after" inside an occurrence: a program that wants the new buffer and the test on it in the same occurrence writes the expression once and uses it twice, or writes a derived `let`.

What this prevents: the server where a connection object is kept in a map by hand, removed in a close callback that sometimes runs after the next read callback, so the map has a dead entry that the next write crashes on. Here the map is the only model, its definition lists every event that changes it, the slot's life is the key's life, and a write to a dead connection is impossible because a dead connection has no slot.

## 13. Testing the counter

```prism
test counter_counts, @counter:
    build
        => expect Ui.Label(text="Count: 0")
    click Ui.Button(label="Increment")
        => settles
    click Ui.Button(label="Increment")
        => expect Ui.Label(text="Count: 2")
    click Ui.Button(label="Decrement")
        => expect Ui.Label(text="Count: 1")
        => expect_none Ui.Label(text="Count: 2")

test counter_left_wins, @counter:
    build
        => settles
    click Ui.Button(label="Increment"), click Ui.Button(label="Decrement")
        => expect Ui.Label(text="Count: -1")

test counter_is_quiet, @counter:
    build
        => settles
    click Ui.Button(label="Increment")
        => quiet
        => returns unit

test stopwatch_banks_time, @stopwatch:
    with Clock = Instant 5000, build
        => expect Ui.Label(text="0 ms")
    click Ui.Button(label="Start")
        => settles
    push Clock = Instant 5100
        => expect Ui.Label(text="100 ms")
    push Clock = Instant 5600
        => settles
    click Ui.Button(label="Stop"), send Time.Every(period=100), tick
        => expect Ui.Label(text="600 ms")
    push Clock = Instant 9000
        => expect Ui.Label(text="600 ms")
        => unrequested Time.Every(..)
```

A test is a World. The Test World accepts every tag, has no effects, is the only source of inputs, and its snapshot is whatever the script says. A script drives the program the way a user would, by sending inputs to elements, and reads what a user would read, the element list. It cannot reach into `@counter` and poke `$count`, because nothing can: there are no reified names, and a test that passes under this World passes under any World that delivers the same inputs.

Each step is a turn or a check between turns. The build is turn zero, so the first `expect` sees the built program. `send` finds exactly one slot matching the pattern, delivers the value, and runs a turn, including any `Defer` occurrences the program causes. Several deliveries on one line go in one occurrence; two to the same slot fail the step, because a slot receives one response per occurrence. `with` sets the snapshot value a state element will see, from the next turn's oracle and push; `push` is `with` followed by a turn with nothing else in it. `quiet` and `returns` look at the widget's result over the last turn.

Here is the first script as the Test World sees it, turn by turn.

**Turn 0 (build).** The list is example 1's after occurrence 0. `expect Ui.Label(text="Count: 0")`: some slot at any depth matches; `2/#0/0` does. Pass.

**Turn 1.** `click Ui.Button(label="Increment")`: one slot matches, `0/#0/0`; the World runs an occurrence with `{0/#0/0: click}` as its responses. Occurrence 1 of example 1. The list now has `2/#1/0: Ui.Label(text="Count: 1")`.

**Turn 2.** The same. `"Count: 2"`. The `expect` passes.

**Turn 3.** `send` to the Decrement slot. `"Count: 1"`. `expect` passes; `expect_none Ui.Label(text="Count: 2")` passes because the generation-`#2` label was dropped at commit and is not in the list.

The second script is the tag law and the leftmost merge in four lines. the comma makes one occurrence with two responses to two slots; `!delta` fires with `-1`; the label says `"Count: -1"`. A test that wanted the other answer would be testing a different program.

The third script says something about the result. `@counter`'s last statement is `@Label ...`, whose result is `unit`, so the widget's result is `unit` and `returns unit` holds; `quiet` holds trivially because the result is not an event. A widget that returned `!delta` would make `fired +1` the interesting step.

The fourth script is example 6 under a clock the script owns. `with Clock = Instant 5000` sets the snapshot before the build, so the oracle answers it in occurrence 0. `push Clock = Instant 5100` is a turn with one push and no responses; the label counts. The line with both delivers a Stop click and a tick in one occurrence, as a real World might: `Step` reads `$clock` as `5600` through a tag and banks `600`. A later push does not move the label, because the watch is stopped.
One more script, for the sandbox of example 7, because it shows a turn that is more than one occurrence:

```prism
@probe:
    !started <- @started
    !r <- @Element Fs.File(path="/etc/shadow", requests=(Read on !started))
    $seen <- Nil then Some !r
    @Label (ShowResponse $seen)

test sandbox_denies_outside_root, (@Sandbox "/jail", @probe):
    build
        => expect_none Fs.File(..)
        => expect Ui.Label(text="(reading...)")
        => requested Defer, unit
    push Clock = Instant 0
        => expect Ui.Label(text="(permission denied)")
```

**Turn 0 (build).** The probe's `Fs.File` slot is created and its request fires; the sandbox's `for` builds the `Nil` arm for it, which fires the `Defer`. The outer list has no `Fs.File` in it, which `expect_none` checks: the sandbox held it back. The label says `"(reading...)"`. But the turn is not over: the Test World, like every World, checks for fired `Defer` requests after each occurrence and runs another occurrence delivering the answer. In that second occurrence, still turn 0, the sandbox fires `Unavailable Denied` into the probe, and the probe's label rebuilds. So by the time the script's first step runs, the label already says `"(permission denied)"`, and the second `expect` above fails.

That is the point of the script as written: a reader who expects `"(reading...)"` has the turn wrong. The build turn is the build occurrence plus any `Defer` occurrences it causes, and a check between turns sees the end of the turn. The corrected script deletes the second `expect` and the `push`, which was only there to make a turn pass, and keeps `requested Defer, unit`, which holds because the `Defer` slot fired in turn 0, and the final `expect`. A test that wanted to see the request before the denial would need a World that delivered `Defer` answers lazily, and there is no such World, because the `Defer` rule is what bounds a turn.

The transcript the Test World prints for each turn, the element list, the request firings per slot, the result and any error, is what the conformance suite compares between the two implementations. A difference is a bug in one or an ambiguity in the specification, and the second kind is the more valuable find.

The goal served is "the spec is the thing", in its testable form. The script never names a path, a widget, or a runtime object; it names elements and values, which are the only things a World can name, and the transcript it checks is the only thing a World can see.

## 14. A shell

```prism
type App = choice A | B

let Other: fn App -> App = \|app| match app:
    A: B
    B: A

let Pick: fn App, Path ==> Slot, Path ==> Slot -> Path ==> Slot = \|app, sa, sb| match app:
    A: sa
    B: sb

let OnlyFor: fn App, App, Path ==> Response -> ?(Path ==> Response) = \|want, have, m|
    if (want == have): Some m
    else: Nil

@Shell: fn @unit, @unit -> @unit = \|@a, @b| @widget:
    ((ignore, $sa)) <- intercept @a, !down_a, $ \|e| @Element e
    ((ignore, $sb)) <- intercept @b, !down_b, $ \|e| @Element e
    !next <- @Button $"Next app"
    $active <- A then Other $active on !next
    let $shown = Pick $active, $sa, $sb
    !child_in <- @Element Ui.VBox(children=(UiOnly $shown))
    !fwd_a <- @Forward (NonUi $sa)
    !fwd_b <- @Forward (NonUi $sb)
    let !click = ToMap !child_in
    let !down_a = !Union (!Filter (OnlyFor A, $active, !click)), !fwd_a
    let !down_b = !Union (!Filter (OnlyFor B, $active, !click)), !fwd_b
    @ unit

@main:
    @Shell @counter, @stopwatch
```

A World can be written in Prism, and this is one: a program that runs two other programs as its children, shows one of them on the screen at a time, and routes the user's clicks to the one that is showing. From the children's side, `@Shell` is their World. The counter asks for buttons and a label and gets clicks; the stopwatch asks for a clock, a timer and buttons and gets pushes, ticks and clicks; neither knows the other exists or that the screen is shared. From the outer World's side, the shell is one program with a box, a button, a clock and a timer.

Two intercepts, one per child, each with its own response event and the forwarding supplier. The screen is example 8's VBox over whichever child is active, with `Pick` lifted over three dynamics choosing the list; the `ChildInput` the box receives names a path in `$shown` as the occurrence opened. The click goes down to the active child only, with `$active` read through a tag so that a click in the same occurrence as a switch of apps goes to the app the user was looking at. Everything that is not a library element, from either child, is forwarded unchanged, and the responses come back under the child's own paths through `!fwd_a` and `!fwd_b`. The inactive child keeps running: its holds persist, its `Time.Every` is still in the outer list, its clock still gets pushes. It is simply not drawn and not clickable.

**Occurrence 0 (build).** Both children are built. `$sa` has the counter's three slots; `$sb` has the stopwatch's six, of which `Clock` and `Time.Every` are not library elements. The oracle for `Clock` is asked through the shell's supplier, which makes an outer `Clock` element in the shell's widget, built once because there is one value, and the outer World answers `Instant 5000`. `$active` is `A`; `$shown` is `$sa`; the box's element is built as generation `#0` with the counter's three library elements as children. `@Forward (NonGui $sa)` builds nothing; `@Forward (NonGui $sb)` builds a `Clock` and a `Time.Every(period=100)` in the shell's widget. The outer World sees:

```
2/#0/0:               Ui.Button(label="Next app")
3/#0/0:               Ui.VBox(children={0/#0/0: Ui.Button(label="Increment"),
                                         1/#0/0: Ui.Button(label="Decrement"),
                                         2/#0/0: Ui.Label(text="Count: 0")})
5/0/[0]/#0/0:         Clock
5/0/[1]/#0/0:         Time.Every(period=100)
s/0:                  Clock
```

Two `Clock` elements, the supplier's and the forwarder's, and that is fine: state is identified by value, so the outer World has one clock and both slots observe it. The stopwatch's own `Clock` slot gets its dynamic from the supplied widget, and the forwarded `Clock` slot's result is bound to nothing. A shell that minded the duplicate would not forward state elements, since the supplier already answers them; this one forwards everything that is not GUI because it is the shortest correct program, and the duplicate costs nothing.

**Occurrence 1.** A tick: `tick` to `5/0/[1]/#0/0` and a push of `Instant 5100` for `Clock`. The forwarded `Time.Every` slot's response is routed by `!fwd_b` to the stopwatch's slot at its path `1`; nothing listens there. The push reaches the supplied `Clock` widget's slot, whose dynamic is the stopwatch's `$clock`; the stopwatch's label, derived from it, rebuilds with the same text. `$sb` changes, but `$shown` is `$sa`, so nothing on screen changes.

**Occurrence 2.** The user clicks the first row of the box. The library above hit-tests the pointer against the layout it drew from `$shown`, the counter's list as it opened, finds the button, and sends `ChildInput(path=0/#0/0, response=click)` to `3/#0/0`. `!click` fires with `{0/#0/0: click}`; `OnlyFor A, $active, !click` reads `$active` as `A` and passes it; `!down_a` fires; the counter's Increment slot gets its click. The counter's label rebuilds; `$sa` changes; `$shown` and `$kids` change; the box rebuilds to generation `#1`. `OnlyFor B` drops the click and `!down_b` is quiet, apart from `!fwd_b`, which is also quiet.

**Occurrence 3.** `click` to `2/#0/0`, Next app, together with a tick and a push. `$active` changes to `B`; `Pick` changes `$shown` to `$sb`; the box rebuilds with the stopwatch's buttons and label. In the same occurrence the tick and push go where they always go.

```
2/#0/0:               Ui.Button(label="Next app")
3/#2/0:               Ui.VBox(children={2/#0/0: Ui.Button(label="Start"),
                                         3/#0/0: Ui.Button(label="Stop"),
                                         4/#0/0: Ui.Button(label="Reset"),
                                         5/#2/0: Ui.Label(text="0 ms")})
5/0/[0]/#0/0:         Clock
5/0/[1]/#0/0:         Time.Every(period=100)
s/0:                  Clock
```

**Occurrence 4.** The user clicks the first row, and in the same frame hits Next app again. `ChildInput(path=2/#0/0, response=click)` to `3/#2/0` and `click` to `2/#0/0`. The library hit-tested against the layout of `$shown` as it opened, the stopwatch's list, and found the Start button at `2/#0/0`. `OnlyFor A, $active, !click` reads `$active` as `B`, before the occurrence, and drops it; `OnlyFor B` passes it; the stopwatch starts. `$active` becomes `A` at commit and the screen shows the counter from the next occurrence. The click went to the app the user was looking at when they clicked, which is the only answer routing allows and the one the user meant.

**Occurrences 5 and on.** Ticks keep coming; the stopwatch keeps counting behind the counter; when the user switches back, the box shows the elapsed time it reached while hidden.

A shell, a window manager and a compositor are this program with more arms: a process table is a `for` over a keyed collection of programs, a window is a region of a bigger box, focus is which child gets the keys. None of it is in the language. The language supplies two things: an interceptor that re-emits everything and supplies by forwarding is observably not there, so each layer can be put in and taken out without the children knowing; and routing is always against the opened list, so no layer can send an input to something the user has not seen. The outer World below the shell is small, because the shell did the multiplexing; it could be the software rasteriser of a Prism-native system, or a Test World that says `click Ui.Button(label="Increment")`, which reaches the button through the box's `children` field by the library's convention, and checks what the box contains afterwards.

## What the fourteen have in common

Every program above is a set of equations over three kinds of name, built once, and driven by a World that sends plain data to paths it has seen and reads plain data back. None of them registers a callback, none of them reads a value "right now", none of them asks for an effect except by firing a request, and none of them has a moment in which some of its values are new and some are old, except through the tag law, which is the one deliberate window onto before. The hard cases a framework usually handles with a lifecycle table, a batching heuristic or a scheduler were each handled above by a sentence: a click in the occurrence of a rebuild goes to the old slot; a widget built mid-occurrence sees the whole occurrence; a state element has its value in the occurrence it is created; a request is answered later or not at all; the leftmost wins. The chapters that follow define those sentences one at a time, and each one points back here to the occurrence where it did its work.
