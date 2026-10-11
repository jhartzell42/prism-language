# Chapter 5 — Widgets and switching

A widget is the one thing in Prism with a lifetime, and this chapter is about what that buys. Events and dynamics describe the whole of time and never need to be born or die; a widget is built in one occurrence, lives through some, and is torn down in another, and so every piece of the language that needs a beginning and an end (state, IO, identity, cycles, subscriptions) is owned by one. Three constructs let a program change its own structure while it runs, `dyn`, `for` and the lifting that quietly spells `dyn` for you, and two primitives, `!Switch` and `$Join`, bring results across the boundary of a part that was replaced. The underlying claim is that one ownership tree, with rebuilds as ordinary consequences of dynamics changing in the occurrence they change, replaces reconciliation, lifecycle and garbage collection at once.

## What a widget is

You have had the afternoon where the subtree was gone but its listener was not. A panel was removed from the screen; a timer it had registered kept firing; the handler read a model that no longer existed and threw, or worse, wrote to one that did. Or the other afternoon: a component was unmounted and remounted by a parent's re-render, its `componentDidMount` ran again, and the server got a second request for the same data, which arrived after the first one and overwrote a newer value with an older one. Both are the same bug. Something with a lifetime was treated as though it did not have one. The listener outlived its panel; the request outlived its component; nobody owned either, so nothing could end them.

Prism's answer is that exactly one kind of thing has a lifetime, and everything that needs one is owned by it. An event is a partial function from occurrences to values. A dynamic is a value at every occurrence. Neither is created and neither is destroyed; they are descriptions of all of time, and when a runtime drops one it is pretending for efficiency, not changing the meaning of a program. A widget is different in kind. It is built, at a position in a tree, in some occurrence; it exists for a while; and it is torn down when its parent replaces or removes it. The sigil `@` marks the things that are built, and the list of what a widget does is the list of everything in the language that is not pure computation.

**Widgets do IO.** All IO is elements, and an element is created by a widget with `@Element`. The slot lives as long as the node that created it and not one occurrence longer. When the node is torn down the element leaves the list, the World sees it go, and whatever the World was doing on its behalf is cancelled or discarded. A file read, a socket, a button, a timer: each is a widget, because each has a beginning and an end, and the thing you reached for in every other framework, a handle you could stash in a field and forget to close, does not exist.

**Widgets own holds.** `$x <- v then !e` is a statement in a widget block and nowhere else, because the hold's value has to belong to something that can be torn down. When a `dyn` rebuilds its block the state inside it resets; when a `for` removes a key the state of that key's block is freed. A hold outside any widget would be state that nothing owns, which is a leak by construction, and the language refuses to write one.

**Widgets own their cycles.** `$count <- 0 then $count + !delta` refers to itself. Something has to hold the placeholder that stands in for `$count` before its definition is complete and the closure that completes it, and that something is the node. A cyclic subgraph is then reachable only through its node and dies with it, rather than keeping itself alive by referring to itself, which is the one kind of leak reference counting cannot collect and the one the ownership tree makes impossible.

**Widgets own subscriptions.** Every occurrence-time binding in a block, every `$count + !delta`, is a node in the propagation graph subscribed to its inputs. The subscription is held weakly from the input's side and owned by the widget's node from the subscriber's side. When the node is torn down its graph nodes stop being reachable and the inputs stop delivering to them. There is no `unsubscribe` in the language and no `removeEventListener` to forget; there is teardown.

**Widgets have identity, and only as nodes.** Building the same widget twice gives two nodes, two paths, two sets of slots, two sets of holds. That is what makes two buttons with the same label two buttons. Events and dynamics have no identity at all: an event used in two places is one event, and asking whether two dynamics are "the same one" is not a question the language can pose.

**Widgets are the GC story.** The whole graph hangs off the widget tree. The tree is owned top-down, from the World's root node through every `<-` bind, every `dyn` and every `for`. A node is torn down when its parent replaces or removes it; teardown is recursive; and because every graph object is owned by exactly one node, there is nothing to collect that the tree does not reach and nothing the tree reaches that is not owned. A result event of a torn-down node, still held by some parent, simply never fires again; a result dynamic keeps its last value. The replaced node is kept until the end of the occurrence in which it was replaced, so that anything it fired in that occurrence is delivered, and is dropped at commit. The Rust implementation needs no tracing collector: ownership of nodes is a tree, and reference counting handles the sharing of plain values.

### Blueprint and widget

A value of type `@a` is a blueprint; a widget is a building made from it. The blueprint is a value: you can store it with `let`, pass it to a function, return it from another blueprint, and bind it as many times as you like. Each bind builds a widget. Widgets never alias. The word is blueprint and not recipe because a recipe's steps are opaque to the dish once it is cooked, and a blueprint stays in correspondence with the building: the bindings of a blueprint are the wiring of the widget for as long as it stands, and its closures are the widget's working parts. Where the runtime chapters say "node" they mean the widget as a data structure, the thing with a path, children and a lifetime; it is the same thing seen from inside.

```prism
@two_buttons:
    let @b = @Button $"OK"
    !x <- @b
    !y <- @b
    @ ((!x, !y))
```

Two buttons, two paths, two click events, and `@b` is neither of them. If you have used a UI framework where a component instance was a value you could hand around, this is the thing to unlearn: `@b` cannot be clicked, because `@b` is not on the screen. Only its nodes are, and a node is not something you can name.

A node is not a value. You cannot hold one in a variable, pass one to a function, put one in a map, or compare two. The only way to get a node is to bind a widget in a hierarchy, and what you get is not the node but its *result*, the event or dynamic or bundle that the widget block's last statement produced. The node itself is a position in the tree, known to the runtime, and the program talks about it only implicitly, by being written at that position.

Suppose nodes were values. Then a program could put one in a hold, and the parent's decision to remove it would mean nothing: the node would live on, its slots would stay in the element list, its subscriptions would keep delivering, and ownership would be a graph rather than a tree. Every runtime whose nodes are values ends up with a collector, because a tree with back-references is not a tree. Every program in such a runtime ends up with the leaked subtree. Refusing node values is what makes teardown a `drop` and the GC story a sentence.

This document says "widget" for both when the context makes it clear, which is how everyone talks, and says "node" when the difference is the point: a hold belongs to a node, a path names a node, teardown happens to a node, a widget is bound to make a node.

### Building is the only doing

A widget block does not run. It is built, once per node, and reading it top to bottom is reading a blueprint: these children, these holds, these graph edges, these slots. The recipe's only effects are on the tree, never on the world. The world is told about the tree afterwards, through the element list, and does what it does with that. In this sense a Prism program is pure even though widgets do IO: a widget does not perform IO, it *is* IO, in the way a function type is a computation rather than the performing of one, and the World is the only thing that performs.

This is what makes the question "what happens when this widget is built" always have the same answer. Nothing happens. Slots appear in a list. If the program wants something to happen, it fires a request event, and `@started` exists so that a widget can fire one in its build occurrence; the response arrives later, from outside. There is no code path by which construction reaches the world, and so there is no ordering problem between construction and the world, no fetch-on-mount that fires twice, and no test that needs a mock installed before the first render.

### Why the archetype is a GUI, and why the word stops meaning that

A button is the smallest thing that has every property above at once. It has a lifetime (it is on the screen or it is not). It has a little state (pressed). It has an element in the world (something to draw, something to click). It has an identity distinct from its twin (two OK buttons are two buttons). And it has a parent that can remove it. Once a button is understood as "the thing with all of these", the next step is noticing that a socket has all of them too: it is opened and closed, it has state (connecting, connected), it has an element (an address the World dials), it has identity (two connections to the same host are two connections), and it has an owner that can drop it. So does a file read in flight. So does a timer. So does an entire application hosted by a shell.

That is why the sigil is called "widget" and why, a few pages in, the word has nothing to do with screens. Anything with a lifetime is a widget; everything without one is a value. The claim of the language is that this division is universal, and the test is whether anything ever needs to be neither. Nothing in this book is.

### What a node owns

A node holds its children, its holds, its cyclic placeholders, its element slots (each with a path, an element value, a response trigger, a state dynamic and a request event) and the graph nodes its occurrence-time bindings compile to. The target node has no backend channel: no public event table, no `backend_data`, no delegate. The run call is the only boundary, and a node is told nothing and tells nothing except through the element list.

Ownership points one way, parent to child, through `Rc`. Subscriber lists in the graph hold `Weak` references. A closure compiled from an occurrence-time expression captures the graph objects it reads through the node that owns it, as indices into that node's environment, never as a strong pointer to a sibling's shared object. Those three rules together are the whole reason there is no collector: the tree owns the nodes, the nodes own their closures, the closures reach the graph through the tree, and nothing points back up. The runtime is bound by all three and the compiler by the third; chapter 11 has the `WidgetBuilder` that makes the first two true by construction and the test, `dropping_a_widget_frees_its_cyclic_loops`, that checks the one place a node could own itself.

## Widget blocks

Widget blocks are where "one way to change anything, written next to the thing" becomes syntax. A hold is a statement that may appear only here, so every piece of state in a program lives inside some widget, owned by its node, with its definition naming every event that can move it. When you want to know what can happen to `$count`, you read the line that defines it and you are done.

A widget block is the body of a widget. `@counter:` at top level defines a widget named `@counter`; `let @counter = @widget:` is the same thing; `\@widget:` is a lambda whose body is a widget block, which is how widget functions are written:

```prism
@Button: fn $Text -> @!Click = \@widget:
    !click <- @Element Ui.Button(label=$_)
    @ !click
```

A widget block is a Reflex `do` block with the monad left out: the statements are the sequence, `<-` is bind, `@` is `return`, and there is no `m` to name because there is only one. `@Button` takes a `$Text`, and the dynamic in the `label` field is what makes the button follow its label: the library's `@Button` is where the `dyn` lives, so a caller writing `@Button "Count: {$count}"` has no lift at the call site. The section on lifting over `@` spells out what that `dyn` is.

### Every statement form

The forms, in the order a reader meets them, each with what it builds:

`pattern <- widget` builds the widget as a child of this block's node and binds its result to the pattern. The pattern is usually a sigiled name. It may be a tuple or record pattern over a bundle:

```prism
((!lines, !eof)) <- @Lines
TextInput(!changed, $value) <- @TextInput "hello"
```

`widget` alone, as a statement, builds it and discards the result. `@Label "Count: {$count}"` on a line by itself is the common case; `@Element Time.Every(period=100)` is another, where the element exists to make the World come back and nobody wants the tick.

`pattern <- @Element e` is the bind form of the one widget that reaches outside: it creates a slot in this block's node, with a path under the node's, and its result is whatever the tag declares, `!Response`, `$State`, a pair of both, or `unit` for a presence element. `!click <- @Element Ui.Button(label="OK")` and `$args <- @Element Process.Args` are the two usual spellings; in a catch-all arm of an interceptor, where the tag is not statically known, the result is the generic `((!resp, ignore)) <- @Element _.element` and the types are checked per slot at runtime. The slot lives as long as the node and the element's fields are fixed for the slot's life; a dynamic in a field lifts the whole `@Element` into a `dyn`, which the lifting section below covers.

`pattern <- intercept @child, !responses, $supplier` binds an interceptor, which is a widget like any other: it builds `@child` in its own node and its result is the pair of the child's result and the child's element list, `((result, $list)) <- intercept @inner, !down, $supply`. Everything about what an interceptor can do with the list is chapter 6's; here it is one more bound statement whose child is owned by this node.

