# Chapter 1 — A language whose only subject is time

The bugs you chase in an interactive program look like many bugs and are one: a value whose writers are not a fact about its type. Functional reactive programming has two types that make the writers a fact, a value that happens and a value that holds, tied by a law: a held value changes only through the events named in its definition, and only when the moment ends. A law like that holds only where the language keeps it. A library can offer it, and cannot stop the code around it from breaking it. Prism is a language that keeps that law and no other, and the list of what it refuses is the evidence that it had to be a language.

## The afternoon

You have had the afternoon where a checkbox un-checks itself. The ticket says: tick "show archived", the box ticks, and then, about one time in five, it unticks. You reproduce it on the second try. You follow the click into the handler, which sets a flag on the model. The model's setter notifies its observers. One observer is the list, which re-renders. The re-render rebuilds the filter bar, and the checkbox in the filter bar reads its state back from the model. You put a breakpoint in the setter and read the stack. The notification went out from inside the setter, one line before the field was assigned. The list read the old value, drew an empty box, and then the field changed under a screen that had already been painted. You move a line. It works. You ship it.

Three months later the ticket comes back. A different checkbox, the same symptom. The setter is fine this time; the line you moved is still where you put it. This time the re-render wakes a second observer, added in the spring for a feature you never saw, which writes the flag from a copy it cached before the click. You fix that too. You know there will be a third.

Or the counter. A field holds a number. A button increments it. A label shows it. Then a keyboard shortcut increments it too, a reset button zeroes it, a message from the server sets it to the server's count, an undo stack restores an old value, and next week's feature will do something nobody has thought of yet. Six writers, each a callback written wherever its trigger lives: six files, in six modules with careful boundaries and private fields. Ask the counter two questions. What can change it? The answer is a search, and the search finds the writers it can find; it does not find the one that reaches the counter through a method three calls away. If the reset and the server's message land in the same frame, which wins? Whichever handler was registered first, typically, and that depends on the order the modules loaded, which is nowhere in the program's text. The modules kept the writers tidy. They said nothing about when.

Or the row. You remove an item from a list. Its click listener was subscribed to a store, so the store still holds the listener, and the listener still holds the row. On the next update the store calls it, and it writes to a widget nobody can see. In the browser's memory tools the detached subtree is alive: reachable, so not garbage. The list removed it, and the removal meant nothing, because something else could keep it.

Or the dependency array. An effect runs when `a` or `b` changes, and `[a, b]` sits next to it, written by hand. Someone adds a read of `c` inside the effect and leaves the array alone. The code and its declared dependencies are two statements of one fact, and they have drifted apart. The linter noticed, in the spring, and someone suppressed the warning with a comment explaining why.

None of these was a stupid bug, and none was written by a careless person. Each was written by someone competent at six in the evening. In each one, nothing complained.

## One bug with four names

The field has names for these. Uncontrolled mutation. Action at a distance. "Who can change this?" Order within a moment. Four names, and one thing underneath: a value whose set of writers is not a fact about its type. Because the set is open, you cannot answer who changes the value. Because the writers live elsewhere, a write here moves a value there, and the connection sits near neither end. Because the writers are unordered, two writes in one moment have no stated order, and the program's history decides. The fourth face shows up as soon as you try to cope with the first three.

A callback answers "what do we do when this happens". Keeping an invariant needs the other question, "what can happen to this", and a program made of callbacks has no place where that answer is written. Maier, Rompf and Odersky opened their case against the observer pattern with one sentence, "Programming interactive systems by means of the observer pattern is hard and error-prone", and two of their nine complaints are the two halves of the afternoon: "there is no guarantee for data consistency in the observer pattern", and the example "is hard to understand because the control flow is inverted".

So every reactive system of any size grows the same things inside it. Dirty flags, so a value knows to recompute. A batching layer, so two writes in one handler produce one redraw. An ordering rule, so a derived value runs after the values it derives from. And a page on the wiki titled "Conventions" that says who may call what, which is a type system written in a wiki. Together these are a runtime, written badly, inside the application.

Moving the runtime into a framework helps, and the rules still leak into your head. React's docs are exact about it: "React waits until *all* code in the event handlers has run before processing your state updates", and "React does not batch across *multiple* intentional events like clicks". Write `setNumber(number + 1)` three times in one handler and the number goes up by one. Write `setNumber(n => n + 1)` three times and it goes up by three. Same three lines, a different spelling of the setter, a different answer, and the rule that decides it is in the framework's documentation, not in your program.

Two communities needed a word for seeing a value half-changed. React's is tearing: "In a user interface, by 'tearing' we mean that a UI has shown multiple values for the same state", which the React 18 working group called "a necessary consequence of concurrency". The FRP literature's is glitch: "such a momentary view of inconsistent data is known as a glitch". The Leptos book draws the standard diamond (one signal, two memos over it, an effect over both) and says a naive push implementation is "glitchy (`D` actually runs with the incorrect value for the second memo during its first run.)" Every serious system solves this, each inside its own scheduler, and each teaches you the scheduler's rules. The checkbox afternoon was a glitch you saw with your own eyes.

## Who may write, and when

You already use one small subtraction and think well of it. `const` says this name will not be reassigned. It removes sentences from your language, and your programs got easier to read when you started using it. Hold on to that feeling; the rest of this chapter asks for bigger versions of it.

