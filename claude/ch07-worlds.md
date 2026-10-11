# Chapter 7 — Worlds

A Prism program cannot write a byte, read a clock, or hand anyone a function to call later, so something outside it must do all of that, and the whole design of the boundary is in how little that something has to know. A World is the outermost interceptor: written in the host language, the owner of the main loop, the only thing that starts an occurrence, and the only thing with effects. It drives the runtime through one synchronous call per occurrence, sending plain data in and reading plain data out, and nothing else ever crosses: no callbacks, no runtime objects, no threads, no second set of rules for "later." This chapter is the run call in full; the snapshot discipline that makes state elements honest and why every system that delivers state to subscribers ends up rediscovering it; turns and `Defer`; the obligations a World takes on and who each one protects; the world core that every World shares; the general components for files, time, processes, networks and errors, each with its tag declaration and an example; the CLI World and the GUI Worlds, including the one where the World is nothing but a socket and the display protocol is Prism; async runtimes, blocking calls, platforms, servers, Worlds written in Prism that host other programs, and parallelism. Throughout, the negative space: what a World may not do, and what each refusal buys.

## The World is the outermost interceptor, and it is small

You know the failure mode. The framework hands you a context object on startup. You stash it in a global, because the socket handler needs it and the socket handler has no other way to get it. Three months later a timer callback reads the context after it was torn down; a year later a test cannot run without a browser, because the context is the browser; and when you finally try to move the application onto a server you discover that the context is also the render loop, the clock and the filesystem, and that forty callbacks hold it. The program and its World grew into each other, and now neither can be replaced.

Prism's answer is that the program and the World never touch. A program is pure construction and pure propagation: it builds a tree of widgets, some of which put *elements* in a list, plain records like `Ui.Button(label="Increment")` or `Fs.File(path=path)`. The World reads that list. What the World sends back is also plain data: responses addressed to elements by path, and the state that state elements asked for. There is no object in the program that refers to the World and no object in the World that refers to the program. The program can run under a World that has no screen, no clock and no filesystem, and be told exactly what it would have done; and the World can be rewritten without the program noticing, which is the same promise seen from the other side.

From the program's side a World is an interceptor. `intercept` stands between a child and everything outside it, supplying the child's responses by path and its state by a supplier function, and making elements of its own upward or not. A World is the interceptor with nothing above it. Its responses come from sockets and keyboards rather than from a Prism event; its supplier is a synchronous host-language function rather than a widget; and it has no "upward," because it is where upward ends. A child cannot tell whether its parent is a sandbox written in Prism or the real World, and that is the test the whole chapter is held to: every policy an interceptor could have, a World could have, and the other way round.

A World is small because the language can do nothing to it. It never registers a callback, so it never has to protect itself from re-entry. It never holds a runtime object, so it never has to worry about a lifetime. It never has to decide what a widget means, because a widget is the program's business and only elements reach the World. Everything it must get right is a short list of obligations, the same for every World, written once in a library the chapter calls the world core. What differs between Worlds is which tags they accept, and that is a list of components.

The goals this serves are the two that pull hardest against each other. The language has no effects, so IO must be impossible from inside it; and the language must be enough, so layout, focus, text editing, sandboxes, mocks and even a World that hosts other programs must be writable in Prism. A fat World would satisfy the first by taking everything; a leaky boundary would satisfy the second by letting the program reach out. The thin World satisfies both: it does only what touches hardware, and everything above the hardware is Prism, under the same laws as the rest of the program.

What a World is not: a renderer, a widget toolkit, a scheduler, a thread pool. A GUI World may contain a renderer, and a server World may contain a thread pool, but those are components inside it, and the language sees only tags. In particular a World never reads `Gui.*` elements in order to lay them out or decide focus. It reads a scene and delivers pointer events and keys to the window; which field is focused, what is under the pointer, where the caret is and what a tab key does are holds in a Prism library.

## The run call

The World drives the runtime through one synchronous call per occurrence. Here is the whole interface, in Rust-flavoured pseudocode. Everything in it is plain data except the oracle, which is a function from plain data to plain data, called only during the call and never retained.

```rust
/// Plain data: an interned tag plus field values, with structural
/// equality and ordering. This is what state identity compares.
pub struct ElementValue { tag: TagId, fields: Vec<Value> }

/// A slot's position in the widget tree, assigned at build and never
/// reused. Opaque to Worlds; ordered so that lists are in tree order.
pub struct Path(Vec<Segment>);
pub enum Segment { Bind(u32), Key(Value), Gen(u64) }

pub struct Inputs<'a> {
    /// Responses addressed to slots, by path. At most one per path.
    pub responses: Vec<(Path, Value)>,
    /// New values for state elements, by element value. At most one per value.
    pub pushes: Vec<(ElementValue, Value)>,
    /// The World's snapshot as this occurrence opens, as a function.
    /// Must answer synchronously for every state tag the World accepts.
    pub oracle: &'a dyn Fn(&ElementValue) -> Value,
}

pub struct Outcome {
    /// The element list after the occurrence, in tree order.
    pub list: Vec<(Path, ElementValue)>,
    /// For each slot whose tag declares `requests`, what it fired this
    /// occurrence, in firing order. Includes slots torn down this occurrence.
    pub requests: Vec<(Path, Vec<Value>)>,
    /// The program's result, for Worlds that read it (a test World does).
    pub result: Value,
}

pub enum RunError {
    UnknownPath(Path),                     // a response to a path not in the opened list
    WrongResponseType { path: Path, .. },  // a response the slot's tag does not declare
    UnsuppliedState(ElementValue, Span),   // a state tag the oracle cannot answer
    DuplicateResponse(Path),               // two responses to one path in one occurrence
    Cycle(Vec<Span>),                      // a prompt cycle found during propagation
    Panic { span: Span, message: String }, // overflow or a native bug, caught
}

impl Runtime {
    /// Run exactly one occurrence. Holds commit at its end. On `Err`,
    /// the graph is as it was before the call.
    pub fn run(&mut self, inputs: Inputs) -> Result<Outcome, RunError>;
}
```

That is the entire surface a World needs. The runtime's graph types exist and are public, because compiled widgets are built from them, but a World never names one. `Runtime` is `Send` and not `Sync`: it can be moved onto a thread and driven there by one owner, and never shared.

### What goes in

*Responses* are addressed by path. A slot with a `responds` field gets at most one response per occurrence; two to one path is an error, because a response event fires at most once per occurrence and the World would be asking the runtime to pick. If the World has two chunks of standard input waiting, they are two occurrences, and the program sees them in order. If it has a click and a tick waiting for different slots, they may be one occurrence, and the program sees them as simultaneous, which is a claim the World is entitled to make about things that were waiting together.

*Pushes* are addressed by element value, not path, because state is identified by value: every slot whose element equals `Fs.Contents(path="/etc/hostname")` observes one and the same dynamic, and a push for that value reaches all of them, including a slot created in the push's occurrence. One push per value per occurrence, for the same reason as responses.

*The oracle* is the World's snapshot as a function. When the runtime creates a slot for a state element during the occurrence, whether in the build occurrence or in a `dyn` rebuild halfway through propagation, it asks the oracle for that element's value and the slot's dynamic starts there. The oracle is a function rather than a map because the World cannot know in advance which values will be asked: a `dyn` over a text input creates `Fs.Contents(path=whatever the user typed)` and the World learns the path by being asked. The answer must come from the World's snapshot as the occurrence opened and nowhere else; the next section is about why.

### What comes out

*The list* is every slot in the program after the occurrence, with its path and its element, in tree order. It is a description of what the program wants from the world, complete each time, and the World reads it the way a compositor reads a scene graph: it reconciles reality to the description. A draw list is drawn. A `Net.Listener(port=8080)` in the list means a socket should be bound; the same element in the next list means it should still be bound; its absence means it should not be. Reading the list is idempotent, and that is what separates a description from a trigger. Nothing is *done because* an element appeared; the World makes the world match the list, and a list that is the same twice means nothing twice.

*The request firings* are the one-shot instructions: a write to standard output, a `Read` on a file, bytes on a socket, a `Code 0` on `Process.Exit`. Each is listed under its slot's path in the order it fired. The World performs them after the turn, in occurrence order and, within an occurrence, in list order. A firing from a slot that was torn down in the same occurrence is still reported, with its path; the World performs it if it can and discards any response, since there is no slot to address. The program asked, and the World does not second-guess that the asker is gone; what it cannot do is answer.

*The result* is the root widget's result, which most Worlds ignore (a CLI World runs `@main : @unit`) and a test World reads for `Returns` and `Fired`.

*The error* says the occurrence did not happen. Because every write during an occurrence is buffered to commit time, the error path is a rollback: the hold values, the switch targets, the nodes built during the occurrence are dropped, and the graph is as it was before the call. The inputs that opened the occurrence are lost, and the World decides what that means for it: a CLI World prints the error and exits nonzero, a GUI World might show it and keep going, a test World records it for an `Errors` step.

### What never crosses

Nothing in `Inputs` or `Outcome` is an event, a dynamic, a widget, a node, a closure over the graph, or a reference into the runtime. The oracle is the nearest thing to an exception and it is not one: it is a function the World owns, over the World's own data, that the runtime calls and forgets. The runtime never calls the World at any other time, never calls it after `run` returns, and never calls it from a thread the World did not call `run` on.

Suppose instead the runtime called the World back during propagation, "a response just fired, do your thing." The World's code would then run inside an occurrence, where holds have not committed and the element list is half built, and it would have to be told what it may and may not read. Every framework with a `componentDidUpdate` (from memory) has that table of rules, and every table has a footnote. Here there is nothing to put in the table because there is no call. Or suppose the World held a handle to the socket slot's response event and fired it directly. Then the World would have to know the event's lifetime, and when the `dyn` around the socket rebuilt, the World's handle would be stale, and the stale handle is exactly the leaked DOM subtree the language was built to make impossible. Addressing by path, against a list the World was given, means the World can only ever name what the program told it exists.

The payoff is the test World. A test is a script of inputs and expectations, and because the World interface is data, a script can be a World. The program you test under a transcript is the program you ship under a screen, with the same run call and the same laws, and a transcript of inputs replays a session exactly.

### Occurrence zero

The first call is occurrence zero, with no responses and no pushes: it builds `@main` and returns the initial list. That is how a World learns what the program wants before anything has happened. The oracle is live during occurrence zero, because the build creates state slots and they need values. A `Clock` slot built in occurrence zero gets the World's clock as of the start; `Process.Args` gets the arguments; `Fs.Contents` of a small readable file gets its contents, and of a file the World cannot give (missing, unreadable, over its size cap) gets `Unavailable` with the reason; the World, reading the list afterwards, sees that someone wants that file and starts watching it.

Here is the file viewer of chapter 2 as one run call, so that the pieces have names. The program fires a `Read` on `/etc/motd` from `@started` and watches `Fs.Contents` of whatever path is in a text input, initially `/etc/hostname`.

```
run(Inputs { responses: [], pushes: [], oracle: |e| snapshot.get(e).unwrap_or_else(|| first_answer(e)) })
→ Outcome {
    list: [
      (1,        Fs.File(path="/etc/motd")),
      (2/#0/0,   Ui.Label(text="(reading...)")),
      (3/0,      Gui.Focusable(name="input")),
      (3/1/#0/0, Ui.Label(text="/etc/hostname|")),
      (4/#0/0,   Fs.Contents(path="/etc/hostname")),
      (5/#0/0,   Ui.Label(text="box")),
    ],
    requests: [(1, [Read])],
    result: unit,
  }
```

The World now knows three things: there is a `Read` to perform on `/etc/motd`, addressed back to path `1` when it completes; someone is watching `/etc/hostname`, which `first_answer` read for the oracle (`Known "box"`, so the label already shows it) and which the World should now keep watching; and there is a screen to draw. It performs the read on a thread, sets the watch, draws, and waits. When the motd read completes, the next call has one response, `(1, Contents "Welcome")`. When someone renames the host, the call after has one push, `(Fs.Contents(path="/etc/hostname"), Known "crate")`, and no responses. The order in which they happen is the order of the calls, and the program's text says what each order means.

## Snapshots, the oracle and pushes

Picture a book builder joining an exchange feed after the open. It needs the book as it stands, and every change from then on, and it needs to know which of those changes the book it fetched already includes. Binance's documentation tells you the procedure (from memory): open the stream and buffer it, fetch a snapshot stamped with a sequence number, drop every buffered update at or below that number, apply the rest, and if an update's first id ever skips past your last, throw the book away and start over. CME's recovery feed says the same thing in different tags (from memory). Kubernetes clients list, note the `resourceVersion`, and watch from it; when the server has forgotten that version they get `410 Gone`, clear the cache, and list again (from memory). Debezium reads the transaction-log position, snapshots the tables, then streams from the position it noted (from memory). ZooKeeper watches are one-shot and the documentation says outright that you cannot reliably see every change; what it does promise is that you see the watch event before you see the data it is about, so a read after the event is current (from memory). inotify's queue can overflow, and the man page's recovery advice is to rebuild the cache from a fresh scan (from memory).

Every one of these is the same design discovered independently: a *snapshot*, a stream of *changes*, and a *version* that says which changes the snapshot already includes. The edge says when; the level says what; the version says they agree. Edges alone lose state, silently: the epoll man page walks through the edge-triggered reader that takes one kilobyte of two and then blocks forever, because the edge for the write was used up and nobody will announce the leftover (from memory). Levels alone lose moments and cost polling: Kubernetes makes a virtue of it ("the system is not required to touch base at 5 before changing to 3", quoted from memory), which is fine for converging on a desired state and useless for anything that needs to know 5 happened. So the robust systems pair the two, and because the two come from different clocks (a REST call and a websocket, a scan and an inotify queue), they bolt on a sequence number to line them up.

Prism already has the clock those systems bolt on. The occurrence is the version. The World numbers the occurrences, it answers each from one snapshot of its own state, and anything stamped with the same occurrence describes the same state. That is the whole of the discipline, and the rest of this section is working it out.

### The snapshot discipline

The World keeps, for every state-element value it has ever been asked about, the value it last supplied. Call that the snapshot. As occurrence n opens, the World has gathered whatever changed since n−1 (a read completed, a file changed on disk, the clock moved, a socket connected) into a new snapshot Sₙ. It then:

- answers the oracle from the *previous* snapshot Sₙ₋₁, and
- passes the difference between Sₙ₋₁ and Sₙ as the pushes.