`let pattern = expression` binds a value of any kind without building anything. `let !delta = -1 on !decrement | +1 on !increment` names an event; `let $label = "Clicked {$count} times"` names a derived dynamic; `let @b = @Button $"OK"` names a widget, which is a value until bound. A `let`-bound widget can be bound later any number of times, and builds that many nodes.

`$x <- v then !e` is a hold. Its value before the first firing of `!e` is `v`; its change event is `!e`; its value after any occurrence in which `!e` fired is that firing's value. In the occurrence the block is built, `$x` is `v`, and if `!e` fires in that same occurrence `$x` changes to it then, which is what makes `@started`-driven initialisation behave the way it looks. Holds are statements, not expressions, because a hold is owned, and the `<-` makes the owner visible: the node whose block this is.

The last statement is the block's result. If it is a widget, the block's result is that widget's result. `@ v` makes a plain value the result; `@ ((!a, $b))` returns a bundle; `@ !never` is a block that returns an event which never fires, which the modal pattern below uses.

`match v:` with a widget in its arms is a statement when `v` is plain and every arm is a widget block, and it chooses which of several widgets this node builds. It is legal only where the scrutinee is plain, which inside a `dyn` or `for` block means the value the block was built for: `match mode: Browse: @BrowsePane … Edit: @EditPane …` under `dyn $state:` is the common form, and `match step:` in the wizard below is another. A `match` over a dynamic with a widget in an arm is rejected, for the reason the lifting section gives: choosing a widget by a changing value is construction, and construction over a dynamic is spelled `dyn`. `if c: @a else: @b` with plain `c` is the two-arm case of the same statement.

Two more things are statements in a widget block and are the subject of the rest of the chapter: `dyn $x:` and `for $m:`, each with a block of its own, each bound with `<-` like any widget, because each is one.

### Recursive bindings and groups

Bindings in a widget block are mutually recursive: a name may be used before the statement that binds it. The counter uses `$count` on the right of its own definition, and the todo list of the worked examples defines `$items` from `!changes`, `!changes` from `!rows`, and `!rows` from `for $items`. The restriction is the one time imposes: the cycle has to pass through a delayed edge, a tag, a `!Switch`'s hold of inner events, or a hold's value side. A cycle of prompt edges alone is a compile error that lists every binding in it. A `let`-bound function may call itself or its neighbours, because a call is not a cyclic value. Other plain values cannot be cyclic at all; a cycle among `let`s of plain non-function values is an error, because there is no occurrence for the cycle to be resolved across.

An implementation resolves the block into strongly connected groups and builds them in dependency order. A group with a cycle is built with placeholders for the forward references, which are closed once the group is built. The runtime's builder exposes this directly: `add_cyclic_event(name)` returns a forward event that fires when the event it is later closed with fires, and `close_cyclic_event_by_name(name, event)` closes it; `add_cyclic_dynamic` and its close are the same for a dynamic whose behavior is a computation that reads through the placeholder once set. The runtime's own `fold_dyn` is written this way and is a good picture of what the compiler emits for a self-referential hold:

```
(handle, both_in) = add_cyclic_event(name)
new_value         = both_in.filter_map(|(old, update)| f(old, update))
dynamic           = Dynamic::hold(initial, new_value)
both_out          = update.tag_map(|update, old| (old, update), dynamic)
close_cyclic_event_by_index(handle, both_out)
```

The hold reads its own old value through a tag, the tag is a delayed edge, and the placeholder is closed before the builder returns. All of this is invisible to the program: a widget block is a set of equations, and reading it top to bottom is one valid order.

The placeholder is why the node owns its cycles rather than the graph owning itself. The `ForwardEvent` and the set-once cell live in the node's `cyclic_events` table; the closed event refers to the placeholder, the placeholder refers to the closed event, and if the node did not hold both the pair would hold each other forever. The test `dropping_a_widget_frees_its_cyclic_loops` is the check that it does not.

### Build time and occurrence time

Every binding in a widget block is one of two things, and the compiler marks which. A *build-time* binding runs once, when the node is built: binding a child widget, a hold's initial value, a `let` of a plain value, the construction of an element's fields. An *occurrence-time* binding is a node in the propagation graph and runs whenever its inputs fire: anything lifted over an event or a dynamic. The sigils decide:

```prism
@report: fn Data -> @unit = \|data| @widget:
    let summary = Summarise data            # once, at build
    let $live = Summarise $feed             # once per change of $feed
    let !each = Summarise !chunk            # once per firing of !chunk
    @Label "{summary} / {$live}"
```

`Summarise data` with plain `data` runs once per build of `@report`, and never again, because nothing in it can change. `Summarise $feed` is a derived dynamic, recomputed in the occurrence `$feed` changes. `Summarise !chunk` runs per firing. There is no annotation to get wrong and no dependency array to maintain: the kinds of the operands are the dependency list.

This matters because people expect `let` to do something it does not. A `let` in a widget block is not a statement that executes in sequence each time "the component renders"; there is no render. If `data` is expensive to summarise, it is summarised once per node, and if you want it summarised again you need a new node, which is what `dyn` makes.

### Results and bundles

A widget's result is any value: an event, a dynamic, a plain value, a function, a widget, or a record of several. A record with sigiled fields is the bundle type:

```prism
type TextInput = record:
    !changed: Text
    $value: Text

@TextInput: fn Text -> @TextInput = \|initial| @widget:
    ...
    @ TextInput !changed, $value
```

Returning a widget from a widget (`@@A`) is legal and means the inner one is not built; the caller gets a blueprint and may bind it, which is how a widget function can return a widget for its caller to place. Returning a function is legal and plain. Results are read by the parent in the occurrence the child is built, and the rules of time say what they hold then: a result dynamic has the value the child's block gives it in that occurrence, a result event fires in that occurrence if the block's definition makes it fire, which under the creation rule means only if something pure like `@started` drives it.

### Widgets as arguments

A widget function can take a widget, and the sigil on the parameter says so:

```prism
@Framed: fn Text, @a -> @a = \|title, @child| @widget:
    @Label $title
    result <- @child
    @Label $"end of {title}"
    @ result
```

`@child` is a blueprint handed in; `result <- @child` binds it, which builds it as a child of `@Framed`'s node, between the two labels, with a path under `@Framed`'s. The caller wrote the recipe and `@Framed` chose where the dish goes. This is how containers are written, and it is the whole mechanism behind `intercept`, whose first argument is a widget for the same reason: the interceptor builds it, in its own node, and so owns it.

Two things follow from a widget argument being a value. `@Framed "A", @counter` and `@Framed "B", @counter` build two counters, two nodes, two sets of holds; the blueprint was shared and the dishes are not. And a widget function that takes a widget and never binds it builds nothing of it: `@Discard: fn @a -> @unit = \|@child| @ unit` is a legal program in which the child never exists. A widget passed to a function is not a child until it is bound; the bind is the ownership.

A widget function cannot bind the same argument twice and get one thing. `!a <- @child` followed by `!b <- @child` is two children. If the function wants one child and two views of its result, it binds once and names the result. If you find yourself wanting to pass a *node* rather than a widget, so that two containers could share one built child, that is the node value the language refuses, and the reason it refuses is that neither container would own it.

### `@main`

The program is a widget `@main` taking no arguments. The World builds it in occurrence zero. Its result type is the World's business: a test World may require `@unit` or a bundle it can assert on; a CLI World ignores it. Arguments and environment are not parameters of `@main`. They are state elements, `Process.Args` and `Process.Env`, with a value in the occurrence they are created, and the reason is the same reason the clock is a state element: they can change while the program runs, and an interceptor can confine or fake them for a subtree. A sandbox that gives its child a different argument list is a supplier function; a `@main` with parameters could not be sandboxed that way without a second mechanism.

### `@started`

`@started: @!unit` is a widget whose result fires once, in the occurrence in which it is built, and never again. It is pure, and it is definable in the language:

```prism
@started: @!unit = @widget:
    !once <- dyn $ unit:
        @ unit
    @ !once
```

A `dyn` over a constant dynamic builds its block once and fires its result event in that build occurrence; the block returns `unit`; the `dyn`'s event is an `!unit` that fires exactly then. The prelude provides `@started` as a primitive because it is used everywhere, and the runtime implements it as a filter over the builder's done event rather than by building a `dyn`, but it needs no semantics of its own.

The conformance suite checks the law, not the implementation: `@started` fires once, in the occurrence the node is built, after the block is built, and never again. The two spellings have to agree on the one observable that follows from "after the block is built": the firing lands at `height + 1` of the build, so a hold in the same block driven by it, `$x <- v then w on !started`, changes in the build occurrence, and anything in the block that reads `$x` through a tag in that occurrence sees `v`, while anything derived from `$x` sees `w`. A runtime whose done event fired at the build's own height, or in the next occurrence, would pass a test that only asked "did it fire once" and fail the one that holds on it; chapter 10's law test for `@started` holds on it.

Its use is kicking off a request as soon as a widget exists:

```prism
!started <- @started
!resp <- @Element Fs.File(path=path, requests=(Read on !started))
```

The request fires in the build occurrence; the response arrives in a later one. That is the whole of "load on mount" in Prism, and the difference from the hook you know is that it is visible: the read happens because this line says a request fires at start, and if the widget is rebuilt by a `dyn` the read happens again because `@started` fires again, which is in the text rather than in a lifecycle table. What `@started` is not: a primitive that makes creating a widget cause an event by itself. Its firing is construction, nothing outside the program observes it, and the creation rule stands. A `dyn` block that needs to act on its own birth uses `@started` inside the block, where no `!Switch` stands between it and the hold it feeds.

## `dyn`

You have written the view that is replaced when the mode changes, and you have written the bug that comes with it. The detail pane is rebuilt when the selection changes; the old pane had a subscription to a store; the subscription closed over the old item's id; the new pane subscribed too; now two handlers fire on every update and one of them writes a stale id into the model. The fix in every framework is a cleanup function, called from a lifecycle hook, in an order the documentation tells you not to depend on. The problem is that "replace this part of the program" was not a thing the language could say, so it was said with construction plus a convention for destruction.

`dyn` is the construct that says it. It takes a dynamic and a block, builds the block for the dynamic's current value, and rebuilds it whenever the dynamic changes, in the occurrence of the change. The old node is torn down at that occurrence's commit, and everything it owned, holds, slots, subscriptions, placeholders, goes with it. There is no cleanup function because there is nothing to clean up by hand; there is no hook because there is no moment at which code runs; there is a tree, and a part of it is replaced.

```prism
@DynLabelButton: fn $Text -> @!Text = \@widget:
    !!rendered <- dyn $_:
        !click <- @Element Ui.Button(label=_)
        @ _ on !click
    @ !Switch !!rendered
```

`dyn $x:` takes a dynamic and a block in which `_` (or a pattern, `dyn pattern <- $x:`) is the *plain* value of `$x`. Inside the block the label is `Text`, not `$Text`, because inside one build it never changes; a change makes a new build. Other dynamics in scope stay dynamic inside the block, and reading them there is ordinary lifting; only the dynamic named at the top triggers a rebuild. That distinction is the one to keep in your head: `dyn` is not "re-render when anything changes," it is "re-build when *this* changes," and everything else is live inside.