Every language answers three questions about a value: who may write it, from where, and in what order. C's answer is anyone, anywhere, any time. Rust's is in its book: "At any given time, you can have *either* one mutable reference *or* any number of immutable references", and "the benefit of having this restriction is that Rust can prevent data races at compile time". A data race, in the same chapter, is two pointers at the same data, one of them writing, and nothing to order them: a rule about who and when. The borrow checker enforces it on every expression, and no crate could have done the same job. Haskell's answer is: whatever the monad you are in says, and you choose the monad.

There is a fourth answer. A value changes only through the events named in its definition, and only when the moment ends.

`$checked <- False then !toggle | !reset`

Read it as a sentence. `$checked` starts `False`. It moves when `!toggle` fires or when `!reset` fires. Nothing else moves it: no setter, no notification, no other file. A rule like this needs a noun, and the plain one is *mutation policy*: a rule about who may write a value, from where and when, which the language keeps on every expression, so a program that breaks it does not compile. A policy is in force whether or not anyone wrote it down, and every program in the first section had one: C's, in a framework's clothes.

The two types behind the fourth answer are older than any framework you use. Elliott and Hudak's Fran gave them their names in 1997: a *behavior* is a value that can vary over time, and an *event* is a what-and-when pair. The Haskell wiki's one-line pitch is the checkbox's own: "A better way to represent an edit widget's content is a time-varying value, called a behavior." Reflex, the library that made the idea practical, states the law that ties the two together in its `Dynamic` documentation: "a combination of a 'Behavior' and an 'Event', with a rule that the 'Behavior' will change if and only if the 'Event' fires". And its `hold` documentation states the rule for reading one: a sample taken in the moment that changes it "will see the __old__ value of the 'Behavior', not the new one".

Prism keeps both and gives up one thing on purpose. Elliott's behaviors are continuous; a Prism dynamic is piecewise constant, a value that changes at discrete moments and holds between them. Elliott's own paper on implementing FRP begins "Functional reactive programming (FRP) has simple and powerful semantics, but has resisted efficient implementation." The restriction to piecewise-constant values is what can be implemented by pushing changes, and the part of FRP that survives it is the part that is a mutation policy: a value changes only when its event fires, and nothing observes the middle.

Prism writes the type in the name. `x` is plain, true now and forever. `!x` is an event, a value that occurs. `$x` is a dynamic, a value that is true now and changes at occurrences. `@x` is a blueprint, a description of something that, built, has a lifetime. `$count + !delta` tells you, before you know what either name means, that one of them is live and one of them happens, and that the sum happens; a compiler sees the same thing, which is why you never write `map`.

The moment needs a name too. Reflex says *frame*, which suggests a screen refresh and a loop that produces them. The synchronous languages say *instant* or *tick*, which suggest a clock. Prism says *occurrence*, because the word claims only that something occurred, and that is the whole ontology: there is no moment without an event, and an occurrence exists because the outside world delivered something. An occurrence has a before and an after and no inside. Within it every dynamic reads as it stood before; at its end every hold commits at once; nothing anywhere sees a half-finished one.

You know this already under another name. Nobody thinks ACID is an implementation detail of a database. It is what the database promises, and you would be angry at one that let a reader see half a write. The occurrence is a transaction in that sense, a promise to the program. The synchronous languages wrote the same promise for hardware controllers decades ago: "All writers run before any readers do." And Prism gets it for nothing, because there is no assignment. Every write is a hold's commit, every commit can wait for the end of the occurrence, and an occurrence that goes wrong can be dropped whole. You get transactions for free because you gave up the thing transactions protect you from.

A transaction is semantics here, not technique, because the program can tell whether it holds. `$count + 1 on !click` reads the count before the click, in every implementation, every time; a test that checks it is a test of the language, and the conformance suite has one.

## Why it has to be a language

A library adds sentences to a language. A policy removes them. So the question for any policy is how much a library can remove, and the answer is: whatever its host lets it name.

That is more than nothing, and the libraries have earned the credit. Reflex removes a great deal. `MonadHold` "is required to create any stateful computations with Reflex", and sampling, requesting and building networks each sit behind their own class, so a function whose signature leaves a class out cannot do what that class does. A signature without an IO class cannot touch the world. <!-- from memory: the IO class Reflex code uses for this is MonadIO, with liftIO; the notes do not name it --> That is a library subtracting from its host, and doing it well. React removes things too, by another route. The Rules of Hooks ("Don't call Hooks inside loops, conditions, nested functions, or `try`/`catch`/`finally` blocks") are a policy about state identity, and the docs can say the rule ensures "all stateful logic in a component is clearly visible from its source code". JavaScript cannot express the rule, so a linter enforces it. And the docs describe the linter's off switch in their own words: "By suppressing the linter, you 'lie' to React about the values your Effect depends on." A subtraction you can turn off is a convention.

Reflex's IO hatch is the honest kind. `liftIO` shows in the signature, the way `unsafe` shows in Rust: a labelled place where the rule stops, and you can grep for it. The harder evidence is what Reflex keeps that is not a hatch. `switchPromptlyDyn`, `tagPromptlyDyn`, `coincidence` and `sample` need no IO and no special capability. They live in the same classes `hold` needs, so a program that uses them has the same signature as one that does not. The library cannot label them, because its own primitives build them. It cannot remove them, because the host can say them. `switchPromptly` was deprecated, and the deprecation is a rename: "Use 'switchHoldPromptly' instead."