So a slot that existed before n holds Sₙ₋₁'s value as n opens and receives n's push, ending n with Sₙ's value. A slot *created* in n reads the oracle, getting Sₙ₋₁'s value, and then receives n's push, and ends n with Sₙ's value. The two kinds of slot end in the same place by different routes, and nothing in the program can tell which route a slot took. That is the rebuild rule of the language applied at the boundary: a widget built mid-occurrence behaves as if built just before the occurrence's inputs, sees all of n, and a `dyn` built in n over a dynamic that changed in n builds once with the new value.

Why the previous snapshot and not the new one? Suppose the oracle answered from Sₙ. A slot created in n would start at Sₙ's value, and then the push for n, which also says Sₙ's value, would arrive and fire its change event with the value it already has. Harmless for the value, but now the slot's dynamic "changed" in n and the one that existed before n also changed in n, and anything derived from either fires in n. Still consistent. The real trouble is a tag read. A widget built in n that reads the new slot's state through a tag in the same occurrence should see the value *before* n, by the tag law, because that is what every other dynamic gives at a tag. Sₙ₋₁ is the value before n. Answering from Sₙ would make a freshly created state element the one dynamic in the language whose tag reads the after-value. Answering from Sₙ₋₁ keeps the law uniform: every dynamic, including one born this occurrence, has a before-value and a change event, and the before-value of a state element is what the World held as the occurrence opened.

Why pushes by value rather than by path? Because two slots asking for the same state must agree in every occurrence, and the cheapest way to guarantee that is to give them one dynamic. A push keyed by path would reach one slot; the World would have to find every slot with an equal element and push to each, and a slot created in n would need a push the World could not have known to send. Keyed by value, the runtime fans the push out, including to slots born in n, and the World's bookkeeping is one map from value to last-supplied value.

### The oracle cannot fail

A state tag the World accepts is always answerable, because the state's type says what unavailable looks like: `Unavailable FsError | Known Text`, `Connecting | Connected | Disconnected Reason`, `NoStamp | Modified Instant`. A World that cannot give a file's contents says why, in the type (`Unavailable NotFound`, `Unavailable Denied`, `Unavailable TooLarge`), and goes on watching it; a World asked for a socket it has not opened answers `Connecting` and opens it. There is no `Pending`, no `null`, no "ask again later," because a dynamic without a value is not a dynamic, and the program's `match` on the state type has an arm for the not-yet case or it does not compile.

A state tag the World does *not* accept is an error in the occurrence the slot is created, reported with the element, the span and the interceptor chain it came through, because nothing later can give it a value. This is deliberately harsher than the `Unsupported` response an unknown request tag gets. A request with no answer is a program-visible fact in a later occurrence: the program fired and got `Unsupported` back and can match on it. A state element with no value has nowhere to put the fact; its dynamic would have to hold something in the occurrence it was born, and the World has nothing to offer. The build error is the right answer, and the way for a portable program to avoid it is below, under cross-platform.

### Where the stitch happens

The World does the stitching, because the World is the only party that sees both clocks. A native read and a native watch are separate system calls that interleave: the watch reports a change while the read is in flight, and whether the read saw the change is unknowable from either alone. The World sets the watch first, reads, reconciles anything the watch reported during the read (a reread, or a check of the modification stamp), and only then puts the result in its snapshot. From the program's side there is one value per occurrence and it is right. The Fs component's inner loop is the Binance procedure with the occurrence as `lastUpdateId` (the field name from memory).

What the World does *not* do is pick a policy the program might want to choose. A book has policy in it: top ten or full depth, what to show during a resync, whether a gap means "stale" or "empty." Policy of that kind belongs in Prism, in an interceptor the program chooses, with the gap in the state's type, and the World offers the feed's edges (`Md.Feed`, a stream of sequenced increments) and snapshot (`Md.Snapshot`, a request) rather than one book for everyone. A World that happens to have a native book builder next to the exchange socket can offer `Md.Book` as a state element beside the feed, and a program that wants its own policy ignores it. The line is: the World stitches where the stitch is mechanical and the source's own protocol dictates it; Prism stitches where there is a choice.

```prism
type BookState = choice:
    Buffering [Delta]          # no snapshot yet; increments wait
    Live Book, Nat             # the book and the last applied sequence number
    Resyncing Book, [Delta]    # a gap; the last good book stays visible

@BookBuilder: fn Symbol, @a -> @a = \|sym, @strategy| @widget:
    !incr <- @Element Md.Feed(symbol=sym)
    !started <- @started
    !snap <- @Element Md.Snapshot(symbol=sym, requests=(unit on (!started | !gap)))
    let !msg = Snapshot !snap | Update !incr
    $book <- Buffering [[]] then StepBook $book, !msg
    let !gap = !Filter (NewGap $book, !msg)
    ((result, $slots)) <- intercept @strategy, !down, $ supply
    !down <- @Forward $slots
    let supply = \|e| match e:
        Md.Book(symbol=s): if s == sym: @ $book else: @Element e
        ignore: @Element e
    @ result
```

`StepBook` is the exchange's procedure as a fold: buffer until the snapshot answers, drop increments at or below its sequence number, apply the rest, go `Resyncing` on a skipped number and ask again. The strategy under it asks for `Md.Book(symbol=sym)` as a state element and gets `$book`, in its creation occurrence, with the policy visible in the type. Under a World with a native book, the same strategy runs with `@BookBuilder` removed. That is the substitutability test from the other direction: the interceptor supplied state in exactly the form the World would have.

### What a push is, and how many

A push is the World saying "as occurrence n opens, this value is now this." It is not an event in the program's sense: a state element's dynamic has a change event, and a push fires it, but the program cannot distinguish a push from any other change of a dynamic, and two pushes for one value in one occurrence are an error, because a dynamic changes at most once per occurrence.

How many pushes per occurrence, and whether a change on disk gets its own occurrence or rides along with a click, is the World's business. A CLI World pushes `Clock` at most once per turn. A GUI World coalesces three resize reports that arrived between frames into one push of the last size, which promises the program no less than the platform promised the World. A World must never coalesce two responses to one slot, because responses are edges and the program may be counting them; it may coalesce state, because state is a level and the last value is the truth.

Every push must satisfy the equality law: in one occurrence, every slot with equal elements sees one value. The runtime guarantees it structurally, by giving equal elements one dynamic; the World guarantees it by answering the oracle and computing the pushes from one snapshot. A World that answered the oracle by calling `stat` would break it, because two calls could straddle a write; that is why the oracle answers from the snapshot map, and why a value not yet in the map is filled once, by the component, and recorded before the answer is given twice.

### The directory listing, stitched

Here is the Fs component's inner loop for `Fs.Dir`, occurrence by occurrence, with the race in it. A program builds `@Element Fs.Dir(path="/home/me")` in occurrence 7.

*Occurrence 7.* The runtime asks the oracle for `Fs.Dir(path="/home/me")`. The World has never been asked; `initial` returns `Unavailable`; the slot's dynamic starts there. The program draws "(unavailable)". After the turn, the component sees a new `Fs.Dir` slot in the list and starts: it registers a watch on `/home/me` *first*, then starts a scan on a thread.

*Between turns.* The scan is running. The watch reports `created b.txt`. The component holds that report, because the scan is not done and it cannot know whether the scan saw `b.txt`.

*The scan completes* with `[[a.txt]]`. The component now reconciles: the watch said `b.txt` was created during the scan, and the scan did not list it, so the component rescans (or, where the platform gives a stable modification stamp, compares stamps and patches). The rescan gives `[[a.txt, b.txt]]`. Only now does the component call `changed(Fs.Dir("/home/me"), Known [[a.txt, b.txt]])`, and the core puts it in `incoming`.

*Occurrence 8*, opened by whatever arrives next (a click, say). The core computes the pushes as `incoming`: one push, `Known [[a.txt, b.txt]]`. The oracle still answers `Unavailable` for the value, which is S₇. The push fires the slot's change event in 8. A second widget that happens to be built in 8 asking for the same directory gets `Unavailable` from the oracle and the push in the same occurrence, and ends 8 holding the same listing as the first. The program draws both.

*Between turns.* The watch reports `deleted a.txt`, then `created c.txt`. The component applies both to its copy and reports `changed(.., Known [[b.txt, c.txt]])` twice; the core keeps the last. One push in occurrence 9, not two: the intermediate listing `[[b.txt]]` is a state the program never sees, which is what level semantics means and what Kubernetes calls not touching base at 5 (from memory).

*The watch overflows.* inotify says events were lost. The component rescans from scratch and reports the result. The program sees an ordinary change. Nothing about overflow crosses the boundary, because the program asked for the listing as a level and the World's job is to keep the level true.

Every occurrence in that sequence answered everything about `/home/me` from one snapshot, and the version that lined up the scan with the watch events was the component's own bookkeeping, which is the part Binance makes every client write. The program wrote none of it.

### Why not events only, and why not dynamics across

Two alternatives deserve a hearing, because each is what you would build first.

*Events only.* The World sends a one-shot answer and a stream of changes, both as responses, and a library widget holds them into a dynamic: drop changes until the answer arrives, let an answer win over a change in the same occurrence, apply changes afterwards. Everything is edges; the boundary is minimal. The cost is that the dynamic has no value in the occurrence its slot is created. A widget built in n sees all of n, including a lexically captured dynamic that changed in n; if its own state arrives in n+1, there is an occurrence in which it has a new value of one thing and a stale value of another, and nothing can hide that from it. Every wrapper also has to apply the stitching rules identically or two readers of one file disagree, which is the equality law reduced to a convention. State elements with a value in their first occurrence are what remove the hiccup, and the oracle is the one mechanism that can supply such a value.

*Dynamics across.* The World hands the program a dynamic. A dynamic is not plain data; it is a runtime object with a lifetime, and a World holding one is a World coupled to the runtime. Written out as plain data, "a dynamic from the World" is an initial value plus a sequence of changes keyed to occurrences, which is a snapshot plus pushes, which is this design. The oracle is how the initial value gets in without being a firing; the pushes are the changes; the occurrence is the key.

An interceptor, which shares the program's clock, can hand a child a dynamic it already holds, and the supplier mechanism is exactly that: a widget returning the state's dynamic, built in the interceptor's node. The World cannot, so its supplier is a function over data, and the two look identical from below.

## Turns and `Defer`

A *turn* is the World's unit: the occurrences it runs before it acts on the result. The World acts on the state at the end of the turn, so it reads the list and performs the request firings after the last occurrence of the turn, not after each one. Most turns are one occurrence: an input arrives, the World opens an occurrence with it, reads the outcome, acts. The exception is `Defer`, an element every World must provide and the only way a turn is ever longer than one occurrence.

```prism
tag Defer of Element:
    requests: !unit
    responds: unit
```

A `Defer` slot that fires its request in an occurrence is answered in a later occurrence *of the same turn*. After each occurrence the world core looks at the request firings; if any `Defer` fired, it runs another occurrence delivering `unit` to each of them, reads the firings again, and repeats until no `Defer` fired. Then the turn is over.

### Why it exists

An interceptor is Prism and cannot start an occurrence. Yet it must sometimes answer a request, and a response is always in a later occurrence than its request; that is the creation rule, and an interceptor is bound by it exactly as a World is. A sandbox denying a file read cannot deliver `Denied` in the occurrence of the `Read`, both because the rule forbids it and because the requesting slot may have been born in that occurrence and is not yet routable. So the sandbox fires a `Defer` and, in the occurrence that answers it, fires `Denied` into its child. From the World's view none of this happened: the turn was one input and one list, and the only thing it did between was answer a `Defer`.

```prism
                Nil:
                    !later <- @Element Defer(requests=(unit on _.!request))
                    @ Denied on !later
```

That arm from the sandbox of chapter 2 is the whole pattern. The child asked in occurrence k; the `Defer` fired in k; the world core opened k+1 with `unit` to the `Defer`'s path; `!later` fired; `Denied on !later` fired; the `for` consolidated it under the child's path; `intercept` routed it against the child's list as k+1 opened, in which the child's slot now exists. The child sees a denial one occurrence after it asked, which is also the soonest a real World could have answered.

Everything that needs "a moment later, before the World acts" is this: a cache answering from what it holds, a mock, a focus manager that wants to move focus to a field built this occurrence (the field is in the list next occurrence, so the move goes through a `Defer`), a layout that feeds a measured size back to a child. None of them is a feature of the language. Each is an ordinary element with the ordinary rule that a response is later than its request.

### Bounded without a cap

Each `Defer` slot is answered at most once per turn. A slot that fires again after being answered in the same turn is an error, and the error has a name: *deferring backwards*. Its request was answered; it is asking to be answered again in a turn that has already given it its moment.

That rule bounds a turn without anyone picking a number. A turn runs at most as many occurrences as there are `Defer` slots that fire, plus one for the input. There is no budget, no timeout, no runaway report to design, and no configuration knob whose value is wrong for somebody. The objection is that a budget would let a deep but finite cascade through one `Defer` finish, where this rule stops it on the first repeat. That cascade is iteration to a fixpoint, and it belongs in a pure function (where it is a loop), or in the next turn (where the platform gets to run in between). A loop through one `Defer` that does not stop is a bug, and finding it on the first turn beats freezing the application's output while the loop spins.

The classic version of the bug is caught early: a child that retries immediately on `Denied` makes its sandbox's `Defer` fire again in the same turn, which is deferring backwards, which the world core reports with the `Defer`'s path and the interceptor chain. A retry belongs behind `Time.After`, where the World's clock separates the attempts and each is its own turn.

A `Defer` slot created in occurrence k of a turn, firing in k (from `@started`, say, or because the request it stands in for fired in k), is answered in k+1 of the same turn. The sandbox needs this: the child's slot and the sandbox's `Defer` for it are both born in k. The bound is then the number of `Defer` slots that exist at any point during the turn, which a program can inflate only by building a fresh `Defer` in every occurrence, with a `dyn` that rebuilds on the previous one's answer and fires the new one from `@started`. That is a loop the program wrote, through a rebuild instead of through one slot, and the world core treats it as deferring backwards too: a `Defer` at a path that differs from an already-answered `Defer`'s path only in its `dyn` generation is the same `Defer` reborn. The rule is stated this way because it makes the bound structural rather than counted; if a program turns up that needs a `Defer` per generation for a finite, legitimate cascade, the rule would loosen to counting generations, and the test would be that program.

### Deferring backwards, precisely

Two things are errors, both reported with the path and the interceptor chain: a `Defer` slot firing after it was answered in this turn, and a `Defer` slot whose path matches an answered one up to `dyn` generation, as above. One thing that looks like a third is not: a `Defer` whose slot was torn down in the occurrence it fired. There is nothing to answer, so the world core answers nothing, and the turn ends if that was the last fired `Defer`.

