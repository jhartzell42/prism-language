# Chapter 9 — Errors

A program's own failures are values, and the failures the language cannot make into values do the least damage the transaction allows. That sentence is the whole chapter. The first half is a discipline the type system enforces: a file that is not there, a socket that closed, an integer divided by zero, an index past the end are all cases of a type, and the `match` that handles them is a compile error to leave out. The second half is about what is left after that discipline has done its work, which is logic errors and broken contracts: a cycle that only exists once two widgets are wired together at runtime, an overflow, a World that answers a path that no longer exists. None of these can be a value, because a value would be a lie about what happened, and so each one has to be contained. The occurrence is a transaction, so the cheapest unit of containment is already there: drop the buffer and the occurrence never happened. The chapter reasons about when a smaller unit is possible (a construction), when a larger one is wanted (quarantine of an owning subtree), what the program and the World each get to see, and why there is no `catch`.

## What can fail, and what cannot

You have had the stack trace that ends in a map lookup. Some callback, three layers down from the click, asked a map for a key that was there a moment ago, got `null` back, and dereferenced it. The fix was a guard, and the guard is now in the code forever, next to a comment nobody will read, and the thing that removed the key is still somewhere else entirely. Most of what a runtime calls an error is this: a value that was allowed to be absent, in a place that assumed it was not, discovered at the worst moment.

Prism's answer to that whole class is to refuse it at the type. `m[k]` returns `?V`, and so does `a[i]`, and so does `a / b` on `Nat` and `Int`. A `match` on a closed choice is exhaustive or it does not compile. There is no null, so there is nothing to dereference. The state a widget asked for has a type with an `Unavailable` case, and the `match` that draws it has an arm for that case. A response from the World is a value of the tag's declared type, and that type says what failure looks like, `Contents Text | Failed Fs.Error`, so a file that is not there is an arm in the program and not an exception in the runtime. Nothing a World component does can throw across the boundary, because the only thing that crosses it is plain data: a component that fails produces a response or does not, and the program's type already allows for a response that never comes.

That removal is the negative space of this chapter, and each piece of it buys something for the implementation as well as the program. No partial matches means the propagation loop never has to unwind through a closure that failed to match. No out-of-range panics means indexing and division are total, each for its own reason. An index past the end has no element, so `a[i]` says `?V`. An integer divided by zero has no value either: no `Nat` or `Int` is the quotient, any one the language picked would be a plausible wrong answer, and the denominator is often data the program has something to say about, so `/` on `Nat` and `Int` returns `?`. `Float` is different on the merits, because the type already has values for the answer. `x / 0.0` is an infinity, `0.0 / 0.0` is NaN, and IEEE says what every operation does with them, so `/` on `Float` follows IEEE and never fails; a ratio of two sensor readings needs no `match` around a quotient that is already a number, and a program that dislikes infinities tests for them where it cares. Neither division can abort an occurrence. No null means no value representation needs a sentinel. No component exceptions crossing means the runtime's only caller is a World that sends plain data, and the only way a World can hurt the runtime is by sending the wrong plain data, which is a protocol error and is listed below. The runtime does not need a general exception mechanism because the language gave it nothing general to catch.

What is left is short. Grouped by who caused it:

**Logic errors in the program.** A cycle in the prompt graph that the compiler could not see: one through a widget function's argument and result, or one formed at runtime by switching. Arithmetic overflow in `Nat` or `Int`. A plain function that recurses without end. These are bugs in the program, and they are still errors, because no type can make "this program's dependency graph has a loop in it" into a value the program could handle.

**Protocol errors at the boundary.** A response addressed to a path that was not in the list as the occurrence opened. A response of the wrong type for the slot's tag. Two responses to one slot in one occurrence. A `Defer` deferring backwards. A state tag that reaches the World with no component that supplies it. A quarantine instruction for a path no error named. Each is a bug in a World or in an interceptor, not in the program whose slots were addressed, and each is detectable at the moment it happens.

**Failures in the world.** A file that is not there, a socket that closes, a timer the host could not set, a request the World does not support. These are not errors in this chapter's sense at all. They are responses and state values, the tag's types have cases for them, and the oracle cannot fail. The discipline that keeps them out of this chapter, unavailability in the type and never a placeholder, is the main reason the chapter is short.

**Exhaustion.** Memory, stack depth in a plain function, a turn that will not end. The last two have language answers. The `Defer` rule bounds a turn to one occurrence per `Defer` slot plus one, without a budget or a timer. A plain function that recurses without end is stopped by a depth guard in the closure compiler and reported as an error with the function's span, below under pure code. Memory is the host's.

Everything below is about the first two groups and the two exhaustions the language answers. The third group is the type system's and memory is the operating system's.

## The occurrence as the unit of failure

Every reactive system you have debugged had a moment where it was half done. The model was updated and the view was not; the first subscriber had been told and the second had not; a listener had been removed from one list and was still in another. An exception in that moment left the system there, and the next event found it there. Reflex's engine, Spider, throws one exception of its own, `EventLoopException`, when it detects a causality loop, and catches nothing inside propagation; what an exception thrown from inside a `push` or `pull` leaves behind is whatever it leaves behind, and the rule that nothing in the network may throw is kept by discipline rather than by the library (from memory). Prism has the same rule and a different reason it holds.

An occurrence is a transaction. Holds commit at the end. The element list's new value is the list after the occurrence, read by the World after the run call returns. Request firings are read by the World after the occurrence commits. `!Switch` adopts its new inner event at commit. A widget replaced by a `rebuild` rebuild is kept until commit and dropped then. Every write an occurrence makes, in other words, is buffered until the occurrence is over, because the language has no assignment: the only state is a hold, and a hold's update is a value waiting for commit. The buffer exists so that nothing can observe a half-updated graph. It is also, with no further work, a rollback mechanism: an occurrence that cannot finish is abandoned by dropping the buffer.

Dropping the buffer means, concretely:

- the pending hold updates are discarded, so every hold keeps the value it had before the occurrence;
- the pending `!Switch` adoptions are discarded, so every switch keeps listening to the inner it listened to before;
- widgets built during the occurrence by `rebuild` or `for` are dropped, and widgets they were replacing are kept, so the widget tree is the tree as the occurrence opened;
- the per-occurrence firing records are cleared, so no event is marked as having fired;
- the action queue is emptied and the current height is reset, so the runtime is ready for the next occurrence;
- the element list is the list as the occurrence opened, which is also the list the World last received.

The run call returns an error instead of a result. Occurrence n did not happen. The inputs that opened it are lost: a click, a response, a push, a `Defer` answer, all of it. The World decides what the loss means, and the policies are below.

The invariant, stated for each party. The *runtime* promises that after an aborted occurrence the program is observably the program as the occurrence opened: every hold, every switch, every slot, every path. The *program* is bound by nothing, because from inside, nothing happened; this is the weakness of the occurrence as the unit, taken up under what the program sees. The *World* is bound to treat the inputs of an aborted occurrence as never delivered: a response it sent was not received, a push it made did not reach the snapshot the program holds, and its own snapshot bookkeeping must say so, which the world core handles by advancing its snapshot only when a run call returns a list. An *interceptor* is bound by nothing special, since an interceptor is Prism and an aborted occurrence rolls it back with everything else.

The one thing that cannot be taken back is work already done: a closure that ran, ran. That is why it matters that closures are pure. A lifted operation that computed a value nobody will commit wasted its time and nothing else. There is no file half-written, because writing files is not something a closure can do; there is no listener half-registered, because subscriptions made during the occurrence belong to widgets that are being dropped. The transaction is cheap precisely because the language already forbade everything a transaction would have had to undo.

Consider the coarser unit, for comparison. Any error ends the process with a message. It is honest, and for a single-program CLI it is the right policy anyway. But a World with many programs in it, a shell or a server that keys a `for` over connections, is then hostage to its worst widget: one client's cycle ends the service for every client. The occurrence as the unit lets such a World drop one occurrence and keep running, and the subtree as the unit, below, lets it drop the one client.

Now the finer unit. Could a failure be contained to a single node, so that the rest of the occurrence completes? Not in general. When a cycle is found mid-propagation, every node below the current height has fired and its value has been read by its subscribers; the nodes above have not. A node that failed in the middle has subscribers that were waiting for it and may have already consumed its old value through a tag; completing the occurrence without it means some subscribers saw an occurrence in which it fired and others saw one in which it did not, which is the half-updated graph the whole language exists to rule out. A smaller unit is coherent only where something already has a boundary of its own, and exactly one thing does.

The Rust runtime's `catch_unwind` around every action in `propagate` is the backstop under all of this: a panic anywhere in an occurrence clears the action map, resets the height and rethrows, and holds and switches are untouched because they commit only in the final batch; the run call turns that path into a returned error and adds the dropping of widgets built in the occurrence (chapter 11). The Haskell implementation has no buffer to drop and rolls back instead by replaying the occurrence on a fresh local timeline (chapter 14); the observable promise is the same.

## Containment at a construction boundary

A `rebuild` building its block in the middle of an occurrence is the one place the unit of failure can be smaller than the occurrence, and it is the common case for runtime cycles, because a cycle the compiler could not see is almost always one that only exists once some generation of some `rebuild` has been built.

A build has a boundary. It is a closure the runtime calls, which creates a widget, builds children into it, closes its cyclic placeholders, creates its slots, and returns a result. Until it returns, nothing outside the widget refers to anything inside it: subscriptions from inside to outside events exist, but those are weak from the outside's side and die with the widget; the slots it created are not yet in any element list the World has read; the `rebuild`'s event has not yet fired with its result. So a build that fails can be abandoned by dropping the half-built widget, and the occurrence around it continues as if the `rebuild` had built something else.

