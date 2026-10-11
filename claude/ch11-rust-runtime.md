# Chapter 11 — The Rust runtime

The Rust runtime turns a built graph into occurrences. It holds events, behaviors and dynamics as nodes with heights; it runs one occurrence per call by popping heights in order; it commits every hold at the end; and it owns the tree of widget nodes that is the whole of a program's state. The whole of it is a height-ordered action map, a commit list, weak subscriber lists and one synchronous call; everything else here is either the code that exists or the consequences of that boundary.

## What the runtime does not need

You have written one of these, or read one, and you know the list. There is a scheduler, with a queue and a policy about what runs next and a worry about starvation. There is a dirty-tracking protocol, because derived values have to find out that their inputs moved. There are re-entrancy guards, because a setter notifies a listener that calls a setter, and sooner or later one of those calls arrives while the first is half done. There is a way to defer effects until the batch is finished, because an effect run in the middle of a batch sees a mixture. There is an unsubscribe story, with its leaks and its double frees. And at the end of the list there is either a tracing collector or a long conversation about weak references, because listeners point at the things they listen to and the things point back. Almost all of this is absent from the Prism runtime, and every absence is bought by a refusal the language already made. The refusals are in the goals as the language's negative space; this section is the same negative space seen from the implementation: each item is a mechanism that does not have to be written, tested, or debugged at three in the morning.

**No scheduler.** A scheduler decides what runs next when several things could. Here nothing could. There is no sampling, so a node cannot ask for a value before its source has fired or declined to, and there is no prompt switching, so a node's set of sources is fixed for the whole of an occurrence. Together these mean every node's position in the order is a function of its prompt sources and nothing else: a height, one more than the greatest height among the events it is computed from. Propagation is "pop the lowest height, run everything at it, repeat." There is no priority, no fairness and no policy. Two programs that differ only in the order their bindings were written propagate identically.

**No dirty tracking.** A dirty-tracking protocol watches writes. There are no writes. A dynamic changes only when its change event fires, and the change event is a node in the same graph as everything else, at a height, run by the same loop. A derived dynamic is a function of its inputs' values and a leftmost of their change events, so "has my input moved" is answered by "did its change event fire in this occurrence," which the graph already knows. The pull side of a dynamic, its behavior, does keep a cache and does invalidate it, but that invalidation is driven by the commit of a hold at the end of the occurrence and is the only kind there is.

**No re-entrancy guards, except one.** Nothing fires an event from nothing and nothing samples, so the only way propagation can arrive back at a node it is already inside is a cycle in the prompt graph. A cycle shows up as a node whose height is being computed in terms of itself, or as an action scheduled at or below the height currently running. Both are one comparison, and both are errors. The current code makes the comparison with `try_lock` on a mutex the single propagating thread already holds, which reads as a concurrency primitive but is a re-entry flag; the target makes it a `Cell<bool>` per node with the same effect and a message that names the loop.

**No deferred effects.** There are no effects to defer. The only outputs are request firings, which are facts about the occurrence collected into the run call's outcome, and the element list, which is a value the World reads after the call returns. The World acts after the occurrence has committed, in the order the outcome gives. There is no "after this batch" hook because no closure inside propagation can touch anything outside the graph.

**No unsubscribe.** A subscription is a `Weak` pointer in the source's subscriber list. The subscriber is owned by the node that built it, the node by its parent in the widget tree, and when the parent drops the node the subscriber's strong count reaches zero and the weak pointer stops upgrading. The source finds the dead entries the next time it consolidates its list. Teardown is `drop`.

**No collector.** Ownership is a tree because nodes are not values: a program cannot hold a reference to a node, so nothing a program does can make two nodes own each other. The one real threat is a closure. An occurrence-time closure compiled from `$count + !delta` has to reach `$count`'s behavior, and `$count`'s hold has to reach the event that updates it, which is the closure's node; if both links were strong, that is a cycle through the heap that reference counting never frees. The rule that prevents it is that a node owns its closures and its placeholders, and every loop is closed through a placeholder that holds only a `Weak` to its target. The strong ends live in the node; the loop points back at them weakly; dropping the node frees the loop. Closure conversion in the compiler makes this mechanical, because a compiled closure captures by index into its own node's values and never by pointer into a sibling's. Cyclic bindings inside one block are the explicit exception and are owned by the node as a unit, which is why the node has slots for them.

**No locks.** The runtime is single-threaded by design. The current code is written over `Arc` and `Mutex`, and every lock in it is either taken by the one thread that is propagating or is there to detect re-entry, which the height comparison already does. There is no other thread that could contend. The target replaces `Arc` with `Rc`, `Mutex` with `RefCell` and `Cell`, keeps `Runtime: Send` so that a World can move it to a thread of its choosing, and gives up `Sync`, which nothing uses.

**No transaction log, no phases, no runtime-started occurrences.** The occurrence is a transaction, but the undo log is not a separate structure: every write a program can observe is already buffered to commit time for the tag law's sake, so rollback is "drop the buffer." There are no phases inside an occurrence; `Defer` is a World feature built on the run call, and the runtime has no branch for it. And the runtime never opens an occurrence of its own; a rebuild finishing, a slot appearing, a request firing cause nothing until the World calls again. These three absences are what make the run call one synchronous function from inputs to an outcome.

The remainder is small. A graph of nodes with heights; one per-occurrence map from height to a list of actions; a commit list of buffered writes; weak subscriber lists; a tree of nodes holding the strong ends of its loops; and one entry point. That is the whole runtime. It is small enough that writing it twice, once in Rust and once as a mapping onto Reflex, is a reasonable plan rather than a heroic one, and small enough that the rest of this chapter can go through all of it.

## The crate as it stands today

`prism_runtime` is one crate in the workspace, branch `on-fire-repros`, tip `c03a6fd`, edition 2024, with three dependencies (`log`, `serde_json`, `thiserror`) and one dev-dependency (`test-log`). `cargo test -p prism_runtime` runs 37 library tests and one integration test, 38 in all, passing. The crate has the graph and the transaction and does not yet have the boundary: no elements, slots, paths or interceptors, and what it exposes to the outside is a test backend that reaches into nodes by name. This section is the crate as a reference, in the order the modules depend on each other.

```
src/lib.rs
src/value/        Value<T>, AnyValue, PrismValueInner, HeapValue, TypeMismatch
src/runtime/      Runtime, Action, schedule, propagate, the occurrence's state
src/event/        Event<T>, EventImpl, EventCallback, SubscriptionManager,
                  external, filter_map, tag, combine, leftmost, switch_hold,
                  forward, never, trace
src/behavior/     Behavior<T>, BehaviorImpl, hold, compute (map, map2, join), constant
src/dynamic/      Dynamic<T>
src/widget/       Widget, PolymorphicWidget, WidgetBuilder, WidgetNode, Slots,
                  WidgetReadyEvent, DynamicWidget, fold_dyn, AnyEvent/AnyDynamic,
                  WidgetDelegate
src/backends/     test_backend
tests/            runtime_send.rs
```

### Values

Everything that flows through the graph is a `Value<T>`, a typed view onto an untyped `AnyValue`:

```rust
pub trait ValueType: PrismValueInner + Clone + Debug + Sync + Send + 'static {}

#[derive(Clone)]
pub struct Value<T: ValueType> { value: AnyValue, phantom: PhantomData<T> }

#[derive(Clone)]
pub struct AnyValue(AnyValueInner);

#[derive(Clone)]
enum AnyValueInner {
    Arc(Arc<dyn HeapValue>),
    U32(u32),
    I32(i32),
    Unit,
}

pub type PrismArc<T> = Value<Arc<T>>;
```

`Value<T>` derefs to `T::Ref` (the `T` itself for immediates, the pointee for `Arc<T>`); `Value::get(self) -> T` unwraps; `From<T>` wraps; `PrismArc::new(t)` and `PrismArc::extract(self) -> T` are the heap conveniences, the second being `Arc::unwrap_or_clone`. `AnyValue::downcast::<T>()` panics on a mismatch; `try_downcast` returns `TypeMismatch { expected, actual }` with two type strings and exists for the test backend. The module doc states the position that matters: Prism is statically typed, the runtime trusts that, and the checked downcast is a debugging aid a release build may one day skip. There is no introspection and no fallible path meant for programs.

`PrismValueInner` says how a Rust type lives inside `AnyValue`:

```rust
pub trait PrismValueInner: Debug + Clone + Sync + Send + 'static {
    type Ref: ?Sized;
    fn get_any(value: &AnyValue) -> &Self::Ref;         // panics on mismatch
    fn try_get_any(value: &AnyValue) -> Result<&Self::Ref, TypeMismatch>;
    fn from_any(value: AnyValue) -> Self;
    fn to_any(self) -> AnyValue;
    fn type_string() -> String;
}
```

Implementations exist for `()`, `u32`, `i32`, `AnyValue` itself (so an erased value is a value), `Arc<T>` for any `T: HeapValue` (`HeapValue` is `Any + Sync + Send + Debug + 'static`, blanket-implemented, with a `type_string` for diagnostics), and three runtime objects as heap values: `Event<T>`, `Dynamic<T>` and `AnyEvent`. The last three make an event of events or a dynamic of widgets expressible as ordinary values, which `switch_hold` and `dynamic_widget` need. A `Value<Event<T>>` is an `Arc<Event<T>>` underneath and `Event<T>` is itself an `Arc<dyn EventImpl<T>>`, so there is a double indirection there that the one-word representation of the values chapter removes. The enum is two words plus a tag (the crate has no `size_of` assertion pinning it), and the representation is deliberately behind the trait so that the rest of the runtime does not notice when it changes.

### The runtime core

This is the whole engine: a map from height to the actions scheduled at it, and a loop that pops the lowest height, runs everything at it, and repeats. Nothing else decides what runs next.

```rust
pub struct Runtime(pub(crate) Mutex<RuntimeInner>);

pub(crate) struct RuntimeInner {
    actions: BTreeMap<usize, Vec<Box<dyn Action>>>,
    pub(crate) height: Option<usize>,
}

pub(crate) trait Action: Send {
    fn act(self: Box<Self>, runtime: &Runtime);
}

pub(crate) struct RuntimeInner {
    actions: BTreeMap<usize, Vec<Box<dyn Action>>>,
    pub(crate) height: Option<usize>,
    clear_at_end: Vec<Box<dyn FnOnce() + Send>>,
}

impl Runtime {
    pub(crate) fn current_height(&self) -> Option<usize>;
    pub(crate) fn clear_at_end(&self, clear: impl FnOnce() + Send + 'static);
}

impl Runtime {
    pub fn new() -> Self;
    pub(crate) fn schedule(&self, height: usize, action: impl Action + 'static);
    pub fn propagate(&mut self);
}
```

The action map is the height-ordered queue: a `BTreeMap` from height to the actions scheduled at it. An occurrence is `propagate`: pop the lowest height, record it as the current height, run the actions at it, repeat until the map is empty. Each action is a boxed object: a `MainAction` that makes a derived event compute and fire, a `TriggerAction` that fires an external event, a `HoldAction` that writes a hold, a `SwitchHoldOuterAction` that retargets a switch, a `DWAction` that rebuilds a `dyn`, a `KeepUntilOccurrenceEnds` that drops a replaced node, a `WidgetReadyTrigger` that schedules done events, and the `FireCombine` and `FireLeftmost` actions of the catch-up path.

`schedule` is where the cycle check lives:

```rust
if let Some(current_height) = this.height && height <= current_height {
    panic!("Scheduling action at height={height} when already at height={current_height}");
}
```

Outside propagation `height` is `None` and anything may be scheduled; inside it, an action may be scheduled only strictly above the height being run. Every derived event schedules itself at its own height, which is above all of its sources, so a scheduling at or below the current height means a node has been reached from above itself: a cycle. `propagate` makes the same check when it pops a height that is not greater than the last, which cannot happen if `schedule` holds and is kept as belt to the braces.

The occurrence's state lives in `RuntimeInner` beside the action map: the current height, and a `clear_at_end` list of closures run when the occurrence ends, one per event that cached a value for this occurrence, so the next occurrence starts clean. Everything that needs that state takes `&Runtime`. `EventImpl::subscribe(runtime, cb)` and `EventCallback::already_fired(runtime, value)` take it because subscribing mid-occurrence has to know which occurrence it is in and schedule its catch-up into it; `Behavior::hold(runtime, initial, event)`, `Dynamic::hold`, `switch_hold` and `trace` take it because they subscribe when they are made. `map`, `filter_map`, `tag`, `leftmost` and `combine` do not take it: they subscribe lazily, when something subscribes to them, and until then they are descriptions with no creation time. That split is Reflex's split exactly. The operations whose meaning depends on when they are made are the ones in `MonadHold` (`hold`, `holdDyn`, `switchHold`, `headE`, `now`); the ones that are pure functions of their inputs are plain functions there too. In Haskell the monad is forced, because a `hold` written as a pure function would be floated and shared by the optimizer and would then mean the wrong thing; in Rust a call happens where it is written, so the parameter is only there to say which runtime and which occurrence. The parameter is also what makes `Runtime` honestly `Send`: an occurrence's state travels with the runtime, not with the thread that happens to be driving it.

Widgets get the handle from `WidgetBuilder::runtime()`, which is why, from the language's side, a hold is written only in a widget block. A hold has a creation occurrence; the only Prism construct with one is a build; and the Haskell implementation runs the typed AST on Reflex, where a hold outside `MonadHold` cannot be typed at all. So `$count <- 0 then $count + !delta` is a statement of a widget block and not an expression that could appear in a `let` at top level, and the runtime API is now shaped so that the Rust side can make the same mistake only by holding a `&Runtime`, which a build or a World has and a leaf closure does not.

Holds commit at `usize::MAX`. Every hold, every `switch_hold` retarget, and the drop of a replaced node schedules its action at `usize::MAX`, so the commit batch is the last thing `propagate` runs and runs only after every event that could fire has fired. Nothing samples a new value early because nothing is new until then. The exit path, when the map is empty, sets the height back to `None` and runs the clear-at-end list.

The panic path is the transaction's abort. Each action runs under `catch_unwind`; on a panic the map is cleared, the height reset, the clear-at-end list drained, and the panic resumed:

```rust
if let Err(panic) = outcome {
    self.0.lock().unwrap().actions.clear();
    self.finish_occurrence();          // height = None; run and drain clear_at_end
    resume_unwind(panic);
}
```

Because holds and switches commit only in the `usize::MAX` batch, a panic before that batch leaves every hold at its value from before the occurrence, which is why this is already a rollback for the values a program can observe. `runtime_keeps_working_after_a_closure_panics` pins it: a mapped closure panics on the value 1, the caller catches the panic, the next value is 2, and the hold ends at 2. A trigger scheduled in the aborted occurrence is released by `TriggerAction`'s `Drop` (below), which `a_trigger_dropped_by_a_panic_accepts_values_again` checks. The panic path does not turn the panic into a value and does not drop the nodes a `dyn` built before the panic; the target's `run` does both.

### Events

An event is a handle to an implementation with exactly two obligations: accept a subscriber, and say its height. It is lazy: it computes whether it fires, and its value, only while someone is subscribed, and an event nobody holds disconnects from its source. Everything else about events is what a subscriber can be told.

```rust
pub struct Event<T: ValueType>(pub(crate) Arc<dyn EventImpl<T>>);

pub(crate) trait EventImpl<T: ValueType>: Send + Sync + 'static {
    fn subscribe(&self, cb: Weak<dyn EventCallback<T>>) -> Option<Value<T>>;
    fn height(&self) -> usize;
}

pub(crate) trait EventCallback<T: ValueType>: Send + Sync {
    fn event_fired(&self, runtime: &Runtime, value: Value<T>);
    fn already_fired(&self, value: Value<T>);
    fn invalidate_height(&self);
}
```

A subscriber has three obligations: be told when the event fires during propagation; be told that it already fired this occurrence, which is the catch-up path; and be told that the event's height may have changed. Subscribers are held weakly. Whoever subscribes is responsible for keeping both the callback and the event alive, and the module doc says why in as many words: this is exactly the bookkeeping FRP exists to take out of user code, and it is kept inside the runtime so that user code never sees it.

`subscribe` returns `Option<Value<T>>`: the event's value in the current occurrence if it has already fired, else `None`. This one line implements the construction rule, that a widget built in the middle of occurrence n behaves as if built just before n's inputs, and it is modelled on Spider's `subscribeAndRead`.

Laziness is why a `dyn`'s output event needs a keep-alive (below) and why the test backend holds the events it watches.

**The subscription manager.** Every derived event is built around a `SubscriptionManager`, which holds the subscriber list, the upstream connection, the per-occurrence cache, and the logic that makes catch-up work:

```rust
pub struct SubscriptionManager<T: ValueType, E: SubscriptionEvent<T>> {
    subscribers: Mutex<Vec<Weak<dyn EventCallback<T>>>>,
    subscription: Mutex<Option<Arc<MainSubscription<T, E>>>>,
    tag: E::Tag,
    occurrence: Arc<Mutex<Option<Option<Value<T>>>>>,
}

pub trait SubscriptionEvent<T: ValueType>: EventImpl<T> + 'static {
    type Inner: ValueType;             // the upstream event's type
    type Tag: Clone + Send + Sync;     // which upstream, for multi-source events
    fn invalidate_height(&self);
    fn handle_main_subscription(&self, runtime: &Runtime, value: Value<Self::Inner>, tag: Self::Tag);
    fn handle_early_subscription(&self, _: &Runtime, _: Value<Self::Inner>, _: Self::Tag) {}
    fn handle_already_fired(&self, value: Value<Self::Inner>, tag: Self::Tag);
    fn finish_already_fired(&self) {}
}
```