What the library can do is warn, and it warns at length, in its own documentation. On `switchHold`: "Use this rather than 'switchHoldPromptly' and 'switchHoldPromptOnly' unless you are absolutely sure you need to act on the new event in the coincidental case", because the lax version avoids "many potential cyclic dependency / metastability failures". On `switchPromptlyDyn`: "The timing requirements that switching before imposes are likely to bring down your app unless you are very careful." On `coincidence`: "You maybe looking for '@switchHold@ @never@' instead." On `tagPromptlyDyn`: "When creating cyclic data flows, generally `tag (current d) e` is preferred." And on `networkHold`, a few files away, the advice that its result "would typically be flattened (via 'switchPromptlyDyn')", the function whose own page says it will bring down your app. Nobody on that project lacks discipline. The hole is in the host, and the warnings are the policy's border, drawn by people who know exactly where it is. A library cannot make the forbidden thing stop being a sentence. `unsafe` marks the place where Rust's rule stops; Reflex's prompt variants are unmarked places inside its rule.

"Remove the dangerous feature" proves too much, of course. Every removal closes some hole. A total language removes general recursion and gets termination; Esterel removes building structure at run time and gets a compiler that refuses contradictory programs; Elm removes local state and gets one model. Something has to say which removals earn their place. Here is the test this book uses: **a refusal earns its place when it closes a documented bug class, and the systems that keep the feature warn against it in their own docs.** The first half says the hole is real. The second says the people who kept the door open agree it is a hole.

Three systems meet the second half in writing. Reflex's warnings are above. React's are about dependencies and outside state: "**Notice that you can't 'choose' the dependencies of your Effect.**", the lie sentence, and for state React cannot see, `useSyncExternalStore`, which is "intended to be used by libraries, not application code", with "When possible, we recommend using built-in React state" on its own page. The TC39 Signals proposal meets it by choice. It defines its design goal this way: "Computation is glitch-free, meaning no unnecessary calculations are ever performed", a weaker property than never observing an inconsistent value, and it says plainly that "The Signal API does not include any built-in function like `effect`", leaving scheduling to the `Signal.subtle` namespace, "better to leave for framework authors". That is a committee choosing the property everyone can ship and saying so in the open. It is also a standard marking the half where the problem lives as advanced and handing it to someone else.

The test refuses two removals as well as endorsing these. Nobody whose system builds structure at run time warns you off building structure at run time, so Esterel's fixed topology fails the second half, and Prism keeps runtime construction and pays for it with heights, a cycle check and a protocol at the boundary. Nobody whose system has local state warns you off local state (React recommends `useState`; Reflex attaches no warning to `hold`), so Elm's removal fails too, and Prism keeps it.

Haskell is the exception that shows the rule working. Haskell already subtracted effects from everything, and lets a library add them back, selectively, by monad. In practice it is a language for building separate silos with different models of mutation, `IO` and `with` and the reactive monads, each with its own rules. That is why the best FRP lives in Haskell. It is also why FRP cannot leave Haskell by the same door: in JavaScript the library adds a `Signal` type and cannot subtract `let`.

Embedding has a cost of its own. Learning a paradigm is far more work than learning one more language within it; you have watched people who know four object-oriented languages complain that Rust has no inheritance, because habits travel with the programmer, not with the language. A paradigm embedded in a familiar host is a paradigm whose alternatives are always in reach, and a learner uses them, builds a worse version of a framework they already knew, and blames the paradigm. The paradigm cost is paid either way. Embedding adds unlearning the host, every day, at every keystroke where the host is available.

None of this is about villains. The second checkbox observer was written by someone competent at six in the evening, because the callback was there. Cheating, in the sense of writing state outside the policy, is not a moral failure to catch in review. In a language with one policy, it is a sentence that cannot be written. It should be easy to build an FRP program in such a language, and impossible to build any other sort.

## What each paradigm fixed, and where the problem came back

Every paradigm for interactive programs answers the same question, who may change state, from where and in what order, and each one fixed a real part of it. Here is each as a picture, with the constraint it adds and the place its own documentation shows the problem coming back.

### Callbacks

A bag of variables and a set of functions that run when something happens. Each function may read and write anything. The constraint added is none, and that is their strength: callbacks are the substrate every other paradigm compiles to, and the thing you reach for at two in the morning because nothing stands in the way. Nothing stands in the way of the second checkbox observer either. Two handlers in one moment run in an order fixed by the program's history, and the question "what can happen to this" has no answer anywhere.

### Objects

The variables are gathered into objects, and only an object's own methods write its fields. That is a real constraint and it bought a lot: the writers of a piece of state at least live in the same file as the state. The problem comes back through the method call. A method reacting to a click calls a method on another object, which calls a third, which calls back into the first before the first has finished. The pattern literature's advice, "Make sure Subject is self-consistent before Notification", exists because the setter in the checkbox afternoon is the default. Encapsulation controls *who* writes. It says nothing about *when*, and in an interactive program when is the whole difficulty.

