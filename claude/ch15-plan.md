# Chapter 15 — Plan

Eleven milestones in two tracks, each one cut so that a model can be handed this book, the code from the milestone before it, and nothing else, and come back with code whose correctness is decided by named tests and matching transcripts rather than by anyone's opinion. The runtime track makes the Rust graph single-threaded and transactional, gives it the boundary, then interception and `for`. The frontend track takes source to a typed AST with a text form, then to a closure compiler. The two meet when the counter compiled from source runs under the Test World with the same transcript as the counter written by hand in Rust. After that the conformance suite, the world core and the CLI World, the Haskell reading, the GUI Worlds, and errors. Each milestone states what it waits on, the work in files and types and functions, what done means, what finishing it proves about the design, and where it is likely to go wrong. The chapter ends with what is deliberately not on the plan and with how to run each milestone with a model when every line will be read by one person.

## How a milestone is cut

You have had the project where the plan was a list of features and the first week went into deciding which feature to start with, because every feature needed a piece of every other. The cure is to cut along the lines the design already has. Prism's design has three hard lines in it: the run call, which is the only thing a World sees; the typed AST, below which nothing decides meaning; and the transcript, which is the only thing a test compares. Every milestone here ends at one of those lines or at a named test, and the plan is ordered so that each line exists before anything needs to cross it.

A milestone has five parts, and a model should be able to find each one without reading the rest of the chapter:

- *Waits on.* The milestones whose code it starts from. Nothing else. There is no step where someone signs off; a milestone is finished when its tests pass, and the next one begins from its code.
- *The work.* Concrete steps: crates, files, types, functions to add or change, in an order that keeps the existing tests green at every step. Where this chapter names a type or a function it is a target, not a constraint; a model that finds a better name should use it and say so.
- *Done.* The tests that pass and the transcripts that match. A test named here is a sentence about the language, and the sentence is its name. A test that cannot be stated as a sentence about the language is testing the implementation's structure, and the chapter on testing across implementations says what happens to those: they are deleted when the structure changes.
- *What it proves.* Which claim of the design the milestone makes true in code. Every refusal in the design (no scheduler, no collector, no locks, no callbacks across the boundary, no sampling) is a claim that something is unnecessary, and the only proof is a working implementation without it.
- *Risks.* Where the work is likely to bend the design to fit the code, and what to watch for.

Sizes are given as a rough count of files touched and tests added, not days. The runtime milestones are each a few hundred to a couple of thousand lines of Rust against a crate of five thousand (from memory); the frontend milestones are larger; the Haskell milestones are smaller than their Rust counterparts because Reflex is most of the runtime.

## The two tracks and where they meet

The runtime track (milestones 1 to 3) and the frontend track (4 and 5) share no code and can start on the same day. The runtime track never needs source: its tests are Rust blueprints written by hand against the builder. The frontend track never needs a runtime: its tests are typed-AST dumps diffed against goldens. They meet at milestone 5, when the closure compiler turns the typed AST of the counter into an `IrWidget` and the Test World drives it through the run call, and the proof that they met is that the transcript is byte-identical to the one the hand-written Rust counter produced in milestone 2.

| Milestone | Track | Waits on |
| --- | --- | --- |
| 1. Runtime: single-threaded and transactional | runtime | nothing |
| 2. Runtime: the boundary | runtime | 1 |
| 3. Runtime: interception and `for` | runtime | 2 |
| 4. Frontend to typed AST | frontend | nothing |
| 5. Closure compiler | both | 2, 4 (its `intercept` and `for` parts: 3) |
| 6. Conformance suite in full | both | 3, 5, and the core half of 7 |
| 7. World core and CLI World | worlds | core: 3; CLI end to end: 5 |
| 8. Haskell frontend | haskell | 4 (the text form) |
| 9. Haskell evaluator and Test World | haskell | 6, 8 |
| 10. GUI Worlds | worlds | 6, 7; the reflex-dom World also 9 |
| 11. Errors | runtime, worlds | 3; slot it anywhere after |

Three things about the table. The world core is listed under 7 but its turn loop is needed by the Test World of 6, so the core half of 7 is handed out as soon as 3 is done and the CLI half waits for the compiler. The Haskell track starts the moment milestone 4 freezes the typed-AST text form, and a second person or model can take it with no knowledge of the Rust code, which is the point of it. Errors is last in the numbering because nothing above needs it, and first to slot in early if a World wants a better message than "the occurrence was aborted".

## Milestone 1: the runtime, single-threaded and transactional

**Waits on** nothing. Starts from `prism_runtime` on branch `on-fire-repros`, 38 tests passing.

The runtime today has locks whose only job is to detect re-entry and an error path that resumes a panic across the boundary. None of that is wrong in the sense of computing the wrong answer; it is wrong in the sense that it makes three design claims untestable: that the runtime needs no locks, that an occurrence is a transaction, and that a cycle is a comparison and not a deadlock. This milestone makes the three claims true, adds the one combinator the boundary cannot do without, and pays the small debts that would otherwise be paid three times.

### The work

*Step 1: `Rc` and `RefCell`.* Mechanical, touches every file, done first and alone so that nothing else is in the diff. `Arc` becomes `Rc`, `Weak` stays `Weak` (the `rc` one), `Mutex` becomes `RefCell` or `Cell`. The `Send + Sync` bounds go from `Blueprint`, `EventImpl`, `BehaviorImpl`, `ValueType`, `Action`, `HeapValue` and the closure bounds on every combinator; `'static` stays. `Runtime` becomes `Runtime { inner: RefCell<RuntimeInner> }` and is `Send + !Sync`: the test `runtime_send` is replaced by one named `a_runtime_can_be_moved_to_a_thread_and_driven_there`, which builds a graph, moves the runtime into `std::thread::spawn`, runs an occurrence, and joins. If `Runtime` ends up `!Send` because something inside holds an `Rc` to a thread-local, that is a bug in this step, not a reason to add `unsafe impl Send`.

*Step 2: re-entry is a cycle, reported.* Every place a lock served as a re-entry guard (`SwitchHold::height` re-locking its own mutex is the known one; the `try_lock` panics "event cycle" are the others) becomes a `Cell<bool>` on the node, set on entry and cleared on exit, and a set flag on entry is a cycle. The runtime's `schedule` keeps its height comparison. Both report through one path: `RuntimeInner::fail(RuntimeError)`, which records the first error of the occurrence; propagation stops at the next action boundary. The test `a_cycle_closed_through_a_cyclic_event_is_reported_not_hung` closes an event in terms of itself through `add_cyclic_event`, runs under a timeout, and expects `Err` with `kind == ErrorKind::Cycle`.

