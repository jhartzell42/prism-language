# Chapter 13 — Rust Worlds and the world core

A World is the only part of a Prism system with effects, and it is written in Rust against one synchronous call: plain data in, plain data out, once per occurrence. Everything a World must get right that is not specific to a screen, a terminal or a socket is the same for every World, so it is written once, in a crate called `prism_world`, and a World is that crate plus a handful of components. This chapter gives the crate's design as code: the component trait, the turn loop, the `Defer` bookkeeping, the snapshot and push machinery, slot tracking, `Unsupported`, error reporting, and the harness that lets any event loop drive a runtime. It then builds four Worlds on it, in increasing order of how much of the outside they touch: the Test World, which has no effects and answers to a script; the CLI World, which runs Prism against a real process; a GUI World that puts pixels in a window; and a World that offers nothing but sockets, under which a Prism library speaks X11 and the same GUI programs run anyway. The chapter ends with the tests: the core's conformance tests against a fake component, and the CLI World's golden transcripts.

## Where a World stands

You have had the year where the framework handed you a context object. You stashed it in a static, or threaded it through every constructor, and it worked, and then a test needed to run without a browser and could not, because half the program had learned to reach into that object whenever it wanted something. The context was a door, and once there is a door, everything walks through it.

Prism has no door. A program describes what it wants as elements, plain values in a list; the outside sends plain values back, addressed by path; and the only thing that ever happens between them is one function call. The World calls the runtime; the runtime never calls the World. The World holds no runtime object apart from the runtime itself, and the runtime holds nothing of the World's. This makes the one-way-ness of the language real at the edge: a program cannot reach the World because there is nothing to reach, and a World cannot reach into a program because the run call takes values and returns values.

The run call, as the Rust runtime exposes it, is the whole surface a World sees:

```rust
pub struct Inputs<'a> {
    /// Responses by path, for slots in the list the World last received.
    pub responses: Vec<(Path, Value)>,
    /// New values for state elements, keyed by element value.
    pub pushes: Vec<(ElementValue, Value)>,
    /// The oracle: the value a state element has as this occurrence opens.
    /// `None` means no component accepts the tag, which aborts the occurrence.
    pub oracle: &'a mut dyn FnMut(&ElementValue) -> Option<Value>,
}

pub struct Outcome {
    /// The element list after the occurrence, in tree order.
    pub list: Vec<(Path, ElementValue)>,
    /// For each slot with a `requests` field that fired: its firings, in order.
    /// Includes slots torn down in this occurrence, so the World can still act.
    pub requests: Vec<(Path, ElementValue, Vec<Value>)>,
    /// The program's result, when the World reads it.
    pub result: ResultView,
}

impl Runtime {
    pub fn run(&mut self, inputs: Inputs<'_>) -> Result<Outcome, RuntimeError>;
}
```

`Path` is opaque to a World except for equality and ordering; `ElementValue` is an interned tag plus plain fields with structural equality; `Value` is the runtime's value type restricted to plain data, which the compiler guarantees for anything that goes into an element. `ResultView` is the program's result in a form a Test World can compare: a plain value, or an event's firing in this occurrence, or a dynamic's current value. A CLI World never looks at it.

That is all. There is no `subscribe`, no `on_event`, no handle to a node. The first call is occurrence zero, with empty responses and pushes; it builds `@main` and returns the initial list. Everything a World does from then on is a function of what came out of the last call and what the outside has done since.

### What the crate refuses to contain

The shortest way to say what `prism_world` is for is to say what it leaves out, because each omission is a decision with a reason.

*No scheduler.* The crate never decides which of two things to do first inside the runtime; that order is fixed by heights and the runtime owns it. The crate orders only the things outside: inputs into occurrences, and request firings into effects.

*No threads of its own.* The core is single-threaded and blocks nowhere. It runs on whatever thread calls it. Worlds that need threads, for reading stdin or waiting on a socket, create them in their components, and those threads talk to the core through one inbox of plain values. Nothing on another thread ever sees the runtime.

*No callbacks into the runtime.* A component is called by the core; it never calls the runtime, never holds a `Runtime`, never holds a `Path` beyond the turn in which it was told the slot exists, except to address a delivery. The trait makes this structural: nothing in a component's signature mentions the runtime.

*No phases.* A turn is an input occurrence followed by whatever `Defer` occurrences it causes, and nothing else. A turn has no named stages for layout, focus and painting: they would be a second ordering mechanism with its own rules for who runs when, and everything they buy is expressible with `Defer` and ordinary dynamics. A GUI library that wants "layout, then focus" writes two `Defer`s in sequence.

*No budget.* A turn is bounded by the `Defer` rule, not by a counter. There is no runaway report to design, no partial-commit policy, no "yield to the platform and continue next frame," because the `Defer` rule bounds the turn: each `Defer` slot answers at most once per turn, so a turn runs at most one occurrence more than there are `Defer` slots. A `Defer` that a rebuild brings back counts as a new slot, which is the one gap in that argument, and the `Defer` section below works through it.

*No GUI.* The crate has no notion of a window, a draw list or a pointer. A GUI World is a World whose components happen to accept tags under `Gui`, exactly as a CLI World's accept tags under `Cli`. The core does not know the difference.

What is left is a few hundred lines: a trait, a loop, three maps and a printer. That is the point. A World should be an afternoon's work, so that there will be several, and the way to make it an afternoon is to put every obligation of the specification in one place that every World gets for free.

## The `prism_world` crate

### The types

The core speaks in the runtime's plain-data types and adds a few of its own.

```rust
pub use prism_runtime::{Path, ElementValue, Value, Tag, RuntimeError, Span};

/// What a tag declares, from the compiled program's tag table.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TagKind {
    pub requests: bool,
    pub responds: bool,
    pub state: bool,
}

/// Something that wants to enter the program at the start of an occurrence.
pub enum Delivery {
    /// A response addressed to a slot by path.
    Response { path: Path, value: Value },
    /// A new value for a state element, keyed by element value.
    Push { element: ElementValue, value: Value },
    /// No payload: a thread sends it only to wake the loop, and the harness drops it.
    Wake,
}

/// Which occurrence of which turn, for error reports and transcripts.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct When {
    pub turn: u64,
    pub occurrence: u32,
}
```

`Wake` carries nothing into the program; a component's thread sends it into the inbox so that the loop wakes and asks `has_gatherable`.

The compiled program brings its tag table with it, so the core knows for any tag whether it requests, responds or has state. That table lets the core decide, for an element nobody accepts, which of the three `Unsupported` behaviours applies.

### The Component trait

A component is a thing that accepts some tags and does, in Rust, what those tags mean. Its contract with the core is eight methods, and each one follows from an obligation in the specification.

```rust
pub trait Component {
    /// Which tags this component accepts. Asked once, at registration.
    fn tags(&self) -> Vec<Tag>;

    /// A slot with one of this component's tags appeared in the list the World
    /// read after a turn. Streams start here; watches start here.
    fn slot_created(&mut self, path: &Path, element: &ElementValue, cx: &mut Cx);

    /// The slot is gone from the list the World read after a turn. Streams
    /// stop here; in-flight operations are cancelled where the OS allows.
    fn slot_removed(&mut self, path: &Path, element: &ElementValue, cx: &mut Cx);

    /// A slot fired its request. Called once per firing, in occurrence order
    /// and then list order, after the turn. The response, if the tag responds,
    /// goes through `cx` and reaches the program in a later turn.
    fn perform(&mut self, path: &Path, element: &ElementValue, request: Value, cx: &mut Cx);

    /// The value a state element has right now. Called at most once per
    /// element value per turn; the core memoises. Must answer: unavailability
    /// is in the tag's type, never in a failure here.
    fn oracle(&mut self, element: &ElementValue, cx: &mut Cx) -> Value;

    /// Called at the start of every turn. The component reports state changes
    /// since the last turn as pushes through `cx`, and may deliver anything
    /// else it has ready. A clock pushes here, once per turn.
    fn gather(&mut self, cx: &mut Cx);

    /// True when `gather` would produce something that should start a turn on
    /// its own: a watch that saw a change, a timer that expired, held stdin
    /// input. A clock answers false. The default is false.
    fn has_gatherable(&self) -> bool { false }

    /// True when nothing this component does can ever deliver again:
    /// no stream running, no operation in flight, no timer set.
    fn quiescent(&self) -> bool;
}
```

`has_gatherable` lets the core ask, before it opens a turn with an empty batch, whether any component has pending work for `gather` to collect, so a component whose input arrives from a thread (standard input, a file watch) can cause a turn without a delivery of its own.

`Cx` is the component's one way to say anything to the core, and it is write-only:

```rust
pub struct Cx {
    deliveries: Vec<Delivery>,            // for the next turn
    diagnostics: Vec<Diagnostic>,         // things to report, not errors
    exit: Option<i32>,                    // a request asked the run to end
    inbox: Inbox,                         // a clonable sender for other threads
}

impl Cx {
    pub fn respond(&mut self, path: &Path, value: Value);
    pub fn push(&mut self, element: &ElementValue, value: Value);
    pub fn diagnostic(&mut self, d: Diagnostic);
    pub fn exit(&mut self, code: i32);
    /// A sender that any thread can use to deliver later, plus a waker.
    pub fn inbox(&self) -> Inbox;
}

#[derive(Clone)]
pub struct Inbox {
    tx: std::sync::mpsc::Sender<Delivery>,
    wake: std::sync::Arc<dyn Wake>,
}

pub trait Wake: Send + Sync {
    /// Called from any thread after a delivery is queued. A CLI World's waker
    /// does nothing (the loop blocks on the channel); a winit World's calls
    /// `EventLoopProxy::send_event`; a tokio World's notifies a `Notify`.
    /// (winit and tokio API names from memory.)
    fn wake(&self);
}
```

Why a trait with these eight methods and not something else?

Suppose a component were an async trait: `async fn perform(...) -> Value`, and the core awaited it. Then the core would be an async function, and the turn would hold the runtime across an await. Every GUI main thread forbids that, and even where it is allowed it means the runtime's state is live while the World is doing something else, which is the door again by another name. The split here is that `perform` returns at once, having started the operation, and the result comes back through the inbox as a `Delivery` for a later turn. A component that wants async machinery, and the tokio World does, spawns a task and hands it the inbox. The core never waits.