What it builds instead is an error widget: a widget with one slot, whose element is the presence element `Error`, and no result. The slot sits where the block's slots would have sat, so its path is the generation's path with one slot segment, `0`: a `rebuild` at position `0` in `@main` whose generation `#1` failed has its `Error` element at `0/#1/0`, and a `for` at position `1` whose key `[7]` failed has it at `1/[7]/0`. That rule is the whole of the error widget's address and it is the same one quarantine uses below, so a transcript that expects `Error` after either knows where to look. The `rebuild`'s event does not fire in that occurrence, because there is no result of the block's type to fire with. The parent is left with whatever it had: a `!Switch` keeps the old generation's event, which belongs to a widget that is being torn down and so never fires again; a hold of dynamics under `$Join` keeps the old generation's dynamic, which keeps its last value. The old generation is still torn down at commit, because the dynamic it was built for has changed and the old block is not the right one for the new value either. The `Error` element is in the list after the occurrence, where an interceptor sees it in its child's list and the World sees it at the root.

The failure itself leaves the runtime in the outcome of the run call that contained it, not as the call's error: `Outcome.errors` is the list of every build failure the occurrence contained, each a `RuntimeError` of kind `Build`, with the cause (a cycle, an unsupplied state tag, a native failure in a build-time closure) in its message and the cause's spans, and the failed widget's path as its owner. The world core takes the list from there to its `errors` element, so a program subscribed to its own errors hears of a contained failure in the next turn exactly as it hears of an abort. A World that offers no `errors` still sees the `Error` element in the list, since the list is the one thing every World reads; a World that reads neither learns nothing, which is a World's right.

This is tentative in one respect: a block whose result is plain data, `rebuild $x: @ Compute _`, gives the parent nothing when it fails, and a parent that holds the `rebuild`'s event into a dynamic keeps a stale plain value with no signal in the language that it is stale. If that turns out to be a trap in practice, the alternative is for the `rebuild`'s result type to acquire an error case, `!(Built a | Failed Error)`, which every caller would then have to match on. The cost of that is paid at every `rebuild` in every program for a case that is a bug; the cost of the current answer is paid once, by whoever reads `Outcome.errors` and the `errors` stream. The second seems right until a program shows otherwise.

The same boundary serves `for`. A key appearing in occurrence n builds its block in n; if the build fails, that key's block is the error widget, the consolidated map has no entry for the key's result, and the other keys are untouched. For a `for` over connections in a server, this is one client failing to connect and the rest unaffected, which is the behavior a server wants and the behavior the occurrence as the unit would not give. The last worked scenario walks through it.

And the same boundary serves the first build of all, occurrence zero. The World builds `@main`; the construction is the whole program; a cycle found there means the program cannot start. There is no outside for the build to continue into, so the run call for occurrence zero returns an error, of the cause's own kind rather than `Build`, and no list, and a CLI World prints it and exits nonzero before it has done anything else. There is no partial program to quarantine and nothing to roll back; it is a compile error that arrived late, and the message, below, is written to read like one.

The build-time abort does not need to abort the occurrence, and the reason is sound rather than convenient. The build happened at some height, when the `rebuild`'s dynamic changed. Everything below that height had fired. The new widget read some of those firings through its subscriptions, which is the mid-occurrence construction rule doing its ordinary work. None of those reads changed anything outside the widget, because reads are pure and the values were cached by the firing event, not computed for the new subscriber. Dropping the widget leaves every event's cached value as it was and every subscriber list without entries that were weak anyway. The occurrence continues at the next height with a tree in which the `rebuild`'s slot holds an error widget, which is a legal tree. Nothing has been half done.

What a World does with a contained failure is its policy, and the policy is lighter than for an abort because nothing was lost: the occurrence committed, its requests were performed, and the program is consistent with a visible hole in it. The CLI World prints the error to stderr and continues. Exiting would be the honest choice for a logic error if the error were the whole program's, but a contained failure is by construction one generation of one block, and a server under the CLI World whose seventh connection failed to build should keep serving the other six. The GUI World draws the `Error` element where the block's slots were, which its layout interceptor already knows how to do. The Test World records every entry of `Outcome.errors` for the `errors` step and prints the `Error` element in the transcript at its path.

## Containment to an owning subtree

Here is the afternoon this section is about. You have a dashboard with twelve panels, each a widget from a different author, and one of them, after an update, has a cycle in it. Under the occurrence as the unit, every occurrence in which that panel's cycle fires is aborted, which means every click anywhere on the dashboard that happens to also tick that panel's clock is lost. The other eleven panels are fine, and nobody can use them. What you want is for the bad panel to become a red box that says what went wrong, and for the other eleven to keep working.

Every node in the graph is owned by a widget, because ownership is a tree and the tree is the GC story. A cycle, found during propagation, runs through some set of nodes, and that set has a lowest common owning widget: walk up from each node on the loop until the paths meet. Not every widget has a path, since a plain widget-function application makes a widget but adds no segment; so the owner is the lowest common owning widget that does have one, a `rebuild` generation, a `for` key's block, an intercept child, or the root. Call it the owner. Quarantine replaces the owner's widget with an error widget, the same one-slot widget the construction boundary builds, at the owner's position in the tree, and tears the subtree down. The `Error` element is then at the owner's path with one slot segment: an owner at `1/[relay]` leaves `1/[relay]/0: Error(..)` in the list, and an owner that is a generation `0/#3` leaves `0/#3/0`.

Quarantine is the Rust runtime's. The Haskell implementation rolls an aborted occurrence back by replaying on a fresh local timeline and takes no `quarantine` input, so the conformance scripts that exercise it are Rust-only, and the differential test compares the two implementations up to the abort and not past it.

It cannot happen in the aborted occurrence, because that occurrence is being rolled back and a rollback that also changes the tree is not a rollback. It happens in the next occurrence the World runs, as an instruction in that run call: `quarantine: [path]`, the owner's path as the error reported it. The runtime performs the quarantine before routing that occurrence's inputs: the owner's widget is torn down, its slots leave the list, the error widget takes its place, and inputs addressed to paths under the owner, which were legal addresses because those paths were in the list the World last received, are delivered to nothing. The runtime reports each one in the outcome with its path, in the same place as request firings from a slot torn down in its occurrence (chapter 11 names the field), so the World can discard the late result it was waiting to deliver and the Test World can check that it was reported rather than lost. The instruction is the World's to give because the policy is the World's: a CLI World that exits on any error will never send it, a dashboard World will send it every time, a server World will send it when the owner is under the connection `for` and exit when it is above.

The owner is the runtime's to name, per kind, from the widget whose closure was running when the error was found. For a cycle, it is the lowest common owner of the loop, as above. For an overflow, or a native failure during propagation, it is the owner of the node whose lifted operation failed, which is the smallest subtree whose removal removes the failing operation. For a contained build failure it is the widget that was being built, which is also where the `Error` element now sits, so a World never needs to quarantine it: the runtime already did the equivalent. For an unknown path, a wrong-typed or duplicate response, a state tag with no supplier at the root, and deferring backwards, there is no owner, because nothing in the tree is at fault, and the field is `Nil`. The runtime remembers every owner it named in its last error until the next run call completes; a `quarantine` path it did not name is refused, before anything else in that call, as a `Component` error naming the path, because the World is allowed to quarantine only what it was told to and a World that sends some other path has a bug the runtime should not perform. The Test World's quarantine script sends the path from the recorded error and nothing else.

What quarantine costs, and the costs are the argument against it:

- The owner's parent gets its result cut off without being told. A `!Switch` over the owner keeps a dead inner and never fires again; a `$Join` keeps the last value; a plain `<-` bind of the owner has an event that is now `!never` and a dynamic that is now constant. The parent's program text says nothing about this. The `errors` stream is where it is said instead, and an interceptor that wants to react sees the `Error` element in its child's list.
- Every hold in the subtree is gone. The panel's state is lost, not paused.
- The owner may be large. A cycle through the root's `!Switch` and a leaf's event has the root as its owner, and the root has no path a World could send; quarantining it would be ending the program with a slower message, and the instruction is not available for it. The error names the owner so the World can see this coming, and a World may reasonably have a policy of quarantining only below some depth, or only under a `for`.
- The occurrence that found the cycle was still aborted, and its inputs still lost. Quarantine does not recover them; it stops the next occurrence from being lost the same way.

Whether a quarantined subtree can come back follows from what the error widget is: a generation of whatever built it. An owner that is a `rebuild` generation is replaced when the `rebuild`'s dynamic next changes, by an ordinary rebuild, with fresh holds and fresh paths; nothing in the language knows the previous generation was an error widget. An owner that is a `for` key's block is replaced if the key leaves and returns. An owner that is an intercept child in a block that is never rebuilt never comes back, because nothing ever rebuilds it. This is the `rebuild`'s own meaning doing the work, and a retry policy is a `rebuild` over a counter that an `errors` subscriber increments, written in Prism where it can be read. The alternative, a runtime that retries on its own schedule, would be an event from nothing, and the language has none.

Whether the `Error` element is one tag or several, `Error.Cycle`, `Error.Overflow`, decides what an interceptor can match on. One tag, with the kind as a closed choice inside it, is the design: matching on an element's tag is for routing it to the code that handles that kind of element, and nothing handles a cycle differently from an overflow at the element level. A GUI library draws every error the same way; a shell lists every error the same way. The kind is a field, and a `match` on the field is available to anyone who wants it. If a World turns out to want a component per error kind, the tags can split; nothing else depends on it.