### Why no phases

Suppose the World ran named stages after the input occurrence: layout, then focus, then frame, each at most once per turn, with an element `Phase(name)` that defers an event until its stage. A GUI library would declare which it uses; size feedback would go to the layout stage, autofocus to the focus stage, animation to the frame stage before paint. It is a real design and it solves real problems, and it is not in the language.

What it costs. There is now a second notion of "later" with an ordering table: which stage runs before which, what happens when a stage defers into an earlier stage, whether an empty stage costs an occurrence, what a stage means under a test World, whether an interceptor may add stages. Each World picks its stages, so a library written against one World's stages runs differently under another. And every stage is expressible without it: a layout feedback is a `Defer` whose answer the layout reads in the next occurrence; ordering between two deferred things is a dynamic one of them reads through a tag; a frame tick is a `Gui.Frames` stream element the World fires when it is about to paint. The language has one "later," which is "the next occurrence of this turn," and nothing finer. Priorities, stages and scheduling belong to the implementation if they belong anywhere, and the implementation does not need them either.

### What a turn looks like from each side

From the World's side: one input (or a batch it chose to deliver together), one call to the world core's `turn`, one outcome. It performs the request firings of every occurrence of the turn in order, reads the final list, reconciles, and waits for the next input.

From the program's side: an occurrence with inputs from outside, possibly followed by occurrences with inputs only from `Defer` slots, each a full occurrence under the ordinary laws. The program cannot tell where a turn ends, and must not care; a program that behaves differently depending on whether two inputs were one turn or two is depending on the World's batching, which the language leaves to the World. What the program *can* rely on is that a `Defer` fired in occurrence k is answered before any new outside input arrives, which is what makes a sandbox's denial land before the user's next keystroke.

From a test's side: `Send` runs a whole turn, including the `Defer` occurrences, and `Expect` sees the end of it. A script that expects `"(reading...)"` after a sandbox denial would fail: by the end of turn zero the denial has landed.

### A turn with a cache, occurrence by occurrence

An HTTP cache is the interceptor that shows every part of a turn, because it answers some requests itself and forwards the rest, and the two take different numbers of occurrences.

```prism
@HttpCache: fn @a -> @a = \|@child| @widget:
    ((result, $slots)) <- intercept @child, !down, $ \|e| @Element e
    !down <- for $slots:
        dyn $_:
            match _.element:
                Http.Fetch(..):
                    let !get = !Filter (CacheKey _.!request)            # Some url for a GET, else Nil
                    let !hit = !Filter (Lookup $cache, !get)             # Some answer if cached
                    let !miss = !Filter (NotCached $cache, _.!request)
                    $hit <- Nil then Some !hit
                    !later <- @Element Defer(requests=(unit on !hit))
                    !fetched <- @Element Http.Fetch(requests=!miss)
                    @ !Filter ($hit on !later) | !fetched
                ignore:
                    ((!resp, ignore)) <- @Element _.element
                    @ !resp
    let !answers = !Filter (Cacheable !down)        # ((url, answer)) for GETs that passed through; an answer names its request
    $cache <- {} then InsertPair $cache, !answers
    @ result
```

For each child `Http.Fetch` slot, a request that the cache can answer fires a `Defer`; one it cannot is forwarded on the cache's own `Http.Fetch` element, and the response comes back to this block and is consolidated under the child's path. `$cache` is read through a tag, so a request sees the cache as the occurrence opened.

Take a child that fires `GET /a` in one turn and, later, `GET /a` again, under a World that answers `/a` in one turn.

**Turn 1, occurrence 1.** Outside input: a click, delivered to a button in the child. The child's widget fires `GET /a` on its `Http.Fetch` slot. The cache's block for that slot sees `_.!request` fire; `$cache` is `{}` as the occurrence opened, so `!hit` is quiet and `!miss` fires; the cache's own `Http.Fetch` slot fires its request. No `Defer` fired. `run` returns with `requests: [(cache/.., [GET /a])]`. The world core sees no `Defer` in the firings; the turn is one occurrence. The World performs the fetch.

**Turn 2, occurrence 1.** The fetch completes. The World delivers `Answered ..` to the cache's `Http.Fetch` slot at its path. `!fetched` fires in the block; the block's result fires; `!down` fires with `{child_path: Answered ..}`; `intercept` routes it to the child's slot, which existed as the occurrence opened; the child's label updates. `!answers` sees a cacheable GET pass through and `$cache` gains `/a`. One occurrence.

**Turn 3, occurrence 1.** Another click. The child fires `GET /a` again. The cache's block reads `$cache` through a tag: it has `/a`. `!hit` fires with the answer; the `Defer` slot's request fires. `!miss` is quiet. `run` returns with `requests: [(cache/../defer, [unit])]`.

**Turn 3, occurrence 2.** The world core sees the fired `Defer` in the list it just received, records its path as answered, and calls `run` again with `responses: [(defer_path, unit)]`, no pushes, the same oracle. `!later` fires; `$hit on !later` reads the hold through a tag and gets the answer committed in occurrence 1; `!Filter` passes it; `!down` fires; the child's slot gets its answer. No `Defer` fired in this occurrence. The turn is over: two occurrences, one input. The World performed no fetch, and the transcript shows `Requested Http.Fetch(..)` for turn 1 and nothing for turn 3, which is what `cache_hit_no_request` asserts.

What the child saw in turn 3 is a response one occurrence after its request, which is the earliest any World could have answered, so a child tested under the cache sees the same timing as one tested without it. A mock built the same way has the same property, and that is why mocks go through `Defer` rather than answering "immediately": there is no immediately, and a mock that pretended there was would hide timing bugs production would show.

## Obligations of a World, and who each protects

A conforming World keeps the list below. Every item is stated with the party it protects, because an obligation nobody would miss is a rule for its own sake, and there are none of those here.

**Send responses only to paths in the list it last received.** *Protects the program.* A slot that is gone has no node to receive, and a response to it is a response to nothing; the runtime reports it as `UnknownPath`. The World is the only party that knows an operation is still in flight after its slot was torn down, so the World discards the late result and cancels the operation if the transport allows. The program never has to defend against an answer to a question it stopped asking.

**Never deliver a response in the occurrence of its request.** *Protects the program's reasoning and the creation rule.* Request firings come out of `run` after the occurrence commits; the earliest they can be answered is the next call. A widget may therefore assume that in the occurrence it asked, nothing it asked for has happened, which is what lets it fire from `@started` without a guard. The runtime enforces this by construction; the obligation is that a World not offer an element that "responds merely because it was created," which it could do by sending a response in the first occurrence after a slot appears with no request having fired. A World may do that only for a tag that declares `responds` without `requests`, a stream, where the response is an observation and not an answer.

**Answer the oracle synchronously and consistently with the pushes.** *Protects the equality law.* One snapshot per occurrence, answered the same to everyone, with the pushes as the diff from the previous one. A World that answers the oracle from a live source instead of its snapshot can give two slots two values in one occurrence and has broken the program, not itself.

**Provide `Defer` with the turn semantics above.** *Protects interceptors.* Every interceptor that answers anything needs it, and a World without it cannot host a sandbox, a cache or a mock.

**Treat the element list as the effect stream, and perform request firings in order.** *Protects determinism, and so the transcript.* The list after the turn is what the World reconciles to; the firings are performed in occurrence order then list order. Two Worlds reading the same outcome do the same things in the same order, which is what makes a golden transcript of a CLI run a test.

**Answer a request on a tag it does not accept with `Unsupported` in the next turn.** *Protects portable programs.* A `Ui.Button` under a CLI World, a `Clipboard` under a World without one: the program fired, and it gets a program-visible fact back rather than silence. A presence element it does not accept (one with neither `responds` nor `state`) is an error it reports and otherwise ignores, since there is nothing to answer.

**Report a state tag it does not accept as an error in the creation occurrence.** *Protects the type.* A dynamic that cannot have a value cannot exist, and the error says so where the slot was made.

**At most one response per path and one push per value per occurrence.** *Protects the runtime's one-firing-per-occurrence law.* A World with two chunks for one slot has two occurrences.

**Discard the responses of a turn that errored, keep its pushes, and report.** *Protects the transaction.* An occurrence that returned `Err` did not happen; the responses that opened it are gone, since the program's slots never received them; the pushes are state the world holds and the program has not seen, so they go again with the next turn, as the diff from whatever the program last held.

What a World is free to do: batch inputs that arrived together into one occurrence or not, coalesce state (never responses), run on whatever thread the platform dictates, multiplex sockets and timers however it likes, keep one OS socket per address or per slot, key native widgets by tree position for continuity across rebuilds. The language says nothing about any of it, and a program that could tell the difference is depending on something the language did not promise.

## The world core

Everything in the obligations that is not about a particular tag is written once, in the host language, as a library every World in that language uses. A World is the core plus its components. This is not a language rule; it is what makes a World small enough that there will be several, and it is also where the obligations are tested, once, instead of per World.

### The turn loop

```rust
pub struct Core {
    runtime: Runtime,
    snapshot: BTreeMap<ElementValue, Value>,   // S_{n-1}: what the program last saw
    incoming: BTreeMap<ElementValue, Value>,   // changes components reported since
    last_list: BTreeMap<Path, ElementValue>,   // the list the World last received
    components: Vec<Box<dyn Component>>,
    turn_no: u64,
}

pub struct TurnOutcome {
    pub list: Vec<(Path, ElementValue)>,
    pub requests: Vec<(Path, Vec<Value>)>,     // every occurrence of the turn, in order
    pub occurrences: u32,
    pub error: Option<RunError>,
}

impl Core {
    pub fn turn(&mut self, responses: Vec<(Path, Value)>) -> TurnOutcome {
        // 1. Check the World's own addressing before the runtime does.
        let responses: Vec<_> = responses.into_iter()
            .filter(|(p, _)| self.last_list.contains_key(p))   // obligation 1
            .collect();

        // 2. Build this turn's snapshot and pushes.
        let previous = self.snapshot.clone_shallow();            // S_{n-1}, for the oracle
        let pushes: Vec<_> = self.incoming.drain().collect();   // S_n − S_{n-1}
        self.snapshot.extend(pushes.iter().cloned());            // S_n
        let oracle = |e: &ElementValue| previous.get(e).cloned()
            .unwrap_or_else(|| self.unavailable_for(e));          // never fails for accepted tags

        let mut all_requests = Vec::new();
        let mut inputs = Inputs { responses, pushes: pushes.clone(), oracle: &oracle };
        let mut answered: BTreeSet<Path> = BTreeSet::new();
        let mut occurrences = 0;

        loop {
            occurrences += 1;
            let outcome = match self.runtime.run(inputs) {
                Ok(o) => o,
                Err(e) => {
                    // The program saw none of this turn: put the pushes back so the
                    // next turn's diff is from what the program last held.
                    self.snapshot = previous;
                    for (ev, v) in pushes { self.incoming.entry(ev).or_insert(v); }
                    return self.failed_turn(e, occurrences);
                }
            };
            all_requests.extend(outcome.requests.iter().cloned());
            self.last_list = outcome.list.iter().cloned().collect();

            // 3. Defer: collect fired Defer slots that are still in the list.
            let defers: Vec<Path> = outcome.requests.iter()
                .filter(|(p, _)| self.last_list.get(p).map_or(false, |e| e.tag == DEFER))
                .map(|(p, _)| p.clone())
                .collect();
            if defers.is_empty() {
                return TurnOutcome { list: outcome.list, requests: all_requests, occurrences, error: None };
            }
            for p in &defers {
                if answered.contains(p) || answered.iter().any(|q| same_up_to_generation(p, q)) {
                    self.snapshot = previous;
                    for (ev, v) in pushes { self.incoming.entry(ev).or_insert(v); }
                    return self.failed_turn(RunError::DeferBackwards(p.clone()), occurrences);
                }
                answered.insert(p.clone());
            }
            // 4. The next occurrence of the same turn: Defer answers, no pushes, same oracle.
            inputs = Inputs {
                responses: defers.into_iter().map(|p| (p, Value::unit())).collect(),
                pushes: Vec::new(),
                oracle: &oracle,
            };
        }
    }
}
```

Four things to read off it. The oracle is the same closure for every occurrence of the turn, over the same `previous`, and the pushes go in only with the first occurrence: the snapshot moves once per turn, as the turn opens, so every occurrence of a turn sees one world. The pushes are the diff the components reported since the last turn; the core does not compute diffs, it collects what components told it changed. Deferring backwards is detected here, by path, before the runtime sees a second answer. And the list the World keeps for addressing is replaced after every occurrence, not every turn, because a slot torn down in a `Defer` occurrence must be gone before the next outside input is addressed.

A `Defer` occurrence could take fresh pushes, letting a file change land in the middle of a turn. Nothing would break, but the sandbox that answered in occurrence k+1 would then be answering against a world that moved under it between k and k+1, and the program could observe the move by comparing a state read in k with one in k+1 of the same turn. Holding the snapshot still for the turn makes the turn what it claims to be: one step, as the outside sees it.

### Bookkeeping the core does for every World

*The last list.* Kept for addressing (responses only to known paths) and for diffing: after a turn, the core tells each component which of its slots are new and which are gone, so a component can open what is newly described and cancel what is no longer. The diff is the component's input, not a trigger in the program's sense: a component that opens a socket for a `Net.Socket` slot is reconciling the world to the description, and will reconcile the same list to the same world without opening a second socket.

*Late results.* A component that finishes an operation for a slot that is gone hands the core a result the core cannot address. The core drops it and records it in diagnostics, with the path and the tag, so that a World developer can see that a response was discarded rather than wondering why it never arrived. Cancellation is the component's job because only the component holds the handle.

*Snapshot and pushes.* Components report `changed(value, new)` into `incoming`; the core commits `incoming` into `snapshot` as the next turn opens and hands it to the runtime as pushes. A component that reports the same value twice before a turn opens has reported one push, the last, which is coalescing done right: state is a level.

*`Unsupported`.* A request firing on a tag no component claims gets a response `Unsupported` in the next turn, queued by the core as an ordinary response. A presence tag no component claims is logged once per path. A state tag no component claims makes `unavailable_for` return an error marker, and the runtime reports `UnsuppliedState` in the creation occurrence.

*Errors.* `failed_turn` discards the turn's responses, puts the turn's pushes back into `incoming` so that the next turn's diff is from what the program last held (the world moved, and the program has not seen it), and returns the error with the occurrence number of the turn in which it happened. How the error is printed is one format, shared, so that every World in one host language prints a cycle the same way.