Objects got two things right, and Prism keeps both: interactive things have identity, and they have lifetimes. A widget in Prism is the thing an object was trying to be, minus the methods. What will not be added is inheritance, or anything else whose purpose is letting one object reach into another's state.

### React

The screen is a pure function of state, re-run when state changes and diffed. That fixes the render side completely. Nothing can draw stale state, because drawing is not a mutation, and every framework since has copied the idea for good reason.

The problem comes back on the state side, and React's own documentation marks each place. State lives in hooks whose identity is their position in the call sequence: "React relies on the order in which Hooks are called." Updates are callbacks calling setters, and whether two of them land in one render depends on whether they share a handler, which is the `+3` that comes out 1. Derived work runs in effects whose dependencies you list by hand, and the docs say plainly what happens when the list is wrong. Effects run after the commit, and on timing the docs use the words they have to: React "will generally let the browser paint the updated screen first", and when a click caused the render, React "may run your Effect before the browser paints".

The later additions each name a problem in the docs' own words. `useSyncExternalStore` exists for state that changes outside React, and its consistency rule is a retry loop: "React will call `getSnapshot` a second time just before applying changes to the DOM. If it returns a different value than when it was called originally, React will restart the update from scratch, this time applying it as a blocking update". That is a transaction's rollback built by hand, inside the framework, giving up concurrency to get consistency back. Suspense handles a value that is not there yet by interrupting the render: "React does not preserve any state for renders that got suspended before they were able to mount for the first time. When the component has loaded, React will retry rendering the suspended tree from scratch." <!-- from memory: Suspense-enabled code signals "not yet" by throwing a promise from render; react.dev does not document the mechanism --> The React Compiler exists because "manual memoization is tedious, easy to get wrong, and adds extra code to maintain", and it aims at what it calls "fine-grained reactivity". Strict Mode will "run one extra development-only setup+cleanup cycle" for every effect, because the framework cannot assume your mount effect is safe to run twice.

You can read each of these as growth: a measured problem, a shipped feature, documented limits. They are. They are also growth that keeps arriving at the same three places: consistency within a moment, a value that is not there yet, and values derived from other values. A pattern that persistent is information about the model underneath.

### Signals

Every piece of state is a cell that records who read it, and a change re-runs exactly the readers. Solid, Svelte, Vue, Angular and Preact all have them, the TC39 proposal lists those frameworks and more among its design inputs, and the convergence is good news: the industry found the behavior half of FRP on its own. A signal's derived values are their definitions, which is the dynamics half of everything this book says.

The problem comes back in two places. The first: there are no events as values. Solid's docs describe signals as "like mutable variables that can point to a value now and another in the future", and what happened is whichever callback wrote the cell. Two writes in one moment need help: "it is important that the subscriber is updated after both signals have been updated", and `batch` is how you arrange it. The second: consistency is each implementation's own work. The proposal promises that "A computed Signal always observes the Signal graph in a consistent state", defines glitch-free as doing no unnecessary computation, leaves effects out, and has a long thread arguing whether evaluation should be eager or lazy. That is a sound standard fixing what everyone agrees on, and the part everyone agrees on is the half without events.

### Rx and streams

Everything that happens is a stream, and a program is a pipeline over streams. Rx has events as values and a rich set of operators over them, which is the other half of FRP, found from the other side. The problem comes back because there are no behaviors as a law. The current state is a stream you remember yourself, through a `BehaviorSubject` that has "a notion of 'the current value'", built by hand. Each subscriber to a plain observable "owns an independent execution of the Observable", so two observers of one source can disagree about what happened. And "Observables are able to deliver values either synchronously or asynchronously", so time is whatever the subscription order makes it.

At scale the missing half gets rediscovered in prose. Kafka's documentation has a section on the duality of streams and tables: "A stream is thus a table in disguise", and "A table is thus a stream in disguise". A table is a hold, `then`, and its changelog is the hold's change event, `!Updated`. Kafka had to write the documentation because its types did not say it.

### Elm

The friendly comparison. One model, one update function, and a runtime that owns every effect: "The Elm program produces HTML to show on screen, and then the computer sends back messages of what is going on." Elm is the existence proof that a mutation policy at the language level, with no escape, works for real interactive programs. Everything in the previous section agrees with Elm.

The policies differ, and the difference is who owns state. Elm has one model and no local state, and its guide is direct about it: "Actively trying to make components is a recipe for disaster in Elm", because "components = local state + methods" and "local state + methods = objects", and "There are no objects in Elm!" The advice is "**Just write the helper functions you need.**" A widget with its own state threads that state through its parent's model and update by hand.

Prism keeps widgets with lifetimes and holds inside them. A row in a list has its own expanded flag, defined in the row, listing the events that move it, and torn down with the row; "one way to change anything, written next to the thing" holds inside the widget, and the parent never hears of it. And Prism keeps something Elm's guide never offers: every widget is testable as it stands. Wrap a widget in an interceptor and the interceptor stands where the World stands. It answers the widget's requests and supplies its state, a file's contents as whatever text the test wants, and the widget cannot tell. No mocking framework, no seam added for the test, no change to the widget. A test of a widget is the widget under a different World, clicked the way a user clicks it.

### Haskell