Suppose instead the core were table-driven: a component registers, per tag, a closure for requests and a closure for the oracle, and the core calls closures. This is the same trait with the names erased and the state scattered. The trait wins because a component has state that spans the methods (the stdin reader's thread handle is created in `slot_created` and joined in `slot_removed`; the timer wheel is written by `perform` and read by `gather`), and a struct with methods is the Rust way to say that.

`oracle` most needs its contract spelled out. It is called by the core from inside the runtime's run call, because the runtime asks for a state element's value at the instant a slot is created, mid-build. So `oracle` must be synchronous and must not block for long. A file watch component answers `Fs.Contents(path)` by reading the file, which is a blocking syscall, and that is accepted: a read of a local file is the kind of blocking a GUI thread does anyway, and the alternative, answering `Unavailable` and pushing the contents next turn, makes every program's first occurrence see a value it will never see again. The rule a component follows: answer from what it has if it has it, read synchronously if the read is cheap (a local file under a size cap), and answer the type's unavailable case, with its reason, when the real answer cannot be had without waiting on something outside the machine or is past the cap.

### Registering components and the World struct

```rust
pub struct World<R: Runtime> {
    runtime: R,
    program: Program,                  // compiled widget plus tag table
    components: Vec<Box<dyn Component>>,
    by_tag: HashMap<Tag, usize>,       // which component accepts which tag
    snapshot: Snapshots,
    slots: SlotTable,
    defer: DeferTable,
    when: When,
    inbox_rx: std::sync::mpsc::Receiver<Delivery>,
    cx: Cx,
    report: Box<dyn Report>,           // where errors and diagnostics go
}

impl<R: Runtime> World<R> {
    pub fn new(program: Program, report: Box<dyn Report>, wake: Arc<dyn Wake>) -> Self;
    pub fn add(&mut self, component: impl Component + 'static) -> &mut Self;
    /// Occurrence zero. Returns the initial list.
    pub fn start(&mut self) -> Result<TurnOutcome, WorldError>;
    /// One turn. The inputs are whatever the caller has gathered.
    pub fn turn(&mut self, inbound: Vec<Delivery>) -> Result<TurnOutcome, WorldError>;
    /// True when no component can deliver again and the inbox is empty.
    pub fn quiescent(&self) -> bool;
    pub fn inbox(&self) -> Inbox;
}
```

Two components claiming one tag is an error at `add`, reported before the program runs. `Defer` is claimed by the core itself and a component that tries to accept it is refused for the same reason.

`TurnOutcome` is what a caller gets back from a turn:

```rust
pub struct TurnOutcome {
    pub when: When,                              // the turn number and its last occurrence
    pub occurrences: Vec<OccurrenceRecord>,      // one per occurrence of the turn
    pub list: Vec<(Path, ElementValue)>,         // after the last occurrence
    pub exit: Option<i32>,                       // a component asked to end the run
}

pub struct OccurrenceRecord {
    pub when: When,
    pub inputs: Vec<Delivery>,                   // what opened it
    pub requests: Vec<(Path, ElementValue, Vec<Value>)>,
    pub result: ResultView,
}
```

A CLI World looks at `exit` and nothing else. A Test World reads all of it. The records exist so that the transcript printer and the `requested`, `fired` and `errors` steps have a single source, the same one the differential test diffs.

### The turn loop as code

A turn is: take what has arrived, open an occurrence with it, and keep opening occurrences while `Defer` requests are outstanding; then act. Here it is, with the bookkeeping calls named and defined in the sections that follow.

```rust
impl<R: Runtime> World<R> {
    pub fn turn(&mut self, mut inbound: Vec<Delivery>) -> Result<TurnOutcome, WorldError> {
        self.when.turn += 1;
        self.when.occurrence = 0;
        self.defer.begin_turn();

        // 1. Components report what changed since last turn (clock, watches).
        for c in &mut self.components { c.gather(&mut self.cx); }
        inbound.extend(self.cx.take_deliveries());

        // 2. Sort inbound into this occurrence's responses and pushes.
        //    Responses to paths not in the last list are discarded with a
        //    diagnostic; two responses to one path split into two turns.
        let (mut responses, pushes, held) = self.slots.sort_inbound(inbound, &mut self.cx);
        self.hold_over(held);                          // becomes next turn's inbound
        let mut delivered = self.snapshot.deliverable(&pushes);   // only values someone has asked about

        // 3. The input occurrence, then Defer occurrences until none fire.
        //    An abort stops the loop; the occurrences before it have committed.
        let mut records = Vec::new();
        let mut failure = None;
        loop {
            self.when.occurrence += 1;
            let first = records.is_empty();
            let outcome = match self.run_occurrence(responses, std::mem::take(&mut delivered)) {
                Ok(outcome) => outcome,
                Err(e) => {
                    if first { self.requeue(&pushes); }       // the pushes never landed
                    failure = Some(e);
                    break;
                }
            };
            if first { self.snapshot.apply_pushes(&pushes); }   // committed: next = prev + pushes
            self.snapshot.swap();                               // prev = next
            records.push(OccurrenceRecord::from(&outcome, self.when));
            self.slots.last_list = outcome.list.clone();
            match self.defer.collect(&outcome, self.when) {
                Ok(deferred) if deferred.is_empty() => break,
                Ok(deferred) => responses = deferred,           // answer the Defers; no pushes
                Err(e) => { failure = Some(e); break; }         // deferring backwards
            }
        }

        // 4. Act on what committed: slot diff, then requests in occurrence order
        //    then list order. Then, if the turn stopped early, report why.
        let (created, removed) = self.slots.diff();
        for (path, el) in &removed { self.route(el).slot_removed(path, el, &mut self.cx); }
        for (path, el) in &created { self.route(el).slot_created(path, el, &mut self.cx); }
        for rec in &records {
            for (path, el, firings) in &rec.requests {
                for v in firings { self.perform(path, el, v.clone()); }
            }
        }
        self.report.diagnostics(self.cx.take_diagnostics());
        if let Some(e) = failure { return Err(e); }
        Ok(TurnOutcome { when: self.when, occurrences: records,
                         list: self.slots.last_list.clone(), exit: self.cx.take_exit() })
    }

    fn run_occurrence(&mut self, responses: Vec<(Path, Value)>, pushes: Vec<(ElementValue, Value)>)
        -> Result<Outcome, WorldError>
    {
        let World { runtime, snapshot, components, by_tag, cx, program, .. } = self;
        let mut oracle = |e: &ElementValue| snapshot.answer(e, |e| {
            let ix = *by_tag.get(&e.tag)?;
            Some(components[ix].oracle(e, cx))
        });
        runtime.run(Inputs { responses, pushes, oracle: &mut oracle })
            .map_err(|e| WorldError::Runtime { error: e, when: self.when })
    }
}
```

Slot removal is told to components before slot creation. A `dyn` rebuild tears down a slot at one path and creates another at a fresh path in the same occurrence; if the component were told about the creation first, a stdin reader would briefly have two slots and start a second reader thread before stopping the first. Removals first means a component never sees two of a thing that should be one.

Requests are performed after the slot diff. A request fired by a slot that was torn down in the same occurrence is still in `requests`, with its path, and `perform` is called for it. The component does the operation if it can, because the program asked, and discards the response, because the slot is gone; `slot_removed` was already called, so the component knows. This is the rule that a write fired in the occurrence that removed the writer still lands.

Pushes land in the first occurrence of a turn only. The `Defer` occurrences that follow are the same turn, and the snapshot did not change between them, so their oracle answers from what is now the previous snapshot and their push list is empty. A clock, which pushes once per turn, is therefore read at the same instant by every occurrence of the turn, which is what makes "a turn is one step" true for state as well as for effects.

The snapshot takes those pushes only after the first occurrence commits. Picture a turn that opens with a click and the clock's push `Clock ↦ Instant 5100`, and the click's occurrence aborts on a cycle. The runtime rolls back, and no `Clock` slot ever saw 5100. Had the snapshot already taken the push, the next turn would open with the snapshot at 5100 and no push pending, so a new `Clock` slot would start at 5100 while every old one still held 5000, with nothing on the way to bring them up. So `deliverable` only picks which pushes to send, `apply_pushes` runs after `run_occurrence` returns `Ok`, and an abort in the first occurrence hands every push back with `requeue`, as inbound for the next turn. The click's response is not re-queued: it was the input to an occurrence that did not happen, and sending it again would run the same cycle again. A push is a fact about the outside, and it is still true next turn.

An abort later in the turn is a different case, because the occurrences before it committed. Picture a turn whose first occurrence fires a `Cli.Stdout` write of `"saved\n"` and a `Defer`, and whose `Defer` occurrence hits a cycle. The write's occurrence happened: its holds committed, and the list the World now holds is the one it produced. If the loop returned the error with `?` from inside, the write would never be performed, and the program would go on believing it had said "saved" to a terminal that never heard it. So the loop collects each occurrence's record as it commits, stops at the abort, performs the firings of everything that committed in the usual order, and only then returns the error. Deferring backwards stops the turn the same way: the occurrence in which a `Defer` fired a second time did commit, so its firings are performed too, and only the answer that would have opened the next occurrence is withheld.

### Defer bookkeeping

`Defer` is the one element the core implements itself:

```prism
tag Defer of Element:
    requests: !unit
    responds: unit
```

A slot that fires `Defer` in occurrence k of a turn is answered in occurrence k+1 of the same turn. The core's job is to find the firings, deliver the answers, and enforce the one rule that bounds the turn: a slot is answered at most once per turn, and a slot that fires again after being answered is deferring backwards.

```rust
#[derive(Default)]
pub struct DeferTable {
    answered: BTreeSet<Path>,     // slots answered so far this turn
}

impl DeferTable {
    pub fn begin_turn(&mut self) { self.answered.clear(); }

    /// The responses for the next occurrence: `unit` to every Defer slot that
    /// fired in this one and is still in the list. An error if any of them
    /// was already answered this turn.
    pub fn collect(&mut self, outcome: &Outcome, when: When)
        -> Result<Vec<(Path, Value)>, WorldError>
    {
        let mut next = Vec::new();
        for (path, el, firings) in &outcome.requests {
            if el.tag != Tag::DEFER || firings.is_empty() { continue; }
            if !outcome.list.iter().any(|(p, _)| p == path) {
                continue;                                  // torn down with the slot: nothing to answer
            }
            if !self.answered.insert(path.clone()) {
                return Err(WorldError::DeferBackwards { path: path.clone(), when });
            }
            next.push((path.clone(), Value::UNIT));
        }
        Ok(next)
    }
}
```

The bound follows from `answered`: every occurrence after the first in a turn is opened by at least one newly answered `Defer` slot, and no slot enters `answered` twice, so a turn has at most one occurrence more than the program has `Defer` slots. There is no counter and no cap, and nothing to tune.

The error is the interesting half. Picture a sandbox that denies a request by firing a `Defer` and answering `Denied` when it comes back, and picture a child that, on `Denied`, immediately asks again. The child's second request fires in the occurrence that delivered the denial; the sandbox's `Defer` fires again in that same occurrence; and the core finds a path already in `answered`. That is deferring backwards, and it is reported, with the path and the occurrence, as an error that stops the turn. The alternative, letting the `Defer` answer again, would be a loop with no bound, and the program that wrote it would hang the World rather than see its mistake on the first turn. A program that wants to retry waits for the next turn, which means waiting for any real input, which is the correct meaning of "later" when the thing being retried was denied by policy.

A `Defer` that fires in the occurrence that tore down its slot is skipped, which is the general rule for request firings from gone slots applied to a request the core itself answers: nothing to answer, nothing to report. The sandbox of chapter 2 does this when its child is removed in the same occurrence that the child asked for something.

The creation rule says why `Defer` is a request element with a response and not, say, a state element the World toggles. A request fires in n and is answered in a later occurrence; that is exactly the gap an interceptor needs, and it is the gap every other request has, so `Defer` costs the language nothing new. The one thing special about it is that every World must provide it, because an interceptor cannot start an occurrence and there has to be some way to get one.

### A `Defer` reborn by a rebuild

The bound above counts slots, and a program can make new `Defer` slots faster than the turn uses them up. On the merits, this part of the core needs more work. Here is the smallest program that shows it:

```prism
@Pulse: fn Int -> @!unit = \|n| @widget:
    !s <- @started
    !later <- @Element Defer(requests=(unit on !s))
    @ !later

@main:
    $n <- 0 then $n + 1 on !later
    !later <- @Pulse $n
    @ unit
```

`@Pulse $n` lifts to a `dyn` over `$n`. The build fires the `Defer` at `1/#0/1`. The next occurrence answers it; `!later` fires; `$n` becomes 1; the `dyn` rebuilds in that same occurrence, and the new `@Pulse` fires its own `Defer` at birth, at `1/#1/1`. That path is not in `answered`, so the core answers it in the next occurrence, which rebuilds again and fires at `1/#2/1`. Every slot is answered once, exactly as the rule asks, and the turn never ends. The program is wrong in the same way the retrying sandbox child is wrong (it answers "later" by asking for "later" again), but `DeferTable` keys by full path, and a full path includes the generation, so the core never sees the repeat.

The candidate fix is to key `answered` by the path with its generation segments stripped, so that a `Defer` reborn at the same tree position counts as the same `Defer`. `1/#0/1` and `1/#1/1` both become `1/#/1`, the second firing is deferring backwards, and the error names both paths. The change is a few lines in `collect`, and it keeps the bound without a counter.

What it might get wrong is a `Defer` that really is a different one. Picture a `dyn` over the selected tab, where each tab's widget puts a `Defer` at the same bind position: tab A's `Defer` is answered in occurrence 2, its answer switches to tab B, and tab B's `Defer`, born in occurrence 2 for a layout pass of its own, fires at birth. Stripped, the two keys are equal, and a program that ends after one more occurrence is reported as deferring backwards. The fix trades a turn that never ends for an error on a program that was fine, and the tab case is ordinary GUI code. Requiring the element value to match as well as the stripped path would separate tab A's `Defer` from tab B's only if their fields differ, and a `Defer` has no fields.

A `for` key is the other edge. Stripping removes generations and leaves `[k]` segments alone, so a queue whose rows are keyed by a fresh id, where each row's answered `Defer` appends the next row and removes itself, escapes the bound the same way the `dyn` did: `0/[1]/1`, `0/[2]/1`, and on. Stripping `[k]` too would close that and break something worse: ten rows that each defer once in one occurrence would share one key, and the second of them would be reported as deferring backwards in an occurrence where nothing went backwards.

Suppose instead the core capped a turn at some number of occurrences. That ends every one of these loops and is exactly the budget this crate refuses, with its runaway report and its question of what to do with the occurrences that did commit. The fix that belongs here identifies a reborn `Defer` the way a reader of the program would, by where it sits and what caused it, and neither full paths nor stripped paths capture that.

### Snapshots, the oracle and pushes

The problem the snapshot solves is a race you have met in any framework with subscriptions. A widget mounts, reads the current value of something, and subscribes to its changes; between the read and the subscribe the value changed, and the widget is now showing a value it will never be told is stale. The language's answer is that the read and the subscription are one thing, a dynamic, and the World's answer is that a dynamic's value as an occurrence opens and the changes delivered in that occurrence come from one consistent pair of snapshots.

```rust
pub struct Snapshots {
    prev: BTreeMap<ElementValue, Value>,   // what the program saw as the occurrence opened
    next: BTreeMap<ElementValue, Value>,   // what it will have seen when it closes
}

impl Snapshots {
    /// The pushes the turn's first occurrence delivers. A push for a value nobody
    /// has asked about yet is not delivered: there is no slot to deliver it to,
    /// and the first slot to ask will get it from the oracle.
    pub fn deliverable(&self, pushes: &[(ElementValue, Value)]) -> Vec<(ElementValue, Value)> {
        pushes.iter().filter(|(e, _)| self.prev.contains_key(e)).cloned().collect()
    }

    /// Called once the turn's first occurrence has committed, and not before:
    /// every push, delivered or not, is recorded as the value.
    pub fn apply_pushes(&mut self, pushes: &[(ElementValue, Value)]) {
        for (e, v) in pushes { self.next.insert(e.clone(), v.clone()); }
    }

    /// The oracle. Memoised: one answer per element value per occurrence.
    /// For a value never seen, the component is asked, and the answer is recorded
    /// in both snapshots so that no push for it is pending.
    pub fn answer(&mut self, e: &ElementValue,
                  ask: impl FnOnce(&ElementValue) -> Option<Value>) -> Option<Value>
    {
        if let Some(v) = self.prev.get(e) { return Some(v.clone()); }
        let v = ask(e)?;
        self.prev.insert(e.clone(), v.clone());
        self.next.insert(e.clone(), v.clone());
        Some(v)
    }

    pub fn swap(&mut self) { self.prev = self.next.clone(); }
}
```

Walk the rebuild rule through it. In turn n the clock component pushes `Clock ↦ Instant 5100`; `apply_pushes` puts it in `next` and, because `Clock` is in `prev` from an earlier ask, keeps it as a push. Occurrence n opens with that push. A stopwatch that was already running has a `Clock` slot; its dynamic fires with `5100` and its hold commits. A stopwatch built in n, inside a `dyn` that rebuilt because of a click in the same occurrence, creates a `Clock` slot mid-build; the runtime asks the oracle; `prev` says `5000`; the slot's dynamic starts at `5000`, then receives the push `5100` in the same occurrence, and ends n at `5100` like its older sibling. Both read `5000` through any tag in n, both are `5100` after. One snapshot per occurrence, answered the same to everyone, which is the equality law stated at the boundary.

Now suppose the oracle answered from `next` instead. The new slot would start at `5100`, receive the push `5100`, and look fine; but a tag read in n by anything that depends on the new slot's dynamic would see `5100` while a tag read through the old slot would see `5000`. Two readings of one state element in one occurrence, different. The equality law is the reason the oracle answers from the previous snapshot, and the push is what brings the new slot up to date within the occurrence.

The `clone` in `swap` is the simple version. A World with a large snapshot (a file watcher over a tree) would keep `next` as a delta over `prev` and swap by applying it; the maps are keyed by element value, so a push for one file touches one entry, and the delta is the push list. Nothing in the contract changes.

### Slot tracking and discarding late results

The World sends responses only to paths in the list it last received, and a result for a slot that has gone is discarded. Both are one table:

```rust
pub struct SlotTable {
    last_list: Vec<(Path, ElementValue)>,          // what the World read after the last turn
    known: BTreeMap<Path, ElementValue>,           // the same, as a map
}

impl SlotTable {
    /// Splits inbound deliveries into this turn's responses and pushes.
    /// A response to an unknown path is discarded with a diagnostic.
    /// A second response to one path in the same batch is held for the next turn:
    /// a slot receives one response per occurrence, and the batch did not say they
    /// were simultaneous in any sense the program could use.
    pub fn sort_inbound(&self, inbound: Vec<Delivery>, cx: &mut Cx)
        -> (Vec<(Path, Value)>, Vec<(ElementValue, Value)>, Vec<Delivery>)
    {
        let mut responses = Vec::new();
        let mut pushes = Vec::new();
        let mut held = Vec::new();
        let mut seen = BTreeSet::new();
        for d in inbound {
            match d {
                Delivery::Push { element, value } => pushes.push((element, value)),
                Delivery::Response { path, value } => {
                    if !self.known.contains_key(&path) {
                        cx.diagnostic(Diagnostic::LateResult { path, value });
                    } else if !seen.insert(path.clone()) {
                        held.push(Delivery::Response { path, value });
                    } else {
                        responses.push((path, value));
                    }
                }
            }
        }
        (responses, pushes, held)
    }

    /// After a turn: which slots appeared and which went, against the list now stored.
    pub fn diff(&mut self) -> (Vec<(Path, ElementValue)>, Vec<(Path, ElementValue)>) {
        let now: BTreeMap<_, _> = self.last_list.iter().cloned().collect();
        let created = now.iter().filter(|(p, _)| !self.known.contains_key(*p))
                         .map(|(p, e)| (p.clone(), e.clone())).collect();
        let removed = self.known.iter().filter(|(p, _)| !now.contains_key(*p))
                         .map(|(p, e)| (p.clone(), e.clone())).collect();
        self.known = now;
        (created, removed)
    }
}
```

A late result is not an error. The World started a read because a slot asked for it; the slot went away before the read finished; the result arrives. Nobody is listening, and that is the correct outcome of the program's decision to stop listening. The core makes it visible: `Diagnostic::LateResult` goes to the report, where a developer can see that a read was wasted, and the World cancels the operation at `slot_removed` time if the OS lets it. The core does not deliver it, because delivering a response to a path the program's list does not have is the one protocol error the runtime would reject with an abort, and the core's job is to never let a World commit it.

Paths are never reused, so a late result cannot be mistaken for a result to a new slot at the same position. A `dyn` that rebuilds gives the new slot a fresh generation in its path, and a result for the old generation is late by construction.

The "two to one path" case is the other half of routing. A reader thread that delivered two chunks of stdin before the loop woke up has produced two responses for one slot. They cannot share an occurrence: a slot receives one response per occurrence, and the runtime enforces that with an error. The core holds the second for the next turn, which is what a World that ran a turn per chunk would have done anyway. A World that wants two inputs in one occurrence, a the comma, has to address two different slots, and the Test World's the comma step checks exactly this before it delivers.

### Unsupported

A program that was written for a GUI World and run under a CLI World has `Ui.Button` slots at the root, and no component accepts `Ui`. Three things can be in that position, and they get three answers, each chosen so that the program sees a fact rather than silence.

```rust
impl<R: Runtime> World<R> {
    fn perform(&mut self, path: &Path, el: &ElementValue, request: Value) {
        match self.by_tag.get(&el.tag) {
            Some(&ix) => self.components[ix].perform(path, el, request, &mut self.cx),
            None => {
                let kind = self.program.tags.kind(&el.tag);
                if kind.responds {
                    self.cx.respond(path, Value::unsupported());
                }
                self.cx.diagnostic(Diagnostic::UnsupportedRequest {
                    path: path.clone(), tag: el.tag, chain: self.chain_of(path) });
            }
        }
    }

    fn route(&mut self, el: &ElementValue) -> &mut dyn Component {
        match self.by_tag.get(&el.tag) {
            Some(&ix) => &mut *self.components[ix],
            None => &mut self.nobody,     // a component that records and does nothing
        }
    }
}
```

*A request tag* nobody accepts gets an `Unsupported` response in the next turn, if the tag responds. `Unsupported` is a value of every response type, which the compiler arranges: every `responds: R` declaration is checked as `R | Unsupported`, and a `match` on a response needs an arm for it or a catch-all. A program under the wrong World therefore sees its request answered, with a value that says so, and can show "not available here" rather than wait forever. The response is in the next turn and not the same occurrence, because it is a response, and the creation rule binds the core like anyone else.

*A presence tag* nobody accepts, `Ui.Label` under the CLI World, is reported once per slot at `slot_created`, through the `nobody` component, and otherwise ignored. There is nothing to respond to and nothing to deny. The report names the path, the tag and the chain of interceptors the element came through, so that the developer sees "a `Ui.Label` reached the CLI World from `@vbox` inside `@main`" and not a bare tag name.

*A state tag* nobody accepts cannot be deferred to the next turn, because a state element has a value in the occurrence it is created, and no value is available. The oracle closure in `run_occurrence` returns `None` for it, and the runtime aborts the occurrence with `RuntimeError { kind: ErrorKind::UnsuppliedState, spans, .. }`. The transaction rolls back; the World reports the error with the slot's source span; the responses that opened the occurrence are dropped, and its pushes go back to wait for the next turn. That is harsh and it is right: a program that reads a state the World does not have cannot run under that World, and the earliest honest moment to say so is the moment the slot would have been born.

The interceptor chain in these reports comes from the path. A path is the chain of bind positions from the root, and the compiled program's binding table maps each position to a source span and the name of the widget function it is in; `chain_of` walks the path and collects the widget names. It is the same walk that gives a cycle error its spans.

### Error reporting

An error from the run call is a value, and the core turns it into text in one format, so that every World's errors read the same and a transcript's `errors` step has one thing to match.

```rust
pub enum WorldError {
    Runtime { error: RuntimeError, when: When },
    DeferBackwards { path: Path, when: When },
    DuplicateTag { tag: Tag, first: &'static str, second: &'static str },
}

pub enum Diagnostic {
    LateResult { path: Path, value: Value },
    UnsupportedRequest { path: Path, tag: Tag, chain: Vec<Name> },
    UnsupportedPresence { path: Path, tag: Tag, chain: Vec<Name> },
    Component { tag: Tag, message: String },      // a component's own trouble: EPIPE, a watch that failed
}

pub trait Report {
    fn error(&mut self, e: &WorldError, program: &Program);
    fn diagnostics(&mut self, ds: Vec<Diagnostic>);
}
```

The printer, `prism_world::report::Text`, writes an error as a header line, one line per span, and one line saying when:

```
error[cycle]: an event depends on itself within one occurrence
  --> stopwatch.prism:31:5   `!action`
  --> stopwatch.prism:34:5   `$watch`
  --> stopwatch.prism:31:5   `!action`
   in turn 7, occurrence 2 (answering Defer at 0/3/1)

error[unsupplied-state]: no component supplies `Gui.Metrics(text="Start", font=Default)`
  --> gui.prism:112:9   `$m <- @Element Gui.Metrics(text=label, font=Default)`
   via @Button, @vbox, @main
   in turn 0, occurrence 1

error[defer-backwards]: Defer at 0/#0/2 fired again after being answered this turn
  --> sandbox.prism:40:29  `!later <- @Element Defer(requests=(unit on _.!request))`
   in turn 12, occurrence 3
```

Spans come from the compiled program: the runtime names every cyclic placeholder and every slot after its source binding, so a runtime cycle reports Prism source and not runtime internals. The "when" line is what a developer needs to reproduce the error under the Test World, where turn 7 of a script is a line number.

Diagnostics are printed in a shorter form, one line each, to the same sink, prefixed `note:`. A CLI World sends both to stderr; a GUI World could show them in a window; the Test World keeps them in the transcript.

What the World does with an error is the World's policy, and the core leaves it to the caller of `turn`: the `Err` comes back after the firings of every occurrence that committed before the abort have been performed, the aborted occurrence has rolled back, its responses are gone and any pushes it was opened with are re-queued, and the caller decides whether to log and continue, or exit. A CLI World exits nonzero. A GUI World might keep running, because the window is still there and the next click is a fresh occurrence. A Test World records it for the `errors` step and continues the script, so that one test can check both the error and what the program does after it.

### The harness: driving a runtime from any event loop

Every World has a loop, and no two have the same one: a CLI World blocks on a channel, a winit World is called back by the platform, a tokio World selects over futures, a browser World is a callback with no loop at all. The core offers the harness so that each of those can drive the same turn with the same four lines.

```rust
pub struct Harness<R: Runtime> {
    world: World<R>,
    inbox_rx: std::sync::mpsc::Receiver<Delivery>,
    held: Vec<Delivery>,                     // responses held over from a previous batch
    policy: BatchPolicy,
}

pub enum BatchPolicy {
    /// One delivery per turn. The CLI World's choice.
    OneAtATime,
    /// Everything queued when the turn starts, subject to one response per path.
    /// A GUI World's choice for simultaneous inputs it can truthfully call simultaneous.
    Drain,
}

pub enum Step {
    /// Nothing was queued; the caller should wait for a wake.
    Idle,
    /// A turn ran.
    Ran(TurnOutcome),
    /// A component asked to end the run, after this turn's effects were performed.
    Exit(i32),
    /// Nothing can ever cause another occurrence.
    Quiescent,
    Error(WorldError),
}

impl<R: Runtime> Harness<R> {
    pub fn new(world: World<R>, policy: BatchPolicy) -> Self;
    pub fn inbox(&self) -> Inbox;

    /// Occurrence zero, then the effects of the initial list. Call once.
    pub fn start(&mut self) -> Step;

    /// Run at most one turn with whatever has arrived. Never blocks.
    pub fn step(&mut self) -> Step {
        let mut batch = std::mem::take(&mut self.held);
        match self.policy {
            BatchPolicy::OneAtATime => {
                if batch.is_empty() {
                    match self.inbox_rx.try_recv() { Ok(d) => batch.push(d), Err(_) => {} }
                }
            }
            BatchPolicy::Drain => {
                while let Ok(d) = self.inbox_rx.try_recv() { batch.push(d); }
            }
        }
        if batch.is_empty() && !self.world.has_gatherable() {
            return if self.world.quiescent() { Step::Quiescent } else { Step::Idle };
        }
        match self.world.turn(batch) {
            Err(e) => Step::Error(e),
            Ok(out) => match out.exit {
                Some(code) => Step::Exit(code),
                None => Step::Ran(out),
            },
        }
    }

    /// Block until something arrives, with a deadline for the next timer.
    pub fn wait(&mut self, until: Option<Instant>) {
        let d = match until { Some(t) => t.saturating_duration_since(Instant::now()),
                              None => Duration::MAX };
        if let Ok(d) = self.inbox_rx.recv_timeout(d) { self.held.push(d); }
    }
}
```

`step` never blocks and never waits, so it can be called from inside a platform callback: a winit `window_event` handler pushes the event into the inbox as a `Delivery` and calls `step` before returning. `wait` is for Worlds that own their thread, and takes a deadline so that a timer component can say when it next needs a turn. The `Wake` trait connects a thread that delivered something to the loop that should call `step`: a CLI World's waker does nothing, because the loop is blocked on the channel and the send wakes it; a winit World's calls the event loop proxy; a tokio World's notifies a `Notify` that the select is waiting on.

The CLI World's whole main is then:

```rust
fn main() -> ExitCode {
    let program = prism_lang::compile_file(&path)?;
    let mut world = World::new(program, Box::new(report::Text::stderr()), Arc::new(NoWake));
    let inbox = world.inbox();
    world.add(process::Process::new(args, env))
         .add(cli::Stdio::new(inbox.clone()))
         .add(cli::Signals::new(inbox.clone()))
         .add(fs::Files::new(inbox.clone()))
         .add(time::Timers::new(inbox.clone()));
    let mut h = Harness::new(world, BatchPolicy::OneAtATime);
    if let Step::Exit(c) | Step::Error(_) = h.start() { return exit_code(c) }
    loop {
        match h.step() {
            Step::Ran(_) => {}
            Step::Idle => h.wait(h.world().next_deadline()),
            Step::Exit(code) => return ExitCode::from(code as u8),
            Step::Quiescent => return ExitCode::SUCCESS,
            Step::Error(e) => return ExitCode::FAILURE,
        }
    }
}
```

That is the entire difference between the CLI World and any other at the level of the loop. Everything that makes it a CLI World is in the components.

A turn is caused by a delivery, and a push gathered from a component is a delivery the core finds by asking. The clock pushes once per turn, but a clock alone never causes a turn, because a program that only looks at the clock has nothing to react to; the turn that reads the clock is caused by something else. So `has_gatherable` asks components whether they have something that should start a turn on its own (a file watch that saw a change, a timer that expired), and the clock answers no. The distinction keeps a program with a `Clock` slot from spinning.

## The Test World

The Test World is the world core plus one component that answers to a script. It has no effects; it accepts every tag; it is the only source of inputs; its snapshot is whatever the script says. It is also the World the conformance suite runs under, in both implementations, so its transcript format is an interchange format and its steps are a specification of what a World can observe.

### The script, in `lang`

A test is an item in a Prism file, parsed by the `lang` crate like any other:

```prism
test counter_counts, @counter:
    build
        => expect Ui.Label(text="Count: 0")
    click Ui.Button(label="Increment")
        => settles
    click Ui.Button(label="Increment")
        => expect Ui.Label(text="Count: 2")
    click Ui.Button(label="Increment"), click Ui.Button(label="Decrement")
        => expect Ui.Label(text="Count: 1")
```

The parser produces a `TestItem { name, widget: Expr, turns: Vec<Turn> }` where the blueprint expression is elaborated and compiled as if it were `@main`, and each turn is one line of the script with its `=>` lines:

```rust
pub struct Turn {
    pub withs: Vec<(Pattern, Expr)>,     // `with p = v`, applied before the inputs
    pub inputs: Vec<Input>,              // `build`, `idle`, or one or more deliveries
    pub checks: Vec<Check>,              // one per `=>` line
}

pub enum Input {
    Build,
    Idle,
    Click(Pattern),
    Send(Pattern, Expr),                 // Expr: a plain value, evaluated at test time
    Push(Pattern, Expr),
}

pub enum Check {
    Expect(Pattern),
    ExpectTop(Pattern),
    ExpectNone(Pattern),
    ExpectTopNone(Pattern),
    ExpectCount(Pattern, usize),
    Returns(Pattern),
    Fired(Pattern),
    Quiet,
    Requested(Pattern, Pattern),
    Unrequested(Pattern),
    Errors(Pattern),
    Settles,
}
```

A `Pattern` here is an element pattern as the language already has it for `match`: a tag with named fields, each field a value pattern or `..`, or `Ns.*(..)`. The value in a `send` or `with` is an ordinary plain expression, type-checked against the tag's declared response or state type, so `send Ui.Button(..), "hello"` is a compile error in the test file. This is why the script lives in `lang` and not in a separate test language: it reuses the pattern grammar, the type checker, and the error messages, and a test file is a Prism file.

The `requested` step's second argument is a pattern over the request value rather than a value, so that `requested Time.Every(..), ignore` can say "it fired, I do not care with what".

### Execution

```rust
pub struct ScriptComponent {
    snapshot: BTreeMap<ElementValue, Value>,     // from `with` entries; nothing else supplies state
    accepted: Vec<Tag>,                           // every tag the program declares
}

impl Component for ScriptComponent {
    fn tags(&self) -> Vec<Tag> { self.accepted.clone() }
    fn slot_created(&mut self, _: &Path, _: &ElementValue, _: &mut Cx) {}
    fn slot_removed(&mut self, _: &Path, _: &ElementValue, _: &mut Cx) {}
    fn perform(&mut self, _: &Path, _: &ElementValue, _: Value, _: &mut Cx) {}   // recorded by the core
    fn oracle(&mut self, e: &ElementValue, cx: &mut Cx) -> Value {
        self.snapshot.get(e).cloned().unwrap_or_else(|| {
            cx.diagnostic(Diagnostic::Component {
                tag: e.tag, message: "no State step for this element".into() });
            Value::missing()
        })
    }
    fn gather(&mut self, _: &mut Cx) {}
    fn quiescent(&self) -> bool { true }
}
```

The component is almost empty, and that is the design: a Test World does nothing, so its component does nothing, and everything the steps need is in the `TurnOutcome` the core already returns. `Value::missing()` is a sentinel the core turns into an `UnsuppliedState` error, so a state element with no `with` step fails the test at the slot's creation with the same error a real World gives for a state tag it does not accept. Tag declarations carry no default state value, so there is nothing to fall back on: a program that creates `Clock` or `Gui.Metrics` while it builds needs a `with` step for that element before the build, and a test that forgets one fails with an error that names the element instead of running against a value nobody chose.

The runner holds the harness and the script and walks the turns:

```rust
pub struct TestRun {
    harness: Harness<RustRuntime>,
    script: ScriptComponent,            // shared with the World through Rc<RefCell>
    last: Option<TurnOutcome>,          // the last turn, for Fired/Quiet/Requested
    last_error: Option<WorldError>,     // for Errors
    transcript: Transcript,
}

impl TestRun {
    pub fn run(mut self, turns: &[Turn]) -> Result<Transcript, Failure> {
        for (i, t) in turns.iter().enumerate() {
            self.turn_line(t).map_err(|why| Failure { turn: i, why })?;
        }
        Ok(self.transcript)
    }

    fn turn_line(&mut self, t: &Turn) -> Result<(), String> {
        for (p, v) in &t.withs { self.script.set(find_state(p)?, eval(v)); }
        let list = self.last_list();
        let mut batch = Vec::new();
        let mut seen = BTreeSet::new();
        for input in &t.inputs {
            match input {
                Input::Build => self.record(self.harness.start()),      // the build reads this line's withs
                Input::Idle  => {}
                Input::Click(p) | Input::Send(p, _) => {
                    let path = find_one(&list, p)?;
                    if !seen.insert(path.clone()) {
                        return Err(format!("two deliveries to {} on one line", path));
                    }
                    let value = match input { Input::Send(_, v) => eval(v), _ => Value::unit() };
                    batch.push(Delivery::Response { path, value });
                }
                Input::Push(p, v) => {
                    let e = find_state(p)?;
                    self.script.set(e.clone(), eval(v));
                    batch.push(Delivery::Push { element: e, value: eval(v) });
                }
            }
        }
        if !matches!(t.inputs[..], [Input::Build]) { self.turn(batch)?; }
        for c in &t.checks { self.check(c)?; }
        Ok(())
    }

    fn check(&mut self, c: &Check) -> Result<(), String> {
        let list = &self.last_list();
        match c {
            Check::Expect(p)        => (find_all(list, p).len() >= 1).then_some(()).ok_or(no_match(p)),
            Check::ExpectTop(p)     => (list.iter().any(|(_, e)| p.matches(e))).then_some(()).ok_or(no_match(p)),
            Check::ExpectNone(p)    => find_all(list, p).is_empty().then_some(()).ok_or(unexpected(p)),
            Check::ExpectTopNone(p) => (!list.iter().any(|(_, e)| p.matches(e))).then_some(()).ok_or(unexpected(p)),
            Check::ExpectCount(p, n) => (find_all(list, p).len() == *n).then_some(()).ok_or(count(p, *n)),
            Check::Returns(p)       => p.matches_result(&self.last_result()).then_some(()).ok_or(..),
            Check::Fired(p)         => self.last_turn_fired(p).then_some(()).ok_or(..),
            Check::Quiet            => (!self.last_turn_fired_any()).then_some(()).ok_or(..),
            Check::Requested(p, vp) => self.last_turn_requested(p, vp).then_some(()).ok_or(..),
            Check::Unrequested(p)   => (!self.last_turn_requested_any(p)).then_some(()).ok_or(..),
            Check::Errors(p)        => self.last_error.as_ref().filter(|e| p.matches_error(e)).map(|_| ()).ok_or(..),
            Check::Settles          => self.last_error.is_none().then_some(()).ok_or("the turn ended in an error".into()),
        }
    }

    fn turn(&mut self, batch: Vec<Delivery>) -> Result<(), String> {
        self.harness.push_all(batch);
        match self.harness.step() {
            Step::Ran(out) => { self.last_error = None; self.record_turn(out); Ok(()) }
            Step::Error(e) => { self.last_error = Some(e.clone()); self.transcript.error(&e); Ok(()) }
            other => Err(format!("unexpected {:?}", other)),
        }
    }
}
```

The Test World's harness uses `BatchPolicy::Drain`, because a script says exactly what is in a turn and nothing else is queued.

`find_all` is the depth rule. It walks every slot in the list, matches the pattern against the element, and then descends into any field whose value is an element or a collection of elements, because that is the convention a GUI library uses to hold children as data in a container element. The Test World knows the convention as a structural fact about values, not as a tag name: a field of type `Element`, `[Element]` or `k ==> Element` is descended into. `Send` to a nested element delivers to the containing root slot's path with the nested index attached in the response, following the same library convention the other way; chapter 2's vertical box shows the Prism side of it.

A `with` takes effect as its own line's push and oracle: the `with`s on the `build` line run before the build, and the build occurrence reads them; `set` writes the script's snapshot, which the oracle reads for values not yet asked about, and the next turn's `gather` reports the changed values as pushes for values already asked about. `push` is `with` plus a turn with the push as its only delivery.

### How `with`, `push`, `requested` and `errors` are implemented

Each of the four that the walkthroughs in chapter 2 lean on has one source of truth in the core.

`with` and `push` are the snapshot. A `with Clock = Instant 5000` before the first turn means the oracle answers `5000` the first time any `Clock` slot asks, in occurrence zero, so a stopwatch built in the build occurrence has its clock in the build occurrence, which is the rule that state is there in its first occurrence. A `push Clock = Instant 5100` later is a `Delivery::Push`, which `apply_pushes` keeps because `Clock` is in `prev`, and which the runtime fires on the shared state dynamic for that value, reaching every `Clock` slot in the program and any `Clock` slot created in the push's occurrence. The script does not know how many slots there are; it sets one value for one element, which is value identity made visible.

`requested` is the `requests` vector of the last turn's occurrence records. `requested Time.Every(..), ignore` finds a slot in any occurrence of the last turn whose element matches and whose firings contain a value matching the second pattern. It looks at the whole turn because a request fired in a `Defer` occurrence is part of what the World saw after the turn, and a script that could only see the first occurrence would be seeing something no World sees. A request from a slot torn down in the same occurrence is in the records too, with its path, so a test can check that a write fired by a widget on its way out was performed.

`fired` and `quiet` are the `result` view of the same records: `fired` passes if the widget's result event fired in any occurrence of the turn with a matching value, `quiet` if it fired in none. `returns` compares the current value after the turn.

`errors` is the `Err` from `turn`. The runner keeps running after an error, with the runtime unchanged (the occurrence rolled back), so that `errors Cycle(..)` followed by `expect Ui.Label(text="Count: 2")` can assert that the label still shows what it showed before the bad occurrence. The pattern matches on the error's kind and, optionally, on the paths and spans in it, so a conformance test for "deferring backwards is an error" is:

```prism
test defer_backwards, @retry_forever_under_sandbox:
    build
        => settles
    click Ui.Button(label="Read")
        => errors DeferBackwards(..)
```

### The transcript printer

The transcript is what the conformance suite compares and what the differential test diffs between implementations, so its form is fixed and both implementations print it the same way. It is one block per turn:

```
turn 0
  occurrence 0
    list
      0/#0/0: Ui.Button(label="Increment")
      1/#0/0: Ui.Button(label="Decrement")
      2/#0/0: Ui.Label(text="Count: 0")
    result: unit
turn 1  response 0/#0/0 click
  occurrence 1
    list
      0/#0/0: Ui.Button(label="Increment")
      1/#0/0: Ui.Button(label="Decrement")
      2/#1/0: Ui.Label(text="Count: 1")
    result: unit
turn 2  response 0/#0/0 click
  ...
turn 5  push Clock Instant 5100
  occurrence 7
    requested 3/#0/1: Time.Every(period=100) <- tick
    list
      ...
turn 6  response 0/2/0 click
  occurrence 8
    requested 0/2/0: Fs.File(path="/etc/shadow", requests=..) <- Read
    requested 0/3/1: Defer <- unit
    list
      ...
  occurrence 9  answering Defer 0/3/1
    list
      ...
turn 7  response 0/#0/0 click
  error[cycle] at occurrence 10
    --> counter.prism:12:5 `!delta`
```

Paths are printed in the form chapter 2 uses (`bind/#generation/bind`, `[key]` for `for`), which is the one representation the two implementations agree to print even though each stores paths as it likes. Elements print as the language writes them, fields in declaration order, with `requests=..` for an event field. Values print as literals. Lists are in tree order, which is path order.

The printer is `impl Display for Transcript`, fed by `record_turn` from the `TurnOutcome` and nothing else. A transcript therefore contains only what the World saw: no hold values, no node internals, no event names. A test that passes on a transcript passes under any implementation that produces the same one, and that is what makes the differential test meaningful.

### What the Test World never touches

The Test World reads nothing but the run call's outcome. It has no handle to a node, no table of named triggers it can fire by name, no way to read a dynamic by name, and no delegate that the runtime calls when an event fires. All of those are ways for a test to depend on the structure of a widget rather than on its behaviour, and a test that depends on structure breaks when the structure is refactored without the behaviour changing, which is the worst kind of test. By addressing only elements, which are the only things a real World can address, a test is a World, and a program that passes under it passes under any World that delivers the same inputs.

It also means the runtime's own tests and the Test World are different layers. The runtime's Rust tests build graphs with the combinators directly and check laws about them; they are allowed to look inside, because they test the runtime's structure on purpose. The Test World tests programs.

## The CLI World

The CLI World is the smallest World that is not a test, and the first place a Prism program meets a real process. The problem it solves is the one every language solves on day one, reading input and writing output, done under the constraint that the program cannot do either: the program has elements, and the World has the file descriptors.

### Structure

The main thread runs the harness loop shown above: `step`, and when idle, `wait` with the next timer's deadline. Everything that blocks lives on a thread that is not the main thread, and talks to the main thread through the inbox. There are at most four such threads: one reading stdin, one waiting on signals, a small pool for file operations, and none for timers, because a timer is a deadline the main loop's `wait` already honours. Nothing on any of those threads touches the runtime. A thread's whole interface to the World is `inbox.send(Delivery::Response { path, value })`.

The components, with the tags each accepts:

| Component | Tags | Kinds |
| --- | --- | --- |
| `process::Process` | `Process.Args`, `Process.Env` | state |
| `cli::Stdio` | `Cli.Stdin`, `Cli.Stdout`, `Cli.Stderr`, `Process.Exit` | responds; requests+responds; requests+responds; requests |
| `cli::Signals` | `Cli.Signals` | responds |
| `fs::Files` | `Fs.File`, `Fs.Contents` | requests+responds; state |
| `time::Timers` | `Time.After`, `Time.Every`, `Clock` | requests+responds; responds; state |
| the core | `Defer` | requests+responds |

`Fs`, `Time` and `Clock` are not CLI tags; they are the same components a GUI World uses, in the same crate, and the CLI World is the first World to link them. The tag declarations are the ones chapter 2 uses:

```prism
type StdinEvent = choice:
    Chunk Text
    End

type Code = Code Int

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

tag Process.Args of Element:
    state: [Text]

tag Process.Env of Element:
    state: Text ==> Text

type Signal = choice Interrupt | Terminate | Hangup | Other Int

tag Cli.Signals of Element:
    responds: Signal
```

The kinds are what the language fixes, and the reasoning for each is below with its component; the field names follow the library.

### `Process.Args` and `Process.Env`: state, constant after start

```rust
pub struct Process { args: Value, env: Value }

impl Component for Process {
    fn tags(&self) -> Vec<Tag> { vec![Tag::PROCESS_ARGS, Tag::PROCESS_ENV] }
    fn oracle(&mut self, e: &ElementValue, _: &mut Cx) -> Value {
        if e.tag == Tag::PROCESS_ARGS { self.args.clone() } else { self.env.clone() }
    }
    fn gather(&mut self, _: &mut Cx) {}            // never pushes
    fn quiescent(&self) -> bool { true }           // an Args slot keeps nothing alive
    fn slot_created(&mut self, ..) {}  fn slot_removed(&mut self, ..) {}  fn perform(&mut self, ..) {}
}
```

Arguments and environment are state elements rather than parameters of `@main` so that an interceptor can give a subtree different ones and a test can set them with `with Process.Args = [["notes.txt"]]`, with no second way to start a program. On a process that cannot observe a change to its arguments the component never pushes, and the `dyn` a program puts over `$args` builds once. A World that can observe a change (a shell World hosting other programs, where "arguments" is something the shell decides) pushes, and the same program rebuilds; nothing in the program says which World it is under.

`quiescent` is `true` because a program whose only slot is `Process.Args` is a program that has nothing left to do, and the run should end with code 0 rather than wait forever for a push that never comes.

### `Cli.Stdin`: a reader thread, started and stopped by the slot

Standard input is a stream element: it responds with chunks and then an end, and the program never asks it for anything. The World reads stdin only while a `Cli.Stdin` slot exists, which has a consequence a shell user will recognise: a program that builds its `Cli.Stdin` inside a `dyn` after reading a file first does not consume stdin until the file has arrived, so a pipeline behind it is not drained early.

```rust
pub struct Stdio {
    inbox: Inbox,
    stream: Arc<StdinStream>,              // outlives any slot; shared with the reader thread
    slot: Option<Path>,                    // the Cli.Stdin slot that is listening, if any
    reader: Option<JoinHandle<()>>,        // one thread, started by the first slot
    out: BufWriter<std::io::Stdout>,
    err: BufWriter<std::io::Stderr>,
    out_failed: bool,
    chunk: usize,                          // max bytes per chunk; a flag for goldens
}

/// What stdin has produced that no slot has taken. The stream owns it, so a slot
/// that goes away leaves it where the next slot will find it.
struct StdinStream { state: Mutex<StdinState>, changed: Condvar }
struct StdinState {
    listening: bool,                       // a Cli.Stdin slot exists
    held: VecDeque<Held>,                  // read and not yet delivered, in order
    ended: bool,                           // End has been read
}
enum Held { Chunk(String), End }

fn read_loop(stream: Arc<StdinStream>, inbox: Inbox, chunk: usize) {
    let mut stdin = std::io::stdin().lock();
    let mut buf = vec![0u8; chunk];
    let mut pending = Vec::new();                  // bytes of a character the last read split
    loop {
        {   // read only for a listening slot, and only once the last read has been taken
            let mut st = stream.state.lock().unwrap();
            while !(st.listening && st.held.is_empty()) { st = stream.changed.wait(st).unwrap(); }
        }
        let n = stdin.read(&mut buf).unwrap_or(0);  // blocks; cannot be interrupted portably
        let mut st = stream.state.lock().unwrap();
        if n == 0 {
            if !pending.is_empty() { st.held.push_back(Held::Chunk(lossy(&pending))); }
            st.held.push_back(Held::End);
            st.ended = true;
        } else {
            pending.extend_from_slice(&buf[..n]);
            let cut = complete_utf8_prefix(&pending);    // up to the last whole character
            let text = lossy(&pending[..cut]);
            pending.drain(..cut);
            if !text.is_empty() { st.held.push_back(Held::Chunk(text)); }
        }
        let done = n == 0;
        drop(st);
        inbox.send(Delivery::Wake);                // a turn may be due; the chunk waits in `held`
        if done { return; }
    }
}

impl Component for Stdio {
    fn slot_created(&mut self, path: &Path, el: &ElementValue, cx: &mut Cx) {
        if el.tag != Tag::CLI_STDIN { return; }
        if self.slot.is_some() {
            cx.diagnostic(Diagnostic::Component { tag: el.tag,
                message: "a second Cli.Stdin slot; stdin is one stream and goes to the first".into() });
            return;
        }
        self.slot = Some(path.clone());
        let waiting = {
            let mut st = self.stream.state.lock().unwrap();
            st.listening = true;
            !st.held.is_empty()                    // something was read while no slot listened
        };
        self.stream.changed.notify_all();
        if self.reader.is_none() {
            let (stream, inbox, chunk) = (self.stream.clone(), self.inbox.clone(), self.chunk);
            self.reader = Some(std::thread::spawn(move || read_loop(stream, inbox, chunk)));
        }
        if waiting { self.inbox.send(Delivery::Wake); }
    }

    fn slot_removed(&mut self, path: &Path, el: &ElementValue, _: &mut Cx) {
        if el.tag == Tag::CLI_STDIN && self.slot.as_ref() == Some(path) {
            self.slot = None;
            self.stream.state.lock().unwrap().listening = false;
            // A read already blocked in read(2) still completes. Its chunk goes into `held`
            // and stays there for the next slot; nothing is sent to this path again.
        }
    }

    fn gather(&mut self, cx: &mut Cx) {
        if let Some(path) = &self.slot {
            let mut st = self.stream.state.lock().unwrap();
            if let Some(ev) = st.held.pop_front() {      // one event per turn, to the slot that listens now
                cx.respond(path, ev.into_value());
                self.stream.changed.notify_all();
            }
        }
        // ... flush stdout and stderr, as below
    }

    fn has_gatherable(&self) -> bool {
        self.slot.is_some() && !self.stream.state.lock().unwrap().held.is_empty()
    }

    fn quiescent(&self) -> bool {
        // No slot: nothing will be asked for, and whatever is held is discarded at exit.
        // A slot: alive until End has been read and delivered.
        self.slot.is_none() || {
            let st = self.stream.state.lock().unwrap();
            st.ended && st.held.is_empty()
        }
    }
    ...
}
```

Two things here are the OS being the OS, and the design accepts both rather than hiding them. A thread blocked in `read(2)` cannot be interrupted portably, so the read in flight when a slot goes will complete. Its chunk is not sent to the dropped slot and not lost. Every read goes into `held`, which belongs to the stream, and `gather` hands one held event per turn to the slot that is listening at that moment; a chunk that arrives after the slot went waits there, and the next `Cli.Stdin` slot, however much later, receives it first, then whatever is read afterwards, in order. A program that drops its `Cli.Stdin` slot and later creates another therefore picks the stream up exactly where the first left it. If no slot ever asks again, the held chunk is discarded when the process ends, the same as the rest of an input nobody read.

Delivering from `gather` and not from the thread makes the drop safe. A thread that sent to the inbox would race the slot's removal: the send could land after `slot_removed`, and the core would discard the chunk as a late result. Here the thread only fills `held` and wakes the loop, and the component, which knows which slot is live, is the only sender. The reader also does not read ahead. It reads only while a slot listens and only when `held` is empty, so a program that drops its slot stops consuming the descriptor after the one read already blocked, and a pipeline behind it is not drained. The second thing is that a `Chunk` is bytes as the OS handed them, which means a line can be split across chunks; the program's `@Lines` library widget keeps the partial line in a hold, as chapter 2's line counter does. The World does not reassemble lines, because a program that wants bytes should get bytes, and lines are a function over chunks.

Text is `from_utf8_lossy`, applied only up to the last whole character: a read that ends in the middle of a multi-byte character keeps those bytes for the next chunk, so `"é"` split across two reads arrives as `"é"` and not as two replacement characters. Without that, the text a program saw would depend on where the OS cut the pipe, which is an accident of buffering and not something the program chose; invalid bytes are still replaced. A `Cli.StdinBytes` tag responding with `Bytes` is the version for programs that want the raw stream, and the only difference is the conversion.

The second `Cli.Stdin` slot is a diagnostic and not an error, because it is the program's business if two widgets both want stdin and only one gets it; the value-identity rule does not apply, since `Cli.Stdin` is a stream element identified by path, and two paths are two conversations on one descriptor, which cannot be made to work. A library that wants to share stdin between two widgets does it in Prism, with one `Cli.Stdin` slot and an interceptor that fans out.

### `Cli.Stdout`, `Cli.Stderr`: buffered per turn, flushed at the end

```rust
impl Component for Stdio {
    fn perform(&mut self, path: &Path, el: &ElementValue, request: Value, cx: &mut Cx) {
        match el.tag {
            Tag::CLI_STDOUT => {
                if self.out_failed { return; }
                if let Err(e) = self.out.write_all(request.as_text().as_bytes()) {
                    self.out_failed = true;
                    cx.respond(path, write_failed(e));     // reaches the program next turn
                }
            }
            Tag::CLI_STDERR => { let _ = self.err.write_all(request.as_text().as_bytes()); }
            Tag::PROCESS_EXIT => cx.exit(request.as_code()),
            _ => {}
        }
    }

    fn gather(&mut self, _: &mut Cx) {
        // Called at the start of the next turn, which is after the previous turn's
        // performs: flush what the last turn wrote. Also flushed before exit.
        let _ = self.out.flush();
        let _ = self.err.flush();
    }
    ...
}
```

Writes are performed in occurrence order and then list order, which is the core's order for `perform`, and are flushed once per turn. The reason for per-turn flushing and not per-write is the same as the reason effects happen after the occurrence: a turn is the unit the outside sees, and a program that writes three lines in one occurrence should produce three lines together, not three syscalls. The reason for flushing at all between turns, rather than at exit, is that a CLI program's output is often being read by something, a terminal or the next process in a pipe, and the program's notion of "I said this" is the turn's end.

Two `Cli.Stdout` slots in one program both write to the one descriptor, in list order within an occurrence. That is right: two widgets that both print are two widgets that both print, and the interleaving is the one the program's structure gives. A program that wants a single ordered log puts one `Cli.Stdout` under an interceptor that merges.

A failed write (the reader of a pipe went away, `EPIPE`) sets the flag and comes back to the writing slot as a `WriteFailed` response through `cx.respond`, in a later turn, as the creation rule requires of any response. `SIGPIPE` is left as the Rust runtime leaves it, ignored (from memory; verify), so that the failed write is seen as `EPIPE` and reported, instead of killing the process silently.

### `Process.Exit`: a request, acted on after the turn

Exit is a request element with `requests: !Code`, and the firing is the whole instruction. The component's `perform` records the code with `cx.exit(code)`; the core finishes performing the turn's other requests, which means every write that fired in the same turn lands first, in order; then `Harness::step` returns `Step::Exit(code)` and main flushes and returns it.

Chapter 2 argued from the program's side why exit is a request and not a presence: nothing in the language happens because an element appears, and an exit that happened because a `Process.Exit` element was in the list would be a trigger by existence. From the World's side the argument is about ordering. A request firing has a position in the turn, after some writes and before others; a presence has only a list position, and "exit after this turn's writes but not before the writes of a widget later in the list" would have no meaning. With a request, `Process.Exit` fired in the same occurrence as a `Cli.Stdout` write lands after that write if the stdout slot is earlier in the list, which is the order the program's structure gives, and a program that wants its goodbye message before exit puts the stdout slot first or fires the exit from a later occurrence.

Two `Process.Exit` firings in one turn: the first in occurrence-then-list order wins, and the second gets a diagnostic. Two programs under a shell World both exiting is the shell's business and it intercepts `Process.Exit` before the real World sees it, as chapter 2's shell does.

A code outside 0–255 is clamped and reported, because that is what the OS will do anyway and reporting it is better than letting a program think it exited with 256.

### `Cli.Signals`: the handler follows the slot

```rust
pub struct Signals { inbox: Inbox, slot: Option<Path>, handle: Option<signal_hook::iterator::Handle> }

impl Component for Signals {
    fn slot_created(&mut self, path: &Path, _: &ElementValue, _: &mut Cx) {
        let mut signals = signal_hook::iterator::Signals::new([SIGINT, SIGTERM, SIGHUP]).unwrap();   // signal_hook API from memory
        self.handle = Some(signals.handle());
        let (inbox, p) = (self.inbox.clone(), path.clone());
        std::thread::spawn(move || {
            for sig in signals.forever() {
                inbox.send(Delivery::Response { path: p.clone(), value: signal_value(sig) });
            }
        });
        self.slot = Some(path.clone());
    }
    fn slot_removed(&mut self, _: &Path, _: &ElementValue, _: &mut Cx) {
        if let Some(h) = self.handle.take() { h.close(); }      // restores the OS default (from memory)
        self.slot = None;
    }
    fn quiescent(&self) -> bool { self.slot.is_none() }
    ...
}
```

With no `Cli.Signals` slot, `SIGINT` does what the OS does: kills the process. With one, the signal arrives as a response and the program decides, typically by firing `Process.Exit`. The handler is installed when the slot appears and removed when it goes, so a program that handles `Ctrl-C` only during a critical section has a `Cli.Signals` slot only during it. This is the rule that a stream lives as long as its slot, applied to something that is not a stream of data but a stream of OS events.

A `Cli.Signals` slot keeps the program alive (`quiescent` is false while it exists), because a program waiting for a signal is waiting for something that can happen. That is the difference from `Process.Args`.

### `Fs.File` and `Fs.Contents`: a thread pool and a watcher

`Fs.File(path, requests: !FsRequest, responds: FsResponse)` is the request element for one-shot operations; `Fs.Contents(path, state: Unavailable FsError | Known Text)` is the state element for a file's current contents, where `FsError` is the closed choice `NotFound | Denied | TooLarge | Other Text`. They are one component because they share the path and, on platforms with a watch, the watcher.

```rust
pub struct Files {
    inbox: Inbox,
    cap: u64,                                           // bytes an Fs.Contents read may return; a flag for goldens
    pool: threadpool::ThreadPool,                       // for Fs.File operations (crate API from memory)
    inflight: BTreeMap<Path, Arc<AtomicBool>>,          // per slot: a cancel flag
    watched: BTreeMap<ElementValue, Watched>,           // per Fs.Contents value: the watch and the last value
    watcher: notify::RecommendedWatcher,                // notify's API from memory
    changes: std::sync::mpsc::Receiver<notify::Event>,  // from the watcher's thread
}

struct Watched { value: Value, slots: usize }

impl Component for Files {
    fn tags(&self) -> Vec<Tag> { vec![Tag::FS_FILE, Tag::FS_CONTENTS] }

    fn perform(&mut self, path: &Path, el: &ElementValue, request: Value, _: &mut Cx) {
        let file = el.field("path").as_text().to_owned();
        let cancel = self.inflight.entry(path.clone()).or_default().clone();
        let (inbox, p) = (self.inbox.clone(), path.clone());
        self.pool.execute(move || {
            let response = match request.as_fs_request() {
                FsRequest::Read => match std::fs::read_to_string(&file) {
                    Ok(t) => fs_contents(t),
                    Err(e) if e.kind() == NotFound => fs_missing(),
                    Err(e) if e.kind() == PermissionDenied => fs_denied(),
                    Err(e) => fs_failed(e),
                },
                FsRequest::Write(t) => match std::fs::write(&file, t) { Ok(()) => fs_written(), Err(e) => fs_failed(e) },
            };
            if !cancel.load(Ordering::Relaxed) {
                inbox.send(Delivery::Response { path: p, value: response });
            }
        });
    }

    fn slot_removed(&mut self, path: &Path, el: &ElementValue, _: &mut Cx) {
        match el.tag {
            Tag::FS_FILE => { if let Some(c) = self.inflight.remove(path) { c.store(true, Ordering::Relaxed); } }
            Tag::FS_CONTENTS => {
                let w = self.watched.get_mut(el).unwrap();
                w.slots -= 1;
                if w.slots == 0 { let _ = self.watcher.unwatch(el.field("path").as_text()); self.watched.remove(el); }
            }
            _ => {}
        }
    }

    fn slot_created(&mut self, _: &Path, el: &ElementValue, _: &mut Cx) {
        if el.tag == Tag::FS_CONTENTS {
            // the oracle has already been asked, so `watched` has an entry
            self.watched.get_mut(el).unwrap().slots += 1;
        }
    }

    fn oracle(&mut self, el: &ElementValue, cx: &mut Cx) -> Value {
        // Fs.Contents only. Read synchronously; start the watch.
        let file = el.field("path").as_text().to_owned();
        let value = contents_of(&file, self.cap);
        if let Err(e) = self.watcher.watch(Path::new(&file), NonRecursive) {
            cx.diagnostic(Diagnostic::Component { tag: el.tag, message: format!("watch {file}: {e}") });
        }
        self.watched.insert(el.clone(), Watched { value: value.clone(), slots: 0 });
        value
    }

    fn gather(&mut self, cx: &mut Cx) {
        while let Ok(ev) = self.changes.try_recv() {
            for p in ev.paths {
                let el = fs_contents_element(&p);
                if let Some(w) = self.watched.get_mut(&el) {
                    let now = contents_of(&p, self.cap);
                    if now != w.value { w.value = now.clone(); cx.push(&el, now); }
                }
            }
        }
    }

    fn quiescent(&self) -> bool { self.inflight.is_empty() }
    ...
}

/// What an Fs.Contents slot holds for this path: the text, or the reason there is none.
fn contents_of(path: impl AsRef<std::path::Path>, cap: u64) -> Value {
    let mut bytes = Vec::new();
    match std::fs::File::open(path).and_then(|f| f.take(cap + 1).read_to_end(&mut bytes)) {
        Err(e) => unavailable(fs_error(&e)),
        Ok(_) if bytes.len() as u64 > cap => unavailable(FsError::TooLarge),
        Ok(_) => match String::from_utf8(bytes) {
            Ok(t) => known(t),
            Err(e) => unavailable(FsError::Other(e.to_string())),
        },
    }
}

fn fs_error(e: &std::io::Error) -> FsError {
    match e.kind() {
        std::io::ErrorKind::NotFound => FsError::NotFound,
        std::io::ErrorKind::PermissionDenied => FsError::Denied,
        _ => FsError::Other(e.to_string()),
    }
}
```

The synchronous read in `oracle` is the whole reason a file viewer shows the file in its first occurrence instead of flashing "unavailable" for one frame. The watch is started in the same call, so that a change between the read and the first turn is not lost: `notify` delivers it to the channel (from memory), `gather` picks it up at the next turn, re-reads, and pushes if the contents differ. The read stops at `cap + 1` bytes, so a huge file costs one capped read and not its size, and every failure maps to one of the four reasons: a missing path is `NotFound`, `PermissionDenied` is `Denied`, a file past the cap is `TooLarge`, and anything else (a directory, bytes that are not text, a device that errors) is `Other` with the operating system's message. The reason is a value like any other, so a file that is deleted pushes `Unavailable NotFound`, one that is created pushes `Known`, and a program that shows "permission denied" differently from "no such file" matches on the reason. On a platform without a usable watch the component falls back to a stat-and-reread in `gather`, comparing modification times; the program cannot tell, except that changes arrive at the next turn rather than causing one.

Whether a changed file should *cause* a turn is the `has_gatherable` question: yes for a watch, because a program showing a file should update when the file changes even if nothing else is happening, so `Files` reports `true` whenever the change channel has something. The main loop's `wait` is then woken by the inbox, which the watcher thread also sends a no-op `Delivery` into for exactly that purpose; a `Delivery::Wake` variant with no payload is the cleanest spelling and the harness drops it after waking.

`Fs.Contents` value identity shows up as the `slots` count: two widgets viewing the same path share one watch, one read, and one push, and the watch stops when the last of them goes. The core's snapshot already shares the value; the component shares the OS resource.

The cancel flag for `Fs.File` is the "cancel if the World can" obligation. A `std::fs::read_to_string` on a pool thread cannot be interrupted, so cancellation here means only that the result is not delivered; the core would have discarded it anyway as a late result, and the flag only saves the send. A World on an OS with cancellable IO does better; nothing in the contract changes.

`quiescent` is `false` while an `Fs.File` operation is in flight, so a program that fired a read and is waiting for the answer is not ended. A watch does not keep a program alive on its own, which is a choice: a program whose only live thing is a file watch is like a program whose only live thing is a signal handler, and the two could reasonably go either way. The reasoning for the difference is that a program that wants to run until a file changes is rare, and a program that forgot to exit after reading a file is common, and the second should end with code 0.

### `Time.After`, `Time.Every`, `Clock`: deadlines in the loop's wait

```rust
pub struct Timers {
    inbox: Inbox,
    after: BTreeMap<(Instant, u64), (Path, u64)>,   // Time.After deadlines, keyed by time then sequence; the token
    every: BTreeMap<Path, Every>,                   // Time.Every slots: period and next deadline
    seq: u64,
    turn_now: Instant,                              // sampled once per turn
    epoch: Instant,
}

struct Every { period: Duration, next: Instant }

impl Timers {
    /// The main loop asks this before blocking.
    pub fn next_deadline(&self) -> Option<Instant> {
        let a = self.after.keys().next().map(|(t, _)| *t);
        let e = self.every.values().map(|e| e.next).min();
        [a, e].into_iter().flatten().min()
    }
}

impl Component for Timers {
    fn tags(&self) -> Vec<Tag> { vec![Tag::TIME_AFTER, Tag::TIME_EVERY, Tag::CLOCK] }

    fn perform(&mut self, path: &Path, _: &ElementValue, request: Value, _: &mut Cx) {
        // Time.After(requests: !Nat, responds: Nat): milliseconds in, the same Nat echoed back
        self.seq += 1;
        let ms = request.as_nat();
        self.after.insert((self.turn_now + Duration::from_millis(ms), self.seq), (path.clone(), ms));
    }

    fn slot_created(&mut self, path: &Path, el: &ElementValue, _: &mut Cx) {
        if el.tag == Tag::TIME_EVERY {
            let period = Duration::from_millis(el.field("period").as_int() as u64);
            self.every.insert(path.clone(), Every { period, next: self.turn_now + period });
        }
    }
    fn slot_removed(&mut self, path: &Path, el: &ElementValue, _: &mut Cx) {
        match el.tag {
            Tag::TIME_EVERY => { self.every.remove(path); }
            Tag::TIME_AFTER => { self.after.retain(|_, (p, _)| p != path); }
            _ => {}
        }
    }

    fn oracle(&mut self, _: &ElementValue, _: &mut Cx) -> Value { instant(self.turn_now - self.epoch) }

    fn gather(&mut self, cx: &mut Cx) {
        self.turn_now = Instant::now();
        cx.push(&ElementValue::nullary(Tag::CLOCK), instant(self.turn_now - self.epoch));
        // Every expired Time.After fires; the earliest Time.Every per slot fires, once.
        while let Some((&(t, s), _)) = self.after.first_key_value().filter(|((t, _), _)| *t <= self.turn_now) {
            let (path, ms) = self.after.remove(&(t, s)).unwrap();
            cx.respond(&path, Value::nat(ms));
        }
        for (path, e) in &mut self.every {
            if e.next <= self.turn_now {
                cx.respond(path, tick());
                while e.next <= self.turn_now { e.next += e.period; }    // skip missed ticks, keep phase
            }
        }
    }

    fn quiescent(&self) -> bool { self.after.is_empty() && self.every.is_empty() }
    ...
}
```

There is no timer thread and no timer wheel: timers are deadlines, the main loop's `wait` takes the earliest as its timeout, and `gather` fires everything that has expired as responses for the turn that follows. That is enough for a CLI World, where timer resolution is milliseconds and there are tens of timers, not thousands; a World with thousands of timers replaces the `BTreeMap` with a wheel and nothing else changes.

`Clock` is a state element pushed once per turn at most, and `turn_now` is sampled once at the start of `gather`, so every occurrence of a turn sees the same instant, and every timer that fires in a turn fires against the same `now` the clock shows. A program that reads `$clock` in the occurrence a `Time.Every` tick arrives reads the time the tick was fired at, through a tag, before the push; after the occurrence it holds the pushed value. The stopwatch in chapter 2 depends on exactly this.

Two timers expiring in the same `wait` are two responses in one batch; under `OneAtATime` they are two turns, in deadline order, each with its own clock sample. Under `Drain` they would be one occurrence, which is legal (two different slots) and honest (they did expire together, as far as a millisecond clock knows). The CLI World uses `OneAtATime` because its programs are written as if each input is a turn, and the chunk-and-end reasoning of chapter 2's line counter is easier to trust when it is also true of timers.

A `Time.Every` tick that was missed, because the process was stopped or a turn took longer than the period, is skipped rather than delivered late in a burst. The phase is kept. This is a World policy, and a World that wants to deliver every tick keys them differently; the program sees ticks either way and the language promises nothing about their spacing.

### Exit when nothing can happen

A CLI run ends on `Process.Exit`, or when nothing can ever cause another occurrence. The second is `Step::Quiescent`, and it is a conjunction over components: the inbox is empty, stdin has nothing more to deliver (no `Cli.Stdin` slot, or `End` has been delivered), no signal slot exists, no `Fs.File` operation is in flight, no timer is set. A program with none of those has a graph with no sources left, and a graph with no sources cannot change, so ending it with code 0 is not a policy; it is reading the situation correctly. The alternative, waiting for an explicit `Process.Exit` forever, turns a forgotten exit into a hang, and the forgotten exit is the common bug.

The one judgment call in the conjunction is which slots count as sources, and it was made per component above: stdin, signals, in-flight requests and timers keep a program alive; arguments, environment, the clock and file watches do not. Each is a line in a component's `quiescent`, and changing one is changing one line.

### The line counter under the real OS

Chapter 2 walked the line counter occurrence by occurrence under a hypothetical World. Here is what the CLI World does with it, with `printf 'one\ntwo\nthree\n' | wc_prism` and the reader's chunk size left at the default.

Occurrence 0 builds three slots. `start` performs no requests (nothing fired) and diffs the list against nothing: `slot_created` for `0: Cli.Stdin` starts the reader thread, and the other two are request elements with nothing to start. The loop calls `step`: nothing queued, not quiescent (a slot is listening and `End` has not come), `Idle`; `wait` blocks on the inbox with no deadline.

The reader thread reads 14 bytes in one `read(2)`, because `printf` wrote them in one `write(2)` and the pipe delivered them together, and puts `Chunk "one\ntwo\nthree\n"` in `held`, then wakes the loop. `wait` returns; `step` finds the held chunk gatherable and turn 1 opens with it as one response under `OneAtATime`; occurrence 1 runs; `$lines` becomes `3`. No requests. The list is unchanged; nothing to start or stop. `Idle` again.

The reader reads 0 bytes and holds `End`. Turn 2, occurrence 2: `!end` fires; `Cli.Stdout` fires `"3\n"`; `Process.Exit` fires `Code 0`. The core performs them in list order: `Stdio::perform` writes `"3\n"` to the buffer, then records exit 0. `step` returns `Exit(0)`; main flushes and exits.

Had the input been typed at a terminal, each line would have been its own chunk and the run would have had four turns before `End`, and the output would be the same, which is the point of the chunk-independence a stream program has to have and chapter 2's `@Lines` provides.

## A tokio-based World skeleton

A CLI World can get by with threads because it has four blocking things. A World with a thousand sockets cannot, and the Rust answer to a thousand sockets is tokio. The question is where tokio goes, and the answer is: on the IO side, never on the propagation side.

Suppose the turn loop were an `async fn`, components were `async`, and `perform` awaited the operation and returned the response. Then a turn would be suspended, with the runtime's state live, while a socket was read; a second input arriving in the meantime would either wait (serialising all IO behind the slowest operation) or start a second turn (two turns in flight on one runtime, which the transaction model forbids). And on any platform with a GUI main thread, there is no awaiting inside a callback at all. The CLI World's design avoids both: a turn is synchronous and brief; an IO task is a future that ends by sending a `Delivery` to the inbox; the loop waits on the inbox and on nothing else.

```rust
pub async fn run_tokio<R: Runtime>(mut harness: Harness<R>,
                                   mut inbox: tokio::sync::mpsc::Receiver<Delivery>,
                                   deadlines: Arc<Deadlines>) -> i32
{
    if let Step::Exit(c) = harness.start() { return c; }
    loop {
        // 1. Wait for the first delivery, or the earliest timer.
        let first = tokio::select! {
            d = inbox.recv() => match d { Some(d) => Some(d), None => return 0 },
            _ = deadlines.sleep_until_next() => None,
        };
        // 2. Batch what else is ready now. One response per path per occurrence;
        //    a second to the same path stays queued for the next turn.
        let mut batch: Vec<Delivery> = first.into_iter().collect();
        let mut paths = BTreeSet::new();
        for d in &batch { if let Delivery::Response { path, .. } = d { paths.insert(path.clone()); } }
        loop {
            match inbox.try_recv() {
                Ok(Delivery::Response { path, value }) if !paths.insert(path.clone()) => {
                    harness.hold(Delivery::Response { path, value });    // next turn
                    break;                                                 // keep order: stop batching
                }
                Ok(d) => batch.push(d),
                Err(_) => break,
            }
        }
        // 3. One turn, synchronous, on this task.
        harness.push_all(batch);
        match harness.step() {
            Step::Ran(_) | Step::Idle => {}
            Step::Exit(code) => return code,
            Step::Quiescent => return 0,
            Step::Error(_) => return 1,
        }
    }
}
```

The components of a tokio World are the same trait with `tokio::spawn` where the CLI World had `std::thread::spawn`:

```rust
impl Component for Sockets {
    fn slot_created(&mut self, path: &Path, el: &ElementValue, _: &mut Cx) {
        // Net.Socket(address, requests: !Bytes, responds: Received Bytes | Closed,
        //            state: Connecting | Connected | Disconnected Reason)
        let addr = el.field("address").as_text().to_owned();
        let (inbox, p, el) = (self.inbox.clone(), path.clone(), el.clone());
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Bytes>(16);       // outbound, bounded
        self.outbound.insert(path.clone(), tx);
        let handle = tokio::spawn(async move {
            let stream = match TcpStream::connect(&addr).await {
                Ok(s) => { inbox.send(Delivery::Push { element: el.clone(), value: connected() }); s }
                Err(e) => { inbox.send(Delivery::Push { element: el, value: disconnected(e) }); return; }
            };
            let (mut rd, mut wr) = stream.into_split();
            let mut buf = vec![0u8; 4096];
            loop {
                tokio::select! {
                    n = rd.read(&mut buf) => match n {
                        Ok(0) | Err(_) => { inbox.send(Delivery::Response { path: p, value: closed() }); return; }
                        Ok(n) => inbox.send(Delivery::Response { path: p.clone(), value: received(&buf[..n]) }),
                    },
                    Some(bytes) = rx.recv() => { if wr.write_all(&bytes).await.is_err() { return; } }
                }
            }
        });
        self.tasks.insert(path.clone(), handle);
    }

    fn perform(&mut self, path: &Path, _: &ElementValue, request: Value, _: &mut Cx) {
        if let Some(tx) = self.outbound.get(path) {
            let _ = tx.try_send(request.as_bytes());      // full: backpressure, below
        }
    }

    fn slot_removed(&mut self, path: &Path, _: &ElementValue, _: &mut Cx) {
        if let Some(h) = self.tasks.remove(path) { h.abort(); }    // cancels the connection
        self.outbound.remove(path);
    }
    ...
}
```

The `inbox.send` here is the tokio channel's `blocking_send` (from memory) or an unbounded send, and that choice is backpressure. If the inbox is bounded and a reader task blocks on sending, the socket is not read until the loop drains, which is the correct behaviour for a program that is slower than its input: the kernel's receive buffer fills, TCP's window closes, and the sender slows down. That is backpressure, and a bounded inbox gives a Prism World it for free. The outbound channel in `perform` is bounded too, and `try_send` failing means the program is writing faster than the socket drains; the component's choice above is to drop with a diagnostic, and a World that wants to tell the program would use a tag whose response includes `Backpressured`, at which point the program's own hold is the buffer and the program decides what to drop. Buffering unboundedly inside the World is the one option not taken, because an unbounded buffer is a memory leak with a delay.

Step 2's batching decides what "simultaneous inputs" means. Two deliveries that were both ready when the loop woke are, as far as the World can know, simultaneous; delivering them in one occurrence tells the program they were, and the program's `|` merges and the comma-style reasoning apply. Delivering them in two turns tells the program one came first, which the World does not know either. Both are honest; the skeleton batches because a server that gets two requests at once should see two requests at once, while the CLI World does not because its programs are written one input at a time. The rule that one slot gets one response per occurrence is enforced either way, and the skeleton stops batching at the first repeat rather than skipping over it, so that the order of deliveries to one path is kept.

The `Deadlines` object is the tokio spelling of the CLI World's `next_deadline`: the `Timers` component writes its earliest deadline into it, and `sleep_until_next` is a `tokio::time::sleep_until` that is recreated whenever the earliest changes. Nothing else about `Timers` changes.

A World that mixes a GUI main thread and tokio IO, which a desktop World does, runs this loop's select on a tokio thread that does nothing but forward: it receives from the inbox and calls the platform's wake (winit's `EventLoopProxy::send_event`, from memory), and the main thread's event handler calls `step`. The turn runs on the main thread, where the platform wants it; tokio never touches the runtime.

## The GUI World

A click at (50, 10) reaches the software rasteriser below as one winit event. The World turns it into one response, `Pointer (Down ((50, 10)), Left)`, addressed to the path of the program's `Gui.Window` slot, runs a turn, and paints the `Gui.Scene` the turn left in the list. It did not look for a button, and it could not have: no button is in the list. The library above it hit-tests the point against the layout it drew, which is the only place that layout exists. Everything this section builds is the same small job done for a different platform event.

GUI is not a World concept. There is one GUI vocabulary, five tags, and any World that can put pixels on a screen and read a pointer and a keyboard can speak it. Layout, hit-testing, focus traversal, text editing, buttons, lists and scrolling are Prism interceptors over those tags, in a library that is part of the conformance suite, so that they run under the Test World too. The World's part is small on purpose, because everything the World does is something a test cannot see.

### The vocabulary

Chapter 7 declares the vocabulary and the event types, and the Rust side reads them as follows. A window and the one place events for it go, keys included; the picture, as a list of plain `Draw.*` values; the name of the focused region; text measurement; and a tick per frame.

```prism
tag Gui.Window of Element:
    title: Text
    size: Extent                   # asked for when the window opens
    responds: WindowEvent          # Resized Extent | CloseRequested | Pointer Pointer | Pressed Key

tag Gui.Scene of Element:
    window: Text                   # the title of the window it paints
    draw: [Draw]                   # Draw.Rect | Draw.Text | Draw.Image | Draw.Clip, in paint order

tag Gui.Focusable of Element:     # presence: no responses; keys arrive at Gui.Window as Pressed Key
    name: Text

tag Gui.Metrics of Element:
    text: Text
    font: Font
    state: Extent                  # width and height of the text; answered synchronously

type Frame = record:
    now: Instant

tag Gui.Frames of Element:
    responds: Frame
```

Only `Gui.Metrics` is a state tag, and none of the five has a `requests` field. A World that speaks this vocabulary therefore performs nothing: it paints a list, answers one question, and delivers events. The negative space is deliberate. There is no region for the World to hit-test, so there is no overlap rule, no disabled-region rule and no pointer capture to get wrong; there is no focus request for the World to grant, so the World and the library cannot hold two opinions about who is focused; and a draw command is a value in a list, so it has no path, no slot and no teardown to track. The clipboard is a separate bare tag, `Clipboard`, which a World offers or does not (chapter 7 says why it is not part of `Gui.*`), and installed fonts are not a tag.

A label whose text changes rebuilds the library's scene: `Gui.Scene` is a presence element with a plain `draw` field, so a new draw list is a fresh slot at the next generation, and the World diffs the new list against the one it painted. The `Gui.Window` slot stays put, which is what keeps a pointer event's address stable across frames.

### A software rasteriser first

The first GUI World draws into a pixel buffer on the CPU and blits it to a window. The choice is deliberate: a software rasteriser has no GPU state, no shader pipeline and no driver, so every bug is a Rust bug, and the whole World is a few hundred lines on top of `winit` for the window and input, `softbuffer` for the blit (its API from memory), and a text rasteriser for glyphs. wgpu (from memory) is a drawing backend with no language consequence and can replace the rasteriser when something is measured to be slow.

```rust
pub struct Screen {
    inbox: Inbox,
    window: Option<Arc<winit::window::Window>>,
    surface: Option<softbuffer::Surface<..>>,
    fonts: FontCache,                               // glyph layout and rasterising
    window_path: Option<Path>,                      // the Gui.Window slot: where pointer events, keys and resizes go
    focusable: Option<Path>,                        // the Gui.Focusable slot, if the list has one: where the focus ring and the candidate window are shown; keys do not go here
    scene: Vec<Draw>,                               // the draw list as of the last turn
    damage: Vec<Rect>,
    frame_slots: BTreeSet<Path>,
    window_slots: usize,
}

impl Component for Screen {
    fn tags(&self) -> Vec<Tag> {
        vec![Tag::GUI_WINDOW, Tag::GUI_SCENE, Tag::GUI_FOCUSABLE, Tag::GUI_METRICS, Tag::GUI_FRAMES]
    }

    fn oracle(&mut self, el: &ElementValue, _: &mut Cx) -> Value {
        match el.tag {
            Tag::GUI_METRICS => self.fonts.measure(el.field("text").as_text(), el.field("font")),
            _ => unreachable!("only state tags are asked"),
        }
    }

    fn slot_created(&mut self, path: &Path, el: &ElementValue, _: &mut Cx) {
        match el.tag {
            Tag::GUI_WINDOW => {
                self.window_slots += 1;
                self.window_path = Some(path.clone());
                self.ensure_window(el.field("title").as_text(), el.field("size").as_extent());
            }
            Tag::GUI_FOCUSABLE => self.focusable = Some(path.clone()),
            Tag::GUI_FRAMES => { self.frame_slots.insert(path.clone()); self.request_redraw(); }
            _ => {}                                  // a Gui.Scene is read from the list in after_turn
        }
    }

    fn slot_removed(&mut self, path: &Path, el: &ElementValue, _: &mut Cx) {
        match el.tag {
            Tag::GUI_WINDOW => {
                self.window_slots -= 1;
                if self.window_slots == 0 { self.close_window(); self.window_path = None; }
            }
            Tag::GUI_FOCUSABLE => if self.focusable.as_ref() == Some(path) { self.focusable = None; },
            Tag::GUI_FRAMES => { self.frame_slots.remove(path); }
            _ => {}
        }
    }

    fn perform(&mut self, _: &Path, _: &ElementValue, _: Value, _: &mut Cx) {}   // no tag here has requests
    fn gather(&mut self, _: &mut Cx) {}
    fn quiescent(&self) -> bool { self.window_slots == 0 }
}
```

Metrics are answered from the same `FontCache` the painter uses, so a label is measured with the glyphs it will be drawn with. The oracle cannot fail, and the cache is keyed by the element value, as the core's snapshot is, so every slot asking about `("Start", Default)` shares one glyph layout. The push for it fires only when the installed fonts change, which is almost never.

The World's half of the loop is winit's `ApplicationHandler` (the winit names below are from memory). Each platform event becomes zero or more deliveries and a `step`:

```rust
impl ApplicationHandler<Delivery> for App {
    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let screen = self.screen.borrow();
        let to_window = |value| screen.window_path.clone()
            .map(|path| Delivery::Response { path, value });
        let mut redraw = false;
        let batch: Vec<Delivery> = match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer = position.into();
                to_window(pointer_moved(self.pointer)).into_iter().collect()
            }
            WindowEvent::MouseInput { state, button, .. } =>
                to_window(pointer_button(state, self.pointer, button)).into_iter().collect(),
            WindowEvent::MouseWheel { delta, .. } =>
                to_window(pointer_scroll(self.pointer, delta)).into_iter().collect(),
            WindowEvent::KeyboardInput { event, .. } =>
                to_window(pressed(key_event(event))).into_iter().collect(),   // the library routes it by focus name
            WindowEvent::Ime(ime) => to_window(pressed(composition_event(ime))).into_iter().collect(),
            WindowEvent::Resized(size) => to_window(resized(size)).into_iter().collect(),
            WindowEvent::CloseRequested => to_window(close_requested()).into_iter().collect(),
            WindowEvent::RedrawRequested => {
                redraw = true;                                        // Gui.Frames fires before the paint
                screen.frame_slots.iter()
                    .map(|path| Delivery::Response { path: path.clone(), value: frame(self.clock.now()) })
                    .collect()
            }
            _ => vec![],
        };
        drop(screen);
        self.harness.push_all(batch);
        self.run_turns(el);
        if redraw { self.screen.borrow_mut().paint(); }
    }

    fn user_event(&mut self, el: &ActiveEventLoop, _: Delivery) { self.run_turns(el); }   // the waker

    fn run_turns(&mut self, el: &ActiveEventLoop) {
        loop {
            match self.harness.step() {
                Step::Ran(out) => { self.screen.borrow_mut().after_turn(&out.list); }
                Step::Idle => break,
                Step::Exit(_) | Step::Quiescent => { el.exit(); break; }
                Step::Error(e) => { self.screen.borrow_mut().show_error(&e); break; }
            }
        }
    }
}
```

Three things are missing from the handler on purpose. It does no hit-testing: every pointer event goes to the one window path, with the point in it. It keeps no focus: a key goes to the window path like a pointer event, and the library routes it to the input whose name is the focus, or drops it when no input is focused. The `Gui.Focusable` in the list is only read, to show the platform's focus ring. And `Resized` is a response like any other, not a pushed state; the library holds the size and lays out again in the occurrence the response arrives in.

`after_turn` finds the `Gui.Scene` in the turn's list and compares its draw list with `scene`, command by command in paint order. A command that differs damages both its old and its new bounds, a longer or shorter list damages the tail, and a World that would rather repaint the whole window is free to, because the language cannot see which it did. It then looks for the one `Draw.Text` whose `ime_at` is set, which the library sets only on the focused input's run, turns the offset into a position with the font cache and tells the platform where the candidate window goes (`set_ime_cursor_area`, from memory). It requests a redraw if `damage` is non-empty or a `Gui.Frames` slot exists. `paint` clears the damaged rects and replays the `Draw.Rect`, `Draw.Text`, `Draw.Image` and `Draw.Clip` commands in list order, clipped to the damage; a `Draw.Clip` pushes its rect on a clip stack for its nested list and pops it after. The pointer-move coalescing that every GUI does is the harness's `Drain` policy plus a line in the handler that replaces a queued `Pointer (Moved ..)` with the latest one, which promises the program no less than the platform does; clicks, keys and composition events are never coalesced.

Live resize on platforms that call back synchronously during a drag is one turn per callback, with the paint inside the callback; the harness's `step` never blocks, so this is safe.

The error policy is a GUI World's and not the core's: on `Step::Error`, the window stays open, the error is drawn in it, and the next input is a fresh occurrence against the rolled-back runtime. A CLI World exits; a GUI World has a window to show the error in, and a user who can see what went wrong and keep going is better served than one whose window vanished.

### How a native-toolkit wrapper differs

A World that wraps GTK or AppKit does not speak the `Gui.*` vocabulary; it offers higher tags, and the same Prism program targets it through a different library.

```prism
tag Gtk.Window of Element:
    title: Text
    children: [Element]

tag Gtk.Button of Element:
    label: Text
    responds: Click

tag Gtk.Entry of Element:
    placeholder: Text
    requests: !EntryRequest        # SetText Text | SelectAll
    responds: EntryEvent           # Changed Text | Activated
    state: Text                    # what the native entry holds right now
```

The whole middle of the World differs. There is no draw list and no rasteriser; instead the component keeps a map from path to native widget, and `after_turn` becomes a reconciliation: a slot created makes a native widget, a slot removed destroys one, and because fields are fixed for a slot's life, a changed label is a removed slot and a created slot, which the component may recognise as the same tree position and the same tag and turn into a `set_label` on the existing native widget rather than a destroy-and-create, keeping the native widget's own state (cursor, selection, scroll position). That is the continuity-by-tree-position policy the specification leaves to Worlds, and here is where it earns its keep: a native entry has state the program cannot see, and the World keeps it across rebuilds by keying on position and tag.

`Gtk.Entry` shows why the native route is the harder one for the language. Its text is state the native widget owns, so the tag has both a `state: Text` the World pushes on every keystroke and a `requests: !EntryRequest` for the program to set it, and the program has two sources of truth to reconcile, which is exactly the situation the Prism text field of chapter 2 avoids by owning the text itself and letting the World see only a scene and a focusable name. The native wrapper is the right World for a program that wants the platform's own widgets, and it is a different World with a different library, and which is better is a library question. The language has an opinion only about the structure: leaves are elements, containment is interception, and `Gtk.Window(children=[[...]])` holding its children as data is the container convention the Test World's depth rule already understands.

## The X11-over-sockets World

The strongest way to show that GUI is not a World concept is to run the GUI library under a World that has no idea what a window is. This World provides sockets and files and nothing else. The X11 protocol, which is bytes over a socket, is Prism library code, and the GUI programs that run under the software rasteriser run under it unchanged.

### What the World provides

```prism
type Address = choice:
    Tcp Text Nat
    Unix Text

type SocketEvent = choice:
    Received Bytes
    Closed

type SocketState = choice:
    Connecting
    Connected
    Disconnected Text

tag Net.Socket of Element:
    address: Address
    requests: !Bytes
    responds: SocketEvent
    state: SocketState
```

The World is the tokio skeleton of the previous section with the `Sockets` component (extended to connect to a Unix socket when the address says so) and the `Files` component, for reading `~/.Xauthority` (that path, and the socket path `/tmp/.X11-unix/X0` below, from memory). Its entire component list is those two plus the core's `Defer`. It has never heard of `Gui`.

### What the library does

`@X11.Display` is an interceptor. From the inside it offers the `Gui` vocabulary: its child creates `Gui.Window`, `Gui.Scene`, `Gui.Focusable` and `Gui.Metrics` slots and gets what those tags promise. From the outside it makes one `Net.Socket` element and one `Fs.Contents` element, and everything between is Prism.

```prism
@X11.Display: fn @a -> @a = \|@child| @widget:
    !start <- @started
    $auth <- @Element Fs.Contents(path=HomePath ".Xauthority")
    ((result, $slots)) <- intercept @child, !down, $ supply
    ((!sock, $state)) <- @Element Net.Socket(address=(Unix "/tmp/.X11-unix/X0"), requests=!out)

    # The connection: a hold of the parser's state across chunks.
    $conn <- X11.Initial then X11.Step $conn, $auth, $slots, !sock
    let !events = X11.Events $conn, !sock                  # parsed Expose, KeyPress, ButtonPress, ...

    # Up: the setup request first (the World queues bytes until the socket connects),
    # then CreateWindow for the Gui.Window and a PutImage of the rasterised scene
    # whenever the draw list or the connection changes, then replies to Expose.
    let !out = (X11.Setup $auth) on !start
             | (X11.Render $slots, $conn) on (!events | !later)
             | X11.Reply $conn, !events

    # Down: a pointer event and a key both go to the Gui.Window slot,
    # by the list as the occurrence opened. Nothing is hit-tested here.
    !later <- @Element Defer(requests=(unit on !events))
    let !down = (X11.Route $slots, $conn) on !later
    let supply = \|e| match e:
        Gui.Metrics(text=t, font=f): @ $ (X11.Measure t, f)
        ignore: @Element e               # Window, Scene and Focusable are read from $slots; NonGui forwards the rest
    @ result
```

`X11.Step` is a pure function from the connection state, the authority file, the current slot list and a received chunk to the new connection state: it parses the setup reply, tracks the sequence numbers, and keeps the window id. `X11.Render` is a pure function from the slot list to the bytes of a `PutImage` request: it takes the `draw` list of the `Gui.Scene` whose `window` names the `Gui.Window`'s title, rasterises the `Draw.Rect`, `Draw.Text`, `Draw.Image` and `Draw.Clip` values into a pixel buffer in Prism (a rasteriser is a pure function over a list of draw values, and glyphs are a bitmap font held as data), and encodes the request. `X11.Route` turns a parsed `ButtonPress` or `MotionNotify` into a `Pointer` response to the `Gui.Window` slot, a `KeyPress` into a `Pressed` response to the same `Gui.Window` slot, a `ConfigureNotify` into `Resized` and the window manager's close message into `CloseRequested`, each keyed by the child's path, which `for` consolidates into `!(Path ==> Response)`. A key with no focused input is dropped by the library above, as in the rasteriser World, and the interceptor does not look at `Gui.Focusable` for routing. `X11.Measure` is the bitmap font's advance table, the same data `X11.Render` draws with, so a label is measured with the glyphs it will be painted with. This interceptor does not produce `Composition` keys, which would need X input methods, and it does not answer `Gui.Frames`: the slot reaches the World, which has no component for it, and `Unsupported` says so. An animated program under this World needs one more component, a clock.

The `Defer` is there for the reason every interceptor needs one: an event arrives on the socket in occurrence n, and the response to a child is a response, which cannot be in the same occurrence as the event that caused it. The interceptor fires `Defer` and routes when it comes back, in n+1 of the same turn; from the outside World's view, one chunk arrived and one turn happened.

Everything else is encoding and decoding bytes, which is a pure function over `Bytes` and so well within the language. The library is longer than it should be because the prelude's `Bytes` operations are thin today: fixed-width integer packing, slicing and padding are written by hand where a fuller `Bytes` module would give them as one call each. That gap is in the prelude, not a limit of purity. The library is long and the interceptor is short, and that is the right proportion.

### What it proves, and what it does not

It proves that the World boundary is where the design says it is. A World provides effects; the GUI library is a set of interceptors over tags; and a program written against the `Gui` vocabulary runs under the rasteriser World, the Test World and the X11 World without a line changed, because in each case something speaks the vocabulary and the program cannot tell what. It proves that a World with one component is a World, and that `Net.Socket`'s combination of a request event, a response event and a state dynamic in one tag is enough to drive a real protocol. It proves that `Defer` is the only "later" an interceptor needs, including for routing input to the window. And it proves the sandbox argument from the other side: an interceptor that offers a window to its child while making only a socket itself is also an interceptor that could offer a fake window, or a logged one, or a window on a remote display, and the child would not know.

It does not prove that this is a good way to build a GUI. A `PutImage` of the damaged region per keystroke over a Unix socket is fine for a text field and bad for a video; the rasteriser in Prism is slower than one in Rust; and X11 is the protocol chosen because it is a bytes-over-socket protocol with a window in it, not because anyone should target it in 2026. The point is what it demonstrates about the boundary, and a World that speaks Wayland or a GPU is a World with a different component set and the same core.

## Conformance tests for the core

World conformance is tested per World, in Rust, against the world core, and the core itself is tested with a fake component whose every method records what it was called with and does what the test says. The tests are the obligations of the specification, one each, with names that are sentences from it.

```rust
#[derive(Default)]
struct Fake {
    tags: Vec<Tag>,
    calls: Rc<RefCell<Vec<Call>>>,                 // everything the core did to it
    oracle_answers: BTreeMap<ElementValue, Value>,
    to_push: Vec<(ElementValue, Value)>,            // delivered at the next gather
    to_respond: Vec<(Path, Value)>,                 // delivered at the next gather
}

#[derive(Debug, PartialEq)]
enum Call {
    Created(Path, ElementValue),
    Removed(Path, ElementValue),
    Perform(Path, ElementValue, Value),
    Oracle(ElementValue),
    Gather,
}

impl Component for Fake { /* records every call; answers from the maps */ }
```

The programs under test are hand-written `IrWidget`s or compiled from Prism source where the frontend is available; the tests below assume source. Each test is a `World` with a `Fake`, a `Harness` with `Drain`, and assertions on `calls` and on `TurnOutcome`.

```rust
#[test]
fn a_defer_fired_in_an_occurrence_is_answered_in_the_next_occurrence_of_the_same_turn() {
    let (mut h, fake) = world_with(r#"
        @main:
            !b <- @Element Ui.Button(label="go")
            !later <- @Element Defer(requests=(unit on !b))
            $text <- "waiting" then "done" on !later
            @Element Ui.Label(text=$text)                 # lifts to a dyn: a fresh label slot per change
            @ unit
    "#);
    h.start();
    send(&mut h, "Ui.Button(label=\"go\")", click());
    let out = ran(h.step());
    assert_eq!(out.occurrences.len(), 2);
    assert!(out.occurrences[0].requests.iter().any(|(_, e, _)| e.tag == Tag::DEFER));
    assert!(out.list.iter().any(|(_, e)| e.matches("Ui.Label(text=\"done\")")));
}

#[test]
fn a_turn_with_two_defers_runs_three_occurrences() { /* two independent Defer chains; occurrences == 3 */ }

#[test]
fn a_defer_that_fires_again_after_being_answered_is_an_error() {
    let (mut h, _) = world_with(r#"
        @main:
            !b <- @Element Ui.Button(label="go")
            !later <- @Element Defer(requests=(unit on !b | unit on !later))
            @ unit
    "#);
    h.start();
    send(&mut h, "Ui.Button(label=\"go\")", click());
    match h.step() {
        Step::Error(WorldError::DeferBackwards { when, .. }) => assert_eq!(when.occurrence, 3),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_late_result_for_a_gone_slot_is_discarded_with_a_diagnostic() {
    let (mut h, fake) = world_with(r#"
        @main:
            !b <- @Element Ui.Button(label="drop")
            $show <- True then False on !b
            dyn $show:
                match _:
                    True:
                        !s <- @started
                        !r <- @Element Fs.File(path="x", requests=(Read on !s))
                        @ unit
                    False: @ unit
            @ unit
    "#);
    h.start();
    let read = fake.perform_calls().remove(0);           // Perform(path, Fs.File(..), Read)
    send(&mut h, "Ui.Button(label=\"drop\")", click());  // tears the slot down
    ran(h.step());
    assert!(fake.calls().contains(&Call::Removed(read.0.clone(), read.1.clone())));
    h.push(Delivery::Response { path: read.0.clone(), value: contents("late") });
    let out = ran(h.step_or_idle());                      // nothing to run but the diagnostic
    assert!(diagnostics(&h).iter().any(|d| matches!(d, Diagnostic::LateResult { path, .. } if *path == read.0)));
    assert_eq!(out.occurrences.len(), 0);
}

#[test]
fn a_slot_created_in_the_push_occurrence_reads_the_previous_snapshot_and_receives_the_push() {
    let (mut h, fake) = world_with(r#"
        @main:
            $t <- @Element Clock
            !b <- @Element Ui.Button(label="new")
            $n <- 0 then $n + 1 on !b
            dyn $n:
                !s <- @started
                $t2 <- @Element Clock
                $first <- (Instant 0) then $t2 on !s            # the tag law: the value before this occurrence
                @Element Ui.Label(text=$"first=$first now=$t2")
                @ unit
            @ unit
    "#);
    fake.answer(clock(), instant(5000));
    h.start();
    fake.push_next(clock(), instant(5100));
    h.push(Delivery::Response { path: find(&h, "Ui.Button(label=\"new\")"), value: click() });
    let out = ran(h.step());
    assert!(out.list.iter().any(|(_, e)| e.matches("Ui.Label(text=\"first=5000 now=5100\")")));
    assert_eq!(fake.oracle_calls(), vec![clock()]);   // asked once, shared by value
}

#[test]
fn a_request_on_a_tag_no_component_accepts_gets_unsupported_in_the_next_turn() {
    let (mut h, _) = world_with_tags(&[], r#"
        @main:
            !s <- @started
            !r <- @Element Http.Get(url="x", requests=(Go on !s))
            $msg <- "" then Describe !r                       # Describe: Unsupported -> "nope", else "ok"
            @Element Ui.Label(text=$msg)
            @ unit
    "#);
    let out0 = ran(h.start());
    assert!(!out0.list.iter().any(|(_, e)| e.matches("Ui.Label(text=\"nope\")")));
    let out1 = ran(h.step());                               // the Unsupported response, delivered in the next turn
    assert!(out1.list.iter().any(|(_, e)| e.matches("Ui.Label(text=\"nope\")")));
}

#[test]
fn a_state_tag_no_component_accepts_is_an_error_in_the_occurrence_the_slot_is_created() {
    let (mut h, _) = world_with_tags(&[], "@main:\n    $v <- @Element Process.Args\n    @ unit");
    match h.start() {
        Step::Error(WorldError::Runtime { error: RuntimeError { kind: ErrorKind::UnsuppliedState, .. }, .. }) => {}
        other => panic!("{other:?}"),
    }
}

#[test]
fn request_firings_are_performed_in_occurrence_order_then_list_order() {
    // two Cli.Stdout slots, the second earlier in the list than the first fires in a Defer occurrence
    // ... assert fake.perform_calls() order is [occurrence 1: list order..., occurrence 2: list order...]
}

#[test]
fn a_request_from_a_slot_torn_down_in_the_same_occurrence_is_performed_and_its_response_discarded() {
    // a dyn child that fires Fs.File Write on the click that removes it:
    // Perform is called; the fake's response is a LateResult diagnostic
}

#[test]
fn removals_are_reported_before_creations() {
    // a dyn rebuild; assert the Removed call precedes the Created call in `calls`
}

#[test]
fn two_responses_to_one_path_in_a_batch_become_two_turns_in_order() {
    // push Chunk "a" and Chunk "b" to one Cli.Stdin path; Drain policy; two Ran steps, "a" first
}

#[test]
fn a_turn_with_nothing_queued_does_not_run_and_a_clock_alone_does_not_cause_one() {
    // a program with only a Clock slot; step() is Quiescent, never Ran
}

#[test]
fn an_abort_in_a_defer_occurrence_performs_the_firings_of_the_occurrences_that_committed() {
    // occurrence 1 fires a Cli.Stdout write and a Defer; occurrence 2 cycles:
    // Perform(write) is in `calls`, and step() is Error
}

#[test]
fn pushes_of_an_aborted_first_occurrence_are_delivered_in_the_next_turn() {
    // a click that cycles, with a Clock push in the same batch: after Error, the
    // next turn's first occurrence delivers the push and every Clock slot agrees
}

#[test]
fn an_aborted_occurrence_leaves_the_list_and_the_snapshot_as_they_were() {
    // a program that cycles on the second click; after Error, a third click sees the state after the first
}
```

The body of each is short enough to read in the failure message. The last one is the rollback obligation seen from the World: the core applies a turn's pushes and swaps the snapshot only after `run_occurrence` returns `Ok`, and `slots.last_list` is likewise updated only on success, so a failed occurrence leaves the World's view of the program exactly where it was, and the next turn addresses the list the program still has.

A second group tests the harness: `OneAtATime` takes one delivery per `step`; `Drain` takes all; `wait` returns at the deadline with nothing queued; `Exit` is returned only after the turn's performs; the waker is called once per delivery from another thread. Each is a few lines.

## CLI golden transcripts

The CLI World's tests run the real binary against the real OS. Each case is a directory:

```
tests/cli/line_counter_many/
    program.prism
    args            # one per line
    env             # KEY=VALUE per line, replaces the environment
    stdin           # bytes
    chunk           # optional: max bytes per stdin chunk, for the --stdin-chunk flag
    fs_cap          # optional: max bytes an Fs.Contents read returns, for the --fs-cap flag
    expected.stdout
    expected.stderr
    expected.code
```

The runner compiles the program, spawns `prism-cli --stdin-chunk=N program.prism args...` with the given environment and stdin piped from the file, collects stdout, stderr and the exit code, and diffs. There is no transcript of turns in a golden, because a CLI program's observable behaviour is its output and exit code, and a test that pinned the number of turns would be pinning how the OS split a pipe. Where chunking matters, the `--stdin-chunk` flag makes it deterministic: with `chunk` set to 4, `"one\ntwo\nthree\n"` arrives as four chunks and the line counter's partial-line hold is exercised against the real reader thread.

The cases, and what each pins:

| Case | Pins |
| --- | --- |
| `line_counter_{empty,one,many}` | one turn per chunk; output after the turn; exit by `Process.Exit` with code 0 |
| `line_counter_split` (`chunk=4`) | the partial line held across chunks; the count is chunk-independent |
| `falls_off_the_end` | a program that prints and never exits ends with code 0 when stdin ends and nothing is in flight |
| `exit_code` | `Process.Exit` with a computed nonzero code; a code over 255 clamped and reported on stderr |
| `two_exits` | the first firing in occurrence-then-list order wins; the second is a `note:` on stderr |
| `output_order` | two `Cli.Stdout` slots firing in one occurrence, in list order; stdout then stderr across turns, each flushed per turn |
| `stdout_before_exit` | a write and an exit fired in one occurrence: the write lands |
| `args_env` | `Process.Args` and `Process.Env` printed from the build turn, with no args, several args, and an empty environment |
| `file_then_stdin` | `Fs.File` read on `@started`, then `Cli.Stdin` built inside a `dyn`; stdin is not read until the slot exists (the harness checks the pipe is still full after the file read's turn) |
| `unsupported` | a `Ui.Button` at the root: `note:` on stderr naming the chain, and the `Unsupported` response reaching a `Cli.Stdout` write |
| `unsupplied_state` | a `Gui.Metrics` state element at the root: the error on stderr with span, exit nonzero, nothing on stdout |
| `signal_handled` | a `Cli.Signals` slot; the harness sends `SIGINT` after reading a `ready` line from stdout; the program prints `interrupted` and exits 3 |
| `signal_default` | no `Cli.Signals` slot; `SIGINT` kills the process with the OS's status |
| `closed_pipe` | stdout piped to a reader that closes early; `EPIPE` is a `note:`, the program keeps running and exits by its own `Process.Exit` |
| `stdin_slot_dropped` | a `Cli.Stdin` slot removed after the first line and rebuilt after a file read; the chunk read in between reaches the new slot first and no line is lost or repeated |
| `fs_contents_reasons` | `Fs.Contents` on a missing path, an unreadable one, a directory and a file past `--fs-cap`: `NotFound`, `Denied`, `Other` and `TooLarge` printed from the build turn |
| `fs_contents_watch` | an `Fs.Contents` viewer; the harness rewrites the file after reading a `ready` line; the program prints the new contents and exits |
| `cycle_at_runtime` | a program that forms a cycle on the second line of input; the error on stderr with both spans, exit nonzero, the first line's output present |

Timers are not in the goldens. A test that waits on wall-clock time is a test that fails on a loaded machine, and everything a timer does to a program is tested under the Test World with `Send Time.Every(..), tick` and `push Clock = ...`. What the CLI World's timer component does with the OS, that `wait` returns at the deadline, is one of the harness tests above with a deadline of a few milliseconds and a generous tolerance.

The `ready`-line handshake in the signal and watch cases is the one place the goldens synchronise with the program, and it is a handshake the program is written to perform: print `ready` after the slot exists. The harness reads stdout until it sees the line, then acts. That is deterministic because stdout is flushed per turn, and the turn that created the slot is the turn that printed the line.

Every case in the table is also a Prism program that runs under the Test World with a script, where it runs in milliseconds with no process spawned, and the CLI golden is the one layer that checks that the real reader thread, the real buffered writer and the real signal handler do what the Test World's script assumed. When the two disagree, the CLI World is wrong, because the Test World is the specification's reading of what a World is; a disagreement that turns out to be the specification being unclear is the more valuable find, and it gets fixed in the specification.