### The component trait

```rust
pub trait Component {
    /// The tags this component claims. Tags are claimed by exactly one component.
    fn tags(&self) -> &[TagId];

    /// The World's snapshot, for a value no component has reported yet:
    /// the tag's unavailable case, never a syscall.
    fn initial(&self, e: &ElementValue) -> Value;

    /// After a turn: slots of this component's tags that appeared and vanished.
    /// Reconcile: open, close, cancel. Never answer here.
    fn reconcile(&mut self, appeared: &[(Path, ElementValue)], vanished: &[(Path, ElementValue)], ctx: &mut Ctx);

    /// After a turn: request firings on this component's slots, in order.
    fn perform(&mut self, path: &Path, e: &ElementValue, fired: &[Value], ctx: &mut Ctx);
}

/// What a component may say to the core. Everything here is queued for a
/// later turn; nothing reaches the runtime during a turn.
pub struct Ctx {
    pub fn respond(&mut self, path: Path, value: Value);          // next turn, if the path still exists
    pub fn changed(&mut self, e: ElementValue, value: Value);     // next turn's push
    pub fn wake(&self) -> Waker;                                   // for threads: "there is input"
}
```

A component never sees the runtime. It sees paths and element values, which are data; it reports responses and changes, which are data; and it is called by the core at two moments, after a turn to reconcile and perform. Its own threads, futures and file descriptors are its business, and the only way they reach the program is by putting a response or a change into the core's queue and waking the loop.

Why a component is *called* rather than subscribed: there are no callbacks across the boundary, and that rule holds inside the World too, between core and component. The core could instead hand each component a stream of list changes and have the component react; that is the same information with the component in charge of timing, and the core exists to own timing. A component that performs between occurrences of a turn, say, would make the turn observable from outside, and the core calling `perform` once after the turn is what prevents it.

### What the core does not do

It does not diff GUI trees, lay anything out, or know what a window is. It does not retry anything, because a retry is a policy and policies are Prism. It does not run occurrences on its own initiative: a World with no input has no turn, and a program under it is a data structure until something arrives. And it does not hold a reference to anything in the runtime but the runtime itself, which it owns.

## General components

A World's components are the tags it accepts. The ones below are general: a CLI World, a GUI World and a server World all accept them, each with its own native implementation, and a program written against them runs under any of the three. Each gets its tag declaration and an example of use. The language fixes the kinds: which parts a tag declares and what each part means. A World may spell its own tags differently.

Three forms recur. A *request* element declares `requests` and `responds`: the program fires, the World answers later, one answer per firing. A *stream* element declares `responds` alone: observations arrive on their own, each in an occurrence after the slot's creation, and the program never asks. A *state* element declares `state`: a value in the creation occurrence from the oracle, pushed afterwards. A tag may declare any combination, and `Net.Socket` declares all three.

### Fs

```prism
type FsRequest = choice:
    Read
    Write Text
    Append Text
    Delete

type FsError = choice:
    NotFound
    Denied
    TooLarge
    Other Text

type FsResponse = choice:
    Contents Text
    Written
    Unavailable FsError

tag Fs.File of Element:
    path: Text
    requests: !FsRequest
    responds: FsResponse

type Contents = choice:
    Unavailable FsError           # the reason the contents cannot be had
    Known Text

tag Fs.Contents of Element:
    path: Text
    state: Contents

type Entry = record:
    name: Text
    kind: EntryKind

type EntryKind = choice:
    File
    Dir
    Link
    Other

type Listing = choice:
    Unavailable
    Known [Entry]

tag Fs.Dir of Element:
    path: Text
    state: Listing

type Stamp = choice:
    NoStamp
    Modified Instant

tag Fs.Watch of Element:
    path: Text
    state: Stamp
```

`Fs.File` is the one-shot conversation: fire a `Read`, get `Contents`, `Written` or `Unavailable` with an `FsError` reason back, addressed to this slot, in a later occurrence. The path is a field, fixed for the slot's life; a program with a dynamic path has a `dyn`, a fresh slot per path, and the World drops any response to the old slot because the slot is gone. One slot can read now and write later, and a program that wants two reads in flight at once has one slot and two firings, or two slots.

```prism
@SaveButton: fn Text, $Text -> @$Text = \|path, $draft| @widget:
    !save <- @Button $"Save"
    !r <- @Element Fs.File(path=path, requests=(Write $draft on !save))
    $status <- "" then match !r:
        Written: "saved"
        Unavailable Denied: "permission denied"
        ignore: "failed"
    @ $status
```

One click, one write, and `$draft on !save` reads the draft as it stood before the click, which is the draft the user was looking at.

`Fs.Contents` is the file as a value that is simply true: the World answers the oracle from its snapshot, filling an entry it has never held by reading the file once under a size cap, and then watches and pushes each time the file changes. A file that cannot be had is `Unavailable` with one of four reasons from a closed choice: `NotFound`, `Denied`, `TooLarge` (past the cap, where a program that wants the file asks `Fs.File`), or `Other` with the operating system's text. The reason is part of the level, so a file that appears is a push from `Unavailable NotFound` to `Known`, and one that vanishes is the push back. Two widgets watching one path share one dynamic. The World stops watching when the last slot for that path is gone.

`Fs.Dir` is the same for a directory listing. A directory is where the stitch earns its keep: the World sets the watch, scans, reconciles what the watch reported during the scan, and only then puts the listing in its snapshot; on an inotify overflow it rescans and the program sees one ordinary change. A program sees `Known [[..]]` and never a partial listing.

`Fs.Watch` is a watch *as state*: its value is the file's modification stamp, pushed when the file changes. A program that wants to do something on change derives it from the dynamic's change, and the idiom is a `dyn` whose block fires from `@started`:

```prism
@ReloadOnChange: fn Text -> @!FsResponse = \|path| @widget:
    $stamp <- @Element Fs.Watch(path=path)
    !!reads <- dyn $stamp:
        !started <- @started
        !r <- @Element Fs.File(path=path, requests=(Read on !started))
        @ !r
    @ !Switch !!reads
```

Every change of the stamp rebuilds the block, whose `@started` fires a fresh `Read`. That is a watch delivered as a level and consumed as edges, by the program's choice, and it is why the watch is not a stream. A stream element `Fs.Changed(path)` responding with `unit` would be the same information as an event, with one difference: a slot created in the same occurrence as a change would miss it, because a response cannot reach a slot born this occurrence, while a state slot created in the push's occurrence sees the push. Watches as state lose nothing at the edges; watches as streams can.

The negative space in Fs: no `Fs.Exists`, because `Fs.Contents` answers it with `Unavailable NotFound` against `Unavailable Denied` or `Known`, and the reason is in the type, so a program that wants to tell absent from unreadable matches on it. No streaming reads; a World that wants to offer them offers `Fs.Stream(path)` responding with chunks, like standard input, and the first version does not.

### Time and Clock

```prism
type Instant = Instant Int        # milliseconds since the World's epoch
type Duration = Duration Int      # milliseconds
type Tick = tick

tag Clock of Element:
    state: Instant

tag Time.After of Element:
    requests: !Nat                 # a delay in milliseconds
    responds: Nat                  # the same number, echoed back

tag Time.Every of Element:
    period: Duration
    responds: Tick

type Frame = record:
    now: Instant

tag Gui.Frames of Element:
    responds: Frame                # a GUI World fires this before it paints
```

There is no `now()`. `Clock` is a state element: a value in the creation occurrence, pushed when the World chooses, at most once per turn for the Worlds in this chapter. `Clock` has no fields, so every clock in a program is one clock and every reading in one occurrence agrees. A test sets it with `State Clock, Instant 5000` and moves it with `Push`, which is why a debounce is testable here and nowhere else.

`Time.After` is a request: fire a delay in milliseconds, get the same number back after at least that long. Each firing is its own timer, and a slot with two firings in flight gets two answers. A debounce is this element and a counter:

```prism
let IsOne: fn Int -> ?unit = \|n| if n == 1: Some unit else: Nil

@Debounced: fn Nat, !a -> @!a = \|d, !in| @widget:
    $last <- Nil then Some !in
    !done <- @Element Time.After(requests=(d on !in))
    let !delta = +1 on !in | -1 on !done
    $pending <- 0 then $pending + !delta
    let !settled = !Filter (IsOne $pending on !done)
    @ !Filter ($last on !settled)
```

Every input starts a timer and bumps a pending count; every expiry decrements it; the output fires with the last input when an expiry finds the count at one, meaning the timer that just expired was the only one alive, so no input arrived during its life. Under a test World the timer is `Send Time.After(..), d`, and the program's text is the whole specification of what "quiet for d" means, including the case the merge decides: an input and an expiry in one occurrence count as an input, and the lost decrement makes the next expiry too early by one. A program that minds sums the two with a `!Union` over a map instead of a merge, and the point is that it had to say so.

`Time.Every` is a stream: a tick per period while the slot exists. Its period is a field, so a dynamic period is a `dyn` and a fresh slot, and the phase restarts; a World that wants phase continuity across the rebuild keys its timer on tree position and tag, and no program can tell. Ticks are responses, so they never coalesce: a World that fell behind skips the missed ticks and delivers the next one; it never delivers two in one occurrence.

`Gui.Frames` is a GUI World's "about to paint" stream, for animation. A program animating a transition puts the slot inside a `dyn` over whether it is animating, so that when nothing wants frames the World stops asking for them.

The stopwatch of chapter 2 is the worked example: `Clock` for the time, `Time.Every` to make the World come back, and a hold that stamps `Start` with the clock as it stood before the click, by the tag law.

### Process

```prism
type Command = record:
    program: Text
    args: [Text]
    env: Text ==> Text
    cwd: ?Text

type SpawnResponse = choice:
    Spawned ProcId
    NotFound
    Denied

tag Process.Spawn of Element:
    requests: !Command
    responds: SpawnResponse

type ProcInput = choice:
    Stdin Text
    CloseStdin
    Signal Sig

type ProcOutput = choice:
    Stdout Text
    Stderr Text
    Exited Code

type ProcStatus = choice:
    Running
    Exited Code
    Gone                           # no such child; the id is stale

tag Process.Child of Element:
    id: ProcId
    requests: !ProcInput
    responds: ProcOutput
    state: ProcStatus
```

Spawning is a request, because starting a process is a one-shot effect with a result (`Spawned id` or `NotFound`), and because the alternative, a `Process.Child(command=..)` whose presence in the list means "a child like this should be running," runs into identity. State is identified by value: two slots with equal elements share one state, so two `Process.Child(command=[["ls"]])` slots would share one `ProcStatus`, while their responses, identified by path, would belong to two conversations. One process or two? The element's fields cannot say. With an id the World allocated, two children are two ids, two elements, two states, and the program keys a `for` on the ids it was given, exactly as a server keys connections on the ids its listener handed out.

```prism
@RunOnce: fn Command -> @$ProcStatus = \|cmd| @widget:
    !started <- @started
    !sp <- @Element Process.Spawn(requests=(cmd on !started))
    $id <- Nil then match !sp:
        Spawned i: Some i
        ignore: Nil
    !$status <- dyn $id:
        match _:
            Some i:
                ((!out, $st)) <- @Element Process.Child(id=i, requests=!never)
                @ $st
            Nil:
                @ $ Gone
    $$status <- $ Gone then !$status
    @ $Join $$status
```

The child's output arrives as responses, each chunk in its own occurrence; its exit is both a response (`Exited`, an edge the program can count) and a state (`Exited`, a level a late-built widget can read). Standard input to the child is a request firing. When the last `Process.Child(id)` slot for an id is gone, the World kills the child: that is its cancel-on-teardown obligation, the same one that aborts an in-flight read. A program that wants a graceful stop fires `Signal Term` first and keeps the slot until `Exited`.

Arguments and environment are state elements, and they belong to the process, so a GUI World offers them as readily as a CLI World does:

```prism
tag Process.Args of Element:
    state: [Text]

tag Process.Env of Element:
    state: Text ==> Text
```

They are state rather than parameters of `@main` for the sake of the sandbox: an interceptor can supply a child a different `Process.Args`, a test can `State Process.Args, [["notes.txt"]]`, and neither needs a second way to start a program. A World that cannot observe a change to the environment never pushes one; a World that can (a shell that lets the user set a variable) pushes it, and the program's `dyn $args` rebuilds, which is what you want.

### Net

```prism
type Link = choice:
    Connecting
    Connected Nat                  # bytes queued and not yet written: backpressure
    Disconnected Text

type SocketEvent = choice:
    Received Bytes
    Closed

tag Net.Socket of Element:
    address: Text                  # "tcp://host:port", "tls://host:port", "unix:/path"
    requests: !Bytes
    responds: SocketEvent
    state: Link

type Accepted = Accepted ConnId

tag Net.Listener of Element:
    address: Text
    responds: Accepted

tag Net.Conn of Element:
    id: ConnId
    requests: ![ConnRequest]       # Send Bytes | Close
    responds: SocketEvent
    state: Link

type HttpRequest = record:
    method: Text
    url: Text
    headers: Text ==> Text
    body: Bytes

type HttpResult = choice:
    Answered HttpAnswer            # status, headers, body
    Failed Text

tag Http.Fetch of Element:
    requests: !HttpRequest
    responds: HttpResult

type Incoming = record:
    id: ReqId
    request: HttpRequest

tag Http.Server of Element:
    address: Text
    responds: Incoming

tag Http.Reply of Element:
    id: ReqId
    requests: !HttpAnswer
    responds: Delivered            # Delivered | Gone
```

`Net.Socket` is the three-part element. The address is a field; a reconnect is a new address and a fresh slot, and the chat client of chapter 2 reconnects by typing. Its request is the bytes to send; its responses are what arrives and the close; its state is the link, shared by every slot with the same address, because there is one connection, while sends and receives are per slot, because each is its own conversation on it. Whether a World keeps one OS socket per address or per slot is invisible if it keeps the equality law.

Backpressure is state. `Connected n` says how many bytes the World has accepted and not yet written, and a program that cares reads it through a tag at the moment it would send:

```prism
    let !out = !Filter (WhenRoom $link, $draft on !send)

let WhenRoom: fn Link, Text -> ?Text = \|link, msg| match link:
    Connected queued: if queued < 65536: Some msg else: Nil
    ignore: Nil
```

A program that ignores it still works; the World buffers. A program that respects it never has a buffer grow without bound, and it did so by reading a dynamic, not by waiting on a writable callback.