A language in which a library may define its own mutation policy through its monads, so FRP can be built as one, and the best FRP has been. The seams show anyway. You keep three views of one thing apart by naming them `dFoo`, `eFoo` and `deFoo`, Hungarian notation for time, because the monads are recursive and shadowing does not work consistently. You order a transformer stack, and the order is a design decision. You write `lift . lift`. And the door to `IO` stays open: Reflex's `Adjustable` documentation warns that "there is some issue here with persistent side-effects: obviously, IO". Prism's sigils replace the naming convention outright: `$x`, `!x` and `@x` are one name with three checked spellings. The community's unofficial motto, which Simon Peyton Jones gave a talk about under the title "(Failing to) avoid success at all costs", parses two ways, and the community prefers "avoid (success at all costs)"; whichever parse you take, FRP's success cannot depend on it.

### The synchronous languages

Esterel, Lustre and their family have Prism's law, earlier, with a clock. Time is divided into discrete ticks, "Computations considered to take no time", "Each signal is only present or absent in a cycle, never both", "All writers run before any readers do". They refuse the static form of a prompt cycle: `present A else emit A end`, "if A is absent, make A present", is "an erroneous program", and the compiler will not compile it. These are allies. They are also designed for controllers whose structure is fixed when the program is compiled, and nobody who builds structure while running warns against it. Prism keeps the law and adds runtime construction: a GUI is built and torn down while it runs, and so is a server's set of connections.

## The goals

Every goal below is a promise that some version of that afternoon cannot happen.

- **There is one way to change anything, and it is written next to the thing.** A hold's definition lists every event that can move it. Read `$checked <- False then !toggle | !reset` and you have read everything that will ever happen to `$checked`. The second checkbox observer has nowhere to live.
- **Two things in one moment are simultaneous, and the program says what that means.** Reset and the server's message in one occurrence? The merge says which wins: `|` takes the left. Not registration order, not luck, and if you want the other answer you swap the operands and the program says so.
- **You can see what moves.** `$count + !delta` tells you that one name is live and one happens, and so does the compiler, which lifts the addition for you. No `map`, no `zipWith`, no dependency array to drift from the code.
- **"Not yet" is a type, not a feeling.** The file that has not loaded is `Unavailable`, a case of its type, and the compiler makes you say what to draw then. Nothing is `null` while you wait, and no render is thrown away and retried.
- **Nothing is chosen for the implementer's comfort.** A rule that is here because it was easier to build is a bug in the specification. The reverse test: anything a convenience does, you can do by hand with the same primitives, and the hand version is the real one. Every refusal in the next section names the bug it closes before the machinery it saves, and the order of those two clauses is the policy.
- **The language is enough.** Layout, focus, text editing, sandboxes, mocks, caches, even a World that hosts other programs: Prism code over a handful of primitives. What is native is what touches hardware.
- **The spec is the thing.** Two implementations read it, one in Rust and one in Haskell over Reflex, and a differential test runs them side by side. When they disagree, the sentence gets fixed. A second implementation proves that the specification can be read twice to the same program, which is a claim about the document. It is not offered as evidence that the language is good; the programs are.

## The refusals are the language

A language is defined by the sentences you can write in it, and also by the ones you cannot. A library can only add to the first list; a language owns the second. So the list below is not a list of missing features. It is the part that makes Prism a language in the sense the previous sections need. Each refusal closes a door you have walked through into a bug, and the same refusal lets the implementation leave out a mechanism: a scheduler, a collector, a lock, a retry loop. The bug comes first in every entry, and the saving second.

Two refusals get the long treatment, because they are the two a reader who knows FRP will fight hardest, and because they meet both halves of the test in writing.

**No sampling.** You cannot ask a dynamic what it is right now. There is no `sample`, no `.current`, no `.value`. You read a dynamic at an event through a tag, `$count on !click`, and get its value before the occurrence; or you build on it with `dyn`, which rebuilds a block when the dynamic changes. This one feels like a missing feature until you remember the checkbox: every "read it right now" in that story was a read of something half-updated. With no sampling there is no half to read, and the implementation may propagate in any order that respects dependencies, because no program can tell the orders apart.

What you lose, said plainly: you cannot write the line that peeks. What replaces it covers every use the peek had. A widget that needs to act as soon as it exists uses `@started`, an event that fires once, in the occurrence the widget is built, and reads what it needs through a tag. Reflex has `PostBuild` for the same job, documented as existing "to allow 'Behavior's and 'Dynamic's to be safely sampled, regardless of where they were created", and it fires after the build. Prism has no safe moment to wait for, because it has no unsafe one.

**No prompt switching.** When `!Switch` is handed a new inner event, it listens to it from the next occurrence, not this one. Here is the program where that matters: a button whose label counts its own clicks.

```prism
@main:
    let $label = "Clicked {$count} times"
    !!clicks <- dyn $label:
        @Element Ui.Button(label=_)
    let !click = !Switch !!clicks
    $count <- 0 then $count + 1 on !click
    @ $count
```

`dyn $label:` builds a button with the current label and rebuilds it whenever the label changes; each build is a new slot with a new path, because an element's fields are fixed for the slot's life. The `dyn` fires `!!clicks` in every build occurrence with the new slot's click event, and `!Switch` turns that event of events into the one `!click` the hold listens to. So every click relabels the button, and every click lands in the occurrence that replaces the button it was addressed to.

