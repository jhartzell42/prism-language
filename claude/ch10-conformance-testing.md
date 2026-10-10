# Chapter 10 — Conformance and testing

A specification that only one implementation can check is a description of that implementation, and the way out is to test the language from the only place where two implementations must look identical: the World's side of the boundary. Every test in this chapter sends plain data to elements and reads plain data back, the way a real World does, and never names a node, a path, a hold or a runtime object. That choice decides everything else here: the Test World that accepts every tag and has no effects; the script language that drives a program one turn at a time; the law tests, one short script per sentence of the chapters on the functors, on time, on widgets and switching, and on elements and Worlds; the integration programs that each pin one piece of meaning; the layout corpus and the typed-AST goldens that pin the frontends; the runtime's own unit tests and the invariant they check after every occurrence; the World tests against the world core; and the differential test that runs all of it under both implementations and diffs the output. The chapter ends with the one rule every test obeys: it is quotable, and if it cannot be stated as a sentence about the language, it is deleted.

## Tests against what a World sees

You have written the test that reaches into a component and reads its private field. It passed. Then someone renamed the field, or moved the state up a level, or replaced the component with one that computes the same thing differently, and the test broke without the program being wrong. Later someone else made the test pass again by exporting the field, and now the field is part of the interface because a test wanted it to be. That is the ordinary life of a white-box test, and a conformance suite cannot live that way, because it has two implementations to be true for at once, and their insides have nothing in common: one is a graph of `Rc` nodes with heights and the other is a Reflex `Spider` timeline. The only thing they share is what comes out of the run call.

So the suite is written against the run call. A test is a program and a script; the script delivers responses and pushes, which is what a World does, and checks the element list, the request firings, the result and the errors, which is what a World sees. The spec promises that these are fully determined by the inputs and the snapshots, which is the determinism law of the chapter on time, and that promise is what makes a script a test: the same script gives the same transcript under every conforming implementation, and a transcript that differs is a bug or an ambiguity, never a matter of taste.

This has a consequence the language already chose for other reasons. There are no reified identifiers, no `Inspect $count`, no way to poke a hold. Suppose there were: a step `Peek $count, 2` that read a named dynamic inside the widget under test. Convenient for the counter. Then the counter is rewritten with the count held inside a `dyn`, and `$count` is now a name in a block the step cannot reach; or the count becomes a field of a record held under another name; or the implementation under test is the Haskell one, in which a name is a `Dynamic t Int` in an evaluator environment, and the step needs a second evaluator feature to find it. Each of those is work, and none of it tests the language. Worse, every test that peeks is a test that passes under the Test World and says nothing about any other World, because no other World can peek. A test that clicks the button and reads the label passes under the Test World and under the GUI World and under a shell written in Prism, because every one of them clicks and reads the same way. The suite tests the language, not a program's private names.

The exclusions are as deliberate as the inclusions. The suite does not test performance or memory; a slow implementation that produces the right transcripts conforms. It does not test any World other than the Test World; a CLI World's obligations are tested per World, in the host language, in a later section. It does not test the order in which an implementation fires nodes within an occurrence, because the language only promises the observable consequences of that order: each event at most once, each dynamic changed at most once, no node seeing a prompt source before it settles. And it does not test diagnostics' wording except in the negative goldens, which are a frontend concern and say so.

## The Test World

The Test World is a World with no effects, written once per host language on top of the world core, with one component that answers to a script instead of to an operating system. Three properties make it the suite's World.

**It accepts every tag at the root.** A program that creates a `Net.Listener` is tested without a network; one that creates a `Ui.Button` is tested without a screen; one that creates a tag nobody has heard of is tested all the same. The Test World does not know what any tag means. It knows that a `requests` field fires values it records, that a `responds` field is an event it can deliver to by path, and that a `state` field is a dynamic it answers from a snapshot the script sets. That is everything a World does with an element, and it is why the Test World can be written before any real World exists: it is the obligations of a World and nothing else. Suppose instead the Test World had components, a fake filesystem with files the script writes, a fake clock that ticks. The tests would then be about those fakes: a fake filesystem that answers reads has an opinion about what a read is, and a program that expects `Missing` for an absent file is testing the fake's opinion. With the script as the only source of inputs, the program's expectation is the script's expectation, written next to the program, and a reader can see both.

**Its snapshot is whatever the script says.** A state element's value in its creation occurrence comes from the oracle, and the oracle answers from the previous snapshot; the script's `State` steps are that snapshot. There is no clock in the Test World except the one the script pushes, which is the goal that there is no clock, applied to testing: a debounce, a timeout, a stopwatch are tested by setting an `Instant` and pushing another, and a transcript replays them exactly.

**It holds no runtime object.** The Test World reads the run call's outcome and nothing else. It does not read behaviors, does not hold event handles, does not receive callbacks. The whole of what it knows about the program after a turn is the element list, the request firings per slot, the result value and any error, which is the same four things a GUI World knows. This is the property the differential test depends on, because the Haskell Test World cannot hold a Rust `Rc`, and it is the property that keeps the suite honest: anything the Test World could learn by holding a runtime object would be something no real World could learn.

A test builds a widget under this World and runs steps in order; the first failing step fails the test, and the failure message prints the script up to that step and the transcript of the turn it failed in.

### The transcript

Everything the Test World sees is printed, turn by turn, in one canonical text form that both implementations produce. The conformance harness compares transcripts; the differential test diffs them; a failing step prints the one it failed in. Here is the counter's first two turns:

```
turn 0
  list:
    0/#0/0: Ui.Button(label="Increment")
    1/#0/0: Ui.Button(label="Decrement")
    2/#0/0: Ui.Label(text="Count: 0")
  result: unit
turn 1  responses: {0/#0/0: click}
  list:
    0/#0/0: Ui.Button(label="Increment")
    1/#0/0: Ui.Button(label="Decrement")
    2/#1/0: Ui.Label(text="Count: 1")
  result: unit
```

A turn header names the turn and its inputs: `responses` by path, `pushes` by element value. The list is one slot per line, `path: element`, in tree order. A slot that fired its request in the turn gets a `requested` line after it, with the values in firing order; an occurrence boundary inside a turn, which only `Defer` causes, is marked `occurrence 2` between the parts of a turn so that a reader can see which occurrence a request fired in. The result is printed when the widget's result is a plain value or a dynamic (by its current value) and as `fired: v` or `quiet` when it is an event. An aborted occurrence prints `error: Error(kind=UnknownPath, path=0/#1/0, occurrence=1)` and no list, since the list did not change.

Paths are opaque to Worlds, and the transcript still has to print them, so the transcript fixes a spelling that both implementations use: bind positions from the root joined by `/`, a `dyn` generation as `#n` from `#0`, a `for` key as `[k]` with the key printed as a value. That spelling is the chapter-2 convention and it is part of the interchange format. An implementation may represent paths however it likes inside, and the Haskell runtime's paths are nothing like the Rust runtime's, but what they print must match character for character or the differential test fails. This is a real constraint, deliberately taken: two implementations that agree on every element but number bind positions differently would be two readings of the sentence "its position in the widget tree, made of the bind positions, `for` keys and `dyn` generations", and the point of the differential test is to find exactly that kind of difference. Slots created by widgets that an interceptor's supplier built print as `s/n`, numbered in creation order within the interceptor's node, because their position is the runtime's business and only their existence is a law.

Element values print as their constructor with named fields in declaration order; `Path ==> Element` fields print with their keys as paths, which is how a `Ui.VBox`'s children appear. Plain values print as Prism literals. Nothing in the transcript is implementation-specific, which is the test of whether it belongs there.

## The script language

A test is a top-level statement:

```prism
test counter: Test @counter: array:
    Expect Ui.Label(text="Count: 0")
    Send Ui.Button(label="Increment"), click
    Expect Ui.Label(text="Count: 1")
```

`test name:` opens a block whose one statement is `Test widget: array:` followed by the steps, one per line. The widget is any widget expression; most tests name a top-level widget, and an interceptor test wraps one: `Test (@Sandbox "/jail", @probe): array:`. The steps are the `array:` block's elements, and that is not an accident of syntax: a script is a plain array of step values, parsed by the frontend as part of a Prism file, and the harness is a function from a widget and an array of steps to a transcript and a verdict. The `test` form exists so that the frontend can check the patterns against the tag declarations in scope, which it could not do for a string.

### Patterns

A pattern is what `match` accepts for an element, written where the step wants one: a tag with named fields and `..` for the rest, `Ui.Button(label="OK", ..)`; a tag with every field, `Ui.Label(text="Count: 0")`; a bare tag for a tag with no fields, `Clock`; `Ns.*(..)` for any tag in a namespace; `ignore` for anything. Field patterns are value patterns: a literal, a nested element pattern, `..` inside a record, `ignore`. Because element fields are plain data fixed for the slot's life, there is no "current value" of a field to compare; a label whose text changed is a different slot with a different element, and a pattern that matched the old one does not match the new one. That is why `Expect Ui.Label(text="Count: 1")` after a click is a complete check: the old label is gone from the list and the new one is in it.

Response and state values in a step are plain Prism expressions of the tag's declared type: `click`, `Contents "hi"`, `Instant 5100`, `{3: Toggle}`. The frontend checks them against the declaration of the tag the pattern names, so a `Send` of a `Tick` to a button is a compile error in the test file rather than a runtime surprise. When the pattern is `ignore` or `Ns.*(..)`, the value is checked at the slot when the step runs.

### The steps

Each step is a check between turns or an action that runs a turn. Here is every one, with its meaning and a script that uses it. The counter, the viewer and the stopwatch are chapter 2's; `@probe` and the rest are defined where they appear.

**`Expect pattern`.** Some slot in the current list, at any depth, matches. Depth means inside container elements by the GUI library's convention, below.

```prism
test expect_sees_the_built_program: Test @counter: array:
    Expect Ui.Label(text="Count: 0")
    Expect Ui.Button(label="Increment")
```

**`ExpectTop pattern`.** Some root-level slot matches: one the Test World itself received, not one found inside a container's `children` field. The difference is what a container test is about:

```prism
test vbox_holds_its_children: Test (@VBox @counter): array:
    ExpectTop Ui.VBox(..)
    ExpectTopNone Ui.Button(..)
    Expect Ui.Button(label="Increment")
```

**`ExpectNone pattern`** and **`ExpectTopNone pattern`.** No slot at any depth, or no root-level slot, matches. `ExpectNone` is how a test says that something is gone, which is the half of a rebuild that a positive check cannot see:

```prism
test rebuild_removes_the_old_label: Test @counter: array:
    Send Ui.Button(label="Increment"), click
    Expect Ui.Label(text="Count: 1")
    ExpectNone Ui.Label(text="Count: 0")
```

**`ExpectCount pattern, n`.** Exactly `n` slots at any depth match. `ExpectCount p, 0` is `ExpectNone p`; `ExpectCount p, 1` is the step to use when the test needs there to be one of something, which `Expect` does not say.

```prism
test one_row_per_key: Test @tallies: array:
    ExpectCount Ui.Label(text="a: 0"), 1
    ExpectCount Ui.Label(text="b: 0"), 1
    ExpectCount Ui.Button(..), 4
```

**`Send pattern, response`.** Exactly one slot must match the pattern; otherwise the step fails before anything is delivered, with the count it found. The response is delivered to that slot's path as the sole input of a new occurrence, and a turn is run: that occurrence plus any `Defer` occurrences it causes. The requirement that exactly one slot match is what keeps a test from sending a click into an ambiguity: two buttons labelled "OK" need a pattern that tells them apart, or a `Together`.

```prism
test send_runs_a_turn: Test @counter: array:
    Send Ui.Button(label="Increment"), click
    Expect Ui.Label(text="Count: 1")
```

**`Together:`** block of `Send`s. All of them are delivered in one occurrence, which the program sees as simultaneous. Two `Send`s that resolve to the same slot fail the step, because a slot receives at most one response per occurrence; a World that has two things for one slot delivers them in two occurrences, and a script says that by writing two `Send`s in a row. The block is the only way a script makes two inputs simultaneous, and it is how every simultaneity law is tested:

```prism
test together_is_one_occurrence: Test @counter: array:
    Together:
        Send Ui.Button(label="Increment"), click
        Send Ui.Button(label="Decrement"), click
    Expect Ui.Label(text="Count: -1")
```

**`State pattern, value`.** Sets the snapshot value for the state element whose *value* matches the pattern. The pattern here is matched against element values, not slots: `State Clock, Instant 5000` sets the snapshot entry for the element `Clock` whether or not any slot has asked for it yet, and `State Fs.Contents(path="/etc/motd"), Known "hi"` sets the entry for that one path. A pattern with `..` or `ignore` in it is a script error, because a snapshot entry is keyed by a whole value. The step runs no turn. Its effect is the next turn's: as that turn's occurrence opens, the oracle answers from the snapshot *before* this `State`, and the difference between that and the snapshot after it is the push. A `State` before the first turn is the snapshot the build occurrence's oracle answers from, which is how a test gives a clock a starting value:

```prism
test state_before_build_is_the_oracle: Test @stopwatch: array:
    State Clock, Instant 5000
    Expect Ui.Label(text="0 ms")
```

Two `State`s for one element before a turn: the second replaces the first, and the push is the difference between the previous turn's snapshot and the final one, since a World gathers everything that changed into one snapshot per occurrence.

**`Push pattern, value`.** `State` followed by a turn with no responses. The turn's only input is the push; a program with no slot for that element runs an occurrence in which nothing fires, which is still a turn and still prints.