```prism
type ErrorKind = choice:
    Cycle
    Overflow
    UnknownPath
    UnsuppliedState
    DeferBackwards
    Component
    Build

tag Error of Element:
    kind: ErrorKind
    spans: [Span]
    occurrence: Nat
    turn: Nat
    message: Text
    owner: ?Path
    path: ?Path
```

The seven kinds are the closed list, and three of them hold more than their names say. `Component` is every failure that is the World's or an interceptor's and not the program's: a World component that crashed, a response of a type the slot's tag does not declare, two responses to one slot in one occurrence, a quarantine of a path no error named, and a panic in native code that a World or a library put inside a closure; `message` says which. `Build` is a failure contained at a construction boundary, whatever its cause, with the cause named in `message` and its bindings in `spans`. `Overflow` is arithmetic past the width of `Nat` or `Int` and also recursion past the depth guard. `spans` is the loop in order for a cycle and the one binding otherwise; `path` is the path involved for an unknown path or a backwards `Defer`; `occurrence` and `turn` say when; `owner` is as above.

`Span` is a source location: file, line, column, and the binding's name. `Path` is the opaque path type an interceptor already handles in `$(Path ==> Slot)`; a program cannot make one, but it can hold one, compare it, look it up in a map it was given, and ask whether one lies under another with the prelude's `Under: fn Path, Path -> Bool`, which is the one operation on paths beyond equality and ordering and exists for the shell below, which needs to know whose subtree an owner is in.

## Static causality and runtime detection

A button that relabels itself is legal and useful: `!!clicks <- rebuild $label: @Element Ui.Button(label=_)` with `$label` derived from the clicks is the worked examples' counter, and nobody wants it rejected. `!!inner <- rebuild $mode: match _: Double: @Double !out` with `let !out = !Switch !!inner` has the same text, bind for bind, and one mode over it is a loop: the generation's result feeds the switch that feeds the generation. Nothing in either block's text tells the two apart, because what makes the second a loop is which generation gets built, and that is a runtime fact. So the compiler finds every cycle it can see, and the question is what it can see.

Within one widget block, the bindings form a graph whose edges the compiler knows: every lifted operation is a prompt edge from its operands, every `|` and `for` consolidation and `rebuild` rebuild is prompt, and every tag, `!Switch` choice and hold value side is delayed. A cycle made of prompt edges alone is a compile error that lists every binding in it, with spans, and `rebuild` is not a delay because its event fires in the occurrence of the rebuild. That check is complete for the block.

What it cannot see is through a function. `!a <- @Plus !tick, !b` followed by `!b <- @Double !a` is a prompt cycle if `@Plus`'s result depends promptly on its second argument and `@Double`'s on its first, and the function's type, `fn !Int, !Int -> @!Int`, does not say. The compiler has to assume either that every function is prompt in every argument, which rejects the `@clicker` of the worked examples and every supervisor pattern, or that none is, which is what it does, and leaves the rest to the runtime. And it cannot see through switching at all, which is the relabelling button and the doubling relay above: whether some generation's inner event depends promptly on `!out` depends on which generation is built.

A function type could say more. Alongside `fn !Int, !Int -> @!Int` the checker could record which arguments the result promptly depends on, an effect in the type, inferred for every function the way the block check infers it for bindings and checked at every call. Then `@Plus` would be known prompt in both arguments and the cycle above would be a compile error at the call site. This is the right kind of answer and a research project: it has to pass through `rebuild`, `for`, `intercept` and higher-order arguments, and it has to be erased, since generic code is. It is not in this specification; its effect, if it comes, is to make the runtime check below a backstop rather than the mechanism, and nothing else in the design moves.

The runtime finds a prompt cycle by the one property a prompt cycle has: no consistent assignment of heights. Each node's height is one more than the greatest of its prompt sources' heights, and a node on a cycle would need to be above itself. Computing a height is a walk upstream, and a walk that reaches a node it is already computing has found the cycle. The runtime has two places to run that walk, and the choice between them decides which unit of failure a cycle lands in.

The first is at the moment an edge is created. A cyclic placeholder is closed during a build; a widget function's result is connected to its arguments' sources during a build; a `!Switch` adopts a new inner at commit. If the height walk runs then, eagerly, a cycle is found at the construction that closed it or at the commit that adopted it, and never in the middle of propagation. The second is lazily, on the first firing through the new edge, which is what a cache-and-invalidate scheme does on its own: the switch adoption clears every cached height downstream, and the next firing recomputes them and finds the loop then, mid-occurrence, in the occurrence after the switch.

Eager is right, and the reason is which unit it lands in. A cycle found at a `rebuild`'s build is contained to the build. A cycle found at a `!Switch`'s adoption is found at commit, and the walk is the first step of commit: it runs after propagation is complete and before the final batch in which holds and switches commit (chapter 11 fixes that batch as the last thing propagation runs), so aborting there discards a buffer nothing has yet been written from; the occurrence is lost, but cleanly. If the walk ran after holds had committed, a rollback would need to undo them, which is the undo log the design does not have. A cycle found lazily is found in the following occurrence, which may be a click on an unrelated button, aborted for a bug it had nothing to do with, and the report names the occurrence that tripped over the cycle rather than the one that made it. The cost of eager is a walk upstream per edge creation, which the height cache bounds: the walk stops at any node whose height is cached, and a node's cached height is valid exactly as long as nothing below it has been invalidated, which is the rule every event type in the runtime already keeps. What would change this is a measurement showing the walk dominates build time in programs with deep fan-in; nothing suggests it, since the same walk runs lazily anyway on the first firing.

The remaining runtime check, that a node never schedules at or below the current height, stays as what it is in the runtime today: an invariant assertion. With eager detection it should never fire for a program's cycle, and if it fires the bug is in the runtime, which is the one case where a panic that ends the process is the right report.

So the units line up with where cycles are found. A cycle in one block is a compile error. A cycle through functions closed at the build of `@main` is an error in occurrence zero and the program does not start. A cycle closed at a `rebuild` or `for` build is contained to that build. A cycle closed by a switch adoption aborts the switching occurrence and is the one case where quarantine has work to do. The invariants a runtime must keep in every case: it never deadlocks, it never loops, it never fires a node twice, and after the error the program is the program as the occurrence opened. The current runtime's `try_lock` on the height mutex is the detection in its crudest form, a re-entry check that panics with the kind of cycle but not its nodes; the walk it replaces will keep the nodes on the loop as it goes, so the report can list them.

## Whether pure code may panic

The counter you shipped has been running for four years and `$count` is a `Nat`. Nobody has clicked it eighteen quintillion times. But somewhere a `$total` is a sum of file sizes in bytes, and someone pointed it at a directory of disk images, and `+` on `Nat` wrapped, and the label now says the directory is three kilobytes. That bug is worse than a crash, because it is a plausible wrong answer, and the discipline here is chosen against it.

Three disciplines are possible, and the choice is between what the language promises and what it can keep.

*Panic and abort.* Overflow is detected and the occurrence aborts. Native code the runtime calls may fail, and the occurrence aborts. The transaction gives this for free and it is honest about logic errors.

*Checked everywhere.* Every operation that can fail returns a value that says so: overflow returns `?` like integer division does, or arithmetic wraps and never fails, and native code returns results. The runtime then never sees a failure from user code at all, and the only aborts are protocol errors, which are World bugs. This is the Reflex rule, nothing in the network may throw, made into a type.

*Both.* The default spelling of arithmetic detects overflow and aborts the occurrence. An explicit spelling exists for code that wants overflow as a value or wants wrapping. The transaction stays as the backstop for native code.

The third is the discipline here, for two reasons that pull in opposite directions and meet in the middle. Against checked-everywhere: `$count + !delta` has to read as arithmetic, because "you can see what moves" is a goal, and `$count + !delta` returning `?Int` would put a `match` around every sum in every program for a case that is a bug when it happens. Integer division returns `?` because integer division by zero is an ordinary value's ordinary behavior, the denominator is often data, and the program usually has something to say about it; `Float` division has infinities to answer with and returns a `Float`. Overflow is not like that: a counter past the machine's width is a bug, and a bug should stop the occurrence rather than be matched on. Against panic-and-abort alone: a hash function wants wrapping multiplication, a checksum wants wrapping addition, and a program that reads sizes from a file wants to know, as a value, that the sum did not fit. So the explicit spellings exist, as prelude functions rather than operators: `Wrap.Add`, `Wrap.Mul` and their siblings wrap; `Checked.Add`, `Checked.Mul` return `?`. They are functions because the hot loop that needs them can afford a name and because an operator spelling is one more token every reader would have to know for every sum they read. If hashing and PRNG code turn out common enough to make `Wrap.Mul a, (Wrap.Add b, c)` painful, an operator spelling can be added without changing anything else.

Native code is why the backstop stays. The runtime calls host functions for text, for maps, for arithmetic, and eventually for whatever a native backend does, and a language that promised its pure code never fails would be making a promise about Rust code it cannot keep. A backstop that is never used costs nothing: it is the same `catch_unwind` the runtime already has. What it buys is that a bug in a native function is reported as an aborted occurrence with the span of the Prism operation that called it, rather than as a process that died. Its kind is `Component`, because the code that failed is code a World or a library put inside a closure and not the program's own; during a build the same failure is contained and reported as `Build`.

Recursion without end in a plain function is the one exhaustion the language turns into an error. A host stack overflow is not a panic in Rust and ends the process, so the closure compiler counts call depth and aborts the occurrence past a limit, as an `Overflow` error with the function's span and the owner of the node whose closure made the call. The limit is the implementation's; the specification says only that a conforming implementation does not hang and does not die. The guard is there to catch recursion without end, not to ration recursion with one: a recursive-descent parser over a long input is an ordinary program and has to work. So the limit is set by memory, not by the host thread's default stack size, which means the closure compiler keeps Prism call frames off the host stack (or runs evaluation on a stack it grows), and a depth a program reaches on purpose is a depth the guard allows.