Prism has one answer for that occurrence. The World addresses the click to the button's path; routing looks the path up in the element list as the occurrence opened and finds the old button, the one the user saw. The old widget keeps propagating until the occurrence commits, so its click reaches `!Switch`, which still listens to the old generation. `$count + 1 on !click` reads the count before the click and fires; the label changes; the `dyn` builds the new button; at commit `!Switch` adopts the new button's click and the old generation is dropped. The program is a cycle (count, label, `dyn`, switch, click, count), and the cycle is legal because it passes through two delayed edges: the tag on `$count`, and `!Switch`'s choice of which inner event to hear.

Reflex offers a choice at the switch, and its docs describe both options exactly. `switchDyn`: "Only the old event is considered the moment a new one is switched in", which is Prism's answer. `switchPromptlyDyn`: "Switching occurs __before__ the inner 'Event' fires". Take the prompt one, and whether the click gets through depends on which button the switch hears; that depends on the new button; the new button depends on the label; the label depends on the count; and the count depends on whether the click got through. A loop inside one frame, the "cyclic dependency / metastability" failure the `switchHold` page names, waiting one word away at every switch in the program. <!-- from memory: Reflex's Spider runtime typically reports this at run time as a causality loop; the notes have no source for the symptom --> And `networkHold`, the ordinary way to rebuild part of a Reflex program, suggests in its own documentation flattening its result "via 'switchPromptlyDyn'".

Reflex code reaches for the prompt forms anyway, because sometimes the coincidental case looks like the case you need. <!-- from memory: that the prompt forms are the more commonly used ones is training, not a verified source --> Prism's answer to each of those cases is design, not apology. A freshly built button cannot be clicked in the occurrence it was born, because the World addresses only paths it has seen, so no click is lost. A child that needs to act at its own birth uses `@started` inside itself, where no switch stands in the way. A parent that needs to know a rebuild happened has the `dyn`'s own event, which fires in every build occurrence including the first. The one remaining case is a new child reacting to the very input that caused it to be built, and that is Esterel's refused program in another notation: it asks for A to be present because A was absent. Prism refuses it by making `dyn` a prompt edge and `!Switch` a delayed one.

What the implementation gets: every node's height is fixed by its sources, checking for a cycle is one comparison, and propagation never re-enters itself. Spider, Reflex's runtime, has special machinery for resetting coincidences in the middle of a frame; a runtime without `coincidence` needs none of it. If you have ever debugged a `coincidence`, you know which side of that trade you want.

The rest go faster.

**No assignment.** You cannot write `count = count + 1`, anywhere. The only state is a hold, and a hold moves only when its event fires. This is the refusal most people flinch at, and it pays for everything else: transactions for free, as the occurrence section showed. It costs something too. A field written by six things has a merge of six events in its definition, and the events come from elsewhere, so "next to the thing" means a list of names, some pointing away. The list is complete and the compiler checked it. The alternative is six setter calls in six files, and the answer to "what can change this" is the result of a search.

**No events from nothing.** No `emit`, no `setTimeout`, no event that fires because time passed. An event fires because an input the World delivered led to it, or not at all. The World owns time and the runtime has one entry point, so the same inputs give the same occurrences every time, and a transcript of a run is a test.

**No effects, no callbacks, no handlers.** A program cannot write a byte, read a clock, or hand anyone a function to call later. It can put an element in a list. The World never holds a runtime object, so the runtime can be single-threaded, swapped for another, or driven by a test World with no screen that tells you exactly what the program would have drawn. `useSyncExternalStore`'s retry loop is what this saves: consistency with state the framework cannot see, recovered after the fact.

**No widget values.** You cannot keep a reference to a button. A widget is a position in a tree, not a thing you can put in a map. That is the detached row: if a widget is a value, someone keeps it, and the parent's decision to remove it means nothing. Here ownership is a tree, teardown is recursive, and there is no collector.

**No request on creation.** Creating an element never makes anything happen by itself. A file is read because a request event fired and the program fired it. Strict Mode's extra setup-and-cleanup cycle exists because mount effects are not safe to repeat; here building is pure, a rebuild costs exactly what it asks for, and in the occurrence a widget is built nothing can depend on an effect having happened, because none has.

**No placeholders.** No `Pending`, no `undefined`, no `null`, no render thrown away to wait. The file you have not read is `Unavailable`, and the `match` that draws it has an arm for that case. You have written the `if (data)` guard a thousand times and forgotten it once; here forgetting it is a compile error.

**No custom monads.** One block type per functor, and the set is fixed. You never order a transformer stack, and the compiler never infers whether something is an event: the name says.

**No inheritance, no reaching into another widget.** A widget's results are what it returns. No protected fields, no `parent.setState`, no event bus. The promise that a hold lists everything that can change it is only true because there is no back door.

**No exceptions as control flow.** If the program is meant to handle a failure, the failure is a value. If it cannot be a value (a cycle, an overflow), the occurrence aborts and nothing half-done survives. The transaction is the error mechanism, and it was already there.

**No ambient anything.** No globals, no implicit context, no environment variable read from the middle of a function. A widget has its arguments and its elements, so a sandbox works: a third-party widget has no channel that the interceptor around it does not see.