A derived event `E` implements `SubscriptionEvent` to say what to do with an upstream firing (`handle_main_subscription`, run at `E`'s height as a `MainAction`), what to do with an upstream value that had already fired when `E` subscribed (`handle_already_fired`), and, for events with several sources, what to do with a source's value that arrives before `E`'s turn (`handle_early_subscription`) and how to finish once every source has been consulted (`finish_already_fired`). The manager does the rest. Its pieces, in the order a firing meets them:

`fire(runtime, value)` is what a source calls: record the value for this occurrence, then call `event_fired` on every live subscriber. `notify(this, event, runtime, compute)` is what a derived event calls from its `MainAction`: if there are no live subscribers, disconnect from upstream and return `false`, which is laziness; otherwise compute the value once, record it, deliver it, and make sure the upstream connection exists.

`record_once(compute)` is the per-occurrence cache. If the manager already has a value for this occurrence, or no occurrence is in progress, it returns `None` without calling `compute`; otherwise it calls `compute`, stores the result (which may itself be `None`, for a filter that dropped the value), and registers a clear-at-end closure. The double `Option` is "has this event been computed this occurrence" outside and "did it fire" inside. The effect is that a closure runs at most once per occurrence however many subscribers read its result and whenever they arrive, which `a_closure_runs_once_per_occurrence_even_when_late_subscribers_read_it` pins with a counter.

`add_subscriber(this, event, cb)` is `subscribe`: connect to upstream if not connected, push the weak callback, return `current()`, the cached value if there is one. `connect` is where catch-up starts. When a derived event connects to its upstream mid-occurrence and the upstream has already fired, `resubscribe` returns that value and the manager hands it to `handle_already_fired`, which computes the derived value through `record_once` and delivers it with `deliver_already_fired`. A chain of maps built in the middle of an occurrence therefore catches up link by link, each computing once and caching, and a hold at the end of the chain receives the value it would have received had it existed from the start. `a_widget_built_mid_occurrence_gets_the_mapped_value_of_an_event_that_already_fired` is that sentence as a test, with a `*10` map and a hold that ends at 70.

`MainSubscription` is the callback the manager installs upstream, and it is where height decides the delivery path:

```rust
fn event_fired(&self, runtime: &Runtime, value: Value<E::Inner>) {
    let Some(this) = self.this.upgrade() else { return };
    let height = this.height();
    let tag = self.tag.clone();
    if runtime.current_height().is_some_and(|current| height <= current) {
        this.handle_already_fired(runtime, value, tag);
        this.finish_already_fired(runtime);
        return;
    }
    this.handle_early_subscription(runtime, value.clone(), tag.clone());
    runtime.schedule(height, MainAction { this, value, tag })
}
```

If the derived event's height is above the current height, the firing is scheduled as a `MainAction` at that height, which is the ordinary push path. If its height is at or below the current height, its turn has already passed in this occurrence: it was subscribed late, and the firing is delivered through the catch-up path at once. The invariant that makes this correct is the one heights give: every firing below the current height has already happened and is cached, so a late subscriber reads caches instead of waiting for pushes that have gone by, and a firing that has not happened yet still arrives by push. `a_widget_built_mid_occurrence_sees_the_same_events_at_any_depth` builds a child from a rebuild at height 2 with one hold on the raw input (height 0) and one on a five-deep map chain (height 5), fires 42, and checks both holds end at 42.

`consolidate()` walks the subscriber list, drops dead weak pointers, and returns the live callbacks upgraded. `push_subscriber` consolidates every sixteen additions so that a list that gains and loses subscribers does not grow without bound; the comment admits this stands in for a real weak bag.

**Height invalidation.** Every derived event caches its height in a `Mutex<Option<usize>>` and computes it as one more than its upstream's `height()`, which caches upstream's in turn. When a `switch_hold` moves to a new inner event, every height computed through it is stale, and the rule that keeps invalidation cheap is in the doc of `EventImpl::height`: invalidation stops early at any node whose cache is already empty, and stopping early is safe only while every node that caches a height also caches whatever it computed that height from. A node can only have a cached height if it asked its upstream, which cached upstream's; so if a node's cache is empty, nothing below it holds a height computed through it, and the walk can stop. Every event type follows the rule: `invalidate_height` clears its own cache and, if the cache was not already empty, passes the call to every subscriber. `switching_to_a_deeper_event_keeps_downstream_heights_valid` is the deterministic check: switch from a height-0 inner to a height-4 inner, fire through two more maps, and the hold at the end gets the value instead of a scheduling panic. A randomised version of the same check, random graphs of switches and maps driven for many occurrences under the invariant checker, is not in the crate and belongs with the checker of the target.

**The combinators.** Each is one file and one struct implementing `SubscriptionEvent`.

```rust
impl<T: ValueType> Event<T> {
    pub fn external() -> (Event<T>, EventTrigger<T>);
    pub fn never() -> Self;
    pub fn filter_map<O: ValueType>(&self, f: impl Send + Sync + Fn(Value<T>) -> Option<Value<O>> + 'static) -> Event<O>;
    pub fn map<O: ValueType>(&self, f: impl Send + Sync + Fn(Value<T>) -> Value<O> + 'static) -> Event<O>;
    pub fn tag<B: ValueType>(&self, behavior: impl Into<Behavior<B>>) -> Event<Arc<(Value<T>, Value<B>)>>;
    pub fn tag_map<B: ValueType, O: ValueType>(
        &self,
        function: impl Fn(Value<T>, Value<B>) -> Option<Value<O>> + 'static + Send + Sync,
        behavior: impl Into<Behavior<B>>,
    ) -> Event<O>;
    pub fn combine<A: ValueType, B: ValueType>(
        function: impl Send + Sync + Fn(OneOrBoth<A, B>) -> Option<Value<T>> + 'static,
        a: Event<A>, b: Event<B>,
    ) -> Event<T>;
    pub fn leftmost(events: Vec<Event<T>>) -> Event<T>;
    pub fn trace(&self, label: String);
}
impl<T: ValueType> Event<Event<T>> {
    pub fn switch_hold(&self) -> Event<T>;
}
```

*External.* `Event::external()` returns the event and an `EventTrigger<T>`, a `Weak` handle to the trigger's inner. The inner has a `SubscriptionManager` with no upstream (its `Inner` type is `()` and its `handle_*` methods are `unreachable!()`), a `scheduled: Mutex<bool>` flag, and height 0. Triggers are scheduled through the runtime:

```rust
impl Runtime {
    pub fn schedule_trigger<T: ValueType>(&self, trigger: &EventTrigger<T>, value: Value<T>)
        -> Result<(), ScheduleError>;
}
pub enum ScheduleError { DuringPropagation, AlreadyScheduled }
```

It returns an error rather than panicking in the two cases a caller can plausibly produce: a call from inside a subscriber while the runtime is propagating, and a second value for one trigger before the next `propagate`. The first value wins. `TriggerAction` clears the `scheduled` flag in `Drop`, not in `act`, so the flag is cleared whether the action ran or was thrown away by the panic path; and it releases the trigger's lock before notifying, so a subscriber that reschedules its own trigger gets `Err(DuringPropagation)` instead of a deadlock. Three tests pin these: `scheduling_a_trigger_during_propagation_is_refused_without_a_panic`, `a_subscriber_rescheduling_its_own_trigger_does_not_hang`, `a_second_value_on_one_trigger_before_propagating_is_refused`. In the target the whole trigger API is internal, since the run call is the only thing that fires one.

*Never.* An implementation whose `subscribe` returns `None` and keeps nothing, height 0, one `Arc` per call.

*Filter-map and map.* `FilterMapEvent` holds the upstream event, the closure and a height cache. `handle_main_subscription` is `notify` with the closure; `handle_already_fired` is `record_once` with the closure followed by `deliver_already_fired`. `map` is `filter_map` with `Some`.

*Tag and tag-map.* `Tag` holds the event, a `Behavior<B>` and the closure. On a firing it reads the behavior with `query_for_tag`, which is the value before the occurrence because holds commit at `usize::MAX`, and applies the function. The doc says it in three words: not prompt, never prompt. `tag` is `tag_map` pairing the two values in an `Arc`. This is the tag law as a combinator, and the last steps of `hold_map2_test` pin it: an update and a query in one occurrence, and the query sees the old sum.

*Combine.* `CombineEvent` is the two-source event. Each source is wrapped by `filter_map` into an `Arc<OneOrBoth<A, B>>` so both arrive with one type; there are two managers, one per source, with the first owning the subscriber list that delivers; `one_or_both: Mutex<Option<OneOrBoth<A, B>>>` accumulates what has arrived this occurrence. A source's firing is `accumulate`d early, through `handle_early_subscription`, and the combined firing happens once, at the combine's own height, through `fire`, which takes the accumulator and `notify`s with the function. The catch-up version, `finish_already_fired`, handles a subscription made mid-occurrence when one source has fired and the other might still: if the combine's height is above the current height it defers a `FireCombine` action to its height, since the other source may yet fire; otherwise both sources are behind it and it computes at once. Its `height` is the maximum of the two sources plus one, recomputed on every call; it writes a cache it never reads, a known issue.

*Leftmost.* `LeftmostEvent` is `combine` over a vector with a winner instead of a pair: one manager per source, each tagged with its index, and `compete` keeps the value with the lowest index seen so far. Every manager connects on subscribe, and the subscriber is pushed onto every manager's list, but only the first manager's list is used to deliver (the others' copies are consulted only by `invalidate_height`, so a subscriber is told of a height change once per source, which is harmless and wasteful). This is `|` and `!Leftmost`; `leftmost_test` checks that left wins in both orders of arrival.

*Switch-hold.* `SwitchHold` is `!Switch`:

```rust
struct SwitchHold<T: ValueType> {
    subscriber_list: SubscriptionManager<T, Self>,
    inner_event: Mutex<Option<Event<T>>>,
    _outer_event: Event<Event<T>>,
    _outer_sub: Arc<SwitchHoldOuterCallback<T>>,
    weak_self: Weak<Self>,
    height: Mutex<Option<usize>>,
}
```

It subscribes to the outer event with a callback that, when the outer fires, schedules a `SwitchHoldOuterAction` at `usize::MAX`: the inner is replaced only at commit, so an inner delivered in n is heard from n+1. The action installs the new inner, clears the switch's height cache, resubscribes the manager to the new inner if anyone is listening, and calls `invalidate_height` on every subscriber, which is the walk above. The switch's height is the current inner's height plus one, or 0 with no inner. The `already_fired` path of the outer callback, for a switch built mid-occurrence over an outer that already fired, defers the same action to `usize::MAX`, so a switch built in n adopts an inner delivered in n at n's commit, exactly as one that existed before n would. `switch_hold_test2` is the law test: outer and inner firing together produce nothing, the next inner firing produces the sum, and two outer firings in a row replace the inner twice.

*Forward.* `ForwardEvent` is the placeholder behind cyclic bindings:

```rust
pub(crate) struct ForwardEvent<T: ValueType> {
    inner: OnceLock<Weak<dyn EventImpl<T>>>,
    subscriber_list: SubscriptionManager<T, Self>,
    weak_self: Weak<Self>,
    height: Mutex<Option<usize>>,
}
impl<T: ValueType> ForwardEvent<T> {
    pub(crate) fn new() -> Arc<Self>;
    pub(crate) fn close(&self, event: &Event<T>);
}
```

Before `close` it has no target and height 0. `close` stores a `Weak` to the target and, if anyone already subscribed, resubscribes to the target and delivers the target's already-fired value if there is one, so a cyclic event closed after its subscribers attached hands them what the closing event fired in this occurrence. Its height is the target's plus one, cached and invalidated like any other. The target is held weakly for the sake of loop ownership, under widgets below.

*Trace.* `Event::trace(label)` subscribes a logging callback at `log::trace!` level and `mem::forget`s it. The doc says it leaks on purpose and violates many design principles; it is a debugging aid and a known issue.

### Behaviors

A behavior is the pull half of push-pull FRP: a value you ask for, computed on demand and cached until something it depends on changes. There are two ways to ask, and the difference between them is the only dirty-tracking in the runtime.

```rust
pub struct Behavior<T: ValueType>(pub(crate) Arc<dyn BehaviorImpl<T>>);

pub(crate) trait BehaviorImpl<T: ValueType>: Send + Sync {
    fn query_for_behavior(&self, dependent: Weak<dyn BehaviorDependent>) -> Value<T>;
    fn query_for_tag(&self) -> Value<T>;
}
pub(crate) trait BehaviorDependent: Send + Sync { fn invalidate(&self); }
```

`query_for_tag` is what a `tag` does: give me the value, I will not be told when it changes. `query_for_behavior(dependent)` is what a derived behavior does: give me the value and remember me, so that when you are invalidated you invalidate me. The dependents list tracks one thing, the commit of a hold.

```rust
impl<T: ValueType> Behavior<T> {
    pub fn constant(value: Value<T>) -> Self;
    pub fn hold(initial: Value<T>, event: Event<T>) -> Self;
    pub fn map<O>(&self, f: impl Send + Sync + Fn(Value<T>) -> Value<O> + 'static) -> Behavior<O>;
    pub fn map2<A, B>(f: impl Fn(Value<A>, Value<B>) -> Value<T> + 'static + Sync + Send,
                      a: Behavior<A>, b: Behavior<B>) -> Self;
    pub fn computation_behavior(computation: impl BehaviorComputation<T> + 'static) -> Behavior<T>;
    pub fn query_for_computation(&self, dep: BehaviorDependencyTracker) -> Value<T>;
}
impl<T: ValueType> Behavior<Arc<Behavior<T>>> {
    pub fn join(&self) -> Behavior<T>;
}
pub trait BehaviorComputation<O: ValueType>: Send + Sync {
    fn compute(&self, dependent: BehaviorDependencyTracker) -> Value<O>;
}
```

*Hold* is the transaction:

```rust
struct HoldBehavior<T: ValueType> {
    value: Mutex<Value<T>>,                 // the held value; not a cache
    _event: Event<T>,                       // keeps the event alive
    weak_self: Weak<Self>,
    dependents: Mutex<Vec<Weak<dyn BehaviorDependent>>>,
}
impl<T: ValueType> EventCallback<T> for HoldBehavior<T> {
    fn event_fired(&self, runtime: &Runtime, value: Value<T>) {
        runtime.schedule(usize::MAX, HoldAction { hold: this, value })
    }
    fn already_fired(&self, runtime: &Runtime, value: Value<T>) {
        runtime.schedule(usize::MAX, HoldAction { hold: this, value })
    }
    fn invalidate_height(&self) {}          // a behavior's height is always usize::MAX
}
```

`Behavior::hold(runtime, initial, event)` subscribes the hold to its event; if `subscribe` returns an already-fired value (the hold was built mid-occurrence after its event fired) it schedules a `HoldAction` at `usize::MAX` at once. Either way the write happens in the commit batch: `HoldAction::act` stores the value and invalidates every dependent, which drops their caches so that the next query recomputes. Between the firing and the commit every `query_for_tag` sees the old value, which is the tag law; after the commit every query sees the new one, which is the change law on the behavior side. A hold built in the occurrence its event fires adopts the value at that occurrence's commit, which is the last clause of the hold law and the test `fold_dyn_counts_an_event_from_the_build_occurrence`: a count that starts at 0, folds the widget's done event, and reads 1 after occurrence zero.

*Computed* behaviors, `DependentBehavior<O, C>`, hold a `BehaviorComputation` and a `cache: Mutex<Option<(Value<O>, Dependents)>>`. On a query with an empty cache they run `compute`, which queries the computation's own behaviors with a tracker pointing back at this one, so each upstream records this behavior as a dependent; the result and the dependents that asked for it are cached together. `invalidate` takes the cache and invalidates the cached dependents. `map` and `map2` are two small computations; `join` is a third, which queries the outer for the current inner and then the inner. The seven `joined_behavior_*` tests pin the behavior half of `$Join`: it starts at the initial inner's value; follows a new inner and ignores the old one afterwards; takes the new inner's new value when the swap and the change land in one occurrence; ignores the old inner changing in the swap occurrence; shows the previous value when sampled mid-occurrence; follows a computed inner; and keeps its dependents bounded across five hundred swaps.

Two details keep the dependents lists honest. Each `query_for_behavior` first retains only entries that are alive and are not the dependent being added, then pushes, so a live dependent appears at most once (`a_behavior_that_never_changes_does_not_collect_dependents`: a `map2` whose one side never changes stays under ten weak entries over a thousand occurrences). And the cache lock is a `try_lock` that panics with "behavior cycle: a behavior depends on itself" when a computation reaches back into itself, which is the re-entry check on the pull side (`a_dynamic_defined_as_a_function_of_itself_is_reported_instead_of_hanging`).

*Constant* is a value and nothing else; it records no dependents because it never invalidates.

### Dynamics

A dynamic is the pair, with the invariant that the event fires exactly when the behavior changes and with the value the behavior then holds. The one fact that makes the pair cheap is that a hold's change event *is* its input event: nothing is built to make a hold a dynamic.

```rust
pub struct Dynamic<T: ValueType> { event: Event<T>, behavior: Behavior<T> }

impl<T: ValueType> Dynamic<T> {
    pub unsafe fn new(behavior: Behavior<T>, event: Event<T>) -> Dynamic<T>;
    pub fn event(&self) -> Event<T>;
    pub fn behavior(&self) -> Behavior<T>;
    pub fn hold(initial: Value<T>, event: Event<T>) -> Dynamic<T>;
    pub fn constant(val: Value<T>) -> Dynamic<T>;
    pub fn map<O>(&self, f: impl Send + Sync + 'static + Clone + Fn(Value<T>) -> Value<O>) -> Dynamic<O>;
    pub fn map2<A, B>(a: Dynamic<A>, b: Dynamic<B>,
                      f: impl 'static + Clone + Send + Sync + Fn(Value<A>, Value<B>) -> Value<T>) -> Self;
}
```

`new` is marked `unsafe` to guard the invariant, which is logical rather than memory-related, a known issue. `hold` pairs `Behavior::hold` with the event itself, which is why a hold's change is prompt in the occurrence its event fires even though its value side commits at the end. `constant` pairs a constant behavior with `never`. `map` maps both halves. `map2` is the change law for two operands: the behavior is `Behavior::map2`, and the event is `combine` over `a.event.tag(b.behavior)` and `b.event.tag(a.behavior)` with the `Both` case taking both new values; so the derived dynamic changes once in an occurrence where either or both operands change, with both new values, which is glitch freedom. `event()` is already the accessor that `!Updated` compiles to. There is no `Dynamic::join`; its design is in the target section.

### Widgets

A widget is a description with a `build` method; a node is what `build` produces through the builder. One widget value bound twice is two nodes, and a `dyn` that rebuilds into a widget of another kind needs the two kinds to be one Rust type to the runtime, which is what the polymorphic form is for.

```rust
pub trait Widget: Send + Sync {
    type Output: ValueType;
    fn build(&self, builder: &mut WidgetBuilder) -> Self::Output;
}

pub trait PolymorphicWidget<O: ValueType>: Sync + Send {
    fn build(&self, builder: &mut WidgetBuilder) -> O;
}
pub struct PolymorphicWidgetAdapter<O: ValueType, W: Widget<Output = O>>(W);
impl<O: ValueType> Widget for Arc<dyn PolymorphicWidget<O>> { type Output = O; ... }
```

`PolymorphicWidget` is the `dyn`-compatible form, and `Arc<dyn PolymorphicWidget<O>>` implementing `Widget` is what lets every compiled widget block be one Rust type to the runtime and lets a `dyn` rebuild into a widget of a different kind. The adapter has a private field and no constructor, so nothing outside its module can make one; the blanket `impl<W: Widget> PolymorphicWidget<W::Output> for W` replaces it and compiles.

**The builder.**

```rust
pub struct WidgetBuilder<'a> {
    node: WidgetNode,
    children: Vec<Arc<WidgetNode>>,
    pub(super) done_event: Arc<WidgetReadyEvent>,
    runtime: &'a Runtime,
}

impl WidgetBuilder<'_> {
    pub fn build_root<W: Widget>(runtime: &Runtime, widget: &W, height: usize) -> (Arc<WidgetNode>, W::Output);
    pub fn bind<W: Widget>(&mut self, widget: &W) -> W::Output;
    pub fn done_event(&self) -> Event<Arc<WidgetNode>>;
    pub fn add_cyclic_dynamic<T: ValueType>(&mut self, name: String) -> (usize, Dynamic<T>);
    pub fn close_cyclic_dynamic_by_index<T: ValueType>(&mut self, ix: usize, dynamic: Dynamic<T>);
    pub fn close_cyclic_dynamic_by_name<T: ValueType>(&mut self, name: &str, dynamic: Dynamic<T>);
    pub fn add_cyclic_event<T: ValueType>(&mut self, name: String) -> (usize, Event<T>);
    pub fn close_cyclic_event_by_index<T: ValueType>(&mut self, index: usize, event: Event<T>);
    pub fn close_cyclic_event_by_name<T: ValueType>(&mut self, name: &str, event: Event<T>);
    pub fn add_external_event<T: ValueType>(&mut self, name: String) -> Event<T>;
    pub fn add_public_event<T: ValueType>(&mut self, name: String, event: Event<T>);
    pub fn add_public_dynamic<T: ValueType>(&mut self, name: String, dynamic: Dynamic<T>);
    pub fn fold_dyn<A: ValueType, B: ValueType>(&mut self, name: String,
        f: impl 'static + Sync + Send + Fn(Value<A>, Value<B>) -> Value<A>,
        initial: Value<A>, update: Event<B>) -> Dynamic<A>;
}
```

`build_root` makes a node for `widget` with no parent: it creates a `WidgetReadyEvent` with a trigger at `height + 1`, calls `build`, wraps the node in an `Arc`, tells the ready event its node, installs the children, and schedules the trigger at `height`. A World calls it with height 0 before the first `propagate`; a `dyn` rebuild calls it from inside an action with the action's height plus one. `bind` is the same for a child: a sub-builder whose ready event shares the parent's trigger, `build`, the child node pushed onto the parent's children. In both, `set_node` on the ready event happens as soon as `build` returns, before any occurrence could reach the node, which is what the rebuild path relies on.

`done_event()` is the event that fires once, with the node, when the node's build has landed. The `WidgetReadyTrigger` is one per `build_root`, scheduled at the build height; when it runs it schedules every `WidgetReadyEvent` registered with it at the trigger's height, which is build height plus one. For the root that is occurrence zero, height 1, which is where `@started` fires and where a `dyn`'s first-build output fires. For a rebuild at height h the trigger is at h+1 and the done events at h+2.

**Cyclic bindings.** A widget block's recursive bindings are built with a placeholder that is closed once the group is built. The builder offers two, typed, and both are stored in the node:

```rust
pub fn add_cyclic_event<T: ValueType>(&mut self, name: String) -> (usize, Event<T>) {
    let forward = ForwardEvent::<T>::new();
    let ix = self.node.cyclic_events
        .add(name, Arc::new((forward.clone(), OnceLock::<Event<T>>::new())));
    (ix, Event(forward))
}
pub fn close_cyclic_event_by_index<T: ValueType>(&mut self, index: usize, event: Event<T>) {
    let (forward, target) = self.node.cyclic_events[index]
        .downcast_ref::<(Arc<ForwardEvent<T>>, OnceLock<Event<T>>)>()
        .expect("cyclic event closed with the wrong type");
    forward.close(&event);
    target.set(event).expect("cyclic event closed twice");
}
```

The slot in the node is typed by `T` at creation, so a close with the wrong `T` fails the downcast while the widget is building (`closing_a_cyclic_dynamic_with_the_wrong_type_fails_while_building`), not in the first occurrence. A cyclic dynamic is a `ForwardEvent` for the change event paired with a `OnceLock<Dynamic<T>>` for the value side, and a computed behavior that reaches the slot through a `Weak` and queries the closed dynamic's behavior:

```rust
struct Computation<T: ValueType> { slot: Weak<(OnceLock<Dynamic<T>>, Arc<ForwardEvent<T>>)> }
impl<T: ValueType> BehaviorComputation<T> for Computation<T> {
    fn compute(&self, dep: BehaviorDependencyTracker) -> Value<T> {
        let slot = self.slot.upgrade().expect("cyclic dynamic sampled after its widget was dropped");
        let d = slot.0.get().expect("cyclic dynamic never closed");
        d.behavior().query_for_computation(dep)
    }
}
```

Here is the ownership arrangement in full, because it does the collector's job. The node holds the strong ends: the slot `Arc` with the `ForwardEvent` and the closing event (or dynamic). The loop points back at the node's slot weakly: the forward holds a `Weak` to its target, the cyclic dynamic's behavior holds a `Weak` to the slot. Everything built from the placeholder (`increment.tag_map(.., accumulator)`, the hold over it, the event that closes the loop) is held strongly by whatever the widget returns or by the node, and strongly along the edges that go *up* to things built earlier. When the node and the widget's outputs are dropped, nothing outside holds the loop strongly and the whole of it is freed; `dropping_a_widget_frees_its_cyclic_loops` takes weak pointers into a loop, drops the node and the outputs, and checks that neither upgrades. A cyclic binding is always closed before `build` returns, which is before any occurrence that could fire through it, so the forward is connected from the first moment its closing event can fire: `cyclic_dynamic_event_fires_when_its_value_changes_in_the_build_occurrence` closes a cyclic dynamic with a hold of the done event and reads 7 from both halves after occurrence zero.

`fold_dyn` is the first client of all this and the model for a compiled hold with a cycle: a cyclic event for the "old value and update" pair, a `filter_map` applying the fold, a `Dynamic::hold` of that, a `tag_map` of the update over the hold producing the pair, and a close. Two tests drive it through a script of increments, one by hand and one through `fold_dyn`, and check both halves after every step.

`add_external_event(name)` makes an external event and registers its trigger in the node by name; `add_public_event` and `add_public_dynamic` register outputs by name. These three are the backend channel, and they go when the run call arrives.

**The node.**

```rust
pub struct WidgetNode {
    pub children: Mutex<Vec<Arc<WidgetNode>>>,
    pub public_dynamics: Slots<AnyDynamic>,
    pub public_events: Slots<AnyEvent>,
    pub _public_event_callbacks: OnceLock<Vec<Arc<DelegateCallback>>>,
    pub cyclic_events: Slots<Arc<dyn Any + Send + Sync>>,
    pub cyclic_dynamics: Slots<Arc<dyn Any + Send + Sync>>,
    pub triggers: Slots<AnyEventTrigger>,
    pub backend_data: JsonValue,
    pub delegate: OnceLock<Arc<dyn WidgetDelegate>>,
}
```

A node owns its children, its cyclic loops, and (today) its public outputs, triggers and delegate. `Slots<T>` is a `Vec<T>` with a `BTreeMap<String, usize>` of names, indexable by either, panicking on a duplicate name; its own comment says it is an `IndexMap`. `AnyEvent`, `AnyDynamic` and `AnyEventTrigger` are type-erased handles with a checked `get::<T>()` and, for the first two, a converted `Event<AnyValue>` or `Dynamic<AnyValue>` so a backend can subscribe without knowing `T`. `WidgetDelegate` is the trait a backend installs on a node to be told of new children, of destruction, and of public event firings; `set_delegate` subscribes a `DelegateCallback` to every public event and recurses into the children.

**`dyn`.**

```rust
impl<T: Widget + 'static + Send + Sync + Debug> Dynamic<Arc<T>> {
    pub fn dynamic_widget(&self) -> impl Widget<Output = Arc<Event<Arc<T::Output>>>>;
}
```

`dynamic_widget` on a dynamic of widgets is `dyn`: a widget whose node holds one child, the build of the dynamic's current widget, replaced whenever the dynamic changes, with an output event that fires in every build occurrence with the built widget's output. Its `build`:

```rust
fn build(&self, builder: &mut WidgetBuilder) -> Self::Output {
    let next_event = Arc::new_cyclic(|weak_self| DynamicWidgetEvent {
        widget: self.clone(), weak_self: weak_self.clone(),
        ready: builder.done_event.clone(),
        subscribers: SubscriptionManager::new(()),
    });
    let initial_widget = self.dynamic.event().0
        .subscribe(Arc::downgrade(&(next_event.clone() as Arc<dyn EventCallback<Arc<T>>>)))
        .unwrap_or_else(|| self.dynamic.behavior().0.query_for_tag());
    let output = Mutex::new(Some(builder.bind(&*initial_widget)));
    let first_time_event = builder.done_event().filter_map(move |_| {
        Some(PrismArc::new(output.lock().unwrap().take().unwrap()))
    });
    let next_event = Event(next_event);
    builder.add_public_event("__rebuilt__".to_string(), next_event.clone());
    Arc::new(Event::leftmost(vec![next_event, first_time_event]))
}
```

Four things happen here. The `DynamicWidgetEvent` subscribes to the dynamic's change event; if the dynamic already changed earlier in this occurrence, `subscribe` returns the new widget and the first build uses it, which is the "builds once with the new value" clause of the construction rule and the test `a_nested_dyn_built_in_the_occurrence_its_dynamic_changes_shows_the_new_value` (a page `dyn` with a title `dyn` inside it and one input changing both; the rebuilt page's title shows 7, not 0). Otherwise the first build samples the behavior. The initial widget is bound as child 0. The first build's output is delivered through a `filter_map` on the done event, so it fires at the done height of the build occurrence. And the output is `leftmost` of the rebuild event and the first-build event, with the rebuild listed first: in an occurrence where both fire (occurrence zero with a trigger that changes the dynamic), the rebuild's output wins, which is the output of the widget that is in the tree at commit.

The `add_public_event("__rebuilt__", ..)` line is a keep-alive. `DynamicWidgetEvent` is the subscriber to the dynamic's change; if nobody held the `dyn`'s output event, the only strong reference to it would be gone and the `dyn` would stop rebuilding. Registering it as a public event stores an `Arc` in the node. A private keep-alive list on the node is the right home, a known issue.

The rebuild is `DWAction`, scheduled at the `DynamicWidgetEvent`'s height, which is the dynamic's change height plus one:

```rust
impl<T: 'static + Widget + Debug> Action for DWAction<T> {
    fn act(self: Box<Self>, runtime: &Runtime) {
        let parent_node = self.event.ready.node.get().unwrap().clone();
        let (child_node, output) = WidgetBuilder::build_root(runtime, &*self.value, self.height + 1);
        if let Some(delegate) = parent_node.delegate.get() { /* tell it about child 0 */ }
        self.event.subscribers.notify(Arc::downgrade(&self.event), None, runtime,
                                      || Some(PrismArc::new(output)));
        let old_node = {
            let mut children = parent_node.children.lock().unwrap();
            std::mem::replace(&mut children[0], child_node)
        };
        old_node.prepare_destruction(runtime);
        runtime.schedule(usize::MAX, KeepUntilOccurrenceEnds(old_node));
    }
}
```

The parent node comes from the ready event's `OnceLock`, set when the `dyn`'s own build returned, so a rebuild in any occurrence, including the one the `dyn` was built in and including a nested `dyn` rebuilt in the same occurrence as its parent, finds its node (`dyn_rebuilds_when_its_input_arrives_in_occurrence_zero`, `nested_dyn_rebuilds_in_the_same_occurrence_as_its_parent`). The delegate is optional (`dyn_rebuilds_without_a_backend_delegate`). The new child is built with `build_root` at the action's height plus one, so its done events land above everything the action depends on. The output fires through `notify` at the action's height. The old node is swapped out at child index 0 and, rather than dropped, handed to a `KeepUntilOccurrenceEnds` action at `usize::MAX`. This is the clause that says a replaced node keeps propagating until the occurrence commits: a parent's `switch_hold` still follows the replaced child's output for the rest of the occurrence, and a tick that reaches the old child in the rebuild occurrence must still reach the old child's cyclic dynamic, whose slot the old node owns. `a_replaced_child_keeps_working_until_its_last_occurrence_ends` fires a tick and a rebuild together and checks that the old counter's output, under the non-prompt switch, delivered the tick.

`already_fired` on the `DynamicWidgetEvent`, the path taken when a `dyn` subscribes mid-occurrence to a change that has already happened and is not the initial build, defers its `DWAction` to `max(height, current + 1)`, so a rebuild is never scheduled at or below the running height.

The `DynamicWidgetEvent` implements `SubscriptionEvent` with `Inner = ()` and `unreachable!()` bodies, because it has no upstream in the manager's sense: its upstream is the dynamic's change event, which it subscribes to as a plain `EventCallback`.

### The test backend

```rust
pub struct TestBackend { node: Arc<WidgetNode>, runtime: Runtime, delegate: Arc<TestDelegate>, event_rx: mpsc::Receiver<EventRecord> }
impl TestBackend {
    pub fn build_tree<W: Widget>(widget: &W) -> (W::Output, TestBackend);
    pub fn main_loop(&mut self, script: TestScript) -> Result<(), ScriptError>;
}
pub fn script<const N: usize>(steps: [TestStep; N]) -> TestScript;
pub fn trigger<T: TestValue, const N: usize>(node_path: [usize; N], name: &str, v: T) -> TestStep;
pub fn delay_trigger<..>(..) -> TestStep;   // trigger without propagating yet
pub fn event<..>(node_path, name, v) -> TestStep;      // expect the next recorded firing
pub fn dynamic<..>(node_path, name, v) -> TestStep;    // read a public dynamic now
pub fn crickets() -> TestStep;                          // nothing recorded
impl TestScript {
    pub fn run<W: Widget>(self, widget: &W) -> Result<W::Output, ScriptError>;
    pub fn test<W: Widget>(self, widget: &W) -> W::Output;
}
```

`build_tree` makes a runtime, builds the root with `build_root` at height 0, installs a `TestDelegate` that clones the node's `public_dynamics` and `triggers` and records every public event firing onto a channel with its node path and name, recurses into children, and propagates once so occurrence zero lands. A script is a list of steps addressed by `[child indexes]` and a name: `trigger` schedules a named trigger and propagates; `delay_trigger` schedules without propagating, so several inputs land in one occurrence; `event` pops the next recorded firing and compares path and value; `dynamic` reads a public dynamic's behavior with `query_for_tag` and compares; `crickets` asserts the channel is empty. `TestValue` is blanket-implemented for any `ValueType + PartialEq` and does the typed trigger and comparison. Eight of the suite's tests are scripts; the rest drive the runtime directly with `Event::external`, `schedule_trigger`, `propagate` and `query_for_tag`.

What the backend does that the target forbids: it holds live runtime objects outside the runtime (the delegate's cloned `Slots`), it reads behaviors directly, and it addresses nodes by child index and name. All three are replaced by the run call, under which a Test World reads only the outcome.

### The 38 tests as the laws they pin

Each test's name is a sentence, and together they are the runtime half of the conformance suite. By law:

- *Leftmost merge and simultaneity*: `leftmost_test`, `combine_test`, `event_filter_test`.
- *The tag law*: `hold_map2_test` (update and query together: the query sees the old value).
- *Non-prompt switching*: `switch_hold_test2`, `switching_to_a_deeper_event_keeps_downstream_heights_valid`.
- *Holds and cycles*: `fold_dyn`, `fold_dyn_cyclic_dyn`, `cyclic_dynamic_event_fires_when_its_value_changes_in_the_build_occurrence`, `fold_dyn_counts_an_event_from_the_build_occurrence`, `closing_a_cyclic_dynamic_with_the_wrong_type_fails_while_building`, `dropping_a_widget_frees_its_cyclic_loops`.
- *Cycles are errors, not hangs*: `a_same_occurrence_cycle_through_a_cyclic_event_is_reported_instead_of_hanging`, `a_dynamic_defined_as_a_function_of_itself_is_reported_instead_of_hanging` (each on a thread with a five-second timeout).
- *`dyn`*: `dynamic_widget`, `dyn_rebuilds_without_a_backend_delegate`, `dyn_rebuilds_when_its_input_arrives_in_occurrence_zero`, `nested_dyn_rebuilds_in_the_same_occurrence_as_its_parent`, `a_replaced_child_keeps_working_until_its_last_occurrence_ends`.
- *Construction mid-occurrence*: `a_widget_built_mid_occurrence_sees_the_same_events_at_any_depth`, `a_nested_dyn_built_in_the_occurrence_its_dynamic_changes_shows_the_new_value`, `a_widget_built_mid_occurrence_gets_the_mapped_value_of_an_event_that_already_fired`, `a_closure_runs_once_per_occurrence_even_when_late_subscribers_read_it`.
- *Abort and recovery*: `runtime_keeps_working_after_a_closure_panics`, `a_trigger_dropped_by_a_panic_accepts_values_again`.
- *The entry points*: `scheduling_a_trigger_during_propagation_is_refused_without_a_panic`, `a_subscriber_rescheduling_its_own_trigger_does_not_hang`, `a_second_value_on_one_trigger_before_propagating_is_refused`, `runtime_can_move_to_another_thread`.
- *Behaviors*: `a_behavior_that_never_changes_does_not_collect_dependents`, the seven `joined_behavior_*` tests.
- *Values*: `round_trips`.

The gaps in this list are the gaps in the crate: no test of `Dynamic::join` because there is none, no routing test because there are no paths, and no law test that reads only what a World could read.

### Two occurrences traced through the code

The pieces above are easier to hold together as one trace than as a list, so here are two, with the heights written down. The first is the `fold_dyn` test's widget, an accumulator over an external `increment` event; the second is the mid-occurrence construction test, where catch-up does its work.

**An increment.** At build, `build_root(&runtime, &TestWidget, 0)` makes the node and a `WidgetReadyTrigger` at height 1, scheduled at height 0, and calls `build`. `add_external_event("increment")` makes the external event `E` (height 0) and stores its trigger. `fold_dyn` then makes, in order: a `ForwardEvent` `F` for the "old value and update" pair, stored in `cyclic_events`; `FM = F.filter_map(f)`; the hold `H = Dynamic::hold(0, FM)`, whose `Behavior::hold` subscribes to `FM`, which connects to `F` (which has no target yet, so the connection reaches nothing); `T = E.tag_map(pair, H)`; and `close_cyclic_event(F, T)`, which stores a `Weak` to `T` in `F` and, since `F` already has a subscriber, resubscribes it to `T`, which connects `T` to `E`. Heights come out as `E=0`, `T=1`, `F=2`, `FM=3`, and `H` commits at the top. Occurrence zero is `propagate`: height 0 runs the ready trigger, which schedules the node's done event at 1; height 1 fires it to nobody; the map is empty; clear-at-end has nothing to do.

The script's `trigger([], "increment", 3)` calls `schedule_trigger`, which checks the runtime is idle, sets the trigger's `scheduled` flag and schedules a `TriggerAction` at 0, then `propagate`. Height 0: the trigger action calls `E.fire(3)`; `record_once` stores `Some(3)` in `E`'s occurrence cache and registers its clear; `E`'s one live subscriber is `T`'s `MainSubscription`, whose `event_fired` finds `T.height() = 1 > 0` and schedules a `MainAction` for `T` at 1. Height 1: `T.handle_main_subscription` calls `notify`: there is a subscriber (`F`'s subscription), so it computes once, and the computation reads `H.query_for_tag()`, which is 0, the value before the occurrence, pairs it with 3, records `(0, 3)` and delivers it to `F`'s subscription, which schedules `F` at 2. Height 2: `F` forwards `(0, 3)` to `FM`'s subscription, scheduled at 3. Height 3: `FM` computes `f(0, 3) = 3`, records it, and delivers to two subscribers: `H`, whose `event_fired` schedules a `HoldAction` at `usize::MAX`, and the test backend's `DelegateCallback` on the public event `new_value`, which puts `3` on the channel. Height `usize::MAX`: the hold action writes 3 and invalidates its dependents, of which there are none. The map is empty: height becomes `None`, and the four clear closures empty the caches of `E`, `T`, `F` and `FM`. The script's `event([], "new_value", 3)` pops the channel and `dynamic([], "accumulator", 3)` reads `H.query_for_tag()`, which is now 3.

Every hold in a Prism program commits this way, and the whole of the tag law is the one line at height 1 that read `H` before height `usize::MAX` wrote it.

**A widget built in the middle.** The test builds an external `E`, a five-deep chain `D1..D5 = E.map(id)` five times (heights 1 to 5), a hold on `D5` so the chain is subscribed, and a `dyn` over `children = Dynamic::hold(new_child(), E.map(new_child))` whose block makes a child with two holds, one on `E` and one on `D5`. The `DynamicWidgetEvent` subscribes to `children`' change event `M = E.map(..)` (height 1), so its own height is 2. Occurrence zero builds the first child, whose holds subscribe to `E` and `D5` and get `None` back, since nothing has fired.

Occurrence one is `42` on `E`. Height 0: `E` fires; its subscribers are `M` (scheduled at 1), `D1` (at 1), and the first child's shallow hold (a hold action at the top). Height 1: `M` computes a new child widget and delivers it to the `DynamicWidgetEvent`, which schedules a `DWAction` at 2; `D1` fires and schedules `D2` at 2. Height 2: `D2` fires, scheduling `D3` at 3; the `DWAction` runs `build_root(child, 3)`. Inside that build, `Dynamic::hold(0, E)` subscribes to `E` and `subscribe` returns `Some(42)` from `E`'s cache, so `Behavior::hold` defers a `HoldAction` with 42 to the top: this hold has caught up. `Dynamic::hold(0, D5)` subscribes to `D5`, which is already connected and has no cached value yet, because height 5 has not run; `subscribe` returns `None`, and the hold waits like any subscriber. The action schedules the new node's ready trigger at 3, delivers the new child's output, swaps the node in at child 0, and hands the old node to a `KeepUntilOccurrenceEnds` at the top; then `propagate` drains the deferred list and schedules the caught-up hold action. Heights 3 and 4: `D3`, `D4`, and the done events. Height 5: `D5` fires 42 to three holds: the test's own, the old child's, and the new child's, each scheduling at the top. Height `usize::MAX`: five hold actions write 42, and the old node is dropped. Both of the new child's holds read 42.

There is a third branch the test does not take and the mechanism covers. Had nobody subscribed to `D5` before the new child did, `D5` would have had no connection and no cache; the child's `subscribe` would run `connect`, which resubscribes up the chain to `E`, finds `E`'s cached 42, and hands it to `D1.handle_already_fired`, which computes `id(42)` once through `record_once` and `deliver_already_fired`s it to `D2`, and so on to `D5`, which returns it to the new hold. The chain catches up on demand, each link computing once, and a later subscriber reads the caches. That is the whole of Spider's `subscribeAndRead`, and it is why a closure runs at most once per occurrence whoever asks and whenever.
## The target

The runtime is a library a World calls. Its public surface is the run call and the handful of plain-data types that cross it; the graph types are public for compiled widgets, which are the only other client, and for nothing else. Everything in this section is a consequence of the boundary (one synchronous call per occurrence, plain data both ways, nothing shared) applied to the core that exists.

### Single-threaded

Every lock in the current code is taken by the one thread that is propagating; the locks are uncontended by construction, and the only work they do beyond locking is the re-entry check that the height comparison already makes. So the pointer and cell types become `Rc`, `Weak`, `RefCell` and `Cell`. The `Send + Sync` bounds on `Widget`, `PolymorphicWidget`, `EventImpl`, `EventCallback`, `BehaviorImpl`, `HeapValue`, `Action` and every closure parameter go, which also lets a compiled closure capture an `Rc` without a wrapper. The occurrence state moves into one shared core that every node points at:

```rust
pub struct Runtime {
    core: Rc<Core>,
    root: Option<Rc<WidgetNode>>,          // the root InterceptWidget's node
    tags: Rc<TagTable>,
}

pub(crate) struct Core {
    actions: RefCell<BTreeMap<usize, Vec<Box<dyn Action>>>>,
    commits: RefCell<Vec<Box<dyn Commit>>>,
    deferred: RefCell<Vec<(usize, Box<dyn Action>)>>,
    height: Cell<Option<usize>>,
    occurrence: Cell<u64>,
    built: RefCell<Vec<Built>>,            // the build journal
    requests: RefCell<Vec<(Path, Value)>>, // the request collector
    errors: RefCell<Vec<RuntimeError>>,    // contained errors to report in the outcome
    pending_quarantine: RefCell<BTreeMap<Path, RuntimeError>>,   // owners the last error named
    depth: Cell<u32>,                      // the recursion depth guard
}

unsafe impl Send for Runtime {}            // sound: see below
```

The runtime is a parameter of every constructor that subscribes when it is made and of `subscribe` itself, so an occurrence's state never leaves the runtime. What a parameter cannot do is stop a caller from passing the wrong runtime: an event built against one runtime and subscribed in another is a bug nothing catches. The target closes that by giving every graph object an `Rc<Core>`, taken from the upstream it is built from: `filter_map` copies its source's, `hold` copies its event's, `combine` and `leftmost` check that their sources agree. The only constructors with no upstream are `never` and `constant`, and they schedule nothing, so they need none. The root of every chain is an event the builder made, and the builder has the core. A `subscribe` then asserts that the runtime it was handed is the one its source belongs to, and the mix is a panic at the call rather than a silent misdelivery. The cost is one pointer per node.

`Runtime` is `Send` and not `Sync`. It cannot be `Send` by derivation, since it holds `Rc`s, so the impl is written by hand, and it is sound because the runtime is the unique owner of every `Rc` in its graph: no handle to any of them crosses the boundary, by the rule that nothing in `Inputs` or `Outcome` is a runtime object, and compiled widgets live inside the graph they are built into. Moving all the clones of an `Rc` to one other thread together is the one case `Rc`'s non-`Send`-ness does not cover, and it is this case. A World may build the runtime on one thread, move it to another and drive it there; no two threads can touch it at once because `run` takes `&mut self` and nothing shares it. The `runtime_send` test becomes "build a graph on one thread, move the runtime, run an occurrence on the other."

The `try_lock` re-entry checks become a `Cell<bool>` per cached height and per behavior cache, set while computing and cleared after, with the same effect and a message that names the node: "event cycle: `name` depends on itself in one occurrence", where `name` is the source span the compiler attaches to every placeholder and slot. The eager detection of the error chapter, below, means this flag is reached only by a runtime bug once the walks at edge creation are in; it stays as the assertion it is.

Why not keep the locks and gain threads for free? Because they buy nothing. The parallelism that would use them is a different runtime variant with a different loop, described at the end of the chapter. Keeping `Mutex` as a down payment on that variant costs a lock operation on every subscriber list and every height read, and makes every closure `Send + Sync`, which pushes `Arc` into compiled code that would otherwise use `Rc`.

### The run call

A World could keep today's form: hold triggers for the slots it cares about, call `schedule_trigger` then `propagate`, read behaviors. That is a World holding runtime objects, and a World that holds a trigger for a slot that has since been torn down holds a pointer into a tree that has moved on; the rules about discarding late results become the World's to enforce against its own data structures. With paths as the only addresses, a late result is a path not in the opened list, and the runtime says so. So the boundary is one call, plain data in, plain data out:

```rust
pub struct Inputs<'a> {
    pub responses: Vec<(Path, Value)>,
    pub pushes: Vec<(ElementValue, Value)>,
    pub quarantine: Vec<Path>,
    pub oracle: &'a dyn Fn(&ElementValue) -> Value,
}

pub struct Outcome {
    pub patch: ListPatch,                  // the root list's change in this occurrence; empty if none
    pub requests: Vec<(Path, Vec<Value>)>,
    pub result: Value,
    pub errors: Vec<RuntimeError>,         // contained build failures, this occurrence
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind { Cycle, Overflow, UnknownPath, UnsuppliedState, DeferBackwards, Component, Build }

pub struct RuntimeError {
    pub kind: ErrorKind,
    pub message: String,
    pub occurrence: u64,
    pub spans: Vec<Span>,                  // the loop in order, or the one binding
    pub owner: Option<Path>,               // the lowest owning node, for quarantine
    pub path: Option<Path>,                // the path involved, for UnknownPath
    pub element: Option<ElementValue>,     // the element involved, for UnsuppliedState
}

impl Runtime {
    pub fn build(main: Rc<dyn PolymorphicWidget<Value>>, tags: Rc<TagTable>, oracle: &dyn Fn(&ElementValue) -> Value)
        -> Result<(Runtime, Outcome), RuntimeError>;
    pub fn run(&mut self, inputs: Inputs<'_>) -> Result<Outcome, RuntimeError>;
    pub fn list(&self) -> ElementList;     // the committed root list: an Rc clone, constant time
}
```

The outcome reports the list as a patch because a GUI changes a label and keeps a thousand rows, and a `Defer` occurrence often changes nothing at all; a full list per call would charge every turn the program's size. A World that wants the whole list, at startup or to rebuild its own tables, asks `list()`, which hands out the persistent map the runtime already holds.

`Value` at the boundary is the runtime's untyped value, today's `AnyValue`, restricted to plain data: no event, dynamic or widget inside, which the compiler guarantees for element fields, responses, pushes and results and which a debug build asserts at the slot. The typed wrapper that is `Value<T>` today is renamed `Typed<T>` so that the boundary can use the plain name; nothing else about it changes. `Value` holds `I64` and `U64` immediates, since `Int` is 64-bit signed and `Nat` 64-bit unsigned; the 32-bit variants go in step 1. `Path`, `ElementValue` and `TagTable` are below. `build` is occurrence zero: it constructs `@main` under a root node and returns the runtime with the initial list as a patch of insertions; a cycle found there, or a state tag the oracle cannot answer, is an `Err` and no runtime. `run` is every occurrence after that. Nothing else a World needs is public; `Event::external`, `schedule_trigger`, `build_root` and `propagate` become `pub(crate)`.

`RuntimeError` is one record with a kind rather than one variant per kind because the World core prints every error the same way and the `Error` element a program can match on has a closed `kind` field; a Rust enum per kind would be unpacked into this record at the boundary anyway. Of the seven kinds, the runtime raises five: `Cycle`, `Overflow`, `UnknownPath`, `UnsuppliedState` and `Build`. `DeferBackwards` is raised by the World core, which is the only thing that knows what a turn is. `Component` is raised by the World for its own components' failures, and by the runtime for one thing: a panic inside propagation whose payload is not the runtime's own, which can only come from native code a World or a library put inside a closure. Two further protocol failures, a response of a type the slot's tag does not declare and two responses to one path in one occurrence, are the World's bugs rather than the program's; `run` refuses them before scheduling anything, as `Component` errors naming the path, and the core treats them as its own fault.

`quarantine` is the World's instruction, taken from an error the runtime reported: the `owner` path of a `Cycle` found at a switch adoption, which is the one error where the occurrence is lost and the next one would be lost the same way. The occurrence section says what the runtime does with it. A path in `quarantine` that no error named is a `Component` error, since the World is allowed to quarantine only what it was told to.

### The occurrence

Two things go wrong in a run call that has no internal order. An input delivered against a list that changed under it reaches a slot that did not exist when the World chose the path, or misses one that did; and a hold written as soon as its event fires is read, by a tag later in the same occurrence, at its new value. The second needs every write held until every read is done, and holding the list's own writes to commit answers the first too: every routing table is an element list's value side, which does not move until commit, so a lookup at any height reads the list the World or the interceptor last received. `run` therefore has five phases, each short.

*Open.* Increment the occurrence counter, which clears every per-occurrence cache at once (the caches are stamped, below). Perform the quarantine, if any. Nothing else: there is no table to copy, because each list's committed value is already the table.

*Fire.* For each response `(path, value)`: look the path up in the root list's committed value, the list the previous call returned; if absent, return `Err` with kind `UnknownPath` before anything has been scheduled; if the slot's tag does not respond, or the value's type is not the tag's response type, or the path was already given a response in this call, return the protocol error; otherwise schedule the root fan at height 0 with the entry. For each push `(element, value)`: find the root state cell for that element value (a cell exists for every distinct element value some live state slot has asked for) and schedule its trigger at height 0. A push for a value no slot holds is ignored; it is stale bookkeeping on the World's side and harms nothing. All inputs are scheduled before any runs, so they are simultaneous.

*Propagate.* The loop of today's `propagate`, with the error path changed and with the journal: every node built inside the loop, by a `dyn`, a list widget or a supplier, is pushed onto `built` with its parent and its position, so that rollback can find it.

*Commit.* The commit list runs: hold writes, switch retargets (each preceded by the eager cycle walk, below), drops of replaced nodes, list-widget removals, supplied-widget removals. In the target the commit list is its own `Vec<Box<dyn Commit>>` on the core rather than the `usize::MAX` entry of the action map. Two reasons. The current encoding makes "after everything" and "the greatest height" the same number, so an action in the commit batch that needs to defer something to commit has nowhere to put it (scheduling at `usize::MAX` while at `usize::MAX` fails the cycle check, and `current + 1` overflows there). And rollback is "drop the commit list", which is one line when the list is its own field.

*Close.* Collect the outcome: the root list's patch for this occurrence, the request firings grouped per slot, the result, and the contained errors. The grouping order is the order of the *opened* list, with slots created in this occurrence after it in creation order, whether or not they survived to commit; a slot torn down in this occurrence that was in the opened list keeps its opened position. This is the order the conformance chapter's transcript prints `requested` lines in, and `a_request_fired_by_a_slot_torn_down_in_the_same_occurrence_is_reported_with_its_path` pins the torn-down case. Within one slot, firings are in firing order, which for one event is at most one per occurrence. Return `Ok`.

**Rollback.** If any action panics, or any check returns an error from inside propagation, `run` does, in order: drop the action map and the deferred list; drop the commit list, which is every hold write, switch retarget and node drop that was going to happen; unwind the journal, which detaches every node built in this occurrence from its parent (restoring the previous child at that position, which is still alive because replaced nodes are dropped only at commit) and drops it, including supplied widgets, since those are journaled under the interceptor's node like any other; reset the height and the depth counter; clear the request collector and the contained-error list; and return `Err`. The caches need nothing: they are stamped with the occurrence number and the next `run` increments it. Nothing resumes the panic across the boundary; a panic is caught at the action and becomes a `RuntimeError`: kind `Overflow` when the payload is the runtime's own `PrismPanic` (which checked arithmetic and the depth guard raise, with the span), kind `Component` for any other payload. After `Err`, the graph is as it was after the previous `Ok`, and the next `run` works. The invariant checker, below, is what tests this: after a failed occurrence, every hold at its previous value, every switch on its previous inner, every node that existed before still existing, every node built during the failure gone, no cache stamped with the failed occurrence's number.

Why rollback is cheap rather than merely possible: every write a program can observe is already buffered to commit time, for the tag law's sake. The only writes during propagation are to per-occurrence caches, which the stamp handles, and to the tree, which is append-only during an occurrence: new nodes are built and journaled, old nodes are replaced only at commit, and until then stay in the tree as the payload of a commit entry. So the undo log is the commit list plus the journal, and both are kept for other reasons. Rollback cannot undo a closure's side effect outside the graph, and compiled code has none, because it is pure by construction.

**Contained failures: the error node.** A `dyn` build, a `for` key's build and a supplier's build each have a boundary of their own: a closure the runtime calls, which creates a node, builds into it, closes its placeholders, creates its slots, and returns. Until it returns, nothing outside the node refers to anything inside it, so a build that fails can be abandoned by dropping the half-built node, and the occurrence continues. The builder implements this as a nested transaction. On entry it records the lengths of the commit list, the deferred list, the journal and the request collector; a failure (a cycle found when a placeholder is closed, an unsupplied state tag, a panic in a build-time closure) truncates all four to the recorded lengths, drops the node, and builds the *error node* in its place: a node with one slot whose element is `Error(error=the report)` and no result. The `dyn`'s event does not fire in that occurrence, because there is no result of the block's type to fire with; a `!Switch` over it keeps the old generation's event, which never fires again once the old generation is dropped at commit, and a hold of dynamics under `$Join` keeps the old generation's dynamic at its last value. The old generation is still torn down at commit, because the dynamic it was built for has changed. The report goes on `errors` in the outcome with kind `Build` wrapping the cause's kind in its message and spans. The root build has no outside to continue into, so a failure there is the `Err` of `build`.

The truncation is why the four lists are plain vectors with push at the end: a build is synchronous and nothing else appends while it runs, so "everything after the recorded length belongs to this build" is exact. A hold built inside the failed build may have deferred its catch-up write onto the commit list; the truncation removes it, and with it the only strong reference to the half-built hold.

**Eager cycle detection.** A prompt cycle has one property: no consistent assignment of heights. The runtime finds one by the height walk, and the choice that decides which unit of failure a cycle lands in is *when* the walk runs. The target runs it at the moment an edge is created, in three places: when a placeholder is closed (`ForwardEvent::close` computes the target's height through `height_checked`, which walks upstream with a per-node in-progress flag and returns the loop's spans when it meets a node already in progress); when a switch adopts a new inner at commit (the retarget computes the new inner's height the same way, and the walk reaches the switch itself exactly when the inner depends on it, because every cached height below the switch was invalidated by the retarget and the walk stops only at cached heights, which are the nodes not downstream of it); and when `Dynamic::join` or the list widget retargets, which are the same operation. A cycle found at a close during a build is a build failure, contained as above. A cycle found at a switch adoption is found in the commit batch, after propagation and before any hold write, and aborts the occurrence with the loop's spans and the owner's path. The owner is the lowest common owning node of the nodes on the loop, found by walking each node's owner chain, which every graph object has as a `Weak<WidgetNode>` set by the builder. The cost is a walk upstream per edge creation, bounded by the cache: the walk stops at any node whose height is cached, and a cached height is valid exactly as long as nothing below it has been invalidated, which is the rule every event type keeps. The runtime check that a node never schedules at or below the current height stays as an assertion against the runtime's own bugs; with eager detection it should never fire for a program's cycle.

**Quarantine.** At open, for each path in `quarantine`: look it up among the owners the runtime named in its last error (it keeps them in `pending_quarantine: BTreeMap<Path, RuntimeError>` until the next successful `run`); find the node at that path; replace it in its parent with an error node holding `Error(error=that report)` at the same position; drop the subtree, which drops its slots, its supplied widgets' interest and its loops. The replacement is in the parent's children map under the same segment, so the parent's list changes in this occurrence like any other child replacement and the World sees the `Error` element in the outcome. Responses in this same call addressed to paths under the quarantined node were legal addresses, since those paths were in the list the World last received; they are delivered to nothing and reported with their paths in `requests` as firings-to-nothing are, so the World can discard them. What the parent held of the quarantined node's results stays as handles to a dropped subtree: an event that never fires again, a dynamic at its last value.

### The graph API

The compiler emits one node per lift. `$a + $b + $c` is one derived dynamic, not a tree of `map2`s allocating pairs, and `$a + $b + !e` is one tag reading two behaviors, because the typed-AST text form prints one node per lift and the graph should not be a lie about the text form. So the target's combinators are n-ary where the current ones are binary, and three constructors exist for the compiler's sake: `hold_first`, for a dynamic-of-widgets bind whose initial value is the first build's result; `map_n`, a derived dynamic over any number of dynamics; and `tag_map_n`, a tag reading any number of behaviors. The whole of the target's graph surface, with the fate of today's public entry points:

```rust
impl<T: ValueType> Event<T> {
    pub(crate) fn external(core: &Rc<Core>) -> (Event<T>, EventTrigger<T>);   // the run call's inputs only
    pub fn never() -> Self;
    pub fn filter_map<O: ValueType>(&self, f: impl Fn(Typed<T>) -> Option<Typed<O>> + 'static) -> Event<O>;
    pub fn map<O: ValueType>(&self, f: impl Fn(Typed<T>) -> Typed<O> + 'static) -> Event<O>;
    pub fn tag_map_n<O: ValueType>(&self, behaviors: Vec<Behavior<Value>>,
                                   f: impl Fn(Typed<T>, &[Value]) -> Option<Typed<O>> + 'static) -> Event<O>;
    pub fn tag_map<B: ValueType, O: ValueType>(&self, behavior: Behavior<B>,
                                   f: impl Fn(Typed<T>, Typed<B>) -> Option<Typed<O>> + 'static) -> Event<O>;   // tag_map_n over one
    pub fn combine<A: ValueType, B: ValueType>(f: impl Fn(OneOrBoth<A, B>) -> Option<Typed<T>> + 'static,
                                               a: Event<A>, b: Event<B>) -> Event<T>;
    pub fn leftmost(events: Vec<Event<T>>) -> Event<T>;
}
impl<T: ValueType> Event<Event<T>> {
    pub fn switch_hold(&self) -> Event<T>;                       // no inner until the first outer firing commits
    pub fn switch_hold_seeded(&self, initial: Event<T>) -> Event<T>;   // follows `initial` until then
}
impl<T: ValueType> Dynamic<T> {
    pub(crate) fn new(behavior: Behavior<T>, event: Event<T>) -> Dynamic<T>;   // checked by the invariant checker
    pub fn constant(value: Typed<T>) -> Dynamic<T>;
    pub fn hold(initial: Typed<T>, event: Event<T>) -> Dynamic<T>;
    pub fn hold_first(event: Event<T>) -> Dynamic<T>;            // initial value: the event's catch-up value in the build occurrence
    pub fn map<O: ValueType>(&self, f: impl Fn(Typed<T>) -> Typed<O> + 'static) -> Dynamic<O>;
    pub fn map_n<O: ValueType>(dynamics: Vec<Dynamic<Value>>, f: impl Fn(&[Value]) -> Typed<O> + 'static) -> Dynamic<O>;
    pub fn map2<A: ValueType, B: ValueType>(a: Dynamic<A>, b: Dynamic<B>,
                                            f: impl Fn(Typed<A>, Typed<B>) -> Typed<T> + 'static) -> Self;   // map_n over two
    pub fn distinct(&self, eq: impl Fn(&Value, &Value) -> bool + 'static) -> Dynamic<T>;   // $Distinct
    pub fn event(&self) -> Event<T>;                             // !Updated
    pub fn behavior(&self) -> Behavior<T>;
}
impl<A: ValueType> Dynamic<Dynamic<A>> {
    pub fn join(&self) -> Dynamic<A>;                            // $Join, prompt on both
}
impl<W: PolymorphicWidget<O>, O: ValueType> Dynamic<Rc<W>> {
    pub fn dynamic_widget(&self) -> DynamicWidget<W, O>;         // dyn: Output = (Event<O>, O), the event and the first build's result
}
impl<K: ValueType + Ord, V: ValueType> Dynamic<PMap<K, V>> {
    pub fn list_widget<W: Widget>(&self, kind: Consolidation, block: impl Fn(&K, Dynamic<V>) -> W + 'static) -> ListWidget<K, V, W>;
}
impl WidgetBuilder<'_> {
    pub fn bind<W: Widget>(&mut self, widget: &W) -> W::Output;
    pub fn started(&mut self) -> Event<()>;                      // @started: a dyn over a constant, takes a position
    pub fn element(&mut self, element: ElementValue, request: Option<Event<Value>>) -> Result<ElementResult, RuntimeError>;
    pub fn add_cyclic_event<T: ValueType>(&mut self, span: Span) -> (CyclicEvent<T>, Event<T>);
    pub fn add_cyclic_dynamic<T: ValueType>(&mut self, span: Span) -> (CyclicDynamic<T>, Dynamic<T>);
    pub(crate) fn build_root<W: Widget>(..) -> ..;               // called by dyn, for and the supplier only
}
impl<T: ValueType> CyclicEvent<T>   { pub fn close(self, event: Event<T>) -> Result<(), RuntimeError>; }
impl<T: ValueType> CyclicDynamic<T> { pub fn close(self, dynamic: Dynamic<T>) -> Result<(), RuntimeError>; }
impl Runtime {
    pub(crate) fn schedule_trigger<T: ValueType>(&self, trigger: &EventTrigger<T>, value: Typed<T>);   // from run only
    pub(crate) fn run_occurrence(&mut self) -> Result<(), RuntimeError>;                              // today's propagate
}
```

`hold_first` is a hold with no initial argument: it subscribes to its event in the build occurrence, takes the catch-up value `subscribe` returns as the initial value, and from then on is an ordinary hold. The compiler emits it for one thing only, the plain-result lift under the canonical node structure, below, where the event is a `dyn`'s output and has always fired by the time the hold is built (the first build is inside the `dyn`'s own build, and its output is the `dyn`'s second result, so the catch-up value exists by construction); a `hold_first` over an event that has not fired is a build error, since there is no value to hold, and the compiler never emits one. `map_n` is the change law for any arity: its behavior is a computation over every operand's behavior, and its event is a single merge node that fires at n with every operand's value after n if any operand changed at n, once however many did, which is glitch freedom with no pair allocation in the middle. `tag_map_n` reads every behavior with `query_for_tag` at the event's firing, which is the tag law; the behaviors are untyped at the signature because the compiler has already checked them and the closure reads them by position. `map2` and `tag_map` stay as the two-operand cases that hand-written widgets and the join use. `distinct` is `$Distinct`: a derived dynamic whose event is the source's filtered by `eq old, new` against the value before the occurrence, one node, which is why dedupe is a prelude function and not a runtime policy.

The one thing the signatures hide is `Rc<Core>`. Every constructor takes it from its first operand, `never` and `constant` take none because they schedule nothing, and the builder hands it to `external`, `started` and the placeholders, which are the roots of every chain.

### `Dynamic::join`

The outer switches to `d2` in the same occurrence that `d2` changes. Whether `d2`'s change sits above or below the switch in height order depends on where the two were built, and both orders are common: a `dyn` over a block that returns a dynamic produces an event of dynamics whose inner change events sit above the `dyn`'s rebuild height, while a hold of two pre-existing dynamics switched by a click has the inner below. A `switch_hold` over the inner events plus a `tag` of the new inner's behavior at the swap gives the *old* value of `d2` when `d2` changes later in the occurrence, and the joined behavior ends at `d2`'s new value, so the two halves disagree.

For `outer: Dynamic<Dynamic<A>>`, `join` is `$Join`: at occurrence n its value before n is the value before n of the inner current before n; its change event fires at n, with the value after n, if the outer changes at n (to a new inner, whose value after n is taken) or if the current inner changes at n. It is prompt on both. The behavior half exists and passes its seven tests. The event half is new, and the correct event is the leftmost of two:

```rust
impl<A: ValueType> Dynamic<Dynamic<A>> {
    pub fn join(&self) -> Dynamic<A> {
        let behavior = self.behavior().map(|d| d.behavior()).join();   // exists today
        let on_outer = JoinOuterEvent::new(self.clone());               // outer change -> new inner's value after n
        let on_inner = self.event().map(|d| d.event()).switch_hold_seeded(self.behavior().query_for_tag().event());
        let event = Event::leftmost(vec![on_outer, on_inner]);
        Dynamic::new(behavior, event)                                   // pub(crate), checked
    }
}
```

`on_inner` is a `switch_hold` over the inners' change events, seeded with the initial inner's: it follows the inner that was current before n and moves to the new one at commit. `on_outer` is the new piece:

```rust
struct JoinOuterEvent<A: ValueType> {
    outer: Dynamic<Dynamic<A>>,
    current: RefCell<Event<A>>,          // the inner's change event as of the last commit
    pending: RefCell<Option<Pending<A>>>, // set when the outer fires in this occurrence
    subscribers: SubscriptionManager<A, Self>,
    height: Cell<Option<usize>>, ...
}
struct Pending<A: ValueType> { inner: Dynamic<A>, heard: Cell<Option<Value<A>>>, sub: Rc<dyn EventCallback<A>> }
```

When the outer fires at n with inner `d`, the joined value after n is `d`'s change value at n if `d` changes at n, else `d`'s current value. Whether `d` changes at n is not known at the outer's height; it is known once `d`'s change height has passed. So `JoinOuterEvent` does not fire at the outer's height plus one. On the outer's firing it subscribes a fresh callback to `d.event()` with catch-up: if `subscribe` returns a value, `d` already changed below and `heard` is set; otherwise `d` will push to the callback if it changes above. It then invalidates its own height and its subscribers' and schedules its firing at its new height, `max(outer.height(), d.event().height()) + 1`, where it reads `heard` if set, else `d.behavior().query_for_tag()`. If `d`'s height is at or below the current height, `d` has either fired (and was caught up) or will not fire this occurrence, and the firing can be scheduled at the current height plus one. At commit, `current` becomes `d`'s event and `pending` is cleared. The height is dynamic, like a switch's, and is recomputed through the same invalidation walk.

With `leftmost` putting `on_outer` first, an occurrence where the outer changes and the *old* inner also changes gives the new inner's value, which is right: the old inner is no longer current after n. An occurrence where only the current inner changes fires `on_inner` with its value. An occurrence where the outer changes and the new inner changes fires only `on_outer`, since `on_inner` is still on the old inner. Both halves agree at every commit, which the invariant checker asserts for every dynamic.

**The swap-and-change case, traced in the "above" order.** Take an external `E` at height 0, `d2 = Dynamic::hold(0, E.map(id).map(id).map(id))` so that `d2`'s change event `C2` is at height 3, `outer = Dynamic::hold(d1, E.map(|_| d2))` whose change event `O` is at height 1, and `j = outer.join()`. Before the occurrence `j` is on `d1`, at value 5, and `JoinOuterEvent` has `current = d1.event()`. The World fires `E = 9`.

```
height 0   E fires 9.  Subscribers: O's map (schedule at 1), d2's first map (schedule at 1).
height 1   O fires d2.  JoinOuterEvent: pending = { inner: d2, heard: None }; subscribe to C2 with
           catch-up -> None (height 3 has not run); invalidate own height; new height is
           max(1, 3) + 1 = 4; schedule firing at 4.
           d2's first map fires 9 to the second (schedule at 2).
height 2   second map fires 9 to the third (schedule at 3).
height 3   C2 fires 9.  Subscribers: d2's hold (commit entry: write 9), the pending callback
           (heard = Some(9)).  on_inner's switch_hold is still on d1.event(), which is quiet.
height 4   JoinOuterEvent fires: heard is Some(9), so the value is 9.  leftmost picks on_outer; j's
           event fires 9.  A map over j fires at 5 and reads 9: the join is prompt.
commit     d2's hold writes 9; outer's hold writes d2; the joined behavior now reads d2 = 9;
           JoinOuterEvent: current = d2.event(), pending = None; on_inner's switch adopts d2.event().
```

The "below" order is the same trace with `C2` at height 1 and `O` at height 3: at height 3 the fresh subscription to `C2` returns `Some(9)` by catch-up, `heard` is set at once, `d2`'s height is at or below the current height, and the firing is scheduled at the current height plus one, 4, where it reads `heard`. The split that an implementer has to get right is in those two lines: below the outer's height, catch-up fills `heard`; above it, the push fills `heard`; and the firing is scheduled above whichever height is greater, never at the outer's height plus one.

**The three law tests.** Each is a statement of the join law with a widget around it, and each has the behavior-only twin among the seven that exist.

- *The outer switches to an inner that does not change.* `join_follows_the_outer_switch_and_reads_the_new_inner`: `d1` at 0, `d2` at 3, the outer switches to `d2` in occurrence 1 and nothing else fires. The joined event fires in occurrence 1 with 3, once, and does not fire in occurrence 2. This is the test that catches a join built from `switch_hold` alone, which waits for `d2` to change before saying anything.
- *The outer switches and the new inner changes in the same occurrence, in both height orders.* `join_follows_the_new_inner_when_both_change_in_one_occurrence`: once with `d2`'s change below the outer's (both external, `d2` listed first in the input) and once with `d2`'s change above it (`d2` derived from the outer's source through three maps). In both, the joined event fires once with `d2`'s new value, and the behavior agrees after commit. This is the case that forces `on_outer` to wait for the inner's height.
- *The old inner is ignored after the switch, and in the switch occurrence.* `join_ignores_the_old_inner_after_a_switch`: the outer switches to `d2` and `d1` changes in the same occurrence; the joined event fires with `d2`'s current value and not `d1`'s new one; in the next occurrence `d1` changes again and the joined event is quiet.

A fourth, `join_is_prompt`, is the change law applied to the join: a dynamic derived from the join by `map` changes in the same occurrence as the join and reads the joined value.

This is the only join, and it is enough for everything the specification needs: `dyn` over a block that returns a dynamic (a hold of the `dyn`'s event initialised with the first build's result, then joined), the canonical cache idiom, and the dynamic consolidation of `for`, which is the same mechanism over a map of inners rather than one.

### `!Updated`

`!Updated: fn $a -> !a` is the prelude's name for a dynamic's change event, and it compiles to `Dynamic::event()`, which is a clone of a handle. It fires with the new value in every occurrence the dynamic changes and never at build: a hold's event is its input event, which did not fire because the hold was built; a derived dynamic's event is a `filter_map` or `combine` over its operands' events, which did not fire because the operand was read; a constant's event is `never`. Nothing is constructed and no widget is involved, which is why it is a plain function and not a widget. A program that wants a firing at build binds `@started`, which is a `dyn` over a constant and takes a bind position, and merges it in.

The one thing worth saying about `!Updated` in the runtime is what it does not do: it does not dedupe. A hold set to the value it already holds changes, its event fires, and everything downstream recomputes; `$Distinct` is a `filter_map` that drops a firing equal to the last one it let through, with equality from the trait table, and a hold of the filtered event. Deduping in the runtime would need equality on every type, which `Value` does not have, and would make `!Updated` on a hold differ from the hold's input event, which the hold law says they are one and the same.
### The keyed list widget

A todo list with a half-typed edit in row three must keep that edit when a row is inserted above it. That is the whole requirement on `for`: a key that stays is never torn down, whatever happens to its neighbours, and a key that appears is built in the occurrence it appears and is part of the result in that same occurrence, birth firings included. `for $m:` is a widget over a dynamic map:

```rust
impl<K: ValueType + Ord, V: ValueType> Dynamic<PMap<K, V>> {
    pub fn list_widget<W: Widget>(&self, kind: Consolidation,
                                  block: impl Fn(&K, Dynamic<V>) -> W + 'static) -> ListWidget<K, V, W>;
}
pub enum Consolidation { Plain, Event, Dynamic }   // the compiler states the result kind
```

Its node holds one child per key, in key order, and the child for key `k` is built from `block(k, per_key)`, where `per_key: Dynamic<V>` is projected from the map: value `m[k]`, with a change event that fires when the map changes and `m[k]` differs from before. A change to an entry's value therefore updates `$_` inside the block and does not rebuild it. (Whether `m[k]` differs is decided by the diff below; an entry whose subtree is shared is unchanged by identity, and an entry the diff reaches is compared with `V`'s equality from the trait table, or counted as changed for a `V` without equality, which is still correct.)

