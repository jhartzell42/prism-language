# Chapter 4 — Events, dynamics, and time

Everything a Prism program does happens in an occurrence, and an occurrence is a transaction: it has a before, an after, and no inside that anything can see. This chapter is the definition of the two kinds of value that live against that clock, events and dynamics, and of the clock itself. Each is given the same way: the afternoon you lost to the problem it solves, the ordinary-language meaning, the denotation, the laws, and a few lines of code that show the law doing work. The laws are small and there are not many of them, and the claim is that they are enough: the tag law alone turns a cyclic definition into a definition, the change law alone rules out glitches, and the rule for what a widget built in the middle of an occurrence sees is one sentence with five consequences. The chapter ends with each law restated as a Test World script, the things deliberately left out of this part of the language and what each absence buys, and four idioms that are what writing against these laws feels like day to day.

## Three questions and one clock

Every interactive program is three questions asked over and over: what is true now, what just happened, and what exists. Most languages answer the first with a variable, the second with a callback, and the third with an object, and the three answers do not agree about time. The variable has whatever value the last writer left; the callback runs whenever the framework gets to it; the object was constructed at some point and will be destroyed at some other point, and neither point is visible from the code that uses it. Prism answers the three questions with three kinds of value whose sigils are part of their names, `$`, `!` and `@`, and defines all three against one clock.

The clock is a sequence of occurrences, numbered from zero. An occurrence is discrete and atomic. Something either happens in occurrence 7 or it does not; there is no "during 7" at which half of it has happened. A dynamic has one value before 7 and one value after 7. An event fires at 7 or is silent at 7. A widget is built in some occurrence, lives through some, and is torn down in one. Occurrence zero is the one in which the program is built, and it exists so that everything built at the start has an occurrence to have been built in. Plain values and optionals have no relation to occurrences at all, which is what makes them plain.

Widgets are the subject of their own chapter. Events and dynamics are the subject of this one, with the clock they are defined against. What follows holds for every event and every dynamic in a program, whether it came from a button, a socket, a `for`, or a line of arithmetic, because none of those is a special case: `!click` is an event in exactly the sense `!delta` is, and the laws do not know which one was native.

## Events

### The handler that ran at the wrong time

You have a text field with a change handler, and a form with a submit handler, and the submit handler reads the field's model. One day a user types a character and presses Enter in the same instant, which on a slow machine means the same frame, and the submit handler runs before the change handler, because the submit handler was registered first, and the form posts the old text. You fix it by reordering two `addEventListener` calls. Nothing in the program's text says that the order matters; the fix is invisible in the diff except as a swap, and the next person to touch the file will put them back in the order that reads better.

That is the whole of what is wrong with callbacks, and the ordering is only the visible part. A callback is code that runs *when* something happens, and "when" was never defined. It runs after some other callbacks and before others, reading state that some of them have changed and some have not. Two things that happen "at once" happen in registration order, which is a fact about the program's history and not about its text. And because a callback is code, it can do anything: read any state, write any state, call anything, including the code that registers more callbacks. The question "what happens when the user presses Enter" has an answer spread across every handler that might run in that frame, in an order nothing records.

Prism's answer is to take "when" away from the code and give it to the language. Nothing runs when something happens. Instead, a thing happening is a fact about a moment, and the program is a set of statements about how facts at a moment relate. Enter was pressed at occurrence 12; the text at occurrence 12 is whatever it was before 12; the submission at 12 is the text before 12, if Enter was pressed at 12. There is no handler to order because there is no handler.

### What an event is

An event is something that happens, with data about the specifics of this happening. A click, with where it was. A chunk of stdin, with its bytes. A response from a file read, with the contents or the error. A timer going off. The common sense of the word is the right one, and it is the one to hold on to against the other sense the word has acquired, in which an "event" is an object with a `.preventDefault()` method that a handler receives. A Prism event is not a value that is true, and it is not an object that is delivered. It is a thing that occurred, and between occurrences there is nothing there to look at.

**Definition.** An event `!A` is a value of type `A` at some occurrences and absent at the others. As a function, it is a partial function from occurrence numbers to `A`.

That is the entire denotation, and it is deliberately the simplest thing that could be written down. It has two consequences that everything else in this section follows from. An event is total over time: `!click` is not "the click that just happened", it is the whole history and future of clicks on that button, and asking "did it fire at 12" is a question with an answer. And an event has no value between firings. There is no `!click.last`, no "the most recent click"; if you want that, you hold it into a dynamic and say what it was before the first click, and the hold is where the "most recent" lives.

### Simultaneity

Two events that fire at the same occurrence are simultaneous, and simultaneity is a thing the language defines rather than a race the implementation resolves. A program never observes two events in an order within an occurrence. There is no "`!a` fired, then `!b` fired" at occurrence 12; there is "`!a` and `!b` both fired at 12", and every operation that takes more than one event says what that means for it.

This is why `!a + !b` is a compile error and not an event of sums. At an occurrence where only `!a` fires, what is the sum? There is no answer, and rather than invent one (zero? skip it? the last `!b`?), the language refuses. The rule is that an operation may have any number of dynamics and at most one event among its operands, and the two or more case is an error naming both events and pointing at the two operations that do know what simultaneity means: `|`, which takes one of them, and `!Leftmost`, which takes one of several. The message has the form "two events in one operation: `!a` and `!b`; merge them with `|` or `!Leftmost`", with both spans, and it is a compile error, never a runtime one.

This is liveable because simultaneity is rare, and when it happens the program has already said what it wants. A click and a keystroke in one frame; a network response and a timer in one turn; two buttons the user mashed together. In every framework you have used, these cases were resolved by registration order and you found out which order when a bug report arrived. Here they are resolved by the line that merges the two events, and reading that line is reading the resolution.

The refusal is about two *events*. An optional inside an event is a different matter and raises no error: `!?a` is an event that sometimes fires with `Nil`, and `?` lifts inside it at the plain layer, so `!?a + 1` is an `!?Nat` that fires whenever `!?a` fires, with `Nil` when it did. The same holds under `$`: `$a / $b` over `Nat`s is a `$?Nat`, because `/` on `Nat` and `Int` returns `?`; on `Float` it follows IEEE and never fails, so the same line over `Float`s is a plain `$Float`. The time functor is always outermost and lifts first; `?` lifts second, inside it; and `match` on the optional is how you get back out. `!Filter` is that `match` packaged for events: it takes an `!?A` and gives the `!A` that fires only on `Some`. The precedence between `?` and the time sigils, and `?$x` as the thing that does not lift, are chapter 3's; here it is enough that a `?` never makes two events coincide, because it never adds a second event.

### `!never`, and why there is no `!always`

`!never` fires at no occurrence. It has every type, and there is one of it. It is the identity for merge, the event you return from a `match` arm that has nothing to say, and the initial inner event of every `!Switch`.

There is no `!always`. It is the first place the language says no to something that seems harmless. An `!always` would fire at every occurrence, and the obvious use is "do something every time anything happens": log the occurrence, recompute a layout, poll a value. But "every time anything happens" is a statement about the program as a whole, and no part of the program is entitled to it. If an occurrence has nothing to do with you, you do not get to know about it. A widget that could observe every occurrence could observe that a sibling it has never heard of received a response, which is exactly the ambient channel the goal of no global anything forbids. It would also break the implementation's one license: the runtime may skip a node whose inputs did not fire, and `!always` would make every node's inputs fire.

Everything you wanted `!always` for is available some other way. A value that should be recomputed when anything it depends on changes is a derived dynamic, and the change law (below) says it changes exactly then. A log of what happened is a merge of the events you care about. A poll is a clock element, which is a dynamic pushed by the World, and a program that depends on one says so in its element list.

### Merge, and why the left wins

`a | b` fires at n if either fires at n; its value is `a`'s if `a` fires, else `b`'s. It is associative, and `!never` is its identity on both sides. `!Leftmost [[e1, e2, ...]]` is the same over an array, taking the first in array order that fires.

The left bias is the answer to "what if both buttons are clicked at once", and it is an answer the program gives by choosing which one to write on the left. In the counter,

```prism
let !delta = -1 on !decrement | +1 on !increment
```

two clicks in one occurrence are a decrement, and if that is wrong the fix is to swap the operands, which is a change you can read.

Suppose instead the merge produced both. `a | b` would be an event of a small list, or of a pair of optionals, and every consumer would have to say what it does with two. For the counter that is `-1 + 1 = 0`, which is a defensible answer, and it is also an answer the program can write by hand: `$count + (Sum !deltas)` over a `for` that consolidates into a map. The question is what the *operator* should mean, and an operator that returns a collection is an operator everyone has to unpack. The common case by a wide margin is that one of the two should win, and the uncommon case has `for`'s consolidation and the library's `!Union` of maps, where both really are wanted and the result really is a collection.

Suppose instead simultaneity in a merge were an error. Then every merge of two events that could ever coincide would need a guard, and most pairs of events can coincide: the World is allowed to batch two inputs into one occurrence, and a program cannot know which pairs it will batch. A merge that could fail at runtime would make every program's correctness depend on the World's batching policy, which is the registration-order bug with a new name.

Suppose instead the merge were right-biased, or took "the most recent". The first is the same design with a different convention. The second is the registration-order bug itself: there is no "most recent" within an occurrence because there is no order within an occurrence.

So the left wins, the program says what is on the left, and when the program wants both it says that too, with a different operator whose result type admits both. A few passing examples of the left bias chosen on purpose:

```prism
let !action = Reset on !reset | Step on !tick        # a reset in the same frame as a tick is a reset
let !key = !shortcut | !typed                         # the shortcut layer sees the key first
let !close = unit on !escape | unit on !close_button  # either closes; it does not matter which
```

The third one is the common case in which the bias is irrelevant because both sides have the same value, and the merge is simply "or".

### `on`: how an event gets its data

`v on !e` fires exactly when `!e` fires, with the value `v`. When `v` is a dynamic, the value is the dynamic's value *before* the occurrence, which is the tag law of the next section applied here. `on` is how a click becomes "a click while the label was X" and a keystroke becomes "Enter, with the text as it stood".

`on` has no semantics of its own. It is a plain function that returns its left operand, `\|v, e| v`, and the lifting rule gives it its meaning: an operation with one event operand fires when that event fires, with the operation applied to the event's value and the dynamics' old values. The whole of `on` is that lifting rule plus a function that ignores its second argument. This matters because it means there is nothing to learn: `$text on !enter` and `(Length $text) on !enter` and `(Submit $text, $user) on !enter` are all the same mechanism, and in the last one the name `on` could be dropped in favour of a three-argument function and nothing would change.