## What the program sees

From inside an aborted occurrence, nothing. That is the weakness of the occurrence as the unit. A program whose click was lost cannot tell that it was lost, because from the program's side the click did not happen, and a program that could tell would be a program that observed an occurrence that was rolled back. The transaction is doing exactly what it should, and the consequence is that the program's own text is silent about runtime errors, always.

What the program can see is what a World tells it, and a World tells a program things through elements. So the program's view of runtime errors is an element:

```prism
tag Errors of Element:
    responds: Error
```

The response is an `Error` element value, the same value the presence element holds when a build fails or a subtree is quarantined, so there is one error value in Prism code and one at the boundary, and the `match` that reads one reads the other. `errors` declares `responds` alone and nothing else: it is a stream element in the sense of chapter 7, observations arriving on their own, and `Kind Errors` is `Kind(requests=false, responds=true, state=false)`. A World that offers it lists it in `World.Tags`, and the obligation that comes with offering it is the world core's to meet: every `errors` slot in the list as the delivery occurrence opened receives every error from the last turn, as a response, in the first occurrence of the next turn, one firing per error. Aborts and contained build failures travel the same way, the first from the run call's error and the second from `Outcome.errors`, so a program cannot tell from the element which it was except by the `kind`. A slot created in the delivery occurrence hears nothing about those errors, under the creation rule, and no error is replayed to a slot that did not exist when it was reported; a program that wants to know about errors subscribes before they happen, like a program that wants to know about clicks. The Test World is built on the core and delivers the same way, and because it accepts every tag, a script may also `send` an `Error` value of its own to an `errors` slot, which is how a program's error bar is tested without a program that fails. The tag and chapter 10's `errors` step share a name on purpose: the step asserts that the last turn ended in a matching error, and the tag is how a program under test hears of the same error one turn later.

A program subscribes to its own runtime errors the way it subscribes to anything else, binds the event, holds it, draws it, and the discipline is the ordinary one: a response arrives in a later occurrence than the thing it is about, and nothing in the aborted occurrence survives to receive it, which is right because nothing in it existed.

```prism
@with_error_bar: fn @a -> @a = \|@child| @widget:
    !err <- @Element Errors
    $last <- Nil then Some !err
    result <- @child
    @Label (ShowError $last)
    @ result
```

Because it is an element, every property elements have applies. An interceptor can offer `errors` to its children itself, so a shell that hosts other programs can give each program its own errors and keep the others' private: it makes its own `errors` element at the root, and for each error it receives it looks at the `owner`, finds the child whose subtree the owner is under, and answers only that child's `errors` slots. The lookup is `Under`, from the section above, against the paths the shell already holds in each child's `$slots`; an error with no owner, a protocol error, goes to the shell's own log and to no child, since no child caused it. A sandbox can refuse the element, by supplying nothing and never answering, or answer with a redacted value whose `spans` are empty. A test can assert on it. A program that does not create one hears nothing, which is the default and is right: most programs have nothing useful to do with "a cycle was found in a widget you did not write".

The second thing a program can see is the `Error` presence element, in two places. In its own list, where the quarantine or the failed build put it, visible to the GUI library's layout interceptor, which draws it, and to the World, which sees it at the root. And in a child's list, from an interceptor's side, where `$slots` holds a slot whose element is `Error(kind=Cycle, ..)`, and the interceptor may do with it what it does with any element: re-emit it, replace it with a friendlier element, log it, or count it. A supervisor is this:

```prism
IsError: fn Element -> Bool = \|e| match e:
    Error(..): true
    _: false

@Supervise: fn @a -> @unit = \|@child| @widget:
    !restart <- @Button $"Restart"
    $gen <- 0 then $gen + 1 on !restart
    rebuild $gen:
        ((ignore, $slots)) <- intercept @child, !down, $ (\|e| @Element e)
        !down <- @Forward $slots
        let $crashed = Any $slots, IsError
        @Label (if $crashed: $"crashed" else: $"running")
    @ unit
```

`@Forward` is the forwarding loop from the sandbox example, so the child's elements reach the outside unchanged, the `Error` element among them. `Any` is a plain function over the map's slots' elements; `$crashed` is a dynamic by lifting. A restart is a change of `$gen`, which rebuilds the `rebuild`, which builds a fresh child with fresh holds and fresh paths: the retry policy is a counter and a `rebuild`, in the program, where it can be read.

Why there is no `catch`. A `catch` would let a parent react to a child's failure in the occurrence it happened, by running an arm that was not otherwise going to run. That is an effect handler: control transferred from the point of failure to a handler chosen by dynamic extent, with the parent's hold updated by something other than its declared event. The language has none of those, on purpose, and the promise that a hold's definition lists everything that can change it is only true because there is no back door. The supervisor above handles a child's failure in the only way the language handles anything, by reading what the child emits and rebuilding on an event, and it handles it in a later occurrence, as every reaction to the outside is. Suppose `catch` existed anyway, as a widget combinator `@Catch @child, @fallback` that built the fallback when the child failed. It would have to decide what the child's result is in the occurrence of the failure, which is the question the construction boundary answered by not firing; it would have to decide whether the fallback is built in the aborted occurrence, which cannot be, since that occurrence is being rolled back; so it would build the fallback next occurrence, which is `rebuild` over an `errors` subscription, which the program can write. Nothing the combinator could do is beyond the primitives, and the hand version is the real one.

## What the World sees

A differential test has failed. The Rust transcript's last line says the turn ended in a cycle and the Haskell transcript's does not, and what you need, from that one line, is which Prism binding closed the loop, without opening either runtime. The error value out of the run call is designed for that moment: a value with everything a human or a test needs to find the bug, and nothing that names a runtime's own objects.

An aborted occurrence returns an error and no list. A completed occurrence returns the list and, in `Outcome.errors`, every build failure it contained. The two never mix: an error that aborted is the `Err` of the call, and an error that was contained is an entry in the `Ok`. Both are the one type:

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind { Cycle, Overflow, UnknownPath, UnsuppliedState, DeferBackwards, Component, Build }

pub struct RuntimeError {
    pub kind: ErrorKind,
    pub message: String,                   // the human form of what the fields do not hold
    pub occurrence: u64,
    pub spans: Vec<Span>,                  // the loop in order, or the one binding
    pub owner: Option<Path>,               // the lowest owning addressable node, for quarantine
    pub path: Option<Path>,                // the path involved, for UnknownPath and DeferBackwards
    pub element: Option<ElementValue>,     // the element involved, for UnsuppliedState
}

pub struct Outcome {
    pub list: Vec<(Path, ElementValue)>,
    pub requests: Vec<(Path, Vec<Value>)>,
    pub result: Value,
    pub errors: Vec<RuntimeError>,         // contained build failures, this occurrence
}
```

One record with a kind, rather than one Rust variant per kind, because every reader of it prints it the same way and the `Error` element a program matches on has a closed `kind` field; a variant per kind would be unpacked into this record at the boundary anyway. What a kind needs beyond the common fields goes in `message` and `spans`: the loop's edges are `spans` in order with the edge kind in the message, the overflow's operands and operation are in the message, the chain of interceptors a bad response passed through is in the message with each one's `intercept` span in `spans`. The runtime does not know what a turn is, so the turn is not here; the world core pairs the error with the turn it was in when it reports (chapter 13), and that pair is what becomes the `Error` element's `turn` field and the transcript line.

The chain matters for protocol errors because the culprit is whichever interceptor sent the bad map or failed to supply, and the chain says who was in a position to. The world core, not the runtime, constructs the `DeferBackwards` value, because the runtime does not know what `Defer` is; it is the same `RuntimeError` type, with the slot's path and the chain, reported through the same channel. The core also constructs `Component` for its own components' failures. The runtime raises the other five, and `Component` for one thing of its own: a panic inside propagation whose payload is not the runtime's, which can only come from native code a World or a library put in a closure, and the two protocol failures it refuses before scheduling anything, a wrong-typed response and a duplicate response, which are the World's bugs and are named as such.

The compiler's part is that every cyclic placeholder, every lifted operation and every `intercept` is named after its source span, so that a runtime cycle lists Prism bindings and not runtime node ids. The typed AST (Part III) keeps spans through closure conversion for exactly this. A runtime that reported `SwitchHold#4412 -> FilterMap#4419` would be reporting its own implementation.

World policies, each the World's own:

- A *CLI World* prints an abort to stderr, in the format below, and exits nonzero. Nothing else is right for a single program that is a logic error; quarantine would leave a program running with a hole in it, and a user of a command-line tool wants the tool to fail rather than produce output from a damaged program. A contained build failure it prints and continues past, as above, since the occurrence committed and nothing was lost.
- A *GUI World* shows the error and keeps running, and sends the quarantine instruction on its next run call when the owner is below the root, so that the panel becomes a red box and the rest of the window keeps working. Its `errors` element delivers the error to any program subscribed. What it does when the owner is the root, or when there is none, is a question of taste; ending the run with a dialog is honest, and so is keeping the window up with the error drawn over it and every further occurrence lost the same way.
- A *server World* logs the error, quarantines when the owner is under the connection `for`, which is one client, and exits when it is above, which is the service.
- The *Test World* records the error for the `errors` step and the transcript. A turn that ends in an error is a turn whose transcript line says so, and the differential test compares that line between implementations like any other; two implementations that disagree about whether a program has a cycle have found an ambiguity in the specification, which is the find the suite exists for.