`PMap<K, V>` is the runtime's map: a persistent B-tree whose nodes are `Rc`s, with structural sharing between versions, and it is the representation of every Prism map value, not only the ones `for` reads. Two things force it. A map in a hold changes by `Insert $todos, k, v` and the like, and on an `Rc<BTreeMap>` that is a copy of the whole map whenever anyone else holds the old one, which the hold's own value side always does until commit; on a persistent B-tree it is O(log n) new nodes and the rest shared. And the list widget has to know which keys appeared, which went and which changed, and on two unrelated maps that is a walk over both, O(n) per change however small the change. Between two versions of one persistent map, the walk descends both trees together and skips every pair of subtrees that are the same `Rc`, so the diff costs O(changed · log n). The list widget runs that diff once per change of the map and drives the per-key projections from it, so a key the change did not touch costs nothing, not even a comparison.

That is as far as a value-only map goes. Derived functions over maps (`Filter`, `MapValues`, `Values`) still compute their result from the whole input on each change, O(n) per change, and their result shares nothing with the previous result, so the diff downstream of them is O(n) too. The full answer is a dynamic whose change event fires with a patch rather than a value, with the value side maintained by applying the patch: Reflex's `Incremental` (`holdIncremental`, `updatedIncremental`, `mergeIncremental` in the Spider sources) is that idea, and the element list, below, is the runtime's one instance of it. Making `Filter` and `MapValues` patch-to-patch functions, so that a `for` over a filtered map pays for what changed, is the next step when a benchmark asks for it; nothing a program can observe changes, because the value side is the same map.