Three uses you will write constantly:

```prism
let !submitted = $text on !enter               # the value at the moment
let !tick_at = $clock on !tick                 # the time before the tick
let !selected = $items[$cursor] on !choose     # a lookup, lifted over two dynamics and one event
```

One thing `on` does not do is pair an event's own value with a dynamic; it discards the event's value. To keep both, apply a function: `(\|k, n| ((k, n))) !key, $count` is an event of pairs, the key that fired and the count before it. Everyone reaches for `(($count, !key))` once and finds that a tuple with an event inside it is a plain tuple, not an event; the sigil's position says what moves, and in `((!a, $b))` nothing does.

### Lifting over one event

A lifted operation with exactly one event operand fires exactly when that operand fires, with the operation applied to the event's value and the dynamics' values before the occurrence. `IsClick !in` filters nothing; it is an event of booleans that fires with every `!in`. `!Filter` turns an `!?A` into an `!A` that fires only on `Some`, and it is the one plain combinator a program needs to drop firings; everything else is a function applied to an event.

The rule has a consequence that is easy to miss: nothing inside the language makes an event fire from nothing. A lifted operation fires when its event operand fires; a merge fires when a side fires; `!Switch` fires when its chosen inner fires; `dyn`'s event fires when its block is built, which happens when its dynamic changes or when the `dyn` itself is built, which happens because something above it was built, and so on up to the root in occurrence zero. Every firing in a program is traceable to an input the World delivered or to the build. The World owns time, the runtime has one entry point, and the same inputs give the same occurrences every time, which is why a transcript is a test.

### `!Switch`

A button whose label is dynamic is rebuilt when the label changes, and each build has its own click event. The parent does not want a new `!click` per label; it wants one `!click`, for the button, whichever label it happens to show. A view that is replaced when a mode changes has a submit event per view, and the model wants one `!submit`. A connection that is reopened has a received-bytes event per socket, and the protocol wants one `!received`. In every case there is a `dyn` whose block returns an event, the `dyn`'s result is an event of events, one per build, and the rest of the program wants the one event that is "whichever is current".

`!Switch: fn !!A -> !A` is that event. At occurrence n it delivers the firing, if any, of the inner event that the outer event most recently delivered *before* n. It starts subscribed to nothing. An inner event delivered at n is heard from n+1.

```prism
@DynLabelButton: fn $Text -> @!Text = \@widget:
    !!rendered <- dyn $_:
        !click <- @Button _
        @ _ on !click
    @ !Switch !!rendered
```

Inside the `dyn` block, `_` is the label as a plain value, because within one build it never changes; a change rebuilds the block. `!!rendered` fires in every build with the new block's click event, tagged with the label. `!Switch` turns the generations into one event.

This pattern is so common that binding over `@` does it for you. `!click <- @Button $label`, with a dynamic where the widget wants a plain label, means exactly the five lines above: a `dyn $label:` around the build, and `!click` is the `!Switch` of the `dyn`'s event. A widget function applied to a dynamic argument lifts to `dyn`, and the bound name is the switch of the generations. When the widget's result is a dynamic rather than an event, the bound name is a hold of the generations' dynamics, `$Join`ed, with the first build's result as the hold's initial value; when the result is plain, the bound name is a `$` holding the first build's value. The rule is stated here once; every later example in this book writes `!a <- @Button $"A"` and means the switch. One consequence to keep in mind: a `!click` bound this way is non-prompt in the label, so a relabel and a click in one occurrence deliver the click from the old button, as the next paragraphs explain.

**Why it is not prompt.** Suppose it were: an inner event delivered at n would be heard at n. Then a block built at n could fire at its parent at n, through the switch, and the parent could react, and the reaction could change the dynamic the `dyn` watches, which would rebuild the block at n, which would fire again. Reflex has `coincidence` and `switchPromptlyDyn` for exactly this, and anyone who has used them has debugged them. The height of a prompt switch's output depends on which inner event it is currently listening to, and the inner event can be chosen *in the same occurrence* by something downstream of the output, which is a cycle that only shows up when a particular inner is chosen. The runtime has to re-enter propagation to handle it, and the program has to reason about what a freshly built thing can and cannot do in its first occurrence.

With a non-prompt switch, every node's height is fixed by its sources as they stood when the occurrence opened. A `!Switch` reads which inner to listen to from a hold, and the hold's value side is a read of before-state, so the switch's output is below nothing that fires at n through it. A cycle through a switch is one comparison at the moment the switch changes inners, not a re-entrant search. And propagation never has to re-enter itself: once a node has fired at n, nothing at n can make it fire again.

**What it costs.** A freshly built block cannot fire at its parent in the occurrence it was born, through the switch. The switch swallows whatever the new block fired at n before it was adopted. Under the rule that elements never fire in the occurrence they were created, the only things a new block can fire at n are pure consequences of its own construction, `@started` and the events of any `dyn` inside it, and a parent that wants to know the block was built already knows: the `dyn`'s own event fired at n with the block's result. If a block needs to act on its own birth, it uses `@started` inside the block, where there is no switch in the way:

```prism
!!loads <- dyn $path:
    !started <- @started
    !resp <- @Element Fs.File(path=_, requests=(Read on !started))
    @ !resp
let !loaded = !Switch !!loads
```

`Fs.File` is chapter 7's one-shot file conversation, `requests: !FsRequest` and `responds: FsResponse`. Here `@started` fires in the block's build occurrence, the `Read` fires in that occurrence, the response comes in a later one, and by then `!Switch` is listening to the new block's `!resp`. Nothing is lost, because nothing a block could want to say to its parent on the occurrence of its birth is something the parent did not already know from building it.

In the button above, a click in the same occurrence as a relabel is delivered from the old button, under the old label. That is the only answer consistent with routing, since the click was addressed to a slot in the element list as the occurrence opened, and the old button is what was there. The old block keeps propagating until the occurrence commits, so its click reaches the switch, which is still listening to it; at commit the switch adopts the new inner, and from n+1 the new button's clicks are the ones heard. There is no moment at which a button is on screen and nothing is listening to it, and no moment at which two are listened to at once.

### Events are values, not callbacks

An event is a value. It can be stored in a record, put in an array, passed to a function, returned from a widget, merged with another, used twice. `[!Click]` is a plain array of events and `!Leftmost` takes one. A record with fields `!changed` and `$value` is how a text input returns two things. None of this requires a subscription, a listener, or a disposal; an event used twice is the same event, and an event nobody uses costs nothing.

Compare the callback. It is not a value you can merge: to say "either of these" you write a third function that both call. It is not a value you can pass without also passing the thing it will be called on. It cannot be used twice without being called twice. Its "subscription" is an effect with a lifetime that someone has to end, and the someone is usually nobody, which is where leaks come from. And it can do anything, so "what can happen when this fires" is not a question with an answer in the type.

The consequence that matters most for reading a program: in Prism, the flow of what-happens is in the data, not in the control. `!delta` is defined from `!increment` and `!decrement`; `$count` is defined from `!delta`; the label is defined from `$count`. Each line names its inputs. To find out what a click does, you follow the uses of `!click`, and every use is a definition of something else. There is no handler body to read, because there is no handler.

## Dynamics

### The variable read in the middle

You have had the afternoon where a checkbox un-checks itself. You click it, the handler sets the model, the model notifies the list, the list re-renders, the re-render reads the model to decide whether the box is checked, and the model, it turns out, was updated *after* the list read it, because the notification went out from inside the setter before the assignment. You move a line and it works. Three months later someone adds a second subscriber that also reads the model during notification, and the box un-checks itself from a different direction.

The variable in that story was never wrong. At every instant it held exactly what the last assignment put there. What was wrong was that "now" had no definition: the list read the model at a moment in the middle of a cascade, and at that moment some things had been updated and some had not, and nothing in the program could say which. Every scheduler, dirty flag and batching layer in every reactive framework exists to paper over this one fact, and the papering is never finished because the fact is structural. A variable that can be read at any instant can be read at the wrong one.

Prism fixes "now" to the occurrence. Before an occurrence a dynamic has one value. After, it has one value. During, the only question anyone can ask is "what was it before", and that question has exactly one answer. There is no instant in the middle of the cascade at which a read can land, because reads do not happen at instants; they happen at occurrences, and an occurrence has no inside.

### What a dynamic is

A dynamic is a value that is true now and will be different later: the count, the text in the box, the contents of a file, the set of connected users, the position of the cursor. It is the thing every program has and most languages call a variable, and it is the only mutable thing in Prism. It is mutable in exactly one way: it changes when its change event fires, to that event's value. That sentence is the whole of the language's mutation policy. Everything else, the World boundary included, is arranged so that this stays the only way anything changes.

**Definition.** A dynamic `$A` has a value of type `A` at every occurrence, and changes only at occurrences. It is a pair: a *current value* and a *change event*. At occurrence n the current value is the value *before* n, and the change event, if it fires at n, has the value *after* n. After n, the current value is the change event's value if it fired, else unchanged.

The current value is *before* n, always; there is no reading of a dynamic at n that gives the after-value except the change event itself, which is an event and is consumed as one. The change event has a name: `!Updated $d` is `$d`'s change event, a plain prelude function of type `fn $a -> !a`, firing with the new value in every occurrence `$d` changes and never at build. The current value has no name: there is no operator that returns it, no way to ask for it except through the laws below. Reflex exposes it as `current`, the behavior half of a `Dynamic`, and lets you `sample` it. Prism does not, and the reasons are the subject of the last part of this section.

### The tag law

In any lifted operation with an event operand, a dynamic contributes its value before the occurrence. `$count + !delta` at n is `(count before n) + (delta at n)`. `$text on !enter` at n is the text before n. `Step $state, $clock, !action` at n is `Step` applied to the state before n, the clock before n, and the action at n.

This is the one window onto *before*, and it is deliberately the only one.

The tag law makes a cyclic definition a definition. Look at the counter's hold:

```prism
$count <- 0 <= $count + !delta
```

`$count` appears on both sides. In a language with assignment this is `count = count + delta`, an equation whose solution is "whatever the last write was". Here it is not an equation: the `$count` on the right is a read through a tag, which by the law is the count *before* the click, and the hold on the left is the count *after*. The line reads "the count after a click is the count before it plus the delta", which is a definition, and the cycle in the graph passes through a read of before-state, so nothing in the occurrence depends on itself.