### Errors inside `Defer` turns

The sandbox denied a request in occurrence 1 of a turn, and that occurrence committed. The child reacted to the denial in occurrence 2 of the same turn, and its reaction overflowed. The user clicked once. Half of what that click did is committed and the other half is rolled back, and the question is what the turn means now.

A turn is several occurrences when `Defer` slots fire: the input occurrence, then one per round of answers. The occurrence is what rolls back and the turn is what stops. The alternative, rolling back the whole turn, would need the runtime to keep every occurrence's buffer until the World acts, which is a multi-occurrence undo log the runtime does not have; it is the same machinery that optimistic parallelism across occurrences would need (Part III), and if that machinery arrives, whole-turn rollback can be offered with it. Until then, the committed occurrences stand, which is coherent because each was a complete transaction on its own, and the turn ends at the abort.

The `Defer` answers the aborted occurrence was delivering are consumed: the slot counts as answered for this turn. Re-delivering what just failed would, in a deterministic program, fail the same way, and would also break the bound on a turn's length, which depends on each slot being answered at most once. `Defer` slots that fired in committed occurrences and were not yet answered are left waiting, as a program is always left when a response does not come, in a state its type allows; the world core does not run further occurrences of a stopped turn. The World acts on the turn's committed state: it performs the requests of the occurrences that committed, drops the aborted occurrence's, and reports. The error is on the `errors` stream in the first occurrence of the next turn. The core's loop for this is in chapter 13; the rule here is the one it implements, and the Test World's `errors` step sees the list as it stood after the last committed occurrence.

### Errors in World component code

The host's `read` panics on a path with a NUL byte in it, because the program interpolated a text that came off a socket into an `Fs.File` field and nobody validated it. Or a toolkit call returns an error code the component's author never checked. Either way the failure is in the World, in host code, and the program whose slot was involved did nothing a type could have refused.

The world core, which calls every component, contains that failure, and the policy follows from what a request already means to a program. A request component that fails leaves the slot unanswered; a request may always go unanswered, so no lie is told, and `Unsupported` would be one, since the World supports the tag and failed. A state component that fails leaves the previous snapshot value standing, with no push, so every slot still agrees and the program sees a state that did not change; in occurrence zero, with no previous value, it is the same unsupplied-state error a missing component gives. A presence component that fails, a window the host could not open, is reported and otherwise ignored, as an unsupported presence element is. In every case the failure is reported as a `Component` error with the tag, the path and the host's message, through the same channel as runtime errors, so a program subscribed to `errors` and a user reading stderr both learn that the file read did not fail in the filesystem but in the World. Chapter 7 holds the normative text for these obligations and chapter 13 the code that meets them; this chapter only places `Component` among the kinds. None of it is a language rule: a World that bypasses the core gets the host language's own behavior, which is a crash, and deserves it.

## Error messages worth getting right

A message is read by someone who has just lost a click and wants to know which line to look at. The forms below are what the CLI World prints and the Test World records, rendered from the error's `message` and its fields; they are examples of the rendering, not a format the fields must reproduce, and chapter 10's transcript line is the fields themselves, which is what the differential test compares. Each names Prism source, lists what was involved in order, and says what was done about it.

A cycle found at a switch:

```
error: cycle found when a switch adopted a new inner event
  occurrence 7 of turn 4 was aborted; nothing from it was committed

  the loop, in order:
    relay.prism:9   !out      = !Switch !!inner | !tick        (switch, adopted this occurrence)
    relay.prism:6   !!inner   = rebuild $mode: ...                  (generation #3, built this occurrence)
    relay.prism:8             @ !out * 2                        (lifted over !out: prompt)
    relay.prism:9   !out                                        (back to the start)

  the cycle passes through no delayed edge once the switch has adopted generation #3
  owner: @relay at path 1/[relay] (quarantine would replace this node)
```

A cycle found at a build:

```
error: cycle found while building a rebuild block
  the build was abandoned; the rebuild's slot holds an Error element; occurrence 7 of turn 4 continued

  the loop, in order:
    plus.prism:4    !a        = @Plus !tick, !b                 (result depends promptly on !b)
    double.prism:3  !b        = @Double !a                      (result depends promptly on !a)
    plus.prism:4    !a                                          (back to the start)

  @Plus and @Double are widget functions; the block check cannot see through them
  at: view.prism:12, rebuild $page: ..., generation #1 (path 0/#1)
```

An overflow:

```
error: arithmetic overflow
  occurrence 3 of turn 3 was aborted; nothing from it was committed

  totals.prism:4    let $doubled = $count * 2
                    Nat multiplication: 9223372036854775808 * 2 does not fit in a Nat
  $count changed in this occurrence from 4611686018427387904 (totals.prism:3, $count + image_size on !add)
  owner: @totals at path 0/#0 (quarantine would replace this node)

  to keep the overflow as a value, use Checked.Mul; to wrap, use Wrap.Mul
```

A response to a vanished path:

```
error: response sent to a path not in the element list
  occurrence 5 of turn 5 was aborted; nothing from it was delivered

  path 0/#1/0 is not in the list as this occurrence opened
  (it was in the list after occurrence 3; a `rebuild` generation change at clicker.prism:3 replaced it in occurrence 4)
  sent by: the World
```

The same error from inside an interceptor:

```
error: response sent to a path not in the element list
  ...
  sent by: @Rewrite at rewrite.prism:5, through its !down argument (rewrite.prism:6)
  the child's list as this occurrence opened does not contain 3/#2/0
```

An unsupplied state tag:

```
error: no supplier for a state element
  the build was abandoned; the rebuild's slot holds an Error element; occurrence 2 of turn 2 continued

  element: Game.Score(player="ada")       declared at game.prism:4 with state: Nat
  created at: hud.prism:7, @Element Game.Score(player=p)
  reached: @Sandbox (sandbox.prism:12) which forwarded it, then the World
  the CLI World has no component for Game.Score
```

Deferring backwards:

```
error: a Defer slot fired again after it was answered in this turn
  turn 6 ended; the second request was discarded and will not be answered

  Defer at path 1/[4]/#0/1, created at sandbox.prism:19
  answered in occurrence 9; fired again in occurrence 9
  the request event: unit on _.!request (sandbox.prism:19)
  a retry in the occurrence of a denial is a loop; put the retry behind Time.After
```

Two responses to one slot:

```
error: two responses to one slot in one occurrence
  occurrence 2 of turn 2 was aborted; nothing from it was delivered

  path 0/#0/0 (Ui.Button(label="Increment"))
  a slot receives at most one response per occurrence; merge them with | where they are made
  sent by: the Test World (a line with two deliveries to one slot)
  this is a Component error: the fault is the sender's, not the program's
```

The things every message has: which occurrence and turn, what was done (aborted, abandoned, discarded), the Prism spans in the order a reader would follow them, the owner when there is one, and for a protocol error, who sent it. The last line is advice when the fix is known, and most of the time it is.

## Worked scenarios

Six programs, each with one error in it, each followed by what happens occurrence by occurrence. The conventions are the worked examples': paths written as bind positions with `#n` for a `rebuild` generation and `[k]` for a `for` key, and the prelude's plain functions.

### A runtime cycle through two widget functions

```prism
@Plus: fn !Int, !Int -> @!Int = \|!x, !y| @widget:
    @ !x | !y + 1

@Double: fn !Int -> @!Int = \|!in| @widget:
    @ !in * 2

@relay:
    !tick <- @Element Time.Every(period=1000)
    $mode <- Plain then Toggle $mode on !tick
    !!inner <- rebuild $mode:
        match _:
            Plain: @ !never
            Double: @Double !out
    let !out = !Switch !!inner | !tick
    $last <- 0 then !out
    @Label $"last: $last"
```

Two cycles are in here and they are found in different places. Take the simpler one first, in a different `@main`:

```prism
@loop:
    !tick <- @Element Time.Every(period=1000)
    !a <- @Plus !tick, !b
    !b <- @Double !a
    @Label $"a"
```

`!a` and `!b` are a recursive group. The block check sees `!a` bound from `@Plus !tick, !b` and `!b` from `@Double !a`, and because it does not know whether `@Plus`'s result depends promptly on its arguments, it does not reject the group. The build of `@loop` in occurrence zero builds the group with placeholders: `@Plus` subscribes its `|` to `!tick` and to the placeholder for `!b`; `@Double` subscribes its `* 2` to `!a`'s placeholder; the placeholders are closed. Closing `!a`'s placeholder creates the edge from the `|` node to the placeholder's forward, and the eager height walk runs: forward of `!a` → `|` → `!b + 1` → forward of `!b` → `!a * 2` → forward of `!a`, which is being computed. The cycle is found during the build of `@main`. Occurrence zero returns an error of kind `Cycle` and no list, the CLI World prints the build-form message with the loop listed through `plus.prism` and `double.prism`, and exits nonzero. The program never started, which is what should happen to a program whose structure is wrong before any input exists.

Now `@relay`, where the cycle only exists in one mode. The block check sees `!!inner` depends on `$mode`, the `Double` arm depends on `!out`, `!out` depends on `!!inner` through `!Switch`, and `!Switch` is a delayed edge, so the block is legal. It has to be: this is exactly the structure of a button that relabels itself, and that is legal and useful. For the scenario, `@relay` is one panel of a dashboard, built by a `for` over the panel names at position `1` in `@main`, so its widget is at `1/[relay]`, its timer slot at `1/[relay]/0` and its label at `1/[relay]/1`.