```prism
test push_moves_the_clock: Test @stopwatch: array:
    State Clock, Instant 5000
    Send Ui.Button(label="Start"), click
    Push Clock, Instant 5100
    Expect Ui.Label(text="100 ms")
```

A `State` followed by a `Send` makes a turn with both a push and a response, which is the only way a script puts a state change and an input in one occurrence, and it is what the law about a push reaching a slot created in the push's occurrence needs.

**`Returns pattern`.** The widget's result matches. A plain result is compared as a value; a dynamic result is compared by its value after the last turn; an event result makes `Returns` a script error, since an event has no value between occurrences, and the step to use is `Fired`. A widget returning a bundle is matched with a record or tuple pattern, with `ignore` for the parts the test does not care about.

```prism
test returns_reads_a_dynamic: Test @accumulator: array:
    Returns 0
    Send Test.Accumulator(..), Add 3
    Returns 3
```

**`Fired pattern`** and **`Quiet`.** The widget's result is an event; `Fired` passes if it fired at some occurrence of the last turn with a value matching the pattern, and `Quiet` passes if it fired at no occurrence of the last turn. Both look at the whole turn, not only its first occurrence, because a World sees turns: a value that came back through a `Defer` is, to the World, a consequence of the input it sent. Both are script errors on a widget whose result is not an event.

```prism
@ok_button:
    !b <- @Button $"OK"
    @ "ok" on !b

test fired_and_quiet: Test @ok_button: array:
    Send Ui.Button(label="OK"), click
    Fired "ok"
    Push Clock, Instant 0
    Quiet
```

Before any turn, `Quiet` holds and `Fired` fails: turn 0 is the build, and the result event of a widget cannot fire in turn 0 through a `!Switch`, though a widget whose result is `@started` directly fires in turn 0 and `Fired unit` holds then. The law test for `@started` uses exactly that.

**`Requested pattern, value`.** Some slot matching the pattern fired its request during the last turn with a value matching the value pattern. `ignore` as the value matches any firing. A slot whose tag has no `requests` field fails the step with a message saying so, rather than quietly never matching. The counterpart is **`Unrequested pattern`**: no slot matching the pattern fired its request during the last turn. The negative form is in the language because the most important law in the suite, that creating an element requests nothing, cannot be stated without it.

```prism
test started_requests_in_turn_zero: Test @viewer: array:
    Requested Fs.File(path="/etc/motd"), Read
    Send Ui.Button(label="Refresh"), click
    Unrequested Fs.File(..)
```