On the map's change event at n, at the change's height h: keys that appear are built in n, each as a journaled build at h+1, under the construction rule, so a new block sees all of n (catch-up does this; the list widget does nothing special); keys that disappear are scheduled for teardown at commit, like a replaced `dyn` child, so an event that fired inside a disappearing block earlier in n is still delivered; keys that stay are untouched. The node's children map is keyed by `Segment::Key(k)`, so a block's slots keep their paths across any change that keeps the key.

The result is consolidated by the kind the compiler states, and all three consolidations are prompt: the consolidated value in n reflects the keys after n.

- `Plain`: a `Dynamic<PMap<K, O>>` whose value is the map of block results and whose change event fires at n, with the new map, when the key set changes at n. The new map is computed at h+2, above the new blocks' builds.
- `Dynamic`: each block returns `Dynamic<O>`; the result is `Dynamic<PMap<K, O>>`, a join over a map. Its behavior queries every current inner; its change event fires at n when the key set changes at n or any current inner changes at n, with the whole map after n. The implementation is `JoinOuterEvent` with a map of inners in place of one: on a key-set change it subscribes to the new inners with catch-up and schedules its firing above all of their change heights; on an inner's change it fires with the previous map updated at that key, one persistent insert. Prompt on both, which is what makes `for` over a list of counters a dynamic of their counts with no lag.
- `Event`: each block returns `Event<O>`; the result is `Event<PMap<K, O>>`, firing at n with exactly the entries whose events fired at n. This is a merge over a dynamic set of members, and the membership is prompt: a key that appears at n is a member at n, and anything its block fires in its creation occurrence is in the consolidated event at n.