**No threads, no scheduling, no priorities, no phases.** The language has no idea of "later". Parallelism belongs to the implementation and can arrive without changing a program.

**No clock.** There is no `now()`. Time is a state element, like the contents of a file, and a test sets it. A debounce takes more lines than `setTimeout` and an element with a response, and it is a unit test. Anyone who has tried to test a debounce knows what that is worth.

**No dynamic fields in elements.** An element is born with its fields and keeps them. A label whose text changes is, to the World, a new label at the same spot. The boundary never sees a sigil, and a World diffs lists of plain data instead of subscribing to anything.

**No introspection.** A test cannot reach into a widget and poke `$count`. It clicks the button and reads the label, so a test that passes under the Test World passes under any World.

The pattern repeats: a feature you would have sworn was essential is removed, the removal closes a hole you have fallen into, and the same removal lets the implementation leave out the machinery the feature needed. A language with these refusals and these two types has no freedom left where it matters. What remains is spelling.

## The boundary, and what the rest of the book has to show

The language has no effects. The World has all of them. A World is written in the host language, owns the event loop, and opens every occurrence. The only thing that crosses between it and the program is plain data. Going out: *elements*, records like `Ui.Button(label="Increment")` or `Fs.Contents(path="notes.txt")`, which a widget puts in its list to say what it wants. Coming in: responses addressed to an element's path, and state identified by an element's value, so two widgets that ask for the same file share one answer.

One rule governs time at the boundary, and it is the rule that keeps every ordering bug of the first section out:

**Elements never fire in the occurrence they were created. Firing within the same occurrence implies they could have been pure functions. Instead, elements with request-like semantics take an input event and fire their output event in a different occurrence.**

Nothing can depend on a side effect having happened during the moment it was requested. State elements are the complement: a state element has a value in the occurrence it is created, supplied by whoever handles it, because a dynamic without a value is not a dynamic. And the boundary is a diode. Inputs cause changes to the element list; changes to the element list never cause inputs. A resize is an input from the window; a text measurement is state. Neither is a slot appearing and thereby firing something.

You have seen this before, and you should say so. Elm's commands go out and its messages come in. Reflex's `Requester` lets an action "trigger requests ... based on 'Event's, and receive responses ... in return". Every actor system works this way. A command queue alone would not be a reason for a language.

The queue is not the point. The point is what comes with it. No sampling and no request on creation make building pure. State elements valued in their first occurrence mean a dynamic is never without a value; `Requester`'s signature, request out on an event and response back on an event, cannot give that, which is a reading of its interface rather than anything its docs say. And `intercept` nests the boundary. A widget can stand between its children and the World, answer some of what they ask for itself, and pass the rest on. A sandbox, a mock filesystem, a cache and a GUI toolkit are all interceptors written in Prism, and a World is the outermost one. Here is a mock, the one the Elm comparison promised:

```prism
@WithNotes: fn @a -> @a = \|@child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ supply
    !down <- @Forward $slots
    let supply = \|e| match e:
        Fs.Contents(..): @ $ Known "hello"
        ignore: @Element e
    @ result
```

`intercept` builds the child and captures its element list as `$slots`. `@Forward`, a five-line widget from the worked examples, sends every slot outward unchanged and brings the responses back down. `supply` answers the child's state: any file it asks for contains `"hello"`, and every other state element goes to the real World. The child is whatever widget you wrote, unchanged, and it cannot tell. A library that types its output list gets the command queue. It does not get a child that cannot tell whether its World is real.

Now the part the steelman is right about. Everything above is about the moment and the boundary, and the moment is the part every shipping system already handles inside a task. The hard parts of a GUI toolkit are layout, text and asynchronous IO, and the competition's scars are there. Here is the design for each, and which part is not yet earned.

Layout and text editing are library code. The World does no geometry. A GUI World speaks five tags: `Gui.Window`, which delivers pointer, key and resize events; `Gui.Scene`, the draw list for a window as plain `Draw.*` values; `Gui.Focusable`, a named region that receives keys; `Gui.Metrics`, text measurement as a state element; and `Gui.Frames`, a tick per frame for whoever asks. Buttons, labels and text inputs are the library's `Ui.*` tags, and the library's interceptors turn them into scenes and hit-test pointer events against the layout they hold. Measurement is state, so it has its value in the occurrence it is asked for: measuring and laying out happen in one occurrence, and the scene that leaves it is consistent. No column flickers while a round trip completes. A text input's text and cursor are a hold in the input widget, outside any `dyn`; keys arrive through a `Gui.Focusable` the library names; the World sees draw commands and never a cursor, so a re-render cannot lose it. The elements-and-interception chapter builds this library, and the use-cases chapter builds an editor and a compositor on it.

Asynchronous IO is the World's. Responses arrive in occurrences the World opens, and the order between two requests in flight is the World's. That does not move React's call-stack problem to the boundary. The World's sequence is the input; a transcript records it; and the program's text says what it does under every order, so the same inputs give the same occurrences. Nondeterminism of arrival is not removed, because nothing can remove it. Nondeterminism of response to arrival is. Cancellation is the World's obligation when a slot is torn down; a stream is an element that responds without being asked; a socket is one tag with requests, responses and state. The Worlds chapter has to make each of these work.