The pattern form is for structure:

```prism
!!views <- dyn ((mode, selected)) <- $state:
    match mode:
        Browse: @BrowsePane selected
        Edit: @EditPane selected
```

Both fields are plain inside; a change to either, since they are one dynamic, rebuilds.

### The result event

The result of a `dyn` is an event of the block's result: `!A` for a block returning `A`. It fires in every occurrence in which the block is built, the first build included, with that build's result. In the occurrence the `dyn` itself is built, its event fires in that occurrence; in every rebuild occurrence it fires again. The event goes to the caller. The block cannot see it unless the caller passes it back in, which, since the block's result is being defined, would be a cycle the block would have to close through a delayed edge like any other.

So `!!rendered` above is an event of events: each firing is a new build's `!Text`. In occurrence zero it fires with generation `#0`'s click-with-label; when the label changes in occurrence n it fires with generation `#1`'s. The parent needs one click event for all generations, and that is what `!Switch` is for, below.

Why does the result go up rather than being something the block owns? Because the parent is what persists across rebuilds and the block is what does not. A block that could see its own `dyn`'s event would be seeing an event whose firings include its own death; the parent, which outlives every generation, is the only thing in a position to hold the stream of generations as one value. Reflex's `dyn` and `networkView` return an `Event t a` to the caller for the same reason.

### What a rebuild resets

A rebuild replaces the node. Everything the old node owned is new in the new one:

Holds inside the block are new holds with their initial values. A `$count <- 0 then ...` in a `dyn` block starts at `0` again on every rebuild; this is either what you want (the pane's scroll position should reset when the item changes) or it is not, and the next subsection is about the second case.

Slots are new slots with fresh paths. A `dyn` adds a generation to the path of everything under it, `#0`, `#1`, `#2`, and a path is never reused: the generation counter belongs to the `dyn` node, only goes up, and is not reset by anything short of the `dyn` node itself being torn down and rebuilt by its own parent, at which point every path under it has a new generation of the parent's in front of it anyway. A World that was doing something on behalf of an old slot stops, because the slot is gone from the list; a response it had in flight for the old path is discarded, because the path is not in the list it last received.

Subscriptions inside the block are gone. An occurrence-time binding in the old block was a graph node owned by the old node; when the old node drops, its subscribers are dropped, and the inputs it listened to, which hold their subscribers weakly, deliver to it no more. Nothing was unsubscribed; something was dropped.

Cyclic placeholders are gone with the node that owned them.

A rebuild does not cut the old node off early. The old node keeps propagating until the occurrence commits, so an event that fired inside it in the rebuild occurrence is delivered to whoever was subscribed. A click on the old button in the occurrence its label changes is delivered from the old button, under the old label, to the `!Switch` that was still listening to it. Then commit, and the old node is dropped.

A timeline makes the rule concrete. Take `@clicker` from the worked examples, a button whose label counts its own clicks:

```prism
@clicker:
    !!clicks <- dyn $label:
        @Element Ui.Button(label=_)
    let !click = !Switch !!clicks
    $count <- 0 then $count + 1 on !click
    let $label = "Clicked {$count} times"
    @ unit
```

| Occurrence | Input | Inside | List after |
| --- | --- | --- | --- |
| 0 | build | generation `#0` built; `!!clicks` fires with its click; `!Switch` records it | `0/#0/0: Button("Clicked 0 times")` |
| 1 | click to `0/#0/0` | `#0`'s click fires; `!Switch` delivers; `$count + 1` fires with 1; `$label` changes; `dyn` rebuilds to `#1`; `!!clicks` fires with `#1`'s click; `!Switch` ignores it this occurrence; commit: `$count` is 1, `!Switch` adopts `#1`, `#0` dropped | `0/#1/0: Button("Clicked 1 times")` |
| 2 | click to `0/#1/0` | as above, to `#2` | `0/#2/0: Button("Clicked 2 times")` |

In occurrence 1 both generations exist for the length of the occurrence: `#0` because it has not committed out yet, `#1` because it was just built. Only `#0` can be clicked, because only `#0`'s path was in the list the World addressed. Only `#1` will be in the list the World reads next. There is no occurrence in which a button is on the screen that nothing listens to, and none in which two are listened to at once.

### When the new generation fails to build

You have seen the page that went blank. The new view threw halfway through its constructor; the framework had already unmounted the old one; the user got a white rectangle and a console full of red. The question a rebuild has to answer before anything else is what the tree looks like when the block it was asked to build cannot be built.

A `dyn` block can fail to build in two ways, and they are reported differently because they are different failures. The first is a cycle found during the build: the new block's prompt graph, together with what it subscribes to outside, has an action scheduled at or below the current height. That is kind `Cycle`, found `AtBuild`, and it aborts the whole occurrence: nothing commits, the dynamic that would have rebuilt the `dyn` keeps its old value, the old generation is still there, and the World is told, through its `errors` stream element if the program subscribed and through the run call's error return either way, which occurrence failed and which bindings were in the cycle. A cycle is a defect in the program's structure, not in its data, and a program with a cycle in one of its generations will have it on every rebuild to that generation, so there is nothing to be gained by containing it.

The second is a failure inside the construction that is not a cycle: a native component the block builds panics, a `for` key whose block fails, an `@Element` whose state tag no supplier and no World accepts. That is kind `Build`, with the owning node's path, and it is contained: the occurrence commits, the dynamic changes, the old generation is torn down at commit like any other, and *there is no new generation*. The `dyn` node has no child until its dynamic changes again. Its event does not fire for the failed build, so a `!Switch` over it keeps listening to the old generation's event, which never fires again because the old generation is gone, and a hold-of-dynamics joined over it keeps the old generation's last dynamic, frozen at its last value. The World sees the old generation's slots leave the list and nothing arrive. The error goes out through `errors` with kind `Build` and the `dyn` node's path as `owner`, in the same occurrence, so a program that subscribes to `errors` learns of the hole in the occurrence after the one that made it, which is the earliest anything from outside the program can be learned.

The `for` case is the same rule per key: a key whose block fails to build is torn down (nothing of it survives) and the consolidated result has no entry for it; the other keys built in the same occurrence are unaffected; the error names the `for` node and the key.

Why keep the old generation on a cycle and drop it on a build failure, rather than one rule for both? Suppose the old generation stayed on a `Build` failure. Then its slots would be in the list under a path whose generation the program has moved past, with holds reading a dynamic that has changed, drawing a value that is no longer the one the `dyn` is over: a screen showing yesterday's selection with today's model behind it. The block that failed was the program's statement of what should be there for the new value, and nothing else can be put there truthfully; a hole is the truthful thing. Suppose instead a cycle were contained. Then the occurrence would have to commit with half its propagation done, since the cycle is found mid-propagation and the actions already run cannot be unrun without rollback, which is exactly what aborting is. The two rules are one rule, "commit what is true or commit nothing," applied to a failure that can be isolated to a node and one that cannot.

Bound by this: the runtime, which must not fire the `dyn`'s event for a failed build and must tear the old generation down anyway; the World, which must report what the run call returns and may pass back `quarantine` paths on its next call if it wants a failing subtree held out of further rebuilds; and the program, which cannot observe a failed build except through `errors` and the list, because there is no event for it. Chapter 9 has the error type and the quarantine protocol in full.

### State that must survive goes outside

The mistake every newcomer makes once is putting a hold inside a `dyn` and expecting it to persist. `Styled: fn Theme, Text -> Text` here is a plain function that decorates a label for a theme:

```prism
@wrong: fn $Theme -> @unit = \@widget:
    dyn $_:
        !inc <- @Button $(Styled _, "+")
        $count <- 0 then $count + 1 on !inc          # resets on every theme change
        @Label (Styled _, "{$count}")
    @ unit
```

Change the theme and the count is zero. The `dyn` is over the theme; the block is rebuilt for a new theme; the hold is in the block; the hold is new. The program says exactly this, if you read it as a description of ownership rather than as code that runs: `$count` belongs to the generation. The fix is to move the hold to the node that persists:

```prism
@right: fn $Theme -> @unit = \@widget:
    !!incs <- dyn $_:
        !inc <- @Button $(Styled _, "+")
        @ !inc
    let !inc = !Switch !!incs
    $count <- 0 then $count + 1 on !inc
    dyn $_:
        @Label (Styled _, "{$count}")
    @ unit
```

Now the button is rebuilt per theme and the count is not. `$count` is read inside the second `dyn` as a dynamic, because it is one from outside, and the label's `dyn` rebuilds on the theme, while the label's text, derived from `$count`, changes live inside each generation (and, since a label's text is a plain field, lifts into a `dyn` of its own per the rule below; the walkthroughs write that out). The rule of thumb: a `dyn` block owns what should die with the value it was built for, and nothing else. If you find yourself wanting a hold to survive a rebuild, it is not the rebuild's hold.

### What a rebuild costs, and what to keep out of one

A rebuild costs what the block builds, and nothing else: no diff against the old generation, no reconciliation, no patching of a retained tree. The old node is dropped and the new one is built. For a label this is one slot; for a pane with a hundred rows it is a hundred blocks, and a `dyn` around that pane over a dynamic that changes per keystroke is a hundred blocks per keystroke. The program says what the `dyn` is over, so the program says what the cost is, and the way to make it smaller is always the same: narrow the dynamic. A `dyn` over `$state` rebuilds on every change of any field; a `dyn` over `$state.mode` rebuilds on mode changes and reads the rest live.

Fresh paths are the other cost, and the one a World notices. Every slot under a rebuilt generation has a new path, so a World that keeps anything by path, a native toolkit widget or an in-flight request, loses it. Focus is the exception by design: a `Gui.Focusable` has a `name` and no other field, and the GUI library holds which name is focused, so a button whose label is a `dyn` is a new slot per label change and keeps focus, provided its name does not change with the label. Where a native widget really must keep its identity, the same split applies: the element that must keep its identity is given no dynamic fields and is created once, and the thing whose fields change is a separate element that nobody needs to address. That split is available for every element and it is the reason dynamic fields are allowed to mean "a fresh slot" rather than something more complicated.

Library widgets take dynamic arguments, so a program normally writes `@Button $label` and never spells the `dyn` or the fresh slot. The fresh slot per change belongs to the library: `@Button` builds a `Ui.Button` element, the library's interceptor answers it, and a relabelled button is a new slot in that interceptor's list, which a World that sees `Gui.*` only never observes. The fresh-slot-per-change rule holds for elements that reach a World; `Gui.Window` or `Fs.Contents` with a dynamic field is a new slot in the World's list, and a `Ui.Button` with one is a new slot in the interceptor's. The button that relabels itself in this chapter's examples illustrates the rule and is not how a program normally labels a button.

### A `dyn` built in the middle of an occurrence

A widget built in occurrence n behaves as though it had been built immediately before n's inputs arrived. For a nested `dyn` this has one consequence that an implementation can get wrong and the specification pins: a `dyn` built in n, over a dynamic that changed in n, builds *once*, with the new value. It does not build with the old value and then rebuild.

```prism
@outer: fn $Mode, $Text -> @unit = \|$mode, $text| @widget:
    dyn $mode:
        dyn $text:
            @Label $(Render _, mode)       # mode is plain here; _ is the text; Render: fn Text, Mode -> Text
    @ unit
```

Suppose `$mode` and `$text` both change in occurrence n, because one input moved both (a hold over a record, say, that the `dyn`s project). The outer `dyn` rebuilds in n. Its new block binds an inner `dyn` over `$text`. That inner `dyn`, at the moment it is built, subscribes to `$text`'s change event and finds that the event has already fired in this occurrence, with the new text; so it builds its block with the new text, once, and its result event fires in n with that build. Had it read `$text`'s value through its behavior side it would have seen the old text, built a label with it, and been wrong until the next change of `$text`, which might be never.

The runtime does this with the Spider-style catch-up: every event caches its current-occurrence value, `subscribe` returns it if the event has already fired, and the `DynamicWidget` takes the initial widget from that cached firing if there is one and from the behavior otherwise. The test `a_nested_dyn_built_in_the_occurrence_its_dynamic_changes_shows_the_new_value` pins it. Its companion, `a_closure_runs_once_per_occurrence_even_when_late_subscribers_read_it`, pins the other half of the rule: a late subscriber reads the cached value, and the closure that produced it is not run a second time, so the at-most-once law holds for everything a mid-occurrence build touches.

### What the runtime promises for `dyn`

Three promises, each the runtime's, each with the test that pins it; chapter 11 has the `DynamicWidget` that keeps them.

The rebuild is an ordinary action at an ordinary height, one above the dynamic's change event, and the new generation is built at a height above that, so that everything the new block fires at birth fires after the block exists. A cycle through a `dyn` is therefore caught by the same comparison as any other cycle, an action scheduled at or below the current height, and no second detector exists for rebuilds.

The old node is dropped by an action at the commit height, not by a destructor in the middle of propagation, so nothing it owned can vanish while something at a lower height is still delivering to it. Nothing is notified when a node goes: the only way anything outside the program learns of it is the list it reads afterwards, so the drop action has no callback. `a_replaced_child_keeps_working_until_its_last_occurrence_ends` is the check.

The `dyn`'s event is the leftmost of a rebuild event and the first build's done event, and the done event fires at `height + 1` of the build, which is why the first firing lands in the build occurrence and after the build has finished. A `dyn` built after its dynamic has already changed in the occurrence rebuilds once and in order, by deferring the same action to `max(height, current + 1)`; the nested-`dyn` test above is the check.

### `dyn` is not a delay

A cycle through `dyn` alone is rejected. The `dyn`'s event fires in the occurrence of the rebuild, promptly, so it is a prompt edge, and a program like

```prism
@bad:
    !r <- dyn $x:
        @ _ + 1
    $x <- 0 then !r                  # error: the cycle through dyn has no delayed edge
```

has no delayed edge anywhere: `$x` changes, the `dyn` rebuilds in that occurrence, its event fires in that occurrence, the hold's change event fires in that occurrence, which is `$x` changing again, in the same occurrence. The compiler rejects it with both bindings named. The reason `dyn` is not made a delay, which would make this legal and a tick behind, is that every other rebuild would then be a tick behind too, and the new node would see the occurrence it was born in as the past. The delayed edges are reads of a dynamic as it stood when the occurrence opened, and there are three of them; a rebuild is not a read.

## `!Switch`, `$Join`, and why they differ

A `dyn` returns an event per generation. The parent wants one event, or one dynamic, across all of them. Two primitives do that, and they are not symmetric, and the asymmetry is one of the few places where the language chooses something that looks like an implementation convenience and is not.

### `!Switch`

The clicker has `!!clicks`, one click event per generation of its button, and it wants to count clicks. It cannot subscribe to a generation's click event by name, because the generation is not a value and the next one will have a different event; it cannot wait for the stream of generations to end, because it never does. It wants one event that is "the current button's click, whichever button that is," and `!Switch` is that event.

`!Switch: fn !!A -> !A`. At occurrence n it delivers the firing, if any, of the inner event that the outer event most recently delivered *before* n. It starts subscribed to nothing. It is not prompt: an inner event delivered at n is heard from n+1.

Which inner event `!Switch` is listening to is a hold, a hold of events, and like every hold it is read on its value side, before the occurrence. That is the delayed edge that makes the clicker's cycle legal: `$count` depends on `!click`, `!click` is the switch, the switch's choice of inner event was made before the occurrence, and so the cycle `$count → $label → dyn → !!clicks → !Switch → !click → $count` passes through a read of something as it stood when the occurrence opened.

The consequence for `dyn` is that whatever a newly built block fires in its creation occurrence is swallowed at the `!Switch`. The parent does not hear it. Under the creation rule this costs nothing anyone wanted: a slot cannot be addressed in the occurrence it was created, so no click, no response, no key can fire inside a fresh block in its birth occurrence. The only things that can fire there are pure, which means `@started` and things derived from it, and a block that needs to act on its own start uses `@started` *inside* the block, where no switch is in the way. The parent, for its part, already knows about the birth, because the `dyn`'s own event fired.

Suppose `!Switch` were prompt. Then in the clicker, in occurrence 1, the new generation's click event would be listened to from the moment it was built, in that same occurrence. Its height would have to be known before the rebuild action that creates it has run, because `!click`'s height is what `$count + 1 on !click` sits above; so either heights are dynamic in the middle of propagation and every node above a switch re-sorts itself mid-occurrence, or the switch's height is an upper bound guessed in advance and wrong when the inner event turns out to be taller. Reflex has `coincidence` for exactly this, and its documentation points you at `switchHold never` instead. A non-prompt switch has a fixed height, one above the outer event, and the comparison that catches a cycle is one comparison.

### `$Join`

A file viewer has a path the user edits and wants to show the file's contents. `Fs.Contents(path=p)` is a state element, so one slot is one dynamic of contents; the path changes, so the slot is inside a `dyn`; and the `dyn` returns an event of dynamics, `!$Contents`, one per path. The label wants one dynamic, "the contents of whatever file the path names now," and it wants it with no gap: in the occurrence the path changes, the new slot has a value (the oracle answered as the occurrence opened), and the label should show it then, not an occurrence later.

```prism
!$contents <- dyn $path:
    @Element Fs.Contents(path=_)
$$current <- @HoldFirst !$contents
let $contents = $Join $$current
```

`$Join: fn $$A -> $A`. At occurrence n its value before n is the value before n of the inner dynamic current before n. Its change event fires at n, with the value after n, if the outer dynamic changes at n (to a new inner, whose value after n is taken) or if the current inner changes at n. It is prompt, like every derived dynamic.

The hold needs an initial dynamic, and the first build's result is the one to use, because the first build happens in the occurrence the `dyn` is built: the `dyn`'s event fires then, the hold adopts the firing in its build occurrence, and `$Join` sees the adopted inner in that same occurrence. `@HoldFirst: fn !a -> @$a` is the prelude's name for that hold, and it is the idiom: a hold written by hand with a placeholder initial dynamic, `$ Unavailable <= !$contents`, has the placeholder as its value for the length of one tag read in the build occurrence, and nothing is gained by having a value there that no build produced. `@HoldFirst` has no semantics beyond what it expands to, a hold whose initial value is the first firing of its event in the occurrence the hold is built, and the lifting section below uses it for exactly that.

This is also the canonical cache idiom: a hold of dynamics, chosen between a live one and a frozen one, joined. A hold of plain values copied out of the live dynamic is the simpler construction, and it has a one-occurrence gap at creation that the join version does not, because a join switches to the new inner's *current* value in the occurrence of the switch rather than waiting for the new inner to fire. Chapter 2's cache is the join form, with the hold of dynamics around the forwarded query.

### The asymmetry

Why is one prompt and the other not? Because a dynamic always has a value to switch to, and an event does not. When `$$current` changes at n to a new inner dynamic, that inner has a value after n (every dynamic does, at every occurrence), and `$Join` can fire with it promptly without asking anything to fire first. When `!!clicks` changes at n to a new inner event, the question "did the new inner fire at n" would have to be answered by an event that was being created during n, at a height nobody knew when the switch's height was fixed. The join never needs a firing from the new inner; it needs a value, and values are always there. The switch would need a firing, and firings have heights.

The two together, then: results that are events cross a `dyn` with a one-occurrence lag that the creation rule makes unobservable, and results that are dynamics cross it with no lag at all. A derived dynamic that lagged would be a glitch, and ruling out glitches is why `$Join` must be prompt; an event that was switched promptly would be a height cycle in every interesting case, and ruling out height cycles is why `!Switch` must not be.

### What the runtime promises for the two

`!Switch` reads its inner event from a hold's value side, before the occurrence; its height is fixed at one above the outer's. `$Join`'s change event is the leftmost of the outer's change mapped to the new inner's value after n and the non-prompt switch over the current inner's change event; its height is dynamic, since inner events have arbitrary heights. Both are the runtime's to keep, and chapter 11 has `switch_hold` and `Dynamic::join`.

The harness tests three cases for join: the inner changes alone; the outer swaps to an inner that changed in the same occurrence, where the change event must deliver the new inner's *new* value and the behavior must agree afterwards; and the outer swaps to an inner that does not change, where the change event must still fire, with the new inner's current value, because the joined dynamic changed even though nothing inside it did. The third is the one an implementation that only listens to the inner's change event gets wrong. For the switch, the law test is the swallowed birth: a block that fires at its parent through `!Switch` in its creation occurrence is not heard, and is heard from the next occurrence on.

## Lifting over `@`

The lifting rule says that an operation over dynamics is a dynamic. What is a widget over a dynamic? `@PlainButton $label`, where `@PlainButton: fn Text -> @!Click` takes a plain label, applies a widget function to a dynamic; `@Element Ui.Label(text=$t)` puts a dynamic where a plain field goes. Both are applications of something that builds to something that changes, and the answer is the one thing the language has for "build again when this changes."

**Binding a dynamic of widgets lifts to `dyn`.** `!x <- @PlainButton $label` means `dyn $label:` around the build, with the bound name being the `!Switch` of the `dyn`'s event:

```prism
!x <- @PlainButton $label

# is

!!xs <- dyn $label:
    @PlainButton _
let !x = !Switch !!xs
```

The lift happens only where the parameter wants a plain value and gets a dynamic. The library's `@Button: fn $Text -> @!Click` takes the dynamic itself and holds the `dyn` inside its body, which is why `@Button $label` has no lift at the call site and the same node structure as the expansion above; the two spellings differ only in which widget's block the `dyn` is written in, and chapter 11 states the canonical node structure both compile to.

For a widget whose result is a dynamic, the bound name is the hold-of-dynamics joined, with the first build's result as the hold's initial value:

```prism
$s <- @Status $url

# is

!$ss <- dyn $url:
    @Status _
$$cur <- @HoldFirst !$ss
let $s = $Join $$cur
```

`@HoldFirst` is a hold whose initial value is the firing of `!$ss` in the occurrence the hold is built, which exists because a `dyn` fires in its build occurrence and a widget built in n sees all of n. No hold written with `then` can be seeded that way, since a hold's initial value is an expression and the first build's result is not in scope until the `dyn` has been bound; this is why the prelude has it, and why it is the idiom wherever a program holds a `dyn`'s event. It expands to the runtime's `hold_first` over the event, and chapter 12 prints it as `HoldFirst` in the typed AST.

For a plain result, binding gives a dynamic: `$x <- @W $d` is `@HoldFirst` over the `dyn`'s `!A`, so `$x` is the current generation's result, held from the first build's value and changing in every rebuild occurrence. A plain result of a rebuilt widget is a value per generation, and a value per generation held is a dynamic. For a bundle, field by field by the kind of each field: an event field switches, a dynamic field joins, a plain field holds.

An element with a dynamic where a field goes is the same thing one level down: `@Element Fs.Contents(path=$p)` is `$(@Element Fs.Contents(path=p))`, a dynamic of widgets, which is a `dyn` with a fresh slot per change of `$p`. This is why the element list never contains a sigil and why the World never diffs anything but plain data: a label whose text changes is, to the World, a new label at the same tree position, and the generation in its path says so.

Walkthroughs write the expansion out only when the generations are the point.

One thing that does not lift on its own is a tuple. Functor values nest inside plain aggregates, so `(($step, $answers))` is a plain pair holding two dynamics, not a `$((Step, Answers))`, and `dyn` over it is a type error. To make one dynamic of two, lift a plain function over them: `Pair: fn a, b -> ((a, b))` is in the prelude, and `Pair $step, $answers` is a `$((Step, Answers))` that changes when either does, once per occurrence however many of its operands moved. The wizard and the tabs interceptor below both write it.

What does *not* lift. A widget over an event is not a `dyn` and not anything else: `@Button !label` is a type error. A widget per firing of an event would be a node with no owner's decision to replace it, built in the occurrence of the firing and torn down when? Never, or at the next firing, and both answers are a lifecycle rule the language does not have. The program that wants a widget per event holds the event into a dynamic first, which says when the old one goes. Likewise `match $x:` with a widget in any arm is rejected: choosing a widget by a changing value is construction, and construction over a dynamic is spelled `dyn`, so that the rebuild is visible where it happens.

## `for`

You have had the afternoon where deleting item 3 cleared the expanded state of item 4. The list was an array; the framework matched rows to data by position; when item 3 went, item 4 was now at position 3, and the row that had been at position 3, with its expanded flag set, was handed item 4's data. The fix was a `key` prop, and the fix after that was a lint rule telling you not to use the index as the key, and the one after that was a conversation about where keys come from. The language's answer is that there is one collection for anything that matters, it is a map, and its keys are the identities.

```prism
!updates <- for id, $item <- $items:
    !checked <- @Checkbox $item.checked
    @ ItemUpdate id, !checked
```

`for k, $v <- $m:` over a dynamic map `$(K ==> V)` builds one copy of the block per key. Inside, `k` is the key, plain, because a key never changes for the life of its block; and `$v` is the entry's value *as a dynamic*: a change to a value that keeps its key updates `$v` and does not rebuild the block. `$item.checked` is ordinary lifting over that dynamic. The binders are ordinary patterns, named by the loop. (The loops inside `update` blocks, chapter 16, bind the reserved indices `ix`, `jx` and `kx` instead.) Most blocks need no names, so the shorthand `for $m:` binds `key` and `$_`, and the examples below use it unless the names help. A key that appears in occurrence n has its block built in n, under the rule for mid-occurrence construction; a key that disappears in n has its block torn down at n's commit, like a `dyn`'s old generation. The parent sees the new key's result in n; the new block does not see itself.

That last clause is the same fact as `!Switch` swallowing a block's birth, seen from the collection side. The consolidated event of a `for` is a merge over the blocks; a block born in n contributes to the merge from n, and so its parent can hear it in n if it fires in n, which only pure code can make it do; but nothing inside the block is downstream of the consolidation, because the consolidation is the block's own result.

### Keys as identity

A key decides three things. Which block survives a change: an entry whose key is still present after a change keeps its node, and an entry whose key is gone loses it. Which element paths stay stable: a `for` adds its key to the path of everything under it, `1/[7]/0/#0/0`, so a button in row 7 keeps its path while rows 3 and 12 come and go, and the World keeps addressing it. And which hold keeps its state: a `$expanded` inside the block belongs to the key's node and lives exactly as long as the key does, through every change to its neighbours.

Keys are ordered, because maps are, so the element list has a defined order, which is tree order, which under a `for` is key order. A list that should display in some other order sorts by making a map whose key sorts that way.

### Consolidation by result kind

Every row has a Toggle button and a Delete button, and the program above the list wants to apply what the user did. Two rows can be clicked in one occurrence (the World batched two pointer events into one turn, or a test put them on one line), and the program above the `for` wants to hear about both at once, with no order between them that it did not ask for:

```prism
!rows <- for $items:
    !toggle <- @Button $"Toggle"
    !delete <- @Button $"Delete"
    @ Toggle on !toggle | Delete on !delete
```

`!rows : !(K ==> Change)`. When row 3's Toggle and row 9's Delete are clicked in one occurrence, `!rows` fires once, with `{3: Toggle, 9: Delete}`. When nothing in any row fires, `!rows` is quiet. The program above the `for` gets to say what simultaneous changes to several rows mean, in one place, by folding the map; it never sees an order between rows that the language did not promise, because there is none.

A `for` has one result, not a map of results, and what it is depends on what the block returns:

| Block returns | `for` returns | Meaning |
| --- | --- | --- |
| plain `A` | `$(K ==> A)` | a dynamic map of the blocks' values, changing when a key appears or disappears |
| `!A` | `!(K ==> A)` | fires in any occurrence in which at least one block's event fires, with exactly the entries that fired |
| `$A` | `$(K ==> A)` | a dynamic map whose value is every block's current value, changing when any block's dynamic changes or the key set changes |
| `@A` | by the inner kind | the blocks are built as children; their results are consolidated by these rules |
| `unit` | `$(K ==> unit)` | the key set as a dynamic; the compiler elides it when nothing binds the `for` |

Consolidation pushes the map as deep into the result as these rules allow, and a record result is consolidated field by field: a block returning `record: !changed: Text; $value: Text` gives a `record: !changed: K ==> Text; $value: K ==> Text`, one map in each field, because a map of records of two functors is less useful than a record of two maps and the rules can always get there.

The `@A` row is the one that looks like a special case and is not. A block whose last statement is a bound widget has that widget's result as its own, so a `for` whose rows end in `@Row $_`, with `@Row: fn $Item -> @TextInput` returning the bundle `record: !changed: Text; $value: Text`, consolidates exactly as if the block had returned the bundle itself:

```prism
TextInput(!changes, $values) <- for $items:
    @Row $_
# !changes : !(K ==> Text)   fires with the rows whose inputs changed this occurrence
# $values  : $(K ==> Text)   every row's current text
```

The row is a child of the `for` node per key, its slots sit under `[k]` in the path, and its two result fields are consolidated field by field into two maps. Nothing about the inner widget being a function application changes what consolidation does; the kind of the result is what is consolidated, wherever the result came from.

The remaining kinds, with an example each. A plain result is one that depends on nothing live, which inside a `for` means on `key` alone:

```prism
$ids <- for $items:
    @ key                        # $(K ==> K): changes only when the key set does
```

A `unit` result is the common case of a row that only draws, `for $items: @Label $_.title`, and consolidates to the key set, `$(K ==> unit)`, which is a dynamic nobody reads; a `for` statement that is not bound has its result elided by the compiler, so a display-only list costs no consolidation node.

A dynamic result:

```prism
$texts <- for $fields:                        # $fields : $(((Nat, Text)) ==> Field): id and initial text in the key
    let ((id, initial)) = key                   # plain, for the life of the block
    TextInput(ignore, $v) <- @TextInput initial
    @ $v
```

`$texts : $(((Nat, Text)) ==> Text)`, the current text of every input, changing when any input's text changes or a field is added or removed.

The mistake to make once is in that last example. Had the initial text been in the entry, `@TextInput $_.initial` would apply a widget function that takes plain text to a dynamic, which lifts to a `dyn`, and the input would be rebuilt, cursor and all, every time the entry changed. The compiler accepts it, because it is a meaningful program; it is just not the one you wanted. The question "is this plain, dynamic or an event inside my `for`" is the one the sigils answer: `key` is plain for the life of the block, `$_` is live, and anything that should be fixed at the block's birth belongs in the key or is read once through a hold that nothing else moves.

### Consolidation is prompt

A key that appears in occurrence n is built in n, and anything its block fires in n is in the consolidated event in n. Only pure code can fire in a block's birth occurrence, and the thing a block fires at birth is a request from `@started`:

```prism
!requests <- for $watched:
    !started <- @started
    @ (Watch key) on !started
```

A key added in n makes `!requests` fire in n with `{new_key: Watch new_key}`, consolidated with whatever other blocks fired, and the parent, which perhaps feeds `!requests` into an element's `requests` field, sends it out in n. The new block, had it been able to see `!requests`, would see its own birth; it cannot, because nothing in the block is downstream of the block's result, and a program that passes `!requests` back into the `for` has to close that cycle through a delayed edge like every other.

### A walkthrough

Take the todo list from the worked examples, reduced to what matters for `for`. `Describe: fn Item, Bool -> Text` renders a row's label from the item and its star, and `StepAll: fn Nat ==> Item, Nat ==> Change -> Nat ==> Item` applies a map of changes to the items; `!Union: fn !(k ==> v), !(k ==> v) -> !(k ==> v)` merges two map events entry-wise, firing when either does:

```prism
@todos:
    !add <- @Button $"Add"
    $next <- 0 then $next + 1 on !add
    let !adds = {$next: Add "item {$next}"} on !add
    !rows <- for $items:
        !delete <- @Button $"Delete"
        !star <- @Button $"Star"
        $starred <- False then not $starred on !star
        @Label (Describe $_, $starred)
        @ Delete on !delete
    let !changes = !Union !adds, !rows
    $items <- {} then StepAll $items, !changes
```

**Occurrence 0.** `$items` is `{}`; `for $items` builds no blocks. The list has one button.

**Occurrence 1, Add.** `!adds` fires with `{0: Add "item 0"}`; `$items`'s change event fires with a one-entry map; `for` sees key `0` appear in this occurrence and builds its block now: two buttons at `1/[0]/0/#0/0` and `1/[0]/1/#0/0`, a hold `$starred` at `False`, a label at `1/[0]/2/#0/0`. Commit.

**Occurrence 2, Add.** Key `1` appears; its block is built. Key `0`'s block is untouched: not rebuilt, not re-anything. Its `$starred` is whatever it was.

**Occurrence 3, Star on row 0.** Inside row 0's block, `!star` fires and `$starred` becomes `True`; the label's text changes, so its `dyn` rebuilds the label slot to `1/[0]/2/#1/0`. The row's result event, `Delete on !delete`, is quiet; `!rows` is quiet; `$items` does not change; the `for` does nothing at all.

**Occurrence 4, an edit arrives for item 0 from elsewhere** (a push, say, or another button outside the `for` that changes the label). `$items` changes; key `0` is still present; so row 0's `$_` changes, its label text changes, the label rebuilds, and `$starred` is still `True`, because the block was not rebuilt. The row's buttons keep their paths.

**Occurrence 5, Delete on row 0.** `!rows` fires with `{0: Delete}`; `StepAll` removes key `0`; `for` tears row `[0]`'s block down at commit. Row `[1]` is where it was, with its paths and its star.

Nothing in this walkthrough involved the program telling the `for` which rows had changed. It changed the map. The `for` compared keys.

### Arrays are second class

`for` over a dynamic array `$[V]` is keyed by index, and the index is the key in every sense: the block's `key` is a `Nat`, the path segment prints as `[3]` exactly as a `Nat ==> V` map's key `3` would, and a transcript does not distinguish the two, because there is nothing to distinguish; a `for` over an array is a `for` over the map from index to element, and the paths say so. It is allowed, and it is the wrong tool for anything with identity: an insertion at the front changes every later key, which rebuilds every later block and resets its state, and the World sees every later path vanish and reappear. That is the array-index-as-key bug, built in, and the language keeps it only because positional data exists, a row of three fixed columns, a list of coordinates, where the index is the identity and nothing else is.

Programs that need positional display of identified items keep a map with allocated ids and sort for display. The idiom, with `Insert: fn k ==> v, k, v -> k ==> v` and `NewItem: Item` a blank item:

```prism
@list:
    !add <- @Button $"Add"
    $next <- 0 then $next + 1 on !add                        # the id allocator
    $items <- {} then (Insert $items, $next, NewItem) on !add  # Nat ==> Item, keyed by id
    !rows <- for $items:
        ...
    @ unit
```

`$next` is a counter that only goes up; an id is never reused, so a key that returns is a new key, which is right. That is the program's half of an invariant whose other half is the runtime's: a key that leaves a `for` and comes back gets a new node, with fresh holds and fresh paths under `[k]`, however short the absence, because the old node was torn down at the commit of the occurrence the key left, and the runtime keeps no table of departed keys to revive. The program's allocator makes the two halves agree: with ids that never repeat, "the same key came back" cannot happen, and the question of whether a returning key should keep its state never arises. A program that reuses keys gets a new node per return anyway, and the runtime's promise is what makes that predictable rather than dependent on how long the key was gone.

When the display order is not id order, make a map whose keys sort the way the display should, with `ReKey: fn k ==> v, (fn k, v -> j) -> j ==> v`:

```prism
let $ordered = ReKey $items, \|id, item| ((item.rank, id))    # $(((Nat, Nat)) ==> Item)
!rows <- for $ordered:
    let ((rank, id)) = key
    ...
```

The key is a tuple of rank and id, so two items with the same rank stay distinct and sort by id, and an item whose rank changes gets a new key and a new block, which is what moving it is. An item whose rank does not change keeps its block through everything else. If re-ranking an item should keep its block, the rank does not belong in the key, and the display order comes from the World or an interceptor reading `rank` out of the elements it lays out; both are ordinary programs, and which is right depends on whether the thing with the state is "the item" or "the item at this rank."

Suppose instead the language gave arrays an opaque per-element identity, minted when an element is created and following it through inserts and removals. That is what `for` over an array would need to be first class. The identity would have to be minted somewhere, which means every array operation in the plain-value layer would have to preserve a hidden field that plain values do not have, or the language would need a second array type with identities, which is a map with allocated keys under a different name. The allocated-id map is that second type written in the language, with the allocation visible, and so it is the one the language recommends and the one the todo list uses. Reflex offers both: `listWithKey` over a `Map` and `simpleList` over a list.

### Nested `for`

A `for` inside a `for` written with the shorthand has its own `key` and its own `$_`, and the outer ones are shadowed, like `_` in a nested lambda. Nesting reads better with named binders, `for row, $r <- $rows:` around `for col, $cell <- $r.cells:`, because both keys stay in scope and the path says `[row]/[col]`. With the shorthand, name the outer key with `let` before the inner `for`.

The runtime's promise for `for` is the `dyn`'s promise per key: a key that appears is built in the occurrence of the change, at a height above the map's change event, and a key that disappears is torn down at commit by the same keep-until-the-end action; `$_` is a per-key dynamic projected from the map, so a change to an entry that keeps its key reaches its block without rebuilding it; and the consolidated event's membership is the blocks as the occurrence opened plus the blocks built in it. Chapter 11 has the keyed list widget that keeps all three.

## What switching may not do

No switching construct creates an occurrence or defers to a later one. A rebuild happens in the occurrence of the change. A key is built in the occurrence it appears. A `dyn`'s event fires in the occurrence of the build. If what the new node wants is something from the World, it asks through an element, in the build occurrence if it likes, and is answered in a later one, and there is no way for a rebuild to "wait for" that answer other than the way every program waits: by holding the response when it arrives and typing the gap.

This is why `dyn` is not a delayed edge and why a cycle through `dyn` alone is an error. It is also why there is no `Defer` in the language: `Defer` is an element, the World's, and the only thing it means is "answer this in a later occurrence of this turn," which a rebuild can ask for like anything else and cannot do by itself.

And nothing is ever triggered by a node appearing or disappearing. A `dyn` that tears a generation down does not fire a "destroyed" event; a `for` whose key vanished does not fire "removed." The parent knows, because the parent made the change to the dynamic that caused it. The World knows, because the element list it reads is different. If a program wants something done when a part of it goes, the part fires a request before it goes, from the same event that causes it to go, and the request is answered later; a draw list a GUI World reads is description, not a trigger. Exit is a request element with a request event, and a shell that wants to know a hosted program has ended reads the list and sees its elements are not in it.

## Patterns

Five constructions that every interactive program has some version of, each written in the language and then run occurrence by occurrence with attention to one question: what rebuilds when. The answer is always readable from where the holds are and what the `dyn`s and `for`s are over. The GUI vocabulary and the path notation are those of the worked examples: `Ui.Button(label)` responds with `click`, `Ui.Label(text)` is a library tag the GUI library turns into `Draw.Text` in a `Gui.Scene`, a dynamic in a field is a `dyn` with a generation in the path, and `@Button`/`@Label` are the library widgets that write that `dyn` for you. A few plain helpers are assumed and given a signature where they are used.

### Master-detail

A list on the left, the selected item's detail on the right. The detail should rebuild when the *selection* changes and only then; an edit to the selected item should update the detail in place.

```prism
type Item = record:
    title: Text
    body: Text

Keys: fn k ==> v -> [k]                        # in key order
FirstKey: fn k ==> v -> ?k = \|m| (Keys m)[0]    # Nil on an empty map
LastKey: fn k ==> v -> ?k
Gone: fn k ==> v, ?k -> ?unit                   # Some unit when the key is Some and absent from the map

let ShowBody: fn ?Item -> Text = \|i| match i:
    Some item: item.body
    Nil: "(no such item)"

@Detail: fn $?Item -> @unit = \@widget:
    @Label (ShowBody $_)
    @ unit

@master_detail: fn $(Nat ==> Item) -> @unit = \|$items| @widget:
    !picks <- for $items:
        !click <- @Button $_.title
        @ key on !click
    $selected <- Nil then Some (!Filter (FirstKey !picks))
    dyn $selected:
        match _:
            Nil: @Label $"Select an item"
            Some id: @Detail $items[id]
    @ unit
```

Three things are live and they are at three different places. `$items` is the model and comes from outside. `$selected` is a hold in `@master_detail`'s own node, outside both the `for` and the `dyn`, so it survives everything. The `dyn` is over `$selected` alone; inside it, `id` is plain and `$items[id]` is a lifted lookup, a `$?Item` that changes whenever `$items` changes, read live by `@Detail` without rebuilding anything but the label's slot. `FirstKey` picks the lowest key when two rows are clicked in one occurrence, and that choice is written down.

**Occurrence 0.** `$items` has, say, keys 1, 2, 3. The `for` builds three blocks, a button each. `$selected` is `Nil`; the `dyn` builds generation `#0`, the "Select an item" label.

**Occurrence 1, click row 2.** Row 2's block fires `2`; `!picks` fires with `{2: 2}`; `FirstKey` gives `Some 2`; `!Filter` passes `2`; `$selected`'s change event fires with `Some 2`. The `dyn` rebuilds: generation `#1` is built with `id = 2`, `$items[2]` is `Some (Item "Two", "...")`, `@Detail` builds a label with the body. Generation `#0` is dropped at commit. What rebuilt: the detail, once. What did not: the three rows.

**Occurrence 2, the model changes item 2's body.** `$items` changes; key 2 is still there, so row 2's `$_` changes and its button's label slot rebuilds (the derived title fires its change event even when the text is equal; the World diffs and draws nothing new). No other row is touched. Inside the detail's generation `#1`, `$items[2]` changes promptly, `ShowBody` changes, and the label's slot rebuilds to the new body. The detail node itself, generation `#1`, is untouched: the `dyn` is over `$selected`, and `$selected` did not change.

**Occurrence 3, a new item 4 arrives.** The `for` builds row 4's block in this occurrence. The detail is untouched.

**Occurrence 4, item 2 is deleted while selected.** Row 2's block is torn down at commit. In the detail, `$items[2]` becomes `Nil` and the label says "(no such item)". `$selected` is still `Some 2`, because nothing fired at it; a program that wants the selection cleared adds `| Nil on !Filter (Gone (!Updated $items), $selected)` to the hold, reading the selection through a tag as the items change, and that is a decision the hold's line shows.

**Occurrence 5, click row 1 and row 3 at once.** `!picks` fires with `{1: 1, 3: 3}`; `FirstKey` says 1. The detail rebuilds to item 1. If the program would rather the higher key won, it says `LastKey`.

The alternative to compare against is a `dyn` over the pair `($items, $selected)`, which is what you get in frameworks where the detail is a function of all its props. That version rebuilds the detail on every model change, which for a detail with its own state (a scroll position, an expanded section) resets that state whenever anything anywhere in the list changes. Here the thing that owns the detail's generation is the selection, and the model flows through it live.

### Tabs

Tabs are the master-detail pattern with the twist that the user expects each tab to remember where it was. There are two honest versions and the language makes you pick one.

```prism
type Tab = choice Inbox | Drafts | Settings

@tabs_simple: @unit = @widget:
    !to_inbox <- @Button $"Inbox"
    !to_drafts <- @Button $"Drafts"
    !to_settings <- @Button $"Settings"
    $tab <- Inbox then Inbox on !to_inbox | Drafts on !to_drafts | Settings on !to_settings
    dyn $tab:
        match _:
            Inbox: @inbox_pane
            Drafts: @drafts_pane
            Settings: @settings_pane
    @ unit
```

**What rebuilds when.** Clicking Drafts in occurrence n changes `$tab`; the `dyn` tears down the inbox pane at n's commit and builds the drafts pane in n. Everything the inbox pane owned goes: its scroll position hold, its selected-message hold, its `Fs.File` slots, any request in flight (the World discards the response, since the slot is gone). Come back to Inbox and it is built fresh, at the top, with nothing selected, and its `@started` fires again, so whatever it loads at start is loaded again. For a settings pane this is usually what you want. For an inbox it is usually not.

The second version lifts the state that should survive out of the `dyn`, into the node that persists, and hands each pane its state as a dynamic. The pane returns an event of the changes it wants made, and the hold outside applies them. Since only one pane exists at a time and the `dyn`'s result has to have one type across all generations, the block returns a record with one event field per pane, `!never` in the fields for the panes that are not showing:

```prism
type TabEvents = record:
    !inbox: InboxState
    !drafts: DraftsState

@tabs_kept: @unit = @widget:
    !to_inbox <- @Button $"Inbox"
    !to_drafts <- @Button $"Drafts"
    $tab <- Inbox then Inbox on !to_inbox | Drafts on !to_drafts
    $inbox <- (InboxState 0, Nil) then switched.!inbox
    $drafts <- DraftsState "" then switched.!drafts
    !!panes <- dyn $tab:
        match _:
            Inbox:
                !ch <- @InboxPane $inbox
                @ TabEvents !ch, !never
            Drafts:
                !ch <- @DraftsPane $drafts
                @ TabEvents !never, !ch
    let switched = TabEvents (!Switch !!panes.!inbox), (!Switch !!panes.!drafts)
    @ unit
```

`!!panes.!inbox` is field access lifted over the outer event: an `!!InboxState`, the inbox's change event per generation, `!never` in generations where the drafts pane is showing. `!Switch` over it is the one `!inbox` change event across all generations, which feeds the hold that lives outside; `switched` is a plain record of two events, and the holds read its fields. The pane takes `$inbox` as a dynamic and returns the changes it wants made; it does not own the state, it edits it, and the hold's line in `@tabs_kept` lists the one event that can move it.

**What rebuilds when, in this version.** Clicking Drafts rebuilds the pane, as before. The inbox pane's node goes, with its slots and its subscriptions. Its *state* does not, because it never had any: `$inbox` is the parent's. Come back to Inbox and `@InboxPane $inbox` is built with the scroll and selection it had. The cycle `$inbox → dyn → !!panes → !Switch → switched → $inbox` is legal through the switch's hold, and the pane's changes in the occurrence it is built (there are none, by the creation rule, unless it fires one from `@started`) are swallowed, which is fine, because the parent's state is what it was.

There is a third version, which keeps every pane built and shows only one. The language has no "hidden" flag, because visibility is a World's or a library's idea, not the language's; the construction is an interceptor around all three panes that holds back the `Ui.*` elements of the inactive ones and forwards everything else. Each pane is wrapped in a `@Tagged` that puts its elements under a known path prefix, and the interceptor's `for` over the captured list keeps or drops each slot by whether its prefix is the active tab's:

```prism
PaneOf: fn Path -> ?Tab                        # which pane's subtree a path is under, by its first segment
IsUi: fn Element -> Bool                       # chapter 6's: True for every Ui.* tag

@tabs_all: @unit = @widget:
    !to_inbox <- @Button $"Inbox"
    !to_drafts <- @Button $"Drafts"
    !to_settings <- @Button $"Settings"
    $tab <- Inbox then Inbox on !to_inbox | Drafts on !to_drafts | Settings on !to_settings
    ((ignore, $slots)) <- intercept @all_panes, !down, $ \|e| @Element e
    !down <- for $slots:                                   # key is the child's path
        let $shown = not (IsUi $_.element) or (PaneOf key) == Some $tab
        !!resps <- dyn ((slot, shown)) <- Pair $_, $shown:
            if shown:
                ((!resp, ignore)) <- @Element slot.element
                @ !resp
            else: @ !never
        @ !Switch !!resps
    @ unit

@all_panes: @unit = @widget:
    @inbox_pane
    @drafts_pane
    @settings_pane
    @ unit
```

`$shown` is true for every non-`Ui.*` slot and for the `Ui.*` slots of the active pane, so a pane's sockets, file reads and timers reach the World whether the pane is showing or not, and its buttons and labels reach it only while it is. A slot never changes, so `Pair $_, $shown` changes only when `$shown` does, and the `dyn` over it rebuilds a row exactly when its slot becomes shown or hidden: the hidden row's element leaves the interceptor's list, the shown row's arrives, and the World sees one pane's buttons go and another's come. Each forwarded element's response is the row's result through `!Switch`, consolidated into `!down : !(Path ==> Response)`, which `intercept` routes to the child by path; the one-occurrence lag of the switch costs nothing, since a freshly forwarded slot cannot be answered in the occurrence it was created anyway. The supplier is the transparent one, since no pane here has a state element the interceptor wants to answer. This is chapter 6's `@Forward` with one `dyn` added per row, and chapter 6's library containers are the same construction with layout as the policy instead of "shown". In the `$shown` line, `(PaneOf key)` and `(IsUi $_.element)` are parenthesised because a paren-less call takes the rest of its expression: `PaneOf key == Some $tab` would hand `PaneOf` the comparison as its argument.

Every pane's slots, requests and state then exist all the time; the World sees only the active pane's drawing; an inbox whose socket should stay open while you edit settings is this version. The cost is that every pane exists, which is a cost you pay knowingly, in a line that says `intercept`.

### A modal dialog

A dialog is a widget that exists for a while and whose result is the answer. The thing to get right is that the dialog's existence is a hold's value, the dialog's answer is what closes it, and the two form a cycle that is legal.

```prism
type Answer = choice Confirmed | Cancelled

@delete_with_confirm: fn Text -> @!unit = \|what| @widget:
    !open <- @Button $"Delete {what}"
    $dialog <- Nil then (Some unit) on !open | Nil on !answer
    !!answers <- dyn $dialog:
        match _:
            Nil: @ !never
            Some ignore:
                @Label $"Really delete {what}?"
                !yes <- @Button $"Confirm"
                !no <- @Button $"Cancel"
                @ (Confirmed on !yes | Cancelled on !no)
    let !answer = !Switch !!answers
    @ unit on !Filter (IsConfirmed !answer)
```

`IsConfirmed: fn Answer -> ?unit` is `Some unit` for `Confirmed` and `Nil` otherwise, so `!Filter` over it is a unit event that fires on confirmation.

`$dialog : $?unit` is open or closed. The `dyn` over it builds nothing but a never-firing result while closed, and the dialog while open. `!answer` is the `!Switch` over generations; `$dialog`'s hold lists both the thing that opens it and the thing that closes it, and the widget's result is a unit event that fires on confirmation.

**Occurrence 0.** `$dialog` is `Nil`; generation `#0` is the `!never` block, which has no slots; `!!answers` fires with `!never`; `!Switch` adopts it. The list has one button.

**Occurrence 1, click Delete.** `!open` fires; `(Some unit) on !open` fires; the merge's left wins (its right is quiet); `$dialog`'s change event fires with `Some unit`. The `dyn` rebuilds: generation `#1` is built with a label and two buttons, at `1/#1/0/#0/0`, `1/#1/1/#0/0`, `1/#1/2/#0/0`; `!!answers` fires with the new block's answer event. `!Switch` is still on `!never` for this occurrence. Commit: `$dialog` is `Some unit`, the switch adopts generation `#1`'s event. The World draws the dialog.

**Occurrence 2, click Cancel.** The World sends `click` to `1/#1/2/#0/0`. `!no` fires; the answer event fires with `Cancelled`; `!Switch` delivers it as `!answer`; `Nil on !answer` fires; `$dialog`'s merge: `Some unit on !open` is quiet, so `Nil` wins; the change event fires with `Nil`; the `dyn` rebuilds to generation `#2`, the empty block; `!!answers` fires with `!never`. Generation `#1` keeps propagating until commit, which is how its `Cancelled` reached the switch at all, and is dropped at commit. `IsConfirmed` says no, the result is quiet.

**Occurrence 3, click Delete again.** As in occurrence 1: generation `#3` is built, with fresh paths `1/#3/...`, and the switch adopts its answer event at commit. The dialog that was cancelled is not "reopened"; a new one exists.

**Occurrence 4, Delete and Cancel together.** The user clicks the Delete button behind the dialog and Cancel in the same frame, and the World delivers both. `!open` and `!answer` both fire. The hold's merge takes the left, `Some unit`, and the dialog stays open: generation `#4` is built because the change event fired, even with the same value as before, and generation `#3` is dropped. The user sees a dialog where there was a dialog. A program that finds that rebuild wasteful filters the hold's event so that opening an open dialog is not a change, and a program that wants Cancel to win writes the merge the other way round; either way the answer is on the hold's line.

**Occurrence 5, Confirm.** `Confirmed` reaches the switch, the hold closes the dialog, the `dyn` rebuilds to the empty block, and the widget's result fires `unit`. Whoever bound `@delete_with_confirm` gets one event, in the occurrence of the click, and the dialog is gone by commit.

What the program does not have: a dialog manager, a z-order stack, a `showModal()` that returns a promise, or any notion of the dialog "closing itself." The dialog's block returns an event; the parent's hold decides what the event means for the dialog's existence; and the cycle between them is the same cycle as the clicker's, closed through the switch. Where the dialog is drawn, over everything else, is a layout question for an interceptor that reads the element list and puts the dialog's elements last.

### A list with selection

Master-detail had the selection outside the list and the detail outside the list. The variant where the rows themselves show the selection is the one that tests whether you have the ownership right, because the easy version rebuilds every row on every click.

```prism
let Mark: fn Text, Bool -> Text = \|t, sel| if sel: "> {t}" else: "  {t}"

@selectable: fn $(Nat ==> Item) -> @$?Nat = \|$items| @widget:
    !picks <- for $items:
        let $is_selected = $selected == Some key
        !click <- @Button (Mark $_.title, $is_selected)
        @ key on !click
    $selected <- Nil then Some (!Filter (FirstKey !picks))
    @ $selected
```

`$selected` is outside the `for`, and each row reads it through a lifted `==`. The button's label is a dynamic, so inside each row the button is a `dyn` over its label.

**Occurrence 1, click row 2.** `$selected` changes to `Some 2`. In row 2, `$is_selected` changes to `True`, the label changes, and the button's `dyn` rebuilds its slot to generation `#1`. In every other row `$is_selected` is recomputed, since `$selected` changed, and is `False` as before; its change event fires with the same value; the label is recomputed to the same text; the label's change event fires with the same text; and the button's `dyn` rebuilds, because a `dyn` rebuilds when its dynamic's change event fires, and equality of the value is not something a derived dynamic checks for you. Every row's button gets a new slot.

That is correct and it is wasteful. Correct, because the World diffs plain data and will draw nothing differently for the rows that did not change. Wasteful, because every row rebuilt a slot, and because every row's button lost its path, which a World keying native widgets by path would notice. The fix is a prelude function that stops change events with equal values, applied where the fan-out is:

```prism
        let $is_selected = $Distinct ($selected == Some key)
```

`$Distinct: fn $a -> $a` for `a: Eq`, a derived dynamic whose change event fires only when the value differs from the value before. With it, clicking row 2 changes `$is_selected` in exactly two rows, the one that was selected and the one that now is, and two buttons rebuild their slots. The rest of the rows see no change event and do nothing.

Why is `$Distinct` not the default? Because the change law says a derived dynamic fires its change event when an operand's does, and making that conditional on equality would require every derived dynamic's value type to be comparable, which functions, events and widgets are not. The dynamic that fires with an equal value is a correct dynamic; it is only an expensive one when a `dyn` sits downstream, and the program says where the de-duplication goes.

**Occurrence 2, item 2's title changes.** Row 2's `$_` changes, its label changes, its button rebuilds. No other row is touched, and `$selected` is `Some 2` still.

**Occurrence 3, row 2 is deleted.** Its block is torn down at commit. `$selected` is `Some 2` and nothing is selected on screen; the widget's result still says `Some 2`, which is the truth about the hold, and the parent decides what a selection of a missing key means.

The contrast case is putting the selection inside: a `$is_selected` hold per row, toggled by the row's own click. Then clicking row 3 does not deselect row 2, because row 2's hold did not hear about it, and making it hear means broadcasting an event into every row, which is the consolidated event upside down. The selection is a fact about the list, so it lives in the list's node, and the rows read it.

### A wizard

A sequence of pages, each with inputs, with Next and Back, and the answers collected at the end. The trap is where the answers live: in the pages, they die with the page; in the parent, each page needs its answer as a plain initial value and returns the edited one.

```prism
type Step = choice Name | Address | Review

type Answers = record:
    name: Text
    address: Text

type Nav = record:
    step: Step
    answers: Answers
    done: Bool

let IfDone: fn Nav -> ?Answers = \|n| if n.done: Some n.answers else: Nil

@wizard: @!Answers = @widget:
    let $page = Pair $step, $answers
    !!navs <- dyn ((step, answers)) <- $page:
        match step:
            Name:
                TextInput(ignore, $name) <- @TextInput answers.name
                !next <- @Button $"Next"
                let $forward = Nav Address, (Answers $name, answers.address), False
                @ $forward on !next
            Address:
                TextInput(ignore, $address) <- @TextInput answers.address
                !back <- @Button $"Back"
                !next <- @Button $"Next"
                let $edited = Answers answers.name, $address
                @ (Nav Name, $edited, False) on !back | (Nav Review, $edited, False) on !next
            Review:
                @Label $"{answers.name}, {answers.address}"
                !back <- @Button $"Back"
                !finish <- @Button $"Finish"
                @ (Nav Address, answers, False) on !back | (Nav Review, answers, True) on !finish
    let !nav = !Switch !!navs
    $step <- Name then !nav.step
    $answers <- (Answers "", "") then !nav.answers
    @ !Filter (IfDone !nav)
```

The `dyn` is over `Pair $step, $answers`, one dynamic of the pair, and the pattern form takes it apart into two plain values. Inside a page, `answers` is plain, so `@TextInput answers.name` gets a plain initial and is built once per page visit; `$name` is the input's live value; `$forward` is a lifted record over it; `$forward on !next` is an event with the record's value as it stood before the click, which is the text as typed.

**Occurrence 0.** `$step` is `Name`, `$answers` is empty, `$page` is the pair, the `dyn` builds the Name page.

**Occurrences 1 through 5, typing.** Each keystroke changes `$name` inside the page: the text input's own label slot rebuilds, `$forward` changes, nothing else. `$page` does not change, because neither `$step` nor `$answers` did. The page is not rebuilt.

**Occurrence 6, Next.** `!next` fires; `$forward on !next` fires with `Nav Address, (Answers "Ada", ""), False`; the switch delivers it as `!nav`; `!nav.step` fires with `Address` and `!nav.answers` with the record; both holds' change events fire in this occurrence; `$page`, a derived dynamic over both, changes once (glitch freedom: one change event per occurrence however many operands moved); the `dyn` rebuilds to the Address page with `answers.name = "Ada"` plain, and an address input initialised from `""`. The Name page is dropped at commit, and `$name` with it; the name is safe in `$answers`, which is outside.

**Occurrence 7, Back.** The Address page's `!back` fires with `Nav Name, (Answers "Ada", "..."), False`, whatever had been typed so far; both holds move; the `dyn` rebuilds the Name page with `answers.name = "Ada"` as the input's initial. The text is there. It did not survive in the page; it went out and came back in, and the line that says so is the hold for `$answers`.

**Occurrence 8, Finish.** `done` is `True`; `!Filter (IfDone !nav)` fires with the answers; the widget's result fires. The holds also move (to the same step and the same answers), so `$page` changes with an equal value and the Review page is rebuilt, which is harmless and could be stopped with `$Distinct` if the review page were expensive.

What this pattern shows about `dyn` over a tuple: the rebuild happens when either part changes, and the design makes both parts change only together, in a navigation occurrence, so one rebuild per navigation. Had the answers been saved live, on every keystroke, `$page` would change on every keystroke and the page would rebuild under the user's cursor; that program is also writable, and it is wrong, and it is wrong in a way you can see from the line `let $page = Pair $step, $answers` together with the line that moves `$answers`. Where the state lives and what the `dyn` is over are the whole design of a wizard, and they are two lines.

## The invariants, and who keeps them

Everything above rests on a short list of promises, each with a party bound by it and a mechanism that enforces it. They are gathered here because a reader changing the design should know which sentence breaks when a rule does.

*Every graph object is owned by exactly one node, and nodes form a tree.* Bound: the runtime and the compiler. Enforced: nodes are not values in the language (the compiler has no construct that yields one), `WidgetBuilder::bind` is the only way to make a child and it pushes the child onto the parent, subscriber lists are weak, and closure conversion makes captures indices into the owning node's environment. The test `dropping_a_widget_frees_its_cyclic_loops` checks the one place a node could own itself.

*A replaced node propagates until the commit of the occurrence that replaced it, and is dropped then.* Bound: the runtime. Enforced: the rebuild action replaces the child and schedules the old node's drop at the commit height; nothing drops a node from inside propagation. Checked by `a_replaced_child_keeps_working_until_its_last_occurrence_ends`.

*A widget built in occurrence n sees all of n.* Bound: the runtime. Enforced: every event caches its current-occurrence value and `subscribe` returns it; holds, switches and `dyn` rebuilds triggered by an already-fired event defer to `max(height, current + 1)`; closures run at most once per occurrence. Checked by the `_mid_occurrence_` tests and the nested-`dyn` test.

*A `dyn`'s event fires in every build occurrence, the first included, and goes to the caller.* Bound: the runtime. Enforced: the result is the leftmost of the rebuild event and the done-event-mapped first build, and the block has no name for it.

*`!Switch` is non-prompt; `$Join` is prompt.* Bound: the runtime. Enforced: `switch_hold` reads its inner event from a hold's value side; `Dynamic::join` fires on the outer's change with the new inner's value after n. The conformance suite has a law test for the swallowed creation-occurrence firing of the switch and three for join: inner changes alone, swap to an inner that changed in the same occurrence, swap to an inner that did not.

*A path is never reused.* Bound: the runtime. Enforced: each `dyn` node keeps a generation counter that only goes up, incremented on every rebuild and never reset while the node lives; each `for` node's key segment is the key itself, and a key's node is dropped when the key leaves, so a returning key is a new node under the same segment with fresh generations beneath it. A quarantine does not touch the counters: a quarantined subtree is held out of rebuilds, and when the World lifts the quarantine the next rebuild takes the next generation, not `#0`. A World may therefore remember anything by path and be sure that a path it has seen leave the list will not come back meaning something else.

*A `for` key that returns is a new key.* Bound: the runtime, which has no table of departed keys and builds a returning key's block from scratch; and the program, whose allocator decides whether keys can return at all. Enforced on the runtime's side by the keyed list widget owning exactly the nodes of the keys currently present; on the program's side by nothing but the allocator's line, `$next <- 0 then $next + 1 on !add`, which is why the idiom is written that way.

*A failed build fires no event and leaves a hole.* Bound: the runtime and the World. Enforced: the rebuild action, on a contained failure, tears the old generation down at commit, installs no child, and does not notify the `dyn`'s subscribers; the error is returned from the run call as kind `Build` with the `dyn` node's path as its owner, and offered through `errors` if the program subscribed. A cycle found during the build is kind `Cycle` and aborts the occurrence instead, which keeps the old generation by rolling everything back. The World is bound to report what it is returned and may send `quarantine` paths on its next call. Chapter 9 has the type and the protocol; chapter 10's `errors` step is the check.

*A `for` keeps a node per key; a key that appears is built in its occurrence and a key that disappears is torn down at commit; consolidation is by result kind and is prompt.* Bound: the runtime and the compiler (which states the result kind). Enforced: the keyed list widget projects a per-key dynamic and defers builds like a `dyn`; the merge for an event result includes blocks built in the occurrence.

*Holds, cycles and `@Element` appear only in widget blocks.* Bound: the program; enforced by the compiler, which has no other place to put them.

*A cycle inside one widget block passes through a delayed edge.* Bound: the program; enforced statically by the compiler over the block's groups, with `dyn` edges marked prompt. Cycles formed through widget functions or by switching are found by the runtime's height comparison and abort the occurrence or the construction.

*Nothing fires because a node appeared or was dropped.* Bound: the runtime, Worlds and interceptors. Enforced in the runtime by there being no such event; in Worlds by the obligation list, as a norm the Test World checks with `quiet`.

*Building has no effects.* Bound: the compiler and the runtime. Enforced: `build` is handed a builder and nothing else; the only thing it can do to the world is put a slot in a node, which the World reads after the occurrence.

## The negative space

Each of the things widgets cannot do is a hole you have fallen through, and each is also a mechanism the implementation does not have to build. The specification lists the refusals for the whole language; these are the four that belong to widgets, with what each buys.

**No node values.** You cannot keep a reference to a button, a pane, a row. A node is a position in the hierarchy and nothing else, and the only way to get one is to bind a widget at that position. What this closes: the detached subtree kept alive by a stale listener, the parent whose decision to remove a child means nothing because someone else still holds it, the ownership graph that needs a collector because it is not a tree. What it buys the implementation: ownership is a tree of `Rc`s pointing down, subscriber lists are `Weak`, closures reach the graph through the node that owns them by environment index, and teardown is `drop`. The Rust runtime has no tracing collector and will not need one; the one threat, a closure capturing a sibling's node strongly, is closed by closure conversion making captures indices rather than pointers, and cyclic placeholders inside one block are the explicit exception, owned by the node as a unit.

**No lifecycle hooks.** There is no `onMount`, `onUnmount`, `componentDidUpdate`, `useEffect`, `ngOnDestroy`. A widget is built, which has no effects, and torn down, which has no effects. What this closes: the cleanup function you forgot, the effect that ran in an order the documentation told you not to rely on, the hook that fired twice in development mode and once in production, the unmount that raced the fetch. What it buys: build is a pure function of the arguments and the occurrence, so two builds with equal inputs make equal nodes; there is no second ordering system beside propagation; and teardown is a drop at commit with nothing to run. The thing you reached for a mount hook to do, kick something off when the widget exists, is `@started` plus a request event, and it is in the block, where the widget's other events are, with the same laws.

**No request on creation.** Creating an element does nothing. A slot appears in the list and the World reads the list; if the program wants something performed it fires the slot's request event, and `@started` exists so it can do that in the build occurrence. What this closes: the fetch-on-mount that fires on every rebuild, the test that cannot run without a mock installed before the first render, the rebuild whose cost is a surprise. What it buys: the diode. Inputs cause list changes and list changes never cause inputs, so there is no loop across turns where a rebuild requests, the response rebuilds, and the cycle check never sees it, and a rebuild costs exactly what its block asks for, which is visible in the block.

**No mutation.** The only state is a hold; the only way to change one is its event; there is no assignment anywhere, not in a block, not in a helper, not in a closure. What this closes: the counter with six updaters in six files, the handler that writes the model before the list reads it, the "which version did I see." What it buys: every write in an occurrence is a hold's commit, so all writes are buffered to the end and an occurrence can be aborted by dropping the buffer. The transaction is not a feature the runtime adds; it is what is left when there is nothing to protect against. Rollback on error is the same drop. And the ownership story above depends on it: a node owns its holds because holds are the only state, and if there were another kind of state, something would have to own that too.

The pattern is the one the specification promises for the language as a whole. A feature you would have sworn a widget system needs is absent; the absence closes a bug you have shipped; and the same absence lets the runtime leave out the machinery the feature would have needed. Nodes are positions, so no collector. No hooks, so no lifecycle scheduler. No request on creation, so no reconciliation of effects against renders. No mutation, so no dirty tracking and free transactions. What remains is a tree, a graph hanging off it, one entry point, and the three constructs of this chapter for changing the tree in the occurrence the program says to.