`Net.Listener` is a stream of accepted connections, each with a World-allocated id, and `Net.Conn(id)` is the connection, keyed by that id in a `for`, as in chapter 2's server. `Http.Fetch` is the client: fire a request, get a result. `Http.Server` is a stream of incoming requests with ids, and `Http.Reply(id)` is how a reply goes back: a request element keyed by the request's id, fired once with the answer. A proxy that answers each incoming request by making an outgoing one is a `for` over the pending ids:

```prism
type Change = choice:
    Open HttpRequest
    Close

let OneOpen: fn Incoming -> ReqId ==> Change = \|i| {i.id: Open i.request}

let Apply: fn ReqId ==> HttpRequest, ReqId ==> Change -> ReqId ==> HttpRequest = \|m, cs|
    Fold cs, m, \|acc, id, c| match c:
        Open r: Insert acc, id, r
        Close: Remove acc, id

@Proxy: fn Text, Text -> @unit = \|listen, upstream| @widget:
    !in <- @Element Http.Server(address=listen)
    let !opens = OneOpen !in
    !dones <- for $pending:
        !started <- @started
        !res <- @Element Http.Fetch(requests=(Rewrite upstream, $_ on !started))
        let !answer = match !res:
            Answered a: a
            Failed why: BadGateway why
        !sent <- @Element Http.Reply(id=key, requests=!answer)
        @ Close on !sent
    $pending <- {} then Apply $pending, (!Union !opens, !dones)
    @ unit
```

Each pending request gets its own `Http.Fetch` slot under its own key, so there is no correlation table: the response comes back to the slot that asked. The `for` item is built in the occurrence the request arrives, `@started` fires there, and the outgoing request is performed after that turn. When the reply is delivered, the block fires `Close`, the consolidated `!dones` removes the id from `$pending`, and the slots go at commit. An incoming request and a delivered reply in one occurrence are both counted, because `!Union` merges the maps rather than choosing one.

### Errors as a stream

```prism
type RuntimeError = record:
    kind: ErrorKind                # Cycle | Overflow | Protocol | Panic
    spans: [Span]
    message: Text

tag Errors of Element:
    responds: RuntimeError
```

A World may offer `Errors`: a stream element that fires, in a later turn, with each runtime error the World received. A program subscribes to it like any stream, and what it does with the information is its own business: show it, log it through `Cli.Stderr`, count it. A program cannot *catch* an error, because the occurrence it happened in did not happen and nothing in the language can observe a rollback from inside; what it can do is learn, afterwards, that one occurred. For a World that quarantines a failing subtree (replacing it with an `Error(span, message)` presence element in the list), the stream and the element say the same thing from two sides: the stream is the edge, the element is the level.

The alternative is a `catch` that lets a parent widget react to a child's runtime error in the occurrence it happens. That is an effect handler in disguise, and it would make a hold's value depend on whether some unrelated subtree threw, which is the back door the language closed. Errors a program is meant to handle are values with types; errors the language cannot make into values are visible as elements and nowhere else.

## The CLI World

The smallest World that is not a test, and the first place a Prism program touches a real process. Its components are the general ones above plus four `Cli.*` tags of its own, declared here with `Process.Exit`:

```prism
type StdinEvent = choice:
    Chunk Text
    End

tag Cli.Stdin of Element:
    responds: StdinEvent

type WriteFailed = choice:
    Closed
    Failed Text

tag Cli.Stdout of Element:
    requests: !Text
    responds: WriteFailed

tag Cli.Stderr of Element:
    requests: !Text
    responds: WriteFailed

tag Process.Exit of Element:
    requests: !Code

type Signal = choice:
    Interrupt
    Terminate
    Hangup
    WindowChange

tag Cli.Signals of Element:
    responds: Signal
```

Standard input is a stream: chunks, then `End`, delivered only while a `Cli.Stdin` slot exists, each in its own occurrence. The stream belongs to the World, not to the slot. A read that is already blocked when the slot goes cannot be taken back, so when it returns its chunk is not delivered to the dropped slot or anywhere else; the World keeps it with the stream, and the next `Cli.Stdin` slot receives it first, ahead of anything read afterwards. A chunk nobody asks for by the time the process ends is discarded with it. Standard output and standard error are requests: a firing is a write, and nothing is written for any other reason. Exit is a request. Signals are a stream, and the World installs a handler only while a `Cli.Signals` slot exists; without one the process keeps the operating system's defaults, so Ctrl-C still kills a program that never asked to be told about it.

### One turn per input

Each chunk, each completed read, each timer expiry, each signal is its own turn. The World's loop is a plain blocking loop on an input queue: block until something is there, take one thing, run a turn, perform the firings, reconcile, block again. Two chunks that arrived while the program was busy are two turns in order, never one occurrence, because they are two responses to one slot. A completed file read and a chunk that arrived together may be one occurrence, since they address different slots, and the World that batches them is making the honest claim that both were waiting.

### Output ordering

After a turn the World performs the turn's request firings in occurrence order, and within an occurrence in list order. For writes that means: everything occurrence 1 wrote, in the order the slots appear in the list, then everything occurrence 2 wrote. A `Cli.Stdout` slot and a `Cli.Stderr` slot that both fired in one occurrence write in list order, which is the order the program bound them. Two firings from one slot in one occurrence are impossible (a request event fires once per occurrence), so a program that wants two lines in one moment fires a text with a newline in it, or a `[Text]` if the tag's request type is an array. Output is flushed after each turn, so a program that writes and then waits on a timer has its output visible while it waits.

### Exit as a request

```prism
    @Element Process.Exit(requests=(Code 0 on !end))
```

When `Process.Exit` fires in a turn, the World finishes the turn, performs every firing of that turn in order (so a write fired in the same occurrence as the exit lands first, and a write fired in a later occurrence of the same turn also lands first), and exits with the code. Exit is a request and not a presence for the reason chapter 2 gives: a presence would have to appear at the right moment, which means a `dyn` over a flag, and the World would have to act on an appearance, which is a trigger by existence. The element is always there, and firing it is the instruction.

A program that never fires `Process.Exit` exits with code 0 when nothing can ever cause another occurrence: the input queue is empty, no stream slot is live (no `Cli.Stdin` or it has delivered `End`, no `Cli.Signals`, no `Time.Every`, no `Net.Listener` or socket), and no request is in flight. That is not a policy so much as an observation: a graph with no sources left cannot change, and a process waiting on it would wait forever. The alternative, requiring an explicit exit, turns every forgotten exit into a hang. `Process.Args`, `Process.Env`, `Clock`, `Fs.Watch` and `Fs.Contents` watches do not keep a program alive; a live `Cli.Stdin`, a `Cli.Signals` slot, a `Time.Every` slot, an in-flight request or timer, and a listener or socket do. Args, env and the clock do not, because the CLI World never pushes them on its own; a `Clock` push arrives with some other input, and when no other input can come, neither can the clock.

### A worked run

`wc -l` from chapter 2, driven by the CLI World:

```
turn 0:  run(no inputs)                 → list [Cli.Stdin, Cli.Stdout, Process.Exit]; no firings
         reconcile: a Cli.Stdin slot exists → start reading stdin on a thread
turn 1:  chunk "one\ntwo\nthr"          → responses [(0, Chunk "one\ntwo\nthr")]; no firings
turn 2:  chunk "ee\n"                   → responses [(0, Chunk "ee\n")]; no firings
turn 3:  end of input                   → responses [(0, End)]
                                        → firings [(1, ["3\n"]), (2, [Code 0])]
         perform in list order: write "3\n" to stdout; exit 0
```

Three turns, three occurrences, and the exit code is the last thing the World did. A golden test for the CLI World is exactly this transcript: program, arguments, environment and standard input in; standard output, standard error and exit code out. Anything involving timers is tested under the Test World instead, with `Push Clock` and `Send Time.After(..), 30000`, because a golden test with a real clock is a flaky test.

### A REPL with a timeout

```prism
@Repl: @widget:
    !in <- @Element Cli.Stdin
    !idle <- @Element Time.After(requests=(30000 on (!started | !line)))
    !started <- @started
    let !line = !Filter (ChunkOf !in)
    let !end = !Filter (IsEnd !in)
    $n <- 0 then $n + 1 on !line
    @Element Cli.Stdout(requests=((Reply $n, !line) | "bye\n" on (!end | !idle)))
    @Element Process.Exit(requests=(Code 0 on (!end | !idle)))
    @ unit
```

The timeout restarts on every line, since each firing is a new timer. An old timer's response still arrives, and the program exits on it, which is a bug: the fix is the debounce pattern, a pending count, and the point of writing it this way first is that the bug is in the program's text, where a test under the Test World (`Send Time.After(..), 30000` after a line) finds it, and not in a race between a signal handler and a read loop.

### Where tokio goes

The CLI World runs its turn loop on the main thread and uses tokio, or plain threads, for reading standard input, waiting on signals, performing file reads and timers. Each of those tasks ends by pushing plain data onto the queue and waking the loop. No task ever touches the runtime, and no task holds anything but a path, a tag and some bytes. The async-runtime section below says why this split is the right one for every World, including the ones with a window.

## GUI: one vocabulary, spoken by many Worlds

GUI is not a language concept, and not a World concept either. Some Worlds can put pixels on a screen and read a pointer and a keyboard, and they offer tags for that. Layout, hit-testing, focus traversal, text editing, buttons, lists and scrolling are Prism, written as interceptors over those tags, in a library that runs unchanged under any World that offers them and under the Test World, which offers everything. The language has an opinion only about structure: leaves are elements, containment is interception, a World draws what it is given and delivers what it sees.

You have lost a cursor to a native text box, and you have fought a toolkit's focus manager for a field it insisted on focusing. Both happened because the toolkit owned state the program also had, and the two copies disagreed. The vocabulary below has no state in it that a program could also hold. A `Gui.Focusable` is a name and nothing else, so a World can show a focus ring; where the caret is, which field is focused and what Tab does are holds in Prism, and there is no second copy.

### The vocabulary

Five tags. A window, the picture in it, the name of the focused region, a way to measure text, and a tick per frame.

```prism
type Pointer = choice:
    Moved Point
    Down Point, Button
    Up Point, Button
    Scroll Point, Vector

type Key = choice:
    Char Char
    Named NamedKey                 # Left | Right | Backspace | Tab | Enter | ...
    Composition CompositionEvent   # IME: Start | Update Text, Nat | Commit Text | End

type WindowEvent = choice:
    Resized Extent
    CloseRequested
    Pointer Pointer
    Pressed Key

type Extent = record:
    w: Float
    h: Float

tag Gui.Window of Element:
    title: Text
    size: Extent                   # the size the program asks for when the window opens
    responds: WindowEvent

tag Gui.Scene of Element:
    window: Text                   # the title of the window it paints
    draw: [Draw]

tag Gui.Focusable of Element:   # presence: the name of the focused region
    name: Text

tag Gui.Metrics of Element:
    text: Text
    font: Font
    state: Extent                  # width and height of the text; answered synchronously

tag Gui.Frames of Element:
    responds: Frame                # `Frame` is the record declared in the Time section
```

The draw commands are not tags. They are plain values in the scene's `draw` list, and the list order is the paint order.

```prism
type Draw.Rect = record:
    rect: Rect
    color: Color

type Draw.Text = record:
    origin: Point
    text: Text
    font: Font
    color: Color
    ime_at: ?Nat                   # an offset into text where an input method's candidate window goes

type Draw.Image = record:
    rect: Rect
    image: ImageId

type Draw.Clip = record:
    rect: Rect
    draw: [Draw]

type Draw = Draw.Rect | Draw.Text | Draw.Image | Draw.Clip
```

A `Draw.Rect` is a rectangle on the screen and nothing else: it has no events, no name, no identity. A button's background is a `Draw.Rect` and the button is a library widget that knows where that rectangle is. The last line is a union of the four records and has no constructors of its own, which is why a draw command is never an element and never needs a path.

A window has a title and a size, both plain fields that rarely change, so its slot is stable for the life of the program. The World has one path to send pointer events and resizes to. The picture is a separate presence element, `Gui.Scene`, which is replaced whenever the picture changes: a fresh slot per frame is right for something nobody is waiting on, and keeping the picture out of the window is what lets `Gui.Window(title="Counter", ..)` stay one fact. The World diffs the new scene against the committed one, or redraws it whole. It decides which, because the language cannot see the difference.

The window answers with four things: the size the platform gave it, a close request, every pointer event over the whole window, and every key. The World does no hit-testing, and the reason is that a World that did would need to know what is clickable. Suppose it hit-tested against rectangles the program declared. Then every clickable thing would exist twice, once as the pixels the library drew and once as the rectangle the World matches, and the two would disagree on the frame where a button moves and its rectangle has not, which is the toolkit bug from the start of this section in a new place. It would also need an answer for overlap, for a scrolled list clipping a half-visible row, for a disabled button and for a pointer capture. The library has the layout and the clip regions and every one of those answers already, so the library hit-tests: the pointer position goes in, the widget path comes out, and the World needs only to know where the pointer is, which only it knows.

Keys go to the window, like pointer events, and the World does not choose who gets them. The library routes each `Pressed` key to the input whose name is the focus, so the World has one place to send keys and no focus order to keep. Which field is focused, what Tab does and where the caret is stay holds in the library, and focus is by name: the library's focus hold is a `Text`, and a rebuild that keeps the name keeps the focus. `Gui.Focusable` is presence only, a name and nothing else, and the library emits at most one at a time, the one it holds as focused. It hears nothing. It exists so that a World can show the platform's focus ring, place an input method's candidate window and tell a screen reader what has focus. Composition events from an input method arrive as keys, at the window. The World puts the candidate window at the `ime_at` offset of the focused `Draw.Text` run, which it finds in the committed scene when the platform asks; the World shapes text itself and so can turn an offset into a position, and the library does not tell it twice.

`Gui.Metrics` is the one state element in the vocabulary, and it answers the question every layout asks: how wide is this text? Its answer is an `Extent`, the width and height of the text in the font. You cannot place a caret or centre a label without it, and only the thing that has the fonts knows. As a request it would answer next turn and the text field would paint one frame with stale positions per keystroke. As state, its oracle is synchronous, the slot has its metrics in the occurrence that creates it, and the layout that depends on them is right in that occurrence. Value identity keeps it cheap: every slot asking about `("Start", Default)` shares one measurement. The font is a plain `Font` value in `Draw.Text` and in `Gui.Metrics`, and the World resolves it.

`Gui.Frames` responds with the time once per frame while a slot exists, for animation, and the World requests continuous redraw only while there is one.