The tag law makes propagation deterministic. At an event there is one version of every dynamic, the one before. There is no "which version did I see", because within an occurrence nothing has a second version to see. Two reads of `$count` in two places in one occurrence, at two different events or at the same event through two different operations, give the same number. Reflex lets you choose: `tag (current d) e` gives the old value and `attachPromptlyDyn d e` gives the new one, and which you wanted depends on whether `d` is updated by something upstream of `e` in this frame, which is a question about the graph's structure that the program's text does not show. The `tagPromptlyDyn` documentation says outright that `tag (current d) e` is preferred when creating cyclic data flows, which is the library telling you that the choice is a trap. Prism removes it. If you want the new value, you want the change event, and you say so by using the change event.

The tag law leaves the implementation free. Because every read at an event is a read of before-state, the implementation may propagate the occurrence in any order that respects prompt dependencies and nothing can tell. It may compute a value early, late, or on demand; it may skip a node whose inputs did not fire; it may, later, run independent nodes in parallel. None of that is observable, because the only observable reads are of a state that was fixed when the occurrence opened.

The tag law is also where the "before" in a stopwatch comes from. A Start click stamped with `$clock on !start` gets the clock as it stood when the occurrence opened, even if the same occurrence also pushes a new clock value. The hundred milliseconds between the two is the price of a readable rule, and a World that cares pushes the clock in an occurrence of its own first.

### The change law, and promptness

A lifted operation over dynamics and no event is a dynamic whose value at every occurrence is the operation applied to the operands' values at that occurrence, and whose change event fires at n, once, with the new value, if any operand's change event fires at n.

So `1 + $x` is a dynamic that is one more than `$x` at every occurrence, before and after. `"Count: {$count}"` is a dynamic text. `$items[$cursor]` is a dynamic optional, changing when either changes. `Total $watch, $clock` changes with the clock while the watch runs. A derived dynamic is never stale: in the occurrence its input changes, it changes.

That is promptness, and it is the point. Suppose derived dynamics lagged: `$x` changes at n, `1 + $x` changes at n+1. Then at the end of n, after commit, `$x` is 5 and `1 + $x` is 5, and anything that reads both, a label showing "x is 5 and x+1 is 5", sees a state that was never true. That is a glitch, and it is the checkbox afternoon in miniature. A derived dynamic that changes in the occurrence its input changes cannot show such a state, because there is no occurrence in which the input has changed and the derivation has not.

The change law and the tag law meet at a lifted operation with both kinds of operand. `$count + !delta` is an event, and its dynamics are read through tags. `$a + $b` is a dynamic, and its change event is prompt. The sigils decide which law applies, which is why you can read it off the line: an event in the operands means a tag; no event means promptness.

A derived dynamic fires its change event when an operand's change event fires, with the new value, *even if the new value equals the old one*. `$edit.text` changes when `$edit` changes, and if only the cursor moved the text is the same text. A dynamic's change event is allowed to fire with the value it already had. Nothing downstream can tell the difference except by comparing, and the comparison is a plain function the program can apply if it wants to; a derived `dyn` over `$edit.text` will rebuild on a cursor move, and a program that minds writes `dyn ($Distinct $edit.text):`. `$Distinct` is a prelude function that needs equality on the value and passes on only the changes that differ from the value before; it is the one place equal values are suppressed, and it is opt-in. The alternative, in which the change law compared values and suppressed equal ones, would need equality on every type a dynamic can hold, including types that have none, and would make the number of firings depend on the representation of values, which the implementation chapter wants to be free to choose.

### Hold

```prism
$checked <- False <= !toggle | !reset
```

There is the whole story of `$checked`. It starts false. A toggle sets it to the toggle's value, a reset sets it to the reset's, a toggle and a reset together are a toggle, and nothing else in the program can touch it, because nothing else in the program has a way to. You have searched a codebase for every assignment to a flag, found eleven, and still missed the one behind a reflection call; here the search is one line long and it is the line that declares the thing. That is the goal of one way to change anything, written next to the thing, and the hold statement is where it is syntax.

Formally: `$x <- v <= !e`, allowed only as a statement in a widget block, defines `$x` whose value before the first firing of `!e` is `v`, whose change event is `!e`, and whose value after any occurrence where `!e` fired is `!e`'s value there. Hold is the only way to make a dynamic that is not derived from another. Every piece of state in a Prism program is a hold, so every piece of state has a line that names its initial value and the one event that can move it, and reading that line is reading everything that will ever happen to it.

The value `v` is plain, or a dynamic if `$x` is a dynamic of dynamics. It is not an expression re-evaluated on each firing; it is the value before the first firing and nothing else. A hold is a statement rather than an expression because it has an owner, the node of the widget block it is written in, and there is no hold outside a widget block because that would be state nothing owns. Which node owns which hold, what a `dyn` rebuild and a `for` key's departure do to the holds inside them, and where state that must survive a rebuild lives are chapter 5's; the one fact this chapter needs is that a hold inside a rebuilt block is a new hold, starting at `v`, and it obeys the construction rule below like any widget built mid-occurrence.

In the occurrence in which the widget is built, `$x`'s value is `v`, and if `!e` fires in that same occurrence, `$x` changes to it then. This is the build occurrence behaving like every other occurrence: the hold has a before-value, `v`, and a change event, which fires or does not. It is how `@started` kicks things off:

```prism
!started <- @started
$phase <- Starting <= Loading on !started | Loaded on !resp
```

`$phase` is `Starting` before occurrence zero and `Loading` after it, because `!started` fired in zero. A tag reading `$phase` in occurrence zero reads `Starting`; a label derived from it shows `Loading` from the end of zero. There is no special first-occurrence rule, and no special "initial render" during which holds are frozen.

### Glitch freedom

No operation observes a derived dynamic with some operands updated and others not. Within one occurrence a dynamic's change event fires at most once.

The diamond is the test:

```prism
$a <- 0 <= !set
let $b = $a + 1
let $c = $a * 2
let $d = $b + $c
```

When `$a` changes from 0 to 5 at n, `$d` changes from 1 to 16, once. It does not pass through 6 (new `$b`, old `$c`) or 11 (old `$b`, new `$c`). Nothing that reads `$d`, a label, a `dyn`, a tag at some event, can see either intermediate, because there is no occurrence in which either is `$d`'s value. A signals library has to work for this property, with a topological scheduler or a two-phase mark-and-sweep, and the schedulers are where its edge cases live. Here it is a law: the change law says `$d`'s value at every occurrence is the sum at that occurrence, and "at that occurrence" has no sub-steps. An implementation that fires each node once, after all of its prompt sources, satisfies it; heights (below) are the standard way.

The at-most-once half is the half that keeps the first half honest. If `$d`'s change event could fire twice at n, once per input, then "the value at n" would be ambiguous, and a hold fed by `$d`'s change event would have to pick. One firing, one value, one commit.

Who is bound: the runtime, for every derived dynamic, and the World, for every state element. A state element is a dynamic the World feeds, and a World that pushed two values for one element value in one occurrence would be making that dynamic change twice at n, which is the at-most-once half broken from outside. The runtime enforces its own half by firing each node once after its prompt sources (heights, below), and enforces the World's half at the boundary: a second push for the same element value in one run call is rejected as a `Component` error, like a duplicate response to one path, and the occurrence does not open. The script form of the law is the diamond test in "Laws as tests"; the boundary half has its test in chapter 10, where the run call is specified.

### Equality of readings

Two widgets that have never met both ask for `Fs.Contents(path="/etc/app.conf")`. One shows the file, the other parses it for a setting. The file changes on disk. You have seen the version of this where the parser ran on the new file and the viewer still showed the old one, for a frame or for a minute, because two caches with two expiry policies were between the disk and the two readers. Here the two slots are one dynamic, because state is identified by element value, and two results of it in one occurrence must match: the viewer and the parser change in the same occurrence or in neither.

The law: two readings of one dynamic in one occurrence, by any means, give the same value. A tag at `!click` and a tag at `!key`, both reading `$count` in occurrence n, read the same `$count`. Two labels derived from `$count` show the same number in every occurrence. A `dyn` over `$count` and a tag on `$count` in the same occurrence agree about what the value was before and what it is after. This is the tag law and the change law stated as a property of the reader rather than of the operation, and it is listed separately because it is the law that extends across the World boundary. The occurrence is the version, for everything. The program is bound to read only through the laws, which the language enforces by having no other way to read. The World is bound to answer one value per element value per occurrence, through its oracle and its pushes, and the runtime enforces that by delivering one snapshot value to every slot that asks and one push to all of them; a World that answered the same value differently to two slots in one occurrence would have broken the program, not merely the World, and the `clock_readings_agree` script below is the check.

### No sampling, and what it buys

You cannot ask a dynamic what it is right now. There is no `sample`, no `.current`, no `.value`, no way to write a plain function that takes a dynamic and returns its contents. You can read it through a tag, at an event, and get the value before the occurrence. You can rebuild on it with `dyn` and get the value as a plain `_` inside a block that is replaced when it changes. You can derive from it and get another dynamic. That is the whole list.

This is the restriction that feels like a missing feature until the checkbox. Every "read it right now" in that story was a read of something half-updated, and the reason it was half-updated is that "right now" was an instant in the middle of a cascade. With no sampling there is no instant. A read is either at an event (and gets before-state, defined) or is a derivation (and gets the current value at every occurrence, prompt and glitch-free). There is nothing in between to get wrong.

The program gains this: every read of state is visible as such. `Step $state, !input` has `$state` in it; you can see that `Step` reads the state, and you can see at which event. A function that could sample would read state invisibly, inside its body, at a moment the call site does not show, and "what does this function read" would again be a question about its implementation.

The implementation gains freedom of order. Since no read can land in the middle, the middle can be anything. Nodes can fire in any order consistent with prompt dependencies. A value nobody reads need not be computed. Later, independent nodes at one height can run in parallel, and nothing in any program has to change, because no program could tell the difference. A language with sampling has to define what a sample in the middle of propagation returns, and whatever it defines becomes a constraint on every optimisation.

The thing you reach for sampling to do is almost always one of two things. "Get the value when this happens" is a tag: `$x on !e`. "Do something different depending on the value" is `dyn`: build the block for this value, and rebuild when it changes. The one thing neither covers is "get the value *after* this occurrence's changes, at this event", and that is the thing the design refuses, because it is the thing that depends on propagation order.

### The hidden behavior