The event consolidation is where the static cycle check and the runtime's height check have to agree, and they do. The merge's height is one more than the greatest among its current members, invalidated and recomputed when the membership changes, exactly as a switch's is. The only thing a block built at n can fire at n is pure code: `@started`, an inner `dyn`'s event, a hold's change from one of those. A block that fires nothing at birth, which is every block whose result is a response from a slot it created (the slot was created at n and cannot receive anything at n), adds a member whose event will not fire this occurrence, and the merge's recomputed height is irrelevant to n. A block that does fire at birth adds a member that fires at a height above h; if the merge had not yet fired at n, its recomputed height is above that member's and it fires once with every entry, old and new; if the merge had already fired at n, because its old height was below h, the newborn's firing asks it to fire twice in one occurrence, which the manager's `record_once` refuses, and the eager walk at the membership change reports it as a cycle: the merge's new height is at or below the current height. That is the runtime's reading of the rule that `@ Delete on !started` as a row result is an error while the ordinary todo list is legal: in the todo list, `!deletes` feeds `$todos` through a hold, `$todos`' change rebuilds the `for`, and the new rows fire nothing at birth, so no edge runs from `$todos` to `!deletes` and the heights are consistent; with `!started` in the row, there is such an edge, and the merge would have to sit above the hold's change, which sits above the merge.

This is also what makes the interceptor's loop legal. An interceptor writes `!down <- for $slots: ... @ resp` and feeds `!down` back into `intercept`. The fan delivers `!down`'s entries to the child's slots at `!down`'s height plus one; the child reacts, perhaps rebuilding a `dyn`; the child's list changes; `$slots` changes, at some height above the fan; the `for` builds new blocks for the new paths, each of which creates an outer slot with `@Element` and returns that slot's response event, a fan output whose height is one above the *outer* responses event and has nothing to do with `$slots`. The new member cannot fire at n, the merge's height does not depend on `$slots`' height, and there is no cycle. The `for` over `$slots` is a membership edge, not a data edge, and a membership edge becomes a data edge only when a newborn speaks.

Arrays under `for` are the same widget keyed by index, with the specification's warning attached: an insertion at the front changes every later key, which rebuilds every later block and resets its state.

### Elements, slots and paths

An `ElementValue` is plain data, a tag and its field values, so that two slots with equal elements are equal for state identity and so that a World can key a map by it. Elements are ordered by tag name, then by plain fields in declaration order, which is the language's ordering on an extensible variant. A path is the address a World sends to, and the counter's Increment button is `0/#0/0` under every implementation: the `dyn` at position 0, its generation 0, the slot at position 0 inside it.

Both are keys in every hot map of the runtime (the memo, the element list, the fan's outputs, the World's tables), and both are compared far more often than they are built. Compared structurally, an element with a large field (a `Gui.Scene` whose draw list is the whole window, an `Fs.Contents` keyed by a long path) costs the field's size per comparison, and a `BTreeMap` lookup makes log n of those; a path costs its depth. So both are interned: built once through a weak table on the core, with the hash computed at construction and cached, so that two equal values are one allocation. Equality is then a pointer comparison. Ordering compares pointers first and answers `Equal` at once on a match, and otherwise compares structurally, which for two different paths stops at the first differing segment.

```rust
#[derive(Clone)]
pub struct ElementValue(Rc<ElementData>);      // interned: equal values share one Rc
pub struct ElementData { pub tag: TagId, pub fields: Box<[Value]>, hash: u64 }   // fields in declaration order
// PartialEq: Rc::ptr_eq. Ord: ptr_eq first, then tag order, then fields. Hash: the cached hash.

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TagId(u32);                          // assigned in tag-name order, so TagId order is name order

pub struct TagInfo {
    pub name: &'static str,                // "Ui.Button"
    pub fields: &'static [&'static str],
    pub requests: Option<TypeId>,
    pub responds: Option<TypeId>,
    pub state: Option<TypeId>,
}
pub struct TagTable { infos: Vec<TagInfo>, by_name: BTreeMap<&'static str, TagId> }

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Segment { Bind(u32), Gen(u64), Key(Value), Supplied(u32) }

#[derive(Clone)]
pub struct Path(Rc<PathNode>);                  // interned, like ElementValue
pub struct PathNode { parent: Option<Path>, last: Segment, depth: u32, hash: u64 }
```

Equality on `ElementValue` excludes the request event because the request event is not a field of the value: it is held by the slot, below. `TagTable` is the compiler's table, handed to the runtime at `build`; it numbers tags in name order, so the derived order on `TagId` is the order by tag name and an element comparison never looks the names up; the runtime consults it for which of the three parts a tag has and for the response and state types to check at the boundary. `Kind: fn Element -> Kind` in the prelude is a lookup in this table.

A `Path` is the chain of segments from the root, and the canonical spelling in transcripts is fixed by the conformance suite: segments joined by `/`, `Bind(i)` as `i`, `Gen(g)` as `#g`, `Key(k)` as `[k]` with the key printed as a Prism literal, `Supplied(n)` as `s/n` followed by the supplied widget's own segments, so a supplied widget whose body is one slot prints `s/0/0`. `Display` on `Path` prints exactly that, and both implementations print the same characters.

What takes a position. Only things that give the tree structure visible from outside take a `Bind` position: a slot, a `dyn`, a `for`, an `intercept`'s captured child, and `@started` (which is a `dyn`). They are numbered in build order within the nearest *addressable* node, which is the root, a `dyn` generation, a `for` key's block, a captured child or a supplied widget. A plain `<-` of a widget block or of a widget function application makes a node, for ownership, but the node adds no segment: its contents are numbered in the enclosing addressable node's sequence, because a plain child lives exactly as long as its parent and is not a unit of teardown. This is the rule that gives the counter its paths: `@Button $"Increment"` at the first statement of `@main` is a lifted application, which is a `dyn` taking position 0, whose generation `#0` holds the button's one slot at position 0, so `0/#0/0`; `@Label` the same at position 2; and `@Element Fs.File(path="/etc/motd")` with plain fields at the second statement of a file viewer is `1` with no generation, because no `dyn` is involved. Holds and `let`s take nothing.