Two things a GUI program seems to need are not in the vocabulary, a clipboard and a font set. A clipboard is a request and a response, `Clipboard` with `Read` and `Write Text`, offered as a bare tag by any World that has one. It is separate from `Gui.*` because it has nothing to do with drawing: a terminal World with access to the system clipboard offers it, and a GUI World on a platform without one does not, and `Unsupported` already says so. Installed fonts are not a tag at all. A program names a font in the value it draws, the World resolves the name, and anything that wants to enumerate fonts for a picker asks a separate state tag, which a World may offer and the five-tag vocabulary does not need. Both stay out for the same reason: each tag in `Gui.*` is something every World that draws pixels must implement, and these two are not.

### The GUI library

Above those tags, in Prism. The library has its own tags under `Ui`: `Ui.Label`, `Ui.Button` (responds `Click`), `Ui.TextInput`, `Ui.VBox`, and so on. They never reach a World; the library's interceptors claim them. `@Button` is a `Ui.Button` that the enclosing box turns into a `Draw.Rect` and a `Draw.Text` at the place it chose; `@TextInput` is a hold with the text and cursor, `Draw.Text` runs and a caret rectangle placed with a `Gui.Metrics` reading (chapter 2's version is the whole thing); `@VBox` is an interceptor that captures its children's `Ui.*` elements, assigns them rows, and re-emits one `Ui.VBox` with the layout; `@Window` is the interceptor at the top. It captures everything, lays it out using the metrics it asked for, emits one `Gui.Window` and one `Gui.Scene` with the whole draw list, hit-tests the window's pointer events against the layout it holds, and routes each hit inward as a `Click`, and each key to the input whose name is the focus. Focus is a hold in `@Window` moved by Tab and by clicks, and the library emits only the focused child's `Gui.Focusable` upward. Scrolling is a `Draw.Clip` and an offset. An accessibility tree, where a World can use one, is a field the library builds from the same element list it already holds, with node identities taken from `for` keys so that focus survives a rebuild that keeps the data's identity.

None of this reaches the World except as `Gui.Window`, `Gui.Scene`, `Gui.Focusable` and `Gui.Metrics` elements, and that is the division of labour: the World rasterises and delivers; the library decides.

### A GUI turn, end to end

The counter of chapter 2 under `@Window @VBox @counter`. `@Window` lays out the box, emits the window and the scene, and hit-tests pointer events by the layout it computed. The World is the software rasteriser, and its font has 8-pixel advances.

**Occurrence 0.** The counter builds its two buttons and a label; `@VBox` captures them and emits a `Ui.VBox` with the three as children; `@Window` captures that and asks for the width of the label's text with a `Gui.Metrics` slot, which has its value in this occurrence. It computes a layout (three rows of 24 pixels, the label centred) and emits:

```
0/0:        Gui.Window(title="Counter", size=Extent(w=200.0, h=72.0))
0/1/#0/0:   Gui.Metrics(text="Count: 0", font=Default)
0/2/#0/0:   Gui.Scene(window="Counter", draw=[[
              Draw.Rect(rect=((0, 0, 200, 24)), color=grey),
              Draw.Text(origin=((4, 4)), text="Increment", font=Default, color=black, ime_at=Nil),
              Draw.Rect(rect=((0, 24, 200, 24)), color=grey),
              Draw.Text(origin=((4, 28)), text="Decrement", font=Default, color=black, ime_at=Nil),
              Draw.Text(origin=((68, 52)), text="Count: 0", font=Default, color=black, ime_at=Nil)]])
```

The World reads the list, sees a window it has not opened, opens a 200×72 window, answers the metrics slot from its font (64 pixels wide), rasterises the scene into the window, presents, and waits. There are no regions in the list and no button anywhere in it: the World has a window to deliver pointers to and a picture to paint.

**Turn 1.** The user clicks at (50, 10). The World delivers `Pointer (Down ((50, 10)), Left)` as a response to `0/0`. `@Window`'s slot fires; the window interceptor hit-tests the point against the layout it holds, read through a tag, which is the layout as the occurrence opened, the one the user saw; row 0 is the `Ui.VBox`'s first child, the Increment button's path; it fires `{vbox_path: Pointer(row=0)}` down; `@VBox` hit-tests the row against its children's list as the occurrence opened and fires `{0/#0/0: Click}` down; the counter's `!increment` fires; `$count` becomes 1; the label rebuilds; the VBox's children change; the VBox rebuilds its element; `@Window` relays out. The label text is a plain field of the metrics slot, so the metrics are a fresh slot at generation `#1`, and its value (still 64) is in the oracle in this occurrence, so the new scene at generation `#1` has the right label position. One occurrence, no `Defer`. The window slot is untouched. The World diffs the scene, rasterises the new draw list, presents.

**Turn 2.** The user drags the window edge; the platform reports a new size. The World delivers `Resized Extent(w=300.0, h=72.0)` to `0/0`. `@Window` holds the size, relays out with wider rows and the label at x=118, and emits a scene at generation `#2`; the metrics are unchanged, so their slot stays. The World resizes its buffer and presents. On a platform that expects the paint before the resize callback returns, the World runs this turn inside the callback and presents before returning; one turn per input makes that a non-event.

**Turn 3.** The user presses a key. The World delivers `Pressed (Char 'x')` to the window at `0/0`, like any other input. There is no `Gui.Focusable` in the list, because the counter has nothing to type into, and `@Window` has no focused input to route the key to, so it drops it. Had there been a text input, `@Window` would have emitted `Gui.Focusable(name="input")` upward, the focused name it holds, so the World could show a focus ring; the key would still have arrived at `0/0`; `@Window` would have routed it to the input named `input`; and the input's hold would have moved its cursor, asked `Gui.Metrics` for the width of the text up to the cursor, and drawn the caret there. The World knows nothing about cursors.

Three turns, three occurrences, and the World's whole job was: open a window, rasterise a list, deliver a point, a key and a size to a path. Everything that looked like a toolkit was Prism, and ran unchanged under the Test World when `Send Gui.Window(..), Pointer (Down ((50, 10)), Left)` drove it.

### Software rasteriser

The smallest GUI World: one window, one scene, a pointer and a keyboard. The World opens a platform window, and after each turn takes the `Gui.Scene` element's draw list, rasterises it into a buffer with its own text rasteriser, and presents it. It answers `Gui.Metrics` from the same font, delivers every pointer event and every key to the window's slot. It shows a focus ring for the `Gui.Focusable` in the list, if any. It is a few hundred lines over a windowing library and a rasteriser, and it is the World the conformance suite's GUI programs run under first, because it has nothing in it that could be wrong except the obligations.

### A native toolkit wrapper

A World that wraps GTK, AppKit or Win32 offers higher tags: `Gtk.Button(label)` responding with `Clicked`, `Gtk.Entry(text)` responding with `Changed Text`, `Gtk.Box(children)`. The World diffs the list against the widget tree it keeps, creating, updating and removing native widgets. The same Prism program targets it through a different library: `@Button` becomes `@Element Gtk.Button(label=$_)` inside a `dyn`.

The hard case is the native text entry. Its cursor and selection live in the toolkit, and a fresh slot per keystroke (which is what a dynamic `text` field means) would lose them. The World's answer is continuity by tree position: a `Gtk.Entry` slot replaced by a `Gtk.Entry` slot at the same bind position and `for` key is the same native widget with its text updated, and the World says so in its documentation. The program cannot tell, and the language promises nothing about it. The alternative, a tag field declared as `$Text` so that the slot persists and the World sees the field change, would make the boundary see a sigil, and the leaf-plus-interceptor pattern has not yet produced a case that needs it.

Which is better, native widgets or a Prism library over draw primitives, is a library question with a platform answer: native widgets get the platform's own accessibility, input methods and look; a Prism library gets one behaviour everywhere and a text box whose cursor cannot be lost. A program can be written for either and some will be written for both.

### HTML and the web

The browser is the proof that the vocabulary works, because the DOM is it. A DOM node is a plain record with a tag and attributes; the browser diffs what you give it against what it has, lays it out, paints it, runs the input method and the screen reader, and delivers events addressed to nodes. A web World offers `Dom.Node(tag, attrs, text, children)` as its one container element, responding with `DomEvent(name, detail)`, and the whole GUI library of the previous sections becomes a different library that emits nodes instead of a `Gui.Scene`. Text editing keeps its state in Prism and puts out an editable node as its bridge: the browser's `beforeinput` and composition events (from memory) arrive as responses, Prism decides what they do to the text, and the World writes the node's text and selection back only when Prism's state differs from what the node last reported, and never during a composition, so there are no echo writes and no caret fights.

There are no threads in a browser page, and no loop to own. Each DOM listener, each `requestAnimationFrame` tick, each completed `fetch` is a callback, and each callback runs one turn. The World's input queue is the browser's event loop; the World's "block for input" is returning from the callback. Nothing about the run call changes, because it was synchronous and single-threaded from the World's view all along.

### A native Prism system: X11 over a socket

Take the vocabulary to its end. The World is a socket system: it accepts `Net.Socket` and nothing else, and it has never heard of a window. The program connects to `unix:/tmp/.X11-unix/X0`, and everything from there up is Prism: an interceptor that captures the GUI library's `Gui.Window`, `Gui.Scene` and `Gui.Focusable` elements, answers its `Gui.Metrics` slots, and emits bytes.

```prism
@X11: fn @a -> @a = \|@app| @widget:
    ((result, $slots)) <- intercept @app, !down, $ \|e| @Element e
    ((!rx, $link)) <- @Element Net.Socket(address="unix:/tmp/.X11-unix/X0", requests=!tx)
    $conn <- Handshaking then StepX11 $conn, !rx               # parse replies and events
    let !tx = Encode $conn, $slots, !rx                       # CreateWindow, PutImage, ...
    let !events = DecodeEvents $conn, $slots, !rx             # ButtonPress → Pointer to the window's path
    !fwd <- @Forward (NonGui $slots)
    let !down = !Union !events, !fwd
    @ result
```

`Encode` is a plain function from the connection state, the current element list and the latest bytes to the bytes to send: on the first list with a `Gui.Window`, a `CreateWindow` and `MapWindow`; on each change of the scene's draw list, a `PutImage` of the rasterised buffer, where the rasteriser is a pure native function `Rasterise: fn [Draw] -> Image` with its own font data and no time in it. `DecodeEvents` turns a `ButtonPress` at a point into a `Pointer` response addressed to the window's slot, and a key press into a `Pressed` response addressed to the same window slot, both read against `$slots` as the occurrence opened, which is the same routing law every interceptor obeys. The point is not hit-tested here, because the library above does that. The interceptor also supplies the `Gui.Metrics` slots the application asked for, from the same font data the rasteriser uses, so a label is measured with the glyphs it will be painted with. The World below knows it is writing bytes to a socket and reading bytes back.

This is not the way to build a desktop; it is the demonstration that the GUI vocabulary does not depend on the World having one, and that "the things that are native are the things that touch hardware" means what it says. A compositor, a terminal emulator and a window manager are each this interceptor with more state, and each can be tested under the Test World by sending it the bytes an X server would.

### IME and accessibility queries

A platform sometimes asks a question and blocks on the answer: where is the caret, so the candidate window can be placed; what is the text of this node, for a screen reader. The World answers from the committed list, where the GUI library keeps the answer current: the `ime_at` offset of the focused text run in the scene, the accessibility tree. That is the right answer for a query about inputs the platform already delivered, because each of those ran its own turn and the committed list reflects all of them. A World that wants the answer to include an input it has not yet delivered may run a turn inside the callback, delivering the query as a response to the window slot and reading the reply field afterwards; nothing forbids a World from running a turn whenever it likes, and the program sees an ordinary occurrence. A callback that arrives *during* a turn (the platform calling back from inside a frame change) is answered from the last committed list without starting anything, because the World never runs a turn inside a turn.

## Networks in depth

A socket is where every abstraction over IO shows its seams, because a socket has all the problems at once: bytes with no boundaries, a peer that can stall, a connection that can drop, and a handshake nobody wants to write. Here is each, and where it lives.

### Framing

`Net.Socket` moves bytes. `Received Bytes` is whatever the operating system handed the World, and the World does not promise that one send on the far side is one `Received` here. Framing is the program's: a hold that accumulates bytes and a plain function that splits off complete frames, in the `for` block or widget that owns the socket, as chapter 2's server accumulates a request head.

```prism
let SplitLines: fn Text -> (([Text], Text)) = ...    # complete lines, and the remainder

@Lines: fn Text, !Text -> @![Text] = \|address, !send| @widget:
    ((!rx, $link)) <- @Element Net.Socket(address=address, requests=(Encode !send))
    let !chunk = !Filter (TextOf !rx)
    let !split = SplitLines (Concat [[$rest, !chunk]])
    $rest <- "" then Second !split
    @ First !split
```

`Concat [[$rest, !chunk]]` reads the remainder through a tag, before the chunk, which is the remainder to append to; `SplitLines` returns the complete lines and the new remainder; the hold keeps the remainder and the widget returns the lines. The remainder and the lines come from one expression because there is no "after" inside an occurrence. A World could offer a `Net.Lines(address)` variant that responds with whole lines, and should, since it is a few lines of host code and saves every program the fold; it is a convenience, and the hand version is the real one.

One frame per occurrence or many? A chunk may hold three lines, and the widget above returns all three in one firing of `![Text]`. A program that wants each line in its own occurrence cannot have it from one response; it holds the lines in a queue and uses `Defer` to drain one per occurrence, or it asks the World for a `Net.Lines` element whose responses are one line each, in which case three lines are three turns. The second is what a World should offer, because the World can make the claim "these arrived separately" and the program cannot.

### Backpressure as state

`Connected n` in the link state says how many bytes are queued. A program that reads it at the moment of sending, through a tag, and declines to send when the queue is deep, has implemented flow control without a writable callback, a promise, or a `drain` event. The World updates the count as it writes, which is a change to a state element, which is a push at most once per turn. That granularity is right: a program does not need to know about every kilobyte, it needs to know whether to send now, and "now" is the occurrence.

The incoming side has no backpressure in the tag, and that is deliberate. A `Received` is a response, and responses are performed by the World in order and never dropped; a World that read faster than a program could consume would queue occurrences, which is the correct place for the queue, since each occurrence is cheap and the program's holds are the buffer. A World that wants to stop reading when a program's queue is deep can do so, and the program never knows.

### Reconnect by address change

The address is a field, so changing it is a `dyn` and a fresh slot, and the chat client of chapter 2 reconnects by typing. A reconnect *to the same address* is the interesting case: the state is shared by value, so a second slot with the old address keeps the old connection alive, and a program that wants a clean reconnect to the same host needs the element to differ. The field to add is a generation: `Net.Socket(address=a, attempt=n)`, with `n` bumped by the program on each retry, which is also where a retry policy lives, behind `Time.After`, in the program's text.