*Step 3: the occurrence as a transaction.* `Runtime::propagate` becomes `Runtime::run_occurrence(&mut self) -> Result<(), RuntimeError>` (the public `run` with `Inputs` and `Outcome` is milestone 2; here the inputs are still triggers scheduled by the test). Inside: open (reset per-occurrence state), propagate by height, commit at `usize::MAX`, clear, close. The error path is a rollback, and a rollback needs three things the code does not keep today: a *commit list* (every hold write, switch retarget and child replacement is already deferred to commit time; collect them in one `Vec<Box<dyn Commit>>` on the runtime instead of as actions at `usize::MAX`), a *build journal* (every node built during the occurrence is pushed to `RuntimeInner::built_this_occurrence: Vec<Rc<Widget>>`, so that on error they are dropped and their parents' child lists restored), and the *fired marks* (the per-event `SubscriptionManager.occurrence` cache is already keyed by occurrence number; bumping the number is the clear). On `Err`, drop the commit list, unwind the journal, bump the occurrence number, return. A panic inside an action is caught with `catch_unwind` and becomes a `RuntimeError` of kind `Component` with the panic's message and the node (a panic in native code a World or library put inside a closure is the `Component` kind); nothing resumes it.

Add `Runtime::check_invariants(&self) -> Result<(), String>` and call it at the end of every test, after every occurrence, including failed ones: no event's cached occurrence value is from the current occurrence number, no hold has an uncommitted pending value, no node's re-entry flag is set, the action map is empty, the build journal is empty. This function is the one piece of test scaffolding that is kept forever; the chapter on the runtime calls it the invariant checked after every occurrence.

*Step 4: `Dynamic::join`.* `impl<T: ValueType> Dynamic<Dynamic<T>> { pub fn join(&self) -> Dynamic<T> }` in `dynamic/mod.rs`, with its own file `dynamic/join.rs`. The behavior half samples the outer's current, then that inner's current. The change event is the leftmost of two: the outer's change, mapped to the new inner's value after this occurrence (its change event's value if it fired in this occurrence, else its current), and a non-prompt switch over the current inner's change event, built with the existing `switch_hold` machinery. Its height is dynamic; reuse the height-invalidation path that `SwitchHold` already has.

*Step 5: small debts, each one a test.* `trace` leaks its tracer (make it a weak subscription). The `unreachable!()` stubs in `SubscriptionEvent` impls (implement or delete the trait method). Scheduling at `usize::MAX` while at `usize::MAX` (commit-time actions that schedule more commit-time actions: with the commit list from step 3 this becomes "append to the list", and the edge disappears). `DWAction` rebuilding at child index 0 (make the child index a field of the action; the general keyed form is milestone 3). `DynamicBlueprint` finding its parent through a non-prompt tag read (a set-once `OnceCell<Weak<Widget>>` on the node, set at build; the test is `a_nested_dyn_rebuilt_in_the_same_occurrence_as_its_parent_finds_its_parent`). `PolymorphicBlueprintAdapter` replaced by a blanket `impl<W: Blueprint> PolymorphicBlueprint<W::Output> for W`, and `widget/runtime.rs` renamed to `widget/polymorphic.rs`. Typed handles for `add_cyclic_dynamic` and `add_cyclic_event`: return `CyclicDynamic<T>` and `CyclicEvent<T>` tokens closed by `close(self, Dynamic<T>)`, so that closing with the wrong type is a compile error; a `compile_fail` doc test pins it.

### Done

All 38 existing tests pass on `Rc`/`RefCell`, with the `Send + Sync` bounds gone. The new tests pass:

- `a_runtime_can_be_moved_to_a_thread_and_driven_there`
- `a_cycle_closed_through_a_cyclic_event_is_reported_not_hung`
- `a_cycle_through_a_switch_is_reported_with_the_nodes_in_it`
- `an_occurrence_that_panics_leaves_the_graph_as_it_was` (build a counter-like graph, run one occurrence, inject a panic in a map closure in the second, expect `Err`, check that the hold's value and every derived dynamic's value equal those after the first occurrence, and that `check_invariants` passes)
- `an_occurrence_that_fails_drops_the_nodes_it_built` (a `dyn` rebuild in the failing occurrence; afterwards the old child is still there and the new one is not)
- `join_follows_the_outer_switch_and_reads_the_new_inner` (outer switches to an inner with current value 3; the joined dynamic changes to 3 in that occurrence)
- `join_follows_the_new_inner_when_both_change_in_one_occurrence` (outer switches and the new inner changes in the same occurrence; the join reads the inner's new value, once)
- `join_ignores_the_old_inner_after_a_switch`
- `join_is_prompt` (a dynamic derived from the join changes in the same occurrence as the join)
- `a_nested_dyn_rebuilt_in_the_same_occurrence_as_its_parent_finds_its_parent`
- `a_closure_runs_once_per_occurrence_even_when_subscribed_late` (the existing test, re-stated in the sentence form)
- `trace_does_not_keep_its_tracer_alive`
- the `compile_fail` doc test on typed cyclic handles

`check_invariants` is called after every occurrence in every test, and passes.

### What it proves

That the runtime needs no locks: a graph driven from one thread, movable between threads, with re-entry detected by a comparison. That an occurrence is a transaction: the `Err` path leaves no trace, which is the whole basis of the error story in the chapter on errors and of the optimistic-concurrency note in the runtime chapter. And that `Dynamic::join` is the only join: `dyn` over dynamics, the cache idiom (a hold of dynamics joined), and the element list under an interceptor all reduce to it.

### Risks

A `RefCell` double borrow is a panic with a worse message than a `Mutex` deadlock is a hang. Every one of them in propagation is a re-entry, and every re-entry is a cycle, so every `borrow_mut` on a node's state during propagation should go through the flag from step 2 and report `Cycle`, never `already borrowed`. Grep the tests for `already borrowed` after the milestone; there should be none.

The rollback is the step most likely to be half done. A model will get the commit list right, because the code already defers writes, and will forget the build journal, because nothing today records what a `dyn` built. The test `an_occurrence_that_fails_drops_the_nodes_it_built` is there to catch exactly that, and it should be written before the rollback is.

`Dynamic::join`'s "new inner's value after this occurrence" is the one place where the behaviour side and the event side can disagree by one occurrence. Two of the join tests pin the two cases; a third worth adding is `join_where_the_outer_switches_to_an_inner_that_does_not_change`, which should emit the inner's current value in the switch occurrence and nothing afterwards.

## Milestone 2: the runtime boundary

**Waits on** milestone 1.

A World, after this milestone, drives the runtime through one call with plain data in and plain data out, and nothing else. The test backend in the crate today reads behaviours directly with `query_for_tag` and receives firings through a `WidgetDelegate` holding live slots; both are ways of reaching into the graph that no World will have, and keeping them would mean the tests test something no World can see. This milestone adds elements, slots, paths, the element list, the request collector, state supply at the root, and the run call, rewrites the counter by hand as a Rust blueprint against the new builder, and replaces the test backend with a harness that is the first World: a few hundred lines of Rust that drive `run` and print a transcript.

### The work

*Step 1: element values.* A new module `element/`: `TagId(u32)` interned through a `Tags` table on the runtime (`intern(&str) -> TagId`, `name(TagId) -> &str`, `namespace(TagId) -> &str`); `ElementValue { tag: TagId, fields: Vec<(FieldId, Value)> }` with structural `Eq` and `Hash`, fields stored in declaration order (`FieldId` is the declaration index) so equality is bitwise over the vector. `Ord` is the language's element ordering: tag name first, then plain fields in declaration order, with the request event excluded as it is from equality. It compares names, never interned `TagId`s, because intern order depends on load order and a sorted map keyed by elements has to iterate the same way in every run and in both implementations. `Value` here is `AnyValue` restricted to plain data: the compiler guarantees it later, and the runtime asserts it in debug builds (`AnyValue::is_plain`, false for anything holding an event, dynamic or blueprint). A `TagKind { requests: Option<TypeId>, responds: Option<TypeId>, state: Option<TypeId> }` registered per tag when interned; the runtime uses only which parts are present, and the `TypeId`s are for the debug assertion at the slot.

*Step 2: paths.* `Path(Vec<Segment>)`, `Segment = Bind(u32) | Key(ElementValue) | Gen(u64)`, `Ord` derived so that tree order is path order. `WidgetBuilder` mints them: `bind` increments the node's bind counter and the child's path is the parent's plus `Bind(n)`; `DynamicBlueprint` adds `Gen(g)` with `g` from a per-node counter that never resets, so a rebuilt slot's path is fresh; `Key` is for milestone 3. `Path` is `Display`ed in the worked-examples chapter's notation (`0/#1/0`, `[k]` for keys) because the transcripts use it.

*Step 3: slots and the element list.* On `Widget`: `slots: Vec<Slot>` where `Slot { path: Path, element: ElementValue, response: Option<EventTrigger<AnyValue>>, state: Option<Dynamic<AnyValue>>, request: Option<Event<AnyValue>> }`. `WidgetBuilder::element(&mut self, e: ElementValue, request: Option<Event<AnyValue>>) -> SlotHandles { response: Option<Event<AnyValue>>, state: Option<Dynamic<AnyValue>> }` creates one, assigning the path and asking the state supplier (step 5) for the dynamic if the tag has state. The node's element list is a derived dynamic `Dynamic<Rc<BTreeMap<Path, ElementValue>>>`, the union of its own slots and its children's lists; rebuild it whole on any child-list change for now, with a comment that an incremental version is an optimisation and must not change a transcript. The root node's list is what the run call returns.

*Step 4: routing and the request collector.* At open, the runtime snapshots the root list's current value into `RuntimeInner::routing: BTreeMap<Path, EventTrigger<AnyValue>>` (the response triggers of every slot in the list as the occurrence opened). A response to a path not in the snapshot is a `RuntimeError` of kind `UnknownPath` naming the path, and aborts the occurrence before any trigger fires. Every slot with a request event subscribes it, at creation, to a per-occurrence collector on the runtime: `RuntimeInner::requests: Vec<(Path, AnyValue)>`, in firing order. A slot torn down in the occurrence still has its firings collected, with its path; the World discards them if it wants.

*Step 5: state supply at the root.* `RuntimeInner::root_supplier: BTreeMap<ElementValue, (Dynamic<AnyValue>, EventTrigger<AnyValue>)>`. When a slot with state is created and no interceptor encloses it (milestone 3 adds the enclosing case), the runtime looks the element value up; on a miss it calls the oracle (the run call's `&dyn Fn(&ElementValue) -> AnyValue`), builds a hold from the answer whose change event is a fresh external trigger, and memoises. Pushes at the start of an occurrence fire the trigger for each pushed value. The hold's value in the creation occurrence is the oracle's answer, which is the snapshot as the occurrence opened; the push arrives in the same occurrence and commits at its end, so a slot created in n ends n holding the pushed value exactly as an older slot does. An entry is dropped when the last slot with that value is torn down (a count on the entry, decremented at teardown).

*Step 6: the run call.* `Runtime::run(&mut self, inputs: Inputs<'_>) -> Result<Outcome, RuntimeError>` with

```
pub struct Inputs<'a> {
    pub responses: Vec<(Path, Value)>,
    pub pushes: Vec<(ElementValue, Value)>,
    pub oracle: &'a dyn Fn(&ElementValue) -> Value,
}
pub struct Outcome {
    pub list: Vec<(Path, ElementValue)>,
    pub requests: Vec<(Path, Value)>,
    pub result: Value,
}
```

Inside: open (routing snapshot, reset the collector), fire the pushes and then the responses at height 0, propagate, commit, read the root list and the result, close. The first call builds the root: `Runtime::build_root<W: Blueprint>(&mut self, w: &W)` runs occurrence zero with no inputs; make it the same code path as `run` with empty inputs, so there is one occurrence loop and not two.

*Step 7: the counter by hand, and the harness.* In `prism_runtime/src/harness/`: `Harness { runtime, snapshot: BTreeMap<ElementValue, AnyValue>, last_list }` with `turn(&mut self, responses, pushes) -> Transcript` and `set_state(&mut self, e, v)`; a `Transcript` with a `Display` that prints the list one slot per line as `path: Tag(field=value, ..)`, then `requested path: value` lines, then `result: value`, in the format the conformance chapter freezes. The harness is not yet the world core: it runs one occurrence per turn and has no `Defer`, which arrives in milestone 3. Then `harness/counter.rs`: `Button`, `Label` and `Counter` as Rust `Blueprint`s against the new builder, `Button` creating a `Ui.Button` slot inside a `dynamic_widget` over its label dynamic and switching its click events, `Label` the same with no response, `Counter` the five lines of the Prism counter in builder calls. This is the fixture milestone 5 has to match, so write it to look like the compiler's output would: one node per bind, a `dyn` per lifted widget application, a cyclic dynamic for the hold.

*Step 8: delete.* `backends/`, `WidgetDelegate`, `WidgetDelegateContext`, `backend_data`, `add_public_event`, `add_public_dynamic`, the public-slot views on `Widget`, `query_for_tag`, the `__rebuilt__` event, `schedule_trigger` as a public function. `Slots` becomes an `IndexMap` or goes. The tests that used the backend move to the harness, with their names re-stated as sentences.

### Done

Every milestone 1 test still passes. The harness drives the hand-written counter and prints this, byte for byte:

```
turn 0
  0/#0/0: Ui.Button(label="Increment")
  1/#0/0: Ui.Button(label="Decrement")
  2/#0/0: Ui.Label(text="Count: 0")
  result: unit
turn 1  responses: {0/#0/0: click}
  0/#0/0: Ui.Button(label="Increment")
  1/#0/0: Ui.Button(label="Decrement")
  2/#1/0: Ui.Label(text="Count: 1")
  result: unit
turn 2  responses: {0/#0/0: click, 1/#0/0: click}
  0/#0/0: Ui.Button(label="Increment")
  1/#0/0: Ui.Button(label="Decrement")
  2/#2/0: Ui.Label(text="Count: 0")
  result: unit
```

The transcript is saved as `harness/goldens/counter.txt` and the test is a diff. And these pass:

- `the_first_run_builds_main_and_returns_the_initial_list`
- `a_response_is_delivered_to_the_slot_at_that_path_as_the_occurrence_opened` (the relabelling button of the worked examples: label changes in n, click in n lands on the old label's slot)
- `a_slot_created_in_an_occurrence_is_addressable_from_the_next`
- `a_response_to_an_unknown_path_is_an_error_and_the_occurrence_does_not_happen`
- `a_slot_removed_in_an_occurrence_still_receives_input_in_it_and_is_gone_in_the_next`
- `request_firings_are_reported_per_slot_in_firing_order` (one slot firing three times across three turns; two slots firing in one occurrence, reported in list order)
- `a_request_fired_by_a_slot_torn_down_in_the_same_occurrence_is_reported_with_its_path`
- `a_state_slot_has_the_oracle_value_in_its_creation_occurrence`
- `a_push_reaches_a_slot_created_in_the_push_occurrence`
- `two_slots_with_equal_elements_share_one_state_and_one_oracle_call` (count oracle calls)
- `a_rebuilt_slot_gets_a_fresh_path` (`dyn` rebuilt twice; three distinct generations)
- `rebuilding_one_sibling_does_not_change_the_others_paths`
- `one_run_call_is_one_occurrence` (run five turns, count occurrences: five)
- `nothing_crosses_the_boundary_but_plain_data` (a compile-time test: `Inputs` and `Outcome` contain no type from `event/`, `dynamic/` or `widget/`; written as a `static_assertions` or a doc comment and a grep in CI)

### What it proves

That a World needs nothing from the runtime but the run call. The harness is a World, and it is the smallest one there will ever be: it holds a snapshot map and a last list, and it never sees an event. Every later World (Test, CLI, GUI, the Haskell ones) is this harness with components, and anything one of them needs beyond `Inputs` and `Outcome` is a design error to be fixed in the design, not an API to add.

It also proves the routing rule and the diode in code: the snapshot at open is the only routing table, and nothing in the runtime reads the list during an occurrence except to build the next one.

### Risks

Paths. A model will be tempted to make a path a pointer or an index into the node tree, because that is cheaper. It has to be a value, ordered in tree order, never reused, because the World keys its bookkeeping by it across occurrences in which the tree changes under it. The test on rebuilding one sibling pins the never-reused part; add `paths_are_in_tree_order` over the list of a two-level tree.

The element list as a derived dynamic over children. The list's change event must fire in the occurrence a child is built, which is the construction rule applied to the list, and a naive implementation that rebuilds the list at commit time gives a list one occurrence stale. The counter transcript catches this (the label's new generation appears in the same turn), but only because the label rebuilds; a program with no rebuild would not notice.

The harness's transcript format will be frozen in milestone 6 and the Haskell implementation will print it too. Keep the printer in one small file with no dependence on the harness so it can move to the world core.

## Milestone 3: interception and `for`

**Waits on** milestone 2.

With the boundary in place the runtime can be driven from outside but not from inside: there is no way for Prism code to stand between a child and the World. This milestone adds the `InterceptWidget`, state supply through an interceptor's supplier, the keyed list widget, and the one World feature the runtime's own tests need to exercise interception, which is `Defer` in the harness turn loop. It is the milestone that makes the claim "a sandbox is ordinary code" testable.

### The work

*Step 1: `InterceptWidget`.* In `widget/intercept.rs`: `InterceptWidget<W: Blueprint> { child: W, responses: Event<Rc<BTreeMap<Path, AnyValue>>>, supplier: Dynamic<SupplierFn> }` where `SupplierFn = Rc<dyn Fn(&ElementValue) -> Rc<dyn PolymorphicBlueprint<Dynamic<AnyValue>>>>`, with `Output = (W::Output, Dynamic<Rc<BTreeMap<Path, ElementValue>>>)`. Its `build` builds the child as a child node marked *captured*: the child's element list does not join the parent's. The captured list is the second half of the result. At open, the runtime snapshots routing tables per interceptor as well as at the root; the interceptor's `responses` event, when it fires, delivers each entry to the child slot at that path in the interceptor's snapshot, and an entry for an unknown path is `UnknownPath` with the interceptor's node named. Delivery is at the responses event's height plus one, which is why routing through an interceptor is a delayed edge and the self-referential `!down` of the rewriting interceptor in the elements chapter is legal.

*Step 2: state supply through the supplier.* `WidgetBuilder::element` for a state tag now asks the nearest enclosing captured node first and the root supplier only if there is none. The interceptor node holds `supplied: BTreeMap<ElementValue, (Rc<Widget>, Dynamic<AnyValue>, count)>`. On a miss: read the supplier dynamic's current value, apply it to the element, build the returned blueprint as a child of the interceptor's node right then, in the same build pass, and memoise its result dynamic. The supplied widget's own slots go into the interceptor's node, so they appear in the interceptor's list and not in the captured child list. The last teardown of a slot with that value drops the entry and the supplied node. The supplier dynamic's current value is read at the instant of creation and never again for that slot: a changed policy affects slots created after the change.

*Step 3: the keyed list widget.* In `widget/keyed.rs`: `Dynamic<Rc<BTreeMap<K, V>>>::keyed_widget(f: impl Fn(&K, Dynamic<V>) -> W) -> impl Blueprint<Output = ...>` where the output is chosen by a `Consolidate` parameter the compiler states: `Dynamic<Rc<BTreeMap<K, O>>>` for dynamic results, `Event<Rc<BTreeMap<K, O>>>` for event results (the entries that fired, in one occurrence), and `()` for unit. One child node per key, path segment `Key(k as ElementValue)`, with a per-key `Dynamic<V>` projected from the map (a `map` over the map dynamic filtered to the key, with the change event dropped when the value is unchanged, so a change to a neighbour does not reach a child). Keys that appear in occurrence n build in n, deferred to `max(height, current + 1)` like a `dyn` rebuild, and their first results are in the consolidated output in n: the consolidation is prompt, which means the consolidated dynamic or event is rebuilt at the point in n after the new children exist. Keys that disappear are torn down at commit. `DynamicBlueprint` is generalised at the same time: its rebuild replaces a named child rather than child index 0, and `keyed_widget` and `dynamic_widget` share the deferred-build code.

*Step 4: `Defer` in the harness.* The harness grows a turn loop: run the occurrence; for every entry in `requests` whose slot's tag is `Defer`, record the path; if any, run another occurrence with `Click`-like unit responses to those paths and no pushes; repeat. A path answered twice in one turn is an error of kind `DeferBackwards` naming the path. This is the seed of the world core and should be written to lift out: a function `turn(runtime, inputs, &mut defer_state) -> Vec<Outcome>` with no harness state in it.

*Step 5: three hand-written interceptors.* In `harness/interceptors.rs`, as Rust blueprints: `Forward` (a keyed list over the captured list, each entry re-creating the element with the same request event and returning its response; supplier forwards by creating the same element), `Sandbox` (holds back every `Fs.*`, answers each request with `Denied` through a `Defer`, supplies `Unavailable` for every state tag), and `Rewrite` (forwards with a path prefix on `Fs.*` elements). These are fixtures for milestone 5 as the counter was for milestone 2: the compiled `@Forward` from the GUI library must match the hand-written one's transcript.

### Done

All earlier tests pass. The new tests:

- `an_interceptor_that_forwards_everything_is_observably_the_identity` (the counter under `Forward`, and under `Forward` twice; all three transcripts identical)
- `nothing_the_child_creates_reaches_the_outside_unless_the_interceptor_sends_it` (a child with two slots under an interceptor that re-emits one; the root list has one)
- `a_response_through_an_interceptor_is_routed_against_the_childs_list_as_the_occurrence_opened`
- `a_response_to_a_path_unknown_to_the_interceptor_is_an_error_naming_the_interceptor`
- `a_sandbox_supplies_unavailable_and_the_child_has_it_in_its_creation_occurrence`
- `a_sandbox_denies_a_request_in_a_later_occurrence_of_the_same_turn` (the probe of the worked examples: the turn runs two occurrences, the label reads "(permission denied)" at the end of the build turn, and the request never appears at the root)
- `a_supplied_widgets_own_elements_appear_in_the_interceptors_list`
- `two_child_slots_of_equal_value_share_one_supplied_widget` (count builds)
- `a_supplied_widget_lives_until_the_last_slot_with_its_value_is_torn_down`
- `a_slot_keeps_the_supplier_it_was_created_under_when_the_supplier_changes`
- `a_state_tag_the_root_cannot_answer_is_an_error_in_the_creation_occurrence` (the harness's oracle has no answer for the tag; a `RuntimeError` of kind `UnsuppliedState` with the tag and path in its report)
- `for_builds_one_child_per_key`
- `for_keeps_a_childs_state_across_an_insert_and_a_neighbours_removal` (each child holds a count; click one, insert before it, remove after it, click again: 2)
- `for_consolidates_a_new_keys_result_in_the_creation_occurrence_and_the_new_child_does_not_see_itself`
- `for_consolidates_events_to_the_entries_that_fired`
- `for_consolidates_dynamics_to_a_map_that_is_prompt`
- `a_child_under_for_does_not_rebuild_when_a_neighbour_changes`
- `defer_is_answered_in_a_later_occurrence_of_the_same_turn`
- `a_turn_with_two_defers_runs_three_occurrences`
- `deferring_backwards_is_an_error`
- `a_cycle_through_an_interceptors_responses_that_is_prompt_is_reported` (an interceptor whose responses depend promptly on its child's list; expect `Cycle`)
- `routing_by_a_sampled_layout_is_not_a_cycle` (a VBox-style interceptor that tags a click with the sampled list; expect no error)

### What it proves

That interception is complete: forwarding, holding back, rewriting, denying and supplying are all written against the two downward supplies and `@Element` upward, with no mode of the primitive for any of them. That state supply by building the supplier widget is the mechanism and needs no chain to walk. That the turn, with `Defer`, is a World concept the runtime never sees: the runtime ran one occurrence per call throughout. And that `for` is a widget like any other, with identity from the data's keys.

### Risks

The supplied widget's node is a child of the interceptor's node but was built in the middle of building a grandchild. The build journal from milestone 1 must record it under the right parent, and teardown must not drop it when the grandchild goes (the count does that). A test that fails the occurrence while a supplied widget is being built is worth adding: `a_failed_occurrence_drops_a_supplied_widget_built_in_it`.

Prompt consolidation in `for` means a derived output that is rebuilt after the children are, within the occurrence, at a height above every child's result. A model will get this right for events and wrong for dynamics, or the other way around; both tests are listed.

`Defer` does not belong in the runtime and the harness must not teach it anything: if the runtime ends up with a `Defer`-specific branch, the diode has been broken. The test `one_run_call_is_one_occurrence` from milestone 2 still holds here.

## Milestone 4: frontend to typed AST

**Waits on** nothing. A new crate, `prism_lang`, with no dependency on `prism_runtime`.

This milestone has no runtime in it, and a model given only the language chapters and the chapter on the Rust language implementation can do all of it. Its output is a program with every meaning decided: every lift explicit, every number typed, every binding marked build-time or occurrence-time, every group placed, every tag named. The deliverable that matters most is the text form of that tree, because it is what the goldens diff, what a reader reads to see what the compiler decided, and what the Haskell frontend must reproduce byte for byte.

### The work

*Step 1: lexer and layout.* `lex.rs` and `layout.rs`: the token set of the lexical chapter (sigils, `<-`, `then`, `|` as merge distinct from the bars of `\|a, b|`, the suffixed operators `+~ +.` and friends, `==>`, `..`, `;`, `ignore`, the comparison operators `< <= > >=` beside `==` and `!=`, the boolean keywords `not`, `and`, `or`, `if` (which guards a `match` arm as well as opening a conditional), and text literals with `{expr}` interpolation: a text literal lexes as literal runs and `{` expression `}` pieces, `\{` and `\}` are literal braces, and `$` inside text is always literal), indent and dedent tokens by the offside rule, `:` at end of line opening a block, and the spacing rule that tells `F -1` from `F - 1` and a prefix `.x` or `[i]` (the pronoun's field or index, wherever `_` is in scope) from `a.x` and `a[i]`. Every token has a `Span { file, start, end }`. The layout corpus is the first test and is a golden: a program in, its token stream out, one token per line.

*Step 2: parser.* `parse.rs` and `ast.rs`: paren-less calls with comma-separated arguments, single parentheses as grouping (a group headed by a call is that call with all its commas, so `(G b, c)` is `G(b, c)`), double parentheses for tuples, tuple types and tuple patterns (`((a, b))`), double brackets for array literals (`[[1, 2, 3]]`, `[[]]`) with single `[i]` always indexing, function types `fn A, B -> C` with parentheses only around a function-typed argument or result, method chains by indentation (a `.Method` that starts a continuation line closes the call above it), `;` between the statements of an inline block, the precedence table (loosest to tightest: `on` and `|`, `or`, `and`, `not`, comparison, arithmetic, so `not a == b` is `not (a == b)`), guarded `match` arms (`pattern if cond:`), `for` with named binders (`for k, $item <- $items:`, the shorthand `for $items:` binding `key` and `$_`), array patterns (`[[]]`, `[[head, ..rest]]`, `[[a, b]]`), the `{expr}` pieces of a text literal parsed as expressions, sigiled names, blocks (`widget`, `dyn`, `for`, `match`, `intercept`), patterns including element patterns with `..` and `Ns.*(..)`, `tag T of Element:` declarations with `requests`/`responds`/`state`, `type` declarations, lambdas (`\|e| body` and the bare `\body` with `_`), and `test` blocks (parsed now, used in milestone 6). Every node has a span from the first commit.

*Step 3: resolver.* `resolve.rs`: scopes, the pronoun and its sigiled forms, tag namespaces, the prelude, and binding groups: dependency analysis over each widget block into strongly connected components in dependency order. The resolver labels each dependency edge *prompt* or *delayed* (a read through `on`, a hold's own name on the right of `<=`, a `dyn`'s event, `!Switch`), which the cycle check needs.

*Step 4: elaborator.* `elab/`: Hindley–Milner on the value layer with the three built-in traits (equality, ordering, numeric) resolved by inference and the suffixed spellings when it cannot; the functor layer read off the sigils with the lifting rule (dynamics and no event: dynamic; dynamics and one event: event, dynamics read through tags; two events: error); `?` lifting at the plain layer; text interpolation lifting like application (a literal that interpolates a dynamic is a dynamic, and one that interpolates an event is an event); `match` on `?` as its eliminator; guards, with a guarded arm not counting toward exhaustiveness and array patterns counting toward it; application lifting, including a dynamic of blueprints lifting to `dyn` with `!Switch` or hold-plus-`$Join` by the result kind; `for` consolidation by result kind; tag field types from declarations; the build-time or occurrence-time mark on every binding; the cycle check over the groups with their edge labels; exhaustiveness for closed choices and the catch-all requirement for extensible ones; the tag-indexed refinement of `Slot.!request` and supplier results inside a `match` arm. The output is `typed.rs`: the typed AST.

*Step 5: the text form.* `typed/text.rs`: one binding per line, `name: type [mark] group = expr @span` (the sigil is part of the type), with every lift an explicit node (`dmap`, `map`, `tag`, `hold`, `hold_first`, `leftmost`, `switch`, `join`, `dyn`, `for` with its can-fire-at-birth flag, `element`, `intercept`, `placeholder`, `close`), printed and parsed from the same definition (a derive or a single table), with a round-trip test on every fixture. Inferred type variables are named in order of first appearance, left to right, top to bottom, so that two frontends agree. `prism dump foo.prism` prints it.

*Step 6: goldens and diagnostics.* `tests/goldens/`: the counter, each program of the test corpus, `lib/gui.prism`, and one small file per rule of the lexical and expression chapters, including one each for a guarded arm (the guard printed on the arm), a `for` with named binders, an array pattern match, and `not a == b` against `(not a) == b`. `tests/negative/`: a rejected program and its exact error with spans, for each of: two events in one operation (names both, points at `|` and `!Leftmost`); a rejected prompt cycle in a widget block (every binding in it, in order, with spans); a cycle through `dyn` alone (pins that `dyn` is not a delay); `a -1` on a `Nat` ("did you mean `a - 1`?"); a `Nat` and an `Int` in one operation, and `+~` on floats ("use `+.`"); a generic function putting its type parameter into an element field; a `match` on an extensible variant with no catch-all; a sigil disagreeing with the inferred kind; a state tag with no supplier at an interceptor whose supplier is a visible lambda. The first four are the ones whose wording the implementation chapter fixes; the rest need the span and the names right, not the prose.

### Done

The layout corpus passes. `prism dump` of the counter is this, character for character, with `@Button` and `@Label` defined above it as in the language implementation chapter:

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
    !increment <- @Button $"Increment"
    !decrement <- @Button $"Decrement"
    let !delta = -1 on !decrement | +1 on !increment
    $count <- 0 then $count + !delta
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

The conventions are the text-form section's: sigils inside the type, `[build]` or `[occ]` on every binding, the group number shared by a placeholder, its `.hold` definition and its `close`, application printed with parentheses and commas, literals with their type (`0:Int`), `{expr}` interpolation printed as `cat`, and the span of each binding's start.

Every golden and negative golden is in and passes, the guard, binder, array-pattern and `not` precedence goldens among them; the printer and parser round-trip every fixture; the four diagnostics read as the implementation chapter specifies. The crate has no dependency on `prism_runtime`.

### What it proves

That the sigils remove inference over functors: HM runs on the value layer and never sees time, and the lifting rule is a lookup on operand sigils. That every decision about meaning can be made before anything is built, and written down in a form a person can read. And that the specification is precise enough to produce an identical tree from a second reading, which milestone 8 tests.

### Risks

The text form will change during the milestone and that is fine; it must stop changing before milestone 8 starts, so the last step of this milestone is to go through every golden once more and declare the form frozen in the file that defines it. After that, a change to the form is a change to both frontends.

The cycle check needs the resolver's edge labels to be right, and the labelling of `dyn`'s event (prompt: it fires in the rebuild occurrence) and `!Switch` (delayed) is where a model will guess. The negative golden for a cycle through `dyn` alone is there for that.

The dynamic-of-blueprints lift is the one place the elaborator inserts a `dyn` the programmer did not write. Its golden should show `switch (dyn ...)` for an event-returning widget and `join (hold_first (dyn ...))` for a dynamic-returning one, and the second form is easy to get wrong (the hold's initial value is the first build's result, which the `dyn`'s event provides in occurrence zero).

## Milestone 5: the closure compiler

**Waits on** milestones 2 and 4. Its `intercept` and `for` parts wait on 3, and the counter does not need them.

This is where the tracks meet. The closure compiler walks the converted typed AST once and produces an `IrWidget`, a Rust value implementing the runtime's `PolymorphicBlueprint`, whose `build` walks a tree of pre-resolved steps in group order calling the runtime's constructors. There is no instruction set and no second syntax; the lint is the verifier, run on one form. Done is a single transcript: the counter compiled from source matches the counter written by hand in milestone 2.

### The work

*Step 1: closure conversion.* `convert.rs` over the typed AST: each lambda becomes a function id and an explicit capture list of environment indices; recursive functions refer to each other by id and never hold each other; each cyclic group's placeholder and close are placed where the resolver's order says. The output is the typed AST annotated, printed by the same text form with the captures shown.

*Step 2: lint.* `lint.rs`: every node's type agrees with its children's; every placeholder has exactly one close; nothing samples a placeholder before its close; every capture names a free variable of the lambda; build order respects the groups; every occurrence-time expression is reachable only from a graph node's closure; no build-time binding reads an occurrence-time one. Run after every conversion and in every test. It has its own negative fixtures: hand-edited converted trees that it must reject.

*Step 3: `IrWidget` and the program.* `ir/`: `Program { functions, blocks, tags, constants }`, `IrWidget { program, block, captures, span }` implementing `PolymorphicBlueprint<AnyValue>`. Plain-value expressions compile to closures over an environment `Vec<AnyValue>`; occurrence-time expressions compile to the closure the corresponding graph node calls. One module, `boxing.rs`, is the only place a Rust `i64`, `f64`, `bool`, `Text` or record becomes an `AnyValue` or comes back, so the representation can change behind it later.

*Step 4: the steps.* One `Step` per typed-AST node kind, each one a call into the builder: `map`, `tag`, `hold`, `leftmost`, `switch_hold`, `join`, `dynamic_widget` over a dynamic of `IrWidget`s, `keyed_widget`, `element` with the tag's kind, `intercept`, `placeholder`/`close` with the typed cyclic handles, `bind` of a nested `IrWidget`, and `started` as a `dyn` over a constant. Generic code is erased: a step whose type is a type variable moves the one-word value.

*Step 5: the driver.* `prism run foo.prism --world harness` builds the root `IrWidget`, hands it to the milestone 2 harness, and prints the transcript for a sequence of inputs given on the command line or in a small file. The language-level script language is milestone 6; this driver is enough to diff the counter.

### Done

Lint passes on every golden's conversion. The compiled counter, driven by the harness with the same three turns as milestone 2, prints a transcript byte-identical to `harness/goldens/counter.txt`. The compiled `@Forward` from `lib/gui.prism` wrapped around the counter matches the hand-written `Forward` fixture's transcript, once milestone 3 is in. And:

- `a_compiled_hold_reads_its_own_old_value_through_a_tag`
- `a_compiled_dyn_fires_its_event_in_the_build_occurrence_and_every_rebuild`
- `a_compiled_switch_is_non_prompt` (the creation-occurrence firing of a `dyn` child is not seen through `!Switch`)
- `a_compiled_closure_captures_by_environment_index_and_not_by_rc` (a widget whose occurrence-time closure reads a sibling dynamic; drop the node; the dynamic's node is freed — a `Weak` upgrade fails)
- `compiled_and_hand_written_widgets_sit_in_one_tree` (a Rust `Button` bound inside a compiled widget and vice versa)
- `lint_rejects_a_placeholder_closed_twice`, `lint_rejects_a_sample_before_close`, `lint_rejects_a_capture_of_a_bound_name`

### What it proves

That the thing executed is construction: a Prism program compiles to code that, run once, calls constructors in the right order with the right captures, and the graph does the rest. That the typed AST is the only intermediate form the language needs today, because the lint checks it and the compiler walks it. And that the two tracks were built against the same reading of the design, which the byte-identical transcript shows better than any review could.

### Risks

Heap cycles through closures. The runtime has no collector because nodes own their closures and closures capture through the owning node's environment, not by `Rc` to a sibling. A compiler that captures a `Dynamic` directly into a closure that the dynamic's own hold subscribes to has made a cycle the runtime will never free. The leak test is listed; run it under a leak checker in CI if one is handy, and in any case assert `Rc::strong_count` where the test can.

The build-time versus occurrence-time split. A model will compile a plain expression that mentions a dynamic as a build-time read of the dynamic's current value, which is sampling, which the language does not have. The typed AST's marks make this a lint error if the lint checks reachability (step 2's last two rules); make sure it does.

Matching the hand-written counter means the hand-written counter has to have the structure of compiler output, which is why milestone 2 said to write it that way. If the transcripts differ only in generation numbers or bind positions, the hand-written one is at fault as often as the compiler.

## Milestone 6: the conformance suite in full

**Waits on** milestones 3 and 5, and the core half of 7.

Everything so far was tested from Rust. This milestone makes the specification testable from Prism: the script language, the Test World as a real World over the world core, the law tests, the programs, and the GUI library written in Prism and run under the Test World. When it is done, a sentence in the language chapters either has a test or is a sentence someone should look at.

### The work

*Step 1: the script language.* `test name, @widget:` forms parsed by `prism_lang` (the parser from milestone 4 already accepts them), one line per turn with the inputs `build`, `idle`, `click`, `send`, `with`, `push` and the checks `expect`, `expect_top`, `expect_none`, `expect_count`, `expect_top_none`, `returns`, `fired`, `quiet`, `requested`, `unrequested`, `errors`, `settles` on `=>` lines. Patterns are element patterns: a tag with named fields and `..`, or `Ns.*(..)`; a value in a pattern is compared structurally. `prism test foo.prism` runs every test block in the file.

*Step 2: the Test World.* In the worlds crate from milestone 7's core half: `TestWorld` is the world core plus one component, `Script`, that accepts every tag. Its oracle answers from the `State` steps. Tags declare no defaults, so a state element with no `State` step is an `UnsuppliedState` error at the slot's creation: a script gives a `State` step before the build for every state tag the program creates at build, which a `State` step may do; its responses are the `Send`s; it records request firings, results and errors for `Requested`, `Fired`, `Quiet` and `Errors`. Depth: `Expect` and `Send` reach into containers by the GUI library's convention (a `children` field holding the keyed element list; a `ChildInput(path, response)` response to route inward), and the Test World knows the convention because the library is part of the suite.

*Step 3: the transcript.* The printer moves from the harness to the world core, prints a turn as its occurrences (each with its inputs, list, request firings and result, and an error if any), and the format is frozen in `transcript.md` beside it with one example per feature. `prism test --transcript` prints it; the suite stores one per test and diffs. Nothing after this milestone changes the format without changing the file and every stored transcript.

*Step 4: the law tests.* One short Prism program and script per sentence the conformance chapter lists: leftmost merge; the tag law; a hold's initial value and adoption in the build occurrence; derived dynamics prompt and at most once; `dyn` firing in its build occurrence and every rebuild; `!Switch` non-prompt including the swallowed creation-occurrence firing; `$Join` prompt on both changes; the mid-occurrence construction rule including a nested `dyn` over a dynamic that changed in the same occurrence building once with the new value; `for` consolidation per result kind, state kept across an insert and a neighbour's removal, a new key's result seen in the creation occurrence; the creation rule and `@started`; routing against the opened list; unknown path is an error; a state element's creation-occurrence value, a push reaching a new slot, two equal slots agreeing; the forwarding identity; the sandbox supplying `Unavailable`; a supplied widget's elements in the interceptor's list; two equal child slots sharing one supplied widget; `Defer` later in the turn, backwards is an error, two `Defer`s are three occurrences. Each file is named after its sentence.

*Step 5: the programs.* The twelve integration programs brought to the current syntax (numbers by inference, `tag` declarations for the `Ui` set, `Click` as a unit type, the `intercept` signature with its three arguments) and three more: `@started` kicking a request answered in a later turn; a `dyn` child whose creation-occurrence output is swallowed at the boundary; a `for` whose parent sees a new key's output in the creation occurrence. Each with its transcript stored.

*Step 6: the GUI library in Prism.* `lib/gui.prism` as the suite's library: the tag declarations

```prism
tag Ui.Label of Element:
    text: Text

tag Ui.Button of Element:
    label: Text
    responds: Click

tag Ui.VBox of Element:
    children: Path ==> Element
    responds: ChildInput
```

and `@Button`, `@Label`, `@VBox`, `@Forward`, `@HBox`, with `@VBox` as the interceptor of the elements chapter: capture the children's list, re-emit the `Ui.*` ones inside one `Ui.VBox` element, forward the rest unchanged, route `ChildInput(path, response)` inward. This is the library the GUI Worlds of milestone 10 run, and it is written now, under a World with no screen, so that layout and routing are tested before any pixel exists.

### Done

`prism test` passes on every law test, every program and the library's own tests, under the Rust implementation, and every transcript is stored and frozen. The counter test from the worked examples chapter, including `counter_left_wins` and the sandbox script with its corrected expectations, passes as written there.

### What it proves

That the specification is the thing: a test names elements and values, never a path or a node, and passes under any World that delivers the same inputs. That GUI is a library: `@VBox` is an interceptor and `@Button` is `@Element` plus a filter, and both run under a World that cannot draw. And that the transcript is enough to compare two implementations, which milestone 9 does.

### Risks

Patterns in scripts need matching on element values with structural comparison of fields, including nested element values in a `children` field. This is a small interpreter and a model will put it in the Test World; it belongs in `prism_lang` beside the element-pattern code the compiler already has, so that the Haskell side has one less thing to re-derive (it reads the same script and must match the same way).

The depth convention is the library's and the Test World must not grow a second one. If a test needs to reach into something that is not a `children` field, the library grows a convention and the Test World follows it.

Bringing the twelve programs to current syntax will surface sentences in the language chapters that two readers would read differently. Each one is a fix to the book first and to the program second.

## Milestone 7: the world core and the CLI World

**Waits on** milestone 3 for the core; the CLI World's end-to-end tests wait on 5.

A World is small because the language cannot do anything to it: it receives plain data and sends plain data back, never registers a callback, never holds a runtime object. Everything a World has to get right is the short list of obligations in the Worlds chapter, and that list is the same for every World, so it is written once. The CLI World is the first World that runs Prism against a real operating system and the smallest one that is not a test.

### The work: the core

A crate `prism_world`:

- `World<C: Component>`: holds the runtime, the snapshot (`BTreeMap<ElementValue, AnyValue>`), the last list received, and the component. `turn(&mut self, inputs: TurnInputs) -> TurnOutcome` runs the occurrence, checks for fired `Defer` requests, runs the follow-up occurrences, enforces once-per-slot-per-turn and the deferring-backwards error, and returns every occurrence's outcome. This is the harness's turn loop from milestone 3 lifted out, and the harness is rewritten to use it.
- Snapshot bookkeeping: `set_state(e, v)` records into the next snapshot; before each occurrence the oracle closure answers from the previous snapshot and the pushes are the diff; the swap happens after the occurrence.
- Slot tracking: a response for a path not in the last list is discarded with a diagnostic before the runtime sees it (the runtime would error; the World's obligation is to not send it).
- `Unsupported`: for a request on a tag no component accepts, a response of `Unsupported` in the next turn; for a presence tag, an error report; for a state tag, the unsupplied-state error at creation, surfaced from the runtime.
- The `Component` trait: `tags(&self) -> &[TagId]`; `perform(&mut self, path, element, request: AnyValue)` for a request firing; `poll(&mut self) -> Vec<Response>` for completions since last asked; `answer(&self, element) -> Option<AnyValue>` for the oracle; `changes(&mut self) -> Vec<(ElementValue, AnyValue)>` for pushes; `created(path, element)` and `removed(path, element)` for presence and stream elements. The core calls components; components never see the runtime.
- Error reporting: one `Report` type with the error kind, spans, the path, the tag and the interceptor chain, and one `Display` every World prints.

Tests, with a fake component: `a_turn_is_the_occurrences_before_the_world_acts`; `defer_is_answered_once_per_turn_and_backwards_is_an_error`; `the_oracle_answers_from_the_previous_snapshot_and_the_pushes_are_the_diff`; `a_slot_created_in_the_push_occurrence_ends_it_holding_the_pushed_value`; `a_late_response_for_a_gone_slot_is_discarded_with_a_diagnostic`; `an_unsupported_request_tag_gets_unsupported_in_the_next_turn`; `an_unsupported_presence_tag_is_reported_and_ignored`; `request_firings_are_performed_in_occurrence_order_then_list_order`.

### The work: the CLI World

A crate `prism_cli` with a binary `prism`:

- The loop: a `std::sync::mpsc` queue of inputs; the main thread blocks on it, runs one turn per input, performs the turn's requests, and repeats. Reader threads (stdin, signals, file IO, timers, sockets) push inputs; `tokio` is not needed for this and should not be pulled in until a component needs it.
- Components, with the kinds the language fixes and the names from the Worlds chapter: `Process.Args` and `Process.Env` (state, constant after start); `Cli.Stdin` (responds with `Chunk Text` then `Eof`; read only while a slot exists); `Cli.Stdout` and `Cli.Stderr` (requests, written after the turn in occurrence order then list order, then flushed; `WriteFailed` as a response); `Process.Exit` (`requests: !Code`; ends the run after the turn's writes); `Cli.Signals` (responds; with no slot, OS defaults); `Fs.File` (requests, responds), `Fs.Contents` (state, stat-and-reread, or a watch where one exists; the oracle reads synchronously up to a size cap and answers `Unavailable(TooLarge)` beyond it, and `Unavailable(reason)` takes its reason from the closed `FsError` choice: `NotFound`, `Denied`, `TooLarge`, `Other(text)`); `Time.After` (requests, responds), `Time.Every` (responds), `Clock` (state, pushed once per turn at most); `Net.Socket` (requests `!Bytes`, responds `Received Bytes | Closed`, state `Connecting | Connected | Disconnected Reason`), which milestone 10's X11 proof needs; `Defer` from the core.
- Exit: on `Process.Exit`'s request, after the turn; and with code 0 when nothing can cause another occurrence (no stdin slot or its `Eof` delivered, no request outstanding, no stream slot that can fire); subscriptions to args and env do not keep a program alive. This is a consequence of the graph having no sources left, and the test for it is a program that prints and falls off the end.
- Golden transcripts: program, args, env and stdin in; stdout, stderr and exit code out, run against the real binary: the line counter on empty input, one line, many lines, lines split across chunks; falling off the end; a computed nonzero exit code; two `Cli.Stdout` slots firing in one occurrence and stdout interleaved with stderr across turns; args and env; a file read with `@started` before stdin is built inside a `dyn`; a `Ui.Button` at the root getting `Unsupported`; SIGINT with and without a `Cli.Signals` slot. Timers are faked through the Test World, so no golden depends on the clock.

### Done

The core's tests pass with the fake component; the harness runs on the core; the Test World of milestone 6 runs on the core. The line counter from the worked examples chapter passes its golden transcripts against the real OS, and so does every transcript above.

### What it proves

That a World is an afternoon's work once the core exists: the CLI World is components and a queue. That the `Defer` rule bounds a turn without a cap. That the run call's "plain data in both directions" survives contact with threads and a real OS, because the reader threads produce values and the main thread performs requests, and nothing else crosses.

### Risks

Performing requests after the turn, in occurrence order then list order, is the obligation a model will perform during the turn instead, because the firing is right there. The ordering test with two stdout slots catches the list-order half; add one where a request's response would arrive in the same turn if performed early (a `Defer` answered after a write) to catch the timing half.

Exiting when nothing can cause another occurrence needs the core to know which slots can fire. A component declares, per tag, whether a slot of that tag is a live source; the core sums. Getting this wrong hangs a program or exits it early, and both goldens (falling off the end; a program with a `Time.Every` that must not exit) are listed.

## Milestone 8: the Haskell frontend

**Waits on** milestone 4 freezing the typed-AST text form. Nothing else; a second person or model can take it with the language chapters, the Haskell chapter, and the frozen text form.

A specification with one implementation is a description of that implementation. The Haskell frontend reads the language chapters a second time, independently, and the test of whether the chapters are a specification is a `diff` on the dumps.

### The work

A cabal project `prism-hs`, with modules `Prism.Lex`, `Prism.Layout`, `Prism.Parse` (hand-written or megaparsec; the precedence table is small; guarded `match` arms, `for` binders and array patterns are parser work), `Prism.Resolve` (groups and edge labels), `Prism.Elab` (HM on the value layer with the three traits; the sigil lifting rule; marks; the cycle check; exhaustiveness), `Prism.Typed` (the tree), `Prism.Typed.Text` (printer and parser from one definition, with the same type-variable naming rule), and an executable `prism-hs dump`. Its tests: the layout corpus; every golden from milestone 4 and 6 as a diff between `prism dump` and `prism-hs dump`; the negative goldens, compared on the error kind and spans, with the prose allowed to differ.

A script `tools/diff-frontends.sh` that runs both dumps over every `.prism` file in the suite and reports differences, run in CI.

### Done

`diff` on the dumps of every program in the suite is empty. Every negative golden is rejected with the same kind and spans.

### What it proves

That the language chapters are a specification. Every divergence found on the way is either a bug in one frontend or a sentence two careful readers read differently, and the second kind goes back into the book; the milestone is as much a review of the language chapters as it is code.

### Risks

Hindley–Milner has choices in it that the text form exposes: where let-generalisation happens, how a type variable is named, whether a monomorphic group's variables are generalised after the group. The language implementation chapter has to state each one, and if a divergence traces to one it does not state, the fix is a sentence in that chapter and then both frontends.

A model writing this with the Rust frontend's source in hand will reproduce its bugs. Do not hand it the Rust source. Hand it the book and the goldens.

## Milestone 9: the Haskell evaluator and Test World

**Waits on** milestones 6 and 8.

The Haskell runtime is Reflex plus a thin layer that gives it slots and the run call. The typed AST is interpreted, not compiled, in a monad over Spider; most of the semantics of the functor and time chapters are inherited, and the work is the boundary and the three places Reflex offers the wrong thing next to the right one.

### The work

- `Prism.Eval`: `PrismM t` as `ReaderT Env (DynamicWriterT t [Slot] (PerformEventT ... (SpiderHost)))` (the transformer names from memory) or the equivalent, with the correspondence table from the Haskell chapter as the implementation: `!A` is `Event t A`; `$A` is `Dynamic t A` with `current` used only where the typed AST says `tag`; `|` is `leftmost`; `v on !e` is `tag (current v) e`; hold is `holdDyn`; `dyn` is `dyn (f <$> x)`; `!Switch` is `switchHold never`; `$Join` is `join`; `for` is `listWithKey`; `@started` is a `dyn` over a constant and never `getPostBuild`; the element list is the writer; `intercept` runs the child's writer and passes the supplier and the outer `tell` down in the reader; state supply runs the supplied widget in the outer context at slot creation, memoised per interceptor by element value; the run call is `runSpiderHost` with `fireEventsAndRead` over trigger refs from `newEventWithTriggerRef` (both names from memory).
- `Prism.World`: the world core in Haskell, a few hundred lines: turn loop, `Defer`, snapshot, slot tracking, `Unsupported`, the transcript printer in the frozen format.
- `Prism.World.Test`: the Test World over the script language, parsed by `Prism.Parse`.
- `prism-hs test foo.prism`, and `tools/diff-transcripts.sh` over the whole suite.

A review checklist, applied by reading the evaluator once it exists: no `current` outside the three places the Haskell chapter names (the `tag` node, routing against the opened list, the supplier read); no `switchPromptly` or `switchPromptlyDyn`, and no `coincidence` outside `for`'s birth term; no `getPostBuild`; no `performEvent` that does anything but collect request firings; no `sample` in a build that the typed AST did not mark.

### Done

The conformance suite passes under `prism-hs test`, and `diff` on transcripts between the two implementations is empty for every law test and program.

### What it proves

That the semantics are Reflex's where they overlap, and that the boundary (slots, paths, the run call, state supply, `Defer`) can be put on Reflex without changing it. Also that two runtimes with different internals (a height-ordered action map; Spider's subscriber graph) produce the same transcripts, which is the strongest evidence available that the transcripts are determined by the language chapters and not by an implementation.

### Risks

Reflex's `dyn` and `networkView` fire their event in the rebuild frame and Spider's `subscribeAndRead` gives the mid-occurrence construction rule for free, but `listWithKey`'s consolidation of a new key's result in its creation frame is a thing to check, not assume: the law test for it will say.

The evaluator is an interpreter over a tree, and a model will reach for `current` whenever it needs a value at build time. Everything at build time is a build-time binding in the typed AST, and those are plain values; a build-time read of a dynamic does not exist. The checklist is the defence.

## Milestone 10: GUI Worlds

**Waits on** milestones 6 and 7; the reflex-dom World also waits on 9.

GUI is not a language concept and not a World concept. Some Worlds know how to put pixels on a screen and read a pointer and a keyboard, and offer tags for that; layout, hit-testing, focus, text editing, buttons and lists are Prism interceptors over those tags and run unchanged under any World that offers them. This milestone makes that claim true three times over, with three Worlds that share one Prism library, and the third one has no host GUI code in it at all.

### The vocabulary

Five tags, declared in `lib/gui.prism` beside the widgets of milestone 6. A window, the picture in it, the name of the focused region, a way to measure text, and a tick per frame.

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

type Frame = record:
    now: Instant

tag Gui.Frames of Element:
    responds: Frame                # a GUI World fires this before it paints
```

The draw commands are values in the scene's `draw` list, in paint order, and never tags:

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

`Gui.Metrics` is state because a layout needs the width of a string in the occurrence it lays out. The World does no geometry and no hit-testing: the window responds with every pointer event over it, the library knows what is under the pointer, and focus is by name, a `Text` hold in the library with at most one `Gui.Focusable` emitted at a time, so a World has one place to send keys, the window, and the `Gui.Focusable` is only for the platform's focus ring, input method candidate window and screen reader.

The library over these has its own tags under `Ui` (`Ui.Label`, `Ui.Button` responding `Click`, `Ui.TextInput`, `Ui.VBox`, and so on), which never reach a World: its interceptors claim them. `@Window` is the interceptor at the top. It captures the children's `Ui.*` elements, lays them out using `Gui.Metrics` readings, emits one `Gui.Window` and one `Gui.Scene` holding the whole draw list, hit-tests the window's `Pointer` responses against the layout it holds (read through a tag as the occurrence opened), and routes each hit inward as a `Click`, and each `Pressed` key to the input whose name is the focus. Focus is a hold of the focused name moved by the Tab key and by clicks, and only the focused child's `Gui.Focusable` goes up. `@VBox` and `@HBox` assign positions; `@TextInput` keeps its text and cursor as holds outside any `dyn`, draws `Draw.Text` runs and a caret `Draw.Rect` placed from a `Gui.Metrics` reading, and receives keystrokes through the library's routing by focus name, from the window's key responses. `@Button` and `@Label` are written over `Draw.Rect` and `Draw.Text`; the milestone 6 versions remain as the Test World's simpler vocabulary. The library's tests run under the Test World first, which sends `Pointer` and `Pressed` responses to the `Gui.Window` slot: a click at a point lands on the right button; Tab moves focus by name; typing moves the cursor; a resize re-lays out.

### The software rasteriser World

A crate `prism_gui_soft`: `winit` for the window and input, `softbuffer` for the pixels, a small rasteriser for rectangles and clipped text with a bitmap or `fontdue` font for `Gui.Metrics`. Components: `Gui.Window` (opens the window, responds with `Resized`, `CloseRequested` and every pointer event as `Pointer` and every key event from `winit` as `Pressed`, a turn per event), `Gui.Scene` (after each turn the draw list is rasterised into the buffer in list order and presented), `Gui.Focusable` (presence; the World notes it to show a focus ring and place the input method's candidate window), `Gui.Frames` (a turn per frame while a slot exists), `Gui.Metrics` (the font's advances, synchronously), plus the CLI World's general components. Which drawing backend is used has no language consequence; `softbuffer` is chosen because it has no GPU dependency and its output can be screenshotted into a golden PNG in a test (from memory).

### The reflex-dom World

In `prism-hs`: the same five tags rendered to the DOM. `Gui.Scene` becomes absolutely positioned `div`s and `span`s, one per `Draw.*` entry, with `Draw.Clip` as a `div` with `overflow: hidden`; `Gui.Window` listens for pointer, key and resize events; `Gui.Focusable` becomes a focusable element that gives the browser a focus ring and a screen reader a name, its key events bubbling to the window listener; `Gui.Metrics` is answered from a hidden `canvas` measurement; a turn per DOM event. This is the cheap way to see Prism programs on a screen, and it is the Haskell implementation doing something the Rust one cannot yet.

### The X11-over-sockets proof

A Prism program, under the CLI World, with no GUI World at all. `lib/x11.prism` is an interceptor `@X11` over the GUI vocabulary: it opens a `Net.Socket` to the X server, speaks the X11 wire protocol (connection setup, `CreateWindow`, `MapWindow`, `PolyFillRectangle`, `ImageText8`, event reads for `ButtonPress`, `KeyPress`, `Expose`, `ConfigureNotify`; request and event names from memory), turns its children's `Gui.Scene` draw lists into request bytes on the socket's `!request` event, and turns `Received` bytes into `Pointer`, `Pressed` and `Resized` responses for the `Gui.Window` slot. It answers its children's `Gui.Metrics` itself through the supplier, with a fixed-width font, so measurement is pure. The counter runs on an X display with the CLI World as the only host code, and the CLI World knows nothing about windows.

### Done

The counter and a text input, from the same `lib/gui.prism`, run on a screen under each of the three, and the library's layout, hit-testing and focus tests pass under the Test World. For the software rasteriser: a golden PNG of the counter after two clicks, compared pixel for pixel. For reflex-dom: a headless-browser test that clicks the button and reads the label. For X11: run under `Xvfb`, take a screenshot with `xwd` (both from memory), compare to the golden; and the transcript of the `Net.Socket` requests for the first three turns is stored and diffed, which pins the protocol bytes without a display.

### What it proves

That one GUI vocabulary serves many Worlds, that layout and focus and text editing are Prism, and, with the X11 interceptor, that "World" and "GUI" are different axes: a World with sockets and nothing else can put pixels on a screen because a Prism program spoke the protocol. The X11 proof is the one that answers the question a sceptic asks first, whether the primitives are really low enough that the toolkit is in the language.

### Risks

Text. Measurement by a state element is right, but a World whose fonts change (a font loaded later) pushes new metrics for the elements it has answered, and the layout must handle the push by re-laying out, which it does by being a derived dynamic over the measurements. Write the layout that way from the start and the risk is gone.

Hit-testing is the library's and reads its layout as the occurrence opened. A click delivered in the occurrence after a resize lands on the old layout; that is the design (the tag law, applied to the layout), and a test under the Test World pins it so that nobody moves hit-testing into a World.

The X11 interceptor is a few thousand lines of Prism and will find holes in the plain-value library (bytes, integer encoding, slicing). Those are prelude work, not language work, and the milestone should say which prelude functions it added.

## Milestone 11: errors

**Waits on** milestone 3. It can be slotted anywhere after; it is last here because nothing above needs it, and the Test World's `Errors` step is the only thing in the suite that reads it.

The transaction from milestone 1 already makes an occurrence abortable. This milestone decides what the World is told, contains a failed `dyn` build, and gives the Test World its `Errors` step.

### The work

- `RuntimeError` gains the full report: the kind, the closed `ErrorKind` (`Cycle`, `Overflow`, `UnknownPath`, `UnsuppliedState`, `DeferBackwards`, `Component`, `Build`), the spans of every binding involved (for a cycle, the whole cycle in order, with the Prism spans the compiler attached to each cyclic slot), the path, the tag and the interceptor chain for a protocol error, and which occurrence of which turn. The compiler names every cyclic placeholder after its source span so that a runtime cycle reports Prism source.
- Containment at the build boundary: a failed `dyn` build does not fire and is reported through `Errors`. A cycle or native failure found while a `dyn` or `for` builds a block in the middle of an occurrence abandons that build, not the occurrence. The block's construction already has a boundary (the deferred build from milestone 3); wrap it so that an error inside drops the half-built node, leaves the slot holding an `Error` presence element in the block's place, does not fire the `dyn`'s event, and records a `RuntimeError` of kind `Build` in `Outcome.errors`, which the world core hands to its `Errors` element. The rest of the occurrence continues. The `Error` tag is declared in the prelude.
- World reporting: the world core's `Report` prints the above; the CLI World prints to stderr and exits nonzero; the Test World records it for `Errors pattern`.
- Overflow in `Nat` and `Int` arithmetic aborts the occurrence with kind `Overflow`; `Checked.*` (returning `?`) and `Wrap.*` (wrapping) are prelude functions bound to Rust's `checked_*` and `wrapping_*`; recursion past the closure compiler's depth guard is also `Overflow`; the transaction is the backstop for native panics, which are kind `Component`.
- Quarantine of the owning subtree (replace the lowest common owning widget's node with an `Error` element in the next occurrence) as a World option on the core, off by default, with one test showing it on.

### Done

- `a_cycle_at_runtime_aborts_the_occurrence_and_reports_every_binding_in_it_with_prism_spans`
- `an_aborted_occurrence_leaves_the_program_as_it_was_and_the_world_is_told`
- `a_cycle_found_while_a_dyn_rebuilds_does_not_fire_the_dyn_and_is_reported_through_errors`
- `a_failed_dyn_build_leaves_an_error_element_in_the_list`
- `the_rest_of_the_occurrence_continues_after_a_failed_dyn_build`
- `a_protocol_error_names_the_path_the_tag_and_the_interceptor_chain`
- `overflow_aborts_the_occurrence_with_kind_overflow`
- `checked_functions_return_nothing_on_overflow_and_wrap_functions_wrap`
- `quarantine_replaces_the_owning_subtree_with_an_error_element_when_the_world_asks_for_it`
- the `Errors` step passes in the Test World on each of the above written as Prism tests.

### What it proves

That the transaction was worth having: the error path is cheap, coherent and silent to the program, and the one smaller unit (a failed construction) falls out of the `dyn` boundary that already exists. And that errors a program is meant to handle are values and nothing else: there is no `catch`, and an interceptor that wants to handle a child's failure handles an `Error` element like anything else a child emits.

### Risks

Whether aborting an occurrence opened by a `Defer` answer aborts the rest of the turn is a World decision; the core should abort the turn and report, because a half-turn is a state no World was written to act on, and a World that wants otherwise can say so. If that reasoning turns out wrong, the change is in the core and not the runtime.

## What is not on the plan

Each exclusion buys something, and the thing it buys is named so that the exclusion can be revisited when the price changes.

*Values representation.* `AnyValue` stays an enum over a boxed trait object and a few inline integers until a profile says otherwise. What this buys: milestones 2 through 10 never wait on a representation decision, and `boxing.rs` from milestone 5 is the one place to change. The urgent subset when the profile arrives is inline `i64` and `f64` and text as one allocation. Constraint: nothing here may change a transcript, and the suite is the test.

*Register code and native backends.* The closure compiler is the only backend. What this buys: one intermediate form, one lint, no verifier between two forms, and no instruction set to keep in step with the runtime's constructors. When something has to ship compiled, register code is a lowering from the converted typed AST, checked against the closure compiler by the differential test, and the target (Cranelift or emitted Rust) is picked then. There is no LLVM.

*`&` references, `read` and `update` blocks.* Chapter 16's, with its own constraints (an `update` block is an expression with a value; no reference escapes it; nothing about occurrences is involved). What this buys: no interaction between references and erasure to design before the language exists. It belongs in the Haskell implementation first, where lens libraries make the experiment cheap.

*User-defined traits and modules beyond one file plus the prelude.* Ordinary language work with no bearing on time. What this buys: the elaborator of milestone 4 is HM plus three built-in traits, which a model can write in one pass.

*Parallelism.* Single-threaded is a decision about the first implementation. The runtime's structure allows fork-join per height (a second runtime variant with `Sync` graph types), optimistic occurrences (the rollback path of milestone 1 is half of it), and sharded runtimes under one World (no runtime support needed). None is needed to make the language exist.

*A World that wraps a native toolkit.* A different World with higher tags (`Gtk.Button`). What the exclusion buys: the three GUI Worlds of milestone 10 share one library and prove the vocabulary; a toolkit World proves nothing about the language and is a library question.

*Incremental compilation, a language server, a package format.* Each is wanted and none changes the typed AST, which is the line that makes them later work rather than now work.

*A scheduler, dirty tracking, re-entrancy guards beyond the one flag, deferred effects, an unsubscribe story, a collector, locks.* Not deferred: absent by design, and each milestone's "what it proves" is partly the proof that the absence holds.

## Working with a model on a milestone

The author will execute this plan with a model writing most of the code and will read every line. That changes what a milestone brief should contain and what a review should look for, and the plan was cut with it in mind.

### What to hand it

For every milestone: this chapter's section for the milestone, verbatim; the chapters of this book the milestone cites (the runtime milestones cite the time, widgets, elements and runtime chapters; the frontend milestones cite the lexical, expression, functor and language-implementation chapters; the Worlds milestones cite the elements, Worlds and conformance chapters; the Haskell milestones cite the Haskell chapter and the language chapters); the worked-examples chapter, always, because its walkthroughs are the transcripts the milestone has to reproduce; and the code from the milestone it waits on, as a checkout, with its tests passing. Nothing else: no design notes, no chat history, no other implementation's source. For milestone 8 in particular, not the Rust frontend.

Hand the whole set at once. A model that gets the runtime chapter without the elements chapter will invent a boundary.

### What to ask for, in order

Ask for four things, as four replies, and read each before asking for the next.

First, a file plan: every file to be added or changed, with one line each on what goes in it, and the list of tests it intends to write with their names as sentences. This is a page and it is where most of the review's value is: a wrong decomposition is visible here and invisible in two thousand lines of code. Push back on any file that does not map to a step in the milestone, any test name that is not a sentence about the language, and any type in a public signature that the design says does not cross a boundary.

Second, the tests, before the implementation, as failing tests. For a runtime milestone these are Rust tests against the builder; for a frontend milestone they are goldens and negative goldens, with the golden text written by hand from the book (the model can write the expected dump of the counter from the typed-AST section before the elaborator exists, and that hand-written golden is the check on the elaborator). For a Worlds milestone they are transcripts written by hand from the worked-examples chapter. A test written after the code tests the code; a test written before it tests the book.

Third, the implementation, in the step order the milestone gives, with the suite green after each step. Ask for the steps as separate commits so that each can be read on its own and reverted on its own. A step that touches every file (milestone 1's first step; milestone 2's deletion step) is one commit with nothing else in it.

Fourth, a list of the sentences in the book the model found ambiguous, wrong, or missing while doing the work, each with the choice it made. This is the milestone's second product, and it goes back into the book before the next milestone is handed out, because the next model reads the book and not the previous model's choices.

### How to check its work

Run the suite, and then read, in this order: the public signatures (do `Inputs` and `Outcome` hold only plain data; does the `Component` trait see the runtime; does the evaluator's `PrismM` expose `current`); the invariants (is `check_invariants` called after every occurrence in every test; is every `borrow_mut` during propagation behind the re-entry flag; does the lint run on every conversion); the tests (is each one a sentence; would it fail if the sentence were false; is there a test that tests structure rather than language, to be deleted); and only then the code.

Each milestone has a short list of things a model will do because they are easier and that the design forbids, and the review should grep for them:

- Milestone 1: `Arc`, `Mutex`, `unsafe impl Send`, `resume_unwind`, `already borrowed` in any test output.
- Milestone 2: any public function on `Runtime` other than `run`, `build_root` and `check_invariants`; any `Event` or `Dynamic` in a type the harness uses; a path that is an index or a pointer.
- Milestone 3: a `Defer` branch in the runtime; a supplier table or tag table in `InterceptWidget`; a consolidated `for` output that fires one occurrence late.
- Milestone 4: a dependency on `prism_runtime`; any inference over sigils; a `dyn` edge labelled delayed; a text form with two printers.
- Milestone 5: an `Rc<Dynamic>` captured into a closure; a build-time read of a dynamic's value; an instruction enum.
- Milestone 6: pattern matching in the Test World rather than in `prism_lang`; a second depth convention; a transcript printed two ways.
- Milestone 7: a request performed during a turn; `tokio` before a component needs it; a component holding a runtime object.
- Milestone 8: the Rust frontend's source in the model's context; a text form that differs in type-variable names.
- Milestone 9: `current` outside its three named places; `switchPromptly`; a `coincidence` outside `for`'s birth term; `getPostBuild`; `performEvent` doing work.
- Milestone 10: layout, hit-testing or focus in a World; a text input with a dynamic field; host code in the X11 proof.
- Milestone 11: a `catch`; an error the program can observe other than as an `Error` element.

Then run the differential checks the milestone enables: the counter transcripts (2 against 5), the forwarding fixtures (3 against 5), the dumps (4 against 8), the transcripts (6 against 9), the library under three Worlds (10). A milestone whose differential check passes has been reviewed by the other track, which is a review no reader can do alone.

### When the model is stuck

A model stuck on a milestone is usually stuck on a sentence, and the fix is to find the sentence. Ask it to quote the part of the book it is implementing and say what it thinks the sentence means; the gap is usually visible in the paraphrase. If the sentence is wrong or missing, fix the book, then re-hand the milestone; do not patch the code around a sentence the book does not have, because the Haskell reading will not have the patch.

A model that proposes a change to the design in order to finish a milestone (a mode on `intercept`, a callback in the run call, a sampled read at build time, a `Defer` the runtime understands) is proposing the thing a milestone exists to prove unnecessary. The answer is the hypothetical-alternative paragraph in the relevant chapter, and if that paragraph does not exist, writing it is the work.

### The order to hand things out

Day one: milestones 1 and 4, to two sessions, because they share nothing. Then 2 and, when 4's text form is frozen, 8 to a third session. Then 3 and the core half of 7. Then 5, which is the first one whose done criterion is a diff between two sessions' work. Then 6 and the CLI half of 7 together, then 9, then 10 and 11 in whichever order a screen or an error message is wanted first. Every milestone after the first two begins with a passing suite from the one before it and ends with a larger passing suite, and nothing in between waits on anything but tests.