`Gen(g)` counts from zero per `dyn` node and never reuses a number, so a rebuilt widget's slots get fresh paths and nothing addressed to the old generation can reach the new one. `Supplied(n)` numbers, per interceptor node, each widget the supplier built, in creation order, never reset; the slots inside it take their own segments under that prefix, so a supplied widget with a `dyn` prints `s/n/0/#g/0` and gets fresh paths per generation like any other. Paths compare lexicographically by segment, with `Bind` by index, `Gen` by number, `Key` by key order, `Supplied` by number, and the variants in that order; since at most one generation of a `dyn` is in any committed list, the order between generations never matters to a World, and tree order is path order. A path is an interned node pointing at its parent's path, so every slot under one node shares that node's prefix, minting a slot's path is one allocation, and comparing two sibling paths compares their parents by pointer and then one segment each.

A slot is a record in its node:

```rust
pub struct SlotRec {
    pub path: Path,
    pub element: ElementValue,
    pub response: Option<Rc<FanOutput>>,        // if the tag responds: this slot's output of the enclosing fan
    pub state: Option<Rc<Supplied>>,            // if the tag has state: keeps the supplied widget alive
    pub request: Option<(Event<Value>, Rc<RequestSink>)>,   // if the tag requests: the event and its collector sink
}
impl WidgetBuilder<'_> {
    pub fn element(&mut self, element: ElementValue, request: Option<Event<Value>>) -> Result<ElementResult, RuntimeError>;
}
pub struct ElementResult { pub response: Option<Event<Value>>, pub state: Option<Dynamic<Value>> }
```

`element` is what `@Element` compiles to. It mints the slot's path from the node's prefix and the position counter, creates the response output if the tag responds (a selection on the enclosing fan, below), asks for the state dynamic if the tag has state (state supply, below, which is where the `Err` can come from), subscribes the request event to the collector if the tag requests, and pushes the record. It returns what the tag declares; in a catch-all arm the compiler asks for both and checks the per-slot types at the slot. The slot lives exactly as long as its node.

### The element list

A list that waited for commit to change would be a list one occurrence stale, and the counter's transcript would show the old label for a turn after every click. So a node's element list is a derived dynamic, changing in the occurrence a child is built, and its value side, which lags to commit like any dynamic's, is exactly the delayed edge that routing reads.

```rust
pub type ElementList = PMap<Path, ListEntry>;   // the persistent B-tree of the list widget section
pub struct ListEntry { pub element: ElementValue, pub slot: Weak<SlotRec> }
pub struct ListPatch { pub inserted: PMap<Path, ListEntry>, pub removed: PSet<Path> }
```