Reflex's `Dynamic` is a pair of a `Behavior` and an `Event`, and both halves are first-class: `current d` is the behavior, `updated d` is the event, and a program can combine them freely. The flexibility is real and so is the cost. `sample`, `tag`, `attach`, `attachWith`, `attachPromptlyDyn`, `tagPromptlyDyn`: the library has two of most things, one reading the behavior (old value) and one reading the event (new value), and the one you want depends on whether the dynamic is updated upstream of your event in this frame, which depends on the graph you cannot see. Pick wrong and you get a stale read or a cycle, and both are quiet.

Prism keeps the pair and hides the behavior half. Every operation that would read it, a tag, `!Switch`'s choice of inner, a hold's own value, `$Join`'s choice of inner before the swap, does so non-promptly, as before-state, and no operation in the language can do otherwise. There is one `tag`, not two, and it is spelled by putting a dynamic in an operation with an event.

The one place promptness remains is construction, and it is deliberate. `dyn $x:` sees the new value of `$x` in the occurrence it changes, and the block it builds sees all of that occurrence (the construction rule, below). Derived dynamics are prompt. `$Join` is prompt. You can always trust that a dynamic's change event has the new value in any occurrence it fires, and that the value after the occurrence is exactly that. What you cannot do is read that new value at an event in the same occurrence, and that is the whole restriction.

### Small things

`$ v` is the constant dynamic: `$ "Increment"` is a dynamic text that never changes, and `$"Increment"` is the same with the sigil attached. A function that takes `$Text` can be given a constant this way, and since a constant dynamic's change event is `!never`, deriving from it costs nothing.

`{$x}` inside a text literal interpolates by lifting: `"Count: {$count}"` is a dynamic text, changing with `$count`, with no sigil on the literal, because a literal that interpolates a dynamic is a dynamic. An event interpolated with `{!line}` makes the literal an event, firing once per line. A `$` in text is always an ordinary character, so `"costs $5"` says what it looks like, and a literal brace is written `\{` or `\}`.

`$$x` is a dynamic of dynamics, and `$Join` flattens it: `$Join $$x` at n is the value after n of the inner that `$$x` holds after n, so it is prompt in the outer and in the inner, including when both change at n. Chapter 5 owns the rest of switching; the law has its script in "Laws as tests" below. `?$x` is an optional dynamic, a plain value that is `Nil` or holds a dynamic; `$?x` is a dynamic whose value is sometimes `Nil`. They are different types, and the order of the sigils is the order you would say it.

## Time

### The occurrence as a transaction

You have a save button and an autosave timer, and one day they fire in the same frame. The button handler serialises the document and starts a write; the timer handler, running second, sees the dirty flag still set (the button handler clears it when the write completes, asynchronously), serialises again, and starts a second write, and the two writes race on disk. The fix is a lock, or an `isSaving` flag, or a queue, and whichever you pick is a transaction you are writing by hand around two handlers that each believed it was alone.

An occurrence is that transaction, written once, for everything. It is opened by the World with a set of inputs, each addressed to an element slot, all simultaneous. The program propagates: every event that fires as a consequence fires, every derived dynamic changes, every `dyn` and `for` that must rebuild rebuilds. Then the occurrence commits: every hold takes its new value, every element list and every result takes its new value, and the World reads the outcome. Then it is closed. Between open and commit nothing outside the program observes anything, and nothing inside the program observes a state that mixes before and after except through the tag law, which is the one deliberate window onto *before*.

The two handlers above become one occurrence with two inputs, a click and a tick. The program's text says what a click and a tick together mean, through a merge, and the write request is one event firing once. There is no flag because there is no second handler to see it.

Holds committing at the end makes the rest possible. Because no write happens until commit, all writes in an occurrence can be buffered, and an occurrence can be aborted by dropping the buffer: a cycle found halfway through, an overflow in arithmetic (kind `Overflow`; programs that want a different answer use the `Checked.*` and `Wrap.*` prelude functions), a protocol error from a World, and the occurrence simply did not happen. Nothing half-done survives, because nothing is done until the end. The error chapter builds on this; here it is enough that the transaction was already there, and it was there because the language has no assignment.

Occurrence zero is the build of `@main`. It has no inputs; it exists so that everything built at the start has an occurrence to have been built in, so that holds have a before-value and a build occurrence like any other hold, and so that `@started` fires at the start the way it fires in any rebuild. There is nothing else special about it. A program that behaves differently in occurrence zero than in a rebuild of its root is a program with a bug, not a program using a feature.

### The dependency graph

The program defines a graph over its events and dynamics. Each event or dynamic is a node; an edge runs from a source to a target when the target's firing or change at an occurrence is determined by the source's. The graph is what the program's text is: `let !delta = -1 on !decrement | +1 on !increment` draws two edges into `!delta`; `$count <- 0 <= $count + !delta` draws one from `!delta` into the hold's change event and one, of a different kind, from the hold's value back into the addition. The whole program is this graph, the World feeds inputs into its leaves, and an occurrence is a wave through it.

Every edge is one of two kinds, and the distinction is the single most useful thing to be able to see in a Prism program.

### Prompt and delayed edges