The unproven part, then: whether a constraint solver is pleasant to write as holds and events; whether an input method's composition survives a text input written in the library; whether a World on an asynchronous runtime keeps the transcript exact under load. The rest of the book is for these. This chapter claims the moment and the boundary, and claims that the toolkit is library code over five tags. It does not claim the toolkit is done.

The claim has edges in the other direction too. Prism is the right language for a program that is state, things happening, and responses: interactive interfaces, stateful network services, incremental computation like build systems and spreadsheets, simulations and games, controllers, shells and window managers. It is the wrong language for programs that are computations: numerics, compilers, batch data work, an algorithm naturally written as a loop with a cursor, a request handler that keeps nothing between requests. Prism programs call out to that kind of code; they do not replace it. The first list is large, the tools for it are bad in a way that is structural, and a language with time in its types fixes the structure. The second list is where the claim stops.

## Two programs

Here is the counter from the afternoon, with every declaration it uses.

```prism
type Element = choice ..
type Click = click

tag Ui.Button of Element:
    label: Text
    responds: Click

tag Ui.Label of Element:
    text: Text

@main:
    !increment <- @Element Ui.Button(label="Increment")
    !decrement <- @Element Ui.Button(label="Decrement")
    let !delta = -1 on !decrement | +1 on !increment
    $count <- 0 then $count + !delta
    @Element Ui.Label(text="Count: {$count}")
```

`Ui.Button` and `Ui.Label` are tags of the GUI library: a button has a label and responds with a click; a label has a text and does nothing. `@Element` makes a slot for an element and gives back what it responds with, so `!increment` is the Increment button's clicks. `on` takes a value at an event, so `-1 on !decrement` fires `-1` whenever Decrement is clicked. `|` merges two events, the left one winning when both fire in one occurrence. `then` holds: `$count` starts at `0` and becomes `$count + !delta` whenever `!delta` fires, with `$count` on the right read through a tag, the count before the click. Bindings in a block are recursive, so `$count` can appear on both sides. The label's text is a dynamic, and a dynamic where a plain field goes lifts the element into a `dyn`: a fresh label slot each time the count changes. The block is a description. Nothing happens until a World builds it.

Ask it the questions from six in the evening. What can change `$count`? Read one line: `!delta`, and nothing else. If both buttons fire in one occurrence, which wins? Read another: `|` takes the left, so the decrement. Is there a setter somewhere else, in another file, registered by next week's feature? There cannot be. The language has no sentence for it.

Here is the World's side. In the build occurrence it gets a list:

```
0:      Ui.Button(label="Increment")
1:      Ui.Button(label="Decrement")
2/#0/0: Ui.Label(text="Count: 0")
```

The user clicks Increment. The World opens an occurrence with a click addressed to path `0`. `!increment` fires, `!delta` fires with `1`, `$count + !delta` fires with `0 + 1`, the label's text changes promptly, the label's `dyn` builds generation `#1`, and everything commits at once. The World reads the new list, sees `2/#1/0: Ui.Label(text="Count: 1")` where generation `#0` was, and redraws. Nothing told it to. Under the Test World a script sends the same click and expects the same label; under a GUI World the library's interceptors turn these tags into a scene. The program is the same in both. Events cause rerenders; rerenders do not cause events.

And here is a file viewer, with "not yet" as a type and exit as a request.

```prism
type Contents = choice:
    Unavailable
    Known Text

type Code = Code Int

tag Fs.Contents of Element:
    path: Text
    state: Contents

tag Process.Exit of Element:
    requests: !Code

let Show: fn Contents -> Text = \|c| match c:
    Unavailable: "(nothing to show yet)"
    Known text: text

@show_file: fn Text -> @unit = \|path| @widget:
    $contents <- @Element Fs.Contents(path=path)
    @Element Ui.Label(text=(Show $contents))
    @ unit

@main:
    @show_file "notes.txt"
    !quit <- @Element Ui.Button(label="Quit")
    @Element Process.Exit(requests=(Code 0 on !quit))
```

`Fs.Contents` is a state element: the file's contents as a value over time. `$contents` has a value in the occurrence the slot is created, the World's answer as that occurrence opened: the text, if the World could read it then, or `Unavailable` if it could not or has not yet. When the file changes on disk, the World pushes the new contents in a fresh occurrence, `$contents` changes, and the label follows. The program has no code for that, because a dynamic that follows the file is what it asked for.

`Show` is a plain function, and the `match` in it has an arm for `Unavailable` because the type has that case and the compiler will not accept a `match` that leaves it out. There is no string standing in for a value, no `Pending`, no render thrown away and retried. A test asserts on `Unavailable` as a value, or sets the file's state to `Known "hello"` before the build and reads the label.

The Quit button's clicks become `Code 0` requests on a `Process.Exit` element. The element sitting in the list does nothing; closing a window does nothing; dropping a widget does nothing. The run ends after the turn in which the program fires the request, because the program said so, in the occurrence after the click arrived at a button that existed when the occurrence opened.

Ask this program the six o'clock questions too. What can change the label? The file, through one state element, and nothing else. What does the user see before the file is read? Whatever the `Unavailable` arm says, chosen by you, checked by the compiler. What ends the program? One request, fired by one event, in one line.

Two short programs, and the questions that took an afternoon in the first section each have a one-line answer. Prism is a language whose only subject is time.