A node's own slots are fixed for its life. The compiler is bound by that: an `Element` step runs only inside `build`, never in an occurrence-time closure, and the lint's mark check refuses the other placement. The builder enforces it: `element` is a method on `WidgetBuilder`, and a builder exists only for the duration of one `build`. So the only sources of change in a node's list are its children that can change: a `dyn`, whose list is the current generation's; a list widget, whose list is the union over keys; and an interceptor, whose supplied widgets come and go. A node's list is the union of its own entries and its children's lists (except a captured child's, which goes to the interceptor instead), and because every child's paths extend that child's segment, the union is disjoint and the map's order is tree order. For a node with fixed children this is a constant; under a `dyn` it is the generations joined; under a list widget it is a map-join over keys.

The list's change event fires in the occurrence a child is built, which is the construction rule applied to the list: the `dyn`'s generation is built at h+1 and the `dyn` node's list changes at h+2 with the new generation's entries, and each ancestor's list changes one height above its changing child's. The list's value stays the old list until commit while its change event has the new one; that is the ordinary dynamic contract, and routing reads exactly this delayed edge.

The change event fires with a *patch*, not a whole map. Rebuilding the union at every ancestor on any change below would cost the subtree's size per ancestor, and a GUI pays that on its first click: one label deep in a window of a thousand rows would copy a thousand entries at every level up to the root. So a `dyn` rebuild's patch is "remove the old generation's entries, insert the new generation's"; a list widget's is the union of the appearing keys' lists and the disappearing keys' paths; a parent's is the union of its changing children's patches, which is cheap because the paths are already distinct. Each node's list value is a `PMap` maintained by applying its patch at commit, which shares every untouched subtree with the previous value, so no World holding an older list forces a copy. The run call reports the root's patch; `Runtime::list` hands out the root's map. A program observes nothing of this: `$slots` in an interceptor is the map, and where Prism code reads its change event, the runtime applies the patch to the old map persistently, in time proportional to the patch, and the value is that new map.

### Routing

A button relabels itself on click: the label changes in occurrence n, the generation is rebuilt, and a click the World sent in that same occurrence was aimed at the old slot, because the old slot was in the list the World had. That click has to land on the old slot, which is still in the tree until commit, and a click aimed at the new slot in n has to be refused, because no World could have known its path. Routing therefore consults the list as the occurrence opened and nothing else, through a keyed fan, one per interceptor and one at the root:

```rust
pub struct ResponseFan {
    source: Event<Rc<[(Path, Value)]>>,          // the responses event, or the run call's inputs
    table: Behavior<ElementList>,                // the captured child's list, value side
    born: u64,                                   // the occurrence the fan was built in
    outputs: RefCell<BTreeMap<Path, Weak<FanOutput>>>,
    owner: Weak<WidgetNode>,                     // the interceptor's node, named in errors
}
```

A `FanOutput` is an event with the fan as its upstream and height one more than the source's. When the source fires, the fan reads `table` with `query_for_tag` and looks each `(path, value)` up in it. A list's value side changes only at commit, so the read is the captured child's list *as the occurrence opened*, whatever has been built since; a path not in it is `UnknownPath`, which aborts the occurrence and names the interceptor; a path in it whose output has no subscribers is skipped at no cost; a path with a live output gets its value, once. The lookup against the opened list is the routing rule, and because the fan reads the list through a tag, the lookup is a delayed edge: `!down` may depend on `$slots` through a `for` and `$slots` may change because of what `!down` delivered, and no height runs backwards. Cost is per fired path, not per slot: a slot that receives nothing is never touched, and an occurrence that delivers nothing to a fan costs that fan nothing.

One case needs the stamp. A fan built in occurrence n belongs to an interceptor built in n, so every slot under it was created in n, and none of them was in any list a sender could have read. The list's value as a tag reads it in n is the list as built, since the node did not exist before n, and would route to those slots. So a fan whose `born` equals the current occurrence treats every lookup as `UnknownPath`, which is the rule that pure code firing at a path created in the same occurrence is an error. From n+1 on, the value side is the committed list and the stamp no longer matters.

A copy of every fan's table taken at open, by a walk over all fans, would give the same answers at a cost proportional to the number of interceptors in every occurrence, including occurrences that deliver to none of them. The value side already holds the opened list, so there is nothing to copy.

`outputs` is bounded by the live slots under the fan, not by every slot ever created there. The `FanOutput` is owned by its `SlotRec`, and `SlotRec`'s `Drop` removes its path from the fan's map, so an entry leaves the map in the same commit that drops the slot; the fan is reached from the slot through a `Weak`, and a fan already gone means nothing to remove. The runtime is bound by this and the checker asserts it: after every occurrence, the fan's map has exactly one entry per slot in the captured child's list that responds. Consolidating on the next lookup, the way a subscriber list does, would also keep the count bounded, but it would leave dead entries in the map between lookups and make the checker's count a lie for an occurrence.

### Request collection

A slot with a `requests` field holds the program's event. The run call has to report, per slot, the values it fired in this occurrence, in order. The collector is the `requests` vector on the core, and a sink per slot feeds it. This replaces the delegate callback in today's crate, through which the test backend hears event firings and which hands it the node's live slots; with the collector, nothing outside the runtime ever holds a runtime object.

```rust
struct RequestSink { path: Path, core: Rc<Core> }
impl EventCallback<Value> for RequestSink {
    fn event_fired(&self, _: &Runtime, v: Value) { self.core.requests.borrow_mut().push((self.path.clone(), v)) }
    fn already_fired(&self, v: Value)          { self.core.requests.borrow_mut().push((self.path.clone(), v)) }
    fn invalidate_height(&self) {}
}
```

At slot creation, if the tag requests, `element` subscribes the sink to the request event, with catch-up, so a request event that fired earlier in the creation occurrence (`Read on !started` in a block built mid-occurrence; `@started` fires at the done height, which is above the build) is recorded too. The sink is owned by the slot record, so it dies with the slot; the request event is held strongly by the slot so that laziness does not disconnect it when nothing else listens. At close, `run` groups the vector by path in the order given under the occurrence's close phase and clears it.

A request from a slot that was torn down in the same occurrence is still in the vector, with its path, and is reported. There are two ways this happens: the slot's node was replaced by a `dyn` rebuild or a key's removal later in the occurrence, in which case the node was alive when the request fired and is dropped at commit; or the slot was created and torn down in one occurrence, a `dyn` built at h+1 and rebuilt again at some h' > h because its dynamic changed twice in n through two different paths, in which case the first generation's `@started` request was collected before the second build. In both the World performs the request if it can and discards any response, because the slot is gone and the next list says so. The collector never filters by liveness, which is why it needs no reference to the tree.

The World never sees an event; it sees a list of what fired.

### State supply

A `state` element has a value in the occurrence it is created, and the value comes from whoever stands outside: an interceptor's supplier, or the World's oracle at the root. The mechanism is the same at every level and has no table and no chain to walk.

Every node knows its nearest enclosing *supply context*, a field set by the builder when an `InterceptWidget` builds its child and otherwise inherited from the parent:

```rust
pub struct SupplyContext {
    supplier: Supplier,
    memo: RefCell<BTreeMap<ElementValue, Weak<Supplied>>>,
    owner: Weak<WidgetNode>,                      // the interceptor's node, or the root
    next_supplied: Cell<u32>,                     // the Supplied(n) counter
}
enum Supplier {
    Widget(Dynamic<Rc<dyn Fn(&ElementValue) -> Rc<dyn PolymorphicWidget<Dynamic<Value>>>>>),
    Oracle,
}
pub struct Supplied { node: Option<Rc<WidgetNode>>, dynamic: Dynamic<Value>, trigger: Option<EventTrigger<Value>> }
```

When `element` is called for a tag with state, it asks the context for the slot's dynamic by element value. The context looks in its memo first: if a `Supplied` for an equal value is alive, it returns it, the slot takes a strong reference, and *the supplier is not consulted*. A new slot whose value already has a live supplied widget joins that widget whatever the supplier dynamic says at that moment; the supplier is read only for a value no live supplied widget serves. This is value identity inside an interceptor held by construction, and it is what the migration interceptor relies on: two child slots for one key never disagree, and a key migrates to a new backend only when its last reader is gone and a new one arrives.

On a miss under an interceptor, supplying means reading the supplier function from its dynamic right now (`query_for_tag`, the value as the occurrence opened, which is what "a slot's policy is fixed at creation" means), applying it to the element, and building the returned widget as a child of the interceptor's node, then and there, in the same build pass, as a journaled build with the nested-transaction boundary. The widget takes the next `Supplied(n)` position in the interceptor's node, and its slots take their own segments under it. The order is build, then memoise: the supplied widget is built to completion under the nested-transaction boundary, and only when its build returns with a result dynamic is the `Supplied` record made, memoised weakly, and handed to the asking slot strongly. A supplier build that fails (a cycle at a placeholder close inside it, a state tag it asks for that the next context out cannot answer, a panic in one of its build-time closures) is truncated like any failed build, leaves nothing in the memo, and surfaces to the asking slot as the `Err` of `element`, with the supplier's error as the cause: the asking slot's own build then fails, and the failure is contained or fatal by the ordinary rule, under a `dyn` as an error node and in occurrence zero as the `Err` of `build`. The slot never sees a half-built `Supplied`, and a later slot with the same value asks the supplier again, since the memo has no entry for it. Because the supplied widget is built in the interceptor's node, its own elements go into the interceptor's list and its own state slots ask the *next* context out, which is how `\|e| @Element e` forwards a state element one level: the supplied widget is one `@Element` with the same value, whose slot asks the outer context, and so on up to the root. A supplied widget's result dynamic is read by the child's slot in the child's creation occurrence, and it has a value then by the same rule applied one level up.

At the root the supplier is the oracle. Supplying is: call the oracle with the element value to get the value before this occurrence; make a *root state cell*, an external trigger and a hold of it initialised from the oracle's answer; memoise it by element value in the same memo. The run call's pushes are deliveries to these triggers at height 0. A slot created in n reads the oracle (the value before n, from the World's previous snapshot) and then, if n's pushes include its value, receives the push in n and ends n holding the new snapshot, exactly like a slot that existed before n. Two slots with equal values share one cell and therefore one dynamic. A state tag the World does not answer is a build error, `UnsuppliedState`, raised at the slot's creation with the element and the slot's span; the oracle's signature cannot express refusal because the specification says it cannot fail, so the world core checks the tag against its table before the call and the runtime checks the answer's type after it, and either failure is the error in the occurrence the slot is created, contained to a `dyn` build when it happens in one and fatal to `build` when it happens in occurrence zero.

The memo holds `Weak<Supplied>` and the slots hold strong references, so the teardown of the last slot with a value frees the supplied widget at the same commit; nothing scans the memo. A supplied widget torn down this way is an ordinary node drop under the interceptor's node, so its elements leave the interceptor's list in the same patch, and the outer context sees the interest go the same way.

### `InterceptWidget`

Forwarding is manual, so the interceptor has no policy to implement: `@Forward`, the sandbox, the VBox and the cache are Prism programs over `intercept`, `for` and `@Element`, and the runtime does not know they exist. The runtime keeps a fan, a supply context, a captured child and its list, and nothing else.

```rust
pub struct InterceptWidget<W: Widget> {
    child: W,
    responses: Event<Rc<[(Path, Value)]>>,
    supplier: Dynamic<Rc<dyn Fn(&ElementValue) -> Rc<dyn PolymorphicWidget<Dynamic<Value>>>>>,
}
impl<W: Widget> Widget for InterceptWidget<W> {
    type Output = (W::Output, Dynamic<ElementList>);
    fn build(&self, builder: &mut WidgetBuilder) -> Self::Output;
}
```

Its `build` does four things and nothing else. It makes a `ResponseFan` with `responses` as the source, the captured child's list behavior as the table, and the current occurrence as its birth stamp. It makes a `SupplyContext` with its supplier and its own node as owner. It binds the child, at a `Bind` position of its own node, with a builder whose fan and context are these, so that every slot the child creates, at any depth that does not pass another interceptor, gets its response output from this fan and its state from this context. And it captures the child's element list: the child node's list dynamic is the interceptor's second result and does *not* join the interceptor's own list. The interceptor's own list holds only the slots its node makes with `element`, which includes those made by supplied widgets.

That is the entire mechanism.

The root is an `InterceptWidget` the runtime makes itself in `build`: its child is `@main`, its responses event is the run call's input trigger, its supply context is the oracle, and its captured list is the one the run call returns.

Two alternatives show what this buys. Suppose state supply were a table: the runtime keeps, per interceptor, a map from tag to supplier, and a slot walks outward through the interceptors until a table answers. Then an interceptor that wants to forward a state element with a rewritten path needs a rewrite entry, one that wants to cache needs a memo entry, one that wants to supply from a dynamic needs a dynamic entry, and the table has become the second language the specification refuses. With the supplier being a function to a widget, the rewrite is `@Element (Rewrite e)`, the cache is a hold around `@Element e`, the dynamic supplier is the dynamic the interceptor was given, and the runtime implements one thing: build the widget here, now, and memoise by value. Suppose, instead, that responses were routed by the runtime walking the tree: an interceptor registers a filter, the runtime applies filters as it descends. Then an input the interceptor wanted to rewrite or hold back would need a mode, and simultaneity between the interceptor's own responses and the forwarded ones would need a rule the runtime chose. With one responses event per interceptor, the merge is `|` in the interceptor's own code, the rule is the one the language already has, and the runtime routes against one snapshot and does nothing else.

### The canonical node structure of a lifted application

A hand-written widget in the harness and the compiled form of the same Prism must produce the same paths and the same transcript, so the structure that a lifted widget application builds is fixed here, once.

`!x <- @Button $label`, where `@Button` takes a plain label, is `dyn $label: @Button _` with `!x` the `!Switch` of the `dyn`'s event. It builds, in the enclosing node:

1. A `DynamicWidget` node at the next `Bind` position, over the dynamic `label.map(|l| button_widget(l))`, where `button_widget` is the widget function's body closed over the plain label. The generation `#g` is the body's node; the body's own position-takers are numbered inside it from 0.
2. In the enclosing node's values, `x = dyn_event.map(|out| out.event).switch_hold()`, owned by the enclosing node through its environment vector.

For a widget whose result is a dynamic, step 2 is `Dynamic::hold(first, dyn_event.map(|out| out.dynamic)).join()`, where `first` is the first build's result, which the builder has in hand because the first build happens inside the `DynamicWidget`'s own build and its output is returned from it (the `dyn` returns both its event and the first build's output, as a pair, for exactly this use). For a widget whose result is plain, `$x <- @Label $text` binds a `$` holding the first build's value, and step 2 is `Dynamic::hold_first(dyn_event.map(|out| out.value))`: a hold whose initial value is the first firing, which has happened by the time the hold is built, and which changes to each rebuild's value in the rebuild's occurrence. A hold built from `first` and the event would be the same thing with a second copy of the value; `hold_first` exists so that the compiler emits one step and the text form prints one node. For a bundle, field by field by the kind of each field. For an element with a dynamic where a field goes, the same with `@Element` inside the generation: `@Element Ui.Label(text=$t)` is a `DynamicWidget` over `t.map(|t| element_widget(Ui.Label { text: t }))`, and its generation's node has one slot at position 0, which is why the label is `2/#g/0`.

The path consequence is the same in all three cases, because the `dyn` is the same: a lifted application takes one position, its generation holds the body's slots from 0, and the bound name, whether `!x`, `$x` or a plain-valued `$x`, takes none. A harness fixture for `@Label $text` with a plain result is a `DynamicWidget` over the label whose generation makes one slot and returns `unit`, and the transcript shows `n/#g/0` with a fresh `g` on every text change.

A hand-written `Button` in the harness that wants to match this must therefore not be "a node with a slot and a `dyn` inside it"; it must be the `DynamicWidget` itself, built with `dynamic_widget` over the label, with the slot made inside the generation. The harness's fixtures are written that way, and the compiled `@Button` from the GUI library must match their transcript byte for byte.

### Compiled widgets

Three places build widgets at occurrence time: a `dyn` rebuilding, a `for` key appearing, and a supplier answering a state slot. Each hands the runtime a widget it has never seen, and the rebuild path must not care whether it got a block of `@main`, a widget function's body, or something a hand-written Rust World made. So every compiled widget block is one Rust type, `IrWidget`, implementing `PolymorphicWidget<Value>` (or `PolymorphicWidget<O>` for the typed result the block declares), and `Rc<dyn PolymorphicWidget<Value>>` is what all three places take. An `IrWidget` is a reference into the compiled program plus the values it captured:

```rust
pub struct IrWidget {
    pub program: Rc<Program>,      // Program { functions, blocks, tags, constants }, shared by every widget of one program
    pub block: BlockId,            // which of program.blocks this widget builds
    pub captures: Rc<[Value]>,     // the values the block captured from its enclosing node
    pub span: Span,
}
```

Its `build` walks `program.blocks[block]`'s steps in the group order the resolver computed, with an environment vector for the block's values. The steps are the block's, not the widget's: two `IrWidget`s over one block with different captures are two nodes from one description, which is what "a widget is a description" means at the Rust level.

```rust
enum Step {
    Let { dst: usize, expr: PlainExpr },                      // build-time plain value
    Bind { dst: usize, widget: WidgetExpr },                  // child widget: builder.bind (no segment)
    Hold { dst: usize, initial: usize, event: usize },        // Dynamic::hold
    HoldFirst { dst: usize, event: usize },                   // Dynamic::hold_first
    Map { dst: usize, src: usize, f: ClosureId },             // filter_map / map
    Tag { dst: usize, event: usize, dynamics: Vec<usize>, f: ClosureId },   // tag_map_n
    DMap { dst: usize, dynamics: Vec<usize>, f: ClosureId },  // map_n
    Leftmost { dst: usize, srcs: Vec<usize> },
    SwitchHold { dst: usize, src: usize },
    Join { dst: usize, src: usize },
    Updated { dst: usize, src: usize },                       // Dynamic::event
    Distinct { dst: usize, src: usize, eq: ClosureId },
    Started { dst: usize },                                   // takes a position
    Dyn { dst: usize, dynamic: usize, block: BlockId, captures: Vec<usize> },  // takes a position
    For { dst: usize, map: usize, block: BlockId, captures: Vec<usize>, kind: Consolidation },   // takes a position
    Element { dst: usize, tag: TagId, fields: Vec<usize>, request: Option<usize> },   // takes a position
    Intercept { dst: usize, child: usize, responses: usize, supplier: usize },        // takes a position
    Placeholder { dst: usize, kind: PlaceholderKind, span: Span },   // add_cyclic_*
    Close { placeholder: usize, src: usize },                       // CyclicEvent::close / CyclicDynamic::close
    Return { src: usize },
}
```

Every step is one call on the builder or one constructor on an event or dynamic, with operands resolved to environment indices. Occurrence-time expressions compile to closures that the corresponding graph node calls, and a closure is a function id into `program.functions` plus a capture list resolved to indices into `captures`, which is copied into the node at build. A `Dyn` or `For` step makes its child widgets as `IrWidget { program, block, captures: this node's values at the listed indices plus the rebuild value, span }`, so the three occurrence-time build sites and the compiler agree on one constructor. Typed-result blocks (a supplier's, returning `Dynamic<Value>`) are `PolymorphicWidget<Dynamic<Value>>` and are downcast nowhere, because the compiler chose the type at the call site. The adapter goes; the blanket impl stands in for it.

### Closures and ownership

A compiled closure captures by index into its own node's values, never by pointer into a sibling's node. The values it captures are either plain data (cloned) or handles to events and dynamics (an `Rc` clone). The handle edges run in one direction: a closure captures things bound *earlier* in group order, which are things built earlier, and the graph edge from a source event to the node that subscribes to it is a `Weak` in the source's subscriber list. So every strong edge points from later to earlier in build order, every reverse edge is weak, and there is no strong cycle through the heap. The widget tree holds every node strongly from the root down, so everything reachable is owned, and when a node is dropped everything it built is dropped unless a parent holds a result handle, in which case the result keeps working as the specification says: an event that never fires again, a dynamic at its last value.

The exception is a cyclic group, where a later binding is referenced by an earlier one. The group is closed through placeholders, and the placeholder is the one edge that points from earlier to later, and it is weak. The node owns the strong ends of every loop it closes in its `cyclic_events` and `cyclic_dynamics` slots, and it owns them as a unit: a group's placeholders are added before the group is built and closed after, and the compiler's lint checks that every placeholder is closed exactly once and that nothing samples one before its close. Dropping the node drops the slots, the loop's only strong roots, and the loop goes with it. A cyclic dynamic whose node is gone but whose handle a parent still holds keeps its last value, because teardown writes the dynamic's current value into a cell the computation falls back to when the slot no longer upgrades.

Two more rules keep this true across the constructs that build at occurrence time. A `dyn` block or `for` block compiled as an `IrWidget` captures from its *enclosing* block's environment, which is the enclosing node's values; the child node then holds handles up into its parent, which is the direction that is allowed. And a supplied widget is built in the interceptor's node and captures from the supplier closure, which was built in the interceptor's block; again handles point up. A child never holds a strong handle to anything in a sibling or in its own descendants, because nothing in the language can name those.

**The recursion depth guard.** A plain function in Prism may recurse without bound, and a host stack overflow is not a panic in Rust; it ends the process. The closure compiler emits, at the entry of every compiled function body, an increment of the core's `depth` counter and a check against a limit, and at exit a decrement; past the limit it raises the runtime's `PrismPanic` with kind `Overflow` and the function's span, which the action loop catches and turns into the occurrence's `RuntimeError`. The limit is the implementation's (a few thousand frames, sized to the host stack with margin) and is a field on the core a World may set; the specification requires only that a conforming implementation neither hangs nor dies. Build-time functions run inside `build`, where the same counter guards them and a failure is a build error, contained or fatal by the usual rule.

### An occurrence traced through the target

Take the counter from the worked examples, built as `@main`, and take occurrence 1, in which the World delivers a click to the Increment button. The list the World received after occurrence 0 was

```
0/#0/0: Ui.Button(label="Increment")
1/#0/0: Ui.Button(label="Decrement")
2/#0/0: Ui.Label(text="Count: 0")
```

*Open.* `run` increments the counter to 1. No quarantine. The root list's committed value is the three paths above, each with a weak pointer to its slot.

*Fire.* `responses = [(0/#0/0, click)]`. The path is in the committed root list; the tag `Ui.Button` responds with `Click`; the value is a `Click`; the path has not been given a response yet. The root fan is scheduled at height 0 with the one entry. No pushes.

*Propagate.* Height 0: the fan fires; its output for `0/#0/0` is the response event of generation 0 of the `dyn` that is `@Button $"Increment"`, and it has a subscriber, the `switch_hold` that is `!increment`, which adopted this inner at occurrence 0's commit; the output fires at 1 and the switch at 2. `!delta` is `leftmost` over two maps of the two buttons' clicks, at 3, and fires with `+1`. `$count + !delta` is a `tag_map` over `!delta` reading `$count`'s behavior, which is 0, the value before the occurrence; it fires `1` at 4. `$count` is `Dynamic::hold(0, that)`, so its change event is that event, and the derived `"Count: {$count}"` is a `map` of it at 5, firing `"Count: 1"`. The `@Label`'s `DynamicWidget` subscribes to that map; its `DynamicWidgetEvent` schedules a rebuild at 6. Height 6: the rebuild builds generation 1 of the label block at 7, journaled under the `dyn` node at `Gen(1)`: the block's one step is `element(Ui.Label { text: "Count: 1" }, None)`, which mints path `2/#1/0`, creates no response output and no state, and pushes the slot. The `dyn` node's list recomputes at 8 and emits the patch: remove `2/#0/0`, insert `2/#1/0`. `@main`'s node is the root interceptor's captured child; its list emits the same patch, and that list is the one the root captured. The old generation's node goes onto the commit list. The request collector has recorded nothing, because no slot in this program requests.

*Commit.* The hold writes `1`. The old label node is dropped, which drops its slot. The list patch is applied at every node it passed through: the root list's value is now the three entries with `2/#1/0: Ui.Label(text="Count: 1")`, and `2/#0/0` is gone from it, which is why a later response to that path is `UnknownPath`.

*Close.* The outcome is the patch (remove `2/#0/0`, insert `2/#1/0`), an empty request vector, `unit`, and no errors. The World applies the patch to its own copy, redraws the label, and waits for the next click.

Had the World sent the click to `2/#0/0` in occurrence 2 instead, it would have been sending to a path not in the list it received, and `run` would have returned `UnknownPath` before scheduling anything. Had a closure in the chain panicked at height 4, the commit list (the hold write) and the journal (empty at that point) would have been dropped, the occurrence number left behind the stale caches, and the hold would still read 0.
## Invariants

A runtime with no scheduler, no dirty tracking and no collector is held together by invariants instead, and the point of this section is to write them down with who is bound by each and what enforces it. The rule for every one is the same: it is either enforced by construction, in which case there is a sentence saying what construction, or it is checked, in which case the debug checker below asserts it after every occurrence and the conformance suite has a test that would fail if it broke.

**At-most-once.** Every event fires at most once per occurrence; every dynamic's change event fires at most once; every closure runs at most once per occurrence. *Bound:* the runtime. *Enforced:* `record_once` on every subscription manager. The cache is written once per occurrence and every later reader, by push or by catch-up, reads it. A second firing of a node in one occurrence is impossible because the manager will not compute twice; the only way to ask for one is a node scheduled twice at different heights, which `schedule`'s assertion turns into a runtime bug report if the second is at or below the current height and `record_once` turns into a no-op otherwise.

**Height order.** An action runs only at a height strictly above every height already run in this occurrence, and every derived event's height is strictly above its prompt sources'. *Bound:* the runtime and every event implementation. *Enforced:* `schedule`'s comparison; the `height` method's `+ 1`; the invalidation rule, which keeps cached heights true after a retarget; the eager walk at every edge creation, which refuses an edge that would make a height inconsistent. The checker verifies that the heights actually run were strictly increasing.

**Delayed edges run at commit.** A hold's value changes only in the commit batch; a switch's inner changes only in the commit batch; a replaced node is dropped only in the commit batch; a list-widget key and a supplied widget are removed only in the commit batch. *Bound:* the runtime. *Enforced:* every one of these is a `Commit`, and nothing else writes to a hold's value, a switch's inner or a node's children during propagation. This is the tag law, the non-promptness of `!Switch`, and the "replaced node keeps working" clause, all from one rule. The checker samples every hold before and after the occurrence and asserts the value changed if and only if its event fired, and to the event's value.

**Dynamic agreement.** For every dynamic, after every occurrence, the behavior's value equals the change event's value if it fired, else the previous value. *Bound:* every constructor of a `Dynamic`. *Enforced:* by construction for `hold`, `constant`, `map`, `map2`; by the `on_outer` design for `join` and the map-join; and by the checker, which registers every dynamic built under it and asserts the equation after every occurrence. This is the invariant `Dynamic::new` is marked `unsafe` to protect, and the checker is what makes it safe to unmark.

**Construction sees all of n.** A node built at any point in occurrence n ends n in the state it would have been in had it been built before n's inputs. *Bound:* the runtime. *Enforced:* `subscribe` returning the already-fired value; `MainSubscription` routing a firing to the catch-up path when the subscriber's height has passed; holds and switches deferring to commit on catch-up; `dyn` building the already-fired value first; the collector's sink subscribing with catch-up. The checker cannot state this directly; the law tests state it, one per subscriber kind.

**Routing against the opened list.** A response addressed to a path is delivered to the slot at that path in the list as the occurrence opened; a path not in that list is an error that aborts the occurrence. *Bound:* the runtime, and the World and every interceptor, which may only send to paths in the list they last received. *Enforced:* every fan looks paths up in its child's list through a tag, whose value side changes only at commit, and a fan built in the current occurrence routes nothing; the run call checks every path against the committed root list before scheduling anything. The checker copies each fan's committed list before the occurrence and asserts after it that every delivered path was in the copy.

**The diode.** Inputs cause list changes; list changes never cause inputs. *Bound:* the runtime, the World, and every interceptor. *Enforced:* for the runtime, by the fan's lookup being a tag read of the list, a delayed edge, so that no prompt path leads from a list change to a response event; for the World, by the structure of the world core's turn loop, which has no code path from an outcome's patch to a run call's inputs except through a component's own event or a `Defer` the program fired, so a World built on the core cannot answer a list change with an input, and by the Test World's `Send` steps, which are script lines a list cannot write; the harness's two boundary checks are the test of it for a World written by hand; for an interceptor, by the cycle check, which rejects any prompt path from a child's list change to the interceptor's responses event, and which admits the membership edge of a `for` over the list because a newborn block fires nothing a response could cause.

**The creation rule.** No slot receives a response in the occurrence it was created, and no state slot lacks a value in that occurrence. *Bound:* the runtime, the World, every interceptor. *Enforced:* the first by routing against the opened list, since a slot created in n is not in it; the second by state supply at slot creation, which cannot return without a dynamic or an error. A World that answers in the request's occurrence is impossible, since the request leaves the run call only after commit.

**Equality of state.** Two slots with equal element values under one supply context hold the same dynamic. *Bound:* the runtime and every supplier. *Enforced:* memoisation by element value in every supply context, consulted before the supplier, with the root cell shared the same way. The checker walks the list and asserts that equal-valued state slots under one context share a dynamic by pointer.

**Ownership is a tree.** Every event, behavior, dynamic and node is owned, directly or through handles, by exactly one node, and the root owns everything. *Bound:* the runtime, the compiler and every widget written in Rust. *Enforced:* the direction rules on captures, weak subscriber lists, weak placeholders, the compiler's lint on capture lists. The checker, in tests that drop a subtree, takes weak pointers into it beforehand and asserts none upgrades afterwards.

**Stale caches are unreadable.** No per-occurrence cache written in occurrence n is read in any later occurrence, on success or on error. *Bound:* the runtime. *Enforced:* the occurrence stamp on every cache, compared against the core's counter on every read; the counter is incremented at every open, including after a failed occurrence. The checker asserts after every occurrence that no manager's stamp is ahead of the counter.

**Rollback leaves no trace.** After `Err`, the graph equals the graph after the previous `Ok`. *Bound:* the runtime. *Enforced:* buffered commits, the append-only tree during propagation, the journal, the nested-transaction truncation in every build. The checker, after an injected failure, compares every hold, every switch's inner, every node's children and the root list against the snapshot taken before the occurrence.

**One call, one occurrence.** The runtime opens an occurrence only inside `run` or `build`, and exactly one per call. *Bound:* the runtime. *Enforced:* there is no other code path to the loop; `schedule_trigger` is `pub(crate)` and called only from `run`. The checker counts occurrences and compares with calls.

**The checker** is a debug-only `Invariants` struct that a test attaches to a core. It registers every dynamic, hold, switch and fan at construction (through a hook the constructors call under `cfg(debug_assertions)`), snapshots their observable state before `run`, and asserts the invariants above after. It runs inside every runtime unit test and inside the Test World under the conformance suite in debug builds, so that any sentence of the specification an implementation drifts from fails a test that names the sentence. It is the one piece of test scaffolding kept forever.

## Testing the runtime

The runtime is tested at two levels, and the levels test different things. The unit tests drive the graph directly in Rust, the way the 38 do today, because a law about `switch_hold` or `join` is easiest to state against the combinators with no program in the way. The harness tests drive the run call with Rust-defined tags, the way a World does, because routing, paths, state supply and interception are statements about what crosses the boundary, and a test that reads anything else is testing the implementation's structure rather than the specification. Neither level touches Prism source; that is the conformance suite's layer, where `Fired` and `Quiet` catch an input that went nowhere because a pattern failed to match, and the twelve integration programs wait for it.

**The harness.** A small Rust World over `Runtime::build` and `Runtime::run`, with a tag table of a dozen Rust-defined tags (a button that responds, a label, a file that requests and responds, a contents element with state, `Defer`), an oracle backed by a `BTreeMap<ElementValue, Value>` the test sets, a turn loop that answers `Defer` and nothing else, and fixtures that build widgets with the same builder calls the compiler emits, in the canonical node structure. After every occurrence the harness runs the invariant checker and two boundary checks of its own: every path it delivered to was in the list it had received before the occurrence, and every path the outcome's patch inserts has a prefix that changed in this occurrence (a `Gen`, `Key` or `Supplied` segment), which catches paths drifting under an unrelated rebuild; and the previous list with the patch applied equals `Runtime::list()`. It drives only through plain data: no triggers, no `query_for_tag`, no node handles. A test that needs a value reads it from an element. Its transcript printer is the one the world core takes over, kept in one small file for that reason.

**The scenario tests.** Each names a sentence and what it would catch if it failed. The third column says where the test lives: *suite* means the conformance chapter owns it, as a law test, an integration program or an entry in its runtime-test list, and this chapter only names it; *harness* means it needs something a Prism program cannot write (a Rust-defined tag, an injected panic, a count of internal deliveries, a quarantine input) and lives only here.

| Scenario | What it catches | Where |
| --- | --- | --- |
| A nested `dyn` whose inner rebuilds in the same occurrence as its parent | The rebuild path losing its node; a rebuild needing anything but the ready event | suite |
| A cyclic event closed in terms of itself promptly, on a thread with a timeout | A cycle hanging instead of returning kind `Cycle` with the loop's spans, at the close | suite |
| A switch adopting an inner that depends on the switch | The cycle found lazily in the next occurrence instead of at the commit of the switching one; the owner path missing | harness |
| An interceptor whose responses event depends promptly on its child's result event | A prompt loop through `intercept` slipping past the cycle check | suite |
| An interceptor whose responses are a `for` over its own child's list, forwarding everything | A false cycle from the membership edge; the sandbox and the VBox rejected | suite (`@Forward`) |
| A `for` whose row result is `@ Delete on !started`, fed back to the map | The membership edge turning into a data edge without a cycle report | harness (the compiler refuses the source; the runtime's report needs a hand-built widget) |
| VBox-style hit-testing: tag a pointer with the opened list, route by the result | A false cycle on legal layout code | suite (the GUI library) |
| The relabelling button: label change in n, click in n, click in n+1 (timeline below) | Routing against the live list; the old generation torn down early; the switch turning prompt | suite (chapter 2's program) |
| A slot removed in n still receives a response addressed in n, and is gone in n+1 | Teardown at the wrong point of the occurrence | suite |
| Rebuild an unrelated sibling; insert at the front of a `for` over a map; rebuild a `dyn` twice | Paths drifting under a sibling's change; keys losing their paths when entries shift; a generation reused | suite for the insert; harness for the sibling and the double rebuild |
| A thousand slots, one response; count fan deliveries | Per-slot filtering coming back in place of the fan | harness |
| The three `join` law tests and `join_is_prompt` | Event and behavior disagreeing; the join waiting for a change instead of taking the current value | suite |
| Root build: count the `dyn` event's and `@started`'s firings over several occurrences | Occurrence zero missing; either firing twice | suite |
| `@started` inside a `dyn` block rebuilt in n | `@started` not firing in the rebuild occurrence | suite |
| A child under `dyn` and `!Switch` whose block fires in its build occurrence | The switch turning prompt; the parent must not see it | suite |
| A `for` that gains a key in n, with the block returning a plain value, a dynamic, and an event that fires at birth | Any consolidation lagging; the newborn's birth firing missing from the event consolidation | suite for the event kind; harness for all three in one fixture |
| A state slot created in the occurrence of a push to its value | The push missing the new slot; the oracle answering after the push instead of before | suite |
| Two child slots of equal value under one interceptor, the supplier changed between them, then one torn down | Two supplied widgets; the second slot consulting the supplier; teardown with the first slot instead of the last | suite |
| A request fired by `@started` in a block that is torn down in the same occurrence | The request missing from the outcome; the outcome without its path | suite |
| An injected panic at height 4 of a six-height occurrence, then a normal occurrence | Rollback leaving a hold written, a node built, or a cache readable | suite's runtime-test list; injected from Rust |
| A `dyn` build that fails at the close of a placeholder, inside an occurrence that otherwise succeeds | The occurrence aborted instead of continuing; the `dyn`'s event firing; the `Error` element missing; a commit or a collector entry from the failed build surviving | harness |
| A quarantine instruction after a switch cycle, with a response to a path under the owner in the same call | The owner not replaced; the error element absent; the response reported as unknown instead of discarded | harness (quarantine scripts are Rust-only) |
| A plain `Widget` boxed as `Rc<dyn PolymorphicWidget<_>>`, and a `dyn` that rebuilds into a different widget type | The erased widget path | harness |
| Several `run` calls in a row, counting occurrences | The runtime opening an occurrence on its own | suite |
| A function recursing past the depth limit in an occurrence-time closure, and in a build-time one | The process dying instead of an `Overflow` error; a build failure not contained | harness |

**The timeline.** The relabelling button is the one scenario worth writing out, because it exercises routing, the switch and teardown at once. `@Button $label` is a `dyn` over the label with an element inside and a `!Switch` over the generations' responses. The label changes in occurrence n; call the old slot `k1` (generation 0) and the new one `k2` (generation 1).

| | list's change event | list's value (opened) | routable | `!Switch` follows |
| --- | --- | --- | --- | --- |
| n | `k2` | `k1` | `k1` | `k1`'s response |
| n+1 | none | `k2` | `k2` | `k2`'s response |

A click the World sends in n was aimed at `k1`, because the World read the list after n−1, and it lands on `k1`: the old generation is still in the tree in n, its response output is the switch's current inner, and the click reaches `!click`. A response sent to `k2` in n is `UnknownPath`. At n's commit the old node is dropped and the switch adopts `k2`'s response. In n+1 the World has the list with `k2`, sends there, and the click arrives through the new generation. The harness asserts all four cells of both rows.

## The gap

In the order worth closing. Each step keeps the suite green and adds the law tests it enables; none is sized larger than a model can be handed with this chapter and the previous step's code.

1. **`Rc`/`RefCell`, the shared core, re-entry flags.** Mechanical, touching every file, so done first and alone, with no behaviour change. Every graph object gains its `Rc<Core>`; `Runtime` gets its hand-written `Send`; `Value` gains `I64` and `U64` and loses the 32-bit variants. Closes the known issues "`Arc`/`Mutex` everywhere", "Events are not tied to a runtime" and "`Dynamic::new` is `unsafe` for a logical invariant". *Done when:* no `Arc`, `Mutex`, `Send` bound or `Sync` bound remains in `src/` outside the one `unsafe impl`; the cycle tests assert the new messages; a graph built on one thread runs on another after the runtime is moved; `Dynamic::new` is `pub(crate)`.
2. **The commit list, the journal, stamped caches, and rollback.** `propagate` becomes `pub(crate) fn run_occurrence(&mut self) -> Result<(), RuntimeError>` with the panic path returning `Err` and the stale-cache clear replaced by the stamp; nodes built during propagation are journaled. Closes "Scheduling at `usize::MAX` while at `usize::MAX`". The invariant checker lands here and runs in every test from now on. *Done when:* an injected panic returns `Err` with the graph unchanged, as the checker asserts; a `dyn` rebuild in a failing occurrence leaves the old child in place and the new one gone; `a_trigger_dropped_by_a_panic_accepts_values_again` still passes with no trigger flag involved; no clear-at-end list exists.
3. **Eager cycle detection and the build boundary.** `height_checked`, the in-progress flags collecting spans, the walk at placeholder close and at switch retarget, the nested-transaction truncation in `bind` and `build_root`, the error node. Closes "Cyclic handles are bare `usize` plus a name", "Unclosed placeholders are found at the first propagate" and "The `DynamicWidgetEvent` has no height cache and no owner". *Done when:* the two cycle tests assert `Cycle` with the loop's spans; a cycle closed inside a `dyn` build is contained and the `dyn`'s event does not fire; a cycle at a switch adoption aborts the switching occurrence with an owner path; the `schedule` check is reached by no test.
4. **`Dynamic::join`, `hold_first`, `map_n`, `tag_map_n`.** `JoinOuterEvent` with its dynamic height; `switch_hold_seeded`; the three law tests and `join_is_prompt`; the n-ary constructors with one test each that a three-operand lift is one node. *Done when:* the seven behavior tests have event twins; the swap-and-change case in both height orders agrees in both halves under the checker; a hold of dynamics plus join builds the cache idiom with no hiccup.
5. **Elements, slots, paths, the element list, request collection, `build` and `run`.** `ElementValue`, `TagId`, `TagTable`, `Path`, `Segment`, `SlotRec`, `element`, the position rule, `PMap` and `PSet`, interned elements and paths, the per-node list with patches on its change event, `Runtime::list`, the root fan, the collector, `Inputs` without `quarantine`, `Outcome`, `RuntimeError`, `Runtime::build` and `Runtime::run`. The test backend is deleted and the harness written at this step, with the counter as the first fixture in the canonical structure. Closes "`WidgetDelegate` exposes live slots", "The test backend reads behaviors directly", "`backend_data` cannot be set" and "`__rebuilt__` as a keep-alive". *Done when:* the eight script tests run as harness scripts reading only the outcome; the counter's transcript matches the conformance chapter's golden byte for byte; routing against the opened list, unknown path as error, a slot created in n addressable from n+1, request firings from a slot torn down in the same occurrence reported with its path, `dyn` generations giving fresh paths, and one call per occurrence each have a test; a benchmark with a thousand slots shows a label rebuild costing time independent of the thousand.
6. **State at the root.** The oracle supply context, root cells, pushes, the `UnsuppliedState` error. *Done when:* a state slot has its value in its creation occurrence from the oracle; a push reaches a slot created in the push's occurrence; two slots of equal value share one cell and cost one oracle call; an unsupplied tag is an error at creation, contained under a `dyn` and fatal in occurrence zero.
7. **`InterceptWidget`.** The fan per interceptor, the supply context per interceptor, `Supplied(n)` positions, the captured list, `Defer` in the harness turn loop (as a seed of the world core, in a function with no harness state in it). *Done when:* an interceptor that forwards everything is observably absent (transcript identical with and without it, and with two); a sandbox supplying `Unavailable` works; a supplied widget's elements appear in the interceptor's list under `s/n/`; two child slots of equal value share one supplied widget regardless of a supplier change between them; a supplied widget lives until the last slot with its value goes; the interceptor's loop over its own child's list builds without a cycle error; pure code firing at a path under an interceptor built in the same occurrence is `UnknownPath`.
8. **The list widget.** `list_widget` with the three prompt consolidations, sharing the deferred-build code with `DynamicWidget`, and the key diff by shared subtrees. Closes "`DWAction` replaces child index 0" and "`dyn` builds twice when its dynamic changes later in the occurrence". *Done when:* each consolidation kind has a test; state survives an insert and a neighbour's removal; a value change updates in place; a key's block is torn down at commit; a key born in n is in all three consolidations in n; `@ Delete on !started` as a row result is reported as a cycle at the membership change.
9. **Quarantine.** `pending_quarantine`, the `quarantine` input, the replacement, the discarded responses. *Done when:* the quarantine scenario passes; a quarantine for a path no error named is refused.
10. **Benchmarks for the costs that are not semantics.** Three programs, each pinned to the cost the performance notes give it: a window of nested boxes with a thousand labels under `Gui.Frames`, one label changing per frame; chapter 5's selectable list with a thousand rows and `$Distinct` on the selection test, one click; a thousand-key map with one `Insert` under `for`. The changes they call for land here, each with no transcript change: shared draw lists with pointer-equality diffs and damage rectangles in the GUI library and the world core, the `$Distinct` warning in the compiler, and native versions of the prelude functions the benchmarks show hot. *Done when:* every transcript is unchanged and each benchmark's cost grows with what changed, not with the thousand.
11. **Small debts.** Every known issue no step above names: `trace`, the `unreachable!()` stubs, the cyclic dynamic sampled after its node is dropped, the `combine`/`leftmost` height caches, `leftmost`'s subscriber lists, the dependents scan, the two source events that record only when subscribed, the copied doc comments, the misnamed file. *Done when:* the list below is empty.

## Known issues

Every loose end in the current code, concretely, with its fix. Most are small; they are listed because a reader executing the plan should not have to rediscover them.

**`trace` leaks its tracer.** `Event::trace` subscribes a logging callback and `mem::forget`s the `Arc`, because subscribers are weak and nothing else would hold it. *Fix:* make `trace` a builder method, `WidgetBuilder::trace(&mut self, event, label)`, which stores the tracer in the node's keep-alive list; or keep the free function behind a `debug-trace` feature with the leak documented as the price of using it outside a node.

**`unreachable!()` stubs in `SubscriptionEvent` implementations.** `EventTriggerInner`, `WidgetReadyEvent` and `DynamicWidgetEvent` are sources, not derived events, yet they implement `SubscriptionEvent` because `SubscriptionManager` requires it, with `Inner = ()` and bodies that panic. *Fix:* split the manager's upstream handling into a separate trait, `Derived<T>`, implemented only by events with an upstream, and give sources a `SubscriptionManager<T, NoSource>` whose connect and notify paths have no upstream to consult. The three stubs disappear and the type system says which events have an upstream.

**Scheduling at `usize::MAX` while at `usize::MAX`.** The commit batch runs at the greatest height, so an action in it that needs to defer to commit has no height above it, and `DynamicWidgetEvent::already_fired`'s `current + 1` overflows there. Nothing in the commit batch builds or subscribes today, so the case is not reached, but it is one new commit action away. *Fix:* the separate commit list of step 2; `defer_commit(entry)` pushes onto it, and the action map never contains the commit height.

**`DWAction` replaces child index 0.** A `dyn`'s node has exactly one child, so the index is right for `dyn`, but the replacement is by position, the delegate is told index 0, and the same code cannot serve a list widget. *Fix:* `WidgetNode::children` becomes `BTreeMap<Segment, Rc<WidgetNode>>` for addressable children plus a `Vec` for plain ones; a `dyn` replaces at `Gen(g)` and a list widget at `Key(k)`, and the element list's prefixes come from the same keys.

**`WidgetDelegate` exposes live slots.** `TestDelegate::for_node` clones `node.public_dynamics` and `node.triggers`, so the backend holds `AnyDynamic` and `AnyEventTrigger` handles outside the runtime for the life of the test, reads behaviors through them with `query_for_tag`, and fires triggers through them by name. A World holding runtime objects is what the boundary forbids, and it is also why the delegate has to be told about every new child. *Fix:* delete `WidgetDelegate`, `WidgetDelegateContext`, `DelegateCallback`, `set_delegate`, `prepare_destruction`, `_public_event_callbacks`, `public_events`, `public_dynamics`, `triggers`, `add_external_event`, `add_public_event`, `add_public_dynamic`, `AnyEvent`, `AnyDynamic`, `AnyEventTrigger` and `backends/test_backend`; the harness over `run` replaces all of it, and the eight script tests are rewritten against the outcome.

**`backend_data` cannot be set.** `WidgetNode::backend_data: serde_json::Value` is initialised to `Null`, no builder method writes it, and the node is behind an `Arc` by the time anyone could. It is the only use of `serde_json`. *Fix:* delete the field and the dependency. Its purpose, telling a backend what a node means, is served by the element list.

**`Arc`/`Mutex` everywhere.** Covered by step 1. The places where the locks do more than lock: the `try_lock` re-entry checks in `FilterMapEvent::height`, `Tag::height`, `SwitchHold::height`, `ForwardEvent::height` and `DependentBehavior::query_for_*`; `TriggerAction` holding and releasing the trigger's `scheduled` lock around notification. *Fix:* `Cell<bool>` re-entry flags; the `scheduled` flag disappears with public triggers.

**The test backend reads behaviors directly.** `TestAction::ExpectDynamic` calls `query_for_tag` on a public dynamic from outside the runtime, a read a World cannot make. *Fix:* the harness reads the outcome; a test that wants a dynamic's value makes the program put it in an element.

**`__rebuilt__` as a keep-alive.** The `DynamicWidgetEvent` is kept alive by registering it as a public event, which also means a backend sees a firing it was not meant to see (the comment notes the delegate attaches after the first one). *Fix:* a private `keep_alive: Vec<Rc<dyn Any>>` on `WidgetNode`, written by the builder.

**Cyclic handles are bare `usize` plus a name.** `add_cyclic_*` returns an index and takes a name; closing by the wrong index with the right type is accepted; two `fold_dyn`s with the same name in one node panic on the duplicate; a wrong-typed close is a runtime panic rather than a compile error. *Fix:* `CyclicEvent<T>` and `CyclicDynamic<T>` handles returned by `add_cyclic_*`, closed by a method that consumes the handle; no names; a `compile_fail` doc test. The compiler, which already names every placeholder by span for diagnostics, keeps the span in the handle, and the span is what the cycle report prints.

**Unclosed placeholders are found at the first propagate.** A cyclic dynamic never closed panics with "cyclic dynamic never closed" the first time it is sampled, far from the widget that forgot. *Fix:* `bind` and `build_root` check every slot in the node is closed before returning, and fail the build with the placeholder's span.

**A cyclic dynamic sampled after its node is dropped panics.** The behavior reaches its slot through a `Weak`; after the node is gone the upgrade fails with "cyclic dynamic sampled after its widget was dropped". A parent that kept the dynamic as a result would hit this. *Fix:* on teardown, before the slot goes, write the dynamic's last value into a cell the computation falls back to, so the dynamic keeps its last value as the specification says.

**`combine` and `leftmost` recompute their height on every call.** Both write a height cache they never read, so every `height()` call walks every source. *Fix:* read the cache; invalidation already reaches them.

**`leftmost` pushes each subscriber onto every manager.** Only the first manager delivers, so the other copies serve only `invalidate_height`, which then tells each subscriber once per source. *Fix:* one subscriber list, on the first manager; the others connect and compete only.

**The dependents list is scanned on every add.** `query_for_behavior` does a `retain` over the whole list before each push to keep it deduplicated. *Fix:* a generation-stamped weak set keyed by the dependent's pointer, which is the `WeakBag` the subscriber list's comment also asks for.

**`Dynamic::new` is `unsafe` for a logical invariant.** *Fix:* `pub(crate)`, backed by the checker.

**Events are not tied to a runtime.** An event built against one runtime can be scheduled in another and nothing notices. *Fix:* the `Rc<Core>` every graph object holds after step 1, which makes the question unaskable.

**`dyn` builds twice when its dynamic changes later in the occurrence.** A `dyn` built mid-occurrence samples the behavior if the change has not happened yet, binds that, and rebuilds by push when the change arrives above it; the end state is right and the first node is replaced before commit, but the first build ran, and any request it fired is reported to the World with a path that is already gone. *Fix:* when `subscribe` returns `None` and the dynamic's change height is above the current height, defer the first build to just above that height, so that it builds once with the value the occurrence ends with. A counter inside the block in the nested-`dyn` test, in both height orders, pins it.

**`dyn` and done events record a firing only if subscribed.** A `dyn`'s output event and a done event fire through a manager that records only when it has subscribers, so a widget built after the firing, in the same occurrence, cannot catch up on it. Nothing today needs to. *Fix:* record unconditionally in those two sources, which costs one clone.

**The `DynamicWidgetEvent` has no height cache and no owner.** Its height is recomputed from the dynamic's event on every call, which is fine, but it has nothing the eager walk can hang an in-progress flag or an owner on. *Fix:* the same `height: Cell<Option<usize>>`, flag and `owner: Weak<WidgetNode>` every other node gets in step 3.

**Copied doc comments.** `Event::combine` and `Event::leftmost` have `filter_map`'s doc, with a "Some(vague)" typo. *Fix:* write them.

**`widget/runtime.rs` is misnamed.** The file holds `PolymorphicWidget` and reads like the crate's runtime module. *Fix:* `widget/polymorphic.rs`.

## Performance notes

What is O(what), where allocation happens, and where a program pays for more than what changed. The numbers are structural; the crate has no benchmarks yet, and the first ones belong with step 10. The runtime's own machinery is proportional to what fired. The places where a GUI pays more than that are the draw list, equal values that still fire, derived functions over maps, comparisons of large keys, and plain compute in closures; each has a paragraph below, and none of the fixes changes what a program means.

**Propagation.** An occurrence costs the sum over fired nodes of their closure plus O(log H) for the `BTreeMap` insertion at scheduling time, where H is the number of distinct heights with actions, plus O(subscribers) per firing to consolidate the list and deliver. Nodes that do not fire cost nothing: a quiet subtree is never visited. A subtree that fires with equal values is not quiet, and the paragraph on equal values says how often that happens. The map is cleared and refilled every occurrence, so its allocation is per height with actions; a `Vec<Box<dyn Action>>` per height and a `Box` per action. A ring of pre-sized vectors indexed by height, reused across occurrences, removes both when it matters.

**Catch-up.** `record_once` costs one `Cell` read, one clone of the value and, today, one boxed clear-at-end closure per fired event per occurrence. The closure is the avoidable part, and the stamp removes it: each manager's cache holds `(u64, Option<Value>)` and a read compares the stamp against the core's counter, so the cache clears itself and the clear-at-end list is gone. Catch-up for a late subscriber is one walk up its sources until a cached value is found, each step constant.

**Heights.** Cached per node after the first computation, invalidated transitively on a retarget with the walk stopping at empty caches. The cost of a retarget is proportional to the number of nodes downstream of it that had computed heights since the last retarget, which is bounded by the subgraph below it and is usually small. The eager walk at an edge creation is bounded the same way and stops at cached heights.

**Behaviors.** Pull with a cache; a query is constant when cached and proportional to the dependency subgraph when not. Invalidation on a hold's commit walks the dependents that have queried since the last invalidation. The retain-on-add scan is O(dependents) per query until the weak set lands.

**Values.** Every value in the graph is an enum of two words plus a tag; heap values are `Arc<dyn HeapValue>` (an `Rc` after step 1) with one allocation per construction and a refcount bump per clone. `Event<T>` and `Dynamic<T>` stored as values pay a second allocation for the outer `Arc`. The one-word representation of the values chapter removes the second indirection and shrinks the enum to a word; nothing in the runtime depends on the current layout, which is behind `PrismValueInner` for exactly this reason.

**Plain compute.** An occurrence-time closure is a boxed Rust closure over two-word `Value`s: every operation matches on the value's tag, every call goes through an `Rc<dyn Fn>`, and every intermediate is a `Value`. For a tight loop (an arithmetic fold, a scan over text) that is roughly an order of magnitude slower than the same loop written in Rust; the figure is an estimate from the structure, not a measurement. Most of a program is not tight loops, but layout, hit-testing and text editing live in Prism, in the GUI library, and run on every frame that changes anything. So the prelude functions that library leans on (traversals of maps and arrays, text slicing and measurement, rectangle arithmetic, sorting) are native Rust in the program's function table, called from compiled closures like any other function, and the library's own hot loops are the first clients of a native backend.

**Routing.** Per delivered response, one `query_for_tag` of the fan's list (constant once cached; the cache is invalidated only when the list commits a change) and one lookup, O(log S) in the number of slots under that fan, plus the output's delivery. A slot that receives nothing costs nothing, and a fan whose responses event is quiet costs nothing: there is no per-occurrence work per interceptor. Depth adds one hop per interceptor an input passes through, which is the price of every level seeing its children's inputs and is per input, not per slot.

**State supply.** One memo lookup per state slot created, O(log V) in the number of distinct element values under the interceptor; one supplied widget per distinct value, however many slots.

**The element list.** Naive: a node's list is rebuilt as the union of its children's on any change below it, which is O(total slots in the subtree) per change and O(depth) such rebuilds per change up to the root, so a label rebuild deep in a tree of N slots costs O(N) per occurrence it changes. Incremental: the patch for a label rebuild is two entries, and applying it at each ancestor is O(log N); the root's map is updated in place behind the `Rc` or cloned on write if a World still holds the previous one. This is the one change in the chapter that turns a per-occurrence cost proportional to the program's size into one proportional to what changed, which is what a GUI with a thousand rows needs.

**Builds.** A `dyn` rebuild costs the block's build plus the journal entry plus the list patch; a failed build costs the same plus the truncation, which is a `Vec::truncate` on four vectors. A supplied widget costs its build once per distinct value. Nothing is allocated for a key that stays or a generation that is not rebuilt.

**Allocation summary.** Per occurrence: the action map's vectors and boxes, one value clone per fired event, the request collector's vector, the outcome's vectors. Per build: nodes, managers, closures, paths (one `Rc<[Segment]>` per slot and per addressable node). Nothing allocates between occurrences, and nothing allocates for a node that does not fire.

## Parallelism

Single-threaded is a decision about the first implementation, not a claim about the model, and the structure the runtime already has is the structure each of three kinds of parallelism needs. None requires a change to a program, and none is needed to make the language exist.

**Fork-join per height.** Within an occurrence, every action at one height is independent of every other at that height: each reads only values below its height, which have already fired or not, and writes only its own node's cache. So the inner loop of `propagate`, "run the actions at this height", can hand them to a pool and join before popping the next height. The one thing that is not independent is a build: a `dyn` rebuild at height h mutates the tree and the journal and subscribes to nodes at many heights, so builds at one height run under a *build lock*, one at a time, while the pure actions at that height run in parallel around them; or, simpler, a height with any build in it runs serially. The variant needs `Sync` graph types, a concurrent subscriber-list consolidation and a concurrent action map: a second runtime compiled from the same source with the pointer and cell types swapped, not a change to this one. The single-threaded variant stays the reference the other is tested against. The win is bounded by the width of the graph at each height, which for a GUI with many independent rows is wide and for a chain of maps is one.

**Optimistic across occurrences.** An occurrence is a transaction with buffered writes and a known read set (every `query_for_tag` and every hold read). Two occurrences whose inputs are addressed to disjoint subtrees can be run speculatively against two copies of the commit list and committed in turn if the second's read set does not intersect the first's write set, else retried. This is ordinary transactional concurrency, and the runtime's rollback path is its first half: a retry is a rollback followed by a rerun. It needs read-set tracking, which is one push per `query_for_tag`, and a World willing to batch inputs into candidate occurrences. It is the variant for a server with many independent clients under one program.

**Sharded runtimes.** Independent subtrees connected only through elements can be separate runtimes, each on its own thread, with a World that routes between them: a server with one runtime per connection, or a GUI with one runtime per window. This needs nothing from the runtime beyond the run call being cheap and the boundary being plain data, both of which the target already is; it is a World design, and a World written in Prism that hosts other programs is the same thing with the routing in Prism.

What none of them changes: the specification's order within an occurrence, which is the height order and is the same under all three; the transcript a program produces, which the differential test checks across variants as it checks across implementations; and the program, which cannot tell.