**Occurrence 0 (build).** `$mode` is `Plain`; the `rebuild` builds generation `#0`, whose block is `@ !never`; `!!inner` fires with `!never`; `!Switch` has it from occurrence 1. `!out` is `!never | !tick`. The list has a `Time.Every` and a label under `1/[relay]`.

**Occurrence 1.** `Tick` to the timer slot. `!tick` fires; `Toggle $mode on !tick` fires with `Double`; `$mode` changes promptly; the `rebuild` builds anew: generation `#1` builds `@Double !out`, which subscribes `!out * 2` to `!out`. This edge, from `!out` into the new generation, is not a cycle yet: nothing from the generation feeds back into `!out` until the switch adopts it. The build succeeds. `!!inner` fires with the new inner event. `!out` fires with the tick's value from the `|`'s right side; `$last` is updated in the buffer; the label rebuilds in the buffer. Propagation finishes. At commit, before the batch in which holds and switches commit, the eager walk runs for the switch's adoption of generation `#1`'s event: switch → `!out * 2` → `!out` → switch. Found. The commit is abandoned: `$mode` stays `Plain`, `$last` stays `0`, generation `#1` is dropped and generation `#0` is kept, the switch keeps `!never`. The run call returns the switch-form message, with `owner` `1/[relay]`. Occurrence 1 did not happen and the tick was lost.

**Occurrence 2, under a CLI World.** There is none; the World exited.