A World *may* reconnect on its own behind one address, since the link state has `Disconnected` and `Connecting` cases to tell the program about it, and the program's sends while `Disconnected` get `Closed` back. Whether it does is World policy; the tag's type makes either policy expressible to the program.

### TLS is a World concern

The address scheme says `tls://`, and the World does the handshake, the certificate check and the encryption, reporting `Connecting` until the handshake completes and `Disconnected "certificate ..."` if it fails. Nothing about TLS is in Prism, for two reasons that both come from the negative space. The language has no effects, and a TLS implementation wants randomness, which is an effect; it could be a state element the World pushes, but then the key material would be a value in a Prism hold, which is the wrong place for a secret, and the handshake would be a state machine over occurrences, which is a slow place for one. And the language is for programs that are "state, things happen, respond," not for computations; a cipher is a computation. So TLS is on the socket's far side of the boundary, with the kernel's TCP stack, and a program that needs to know which certificate it got asks a state element for it.

### One socket per address, or per slot

Two `Net.Socket(address="chat.example:7000")` slots in one program share one `Link` and have separate request and response streams. The World may keep one OS socket and multiplex the two slots' sends onto it in firing order, delivering every `Received` to both, which is what "one connection, two conversations" means; or it may keep two OS sockets and lie about the shared state, which it may not, since the equality law says the two slots agree on `Link` in every occurrence. One socket per address is the implementation that keeps the law for free, and it is the one the first World uses.

### A listener and its connections

`Net.Listener(address)` is a stream of `Accepted id`, and `Net.Conn(id)` is the connection, created by the program under a `for` keyed by the ids it was given. The World binds the address when a listener slot exists after a turn and unbinds it when none does, and it accepts only while a listener slot exists, so a program that wants to stop accepting removes the element (a `dyn` over a flag) and the World stops. That is reconciliation, not a trigger: a listener in the list twice is one bound socket.

A connection the World accepted for which the program never creates a `Net.Conn` slot is the World's to close, after a grace period it documents; a `Net.Conn(id)` slot for an id the World never allocated gets `Gone` as its state and `Closed` as its first response. The ids are the World's and the program only ever echoes them.

### HTTP

`Http.Fetch` is a request element and nothing more: fire a request record, get a result in a later turn, with non-2xx statuses as `Answered` (they are answers) and transport failures as `Failed`. Redirects, compression and connection pooling are inside the World. A program that wants a streaming body asks for `Http.Stream`, which responds with chunks and an end, and a World offers it when it can.

`Http.Server(address)` responds with `Incoming(id, request)`, and `Http.Reply(id)` is a request element the program fires once per id. A reply to an id the World is not waiting on is answered `Gone`; a second `Http.Reply(id)` slot for one id is a second conversation the World answers `Gone` too. Concurrent requests are several ids under one `for`, each with whatever outgoing calls it needs as its own slots, and the proxy above is the whole pattern.

## Async runtimes

A World built on tokio has a main loop that looks like this, and the point of showing it is where the run call sits in it.

```rust
enum Input {
    Response(Path, Value),             // a completion addressed to a slot
    Changed(ElementValue, Value),      // a state element's new value
}

async fn main_loop(mut core: Core, mut rx: mpsc::Receiver<Input>) {
    loop {
        // 1. Wait for the first input; then take everything else already queued.
        let first = match rx.recv().await { Some(i) => i, None => break };
        let mut batch = vec![first];
        while let Ok(more) = rx.try_recv() { batch.push(more); }

        // 2. One turn per response-bearing input (responses to one slot never share
        //    an occurrence); state changes ride along with the next turn.
        for inputs in core.partition(batch) {
            let outcome = core.turn(inputs);        // synchronous; never awaits
            core.dispatch(outcome).await;           // perform + reconcile: spawn tasks
        }
    }
}
```

The run call is synchronous and single-threaded from the World's view, and it is inside `core.turn`, which never awaits. Everything asynchronous happens in `dispatch`, after the turn: a `Read` firing becomes a `tokio::fs::read` task that ends by sending `Input::Response(path, Contents ..)` on the channel; a `Net.Socket` slot appearing becomes a connect task that sends `Input::Changed(socket, Connected 0)` and then a read loop that sends `Input::Response(path, Received ..)` per chunk; a `Time.After` firing becomes a `sleep` that sends `Input::Response(path, delay)`, echoing the number it was fired with. The tasks hold a path, an element value and a channel sender. None holds the runtime, the core, or anything that could reach either.

### Why synchronous, and why one thread

Suppose the run call were `async`, awaiting inside propagation when a response was needed. Then an occurrence could be suspended with holds uncommitted while another input arrived, and either the second input waits (so the await bought nothing) or it opens a second occurrence inside the first (which the transaction forbids). An occurrence is a transaction, and a transaction that yields halfway is the checkbox bug with a scheduler attached. And suppose the runtime were driven from several tokio worker threads. Then two occurrences could run at once, which is the parallelism the model allows across occurrences, but as transactional concurrency with read-set checking and retry, not as "whichever worker got there first." That is a runtime variant, described in the last section, and nothing a World does.

So the runtime is `Send + !Sync`: one owner, one thread, and the World's choice of which thread. A GUI World's main thread, because the platform says so. A CLI World's main thread, because there is no reason not to. A server World's one thread per shard, because that is the parallelism that costs nothing. tokio is the IO side of every World, never the propagation side. A World grown with tokio as its propagation loop would learn to await inside a turn and hold graph state across an await, and no GUI main thread allows either, so the World that was developed would not be the one users got.

### One occurrence per readiness event, or a batch

A readiness event (a socket readable, a timer expired, a read completed) becomes one input on the channel. When the loop wakes, several may be queued, and the World chooses. Responses to different slots that were waiting together may be one occurrence: the program sees them as simultaneous, which the World is entitled to claim, and a program whose text has a merge in it has already said what simultaneity means. Responses to one slot are always separate occurrences, in arrival order. State changes are coalesced to the last value per element and ride with the next turn, because state is a level.

Two policies are both conforming and feel different. *One turn per input* gives the program the finest view of order and the most occurrences; it is right for a CLI World and for anything with a transcript, because the transcript is unambiguous. *Batch what is queued* gives fewer occurrences and lower latency under load; it is right for a GUI World between frames, and for a server. A test World does what the script says, and the `Together` step is how a script says "batch these."

### Completions become responses, and vanished slots cancel

A task that completes sends `Input::Response(path, value)`. The core, at the start of the next turn, checks the path against the list it last received; if the slot is gone, the response is dropped and logged. The task itself does not know the slot is gone, so the World keeps, per in-flight operation, an abort handle keyed by path, and `reconcile` aborts the handle when the slot vanishes: an in-flight `fetch` is cancelled, a `sleep` is dropped, a socket is closed after its queued writes drain. A write that already landed stays landed; the World cannot un-write.

The same mechanism is why a `dyn` rebuild "reloads": the old slot's read is cancelled with the old slot and the new slot's `@started` fires a new one. Guessing that the two are the same read, by position, is how a half-typed draft ends up on the wrong row; the World does not guess.

### The browser, where there is no loop

A web World has no `main_loop`, because the page's event loop is not the World's to own. Every callback runs one turn: a DOM listener, a `requestAnimationFrame` tick, a resolved `fetch`. The channel is replaced by the browser's own queue, and "spawn a task" is `spawn_local` (from memory) of a future that ends by running a turn with its result. Nothing about `Core::turn` changes, and nothing about the obligations. A World that cannot block is still a World, because a turn never blocked.

## System calls and blocking operations

Some things block and have no asynchronous form worth using: `getaddrinfo` on some platforms, a synchronous filesystem on others, `fork`/`exec`, a database driver. The rule is that blocking happens on a thread inside the World, never inside the runtime, and never inside the oracle.