An edge is *prompt* when the target's firing or change at n depends on the source's firing or change *at n*. These are the edges that make the wave: lifting over an event (`F !e` fires at n because `!e` did), lifting over dynamics (the change law: `$a + $b` changes at n because `$a` did), merge (`!a | !b` fires at n because a side did), `for`'s consolidation (the map fires at n because a block's event did), `$Join` (changes at n because the outer or the current inner did), and construction (`dyn $x:` rebuilds at n because `$x` changed at n, and its event fires at n with the new block's result).

An edge is *delayed* when the target reads the source's value *before* n, which is to say its behavior side. There are three such edges, and they are the same thing seen from three places:

- A **tag**: a dynamic operand in an operation that has an event operand. `$count` in `$count + !delta`.
- **`!Switch`'s choice of inner**: which inner event the switch listens to at n is the one delivered before n. It is a hold of inner events, read before the occurrence.
- **A hold's own value side**: the value a hold has at n is the value it had after n−1.

All three are a read of a dynamic as it stood when the occurrence opened. That is what "delayed" means and all it means: not "later", not "next occurrence", not "scheduled", but "before". A delayed edge does not make anything happen later. It reads the past.

The distinction matters because only prompt edges constrain propagation order within an occurrence, and only prompt edges can form a cycle that means anything. A delayed edge reads a value that was fixed before the occurrence opened, so it imposes no order and closes no loop. The counter's cycle is `!delta → (change event) → $count → (tag) → $count + !delta → !delta`, and the tag edge is delayed, so the prompt graph of the counter is a tree.

Three ways to recognise the two kinds in source, since everything comes down to recognising them:

```prism
let !next = $count + 1 on !tick        # delayed into $count (tag), prompt from !tick
let $double = $count * 2               # prompt from $count (change law)
let !heard = !Switch !!generations     # delayed into !!generations' hold, prompt from whichever inner was chosen
```

The rule of thumb: an event among the operands means every dynamic in the operation is read through a delayed edge; no event means every dynamic feeds a prompt one.

### Heights

The prompt graph must be acyclic. Given that, propagation in dependency order is well defined, and an implementation that fires each node once, after all of its prompt sources have settled, is correct.

Heights are the standard way to do it, and both implementations use them, but the specification does not require them. Each node sits at a height strictly above every prompt source; nodes fire in height order, lowest first; a node at height h fires only when every node below h that could fire at n has. A `!Switch` whose chosen inner changes recomputes its height from the new inner and invalidates every cached height downstream, so that the next occurrence's order is right for the new graph. A node built mid-occurrence gets a height from its sources as they are at that moment. Heights are cheap, they make "has everything below me fired" a single comparison, and they make a cycle detectable the moment a node tries to schedule below the current height.

The specification requires only the observable consequences: each event fires at most once per occurrence, each dynamic changes at most once, and no node observes a prompt source before that source has settled. An implementation that met those three by sorting the graph topologically at the start of every occurrence, or by pulling values on demand with memoisation, or by running a fixpoint iteration that happens to converge in one pass for an acyclic graph, would conform. The reason to say so is that the conformance suite tests transcripts and not heights, and the Haskell implementation runs on Reflex, whose heights are Spider's and not the Rust runtime's.

One consequence of "fires each node once, after all of its prompt sources": a node whose prompt sources did not fire at n does not fire at n and is not visited. An occurrence costs work in proportion to what happened in it, not in proportion to the program's size. A program with ten thousand holds and one click does the work of one click. That is the push model, and the reason the language can afford to make every derived value a live node.

### Cycles

A cycle in the prompt graph is a definition that depends on itself in one occurrence, and it has no meaning. `let !a = !b` with `let !b = !a` is the simplest: at n, does `!a` fire? Only if `!b` does, which only if `!a` does. There is no answer, not even "no", because `!never` is only one of the fixed points and nothing in the program chose it.

**Inside one widget block**, cycles are found at compile time. The rule: a binding cycle within a block is legal if and only if it passes through at least one delayed edge. A cycle made of prompt edges alone is an error that lists every binding in it, with spans. The compiler has every binding of the block in front of it and knows for each operation which of its operands are dynamics and which are events, so it knows which edges are tags. The counter passes: its cycle has a tag in it. This does not:

```prism
@broken:
    let !a = 1 + !b
    let !b = 2 + !a
    @ unit
```

and the error names `!a` and `!b` and points at both lines.

`dyn` is not a delay. Its event fires in the occurrence of the rebuild, with the new block's result, and the block it builds sees the occurrence it was built in. So a cycle through a `dyn` alone, in which the dynamic a `dyn` watches is derived from the `dyn`'s own event, is a prompt cycle and is rejected:

```prism
@also_broken:
    !gen <- dyn $n:
        @ _ + 1                         # the block's result is plain, so the dyn's event is !Nat
    $n <- 0 <= !gen                     # the hold's change event is !gen
    @ unit
```

`$n` changes at n because `!gen` fired; the `dyn` rebuilds at n because `$n` changed; `!gen` fires at n because the `dyn` rebuilt. If this were allowed it would rebuild forever within one occurrence. The fix, if something like it is wanted, is to read `$n` through a tag at some event that is not the `dyn`'s own, or to put a `!Switch` in the loop, which reads its inner from before.

The structure that looks like the cycle above is also the structure of every list program, so `for` needs its own rule. The consolidated event of a `for` over `!A` blocks fires at n with the entries of every block that fired at n, including a block built at n. The edge from a block's event to the consolidated event is prompt. The edge from the collection's change event to a *new* block's event exists only through whatever that block fires in the occurrence it is born, and the only things a block can fire then are pure consequences of its own construction: `@started`, and the events of `dyn`s inside it. Elements cannot fire in the occurrence they are created, so a button inside the block cannot. The compiler has the block's body and can see whether its result depends promptly on anything that fires at build; if it does not, the collection's change is not promptly upstream of the consolidated event, and the usual list program is legal:

```prism
@todos:
    !add <- @Button $"Add"
    $next <- 0 <= $next + 1 on !add
    let !adds = {$next: Add "item {$next}"} on !add
    !rows <- for $items:
        !delete <- @Button $"Delete"
        @ Delete on !delete
    let !changes = !Union !adds, !rows
    $items <- {} <= StepAll $items, !changes
```

(`!Union` merges two maps of firings, the left winning a shared key; `StepAll` folds a map of changes into the items in key order. Both are plain library functions.) `$items` is defined from `!changes`, which comes from `!rows`, which is the consolidation of `for $items`. The row's result is `Delete on !delete`, and `!delete` is a `!Switch` inside `@Button`, which reads before. A row born at n fires nothing at n. The cycle is legal, and would be a compile error with `@ Delete on !started` as the row's result, which is right: that program adds a row and immediately deletes it, in the same occurrence, in which the deletion rebuilds the `for`, which builds nothing, which changes `$items` again. For a `for` whose blocks return plain values or dynamics, the consolidated `$(K ==> A)` changes at n whenever a key appears at n, so the edge from the collection to the consolidation is always prompt and a hold fed from it that feeds the collection is always a cycle.

The check can only see what is in front of it. The todo list is legal because the row's body is written in the same block as the hold it feeds; the compiler reads `@ Delete on !delete`, finds a switch, and knows the row is silent at birth. Write the same list with the row passed in, `for $items: @Row _` where `@Row` is a parameter of the enclosing widget function, and the body is not visible, so the compiler assumes the worst: a row might fire at birth, the edge from `$items` to `!rows` is taken as prompt, and `$items <- {} <= StepAll $items, !changes` is rejected as a cycle even if every `@Row` ever passed is silent. The elaborator records "can fire at birth" on every `for` node, from the body when it can see one and conservatively when it cannot, and the typed AST prints the flag; the Haskell implementation reads it to decide whether the consolidation needs Reflex's `coincidence` over the membership change, which is the one place that combinator is permitted. A program that wants a parameterised row and the closed loop breaks the loop with a delayed edge the compiler can see: the rows feed a hold, and the collection is stepped from that hold through a tag at some other event, so the cycle passes through a read of before-state wherever the rows came from.

**Across widget functions and through switching**, not every cycle can be found statically. `!out <- @Echo !out` is a cycle if `@Echo`'s result depends promptly on its argument and not if `@Echo` holds the argument first, and the block that binds it does not see `@Echo`'s body. A `dyn` may choose, at runtime, between a block whose result is prompt in some input and one whose result is not, and the cycle exists only when the first is chosen. An implementation must detect these when they occur and treat them as an error: it must not deadlock, must not loop, and must not fire a node twice. In a height-ordered runtime the moment is precise: a node that tries to schedule at or below the current height is a node that was already supposed to have fired, and that is the cycle, reported with the chain of nodes that led to it. What happens next, abort of the occurrence or quarantine of the subtree, is the error chapter's subject; what this chapter promises is that the occurrence in which a cycle is found commits nothing.

A note on what is *not* a cycle, because the form fools people. An interceptor's response event is defined from a `for` over its child's element list, and the child's element list is produced by the `intercept` that consumes the response event. That is a loop in the text. It is legal because routing reads the list as the occurrence opened, which is a tag, and the slots a new input could reach are the ones that existed before it. Hit-testing is the same: a VBox tags a click with its layout as it stood when the occurrence opened, and no cycle exists in the heights. Anything that reads the list to route an input reads before, by the routing rule, and the routing rule is the tag law applied to the list.

### Construction in the middle of an occurrence

`dyn` and `for` build widgets while an occurrence is in progress. The new widget subscribes to events that may already have fired at n, derives from dynamics that may already have changed at n, and contains holds, `dyn`s and `@started`s of its own. The question is what it sees, and the answer is one sentence:

**A widget built in occurrence n behaves as though it had been built immediately before n's inputs arrived.**

Everything else is a consequence. The rule binds the runtime and nothing else: a program cannot break it and a World cannot see it being broken except through the transcript. The Haskell implementation gets it from Spider, whose subscriptions read the current frame's value on connection; the Rust runtime must build the catch-up cache sketched at the end of this section. The scripts that check it are `nested_dyn_builds_once` and `hold_resets_and_catches_up` in "Laws as tests", and the differential test between the two implementations catches the half the scripts cannot see, the doubled request firing.

**It sees every firing of n.** An event that fired at n, at any point in the order, is seen as having fired by anything the new widget subscribes to it. A lifted operation over such an event, inside the new widget, fires at n. A hold built at n whose source fired at n adopts that value at commit. If the new block contains `$seen <- Nil <= Some !input` and `!input` fired at n before the block was built, `$seen` is `Some` after n. This holds at any depth: an event three combinators down from an input that fired is seen as fired, exactly as it would be by a subscriber that had been there all along.

**Its tags read before n.** A tag inside the new widget reads values before n, like every tag. The new widget is not a privileged observer of the middle of the occurrence; it is an ordinary widget that happens to have been built at a particular moment, and the moment is "before the inputs", so before-state is what it reads.

**A `dyn` built at n over a dynamic that changed at n builds once, with the new value.** This is the consequence with teeth. Picture a page whose title is derived from the page number, with an outer `dyn` over the page and an inner `dyn` over the title:

```prism
@pages:
    !next <- @Button $"Next"
    $page <- 0 <= $page + 1 on !next
    let $title = "Page {$page}"
    dyn $page:
        @Label $"body of {_}"
        dyn $title:
            @Element Ui.Label(text=_)
    @ unit
```

A click at n changes `$page` and, promptly, `$title`. The outer `dyn` rebuilds at n. Its block builds an inner `dyn` over `$title`, which has already changed at n. The inner `dyn` builds its block once, with `"Page 1"`, because it sees `$title`'s change event as having fired at n. It does not build `"Page 0"` and then rebuild. The label the World sees after n is `"Page 1"` and there was never a `"Page 0"` slot in this generation.

**Every computation runs at most once per occurrence**, regardless of how many late subscribers read it. If the inner `dyn` and a tag and a hold all read the same derived event at n, the closure behind that event ran once, and all three got its one value.

**Its elements exist from n, and nothing can reach them at n.** A slot the new widget creates is in the element list after n, but no input addressed to it can arrive at n (routing is against the list as n opened), and nothing it requests can answer at n (the creation rule). A state element it creates has, at n, the value before n, which is what the World's snapshot for n holds, and receives n's push like every other slot; so it ends n holding the same value a slot that had existed before n would hold.

**Why the alternatives are wrong.** Each rule an implementation might have adopted instead fails in a way you would otherwise meet in production.

Suppose a late subscriber were hidden from earlier firings: a widget built at n sees only what fires *after* it is built. The nested `dyn` above is then built after `$title` changed, sees no change, builds with the value its dynamic had before n, `"Page 0"`, and stays wrong until the next time the title changes. The outer page says 1 and the inner title says 0, and nothing in the program is wrong except the runtime. This is the stale nested `dyn`, and it is the first bug anyone writing this runtime meets, because it is what the obvious implementation does.

Suppose instead that each late subscriber re-ran the closures it needed: the new widget, on subscribing, computes the event's value afresh from its inputs. Now a closure runs once for the original subscribers and once more for each late one. A closure that is pure gives the same value each time, which hides the problem until the value is used for something that counts, a request fired from a lifted operation that now fires twice, or a cost that is now proportional to subscribers. At-most-once is a law, and this breaks it.

Suppose instead the `dyn` built with the old value and then, hearing the change, rebuilt in the same occurrence. The World sees one generation after n, so the transcript looks right. But the first build happened: its `@started` fired, and any request it fired from `@started` fired, so the World is handed two requests, one for a page that never existed. Its holds were created and discarded. Its elements were in the list for a moment nobody saw, and its paths were minted and dropped, which chapter 5's path rules make visible in every later path. Two builds in one occurrence cost twice the work and leave a visible trace in the request log, and the request log is what the conformance suite compares.

Suppose instead that construction were deferred: the `dyn` notes the change at n and builds at n+1, in an occurrence the runtime opens itself. Now the parent's `dyn` event fires one occurrence late, so a hold on the block's result lags its cause; the element list's change event fires one occurrence after the dynamic that caused it; a World that batched inputs sees an occurrence it did not open; and the runtime has gained a scheduler, a queue, and a budget for how many self-opened occurrences a turn may contain. Every one of these is a mechanism the language was designed not to need, and the only thing bought is a simpler rebuild path.

The rule as stated is the only one consistent with the change law for a widget that would have existed from the start of n, and the change law is not negotiable. It is also the rule Reflex's Spider implements, under the name `subscribeAndRead`: subscribing to an event returns its value for the current frame if it has one, and a `hold` subscribed mid-frame adopts that value at the frame's end. Both halves are in `Spider.hs`: `subscribeAndRead` returns a `SubscribeResult` with the occurrence alongside the subscription, and `hold` queues a `SomeHoldInit` whose initialisation calls `subscribeAndRead` and, when an occurrence comes back, defers a `SomeAssignment` to the end of the frame.

**How an implementation honours it.** The realisation is simpler than the consequences suggest. Every event caches its value for the current occurrence; whoever computes it first records it, and the cache is cleared at commit. Subscribing to an event mid-occurrence returns its cached value if it already fired. A derived event subscribed mid-occurrence connects to its sources, takes any already-fired values, computes its own value once, caches it, and delivers it to its new subscriber. A hold subscribed late defers its adoption to commit. A `dyn` built late reads its dynamic's value *as of now*: the change event's value if it has fired at n, else the current value, and if the change fires later at n the `dyn` hears it by the ordinary push. Because every closure is pure and every delayed read is of before-state, a value pulled early is the same value the push would have delivered, and the cache ensures it is computed once however it was asked for. Heights still give the push order; the cache lets a late arrival catch up without disturbing it.

### Determinism

A bug report arrives with a transcript attached: the inputs the World sent, turn by turn, and the element lists it got back. You replay it and the bug replays, on your machine, on the first try, with the same paths and the same requests in the same order. You have never had that. Every bug report you have ever had came with "steps to reproduce" that reproduced on the reporter's machine and not on yours, because the thing that went wrong depended on a timer, a thread, an iteration order, or which handler was registered first, and none of those was in the steps. Here the transcript of what the World sent is the program's entire input, and replaying it replays the bug.

The property: given the same sequence of input sets and the same state-element snapshots, a program's sequence of element lists and outputs is fully determined. There is no scheduling, no ordering between simultaneous inputs, no clock the program can read except as a state element, no randomness, no identity of nodes that a program could observe. Two runs with the same inputs are the same run.

This is the property the conformance suite tests and the differential test between the two implementations relies on. Every rule in this chapter contributes to it. The tag law removes "which version did I see"; the left bias removes "which handler ran first"; no sampling removes "what instant did I read at"; the construction rule removes "was it built before or after the firing"; no events from nothing removes "when did the timer go off". A function from inputs to outputs is left, and functions are testable.

The one thing determinism does not promise is that two *implementations* compute the same thing the same way. They may differ in order, in what they skip, in how they represent a list. They must not differ in a transcript, and when they do, the sentence in this document that they read differently is what gets fixed.

### What an occurrence is not

An occurrence does not correspond one-to-one to anything outside the program. It is not a GUI callback: a World may put a click and a key that arrived in the same frame into one occurrence, and the program sees them as simultaneous. It is not an iteration of a run loop: a World may run several occurrences for one iteration, because a `Defer` was requested, and the program sees them as several. It is not a network packet or a tick of a clock: a packet becomes a response to a slot and a tick becomes a push to a state element, and each becomes an occurrence only when the World opens one. A World that batched two packets into one occurrence and one that delivered them as two would both be conforming, and the program's text says what each choice means.

An occurrence is also not a thread. There is no occurrence in which nothing happens; the World opens one because it has inputs or because a `Defer` is owed. The program cannot ask for one. There is no idle, no `requestAnimationFrame`, no "next tick". A program that wants to do something later asks an element, and the element answers in an occurrence the World opens when it is time.

And an occurrence is not a unit of cost. Nothing says an occurrence is fast or slow, and nothing limits what one may rebuild. It is a unit of *meaning*: the smallest thing that either happened or did not.

## Laws as tests

A law that only one implementation can check is a description of that implementation. Each law above is restated here as a short program and a Test World script, written against what a World can see: elements sent, elements expected, requests fired, errors reported. Nothing reaches into a widget, because nothing can. The scripts are the form in which the laws enter the conformance suite, and both implementations run every one of them.

The programs share a small vocabulary, declared once. `Ui.Label` and `Ui.Button` are the GUI library's tags, which the Test World accepts directly, as it accepts every tag. The two library widgets are written out so that nothing is hidden; a program may write `@Element Ui.Button(label=$x)` directly and mean exactly what `@Button $x` means.

```prism
type Element = choice ..
type Click = click
type Instant = Instant Int            # milliseconds since the World's epoch

tag Ui.Label of Element:
    text: Text

tag Ui.Button of Element:
    label: Text
    responds: Click

tag Clock of Element:                 # state the World pushes; one clock per program
    state: Instant

tag Test.Log of Element:              # a request the Test World records and never answers
    requests: !Text

tag Echo of Element:                  # a request the script answers by hand
    requests: !Text
    responds: Text

@Label: fn $Text -> @unit = \@widget:
    dyn $_:
        @Element Ui.Label(text=_)
    @ unit

@Button: fn $Text -> @!Click = \@widget:
    !!clicks <- dyn $_:
        @Element Ui.Button(label=_)
    @ !Switch !!clicks

let Ms: fn Instant -> Int = \|i| match i:
    Instant n: n
```

`!Updated $d` is `$d`'s change event, a plain prelude function; it never fires at build. Two scripts below count its firings, and the count starts at zero.

### The left wins

```prism
@merge_law:
    !a <- @Button $"A"
    !b <- @Button $"B"
    $last <- "none" <= "a" on !a | "b" on !b | !never
    @Label $last

test merge_left_wins: Test @merge_law: array:
    Expect Ui.Label(text="none")
    Send Ui.Button(label="B"), click
    Expect Ui.Label(text="b")
    Together:
        Send Ui.Button(label="A"), click
        Send Ui.Button(label="B"), click
    Expect Ui.Label(text="a")
```

Two clicks in one occurrence are the left one. The `!never` on the right is there to pin its identity law in passing: merging it changes nothing.

### The tag reads before

```prism
@tag_law:
    !bump <- @Button $"Bump"
    $n <- 0 <= $n + 1 on !bump
    $seen <- -1 <= $n on !bump
    @Label "n={$n} seen={$seen}"

test tag_reads_before: Test @tag_law: array:
    Expect Ui.Label(text="n=0 seen=-1")
    Send Ui.Button(label="Bump"), click
    Expect Ui.Label(text="n=1 seen=0")
    Send Ui.Button(label="Bump"), click
    Expect Ui.Label(text="n=2 seen=1")
```

`$n on !bump` and `$n + 1 on !bump` read the same `$n` at the same event, and both read the value before the click. An implementation that read the new value for either would show `seen=1` after the first click.

### A hold has its initial value in its build occurrence, and adopts a firing there

```prism
@hold_law:
    !started <- @started
    $phase <- "starting" <= "started" on !started
    $before <- "?" <= $phase on !started
    @Label "{$phase} / {$before}"

test hold_in_build_occurrence: Test @hold_law: array:
    Expect Ui.Label(text="started / starting")
```

One step, two facts. `$phase` adopted the firing of `!started` in occurrence zero, so the label shows `started` after the build. The tag on `$phase` at that same firing read `starting`, the initial value, because a tag reads before the occurrence even in occurrence zero.

### Derived dynamics are prompt, change once, and the diamond does not glitch

```prism
@diamond:
    !set <- @Button $"Set"
    $a <- 0 <= 5 on !set
    let $b = $a + 1
    let $c = $a * 2
    let $d = $b + $c
    let !d_changed = !Updated $d
    $fires <- 0 <= $fires + 1 on !d_changed
    @Label "d={$d} fires={$fires}"

test diamond_changes_once: Test @diamond: array:
    Expect Ui.Label(text="d=1 fires=0")
    Send Ui.Button(label="Set"), click
    Expect Ui.Label(text="d=16 fires=1")
```

`fires` is 0 after the build: `!Updated` never fires at build, and `$d` has not changed. After the click `$fires` is 1: `$d` changed exactly once at that occurrence, from 1 to 16. An implementation that let `$d` change per input would show `fires=2`; one that lagged `$d` by an occurrence would show `d=1` after the click. The script does not and cannot see the intermediate values 6 and 11. The count is what makes their absence checkable.

### Two readings agree

```prism
@equal_readings:
    !bump <- @Button $"Bump"
    !other <- @Button $"Other"
    $n <- 0 <= $n + 1 on !bump
    $x <- -1 <= $n on (!bump | !other)
    $y <- -1 <= $n on (!other | !bump)
    @Label "{$n} {$n}"
    @Label "{$x} {$y}"

test readings_agree: Test @equal_readings: array:
    Together:
        Send Ui.Button(label="Bump"), click
        Send Ui.Button(label="Other"), click
    Expect Ui.Label(text="1 1")
    Expect Ui.Label(text="0 0")
```

Two tags on `$n` at two different events in one occurrence, and two interpolations of `$n` in one label, agree. The second label pins that the order of the merge does not change what the tag reads.

The cross-World half of the law needs a state element, and the script needs a `State` step before the build, because the program creates a `Clock` slot in occurrence zero and the oracle must have something to answer; a state tag the program creates at build with no `State` step before it is a script error, not a default.

```prism
@Stamp: fn Text -> @unit = \|name| @widget:
    $clock <- @Element Clock
    @Label "{name}={Ms $clock}"
    @ unit

@clock_readings:
    !bump <- @Button $"Bump"
    @Stamp "a"
    @Stamp "b"
    $clock <- @Element Clock
    $stamped <- -1 <= Ms $clock on !bump
    @Label "stamped={$stamped}"

test clock_readings_agree: Test @clock_readings: array:
    State Clock, Instant 0
    Expect Ui.Label(text="a=0")
    Expect Ui.Label(text="b=0")
    Push Clock, Instant 5000
    Expect Ui.Label(text="a=5000")
    Expect Ui.Label(text="b=5000")
    ExpectNone Ui.Label(text="a=0")
    State Clock, Instant 5100
    Send Ui.Button(label="Bump"), click
    Expect Ui.Label(text="a=5100")
    Expect Ui.Label(text="b=5100")
    Expect Ui.Label(text="stamped=5000")
```

Three slots in three widgets that have never met bind `Clock`, and `Clock` has no fields, so the three are one dynamic; the Test World pushes one value for it and all three labels move in the same occurrence, which the `ExpectNone` pins. The last three steps are the stopwatch's "before" from the tag law section: a `State` step runs no turn, so the click's turn opens with the new clock as its push and the click as its response, in one occurrence. The two stamp labels show `5100`, because a state element changes promptly like any dynamic, and `stamped` shows `5000`, because a tag reads before the occurrence even when the same occurrence pushes a new value. A World that answered `a` and `b` differently in one occurrence would have broken this program, and this is the script that says so.

### `!Switch` is not prompt, and swallows a block's birth

```prism
@switch_law:
    !rebuild <- @Button $"Rebuild"
    $gen <- 0 <= $gen + 1 on !rebuild
    !!born <- dyn $gen:
        !started <- @started
        @ _ on !started
    $heard <- -1 <= !Switch !!born
    let !built = !Updated $gen
    $built <- -1 <= !built
    @Label "heard={$heard} built={$built}"

test switch_swallows_birth: Test @switch_law: array:
    Expect Ui.Label(text="heard=-1 built=-1")
    Send Ui.Button(label="Rebuild"), click
    Expect Ui.Label(text="heard=-1 built=1")
```

Each generation fires `@started` in the occurrence it is born, and `!Switch` never hears it, because the switch adopts the new inner at commit and the firing was before commit. The parent still knows the block was built: `!!born`, the `dyn`'s own event, fired in occurrence zero and again at the click, with each generation's `@started`-tagged event as its value, and a parent that wants to count generations counts that. `built` is `-1` after the build, since `!Updated $gen` does not fire at build, and `1` after the click. `heard` never moves in this program, and that is the law.

```prism
@switch_follows:
    !rebuild <- @Button $"Rebuild"
    $gen <- 0 <= $gen + 1 on !rebuild
    !!clicks <- dyn $gen:
        !c <- @Element Ui.Button(label="gen {_}")
        @ _ on !c
    $clicked <- -1 <= !Switch !!clicks
    @Label "clicked={$clicked}"

test switch_follows_generations: Test @switch_follows: array:
    Send Ui.Button(label="gen 0"), click
    Expect Ui.Label(text="clicked=0")
    Send Ui.Button(label="Rebuild"), click
    ExpectNone Ui.Button(label="gen 0")
    Together:
        Send Ui.Button(label="Rebuild"), click
        Send Ui.Button(label="gen 1"), click
    Expect Ui.Label(text="clicked=1")
    Expect Ui.Button(label="gen 2")
    Send Ui.Button(label="gen 2"), click
    Expect Ui.Label(text="clicked=2")
```

The `Together` is the case that matters: a click on generation 1 in the occurrence generation 1 is replaced is delivered from generation 1, under its own number, because routing is against the list as the occurrence opened and the switch is still listening to the old inner. From the next occurrence the new button is the one heard.

### `$Join` is prompt on the outer and on the inner, including both at once

```prism
@join_law:
    !swap <- @Button $"Swap"
    !bump <- @Button $"Bump"
    $a <- 0 <= $a + 1 on !bump
    $b <- 100 <= $b + 1 on !bump
    $use_b <- False <= (not $use_b) on !swap
    !$chosen <- dyn $use_b:
        @ (if _: $b else: $a)
    $$current <- $ $a <= !$chosen
    @Label "{$Join $$current}"

test join_is_prompt: Test @join_law: array:
    Expect Ui.Label(text="0")
    Send Ui.Button(label="Bump"), click
    Expect Ui.Label(text="1")
    Send Ui.Button(label="Swap"), click
    Expect Ui.Label(text="101")
    Together:
        Send Ui.Button(label="Swap"), click
        Send Ui.Button(label="Bump"), click
    Expect Ui.Label(text="2")
```

The last step is the hard case. The outer changes (back to `$a`) and the new inner changes (`$a` goes to 2) in the same occurrence, and the join shows 2: the new inner's value *after* the occurrence. An implementation built from a non-prompt switch plus a sample at swap time shows 1, the new inner's value before, and its label disagrees with its holds from then on.

The third case is the outer switching to an inner that does not change in that occurrence, and it has its own script so that a failure names the case:

```prism
@join_quiet_inner:
    !swap <- @Button $"Swap"
    !bump_a <- @Button $"Bump A"
    $a <- 0 <= $a + 1 on !bump_a
    $b <- 100 <= !never
    $use_b <- False <= (not $use_b) on !swap
    !$chosen <- dyn $use_b:
        @ (if _: $b else: $a)
    $$current <- $ $a <= !$chosen
    @Label "{$Join $$current}"

test join_follows_quiet_inner: Test @join_quiet_inner: array:
    Send Ui.Button(label="Bump A"), click
    Expect Ui.Label(text="1")
    Send Ui.Button(label="Swap"), click
    Expect Ui.Label(text="100")
    Send Ui.Button(label="Bump A"), click
    Expect Ui.Label(text="100")
    Send Ui.Button(label="Swap"), click
    Expect Ui.Label(text="2")
```

`$b` never changes: its hold is fed by `!never`. Swapping to it shows its current value, 100, in the swap's occurrence, with no change event from the inner to help; an implementation that only updated the join on the inner's change would keep showing 1. Bumping `$a` while `$b` is chosen changes nothing visible, and swapping back shows the 2 that `$a` reached while it was not being watched: the join reads the inner's current value when it adopts it, and that value is whatever the inner's own history made it. Chapter 5 says how `$Join` is built from a hold of the inner's change events and a prompt read of the inner at the swap; that construction has to pass this script.

### A nested `dyn` built in the occurrence its dynamic changed builds once, with the new value

```prism
@nested_law:
    !next <- @Button $"Next"
    $page <- 0 <= $page + 1 on !next
    let $title = "Page {$page}"
    dyn $page:
        dyn $title:
            !started <- @started
            @Element Test.Log(requests=(_ on !started))
            @Element Ui.Label(text=_)
    @ unit

test nested_dyn_builds_once: Test @nested_law: array:
    Expect Ui.Label(text="Page 0")
    Requested Test.Log, "Page 0"
    Send Ui.Button(label="Next"), click
    Expect Ui.Label(text="Page 1")
    ExpectNone Ui.Label(text="Page 0")
    Requested Test.Log, "Page 1"
```

The inner `dyn` is built partway through the click's occurrence, after `$title` has changed. An implementation that hid the earlier change from it would leave `Page 0` in the list, and the `ExpectNone` catches that. An implementation that built `Page 0` and then rebuilt would fire `Test.Log` twice in that turn, and the turn's transcript, which lists every request firing per slot, shows two where the law says one; the differential test compares transcripts, so the doubled firing is visible even though the script's `Requested` step only asks whether the right one fired.

### A hold inside a rebuilt block starts over, and sees the occurrence it was born in

```prism
@reset_law:
    !rebuild <- @Button $"Rebuild"
    !bump <- @Button $"Bump"
    $gen <- 0 <= $gen + 1 on !rebuild
    dyn $gen:
        $n <- 0 <= $n + 1 on !bump
        @Label "n={$n}"
    @ unit

test hold_resets_and_catches_up: Test @reset_law: array:
    Send Ui.Button(label="Bump"), click
    Send Ui.Button(label="Bump"), click
    Expect Ui.Label(text="n=2")
    Send Ui.Button(label="Rebuild"), click
    Expect Ui.Label(text="n=0")
    Together:
        Send Ui.Button(label="Rebuild"), click
        Send Ui.Button(label="Bump"), click
    Expect Ui.Label(text="n=1")
```

`!bump` is captured from outside the `dyn`; the hold is inside it. A rebuild makes a new hold at 0. The `Together` builds a new block in an occurrence in which `!bump` has already fired, and the new hold adopts it: `n=1`, exactly what a hold that had existed from the start of the occurrence would show.

### Nothing answers in the occurrence it was asked

```prism
@creation_law:
    !started <- @started
    !back <- @Element Echo(requests=("hi" on !started))
    $got <- "nothing" <= !back
    @Label $got

test no_answer_in_the_request_occurrence: Test @creation_law: array:
    Expect Ui.Label(text="nothing")
    Requested Echo, "hi"
    Send Echo, "hi"
    Expect Ui.Label(text="hi")
```

The request fired in the build occurrence, from `@started`, which is pure; the label after the build still says `nothing`, because no response can arrive in the occurrence of its request. The script answers by hand in the next turn. This is also the test that there are no events from nothing: without the `Send`, the label never changes, however many turns the script runs.

### A prompt cycle formed through a widget function is reported, not run

```prism
@Echo: fn !Int -> @!Int = \|!in| @widget:
    !started <- @started
    @ (!in + 1 | 0 on !started)

@runtime_cycle:
    !out <- @Echo !out
    @ unit

test cycle_is_an_error: Test @runtime_cycle: array:
    Errors Error(kind=Cycle, ..)
```

The block that binds `!out` cannot see that `@Echo`'s result is prompt in its argument, so the compiler lets it through. In occurrence zero `@started` fires, `!out` fires with 0, `!in + 1` fires with 1, which is `!out` again, in the same occurrence. The runtime reports a cycle and the build turn ends in an error: one `Error` value with `kind=Cycle`, the spans of the nodes in the chain, and the occurrence number. What the World does with it, an aborted occurrence or a subtree in quarantine, is the error chapter's. The step says only that the turn ended in a cycle error and that the implementation did not hang.

A cycle of the same kind inside one block is not a test in this suite, because it never runs: `let !a = 1 + !b` with `let !b = 2 + !a` is rejected by the compiler with both bindings named, and the conformance suite's compiler corpus holds it as a program that must not compile.

### Determinism

Every script above is a determinism test twice over. Each is run under both implementations and the transcripts compared turn by turn, and each is run more than once under each, which is cheap to do and catches the implementation that depends on an iteration order of a hash map or on the address of a node. A script that passes once and fails once has found a bug in an implementation; a script whose two implementations disagree has found a bug in one of them or an ambiguity in a sentence, and the second kind of find is the more valuable.

## The negative space

Four things are missing from this part of the language, and each absence is a feature. The pattern is the same each time: the thing you would have sworn was essential is removed, the removal closes a hole you have fallen into, and the same removal lets an implementation leave out a mechanism it would otherwise need. The four are stated together here because they are the four that people ask for first, and because their reasons overlap.

### No sampling

No `sample`, no `.current`, no plain function from `$A` to `A`; a dynamic is read through a tag, through `dyn`, or by derivation, and "No sampling, and what it buys" above has the reasoning. What it closes: the read in the middle of the cascade. What it buys the program: every read of state is visible as a tag. What it buys the implementation: freedom of order, and no definition of "a sample in the middle" that every later optimisation would have to honour.

### No prompt switching

`!Switch` hears a new inner from the next occurrence; there is no `coincidence`, no `switchPromptly`, and no way to write one, for the reasons given under "`!Switch`" above. What it closes: the cycle that exists only when a particular inner is chosen, and the re-entrant propagation that handling it needs. What it costs: a block cannot fire at its parent through the switch in the occurrence of its birth, and the parent already knows of the birth from the `dyn`'s event. What it buys the implementation: heights fixed as the occurrence opened, one pass, no re-entry, no second node type for switching.

### No events from nothing

No `!always`, no `emit`, no `setTimeout`, no `requestAnimationFrame`, no way for a Prism program to cause an occurrence or to fire an event that is not a consequence of an input or of construction.

What it closes: the ambient channel. A widget that could fire on its own could observe or affect things it was never handed. A widget that could observe every occurrence could learn about siblings it has never met. A program that could schedule its own occurrences would have a scheduler, with priorities and starvation and a cap, written somewhere in the runtime, and the program's behaviour would depend on it.

What it buys the program: a transcript is a test. The World's inputs are the program's entire input, and replaying them replays the run.

What it buys the implementation: one entry point. The runtime runs one occurrence when asked and returns; it never opens one on its own, keeps nothing queued, and has no thread, no timer, and no wakeup. A World that wants something to happen later owns the timer, and that is the World's business, in the host language, where timers belong. The runtime is a function from "this input arrived" to "this occurrence happened", and functions are the thing both implementations know how to test.

### No clock

There is no `now()`. Time is a state element, `Clock`, like the contents of a file: it has a value in the occurrence its slot is created and the World pushes a new one when it chooses. A program that wants to be woken asks a `Time.After` or `Time.Every` element, and is woken in an occurrence the World opens.

What it closes: the untestable debounce. Anyone who has tried to unit-test a timeout with a real timer knows what a clock that a script can set is worth. It also closes the program whose behaviour depends on how long an occurrence took to compute, which is a program whose bugs appear only under load.

What it buys the program: `Step $watch, $clock, !action` has every input it needs in its arguments. The stopwatch's state machine is a plain function, and it runs identically under a GUI World, a server World and a test that says `Push Clock, Instant 5100`.

What it buys the implementation: nothing in the runtime knows what time it is, so nothing in the runtime has to be told, and two implementations cannot disagree about it. Determinism would be false the moment a program could read a wall clock; with the clock as data, it is a theorem.

### What the four have in common

Each is a refusal to let a program observe something about *when* it is running: the instant within an occurrence, the occurrence after the one it is in, an occurrence it did not cause, the wall-clock time. Only what the laws define remains observable: before-values, firings at this occurrence, and the values the World pushed in. The laws are small because the observables are few, and the observables are few because these four doors are shut.

## How this feels in practice

The laws are the whole language, and after a few programs they stop being laws and become habits. Four of the habits, each small, each something you will write in the first hour, with the law it leans on named once.

### Fold with hold

All state has the form of the counter's line:

```prism
$count <- 0 <= $count + !delta
```

A fold is a hold whose change event reads the hold through a tag. The initial value on the left, the step on the right, the dynamic on both sides. Anything a `reduce` can express is this line with a different function:

```prism
$total <- 0 <= $total + !amount
$history <- [[]] <= Append $history, !line
$state <- Idle <= Step $state, !action
$best <- Nil <= Better $best, !score            # Better: fn ?Int, Int -> ?Int
```

When more than one event moves the state, the merge picks who wins when they coincide, and the hold is still one line:

```prism
$count <- 0 <= 0 on !reset | $count + !delta
```

A reset in the same occurrence as an increment is a reset, and the next person to read the line knows it. When two events should *both* apply, the merge is wrong and the right tool is a map through `for`'s consolidation or a library `!Union`, folded in key order, as in the todo list; the hold is still one line, and the step function says what it means to apply several changes at once.

When the step needs another dynamic, it goes in the operation and is read through a tag:

```prism
$watch <- (Watch 0, Stopped) <= Step $watch, $clock, !action
```

The clock is the clock before the occurrence. If the same occurrence pushes a new clock, the stamp is still the old one; that is the tag law, and a program that wants the newer time cannot have it, because at an event there is one version.

The habit to acquire: when you want state, write the hold first and let its line be the complete answer to "what can change this". If the line is getting long, the step is a function with a name; if the merge is getting long, the events want a `choice` type and one merge of tagged actions, like `Start on !start | Stop on !stop | Reset on !reset`.

### Gating with `on`

`$x on !e` is "the value when this happens", and most of what people do with `sample` is this:

```prism
let !submitted = $text on !enter              # the text, as it stood, when Enter was pressed
let !chosen = $items[$cursor] on !choose      # an optional, since the index might be out of range
let !stamped = (\|k, t| ((k, t))) !key, $clock  # the key that fired, paired with the clock before it
```

The third line is the one everyone first writes as `(($clock, !key))`, which is a plain tuple holding a dynamic and an event, and does not fire. The sigil's position says what moves, and in a tuple nothing does.

To drop firings on a condition, lift a function that returns an optional and filter:

```prism
let When = \|!e, $open| !Filter ((\|x, g| if g: Some x else: Nil) !e, $open)
let !accepted = When !submitted, not $busy
```

`When` is a plain function that happens to take an event and a dynamic, so its name has no sigil; the sigil goes on what it returns, and `!accepted` is an event because `!Filter` returns one.

`$busy` is read through a tag, so a submit in the same occurrence as the response that clears `$busy` is dropped: the program was busy before that occurrence, and the submit sees before. If that is the wrong answer for your program, the fix is visible: fold `$busy` from a merge in which the clearing wins, and gate the *next* submit, or make the gate a dynamic the response sets promptly and read it one occurrence later. Either way the choice is in the text.

A gate that depends on the event's own value is just a function: `!Filter (\|n| if n lt 10: Some n else: Nil) !n` keeps the small ones. A gate on a dynamic and the value together is one lifted function with both as arguments. There is no separate `when`, `gate` or `filterWhen` to learn; there is lifting, `on`, and `!Filter`.

### Debouncing with a Time element

You want to search as the user types, but not on every keystroke: wait until they have stopped for three hundred milliseconds. In every framework this is a timer you cancel and restart, and the cancel is where the bug lives, because the callback that was already in flight when you cancelled it still runs. Here there is no cancel, because there is no callback. A timer is an element that answers later, and "later" is an occurrence in which the program checks whether the answer is still wanted.

```prism
tag Ui.TextInput of Element:       # the GUI library's text input; the Test World sends it the whole text
    name: Text
    responds: Text

tag Time.After of Element:
    delay: Int                     # milliseconds
    requests: !Nat                 # a token; the World answers with the same token after `delay`
    responds: Nat

@Debounce: fn Int, !a -> @!a = \|delay, !in| @widget:
    $gen <- 0 <= $gen + 1 on !in
    !back <- @Element Time.After(delay=delay, requests=($gen + 1 on !in))
    $latest <- Nil <= Some !in
    let Settle = \|token, gen, latest| if token == gen: latest else: Nil
    @ !Filter (Settle !back, $gen, $latest)
```

Every input increments a generation and asks the timer for that generation back. When a token comes back, it is compared with the generation as it stands; if a newer input has moved the generation on, the token is stale and is dropped; if not, the latest input is released. `$gen + 1 on !in` reads the old generation and requests the new one, which is the number the hold will have after the occurrence; `Settle !back, $gen, $latest` reads both holds before the occurrence the token arrives in. There is no cancellation. Stale timers go off and are ignored: every token requested is answered, the World owes an answer to each one it accepted, and the program's three lines decide which answers matter. The only tokens the World does not answer are the ones addressed to a slot that no longer exists, because the `@Debounce` that asked was torn down; the World performs the timer anyway and discards the result, under its obligation to discard late results for gone slots. Nothing is cancelled anywhere, and nothing leaks, because a timer is a request the World owns and a slot that is gone is a slot the World knows is gone.

`Time.Every` is the other timer, and it is not a debounce. It is a stream element that ticks once per period for as long as the slot exists; it takes no request and answers no token, so there is nothing to compare a tick against, and a tick that lands while the program is busy is skipped rather than queued. A debounce needs "later than *this* input", which is a request, and a request is `Time.After`.

Occurrence by occurrence, with a 300 ms delay: a keystroke at occurrence 4 requests token 1 and `$gen` commits 1. A keystroke at 5 requests token 2, `$gen` commits 2. The World answers token 1 at occurrence 6: `$gen` before 6 is 2, the token is 1, `Settle` returns `Nil`, nothing is released. The World answers token 2 at occurrence 9: `$gen` is 2, the token is 2, `$latest` is released. If a keystroke and a token arrive in the same occurrence, the token is compared against the generation before the keystroke and the keystroke's own input is in `$latest` only from commit, so the release, if any, is the previous input and a fresh timer is requested for the new one. Every case is decided by the tag law and nothing else.

```prism
@debounced_search:
    !typed <- @Element Ui.TextInput(name="query")        # the text as it stands after each edit
    $query <- "" <= !typed
    !settled <- @Debounce 300, !typed
    $shown <- "" <= !settled
    @Label "searching: {$shown}"

test debounce_waits_for_the_last: Test @debounced_search: array:
    Send Ui.TextInput(name="query"), "a"
    Requested Time.After(delay=300), 1
    Send Ui.TextInput(name="query"), "ab"
    Requested Time.After(delay=300), 2
    Send Time.After(delay=300), 1
    Expect Ui.Label(text="searching: ")
    Send Time.After(delay=300), 2
    Expect Ui.Label(text="searching: ab")
```

The test owns the timer. It can answer the tokens in any order, together, or never, and each choice is a case the program handles by the same three lines. Under a real World the text input is the library's `@TextInput` over `Ui.TextInput`, fed by `Gui.Window`'s key events through the library's focus routing; the debounce does not know and does not need to. This is the "no clock" absence paying for itself: a debounce is testable without waiting, and the program that was tested is the program that ships.

### Accumulating a map

A stream of words, and you want a count per word, shown as a list that updates in place:

```prism
tag Words of Element:
    responds: Text

let Bump: fn Text ==> Nat, Text -> Text ==> Nat = \|m, w| match m[w]:
    Some n: Insert m, w, (n + 1)
    Nil: Insert m, w, 1

@word_counts:
    !word <- @Element Words
    $counts <- {} <= Bump $counts, !word
    for $counts:
        @Label "{key}: {$_}"
    @ unit
```

The hold is the fold-with-hold habit over a map: one line, one event, one step function. The `for` builds one label per key. Inside the block, `key` is the word and `$_` is the count *as a dynamic*: when a word that already has a row is counted again, the row's `$_` changes and its label rebuilds, and the row itself, its node, its path, any hold it might have, stays. A new word is a new key and a new row, built in the occurrence the word arrived. A word never disappears here, but if `Bump` removed keys at zero, the rows would be torn down at that occurrence's commit.

This program uses a map because the key is an identity. An array of pairs would make every row's position its identity, and a new word sorting into the middle would rebuild every row after it. Keys are ordered, so the labels appear in word order, which is what a map is for.

Two refinements that are both one line. Several words in one occurrence, if the element delivered chunks: make the response a `[Text]` and the step a fold over it, `$counts <- {} <= Fold !words, $counts, Bump`, where `Fold` is the plain function and `$counts` is read once, before. And a per-row piece of state that is not in the model, a highlight toggled by clicking the row: a hold inside the `for` block, owned by the key, which survives every other key's arrival and departure and dies with its own:

```prism
let Mark: fn Bool -> Text = \|m| if m: " *" else: ""

    for $counts:
        !click <- @Button $"{key}"
        $marked <- False <= (not $marked) on !click
        @Label "{key}: {$_}{Mark $marked}"
```

`$"{key}"` is a constant dynamic of the key, which `@Button` takes as its label. Nothing about `$counts` changed to make room for it. That is the whole experience of the language in miniature: state is a hold, it lives where it is written, and the structure of the program is the structure of the data.