(Chapter 2's viewer has no Refresh button; the script is illustrating the step and a version with the button is in the law tests.)

**`Errors pattern`.** The last turn ended in a runtime error matching the pattern. Errors are `Error` values with a `kind`, so the patterns are record patterns on the kind and the fields the test cares about: `Error(kind=UnknownPath, ..)`, `Error(kind=DeferBackwards, ..)`, `Error(kind=Cycle, ..)`, `Error(kind=UnsuppliedState, element=Clock, ..)`, `Error(kind=Overflow, ..)`. When an occurrence aborts, its holds do not commit, its new nodes are dropped, the element list is what it was before the occurrence, and the turn ends there; a check after an `Errors` step sees the list as it stood before the failed turn. A turn that ends in an error with no `Errors` step following fails the script, so a test cannot pass by accident through a crash. The tags and fields of `Error` are tentative in the way the whole error story is: if the unit of failure becomes a quarantined subtree rather than the occurrence, `Errors` will still match the error and a later `Expect Error(..)` will see the quarantine element in the list, and nothing else in the script language changes.

```prism
tag Secret of Element:
    state: Text

@needs_secret:
    $s <- @Element Secret
    @Label $s

test unsupplied_state_fails_at_creation: Test @needs_secret: array:
    Errors Error(kind=UnsuppliedState, element=Secret, ..)
```

That script has no `State Secret` before the build, so the slot's creation in occurrence 0 is the error and turn 0 is the turn that ends in it. The same script with `State Secret, "open"` as its first line builds a label showing `open`.

### Every state element needs a `State` first

A tag declaration says what an element means and nothing else, so a state element has no value until someone supplies one. In the Test World the supplier is the script: a program that creates a `Clock` at build has `State Clock, Instant 0` as the first line of its test, and a program that creates one later, say on a click, has the step before the `Send`. Without it the slot's creation is the unsupplied-state error above.

```prism
test a_clock_needs_a_value_before_the_build: Test @stopwatch: array:
    Errors Error(kind=UnsuppliedState, element=Clock, ..)

test and_has_one_after_it: Test @stopwatch: array:
    State Clock, Instant 0
    Expect Ui.Label(text="0 ms")
```

The line is noise in a test that is not about the clock, and it is also the honest line: a real World answers its oracle from its own knowledge, a World without the component cannot answer at all, and the script says which of those it is pretending to be. Suppose instead each tag could declare a default value. Every test would then pass without a line that a real World has to earn, and a program that works under the Test World would fail on the first World without the component, which is a bug the suite exists to catch before it ships.

### Depth: the GUI library's convention

`Expect` finds a button inside a box, and `Send` clicks it, by a convention that belongs to the suite's GUI library and not to the language. The library's container tags hold their children's elements as data in a field named `children` of type `Path ==> Element`, keyed by the child's path in the container's own child list, and respond with a `ChildInput`:

```prism
type ChildInput = record:
    path: Path
    response: Response

tag Ui.VBox of Element:
    children: Path ==> Element
    responds: ChildInput
```

`Expect` searches every `children` field recursively, so a pattern matches a slot at any depth. When `Send` resolves to a slot inside one or more containers, the Test World wraps the response in one `ChildInput` per level, innermost first, and delivers the result to the outermost container's path, which is the only path it has. The delivery then takes the route a real World's input would: the container's `@VBox` receives a `ChildInput`, routes its response to the child path through its `intercept`, and if that child is itself a container, the same again. A test cannot bypass the library's routing, which is the point; the law that routing is against the opened list applies at every level and a test of a nested button tests all of them.

The library is part of the suite, which is why the Test World can know its convention without the language knowing it. A different library with a different convention, say children in a `rows` field, would ship its own depth rule with it, and a Test World knows the convention of the library it was built with. The language fixes only that leaves are elements and containment is interception; the spelling of a container's children belongs to the library.

The suite's library is small and written out in full later in this chapter, because the integration programs are built on it and because its own typed-AST dump is one of the goldens.

### Turns, and when checks run

The build is turn 0. It is a real turn: occurrence 0 plus any `Defer` occurrences the build causes, and a check before the first `Send` sees the end of it. Every `Send`, `Together` or `Push` is one turn: the occurrence it opens plus every `Defer` occurrence that follows until no `Defer` request is outstanding. `State` and the checks run no turn. A check always runs between turns, against committed state, which is the only state a World ever reads.

Those sentences have consequences a reader gets wrong the first time, and the suite has a test for each one. A check never sees the inside of a turn: there is no step that observes occurrence 1 of a two-occurrence turn, because no World can. A `Defer` answered in a turn is answered before the next check, so a sandbox's denial is visible in the first `Expect` after the request, not one step later. `Fired` and `Quiet` are about the whole turn, so a result event that fires in a `Defer` occurrence counts. And the number of occurrences in a turn is bounded by the number of `Defer` slots plus one, so a script never hangs: a turn that would run forever is a deferring-backwards error, which `Errors` catches.

Chapter 2's sandbox script is the worked case: a reader who expects to see `"(reading...)"` after the build has the turn wrong, because the denial came back through a `Defer` inside turn 0. The corrected script is in the law tests below, under the `Defer` laws.

One more rule about what a turn contains. A `Send` whose response reaches a slot that was torn down in the same occurrence, because the input rebuilt the `dyn` around it, is delivered: the slot existed as the occurrence opened, which is what routing requires, and the old node keeps propagating until commit. The script sees the consequences in the list after the turn, and chapter 2's relabelling button is the program that shows it.

## The law tests

One short program and one short script per law, each pinning one sentence of the chapters on the functors, on time, on widgets and switching, and on elements and Worlds. The scripts are the suite's spine: when an implementation fails one, the failure names the sentence, and when a sentence changes, the script that pins it changes with it. They are deliberately tiny. A law test that needs a hundred-line program is testing a program, and the integration programs are for that.

Every script below assumes the suite's shared declarations, which are also the integration programs' and the GUI library's:

```prism
type Element = choice ..
type Click = click
type Tick = tick
type Instant = Instant Int

tag Ui.Label of Element:
    text: Text

tag Ui.Button of Element:
    label: Text
    responds: Click

tag Clock of Element:
    state: Instant

tag Defer of Element:
    requests: !unit
    responds: unit

type FsRequest = choice Read | Write Text

type FsResponse = choice:
    Contents Text
    Missing
    Denied

type Contents = choice:
    Unavailable
    Known Text

tag Fs.File of Element:
    path: Text
    requests: !FsRequest
    responds: FsResponse

tag Fs.Contents of Element:
    path: Text
    state: Contents

@Label: fn $Text -> @unit = \@widget:
    @Element Ui.Label(text=$_)
    @ unit

@Button: fn $Text -> @!Click = \@widget:
    @Element Ui.Button(label=$_)
```

`@Label` and `@Button` use the shorthand: a dynamic where a field goes lifts the element into a `dyn`, one fresh slot per change, with the button's click the `!Switch` over the generations. Chapter 2 unrolls both.

### Events

**The leftmost wins.** `a | b` fires at n if either fires at n; its value is a's if a fires, else b's.

```prism
@left_wins:
    !inc <- @Button $"Inc"
    !dec <- @Button $"Dec"
    $n <- 0 <= $n + (-1 on !dec | +1 on !inc)
    @Label "n={$n}"

@right_wins:
    !inc <- @Button $"Inc"
    !dec <- @Button $"Dec"
    $n <- 0 <= $n + (+1 on !inc | -1 on !dec)
    @Label "n={$n}"

test leftmost_takes_the_left: Test @left_wins: array:
    Together:
        Send Ui.Button(label="Inc"), click
        Send Ui.Button(label="Dec"), click
    Expect Ui.Label(text="n=-1")

test leftmost_is_the_program_choice: Test @right_wins: array:
    Together:
        Send Ui.Button(label="Inc"), click
        Send Ui.Button(label="Dec"), click
    Expect Ui.Label(text="n=1")
```

Two programs that differ in the order of two operands give two answers, and the test says which is which. The second test is there so that nobody can make the first pass by always taking the decrement.

**`!never` is the identity of `|` on both sides.**

```prism
@never_identity:
    !b <- @Button $"Go"
    $left <- 0 <= $left + 1 on (!never | !b)
    $right <- 0 <= $right + 1 on (!b | !never)
    @Label "{$left} {$right}"

test never_is_the_identity: Test @never_identity: array:
    Send Ui.Button(label="Go"), click
    Expect Ui.Label(text="1 1")
```

**`v on !e` fires exactly when `e` fires, with the value `v`.**

```prism
@on_button:
    !b <- @Button $"OK"
    @ "ok" on !b

test on_fires_with_its_event: Test @on_button: array:
    Quiet
    Send Ui.Button(label="OK"), click
    Fired "ok"
    Push Clock, Instant 1
    Quiet
```

The `Push` is a turn with no click in it, which is the only way a script gets a turn in which the button was not pressed, and `Quiet` is the claim that nothing fires from nothing.

**The tag law.** In a lifted operation with an event operand, a dynamic contributes its value before the occurrence.

```prism
@tag_reads_before:
    !b <- @Button $"Go"
    $n <- 0 <= $n + 1 on !b
    $seen <- 0 <= $n on !b
    @Label "n={$n} seen={$seen}"

test a_tag_reads_the_value_before: Test @tag_reads_before: array:
    Send Ui.Button(label="Go"), click
    Expect Ui.Label(text="n=1 seen=0")
    Send Ui.Button(label="Go"), click
    Expect Ui.Label(text="n=2 seen=1")
```

`$n on !b` and `$n + 1 on !b` fire in the same occurrence and both read `$n` through a tag; the first sees the count before the click, the second computes the count after. Any implementation that let `$seen` see `1` after one click has let a tag read a half-finished occurrence.

### Dynamics

**The change law.** A derived dynamic changes in the occurrence its operands change, and a derived dynamic of a derived dynamic does too.

```prism
@derived_prompt:
    !b <- @Button $"Go"
    $n <- 0 <= $n + 1 on !b
    let $a = $n + 1
    let $b = $a * 2
    @Label "{$a} {$b}"

test derived_dynamics_are_prompt: Test @derived_prompt: array:
    Expect Ui.Label(text="1 2")
    Send Ui.Button(label="Go"), click
    Expect Ui.Label(text="2 4")
```

**At most once.** Within one occurrence a dynamic's change event fires at most once, however many paths lead to it.

```prism
@diamond:
    !b <- @Button $"Go"
    $n <- 0 <= $n + 1 on !b
    let $sum = ($n + 1) + ($n * 2)
    !built <- dyn $sum:
        @ unit
    $k <- 0 <= $k + 1 on !built
    @Label "builds={$k}"

test a_diamond_changes_once: Test @diamond: array:
    Expect Ui.Label(text="builds=1")
    Send Ui.Button(label="Go"), click
    Expect Ui.Label(text="builds=2")
```

`$sum` depends on `$n` by two routes. A `dyn` over it rebuilds once per change event, and the `dyn`'s own event counts the builds: one for the first build, one per click. An implementation with a glitch, where `$sum` changed once per route, would show `builds=3` after one click. This is the only way a script can count a dynamic's changes, and it is deliberately indirect: there is no step that observes a change event, because no World can.

**A hold's initial value, and adoption in the build occurrence.** `$x <- v <= !e` has the value `v` before the first firing of `e`; if `e` fires in the occurrence the widget is built, `$x` changes to it then.

```prism
@hold_adopts:
    !started <- @started
    $x <- 0 <= 1 on !started
    $y <- 0 <= $x on !started
    @Label "x={$x} y={$y}"

test a_hold_adopts_a_build_occurrence_firing: Test @hold_adopts: array:
    Expect Ui.Label(text="x=1 y=0")
```

`$x` adopts `1` in occurrence 0 because its source fired there. `$y` reads `$x` through a tag in the same occurrence and gets the initial value, `0`, which is the tag law and the hold's "value before the first firing" in one line.

**A hold set to the value it already has still changes.** The change law says a derived dynamic's change event fires when an operand's change event fires; a hold's change event is its event, and nothing compares values.

```prism
@same_value:
    !b <- @Button $"Same"
    $v <- 7 <= 7 on !b
    !built <- dyn $v:
        @ unit
    $k <- 0 <= $k + 1 on !built
    @Label "builds={$k}"

test an_equal_value_is_still_a_change: Test @same_value: array:
    Send Ui.Button(label="Same"), click
    Expect Ui.Label(text="builds=2")
```

This pins a sentence that is easy to get the wrong way round when an implementation adds a short-circuit on equality as an optimisation. A `dyn` over a hold rebuilds when the hold's event fires, and a program that does not want that filters the event.

### Time

**A widget built in occurrence n sees all of n.** A hold built in n whose source fired in n adopts that value at commit.

```prism
@sees_all_of_n:
    !add <- @Button $"Add"
    $keys <- {} <= (Insert $keys, Size $keys, unit) on !add
    for $keys:
        $hits <- 0 <= $hits + 1 on !add
        @Label "{$key}: {$hits}"

test a_new_block_sees_the_event_that_made_it: Test @sees_all_of_n: array:
    Send Ui.Button(label="Add"), click
    Expect Ui.Label(text="0: 1")
    Send Ui.Button(label="Add"), click
    Expect Ui.Label(text="0: 2")
    Expect Ui.Label(text="1: 1")
```

The click that inserts key 0 is the click that key 0's block counts: the block behaves as though it had been built just before the click arrived. The alternative, hiding earlier firings from late subscribers, would show `0: 0` and the block would be one click behind for its whole life.

**A `dyn` built in n over a dynamic that changed in n builds once, with the new value.**

```prism
@nested_dyn:
    !b <- @Button $"Go"
    $n <- 0 <= $n + 1 on !b
    let $tens = $n * 10
    dyn $n:
        let outer = _
        dyn $tens:
            @Label "outer={$outer} inner={$_}"
        @ unit
    @ unit

test a_nested_dyn_builds_once_with_the_new_value: Test @nested_dyn: array:
    Expect Ui.Label(text="outer=0 inner=0")
    Send Ui.Button(label="Go"), click
    Expect Ui.Label(text="outer=1 inner=10")
    ExpectNone Ui.Label(text="outer=1 inner=0")
    ExpectCount Ui.Label(..), 1
```

The outer `dyn` rebuilds on the click; the inner one is built inside the new block, in the same occurrence, over `$tens`, which changed in that occurrence. It sees the change event and builds with `10`. An implementation that built it with the old value and then rebuilt it would leave a `Ui.Label(text="outer=1 inner=0")` in some intermediate list, and although no script can see an intermediate list, it would also have fired the inner `dyn`'s event twice in one occurrence, which the diamond test's technique catches; this test keeps the direct form because the sentence is about the value.

**Routing against the opened list.** An input addressed to a path in occurrence n is delivered to the slot at that path in the list as n opened; the old node keeps propagating until commit.

```prism
@clicker:
    !c <- @Button $label
    $count <- 0 <= $count + 1 on !c
    let $label = "Clicked {$count} times"

test a_button_is_clicked_under_its_old_label: Test @clicker: array:
    Send Ui.Button(label="Clicked 0 times"), click
    Expect Ui.Button(label="Clicked 1 times")
    ExpectNone Ui.Button(label="Clicked 0 times")
    Send Ui.Button(label="Clicked 1 times"), click
    Expect Ui.Button(label="Clicked 2 times")
```

The button the script clicks is the one in the list as the turn opened. Its click moves the count, the label changes, the `dyn` inside `@Button` rebuilds, and the new slot is the one the next `Send` finds. The cycle through `$label`, the `dyn`, `!Switch` and `$count` is legal because `!Switch` chooses its inner event from before the occurrence and `$count + 1` reads through a tag.

### Widgets

**`@ v` builds nothing and returns `v`.**

```prism
@pure_five:
    @ 5

test a_pure_widget_builds_nothing: Test @pure_five: array:
    Returns 5
    ExpectNone ignore
```

**Each bind builds a separate node.** `let @b = ...` followed by two binds gives two nodes, two paths, two holds.

```prism
@twice:
    let @counted = @widget:
        !c <- @Button $"Hit"
        $n <- 0 <= $n + 1 on !c
        @Label "n={$n}"
    @counted
    @counted

test a_widget_bound_twice_is_two_nodes: Test @twice: array:
    ExpectCount Ui.Button(label="Hit"), 2
    ExpectCount Ui.Label(text="n=0"), 2
```

A `Send Ui.Button(label="Hit")` here would fail, since two slots match, and that failure is itself a check that there are two. The independence of the two holds is the `for` test below, where keys tell the twins apart.

**Holds are owned by the node and torn down with it.** A rebuild replaces the node; holds inside the block are new holds with their initial values.

```prism
@reset_on_rebuild:
    !flip <- @Button $"Flip"
    $gen <- 0 <= $gen + 1 on !flip
    dyn $gen:
        !hit <- @Button $"Hit"
        $n <- 0 <= $n + 1 on !hit
        @Label "gen {$_}: {$n}"
    @ unit

test a_rebuild_resets_the_holds_inside: Test @reset_on_rebuild: array:
    Send Ui.Button(label="Hit"), click
    Send Ui.Button(label="Hit"), click
    Expect Ui.Label(text="gen 0: 2")
    Send Ui.Button(label="Flip"), click
    Expect Ui.Label(text="gen 1: 0")
    ExpectNone Ui.Label(text="gen 0: 2")
```

**`@started` fires once, in the build occurrence.**

```prism
@starter:
    !started <- @started
    $n <- 0 <= $n + 1 on !started
    @Label "started {$n}"

@started_result:
    @started

test started_fires_once_at_build: Test @starter: array:
    Expect Ui.Label(text="started 1")
    Push Clock, Instant 1
    Expect Ui.Label(text="started 1")

test started_is_visible_as_a_result: Test @started_result: array:
    Fired unit
    Push Clock, Instant 1
    Quiet
```

The second script is the one case where `Fired` holds before any `Send`: the widget's result is `@started`'s event directly, with no `!Switch` between, so turn 0 is a turn in which it fired.

**`@started` inside a `dyn` fires again on every rebuild.** A rebuilt widget that loads on start loads again, visibly, in the program's text.

```prism
@restarter:
    !flip <- @Button $"Flip"
    $gen <- 0 <= $gen + 1 on !flip
    dyn $gen:
        !started <- @started
        @Element Fs.File(path="/f", requests=(Read on !started))
    @ unit

test a_rebuild_restarts: Test @restarter: array:
    Requested Fs.File(path="/f"), Read
    Send Ui.Button(label="Flip"), click
    Requested Fs.File(path="/f"), Read
    Push Clock, Instant 1
    Unrequested Fs.File(..)
```

### Switching

**`dyn` fires in every build occurrence, including the first.** The diamond test above counts those firings; this one checks the value.

```prism
@dyn_values:
    !b <- @Button $"Go"
    $n <- 0 <= $n + 1 on !b
    !vals <- dyn $n:
        @ _ * 100
    $last <- -1 <= !vals
    @Label "last={$last}"

test dyn_fires_with_each_build_result: Test @dyn_values: array:
    Expect Ui.Label(text="last=0")
    Send Ui.Button(label="Go"), click
    Expect Ui.Label(text="last=100")
```

**`!Switch` is not prompt: a creation-occurrence firing is swallowed.**

```prism
@swallowed:
    !flip <- @Button $"Flip"
    $gen <- 0 <= $gen + 1 on !flip
    !!starts <- dyn $gen:
        !started <- @started
        @ !started
    $heard <- 0 <= $heard + 1 on !Switch !!starts
    @Label "heard={$heard}"

test a_switch_does_not_hear_the_birth: Test @swallowed: array:
    Expect Ui.Label(text="heard=0")
    Send Ui.Button(label="Flip"), click
    Expect Ui.Label(text="heard=0")
```

Each generation fires `!started` in the occurrence it is born, and `!Switch` adopts the generation's event at commit, so it never hears it. A block that needs its own start uses `@started` inside the block, as `@restarter` does.

**`!Switch` follows the new inner event from the next occurrence.** The `@clicker` script above shows it: the second `Send` reaches the rebuilt button and its click is heard. This one shows the same through a `dyn` whose generations are different widgets:

```prism
@toggle:
    !flip <- @Button $"Flip"
    $on <- False <= (not $on) on !flip
    !!pressed <- dyn $on:
        if _:
            @Label $"On"
            @ !never
        else:
            @Button $"Press me"
    $presses <- 0 <= $presses + 1 on !Switch !!pressed
    @Label "presses={$presses}"

test a_rebuilt_slot_is_heard_from_the_next_occurrence: Test @toggle: array:
    Send Ui.Button(label="Press me"), click
    Expect Ui.Label(text="presses=1")
    Send Ui.Button(label="Flip"), click
    ExpectNone Ui.Button(label="Press me")
    Send Ui.Button(label="Flip"), click
    Send Ui.Button(label="Press me"), click
    Expect Ui.Label(text="presses=2")
```

**`$Join` is prompt on both the outer and the inner change.**

```prism
@join_prompt:
    !flip <- @Button $"Flip"
    $gen <- 0 <= $gen + 1 on !flip
    !$inner <- dyn $gen:
        !hit <- @Button $"Hit"
        $n <- _ * 100 <= $n + 1 on !hit
        @ $n
    $$cur <- $ 0 <= !$inner
    @Label "cur={$Join $$cur}"

test join_follows_inner_and_outer_changes: Test @join_prompt: array:
    Expect Ui.Label(text="cur=0")
    Send Ui.Button(label="Hit"), click
    Expect Ui.Label(text="cur=1")
    Send Ui.Button(label="Flip"), click
    Expect Ui.Label(text="cur=100")
    Send Ui.Button(label="Hit"), click
    Expect Ui.Label(text="cur=101")
```

The hit changes the inner dynamic; the join changes in that occurrence. The flip changes the outer: a new block, a new inner with initial `100`, and the join shows `100` in the occurrence of the flip, not one later. This is the asymmetry with `!Switch` written as two `Expect`s.

**`for` consolidates events into one firing with exactly the entries that fired.**

```prism
@for_events:
    let $m = $ {"a": unit, "b": unit}
    !hits <- for $m:
        !h <- @Button ($ key)
        @ key on !h
    $last <- 0 <= Size !hits
    @Label "fired {$last}"

test for_consolidates_simultaneous_firings: Test @for_events: array:
    Together:
        Send Ui.Button(label="a"), click
        Send Ui.Button(label="b"), click
    Expect Ui.Label(text="fired 2")
    Send Ui.Button(label="a"), click
    Expect Ui.Label(text="fired 1")
```

**`for` consolidates dynamics into a dynamic map, and plain values into a dynamic map.**

```prism
@for_dynamics:
    let $m = $ {"a": unit, "b": unit}
    $counts <- for $m:
        !h <- @Button ($ key)
        $n <- 0 <= $n + 1 on !h
        @ $n
    $caps <- for $m:
        @ Upper key
    @Label "total={Fold $counts, 0, \|acc, k, v| acc + v} {Fold $caps, "", \|acc, k, v| Concat [[acc, v]]}"

test for_consolidates_dynamics_and_values: Test @for_dynamics: array:
    Expect Ui.Label(text="total=0 AB")
    Send Ui.Button(label="a"), click
    Send Ui.Button(label="a"), click
    Send Ui.Button(label="b"), click
    Expect Ui.Label(text="total=3 AB")
```

**A key's block keeps its state across an insert and a neighbour's removal.**

```prism
@Tally: fn $Text -> @unit = \@widget:
    !hit <- @Button $_
    $n <- 0 <= $n + 1 on !hit
    @Label "{$_}: {$n}"

@tallies:
    !add <- @Button $"Add c"
    !drop <- @Button $"Drop a"
    $names <- {"a": "a", "b": "b"} <= ((Insert $names, "c", "c") on !add) | ((Remove $names, "a") on !drop)
    for $names:
        @Tally $_

test state_survives_an_insert: Test @tallies: array:
    Send Ui.Button(label="a"), click
    Send Ui.Button(label="a"), click
    Send Ui.Button(label="Add c"), click
    Expect Ui.Label(text="a: 2")
    Expect Ui.Label(text="c: 0")

test state_survives_a_neighbours_removal: Test @tallies: array:
    Send Ui.Button(label="b"), click
    Send Ui.Button(label="Drop a"), click
    ExpectNone Ui.Button(label="a")
    Expect Ui.Label(text="b: 1")
```

**The parent sees a new key's result in the creation occurrence; the new block does not see itself.**

```prism
@new_key_seen:
    !add <- @Button $"Add"
    $keys <- {} <= (Insert $keys, Size $keys, unit) on !add
    !starts <- for $keys:
        !started <- @started
        @ key on !started
    $seen <- 0 <= $seen + Size !starts
    @Label "seen={$seen}"

test a_parent_sees_a_new_key_at_once: Test @new_key_seen: array:
    Expect Ui.Label(text="seen=0")
    Send Ui.Button(label="Add"), click
    Expect Ui.Label(text="seen=1")
    Send Ui.Button(label="Add"), click
    Expect Ui.Label(text="seen=2")
```

`for` consolidation is prompt where `!Switch` is not: a block born in n is in the consolidated event in n. Only pure code can fire in a block's birth occurrence, and `@started` is the pure code that does.

**`for` over an array is keyed by index, and state follows the index.** This is the sentence that makes arrays second class, and it is pinned so that nobody improves it by accident.

```prism
@indexed:
    !front <- @Button $"Push front"
    $xs <- [["a", "b"]] <= (Prepend $xs, "z") on !front
    for $xs:
        !h <- @Button $_
        $n <- 0 <= $n + 1 on !h
        @Label "{$key}: {$n}"

test an_array_block_keeps_state_by_position: Test @indexed: array:
    Send Ui.Button(label="b"), click
    Expect Ui.Label(text="1: 1")
    Send Ui.Button(label="Push front"), click
    Expect Ui.Button(label="z")
    Expect Ui.Label(text="1: 1")
    Expect Ui.Label(text="2: 0")
```

After the insertion at the front, the block at index 1 holds `"a"` and still has the count that was given to `"b"`. A program that wants the count to follow the item keeps a map with allocated ids, which is chapter 2's todo list. If arrays ever get a stable per-element identity, this test is the one that changes, and it would then say `Expect Ui.Label(text="2: 1")`.

### Elements and interception

**Elements never fire in the occurrence they were created, and `@started` fires the request in the build occurrence.**

```prism
@requester:
    !started <- @started
    !r <- @Element Fs.File(path="/f", requests=(Read on !started))
    $got <- "none" <= "some" on !r
    @Label "got={$got}"

test a_request_is_answered_later: Test @requester: array:
    Requested Fs.File(path="/f"), Read
    Expect Ui.Label(text="got=none")
    Send Fs.File(path="/f"), Contents "x"
    Expect Ui.Label(text="got=some")
```

**Creating an element requests nothing.**

```prism
@silent:
    @Element Fs.File(path="/f", requests=!never)
    @ unit

test creation_requests_nothing: Test @silent: array:
    Unrequested Fs.File(..)
    Push Clock, Instant 1
    Unrequested Fs.File(..)
```

This is the shortest test in the suite and the one that guards the most. A World component that performed a read because an `Fs.File` appeared would not fail this test, since the Test World has no components; this test checks the Test World's own script component, and the real Worlds are checked against the same sentence in their own tests.

**A response to a path not in the opened list is an error.** No script can send to an unknown path, because `Send` resolves against the current list; the only way to make the error is an interceptor that answers a slot in the occurrence the slot was created.

```prism
@Eager: fn @a -> @a = \|@child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ \|e| @Element e
    !down <- for $slots:
        dyn $_:
            match _.element:
                Ui.Button(..):
                    !started <- @started
                    !resp <- @Element _.element
                    @ !resp | click on !started
                ignore:
                    ((!resp, ignore)) <- @Element _.element
                    @ resp
    @ result

test answering_at_creation_is_an_unknown_path: Test (@Eager @left_wins): array:
    Errors Error(kind=UnknownPath, ..)
```

The child's buttons are created in occurrence 0; `@Eager`'s blocks for them are built in the same occurrence, fire `click` from `@started`, and `!down` fires with paths that are not in the list as the occurrence opened, which for occurrence 0 is empty. The build aborts. This is also the law that an interceptor answers requests only in a later occurrence, from the other side: the one way to answer a slot is a `Defer`, and `@Eager` did not use one.

**A state element has, in its creation occurrence, the value from the snapshot.**

```prism
@clock_label:
    $t <- @Element Clock
    @Label "t={$t.value}"

test state_is_there_at_creation: Test @clock_label: array:
    State Clock, Instant 5
    Expect Ui.Label(text="t=5")
```

**A push reaches a slot created in the push's occurrence.**

```prism
@late_clock:
    !show <- @Button $"Show"
    $shown <- False <= True on !show
    dyn $shown:
        if _:
            $t <- @Element Clock
            @Label "t={$t.value}"
        else:
            @Label $"hidden"
    @ unit

test a_push_reaches_a_slot_born_in_its_occurrence: Test @late_clock: array:
    State Clock, Instant 1
    Expect Ui.Label(text="hidden")
    State Clock, Instant 2
    Send Ui.Button(label="Show"), click
    Expect Ui.Label(text="t=2")
    ExpectNone Ui.Label(text="t=1")
```

The `State` before the `Send` makes the `Send`'s turn a turn with a push and a response. The `Clock` slot is created by the click, reads the oracle, which answers from the previous snapshot, `1`, and then receives the push to `2` in the same occurrence, ending it holding what every other slot holds. A test that showed `t=1` would be an implementation that sampled the oracle and forgot the push, which is a slot one occurrence behind the world for its whole life.

**Two slots of equal value observe one state.**

```prism
@two_clocks:
    $a <- @Element Clock
    $b <- @Element Clock
    @Label "{$a.value} {$b.value}"

test equal_elements_share_one_state: Test @two_clocks: array:
    State Clock, Instant 1
    Expect Ui.Label(text="1 1")
    Push Clock, Instant 2
    Expect Ui.Label(text="2 2")
```

**An interceptor that forwards everything is observably not there.**

```prism
@Transparent: fn @a -> @a = \|@child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ \|e| @Element e
    !down <- for $slots:
        dyn $_:
            ((!resp, ignore)) <- @Element _.element
            @ resp
    @ result

@counter_with_clock:
    $t <- @Element Clock
    !inc <- @Button $"Inc"
    $n <- 0 <= $n + 1 on !inc
    @Label "n={$n} t={$t.value}"

test forwarding_is_the_identity: Test (@Transparent (@Transparent @counter_with_clock)): array:
    State Clock, Instant 1
    ExpectCount Ui.*(..), 2
    Expect Ui.Label(text="n=0 t=1")
    Send Ui.Button(label="Inc"), click
    Push Clock, Instant 2
    Expect Ui.Label(text="n=1 t=2")
```

The transcript of this test is compared, by the harness, with the transcript of the same script run on `@counter_with_clock` alone, element by element. Paths differ, because the forwarder's slots are at the forwarder's paths; everything else must be identical. The `Clock` appears more than once in the outer list, once per forwarder's supplier and once per forwarded slot, and each is the same state by value. A forwarder that minded the duplicates would not forward state slots; this one is the shortest correct program.

**A sandbox supplies `Unavailable`, and nothing confined reaches the World.** `@Sandbox` and `ShowContents` are chapter 2's.

```prism
@peek:
    $c <- @Element Fs.Contents(path="/etc/secret")
    @Label (ShowContents $c)

test a_sandbox_denies_state: Test (@Sandbox "/jail", @peek): array:
    State Fs.Contents(path="/etc/secret"), Known "hunter2"
    Expect Ui.Label(text="(unavailable)")
    ExpectNone Fs.Contents(path="/etc/secret")
```

The script sets the real value and the child never sees it: the supplier returned `@ $ Unavailable`, which is a widget with no elements, so nothing about `/etc/secret` is in the outer list.

**A supplied widget's own elements appear in the interceptor's list.**

```prism
@peek_home:
    $c <- @Element Fs.Contents(path="/home/me/notes")
    @Label (ShowContents $c)

test a_supplier_asks_the_outer_world: Test (@Sandbox "/jail", @peek_home): array:
    State Fs.Contents(path="/jail/home/me/notes"), Known "hi"
    ExpectTop Fs.Contents(path="/jail/home/me/notes")
    ExpectNone Fs.Contents(path="/home/me/notes")
    Expect Ui.Label(text="hi")
    Push Fs.Contents(path="/jail/home/me/notes"), Known "bye"
    Expect Ui.Label(text="bye")
```

**Two child slots of equal value share one supplied widget.**

```prism
@Supplied: fn @a -> @a = \|@child| @widget:
    ((result, ignore)) <- intercept @child, !never, $ \|e| @Element e
    @ result

@two_readers:
    $a <- @Element Fs.Contents(path="/x")
    $b <- @Element Fs.Contents(path="/x")
    @Label "{ShowContents $a} {ShowContents $b}"

test equal_slots_share_a_supplier: Test (@Supplied @two_readers): array:
    State Fs.Contents(path="/x"), Known "v1"
    ExpectCount Fs.Contents(path="/x"), 1
    ExpectNone Ui.*(..)
    Push Fs.Contents(path="/x"), Known "v2"
    Returns unit
```

`@Supplied` forwards nothing and supplies by forwarding, so the only elements in the outer list are the supplied widgets'. Two child slots, one supplied `Fs.Contents`. The labels are held back, so the test cannot read them, and the harness checks the sharing through the count; the integration program for suppliers, later, reads the labels through a forwarder.

**`Defer` is answered later in the same turn.** `@Sandbox` denies a request through a `Defer`, and the denial is visible by the first check after the build.

```prism
let ShowResponse: fn ?FsResponse -> Text = \|r| match r:
    Nil: "(reading...)"
    Some (Contents t): t
    Some Missing: "(no such file)"
    Some Denied: "(permission denied)"

@probe:
    !started <- @started
    !r <- @Element Fs.File(path="/etc/shadow", requests=(Read on !started))
    $seen <- Nil <= Some !r
    @Label (ShowResponse $seen)

test a_sandbox_denies_within_the_turn: Test (@Sandbox "/jail", @probe): array:
    ExpectNone Fs.File(..)
    Requested Defer, unit
    Expect Ui.Label(text="(permission denied)")
```

Turn 0 is two occurrences: the build, in which the probe requests and the sandbox fires its `Defer`, and the answer, in which the sandbox fires `Denied` into the probe. The script's first check runs after both. A reader who writes `Expect Ui.Label(text="(reading...)")` here has the turn wrong, which is why chapter 2 shows that version failing.

**Deferring backwards is an error.** A `Defer` slot that fires again after being answered in the same turn.

```prism
@backwards:
    !started <- @started
    !back <- @Element Defer(requests=(unit on !started | unit on !back))
    @ unit

test deferring_backwards_is_an_error: Test @backwards: array:
    Errors Error(kind=DeferBackwards, ..)
```

Occurrence 0 fires the request; occurrence 1 of the same turn answers it, and the answer fires the request again. The World refuses to open a third occurrence for it, the turn ends in the error, and there is no turn that runs forever. This is the only bound on a turn and it needs no cap.

**A turn with chained `Defer`s runs one occurrence per link; independent `Defer`s are answered together.**

```prism
@chain:
    !started <- @started
    !r1 <- @Element Defer(requests=(unit on !started))
    !r2 <- @Element Defer(requests=(unit on !r1))
    $log <- [[]] <= (Append $log, "a" on !r1) | (Append $log, "b" on !r2)
    @Label (JoinText ",", $log)

@fan:
    !started <- @started
    !r1 <- @Element Defer(requests=(unit on !started))
    !r2 <- @Element Defer(requests=(unit on !started))
    $log <- [[]] <= (Append $log, "a" on !r1) | (Append $log, "b" on !r2)
    @Label (JoinText ",", $log)

test chained_defers_are_three_occurrences: Test @chain: array:
    Expect Ui.Label(text="a,b")

test independent_defers_are_answered_together: Test @fan: array:
    Expect Ui.Label(text="a")
```

In `@chain`, `r2`'s request fires only when `r1` is answered, so the turn is three occurrences and the log has both entries. In `@fan`, both requests fire in occurrence 0, the World answers both in occurrence 1, both `Append`s fire in that one occurrence, and the leftmost wins: the log has one entry. The second script is also a test that the World does not answer `Defer`s one per occurrence, which would be a turn length that depended on the World's mood.

**A state tag with no supplier is an error at the slot's creation.** Shown with the script language's own rule, since the Test World's supplier is the script; `Secret` and `@needs_secret` are the ones declared with the `Errors` step above.

```prism
test an_unsupplied_state_fails_at_creation: Test @needs_secret: array:
    Errors Error(kind=UnsuppliedState, element=Secret, ..)

test a_supplied_state_is_fine: Test @needs_secret: array:
    State Secret, "shh"
    Expect Ui.Label(text="shh")
```

That is forty-four scripts. Each one is short enough to appear whole in a failure message, each is named by its sentence, and each would be the first thing to change if its sentence changed.

## The integration programs

Where a law test pins one sentence, an integration program pins one piece of meaning as a user of the language meets it: a whole widget with several tests, built on the suite's GUI library, driven through containers, with the laws interacting. Twelve of them exist and nine more join them. Each program file holds its widgets and its tests, and each is also a typed-AST golden and a differential fixture, so a program earns its place three times.

### The GUI library

The library is Prism over `@Element` and `intercept`, and it is the convention the Test World's depth rule depends on. Here it is in full, with the shared declarations of the law tests assumed.

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
    Filter slots, \not (IsUi _.element)

let ToMap: fn ChildInput -> Path ==> Response = \|c|
    {c.path: c.response}

# Re-emit every slot unchanged; route each response back under the child's path.
@Forward: fn $(Path ==> Slot) -> @!(Path ==> Response) = \|$slots| @widget:
    for $slots:
        dyn $_:
            ((!resp, ignore)) <- @Element _.element
            @ resp

# Hold the GUI children back as data in one VBox; forward everything else.
@VBox: fn @a -> @a = \|@child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ \|e| @Element e
    !child_in <- @Element Ui.VBox(children=(UiOnly $slots))
    !fwd <- @Forward (NonUi $slots)
    let !down = !Union (ToMap !child_in), !fwd
    @ result
```

`@Element Ui.VBox(children=(UiOnly $slots))` has a dynamic where a field goes, so it is a `dyn` with a fresh `Ui.VBox` slot per change of the child's `Ui.*` list, and `!child_in` is the `!Switch` over the generations. The supplier `\|e| @Element e` forwards every state element to the outer context; the `for` in `@Forward` builds one block per non-`Ui.*` child slot, each re-emitting the element and returning the response the outer context sends to it, consolidated under the child's path. `ToMap` turns a `ChildInput` into a one-entry response map, and `!Union` merges it with the forwarded responses, so a click routed through the box and a file response for a forwarded slot can land in one occurrence.

The library's own typed-AST dump is a golden, and `@VBox` under the Test World, with a counter inside, is program 06.

### The twelve

| Program | What it pins |
| --- | --- |
| `01_element_roundtrip` | An element goes out and a response comes back to the same slot; a response the widget does not want is ignored by the widget, not the World. |
| `02_counter` | The counter, and the leftmost merge when both buttons fire in one occurrence. |
| `03_hold_cycle` | A hold whose dynamic appears in its own update, read through a tag; `Returns` on a dynamic result. |
| `04_intercept_consumes` | What an interceptor does not re-emit never reaches the World, while the child stays live. |
| `05_forward_transparent` | Wrapping in a forwarder twice is observably the identity. |
| `06_vbox_routing` | A click routed down through a container; a non-`Ui.*` response bypassing it; both in one occurrence. |
| `07_dyn_rebuild` | A rebuild swaps slots, new slots get fresh paths, and a rebuilt slot receives input from the next occurrence. |
| `08_for_keyed` | One child per key, with state kept across an insert and a neighbour's removal. |
| `09_non_prompt` | Routing against the list as the occurrence opened: a button clicked under its old label. |
| `10_tag_matching` | `match` on elements: by tag, by namespace, by field value, with the catch-all; a nested element in a field. |
| `11_simultaneous` | Several responses to different slots in one occurrence are all delivered. |
| `12_widget_output_events` | `Fired` and `Quiet` on a widget's result event. |

The programs as written here are updated to the current syntax: numbers by inference with `-1` and `+1` where a literal needs a sign, tags declared with their fields and response types, `click` as the click, the three-argument `intercept`, and the `ChildInput` convention. Where a program's test coincides with a law test above, the program keeps it, because the program is also a golden and a differential fixture and the duplication costs nothing.

**01. An element goes out and comes back.**

```prism
type Poke = choice:
    Set Nat
    Other

tag Test.Probe of Element:
    name: Text
    value: Nat
    responds: Poke

let SetValue: fn Poke -> ?Nat = \|p| match p:
    Set n: Some n
    Other: Nil

@echo:
    !in <- @Element Test.Probe(name="echo", value=$v)
    $v <- 0 <= !Filter (SetValue !in)

test element_roundtrip: Test @echo: array:
    ExpectTop Test.Probe(name="echo", value=0)
    Send Test.Probe(name="echo", ..), Set 5
    ExpectTop Test.Probe(name="echo", value=5)
    Send Test.Probe(name="echo", ..), Other
    ExpectTop Test.Probe(name="echo", value=5)
```

`value=$v` lifts the element into a `dyn`, so `ExpectTop Test.Probe(name="echo", value=5)` after the `Send` is a fresh slot at generation `#1`, and the `Send` of `Other` is addressed to that one. The response type is declared, so a `Send` of something that is not a `Poke` is a compile error in the test file; what the program ignores is a `Poke` it has no use for.

**02. The counter.**

```prism
@counter:
    !increment <- @Button $"Increment"
    !decrement <- @Button $"Decrement"
    let !delta = -1 on !decrement | +1 on !increment
    $count <- 0 <= $count + !delta
    @Label "Count: {$count}"

test counter: Test @counter: array:
    Expect Ui.Label(text="Count: 0")
    Send Ui.Button(label="Increment"), click
    Send Ui.Button(label="Increment"), click
    Expect Ui.Label(text="Count: 2")
    Send Ui.Button(label="Decrement"), click
    Send Ui.Button(label="Decrement"), click
    Expect Ui.Label(text="Count: 0")

test counter_same_occurrence_left_wins: Test @counter: array:
    Together:
        Send Ui.Button(label="Increment"), click
        Send Ui.Button(label="Decrement"), click
    Expect Ui.Label(text="Count: -1")
```

**03. A hold in its own update.**

```prism
type Add = Add Int

tag Test.Accumulator of Element:
    total: Int
    responds: Add

@accumulator:
    !inc <- @Element Test.Accumulator(total=$total)
    $total <- 0 <= $total + !inc.value
    @ $total

test accumulator: Test @accumulator: array:
    Returns 0
    Send Test.Accumulator(..), Add 3
    Returns 3
    Send Test.Accumulator(..), Add -4
    Returns -1
    Send Test.Accumulator(..), Add 1
    Returns 0
    ExpectTop Test.Accumulator(total=0)
```

`!inc.value` is `.value` lifted over the event, unwrapping the newtype; `$total + !inc.value` reads `$total` through a tag. The hold is `Int` because `Add` holds an `Int`, and the `0` follows.

**04. An interceptor that swallows.**

```prism
@counted:
    !increment <- @Button $"Increment"
    $count <- 0 <= $count + 1 on !increment
    @Label "Count: {$count}"
    @ $count

@Swallow: fn @a -> @a = \|@child| @widget:
    ((result, ignore)) <- intercept @child, !never, $ \|e| @Element e
    @ result

test swallow_hides_elements: Test (@Swallow @counted): array:
    ExpectNone Ui.*(..)
    Returns 0

@CountSlots: fn @a -> @a = \|@child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ \|e| @Element e
    !down <- @Forward $slots
    @Element Test.SlotCount(count=(Size $slots))
    @ result

tag Test.SlotCount of Element:
    count: Nat

test count_and_forward: Test (@CountSlots @counted): array:
    ExpectTop Test.SlotCount(count=2)
    Send Ui.Button(label="Increment"), click
    Expect Ui.Label(text="Count: 1")
    Returns 1
```

The child under `@Swallow` is built and its result is live, and nothing it made is in the outer list. Under `@CountSlots` the count is a dynamic field, so `Test.SlotCount` is a `dyn`, one slot per change of the child's list; the count stays `2` across the click because the child replaces a label and keeps two slots.

**05. Forwarding twice is the identity.**

```prism
@Forwarded: fn @a -> @a = \|@child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ \|e| @Element e
    !down <- @Forward $slots
    @ result

test forward_is_identity: Test (@Forwarded (@Forwarded @counted)): array:
    ExpectCount Ui.*(..), 2
    Send Ui.Button(label="Increment"), click
    Send Ui.Button(label="Increment"), click
    ExpectTop Ui.Label(text="Count: 2")
    Returns 2
```

The harness also compares this transcript with `@counted`'s under the same script, paths aside.

**06. A container routes down.**

```prism
type Accepted = Accepted Nat

tag Net.Listener of Element:
    port: Nat
    responds: Accepted

@panel:
    ((!ok, !conn)) <- @VBox @widget:
        @Label $"Hello"
        !b <- @Button $"OK"
        !l <- @Element Net.Listener(port=8080)
        @ ((!b, !l))
    $oks <- 0 <= $oks + 1 on !ok
    $conns <- 0 <= $conns + 1 on !conn
    @Label "ok={$oks} conns={$conns}"

test vbox_wraps_gui: Test @panel: array:
    ExpectTop Ui.VBox(..)
    ExpectTopNone Ui.Button(..)
    Expect Ui.Button(label="OK")
    Expect Ui.Label(text="Hello")
    ExpectTop Ui.Label(text="ok=0 conns=0")

test vbox_forwards_non_gui: Test @panel: array:
    ExpectTop Net.Listener(port=8080)
    ExpectCount Net.Listener(..), 1

test click_routes_through_vbox: Test @panel: array:
    Send Ui.Button(label="OK"), click
    ExpectTop Ui.Label(text="ok=1 conns=0")

test forwarded_response_bypasses_vbox: Test @panel: array:
    Send Net.Listener(port=8080), Accepted 7
    ExpectTop Ui.Label(text="ok=0 conns=1")

test both_in_one_occurrence: Test @panel: array:
    Together:
        Send Ui.Button(label="OK"), click
        Send Net.Listener(port=8080), Accepted 7
    ExpectTop Ui.Label(text="ok=1 conns=1")
```

The `Send` to the button is delivered by the Test World as `ChildInput(path=p, response=click)` to the `Ui.VBox` slot, where `p` is the button's key in the box's `children`; `@VBox` turns it into `{p: click}` and `intercept` routes it. The listener's `Accepted` goes to the forwarded slot at the top and comes back under the child's path through `!fwd`. In the last test both arrive in one occurrence and `!Union` delivers both.

**07. A rebuild swaps slots.** The `@toggle` widget of the law tests, with the original two scripts:

```prism
test dyn_swaps_slots: Test @toggle: array:
    Expect Ui.Button(label="Press me")
    ExpectNone Ui.Label(text="On")
    Send Ui.Button(label="Flip"), click
    ExpectNone Ui.Button(label="Press me")
    Expect Ui.Label(text="On")
    Send Ui.Button(label="Flip"), click
    Expect Ui.Button(label="Press me")

test rebuilt_slot_receives_input: Test @toggle: array:
    Send Ui.Button(label="Flip"), click
    Send Ui.Button(label="Flip"), click
    Send Ui.Button(label="Press me"), click
    Expect Ui.Label(text="presses=1")
```

The transcript shows the "Press me" button at `2/#0/0` in turn 0 and at `2/#2/0` after the second flip: a fresh path per generation, never reused, which is the sentence the transcript pins and the script cannot.

**08. One child per key.** `@tallies` of the law tests, with its third script:

```prism
test one_child_per_key: Test @tallies: array:
    ExpectCount Ui.Label(text="a: 0"), 1
    ExpectCount Ui.Label(text="b: 0"), 1
    ExpectNone Ui.Label(text="c: 0")
```

**09. Clicked under the old label.** `@clicker` of the law tests, as written there.

**10. Matching on elements.**

```prism
type Inner = Inner Text

tag Test.Box of Element:
    value: Nat
    inner: Inner

let Describe: fn Element -> Text = \|e| match e:
    Ui.Button(..): "button"
    Ui.*(..): "other ui"
    Test.Box(value=3, ..): "box with 3"
    Test.Box(..): "some box"
    ignore: "unknown"

@describe_button:
    @ Describe Ui.Button(label="x")

@describe_label:
    @ Describe Ui.Label(text="x")

@describe_box_3:
    @ Describe Test.Box(value=3, inner=(Inner "i"))

@describe_box_4:
    @ Describe Test.Box(value=4, inner=(Inner "i"))

@describe_other:
    @ Describe Fs.Contents(path="/tmp/x")

test tag_match: Test @describe_button: array:
    Returns "button"

test namespace_match: Test @describe_label: array:
    Returns "other ui"

test field_value_match: Test @describe_box_3: array:
    Returns "box with 3"

test field_value_fall_through: Test @describe_box_4: array:
    Returns "some box"

test catch_all: Test @describe_other: array:
    Returns "unknown"

@nested:
    !in <- @Element Test.Probe(name="outer", value=$n)
    $n <- 0 <= 1 on !in
    @Element Test.Box(value=$n, inner=(Inner "i"))
    @ unit

test a_nested_element_field_is_matched: Test @nested: array:
    ExpectTop Test.Box(value=0, inner=Inner "i")
    Send Test.Probe(name="outer", ..), Other
    ExpectTop Test.Box(value=1, inner=Inner "i")
```

There is no test here for a field of the wrong type falling through to the next arm, because a tag's field types are fixed by its declaration and a `Test.Box(value="3")` cannot be built; that case is a negative golden.

**11. Simultaneous responses.**

```prism
@pair:
    !a <- @Button $"A"
    !b <- @Button $"B"
    $as <- 0 <= $as + 1 on !a
    $bs <- 0 <= $bs + 1 on !b
    $last <- "" <= "A" on !a | "B" on !b
    @Label "{$as} {$bs} {$last}"

test together_delivers_all: Test @pair: array:
    Together:
        Send Ui.Button(label="A"), click
        Send Ui.Button(label="B"), click
    Expect Ui.Label(text="1 1 A")

test separate_occurrences: Test @pair: array:
    Send Ui.Button(label="B"), click
    Send Ui.Button(label="A"), click
    Expect Ui.Label(text="1 1 A")
    Send Ui.Button(label="B"), click
    Expect Ui.Label(text="1 2 B")
```

**12. A result event.** `@on_button` of the law tests, with `Fired "ok"` after a click and `Quiet` after a push.

### The successors

Nine programs for the sentences the first twelve do not reach. Each is a program a user might write, not a minimal law script, and the tests say what the program means at the points where a reader guesses wrong.

**13. `@started` kicks a request whose answer arrives in a later turn.**

```prism
@motd:
    !started <- @started
    !refresh <- @Button $"Refresh"
    !resp <- @Element Fs.File(path="/etc/motd", requests=(Read on (!started | !refresh)))
    $shown <- Nil <= Some !resp
    @Label (ShowResponse $shown)

test started_requests_and_waits: Test @motd: array:
    Requested Fs.File(path="/etc/motd"), Read
    Expect Ui.Label(text="(reading...)")
    Send Fs.File(path="/etc/motd"), Contents "Welcome"
    Expect Ui.Label(text="Welcome")
    Unrequested Fs.File(..)

test refresh_requests_again: Test @motd: array:
    Send Fs.File(path="/etc/motd"), Contents "Welcome"
    Send Ui.Button(label="Refresh"), click
    Requested Fs.File(path="/etc/motd"), Read
    Expect Ui.Label(text="Welcome")
    Send Fs.File(path="/etc/motd"), Missing
    Expect Ui.Label(text="(no such file)")

test a_response_in_the_request_turn_is_impossible: Test @motd: array:
    Send Ui.Button(label="Refresh"), click
    Expect Ui.Label(text="(reading...)")
```

The last script looks trivial and is the creation rule in its everyday form. The refresh turn fires a request; the label is still `"(reading...)"` after it because no World can answer in the request's occurrence and the script has not answered yet. The trigger is a value, `!started | !refresh`, which a test, a mock or a gate can replace without touching the element; that is what `@started` buys over an element that loaded itself.

**14. A `dyn` child's creation-occurrence output is swallowed, and a request from inside is not.**

```prism
@reloader:
    !flip <- @Button $"Flip"
    $gen <- 0 <= $gen + 1 on !flip
    !!births <- dyn $gen:
        !started <- @started
        !resp <- @Element Fs.File(path="/data", requests=(Read on !started))
        $got <- "none" <= "some" on !resp
        @Label "gen {$_}: {$got}"
        @ !started
    $heard <- 0 <= $heard + 1 on !Switch !!births
    @Label "heard={$heard}"

test birth_is_swallowed_but_the_request_is_not: Test @reloader: array:
    Expect Ui.Label(text="heard=0")
    Requested Fs.File(path="/data"), Read
    Expect Ui.Label(text="gen 0: none")
    Send Fs.File(path="/data"), Contents "x"
    Expect Ui.Label(text="gen 0: some")
    Send Ui.Button(label="Flip"), click
    Expect Ui.Label(text="heard=0")
    Expect Ui.Label(text="gen 1: none")
    ExpectNone Ui.Label(text="gen 0: some")
    Requested Fs.File(path="/data"), Read
```

The parent never hears `!started` through the `!Switch`, in either generation. The request fired by that same `!started` reaches the World every time, because it goes out through the element list and not through the switch. The two paths out of a block are different in kind, and this program is the place a reader sees both at once.

**15. A `for` parent sees a new key's output in the creation occurrence.**

```prism
type Change = choice:
    Opened
    Toggled

@registry_count:
    !add <- @Button $"Add"
    $next <- 0 <= $next + 1 on !add
    $items <- {} <= (Insert $items, $next, False) on !add
    !events <- for $items:
        !started <- @started
        !toggle <- @Button "Toggle {$key}"
        @ Opened on !started | Toggled on !toggle
    $opened <- 0 <= $opened + Size (Filter !events, \_ == Opened)
    $toggled <- 0 <= $toggled + Size (Filter !events, \_ == Toggled)
    @Label "opened={$opened} toggled={$toggled}"

test a_new_key_reports_in_its_birth_occurrence: Test @registry_count: array:
    Send Ui.Button(label="Add"), click
    Expect Ui.Label(text="opened=1 toggled=0")
    Send Ui.Button(label="Toggle 0"), click
    Expect Ui.Label(text="opened=1 toggled=1")
    Together:
        Send Ui.Button(label="Add"), click
        Send Ui.Button(label="Toggle 0"), click
    Expect Ui.Label(text="opened=2 toggled=2")
```

In the `Together`, key 1 is born and reports `Opened` while key 0 reports `Toggled`, and the consolidated event has both: a map with two entries, one from a block that existed as the occurrence opened and one from a block that did not. The parent's two holds each count what they count. The new block does not see itself, which is why its own `Opened` is in the parent's event and not in anything inside key 1.

**16. A state element is sampled at creation.**

```prism
let Show: fn Contents -> Text = \|c| match c:
    Unavailable: "-"
    Known t: t

@watcher:
    !open <- @Button $"Open"
    $open <- False <= True on !open
    dyn $open:
        if _:
            $c <- @Element Fs.Contents(path="/cfg")
            @Label (Show $c)
        else:
            @Label $"closed"
    @ unit

test state_is_sampled_when_the_slot_is_made: Test @watcher: array:
    State Fs.Contents(path="/cfg"), Known "v1"
    Expect Ui.Label(text="closed")
    Send Ui.Button(label="Open"), click
    Expect Ui.Label(text="v1")

test a_slot_made_with_no_value_set_is_the_error: Test @watcher: array:
    Expect Ui.Label(text="closed")
    Send Ui.Button(label="Open"), click
    Errors Error(kind=UnsuppliedState, element=Fs.Contents(path="/cfg"), ..)
    Expect Ui.Label(text="closed")

test a_snapshot_set_while_closed_is_seen_on_open: Test @watcher: array:
    State Fs.Contents(path="/cfg"), Known "v1"
    State Fs.Contents(path="/cfg"), Known "v2"
    Send Ui.Button(label="Open"), click
    Expect Ui.Label(text="v2")
```

The second script is the negative space: the build needs no `State` because no `Fs.Contents` exists yet, and the click that creates one fails with the unsupplied-state error, the turn aborts, and the list is the closed one it was before. The third script sets the snapshot twice before one turn. The oracle answers `v1`, the previous snapshot; the push in the same occurrence is to `v2`; the slot ends the turn at `v2`. A reader who expected `v1` has the order of oracle and push backwards, and a reader who expected the second `State` to be lost has forgotten that a World gathers everything that changed into one snapshot per occurrence.

**17. A push reaches a slot created in the push's occurrence, and every slot alike.**

```prism
@gauges:
    !more <- @Button $"More"
    $n <- 1 <= $n + 1 on !more
    let $keys = Range $n
    $readings <- for $keys:
        $t <- @Element Clock
        @ $t.value
    @Label (JoinText ",", (MapValues $readings, ToText))

test every_gauge_reads_one_clock: Test @gauges: array:
    State Clock, Instant 10
    Expect Ui.Label(text="10")
    State Clock, Instant 11
    Send Ui.Button(label="More"), click
    Expect Ui.Label(text="11,11")
    Push Clock, Instant 12
    Expect Ui.Label(text="12,12")
    Send Ui.Button(label="More"), click
    Expect Ui.Label(text="12,12,12")
```

`Range n` is the map `{0: unit, ..., n-1: unit}`. The second gauge is created in the occurrence of the `11` push and shows `11` with the first. The third is created with no push in its occurrence and shows the snapshot. Every reading in every occurrence is the same, which is the equality law stated as a label.

**18. A supplied widget's elements appear in the interceptor's list.** `@Sandbox` of chapter 2, which re-emits the child's GUI elements so that the labels are readable at the top, with a child that reads a confined path and an unconfined one:

```prism
@two_files:
    $home <- @Element Fs.Contents(path="/home/me/a")
    $etc <- @Element Fs.Contents(path="/etc/b")
    @Label "home={Show $home} etc={Show $etc}"

test a_sandbox_rewrites_state_paths: Test (@Sandbox "/jail", @two_files): array:
    State Fs.Contents(path="/jail/home/me/a"), Known "A"
    State Fs.Contents(path="/etc/b"), Known "B"
    ExpectTop Fs.Contents(path="/jail/home/me/a")
    ExpectNone Fs.Contents(path="/home/me/a")
    ExpectNone Fs.Contents(path="/etc/b")
    Expect Ui.Label(text="home=A etc=-")
    Push Fs.Contents(path="/jail/home/me/a"), Known "A2"
    Expect Ui.Label(text="home=A2 etc=-")
    Push Fs.Contents(path="/etc/b"), Known "B2"
    Expect Ui.Label(text="home=A2 etc=-")
```

The supplied widget for the confined path made an `Fs.Contents` with the rewritten path in the sandbox's node, and that is what the top sees. The supplied widget for the refused path is `@ $ Unavailable`, with no element, and a push to the real `/etc/b` goes nowhere because nothing in the list asked for it.

**19. Two equal slots share one supplier.** Chapter 2's `@Cached`, which supplies `Fs.Contents` through a hold refreshed by a button, with a child that has two readers of one path and one of another:

```prism
@readers:
    $a <- @Element Fs.Contents(path="/cfg")
    $b <- @Element Fs.Contents(path="/cfg")
    $c <- @Element Fs.Contents(path="/other")
    @Label "{Show $a} {Show $b} {Show $c}"

test equal_readers_share_one_cache_entry: Test (@Cached @readers): array:
    State Fs.Contents(path="/cfg"), Known "v1"
    State Fs.Contents(path="/other"), Known "o1"
    ExpectCount Fs.Contents(path="/cfg"), 1
    ExpectCount Fs.Contents(path="/other"), 1
    Expect Ui.Label(text="v1 v1 o1")
    Push Fs.Contents(path="/cfg"), Known "v2"
    Expect Ui.Label(text="v1 v1 o1")
    Send Ui.Button(label="Refresh"), click
    Expect Ui.Label(text="v2 v2 o1")
```

`@Cached` forwards every slot, so the child's own `Fs.Contents` slots are re-emitted by `@Forward` and would count too; the `ExpectCount` of `1` therefore says that `@Cached` here is the variant that forwards responses for request slots only, `@Forward (NonState $slots)`, with `NonState` filtering by the tag's declared kind. That variant is the one in the suite, because the count is the point. Both readers show `v1` after the push and `v2` after the refresh, in the same occurrence, because they observe one hold.

**20. The `Defer` bound.** A chain of five, and a fan of five, with an occurrence counter the program keeps itself:

```prism
@chain5:
    !started <- @started
    !r1 <- @Element Defer(requests=(unit on !started))
    !r2 <- @Element Defer(requests=(unit on !r1))
    !r3 <- @Element Defer(requests=(unit on !r2))
    !r4 <- @Element Defer(requests=(unit on !r3))
    !r5 <- @Element Defer(requests=(unit on !r4))
    $steps <- 0 <= $steps + 1 on (!r1 | !r2 | !r3 | !r4 | !r5)
    @Label "steps={$steps}"

@fan5:
    !started <- @started
    !r1 <- @Element Defer(requests=(unit on !started))
    !r2 <- @Element Defer(requests=(unit on !started))
    !r3 <- @Element Defer(requests=(unit on !started))
    !r4 <- @Element Defer(requests=(unit on !started))
    !r5 <- @Element Defer(requests=(unit on !started))
    $steps <- 0 <= $steps + 1 on (!r1 | !r2 | !r3 | !r4 | !r5)
    @Label "steps={$steps}"

test a_chain_of_five_is_six_occurrences: Test @chain5: array:
    Expect Ui.Label(text="steps=5")

test a_fan_of_five_is_two_occurrences: Test @fan5: array:
    Expect Ui.Label(text="steps=1")
```

`$steps + 1 on (...)` moves once per occurrence in which any `Defer` answered, so it counts occurrences after the first. Five in the chain, one in the fan. The transcript marks each occurrence inside turn 0, and the differential test checks that both implementations agree on how many there were, which is a check on the world core as much as on the runtime.

**21. Deferring backwards, as the bug it catches.** A child that retries at once when denied, under the sandbox:

```prism
@retrier:
    !started <- @started
    !resp <- @Element Fs.File(path="/etc/shadow", requests=(Read on (!started | !again)))
    let !again = !Filter (IsDenied !resp)
    @Label $"asked"

let IsDenied: fn FsResponse -> ?unit = \|r| match r:
    Denied: Some unit
    ignore: Nil

test an_immediate_retry_is_deferring_backwards: Test (@Sandbox "/jail", @retrier): array:
    Errors Error(kind=DeferBackwards, ..)
```

Occurrence 0: the request; the sandbox fires its `Defer`. Occurrence 1: the denial; the child retries; the sandbox's `for` block for that slot fires `unit on _.!request` again, and the `Defer` slot it owns has already been answered this turn. The error names the `Defer`'s path and the interceptor it belongs to. This is a program that in a callback framework would spin forever, and here it is a reported error in turn 0 with no turn that ran long. A retry belongs behind a `Time.After`, where the World's clock separates the attempts, and a second version of `@retrier` in the same file does that and passes.

## The layout corpus

Two lexers written by two people from one description of the offside rule will disagree on something, and the place they disagree will be a line nobody thought to write down: a statement continued onto a more-indented line, a dedent that closes two blocks at once, a colon inside braces. The corpus is those lines, written down, each with the token stream both lexers must produce. It is the first thing a frontend must pass, before parsing, because every later disagreement would otherwise be blamed on the parser.

A corpus entry is a source file and a token file. Tokens are printed one per line as `kind(text)` with layout tokens as `open`, `sep` and `close`; here they are shown on one line each with the layout tokens in angle brackets, which is how a failure message prints them.

**A statement continued by a more-indented line.**

```prism
@w:
    $count <- 0 <= $count
        + !delta
```

```
ident(@w) colon <open> ident($count) bind nat(0) hold ident($count) plus ident(!delta) <close>
```

The second line of the statement is indented more than the block's column, so it continues the statement and no `sep` is emitted.

**A dedent of two levels at once.**

```prism
@w:
    dyn $x:
        @Label $"a"
@v:
    @ unit
```

```
ident(@w) colon <open> kw(dyn) ident($x) colon <open> ident(@Label) sigil($) text("a") <close> <close> <sep> ident(@v) colon <open> sigil(@) kw(unit) <close>
```

Two `close`s, then a `sep` at column 0 because the top level is itself a block, then `@v`. A lexer that emitted one `close` would leave the `dyn` block open across a top-level definition, and the parser's error would point at the wrong line.

**An inline block and a layout block in one `match`.**

```prism
let f = \match _:
    True: 3
    False:
        4
```

```
kw(let) ident(f) eq lambda kw(match) ident(_) colon <open> ident(True) colon <open> nat(3) <close> <sep> ident(False) colon <open> nat(4) <close> <close>
```

The inline form `True: 3` opens a block that is the rest of the line and closes at the end of it; the layout form `False:` opens a block on the next line. Both arms are statements of the `match` block and both are blocks, which is what makes an arm's body able to be either form.

**A colon inside brackets, and a trailing comment after the opening colon.**

```prism
@w:   # the colon above opens a block, this comment is not a token
    let m = {"a": 1,
             "b": 2}
    @ m
```

```
ident(@w) colon <open> kw(let) ident(m) eq lbrace text("a") colon nat(1) comma text("b") colon nat(2) rbrace <sep> sigil(@) ident(m) <close>
```

Layout is suspended inside the braces: the line break and the indentation before `"b"` are whitespace, and the colons are key separators, not block openers. The comment after `@w:` is whitespace too, so the colon is still the last token on its line and opens the block.

**Interpolation, nested braces and a literal `$`.** A text literal is one token whose parts are literal text and `interp` groups; each group holds the tokens of its expression, lexed with layout suspended up to the matching `}`.

```prism
@w:
    @Label "n={Size {"a": 1}}, \{ok\}"
    let p = "cost: $5, {$n} left"
```

```
ident(@w) colon <open> ident(@Label) text("n=", interp(ident(Size) lbrace text("a") colon nat(1) rbrace), ", {ok}") <sep> kw(let) ident(p) eq text("cost: $5, ", interp(ident($n)), " left") <close>
```

The braces inside the interpolation nest, so the first `}` after `1` closes the map and the second closes the interpolation; the colon inside them is a key separator, not a block opener; the nested `"a"` is a text token inside the group. `\{` and `\}` are literal braces and join the surrounding literal part, which is why the last part is `", {ok}"`. The `$` in `$5` is text, and the `$` before `n` belongs to the name inside the group. A lexer that treated `$` in text as the start of an interpolation would split the first literal of the second line at the dollar sign.

**`F -1` against `F - 1`, and `\F .x` against `a.x`.**

```prism
let a = F -1
let b = F - 1
let c = \F .x
let d = a.x
```

```
kw(let) ident(a) eq ident(F) int(-1) <sep>
kw(let) ident(b) eq ident(F) minus nat(1) <sep>
kw(let) ident(c) eq lambda ident(F) dot ident(x) <sep>
kw(let) ident(d) eq ident(a) field(x)
```

The spacing rule decides at the lexer: a `-` preceded by whitespace and followed immediately by a non-space is a prefix sign and the literal is signed; `-` with space on both sides is binary. A `.` in prefix position is printed as `dot` then the name, and a `.` attached to a name is a `field` token. The lexer is where this lives because it needs the whitespace. The parser then gives a prefix `.x` its meaning: wherever `_` is in scope it is the pronoun's field, `_.x`, so `c` is `\F _.x`; at the start of a continuation line it is a method-chain step. A sibling entry pins that `F .x` is a call of `F` on `_.x` when `_` is in scope, and an unbound-pronoun error, with that error's span, when it is not.

**`|` as merge against `|a, b|` as lambda bars.**

```prism
let m = !a | !b
let f = \|a, b| a | b
```

```
kw(let) ident(m) eq ident(!a) merge ident(!b) <sep>
kw(let) ident(f) eq lambda lbar ident(a) comma ident(b) rbar ident(a) merge ident(b)
```

A `|` immediately after `\` opens an argument list and the next `|` closes it; every other `|` is merge. The second `|` inside the lambda body is merge even though an argument list was just closed.

**A sigil before an open bracket.**

```prism
let s = $ (x + 1)
let t = $(x + 1)
```

```
kw(let) ident(s) eq sigil($) lparen ident(x) plus nat(1) rparen <sep>
kw(let) ident(t) eq sigil($) lparen ident(x) plus nat(1) rparen
```

Both are the free sigil: a sigil is attached only when the next character is a name character, an underscore, a quote or another sigil, and `(` is none of those. The two lines produce the same tokens, which is the entry's point.

The corpus has about forty entries, the eight kinds above and their variants: a block opened mid-statement by `for $x:` and closed by the next statement; an `else` at the column of its `if`; a `match` whose arms are all inline; a text literal with an interpolated `{expr}` containing a colon, and one with an interpolation that holds another text literal; a line with only a comment inside a block, which neither opens nor closes nor continues anything; a tab in leading whitespace, which is an error with a span. The corpus is also the first differential test: the Rust and Haskell lexers are run over every entry and the token files diffed, before either implementation has a parser.

## Typed-AST goldens and negative goldens

The frontends are tested by what they decide. Elaboration's output is the typed AST, a tree in which every meaning has been fixed: every lift is explicit, every literal and operator has its numeric type, every binding is marked build-time or occurrence-time and placed in its group, every recursive group has its placeholder and close, every element construction names its tag. The typed AST has a text form, printed and parsed from one definition, and a golden test is a Prism file and its dump. When elaboration changes, the diff is something a person reads, and the question "did this change meaning" is answered by whether a golden changed.

### The text form

One binding per line: the name with its sigil, its type, its mark (`[build]` or `[occ]`), its group, and the expression with every lift explicit, followed by the span. The format is chapter 12's, spelled the same here because the goldens are the same files. Here is the counter, with the two library widgets it uses, which is the first golden and the one every reader of the compiler should be able to write by hand:

```prism
@Button: fn $Text -> @!Click = \|$label| @widget:
    !!clicks <- dyn $label:
        @Element Ui.Button(label=_)
    @ !Switch !!clicks

@Label: fn $Text -> @unit = \|$text| @widget:
    dyn $text:
        @Element Ui.Label(text=_)
    @ unit

@counter:
    !increment <- @Button "Increment"
    !decrement <- @Button "Decrement"
    let !delta = -1 on !decrement | +1 on !increment
    $count <- 0 <= $count + !delta
    @Label "Count: {$count}"
```

```
@Button: fn $Text -> @!Click = \|$label| @widget:                                   @1:1
  !!clicks: !!Click [build] g1 = dyn $label \|label| @widget:                          @2:5
    _: !Click [build] g1 = element Ui.Button{label=label} responds                     @3:9
    @ _
  @ switch !!clicks

@Label: fn $Text -> @unit = \|$text| @widget:                                        @6:1
  _: unit [build] g1 = dyn $text \|text| @widget:                                      @7:5
    _: unit [build] g1 = element Ui.Label{text=text}                                   @8:9
    @ unit
  @ unit

@counter: @unit                                                                       @11:1
  !increment: !Click [build] g1 = bind @Button(const "Increment")                     @12:5
  !decrement: !Click [build] g2 = bind @Button(const "Decrement")                     @13:5
  !delta: !Int [occ] g3 = leftmost [map !decrement \|ignore| -1:Int, map !increment \|ignore| +1:Int]   @14:9
  $count: $Int [build] g4 = placeholder                                               @15:5
  $count.hold: $Int [build] g4 = hold 0:Int (tag !delta [$count] \|d, c| Num.Add[Int] c, d)   @15:5
  close $count $count.hold                                                            @15:5
  _: unit [build] g5 = bind @Label(dmap [$count] \|c| cat ["Count: ", ToText[Int] c])   @16:5
  @ _
```

Everything the compiler chose is on the page. `-1 on !decrement` became a `map` over the event yielding an `Int` literal, because `on` returns its left operand and lifting did the rest. `|` became `leftmost`. `$count + !delta` became a `tag`, which is the one window onto the value before the occurrence, applied with `Num.Add[Int]`, because `!delta` is `Int` and inference flowed from the literal `-1` through the merge into the hold. `$count` is a cycle through that tag, so its group has a placeholder, the hold under the name `$count.hold`, and a close. The two binds, the hold and the label bind are `[build]`; the merge is `[occ]`, and so are the closures inside `map`, `tag` and `dmap`, whose marks are implied by their position. The label's interpolation is a `cat` of parts over a `dmap` of `$count`, with `ToText[Int]` rendering the number. `@Button` and `@Label` are library widgets whose single `dyn` is the programmer's; the `element` lines are the only place a tag appears. Nothing below this line decides anything about meaning, which is the sentence the golden pins.

The spellings, `leftmost`, `tag`, `hold`, `map`, `dmap`, `const`, `bind`, `element`, `placeholder`, `close`, `cat`, `Num.Add[Int]`, `ToText[Int]` and literals as `0:Int`, are the printer's and are frozen by the goldens; the Haskell frontend must print the same characters for the same source. Elements print as `element Ui.Button{label=label}` followed by `responds` when the tag responds, `dyn` as `dyn $x \|x| @widget:` with the block's bindings indented beneath it, `for` with its kind and whether the block can fire at birth, and `intercept` as `intercept child responses supplier`. A closure after conversion prints its capture list, `\[$total]|d| ...`, and that form is the one the lint checks.

### What the goldens are

The counter. The GUI library, whose dump shows the `dyn` under a dynamic field, the `for` in `@Forward` consolidating to an event, and the `intercept` in `@VBox`. Every integration program. And one small file per rule of the chapters on the lexical structure and on expressions: the comma rule, including `intercept @_, !Union !kid_ins, !fwd_ins` with its golden showing it parsing as `intercept(@_, Union(!kid_ins), !fwd_ins)`, which is the reason the library writes the parentheses; the precedence table, one file per adjacent pair of levels; the spacing rule; a lambda with a sigiled underscore taking that kind; an unannotated `let` lambda lifted at two call sites with different kinds; a `match` on a dynamic becoming a dynamic of the arms; `for` as an expression over a plain map; text interpolation of each primitive; a hold whose literal type is pinned only by an operand three lines away.

What a golden catches, in the order these have bitten: a lift that changed kind, say a dynamic read promptly where it should be a tag, which is a glitch that no script may notice for months; a literal or operator that resolved to a different numeric type; a binding that moved between build-time and occurrence-time, which changes how often an expensive function runs; a recursive group split or placed differently, which changes build order; a `for` consolidating to the wrong kind.

### Negative goldens

A rejected program and its exact error, spans included. These exist because an error can get worse silently: the program is still rejected, the test that checks "rejected" still passes, and the message that used to name both events now says "type error". Each negative golden is the message, so a worse message is a diff.

```
error: two events in one operation
  --> two_events.prism:3:16
   |
 3 |     let !sum = !a + !b
   |                ^^   ^^
   |
   = both `!a` and `!b` fire; an operation lifts over at most one event
   = help: merge them with `!a | !b` or `!Leftmost [[!a, !b]]`, or hold one and read it through a tag
```

```
error: cycle without a delay
  --> prompt_cycle.prism:2:5
   |
 2 |     let $a = $b + 1
   |         ^^ `$a` depends on `$b`
 3 |     let $b = $a * 2
   |         ^^ `$b` depends on `$a`
   |
   = every edge in this cycle is prompt; a cycle must pass through a tag, a `!Switch`, or a hold's value
```

```
error: cycle without a delay
  --> dyn_cycle.prism:2:5
   |
 2 |     !!r <- dyn $x:
   |     ^^^ `!!r` depends on `$x`
 4 |     $x <- 0 <= !Switch !!r
   |     ^^ `$x` depends on `!!r`
   |
   = `dyn` is not a delay: its event fires in the occurrence of the rebuild
```

The third is the sentence "dyn is not a delay" as a negative golden; the cycle is legal with `!Switch` between `!!r` and the hold, and a sibling positive golden has exactly that program with the switch.

```
error: `+~` is `Int` addition
  --> suffix.prism:3:23
   |
 3 |     let $mean = $sum +~ 0.5
   |                      ^^ both operands are `Float`
   |
   = help: use `+.` or `+`
```

There is deliberately no golden for an integer literal nothing pins: `$n <- 0 <= $n + 1 on !tick` is an `Int` counter, because an unconstrained integer-spelled literal defaults to `Int`, and a positive golden pins that.

```
error: `a -1` is a call
  --> spacing.prism:2:13
   |
 2 |     let b = a -1
   |             ^ `a` is a `Nat`, not a function
   |
   = help: did you mean `a - 1`?
```

```
error: a tag's field has a concrete type
  --> generic_box.prism:1:40
   |
 1 | let Wrap: fn a -> Element = \|x| Test.Box(value=x)
   |                                               ^ `value` is a `Nat`; `x` is `a`
```

```
error: `match` on an extensible variant needs a catch-all
  --> no_catch_all.prism:1:30
   |
 1 | let IsButton = \|e| match e:
   |                     ^^^^^
 2 |     Ui.Button(..): True
   |
   = `Element` is open; add an `ignore:` arm
```

```
error: sigil disagrees with the kind
  --> wrong_sigil.prism:2:5
   |
 2 |     let $sum = $a + !b
   |         ^^^^   ^^^^^^^ this is an event
   |
   = help: name it `!sum`
```

```
error: a lowercase name is snake_case
  --> naming.prism:1:5
   |
 1 | let myValue = 3
   |     ^^^^^^^
```

```
error: `update` is reserved for future work
  --> future.prism:3:5
   |
 3 |     update $items:
   |     ^^^^^^
   |
   = `read`, `update` and `&` are not part of this version of the language
```

Runtime errors have negative goldens too, through the `Errors` step, and their text form is the error value as the transcript prints it: `Error(kind=UnknownPath, path=0/#1/0, turn=1)`, `Error(kind=DeferBackwards, path=1/[3]/#0/1, turn=0, occurrence=2)`, `Error(kind=Cycle, spans=[[...]])` with the spans of every binding in the cycle in order, `Error(kind=UnsuppliedState, element=Secret, path=0)`. The compiler names every cyclic slot after its source span so that a runtime cycle through two widget functions, which no static check can see, reports Prism source and not runtime internals; the golden for that case is a program with two widget functions whose results feed each other's arguments promptly, and its expected error lists both.

## Runtime unit tests and the invariant

Under the conformance suite sits the Rust runtime's own test suite, written in Rust, against the graph directly, with no Prism in it. It exists because a combinator that drifts from its law should fail before any Prism program can reach it, and because some laws, like a closure running at most once per occurrence, are not observable from a World at all and have to be checked from inside.

Each test is one sentence about the runtime. The ones that pin the laws of the chapters on the functors and on time, with the names they have or will have:

- `leftmost_takes_the_left_when_both_fire`
- `a_tag_reads_the_value_before_the_occurrence`
- `a_hold_adopts_a_firing_in_its_build_occurrence`
- `a_derived_dynamic_changes_once_however_many_routes_lead_to_it`
- `switch_hold_hears_a_new_inner_from_the_next_occurrence`
- `a_child_under_dyn_and_switch_is_not_heard_in_its_build_occurrence`
- `dyn_fires_in_its_first_build_and_in_every_rebuild`
- `a_nested_dyn_built_in_the_occurrence_its_dynamic_changes_shows_the_new_value`
- `nested_dyn_rebuilds_in_the_same_occurrence_as_its_parent`
- `a_widget_built_mid_occurrence_sees_the_same_events_at_any_depth`
- `a_widget_built_mid_occurrence_gets_the_mapped_value_of_an_event_that_already_fired`
- `a_closure_runs_once_per_occurrence_even_when_late_subscribers_read_it`
- `a_replaced_child_keeps_working_until_its_last_occurrence_ends`
- `dropping_a_widget_frees_its_cyclic_loops`
- `joined_dynamic_takes_the_new_inner_value_when_it_changes_in_the_switch_occurrence`
- `joined_dynamic_ignores_the_old_inner_changing_in_the_switch_occurrence`
- `joined_dynamic_fires_once_when_outer_and_inner_change_together`
- `a_same_occurrence_cycle_through_a_cyclic_event_is_reported_instead_of_hanging`
- `a_dynamic_defined_as_a_function_of_itself_is_reported_instead_of_hanging`
- `a_cycle_through_intercept_is_reported`
- `a_failed_occurrence_leaves_the_graph_as_it_was`
- `a_runtime_moved_to_another_thread_keeps_working`
- `one_call_is_one_occurrence`
- `a_slot_removed_in_n_still_receives_input_in_n_and_is_gone_in_n_plus_1`
- `input_to_a_path_not_in_the_opened_list_is_an_error`
- `a_request_fired_by_a_slot_torn_down_in_the_same_occurrence_is_reported_with_its_path`
- `a_keyed_list_keeps_a_childs_state_across_an_insert_and_a_removal`
- `an_insert_at_the_front_of_an_array_list_rebuilds_every_later_child`
- `a_state_slot_reads_the_oracle_then_the_push_in_its_creation_occurrence`
- `equal_state_elements_share_one_supplied_widget_until_the_last_slot_goes`
- `a_supplied_widgets_elements_join_the_interceptors_list`

The join tests are the `Dynamic::join` of the chapter on the Rust runtime, and the last seven are the boundary; they are written against the run call with a hand-built widget, which is the harness the Test World replaces for Prism programs.

### The invariant, checked after every occurrence

Every runtime test runs with an invariant check after every occurrence, failed or not, and the check is the list of things the transaction promises:

- Every event's fired mark is clear, and every event's cached current-occurrence value is gone. A mark left set would make the next occurrence see a firing that did not happen.
- No hold holds a value that was committed before `usize::MAX`: during the occurrence, every hold's visible value is its value before the occurrence, and after commit, its value is the last value its event fired with. The check keeps a shadow copy per hold and compares.
- No node fired twice in the occurrence; the check counts firings per node and reports the first node that reached two, with its height and the heights of its sources.
- The action map is empty and the current height is reset. A leftover action would run at the start of the next occurrence as if it were part of it.
- The routing table the occurrence used equals the element list as the occurrence opened, and every input delivered went to a path in that table. This is the routing law stated as a check, and it catches both routing against the updated list mid-occurrence and tearing a slot down too early.
- Every node built during the occurrence is owned by exactly one node, and every node torn down during it is owned by none and has no strong references into it from any subscriber list. The second half is what makes "no collector" a fact rather than a hope; the check walks the tree and the subscriber lists.
- After a failed occurrence: the element list, every hold's value, every switch's current inner and every `dyn`'s current generation are what they were before the occurrence opened. The test that injects a panic into a closure in the middle of propagation and then checks this is the rollback test, and it runs the same check as every other test; nothing special is needed because the invariant already says what "as it was" means.

The invariant is a function in the runtime crate, compiled in tests only, and it is the one place where the runtime's internals are tested as internals. That is allowed because the invariant's items are each a sentence of the chapter on time; the quotable-test rule, below, is satisfied by the list rather than by the function's name.

## World tests against the world core

A World is the world core plus its components, and the core's obligations are the same for every World, so they are tested once, in the host language, with a fake component. The fake component accepts a handful of tags, records every call the core makes to it, performs requests by appending them to a log, answers the oracle from a map the test sets, and produces responses and pushes when the test tells it to. The runtime under it is real; the program is a small hand-built widget, or the compiled counter once the closure compiler exists.

What the core tests check, each a sentence from the chapter on Worlds:

- *The turn loop.* One input, one occurrence, one outcome returned; the test counts run calls and they match the inputs delivered exactly.
- *The `Defer` bound.* A program with `d` `Defer` slots in a chain runs `d + 1` occurrences in one turn and the core returns after the last; a fan of `d` runs two; a random program with `d` `Defer`s never runs more than `d + 1`.
- *Deferring backwards.* A `Defer` answered this turn that fires again is reported as `DeferBackwards` with the slot's path and the occurrence, and the core does not open another occurrence for it.
- *Snapshot and push consistency.* The test changes the fake's state between turns and during a turn; for every occurrence, the oracle's answers equal the previous snapshot and the pushes equal the difference to the current one; the core never answers an oracle from the snapshot it is pushing. A slot created in an occurrence with a push to its value ends that occurrence holding the pushed value.
- *Discarding late responses.* The fake produces a response for a slot that left the list in the previous turn; the core drops it, logs it to diagnostics, and asks the fake to cancel; nothing reaches the runtime, and the runtime's unknown-path error is not raised because the core caught it first.
- *Requests from torn-down slots.* A slot that fires a request and is torn down in the same occurrence has its request reported with its path; the core performs it through the component and discards any response.
- *`Unsupported`.* A request on a tag no component accepts gets an `Unsupported` response in the next turn, not the same occurrence; a presence tag nobody accepts is reported once; a state tag nobody accepts is a build error in the occurrence the slot is created, with the slot's path.
- *Effects after the turn.* The fake's log shows requests performed after the run call returned, in occurrence order and, within an occurrence, in list order, never during propagation.
- *Error reporting.* An aborted occurrence comes out of the core as an error with the kind, the spans, the turn and the occurrence, in the one format every World prints.

The Test World is a World, so it runs under these tests too, with the script component standing in for the fake. The CLI World's tests are separate, below, because its components touch the operating system and need a process around them.

## CLI golden transcripts

The CLI World is the first World that runs Prism against a real operating system, and its tests are the only ones in the suite that run a real binary. Each test is a program, its arguments and environment, its standard input (as a list of chunks, because chunking is the thing to pin), and the expected standard output, standard error and exit code:

```
program: wc.prism
args: []
env: {}
stdin: ["one\ntwo\nthr", "ee\n"]
stdout: "3\n"
stderr: ""
exit: 0
```

The harness runs the binary with the given stdin delivered chunk by chunk with a pause between chunks, so that each chunk is its own turn, and compares. Timers are not in these transcripts: a program with a `Time.After` is tested under the Test World with a `Push Clock`, so that no golden depends on wall-clock time.

The transcripts, each with what it pins:

- *Line counting*, chapter 2's `@wc`, on empty input, one line, many lines, and lines split across chunks. Pins one turn per chunk, the partial line held to the next chunk, output written after the turn, and exit by `Process.Exit`'s request.
- *Falling off the end*: a program that prints and never fires `Process.Exit`. Pins exit 0 when nothing can cause another occurrence: no stdin slot, no signal slot, no request in flight, no timer; args, env, clock and watch slots do not keep a run alive.
- *Exit codes*: a computed nonzero code; two `Process.Exit` slots firing in one turn, where the first in occurrence-then-list order is the code the process exits with; a code outside 0–255, which is an error.
- *Output order*: two `Cli.Stdout` slots firing in one occurrence, written in list order; stdout and stderr interleaved across turns, in turn order; writes and an exit in one turn, with the writes landing first.
- *Args and env*: a program that prints `Process.Args` and `Process.Env` from their state elements, with no arguments, several arguments, and an empty environment. Pins that both have their value in occurrence 0 from the oracle and that nothing is requested to get them.
- *File first, then stdin*: a program that reads a file from `@started` and builds its `Cli.Stdin` slot inside a `dyn` after the response. Pins that stdin is not read until the slot exists, which also keeps the transcript free of a race.
- *Unsupported*: a `Ui.Button` at the root. Pins the World's error on stderr naming the interceptors the element came through, the `Unsupported` response reaching the program's slot in the next turn, and the program continuing.
- *Signals*: with and without a `Cli.Signals` slot, by sending the process a signal from the harness. Pins OS defaults without a slot and the signal arriving as a response with one.
- *A closed pipe*: stdout closed by the harness before a write. Pins that the write's failure comes back as a response rather than killing the process.

The CLI World's transcripts are also the only tests in which an implementation's behaviour against time is checked at all, and only by order: a chunk's turn completes before the next chunk is delivered. Anything finer is the World's business and not tested.

## The differential test

Everything above runs under both implementations, and the differential test is `diff`. Two commands produce the two interchange formats: `prism dump foo.prism` prints the typed AST in the text form, and `prism test foo.prism --transcript` runs every test in the file under the Test World and prints the transcript of each, turn by turn. The differential harness runs both commands under both implementations over every file in the suite, the layout corpus's token files included, and reports every byte that differs.

An empty diff is the claim that both implementations read the specification the same way, for every program in the suite, at the level of what a World sees and at the level of what the compiler chose. It is not a claim that either implementation is right; the conformance scripts make that claim, and the differential test adds the second reader.

A divergence means one of two things, and telling them apart is the whole value of the exercise. Either one implementation has a bug, which the law test that pins the sentence will usually have caught already and the diff localises to a construct; or two careful readers read a sentence of the specification differently, and the sentence is ambiguous. The second kind is the more valuable find, and the procedure for it is fixed: the sentence gets rewritten so that it admits one reading, a law test is added that pins that reading, and both implementations are brought to it. A divergence is never resolved by making one implementation copy the other's behaviour without a sentence to point at, because that turns the second reading back into the first and the specification into a description of the Rust runtime.

Three kinds of divergence have been anticipated and have a place to land. A path printed differently is a reading of "its position in the widget tree" and is fixed in the transcript format's definition, not in either runtime. A typed-AST dump that differs in a lift or a numeric type is a reading of the lifting rule or the inference rule, and the goldens, which both frontends must match, decide it before the transcripts can. A transcript that differs in which occurrence of a turn a request fired in is a reading of the `Defer` rule or the creation rule, and the world core tests decide it. Anything else is new, and new is what the test is for.

The differential test runs on every change to either implementation and on every change to the specification, since a change to a sentence is a change to what the goldens and transcripts should say. When a sentence changes and the diff stays empty, both implementations have the same bug, and the law test is the only thing that can find it; that is why there are both.

## The quotable-test rule

Every test in every layer obeys one rule: it is quotable. Its name is a sentence from the specification, or a sentence that could be added to it, and its body is short enough to read whole in the failure message. `a_push_reaches_a_slot_born_in_its_occurrence`, `leftmost_takes_the_left`, `an_immediate_retry_is_deferring_backwards`: each is a claim about the language that a reader can check against the text without knowing anything about the implementation that failed it.

The rule has teeth in both directions. A test that cannot be stated as a sentence about the language is testing the implementation's structure, and those tests are deleted when the structure changes, which is the only honest thing to do with a test whose subject has gone. A test that the runtime is `Send + Sync` would be a fact about locks the design does not want; the test the suite has is that a runtime can be moved to another thread, which is the sentence the design does want. And a sentence in the specification with no test is a sentence that two implementations may read differently without anyone noticing, so when a reader of the specification finds a sentence they cannot write a test for, the sentence is suspect: either it is not observable, in which case it is not a rule and should say so, or it is observable and the suite is missing a script.

You have read a test suite that was a pile of regression cases named after bug numbers, and you have had to work out, from the body, what each one was for. The rule above is the refusal to build that pile. A conformance suite is the specification, said a second time in a form a machine can check, and every one of its tests should read like a line of the first saying.