*Not inside the runtime*, because the run call is one thread doing one transaction, and a block there stalls every input behind it; in a GUI World it stalls the frame. A component that needs to block spawns a thread (tokio's `spawn_blocking`, from memory, or a plain `std::thread`) and sends the result on the channel like any completion.

*Not inside the oracle*, with one bounded exception. The oracle is the snapshot as a function, and it must answer the same value to every slot that asks in one occurrence. A World that implemented `Fs.Contents`'s oracle by reading the file on every ask would give two slots created in one occurrence two different values if the file changed between the reads, which breaks the equality law. So the oracle reads the snapshot map, and the one exception is the first ask for a local file: the component reads it then, under a size cap, and records the result in the snapshot before answering, so every later ask in every occurrence gets the recorded value and the change arrives as a push. Past the cap the answer is `Unavailable TooLarge`. Anything that waits on something outside the machine (a socket, a DNS lookup, a database) is never read in the oracle: its answer is the tag's unavailable case, and the component starts the work after the turn, when it sees the slot in the list.

The cap keeps the exception honest. Without it, the oracle would block on whatever size of file a program named, and the latency would land inside the occurrence; with it, the read is the kind of blocking a main thread does anyway, and a file past the cap gets the true frame, `Unavailable TooLarge`, and a program that minds draws the previous state instead, with a hold. The Test World has no disk, so it models the oracle with `State` steps, including `State Fs.Contents(path="/big"), Unavailable TooLarge`.

The same discipline applies to `Process.Spawn`: the spawn happens on a thread after the turn, the `Spawned id` comes back as a response, and the program that fired from `@started` sees it a turn later, never in the build.

## Cross-platform

A World is per platform; a program is not, if it stays inside the general components and the GUI vocabulary. The line between the two is drawn by tag namespace.

### Which components are per platform

The general components, `Fs`, `Time`, `Clock`, `Process`, `Net`, `Http`, `Errors`, `Defer`, and the GUI vocabulary, are offered by every World that can, with one meaning. Raw access is per platform and named so: `Sys.Posix` on Linux and macOS, `Sys.Win32` on Windows, `Sys.Wasi` on a WASI host, nothing on the web. A shared `Sys` that meant different things in different places would only pretend to port, and a program that uses `Sys.Posix` has said in its text that it runs on POSIX.

### What a program sees

A request on a tag the World lacks gets `Unsupported` in the next turn, which the program matches on: a `Clipboard` under a World with no system clipboard, a `Process.Spawn` in a browser. A state tag the World lacks is a build error in the creation occurrence, because a dynamic cannot exist without a value. That asymmetry makes a portable program's job clear: it may fire optimistically and handle `Unsupported`, but it must not create a state element it is not sure of. The way to be sure is a state element every World provides, alongside `Defer`:

```prism
tag World.Tags of Element:
    state: [Text]                  # the tags this World accepts, by name
```

A program reads it in occurrence zero (it is state, so it has a value then) and builds under a `dyn` over it, choosing a `Clipboard` path or a fallback by a plain `match`. An interceptor can supply a child a narrower `World.Tags`, which is what a sandbox should do so that the child never tries. The Test World's `World.Tags` is everything, and a test that wants to exercise the fallback sets it narrower with a `State` step. This is the one component that is about the World itself, and it is state rather than a request because the question "what can I ask" has to be answerable before anything is asked.

### Platforms

*Linux and Windows.* A main thread for the turn loop (the platform's event loop for a GUI World, a plain loop for a CLI World), tokio threads for IO, every general component native. Linux is X11 and Wayland under one windowing library; Wayland wants the IME cursor rectangle pushed rather than queried (from memory), which is the focusable's `rect` field written out at reconcile when it changes.

*macOS.* The event loop must be created on the main thread and the window system allows interface calls only there (from memory), so the turn loop lives on the main thread and the World's IO tasks wake it through the platform's proxy. Live resize puts the main loop into a tracking mode that expects synchronous layout and paint per size change (from memory); a resize is a response to the window slot, a turn, and a paint before the callback returns, which the one-turn-per-input loop does without special casing.

*Web.* No threads, no filesystem, no sockets; `Fs` is absent or offered over the origin-private filesystem, `Net.Socket` is absent or a WebSocket, `Http.Fetch` is `fetch`, the GUI vocabulary is the DOM. One turn per callback. `World.Tags` is how a program finds out.

*Mobile.* A GUI World with two more things: a lifecycle state element (`App.Lifecycle: Foreground | Background | Suspended`) the program reads and, since it is state, a late-built widget reads correctly; and a World that may be killed between turns, so that any state worth keeping goes through a store component (below). The turn loop runs on the platform's main thread, as on macOS.

*A server.* No screen; the general components plus `Http.Server`, `Net.Listener` and a store; and the parallelism question, next.

## Servers

A stateful service is "state, things happen, respond" with the screen removed, and the only new question is how many runtimes there are. A runtime is single-threaded and a run call is cheap, so the answer is: as many as there are independent things, and a World that routes between them.

### One runtime per connection, or per shard

A chat server has rooms. Two rooms share nothing: a message in one changes nothing in the other, and if they were one program, a `for` over rooms would build two subtrees connected only through the listener. So they can be two runtimes, each running the same program with a different `Process.Args`, on two threads, with a World that owns the listener and routes each accepted connection to the runtime for its room. From each runtime's side the World is ordinary: a `Net.Conn(id)` for each connection it was given, a `Store` for persistence, and `Defer`. From the World's side there are N cores, each with one runtime, each driven by its own thread, and a routing table from connection id to core.

```rust
struct Shard { core: Core, tx: mpsc::Sender<Input> }

async fn router(listener: TcpListener, shards: Vec<Shard>) {
    loop {
        let (sock, _) = listener.accept().await?;
        let room = read_room_header(&sock).await?;          // which program this belongs to
        let shard = &shards[hash(room) % shards.len()];
        shard.tx.send(Input::Response(listener_path(shard), Accepted(alloc_id()))).await;
        spawn_socket_tasks(sock, shard.tx.clone());         // bytes → Response(conn_path, Received ..)
    }
}
```

Per connection instead of per shard is the same design with N equal to the number of connections, and it is right when connections share nothing at all (a request-response protocol with per-connection state). Per shard is right when a shard's connections share a model, which is most servers. Nothing in the program changes between the two; the World's routing table does.

### Load balancing

A shard that is busy is a thread that is busy, and the World sees it as a channel that backs up. Rebalancing means moving a room from one runtime to another, which means tearing down its subtree in one program and building it in another, and that is a program-level question: the room's state lives in holds, and holds do not cross runtimes. So the room's state has to be in a store that both runtimes can reach, and the World tells the old runtime the room is gone (its `Process.Args`, or a `Shard.Rooms` state element, changes; the `for` tears the room down) and the new runtime that it has arrived. The model's state comes back from the store in the new runtime's first occurrence for that room, as state, because the store is a state element.

### Persistence via state elements

```prism
tag Store.Value of Element:
    key: Text
    state: Stored                  # Absent | Present Bytes

tag Store.Put of Element:
    key: Text
    requests: !Bytes
    responds: PutResult            # Stored | Failed Text
```

A room's model is `Store.Value(key="room:lobby")`, a state element with a value in its first occurrence (the World's snapshot, loaded from the store when the World started or when the key was first asked) and pushed when any runtime writes it. Writes are `Store.Put` firings. A model held in Prism is then a hold whose initial value is a `dyn` over the stored value's first reading, and whose changes are written back by firing `Store.Put` on them, which is the cache pattern of chapter 2 with the direction reversed. Two runtimes watching one key see one value per occurrence, each, because each World core keeps the equality law within its own runtime; across runtimes, "the same occurrence" has no meaning and the store's own version is the clock.

A host that instantiates a runtime per request (a WASI HTTP handler, from memory) is this with N equal to the number of requests and a lifetime of one turn. The program puts out `Http.Server`; the host delivers the request as the first response after the build; the program fires `Http.Reply`; the World sets the host's out-parameter and drops the runtime. State that outlives the request is in the store. A World whose life is one turn is still a World, because the turn was always the unit and the loop was always the platform's.

## Worlds that nest

A World can be written in Prism, and the proof is the shell of chapter 2: a program that runs two programs as its children, shows one, routes clicks to it, and forwards everything else. From each child's side, the shell is its World. From the real World's side, the shell is one program. The interface is the same at every level, and that is the point of `intercept` having exactly the World's two downward supplies and the World's one upward channel: a World and an interceptor are the same thing with a different host language.

### A shell

A process table is a `for` over a keyed map of programs. Each entry is an `intercept` of a child, with the child's `Cli.Stdin`, `Cli.Stdout` and `Process.Exit` held back and answered by the shell, and everything else forwarded. The shell's own `Cli.Stdin` is the terminal; it parses commands, builds children, pipes one child's `Cli.Stdout` firings into the next child's `Cli.Stdin` as responses (through a `Defer`, because a response is always later than the request it answers), and removes a child from the table when its `Process.Exit` fires. The outer World below the shell is a CLI World that knows nothing about the children: it sees one program with one standard input.

```prism
@Pipe: fn @unit, @unit -> @unit = \|@producer, @consumer| @widget:
    ((ignore, $ps)) <- intercept @producer, !down_p, $ \|e| @Element e
    ((ignore, $cs)) <- intercept @consumer, !down_c, $ \|e| @Element e
    !writes <- for $ps:                          # the producer's stdout firings, by its path
        dyn $_:
            match _.element:
                Cli.Stdout(..): @ _.!request
                ignore: @ !never
    let !out = JoinText "", (Values !writes)
    $seq <- 0 then $seq + 1 on !out
    !delivered <- for $queue:                    # one Defer slot per queued chunk
        !started <- @started
        !later <- @Element Defer(requests=(unit on !started))
        @ $_ on !later
    let !changes = !Union {$seq: Enqueue !out}, (MapValues !delivered, \|ignore| Dequeue)
    $queue <- {} then StepQueue $queue, !changes
    let !down_c = {StdinPath $cs: Chunk (JoinText "", (Values !delivered))}
    !fwd_p <- @Forward (Except (\|e| match e:
        Cli.Stdout(..): True
        Process.Exit(..): True
        ignore: False), $ps)
    !fwd_c <- @Forward (Except (\|e| match e:
        Cli.Stdin(..): True
        ignore: False), $cs)
    let !down_p = !fwd_p
    @ unit
```

The producer's write is captured by the first `for` (it never reaches the World) and queued under a sequence number. The second `for` builds a block per queued chunk, and each block has its own `Defer`, fired from `@started` in the occurrence the chunk was queued and answered in the next; the block then fires the chunk, the consolidated `!delivered` goes to the consumer's stdin slot, and the key leaves the queue. One `Defer` per delivery keeps the pipe inside the turn rule: a producer that writes in two occurrences of one turn (possible if it is itself fed by a `Defer`) makes two blocks and two `Defer` slots, each answered once, instead of one slot asked to answer twice. `StepQueue` folds `Enqueue` and `Dequeue` changes into the map, and the `!Union` means a write and a delivery landing in one occurrence are both applied. `StdinPath` is a plain function over the consumer's list, `Except` is `@Forward` with a filter, and the consumer cannot tell that its standard input is another program rather than a terminal, which is the test.

### A window manager

The shell with a screen. Each application is an `intercept` whose `Gui.Window` and `Gui.Scene` are held back and replaced by a part of one big scene: the manager wraps the application's draw list in a `Draw.Clip` at the application's offset, and the manager's own hit-test routes pointer events whose point falls in that frame to the application's window slot with the point translated. Focus is a hold in the manager moved by clicks and a key chord; keys go to the focused application's `Gui.Window` slot, and the application's own library routes them by focus name. The manager emits one `Gui.Window` and one `Gui.Scene` to the World below, and the World below is the software rasteriser, or the X11 interceptor over a socket World, or a native toolkit wrapper showing one native window. None of them knows there are several applications.

### Sandboxing third-party widgets

A date picker you downloaded has, in most languages, the same ambient access you do: `std::fs` is an import away. Here it has nothing: every file it wants is an element in its list, and that element reaches the World only if some interceptor above it makes one. Wrapping it in `@Sandbox` (chapter 2) confines its `Fs` to a directory and denies the rest, one occurrence after each request, and the widget's code does not change and cannot tell. The guidance for third-party widgets is: wrap them. The standard library ships `@Sandbox`, `@Confine` for a namespace list, and `@Deny` for everything, and a program that forgets is no worse off than any program in any language, while a program that remembers has something no other language gives: confinement per subtree, in the program, with no process boundary.

The thing a capability system has to get right is that nothing below the sandbox can reach above it except through the sandbox. Here that is structural: there is no global, no ambient World, no event bus, and the only upward channel is the element list, which the sandbox reads in full. What the sandbox must not do, as in any capability system, is turn the child's data into authority: a sandbox that forwards a file request with the child's path unchanged has given the child the filesystem.

### Which World is at the bottom

The outer World under a nested stack is smaller than the one you would have written without the stack, because the stack did the multiplexing. A shell needs a CLI World; a window manager needs a World with one window; a test needs the Test World. The language promises that an interceptor which re-emits everything and supplies by forwarding is observably not there, so each layer can be put in and taken out, and the children run under the Test World with the layer or without it, which is how a window manager is tested without a screen.

## Worlds and parallelism

The model has three kinds of parallelism, and a World is involved in exactly one of them.

*Within an occurrence.* Every node at one height is independent of every other at that height, so propagation can fork-join per height level. That is a runtime variant with `Sync` graph types, and a World drives it through the same run call on the same one thread. The World cannot tell.

*Across occurrences.* An occurrence is a transaction with buffered writes, so two occurrences can run optimistically and the one whose read set overlaps the other's write set retries. That is a runtime variant too, and from the World's view the run call still returns outcomes in order. The World cannot tell either, except that it may be asked to supply two occurrences' inputs before the first has returned, which changes the call's signature to a submit-and-collect pair and nothing else.

*Across the program.* Independent subtrees connected only through elements can be separate runtimes, each on its own thread, with a World that routes between them. This is the server above, and it is the only parallelism a World sees, because the World is the one doing it. It needs no runtime support beyond the run call being cheap, and the no-callback rule makes it free: a World with N runtimes is N single-threaded run calls and a routing table, and nothing crosses between runtimes but plain data through the World.

The World's own parallelism, its IO threads and tasks, is also free for the same reason. A World in which the runtime called back into World code would have to decide which thread the callback ran on and what it may touch; a World that is only ever called, by its own loop, after a turn, has no such question.

## Testing a World

The conformance suite tests the language under the Test World and says nothing about any other World. A World is tested separately, in its host language, against the world core, and the tests are the obligations turned into assertions. Most of them run against the core with fake components and no platform at all, which is why the core is a library: the loop, `Defer`, the snapshot and the discard rule are tested once, and a platform World tests only what the platform adds.

### The core, with fakes

| Test | What it catches |
| --- | --- |
| A response to a path not in the last list is dropped and logged, never passed to `run`. | A World addressing something the program does not have. |
| A `Defer` fired in occurrence k is answered in k+1 of the same turn; `Fired` on the child's side shows the answer in the turn. | `Defer` answered in its own occurrence, or deferred to the next turn. |
| A `Defer` that fires again after being answered ends the turn with an error naming its path. | Deferring backwards passing as a loop. |
| A `dyn` rebuilt by a `Defer` answer that fires a new `Defer` from `@started`, three times. | The generation rule not holding; a turn that never ends. |
| Random programs with D `Defer` slots: every turn runs at most 1 + D occurrences. | Any path to an unbounded turn. |
| A turn with two `Defer`s that fire in occurrence 1 runs three occurrences, and both answers land in occurrence 2. | Deliveries split that should land together. |
| Two components report changes to one value before a turn; the turn has one push with the last value. | State pushed twice in an occurrence; pushes not coalesced. |
| A slot created in the occurrence of a push ends the occurrence with the pushed value, and a tag read in that occurrence sees the oracle's value. | The oracle answering from Sₙ; the push not reaching a new slot. |
| Two slots with equal elements created in different occurrences hold equal values in every occurrence. | The equality law broken by the core's bookkeeping. |
| The oracle is called for a value no component owns: `UnsuppliedState` in the creation occurrence, with the span. | A state tag silently defaulting. |
| A request on an unclaimed tag gets `Unsupported` next turn; a presence element on an unclaimed tag is logged once. | Portable code waiting forever. |
| A request fired by a slot torn down in the same occurrence is handed to `perform` with its path; its response, if any, is discarded. | Firings lost because the slot went; responses to nothing. |
| A component's `reconcile` sees a slot appear once however many occurrences the turn had, and never sees one that appeared and vanished inside a turn. | Resources opened per occurrence; resources opened for slots the outside never saw. |
| `run` returns `Err`; the turn's inputs are dropped; the next turn's pushes are the diff from the last committed snapshot. | A failed turn's inputs replayed; a snapshot that lost the world's movement. |
| The same inputs, twice, from a fresh core: identical outcomes. | Anything in the core that depends on time or thread order. |

### A platform World

Each World adds the tests only its platform can run. For the CLI World: golden runs with fixed arguments and standard input (an echo, the line counter, a program that writes to both streams in one occurrence with the order kept, `Process.Exit` fired in the same turn as writes with the writes landing first, `SIGINT` with and without a `Cli.Signals` slot, a closed standard-output pipe producing `WriteFailed`, a `Cli.Stdin` slot dropped mid-read and rebuilt, with the chunk read in between delivered to the new slot first, a program with no live sources exiting zero). For a GUI World: a scripted click at a point reaching the right widget through the library's hit-test, a resize turn that paints before the callback returns, a composition sequence reaching the window as `Composition` keys with the candidate window placed at the `ime_at` offset read from the committed scene, a `Gui.Frames` slot inside a `dyn` starting and stopping frame requests. For a server World: concurrent requests under one `for`, one of them waiting on an outgoing call, with the replies landing under the right ids, and a routing World with two shards that never deliver each other's connections.

What none of them tests: the language. A World test never asserts on a hold or a widget's result; it asserts on what crossed the boundary, in both directions, and on what the World did with it.

## Negative space

What a World may not do, and what each refusal buys. The list is the design.

**No callbacks across the boundary.** The runtime never calls the World, and the World never hands the runtime a function (the oracle is called during one run call and forgotten). What it buys: the World runs on whatever thread the platform dictates, with IO wherever it likes, because nothing can reach the runtime from the wrong thread; a World is replaceable without the program noticing; a session is a transcript of plain data and replays exactly; the Test World exists.

**No runtime objects in a World.** No event handle, no dynamic, no node, no slot. What it buys: a World written in an afternoon, with no lifetime to track and nothing to leak; a World that cannot address what the program has not told it exists, because it has only paths from a list it was given; the sharded server, where a runtime moves to a thread and nothing follows it.

**No phases.** One "later," the next occurrence of this turn, through `Defer`. What it buys: a turn bounded by the program's own structure rather than a configured cap; no ordering table for stages; no stage-dependent behaviour that differs between Worlds; a GUI library that runs the same under a test.

**No presence triggers.** Nothing happens because an element appeared or a widget was dropped. The list is a description the World reconciles to; one-shot effects are request firings. What it buys: reconciliation is idempotent, so a rebuild costs exactly what it asks for and never a surprise fetch; the World never watches for appearances; exit, writes and spawns are events a program can merge, gate and mock.

**No World-side sampling of the program.** The World reads the list after a turn and nothing during one; there is no "what is `$count` right now" for a World any more than for a widget. What it buys: the occurrence is atomic from outside, so a half-finished turn is never drawn, written or answered.

**No dynamic fields in elements.** An element is born with its fields and keeps them; a change is a fresh slot. What it buys: the boundary never sees a sigil, value identity means what it says, and the World diffs lists of plain data instead of subscribing to anything.

**No policy in the World that a program might want to choose.** Retries, caches, book-building, framing, focus, layout: Prism. What it buys: every policy is in the program's text, testable under the Test World, and replaceable per subtree by an interceptor; the World stays the size of its hardware.

**No scheduler, no priorities, no budget.** The World performs firings in the order the program fired them and runs occurrences in the order inputs arrived. What it buys: two Worlds reading one outcome do the same things; the transcript is the specification of the run.

**No clock inside.** Time is a state element the World pushes. What it buys: a debounce is a test with a `Push`; a transcript replays with the same timestamps; a program cannot ask for a finer time than the World gave it and so cannot depend on one.

The pattern is the language's own: each thing a World cannot do is a thing that would have made the boundary a second set of rules, and each removal is also a mechanism the World never has to build. A World is small because the list above is long.

## What a World is, in one paragraph

A World is a host-language program that owns a runtime and a loop. It builds `@main` in occurrence zero. It keeps a snapshot of every state value it has supplied and answers the oracle from it. It turns what the outside does into responses by path and pushes by value, runs one synchronous occurrence per turn plus one per fired `Defer`, performs the request firings in order, reconciles the world to the element list, and waits. It discards what it cannot address and reports what it cannot run. It accepts some tags and answers `Unsupported` for the rest. Everything else, from a button to a window manager, is Prism, and would be the same Prism under any other World that accepted the same tags.