**Occurrence 2, under a GUI World that quarantines.** The error named the owner: the lowest common owning widget of the switch (bound in `@relay`'s block) and the generation's `* 2` node (owned by the generation, owned by the `rebuild`, owned by `@relay`'s widget) is `@relay`'s widget, which has no segment of its own, so the owner is the key's block at `1/[relay]`. The World's next run call says `quarantine: [1/[relay]]`. The runtime finds the path among the owners it named last, tears the subtree down before routing, and the list has `1/[relay]/0: Error(kind=Cycle, ..)` where the timer and the label were. A tick queued for `1/[relay]/0` under the old list is delivered to nothing and reported with its path, and the World discards it. The dashboard around it keeps working, and its layout interceptor draws the error. Had the World sent `quarantine: [1/[clock]]`, a panel no error named, the run call would have returned a `Component` error naming the path and run nothing.

**Occurrence 2, under a GUI World that does not quarantine.** Every tick toggles `$mode`, every toggle is aborted, every tick is lost, and the program sits at `last: 0` forever with an error on stderr per second. Nothing is inconsistent; the program is stuck in an honest way. The `errors` stream tells any subscriber so.

The fix in the program is to say what the program meant. If `Double` mode was meant to double the tick, `@Double !tick` instead of `@Double !out` has no cycle. If it was meant to feed back, the feedback has to pass through a delayed edge that is not the switch's choice, which means a hold: `$prev <- 0 then !out` and `@Double ($prev on !tick)`, which doubles the previous value at each tick and is what feedback means in a language without prompt loops.

### An overflow in a derived dynamic

```prism
@totals:
    !add <- @Button $"Add a disk image"
    let image_size: Nat = 4611686018427387904
    $count <- 0 then $count + image_size on !add
    let $doubled = $count * 2
    @Label $"bytes: $doubled"
```

`$count` is a `Nat`, because `image_size` is annotated as one and inference takes the literal `0` and the `2` along with it. The increment is large so the scenario fits in three clicks.

**Occurrence 0 (build).** `$count` is `0`, `$doubled` is `0`. The list is a button and a label.

**Occurrence 1.** `Click`. `$count + ... on !add` fires with `4611686018427387904`; `$count`'s update is buffered; `$doubled` recomputes promptly to `9223372036854775808`, which fits; the label rebuilds. Commit. `bytes: 9223372036854775808`.

**Occurrence 2.** `Click`. `$count` would become `9223372036854775808`, which fits in a `Nat`; `$doubled` recomputes: `9223372036854775808 * 2` does not. The multiplication is the lifted operation at `totals.prism:4`, and it detects the overflow and aborts. The buffer is dropped: `$count` stays at its occurrence-1 value, `$doubled` stays, the label stays. The run call returns the overflow message, naming the operation, both operands, the binding whose change led there, and the owner, which is the widget that owns the `* 2` closure's binding: `@totals`'s widget if it has a segment, or its nearest ancestor that does. The click is lost.

**Occurrence 3.** `Click` again. The same, every time. The program is stuck one click before the overflow, which is the honest outcome: the alternative, wrapping, would have shown `bytes: 0` and been believed.

Quarantine is possible here but pointless: the owner is the whole counter, and replacing it with an error box to stop a click from being lost is no better than losing the click. A World's quarantine policy should be about keeping the rest of a program running, not about every error.

The fix is in the type or in the spelling. If the total can legitimately exceed a `Nat`, it is not a `Nat`; `Float` or a bignum from the prelude says so. If the program wants to show the user that the sum did not fit, `Checked.Mul $count, 2` is a `$?Nat` by lifting, and the label's text comes from a `match` with an arm for `Nil` that says `bytes: too many`. That arm is the program handling its own failure as a value, which is what this chapter says should happen to every failure that can be one.

### A World sending to a vanished path

The relabelling button of the worked examples:

```prism
@clicker:
    !!clicks <- rebuild $label:
        @Element Ui.Button(label=_)
    let !click = !Switch !!clicks
    $count <- 0 then $count + 1 on !click
    let $label = $"Clicked $count times"
    @Label $label
```

After occurrence 1 the button is at `0/#1/0` and the list the World received says so; `0/#0/0` is gone. Suppose the World has a click for `0/#0/0` queued from before it read the list after occurrence 1, and, in violation of its obligations, sends it.

**Occurrence 2.** The run call's responses are `{0/#0/0: Click}`. Routing is against the list as occurrence 2 opened, which is the list after occurrence 1. `0/#0/0` is not in it. The runtime does not deliver the click to nothing, and does not deliver it to `0/#1/0` on the grounds that it is "the same button", because it is not: the user clicked a button labelled `Clicked 0 times` and that button is gone. It aborts, before any propagation, and returns `UnknownPath` with the path in `path`, no owner, and the sender in the message, which is the World.

Why abort the whole occurrence rather than drop the one stray input and deliver the rest? If the click had come on one line with a legitimate response to `0/#1/0`, the legitimate one is lost too. The argument for dropping only the stray is that the program is being punished for the World's bug. The argument for aborting is that the run call's contract is all-or-nothing and the obligation was cheap to meet: the world core checks every path against the list it last received before it calls the runtime, so a World built on the core never trips this error, and a World that bypassed the core and got the list wrong may have got the rest of the occurrence wrong too. The runtime check exists for Worlds that bypass the core and for interceptors, whose `!down` maps are Prism and can have stale keys through an ordinary bug, which is the next form.

**The same error from an interceptor.** `@Rewrite` of the specification holds its child's `$slots` and routes `!down` by the child's paths. Suppose a version of it holds responses in a hold across occurrences, so that a response for a child slot that was torn down in occurrence 4 is still in the map it fires in occurrence 5. `intercept` routes the map against the child's list as occurrence 5 opened; the path is not there; occurrence 5 is aborted with `UnknownPath`, the sender being `@Rewrite` by its `intercept` span, which is the one entry in `spans`. This is an interceptor bug and the abort is the runtime finding it; a World's own stale-response discard does not apply inside the program, where the interceptor is the World from the child's point of view and has the same obligation.

### A state tag with no supplier

```prism
tag Game.Score of Element:
    player: Text
    state: Nat

@hud: fn $Text -> @unit = \|$player| @widget:
    $score <- @Element Game.Score(player=$player)
    @Label $"score: $score"
```

`Game.Score` is a state tag the program's author declared. A game World has a component for it. The CLI World does not. The element has a dynamic field, so it lifts into a `rebuild`: a fresh `Game.Score` slot per change of `$player`.

**Occurrence 0 (build), under the CLI World.** `@hud` is built under `@main`; the `rebuild` builds generation `#0`; `@Element Game.Score(player="ada")` creates a state slot and asks for its value, in this occurrence, because a dynamic without a value is not a dynamic. The request goes outward: no interceptor between here and the root supplies it, so it reaches the World's oracle, and the CLI World has no component for `Game.Score`. The oracle cannot fail, so this is not the oracle failing; it is a tag the World does not accept, which is a build error in the occurrence the slot is created. The build that was in progress is `@main`'s, so occurrence zero returns `UnsuppliedState` with the element in `element`, the chain (empty here) in the message, and the World's name, and the CLI exits with the message above. The program could not have run: there is no later occurrence in which a value could appear.

**Occurrence 3, under a sandbox that forwards.** Put `@hud` under `@Sandbox` under the CLI World, and have `$player` change in occurrence 3. The `rebuild` builds anew mid-occurrence: generation `#1` creates `Game.Score(player="bob")`; the sandbox's supplier is applied to it and returns `@Element Game.Score(player="bob")`, the forwarding case, which creates a slot in the sandbox's widget, which asks the World, which has no component. The build in progress is generation `#1`'s. It is abandoned: the half-built generation is dropped, the sandbox's supplied widget with it, the `rebuild`'s slot holds an `Error` element at `0/#1/0`, the `rebuild`'s event does not fire, and `$score`, which is a hold of dynamics joined with `$Join`, keeps generation `#0`'s dynamic and its last value. Occurrence 3 continues and commits; `Outcome.errors` has one entry, kind `Build`, with the unsupplied tag in its message and owner `0/#1`. The label still says ada's score, which is stale, and the `Error` element is in the list beside it, and the `errors` stream says why in the next turn. The program is wrong in a visible way, and nothing else in it was touched.

**Under a sandbox that supplies.** A sandbox whose supplier has an arm `Game.Score(..): @ $ 0` never lets the tag reach the World, and no error occurs anywhere; the hud shows zero. That is the rule that unavailability lives in the type at work: a supplier's type says it returns a blueprint for every element, so an interceptor cannot fail to supply, and the only place a state tag can go unsupplied is the World, whose set of components is finite and known.

**Under the Test World.** The Test World accepts every tag, so the question is what value it gives. A `with Game.Score(player="ada"), 7` step before the build sets the snapshot and the oracle answers `7`. With no `with` step, the test fails at the slot's creation with the unsupplied-state error, which is the test being told it forgot to set up a state it used, before any step runs.

### Deferring backwards

The sandbox of the worked examples, with a child that does the thing the error exists to catch:

```prism
@impatient:
    !started <- @started
    !r <- @Element Fs.File(path="/etc/shadow", requests=(Read on !started | Read on !denied))
    let !denied = !Filter (IsDenied !r)
    @Label $"trying"
```

On denial, the child asks again, in the occurrence of the denial. Under a real filesystem that denies at once, the World's response would arrive a turn later each time and the program would be a slow loop the user could stop. Under the sandbox, the denial comes through `Defer`, and the loop closes inside one turn.

**Occurrence 0 (build).** The child's `!started` fires; `Read` fires on the `Fs.File` slot; the sandbox's `for` builds the `Nil` arm for it, because `/etc/shadow` is outside the root; `unit on _.!request` fires, so the sandbox's `Defer` slot's request fires. The run call returns; the world core sees a fired `Defer` and runs another occurrence of the same turn.

**Occurrence 1, same turn.** `unit` is delivered to the `Defer` slot. `!later` fires; `Denied on !later` fires; the `for` consolidates it; `!down` fires with `{p: Denied}`; the child's slot receives `Denied`; `!r` fires; `!denied` fires; `Read on !denied` fires, so the child's request fires again, in this occurrence; the sandbox's block for that slot is still there, so `unit on _.!request` fires again, and the `Defer` slot's request fires again. Propagation completes and the occurrence commits, because the runtime does not know what `Defer` is and nothing in the occurrence was wrong by the runtime's rules. The run call returns with the `Defer` slot's request firings listed.

The world core now sees that a `Defer` slot it answered in this turn has fired again. That is deferring backwards: answering it again would be a second answer in one turn, which the bound on a turn forbids. The core ends the turn, reports `DeferBackwards` with the slot's path in `path`, the chain in the message, and the two occurrences, and discards the second request. It does not answer it in the next turn, because that would turn the loop into one denial per turn forever, with an error per turn, which is a hang with extra steps. The child is left waiting for a response that will not come, which its type permits; the label says `trying`, which is true.

Occurrence 1 was committed, not aborted, and that is right: the occurrence was consistent, and the error is a protocol error between the interceptor and the World that the runtime could not see. The World's policy applies at the turn: the CLI exits nonzero, the GUI shows the message and continues, the Test World records it for an `errors Error(kind=DeferBackwards, ..)` step, which is how the suite pins this rule.

The message says what to do instead, and it is the general rule for retries: a retry in the occurrence of a denial is a loop, and a retry belongs behind `Time.After`, where the World's clock separates the attempts and the user has a chance to stop them. The sandbox is not at fault and needs no change; it answered once, as it must, and the child asked again at once, which no interceptor can answer without starting an occurrence of its own, which nothing in Prism can.

### A `for` key whose block fails to build

The server of the worked examples, with a handler chosen per connection, and one handler that has the cycle of the first scenario in it:

```prism
@Join: fn !Bytes, !Bytes -> @!Bytes = \|!x, !y| @widget:
    @ !x | !y

@Rev: fn !Bytes -> @!Bytes = \|!in| @widget:
    @ Reverse !in

@Echo: fn !Bytes -> @!Bytes = \|!in| @widget:
    @ !in

@Mirror: fn !Bytes -> @!Bytes = \|!in| @widget:
    !a <- @Join !in, !b
    !b <- @Rev !a
    @ !b

@server:
    !accepted <- @Element Net.Listener(address=":7000")
    $n <- 0 then $n + 1 on !accepted
    !closes <- for $conns:
        !ev <- @Element Net.Conn(id=key, requests=!reply)
        !out <- match _:
            Echo: @Echo (!Filter (Received !ev))
            Mirror: @Mirror (!Filter (Received !ev))
        let !reply = [[Send !out]]
        @ Shut on !Filter (IsClosed !ev)
    $conns <- {} then ApplyDeltas $conns, (!Union (Open !accepted, ModeFor $n), !closes)
    @ unit
```

`ModeFor` is a plain function that picks a handler by connection number, `Echo` for most and `Mirror` for every seventh, which is the kind of thing a load experiment does. `@Mirror` has a prompt cycle through two widget functions, `!a` from `@Join !in, !b` and `!b` from `@Rev !a`, that the block check cannot see, because it does not know that `@Join`'s `|` is prompt in its second argument. `$conns` is a map from connection id to its mode. The listener is at `0`, the `for` at `1`, and a key's `Net.Conn` slot is at `1/[id]/0`.

**Occurrence 0 (build).** One slot, the listener; `$conns` is `{}`; the `for` builds nothing.

**Occurrences 1 to 6.** Six `Accepted` responses, one per turn. Each adds a key to `$conns` with mode `Echo`; the `for` builds the key's block mid-occurrence; `@Echo` has no cycle; the block creates its `Net.Conn` slot at `1/[id]/0`. The list after occurrence 6 has the listener and six connections. Each client's bytes come back reversed by nobody and unchanged by everybody, which is what an echo server is.

**Occurrence 7.** `Accepted 7`. `$n` becomes 7; `ModeFor 7` is `Mirror`; `$conns` gains `7: Mirror`; the `for` builds the block for `[7]` at the height where `$conns` changed. Inside the block, `@Element Net.Conn(id=7, ..)` creates the slot at `1/[7]/0`, and `@Mirror` builds its recursive group with placeholders; closing `!a`'s placeholder creates the edge from `@Join`'s `|` to the forward of `!a`, and the eager walk runs: forward of `!a` → `|` → forward of `!b` → `Reverse` → forward of `!a`, which is being computed. Found, inside the build of the key's block. The build is abandoned: the half-built widget, its `Net.Conn` slot and `@Mirror`'s widgets are dropped, and an error widget takes the block's place at `1/[7]`, with `1/[7]/0: Error(kind=Build, ..)` as its one slot. The consolidated `!closes` has no entry for `7`, and the other six keys were not touched: no hold in them changed, no subscription moved, and their slots are where they were. The occurrence continues at the next height and commits. The run call returns the list, with the six connections and the error element, and `Outcome.errors` with one entry: kind `Build`, message naming the cycle, `spans` the loop through `mirror.prism` and `join.prism`, owner `1/[7]`.

**What the World does.** The CLI World, which includes `Net`, prints the build-form message to stderr and continues; the six clients are being served and nothing justifies dropping them. Connection 7 has no `Net.Conn` slot and never will, since the error widget is what the key built; under the World's own rule for a connection the program never claimed, the World closes it after its grace period. Nothing in the program asked for that close; the World is reconciling an accepted socket to a list that does not mention it. Key `7` stays in `$conns` with its error widget until the program removes it, which it never will, since `!closes` never fires for it; a server that wants the key gone subscribes to `errors`, reads the owner, and removes the key whose path it is, which is three lines and an `Under`. A server World that quarantines has nothing to do here: the failure was contained at the build and the error widget is already in place.

**Occurrence 8.** `Accepted 8`. `ModeFor 8` is `Echo`; the key builds and serves. The seventh connection's failure left no trace in the eighth's build, which is the property the scenario exists to pin: a `for` block's build failure is that key's and no other's.

The fix is in `@Mirror`, as in the first scenario: the feedback needs a delayed edge, `$prev <- Empty then !b` and `@Join !in, ($prev on !in)`, which mirrors the previous reply into the next one and is what the handler presumably meant.

## The invariants, collected

What the runtime promises:

- An occurrence that cannot complete is abandoned, and afterwards every hold, switch, node, slot and path is as it was when the occurrence opened. Nothing is half done, ever.
- A build that cannot complete, inside `rebuild` or `for`, is abandoned without aborting the occurrence; the slot at the block's path with segment `0` holds an `Error` element; the failure is in `Outcome.errors` with kind `Build`.
- A prompt cycle is found when the edge that closes it is created, during a build or as the first step of the commit that adopts it, never by looping, deadlocking, or firing a node twice. The scheduling-height check remains as an assertion against the runtime's own bugs.
- Overflow in the default arithmetic and recursion past the depth guard abort the occurrence as `Overflow`; a failure in native code aborts the occurrence as `Component`; the error names Prism spans.
- A response to a path not in the opened list aborts as `UnknownPath`; a response of the wrong type and two responses to one slot abort as `Component` with the sender named; a state tag that reaches the root with no component abandons the build or, at the root, aborts as `UnsuppliedState`.
- A quarantine instruction names only an owner the last error named, or the run call returns a `Component` error and runs nothing; a performed quarantine puts the `Error` element at the owner's path with segment `0` and reports every input addressed under the owner, with its path, in the outcome.
- The error out of the run call, and every entry of `Outcome.errors`, is a value with the occurrence, the kind, the spans, and the owner where there is one, and nothing that names a runtime object.

What the World promises:

- To treat the inputs of an aborted occurrence as undelivered and keep its snapshot bookkeeping consistent with that.
- To pair every error with its turn, and, if it lists `errors` in `World.Tags`, to deliver each error of a turn to every `errors` slot in the list as the next turn's first occurrence opened, as a response, one firing per error.
- To send `quarantine` only for an owner the last error named, and to discard the inputs the outcome reports as delivered to nothing under it.
- To stop a turn at an abort, perform the requests of the occurrences that committed, leave unanswered `Defer` slots waiting, and report.
- To report deferring backwards, discard the stray request, and end the turn.
- To contain its own components' failures to a withheld response or an unchanged snapshot, reported as `Component`, never a crash and never `Unsupported`, if it is built on the core.

What an interceptor promises: nothing beyond what every program promises, because an interceptor is Prism. Its stale `!down` key is a program bug found by the runtime; its supplier cannot fail by type; its `Defer` misuse is found by the World.

What the program promises: nothing. From inside, nothing happened.

The negative space, as a list, since it is what makes the rest cheap. There is no `catch`, no `try`, no error type the runtime hands to Prism code except through an element, no retry the runtime performs on its own, no partial commit, no whole-turn rollback, no error that a World sees as an exception, and no error whose report names a runtime node instead of a Prism span. Each absence is a mechanism the implementation does not have to build: no unwinding through closures, no handler lookup, no multi-occurrence undo log, no scheduler for retries, and one error channel, the run call's return value and its outcome's error list, for everything. The transaction was already there, and this chapter is the list of what it turns out to cover.

## Causality in types

A cycle in the prompt graph inside one widget block is a compile error with every binding in it listed, and that check has been enough for every program in the suite. Then you write a layout library. `@Pack` takes a list of children's size dynamics and returns a dynamic of positions; a child takes its position and returns its size; both are widget functions, each is acyclic inside its own block, and the first time a program wires a child's size from its own position through both of them, the cycle exists only at the call site, where neither function's body is visible. The compiler says nothing. The runtime finds it the first time the occurrence runs, a node scheduling below the current height, and aborts the occurrence with the spans of the two bindings at the call site and no explanation of why they form a cycle. This item is a type-level record of which arguments a function's result depends on promptly, so that the check inside a block can cross a function boundary and the runtime's detection becomes a backstop. It is an effect system in a small way, and it is research.

### The check as it stands

Inside a widget block, every binding is a node and every use is an edge, labelled prompt or delayed by the construct that makes it: lifting over an event or a dynamic, `|`, `for` consolidation and `rebuild`'s rebuild are prompt; a tag, `!Switch`'s choice of inner event and a hold's value side are delayed. The resolver finds the strongly connected groups; the elaborator rejects any group whose cycle has no delayed edge. A widget function's body is one such block, so its own cycles are caught. What the check cannot see is an edge from an argument to the result: `@F !a` returns `!b`, and whether `!b` fires promptly on `!a` is a fact about `@F`'s body that its type `fn !A -> @!B` does not state. At the call site the elaborator must assume either that every result depends promptly on every argument, which rejects the layout library and most of the GUI corpus, or that none does, which is what it assumes now and why the cycle above is found at runtime.

### The annotation

A function type gains, for each sigiled result, the set of arguments it depends on promptly. Written out, in a form a signature could use:

```prism
@Button: fn $Text -> @!Click                       # result depends promptly on nothing
Scale:   fn !Int, $Int -> !Int    prompt !_1       # result fires promptly on the first argument, reads the second through a tag
@Pack:   fn [$Extent] -> @$[Pos]  prompt $_1         # positions change promptly when any size changes
```

The default, with no annotation, is that the compiler infers the set from the body: a function's result depends promptly on an argument if there is a path of prompt edges from the argument to the result inside the body. Inference is a reachability pass over the same labelled graph the cycle check already builds, so it costs nothing new. Written annotations are for signatures the author wants to pin, the way a type annotation pins an inferred type, and the body is checked against them.

At a call site the elaborator then adds, for each argument the result depends on promptly, a prompt edge from that argument to the binding of the result, and the group check runs as before. The layout cycle becomes:

```prism
$positions <- @Pack [[$size]]        # prompt on $size, by @Pack's type
$size <- @Child (First $positions)   # prompt on $positions, by @Child's type
```

`First: fn [Pos] -> Pos` is a plain library function, lifted over `$positions`, and the lifted application is prompt like any lift. Two prompt edges and no delayed one, rejected with both bindings' spans and the two signatures that make the edges prompt. The fix is the one the author would have reached for after the runtime abort, a tag or a hold between the two, and now the message says so before the program runs.

The same cycle through an element is the case the annotation has to be exact about, because it is the case the GUI library actually has. The library's `@HBox` is `fn @a -> @a`: an interceptor over its child, with the child's `Ui.*` elements in a `Ui.HBox(children=...)` element it puts out. A layout interceptor above it supplies positions as state to the children, so a child that wants its position asks for it through a tag:

```prism
tag Ui.Placed of Element:
    size: Extent
    state: Pos
```

A child writes `$pos <- @Element Ui.Placed(size=$size)`, and a dynamic in a field lifts into a `rebuild`: a fresh slot per size change, with the slot's state read from the supplier the layout interceptor built. If the child then computes `$size` from `$pos`, the loop runs through the element list: `$size` changes, the `rebuild` builds anew the slot promptly, the interceptor's `$slots` changes promptly, the supplier's state dynamic for the new slot is built from `$slots` and read into `$pos`, and `$pos` feeds `$size`. Every edge in that loop is prompt, including the one that looks as if it might not be, inside `intercept`: the state a slot sees is the supplier widget's result, and the supplier is built in the interceptor's widget at slot creation, as a widget built mid-occurrence that sees the occurrence's inputs, so it is prompt on `$slots`. So the loop is a real cycle and the runtime finds it today. The annotation that catches it at compile time is on `@Element`: for a state tag, the returned `$State` is prompt on the slot's element value, and the element value is prompt on every dynamic that went into a field, because each one is a `rebuild`. With that one rule the elaborator sees `$pos` prompt on `$size` across the `@Element` boundary, and the message names the field. The two-function form above and the element form are the same cycle; the element form is the one where the type has to say something at a boundary the program did not write.

### What makes it research

Three places where the small version stops being small.

*Higher-order functions.* `Map: fn (fn a -> b), !a -> !b` is prompt on its event argument, and whether it is prompt on anything else depends on the function passed in, which has no sigiled arguments and so no dependency set of its own. The clean answer is that a plain function's dependency is empty, since a plain value has no time in it, and the only prompt dependencies come from sigiled arguments. That holds for every prelude function. It breaks when a function argument is a widget function, `fn (fn $A -> @!B), $A -> @!B`, whose dependency set is a variable the caller fills in. That is dependency polymorphism, and it is the point at which the annotation becomes an effect system with variables and inference becomes unification over sets. It is tractable; it is also where the design has to stop being a reachability pass and become a type system, and the payoff should be measured against how often widget functions take widget functions in real libraries.

*Switching.* A `rebuild` over `$x` returns an event that fires promptly on `$x`'s change; the block's result is built fresh each time, and what the result depends on inside the block is the block's business and acyclic by the block check. So `rebuild` contributes exactly one prompt edge, from its dynamic to its event, and the block's dependency on captured outer dynamics is prompt through them as well. `!Switch` is delayed by definition and contributes nothing. `for` is prompt on its map and its consolidated result is prompt on whatever the block's result is prompt on, through the per-key `$_`. None of these needs the annotation to say anything new, but the inference pass has to understand them, and the construction-in-the-middle rule means a cycle formed by a rebuild is a cycle the new block's result makes at the `rebuild`'s call site, which the annotation on the block's captured names covers.

*Interceptors.* The recursive binding every interceptor has, `!down` defined from `$slots` which `intercept` produces from `!down`, is legal because routing reads the list as the occurrence opened, which is a tag. `intercept`'s type has to say so: its result list is prompt on the child and delayed on the responses argument. That is one annotation on one primitive and it is exact.

### What it must not change

No program that was legal and never hit a runtime cycle becomes illegal, which is soundness in the direction that matters, and it holds wherever the inference can see a body: there the annotation only adds prompt edges that exist, so a rejected program is one that would have aborted on the first occurrence to exercise the cycle. Where the inference cannot see a body it over-approximates, and the two places are named so a reader knows where a rejection may be of a program that would never have aborted. A blueprint parameter, `@child` in `fn @a -> @a`, has no body at the call site, so its result is treated as prompt on every sigiled argument the caller passed it. A `for` over a blueprint parameter whose body is not visible is treated as able to fire at birth, which is the rule the static cycle check has already and this pass inherits: that adds a prompt edge from the collection's change to the consolidated result that may not exist. In both cases the written annotation on the parameter is what the author writes to get the program accepted: `fn (@a prompt nothing) -> @a` says the child's result depends promptly on nothing, the elaborator checks the argument's inferred set against it at the call site, and the body of the function is checked under that assumption. That is the same bargain a type annotation on a higher-order parameter makes, and it keeps the default conservative without making it final. The runtime's detection stays, because a cycle formed through a path the type system does not track, a function value stored in a record and applied later, for instance, still has to be found and still has to abort the occurrence rather than loop. The typed AST gains a field on function types and nothing else changes in it; the Haskell implementation can ignore the field, because Reflex finds the cycle at runtime the same way the Rust runtime does, and the differential test is unaffected because an accepted program means the same under both.

### The fact it waits on

Runtime cycle errors in practice: a library whose users hit the abort at call sites and have to read two function bodies to understand it. The GUI library's layout widgets are where it will happen first if it happens. Until then the block check plus the runtime backstop is the design, and the annotation is a note about where the type system would go next.
