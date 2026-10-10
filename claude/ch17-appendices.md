# Chapter 17: Appendices

Four reference tables, each for a reader who already knows the design and wants one page to check against. Appendix A sets Prism beside Reflex construct by construct, because Reflex is the direct ancestor and almost every choice in Part II is a choice to keep something of Reflex's or to remove it; the notes under the table name the Reflex construct, say what Prism does instead in one worked pair, and point at the chapter that owns the argument. Appendix B lists the prior art, one entry per system, with what each contributed and which claims are quoted from a source. Appendix C is the syntax on one page. Appendix D is the element vocabulary: every tag the design uses anywhere, with its fields, its parts, its identity, and one line of meaning, so that the names stay the same from chapter to chapter and from the specification to the Worlds that implement it.

## A. The Reflex correspondence

You have written the Reflex program where a `Dynamic` was sampled with `current` in one place and read through `updated` in another, and the two disagreed by one frame, and you spent an afternoon finding out which was right. Most of what Prism changes about Reflex is on that afternoon's path. The table gives the mapping; the notes after it take the rows where Prism and Reflex differ, show the two spellings side by side, and name the chapter that argues the choice.

| Prism | Reflex | In one phrase |
| --- | --- | --- |
| `!A` | `Event t a` | the same thing |
| `$A` | `Dynamic t a` | behavior half hidden |
| (no spelling) | `Behavior t a`, `current`, `sample` | removed; only the tag law reads *before* |
| `@A` | `m a` for the widget monad; `PrismM t a` in the Haskell implementation | the same thing, minus the transformer stack |
| widget block, `<-`, `@ v` | `do`, `<-`, `return` | the same thing |
| `?A` | `Maybe a` | a sigil, lifted at the plain layer |
| lifting by sigils | `fmap`, `<$>`, `zipDynWith`, `ffor` | inferred from spelling, not written |
| `v on !e` (dynamic `v`) | `tag (current v) e`, `attachWith` | the one window onto *before* |
| `v on !e` (plain `v`) | `e $> v`, `v <$ e` | the same thing |
| `a \| b`, `!Leftmost [[..]]` | `leftmost [a, b]` | the same thing |
| `!never` | `never` | the same thing |
| `$x <- v <= !e` | `holdDyn v e` | a statement, so it has an owner |
| `!Updated $x` | `updated x` | a prelude function; never fires at build |
| `$Distinct $x` | `holdUniqDyn` | the only deduplication; holds never dedupe |
| `match $x:` with plain arms | `ffor x $ \case ..` | lifted `match` |
| `match !e:` with plain arms | `fforMaybe`, `push` | lifted `match`; `!Filter` is `fmapMaybe` |
| `!Switch !!ee` | `switchHold never ee` | the only switch; non-prompt |
| (none) | `switchHoldPromptly`, `switchPromptlyDyn`, `coincidence` | removed |
| `$Join $$dd` | `join` on `Dynamic` | the same thing, prompt |
| `dyn $x: ..` | `dyn`, `networkView` | fires in every build occurrence incl. the first |
| hold of `!$B` + `$Join` | `widgetHold`, `networkHold` | the cache idiom; no primitive |
| (none; `dyn` covers it) | `runWithReplace`, `Adjustable` | the class under `dyn` |
| `for $m:` | `listWithKey`, `listHoldWithKey` | keyed, prompt consolidation (the Haskell implementation's one `coincidence`) |
| `for $array:` | `simpleList` | index-keyed, discouraged in both |
| `@started` | `getPostBuild` | a `dyn` over a constant; same occurrence everywhere |
| recursive bindings in a block | `rec` / `mdo`, `MonadFix` | implicit; P8 checks the cycle |
| prompt-cycle error | `EventLoopException` "causality loop detected" | the same failure, reported with spans |
| request element (`requests` + `responds`) | `Requester`, `requesting` | plain data; routed by path |
| state element (`state`) | `MonadQuery`, `queryDyn` | has a value in its first occurrence |
| the element list `$(Path ==> Slot)` | `DynamicWriterT`, `tellDyn` | an ordinary dynamic |
| `intercept` | `runDynamicWriterT` + `runRequesterT` + `runQueryT`, stacked | one primitive, two directions |
| routing responses by path | `fan`, `select`, `fanMap` | the runtime's job |
| the run call | `runSpiderHost`, `fireEventsAndRead`, `newEventWithTriggerRef` | plain data in, plain data out |
| (none) | `performEvent`, `performEventAsync` | removed; the World performs |
| (none) | `newTriggerEvent` | removed; only the World starts an occurrence |
| the occurrence | a Spider frame | the same thing, named as a transaction |
| mid-occurrence construction rule | Spider's `subscribeAndRead` | the same behavior, as a law |
| `Defer` element | (improvised per application) | one element, turn semantics |
| (none) | `Incremental`, `Patch` | removed; a `for` over a map is the patch. A patch type would return if `for` over a large map proved too slow to diff by key |

### The behavior half is hidden

Reflex exposes `current`, `updated` and `sample` on every `Dynamic`; Prism's `$` has no `current`, and the only way to turn a dynamic into a plain value is `dyn`, which rebuilds on change. Chapter 4 ("No sampling, and what it buys", "The hidden behavior") owns the argument; `current` survives in exactly one place, the Haskell implementation's evaluator, where a typed-AST tag node is `tag (current d) e`.

The spelling that costs the afternoon, and what replaces it:

```haskell
-- Reflex: a build-time read that looks like a dynamic and is a constant
label0 <- sample (current label)
el "span" $ text label0
```

```prism
# Prism: the same intent is a dyn; the label is live, and a change rebuilds
dyn $label:
    @Label _
```

### Lifting replaces `fmap`, `zipDynWith` and friends

Reflex writes each lifting rule as a combinator (`fmap`, `zipDynWith`, `tag`, `attachWith`, `ffor`, `<*>`); Prism reads the rule off the operands' sigils at every application, and the type-class constraints on a Reflex signature collapse into the `@` on a function's name. Chapter 3 owns lifting; this gives up functions polymorphic over whether their argument is a dynamic, and the rule that a plain `let` lambda lifts afresh at each application gives most of that back (`let F = \|x| x + 1` serves `F 3`, `F $n` and `F !e`).

```haskell
-- Reflex: three things said, one of them the tag law
count' <- holdDyn 0 $ attachWith (+) (current count) delta
```

```prism
# Prism: the tag law is the reading of a dynamic at an event
$count <- 0 <= $count + !delta
```

### Only one switch, and it is not prompt

Reflex has `switchHold`, `switchHoldPromptly`, `switch` and `coincidence`, with its own documentation recommending the non-prompt form unless you are absolutely sure; Prism's `!Switch` is `switchHold never` and there is no other. Chapter 5 ("`!Switch`, `$Join`, and why they differ") owns the argument, whose core is the element boundary: a block rebuilt in occurrence n cannot have any event addressed to it in n, so a prompt switch could only ever deliver construction noise from it.

```haskell
-- Reflex: the choice is per call site, and the prompt one is the trap
clicks <- switchHoldPromptly never =<< dyn (button <$> label)
```

```prism
# Prism: binding a dynamic of widgets is the dyn plus the one switch
!clicks <- @Button $label
# the click of the occurrence that relabelled the button goes to the old slot, under the old label
```

### There is no PostBuild

`getPostBuild` fires once after the enclosing action has finished building, and "finished" is the instance's to define, so the frame differs between the root and the inside of a `dyn`; `@started` is a `dyn` over a constant, so it fires in the build occurrence everywhere. Chapter 5 ("`@started`") owns the definition. `@started` serves the one use of post-build that survives the removal of sampling, firing a request as soon as a widget exists:

```haskell
-- Reflex: in a later frame at the root; in the rebuild frame inside dyn
pb <- getPostBuild
resp <- requesting (Read path <$ pb)
```

```prism
# Prism: in the build occurrence, wherever the widget is
!started <- @started
!resp <- @Element Fs.File(path=path, requests=(Read on !started))
```

### Elements instead of `Requester` and `MonadQuery`

`Requester`'s method documentation says a response occurs at a later time than its request, which is Prism's creation rule stated by the ancestor; a request element is that idea with the request site a path in the element list rather than a key in a transformer's state. `MonadQuery` is the state half, and its result dynamic has whatever the runner's results dynamic holds when the child is built, which for a query nobody has seen is `mempty`; a state element gets its value from an oracle answered as the occurrence opened. Chapter 6 ("Why state has to be there in its first occurrence") owns the argument.

```haskell
-- Reflex: the first frame has mempty next to fresh captured dynamics
res <- queryDyn (constDyn (Lookup key))
let user = ffor res $ \r -> Map.lookup key (unResult r)   -- Nothing for a frame
```

```prism
# Prism: a value in occurrence zero, with the outside's unavailability in the type
$user <- @Element Store.Value(key=key)       # $Stored: Absent | Present bytes
```

Reflex can express a query as a group with a `crop`, so the interest set is patched rather than re-sent; Prism cannot: an interceptor that wants the aggregation writes a `for` over the child's slots, groups by target, and forwards one. That is more code than `QueryMorphism`, and it is ordinary code. The aggregation Reflex gets from the query monoid, Prism gets from value identity: two slots whose elements are equal observe one state, supplied once.

### The boundary is plain data

A Reflex application improvises its host: `runSpiderHost`, trigger refs, `fireEventsAndRead`, `performEvent` for effects and `liftIO` anywhere in the widget monad, so two Reflex applications with the same widgets can differ about simultaneity. Prism has one run call, chapter 7 ("The run call") owns it, and the Haskell implementation uses `fireEventsAndRead` and trigger refs to implement that call and nowhere else (chapter 14, "What must not be used").

```haskell
-- Reflex: the host decides what fires together
fireEventsAndRead [tClick :=> Identity (), tTick :=> Identity now] readOutputs
performEvent_ $ liftIO . writeFile path <$> saves
```

```prism
# Prism: the program fires a request; the World performs it after the turn
@Element Fs.File(path=path, requests=(Write $draft on !save))
```

### `listWithKey` is `for`

`listWithKey :: Dynamic t (Map k v) -> (k -> Dynamic t v -> m a) -> m (Dynamic t (Map k a))` is `for` almost exactly, and `simpleList` is the array form both discourage; `for` adds consolidation by result kind, and the consolidated event is prompt, so the parent sees a new key's first firing in the key's creation occurrence without choosing a switch. Chapter 5 ("`for`") owns the walkthrough. The insert-at-front case, in both:

```haskell
-- Reflex: simpleList rebuilds every row; listWithKey builds one
rows  <- simpleList items row            -- insert at index 0: every row rebuilt
rows' <- listWithKey itemsById row       -- insert key 7: one row built
clicks <- switchHold never $ mergeMap <$> updated rows'   -- which switch decides the first firing
```

```prism
# Prism: keys are identity, and the consolidated event is the result
!clicks <- for $items_by_id:
    !c <- @Button $"Remove {key}"
    @ key on !c
# insert key 7 in occurrence n: one block built in n; its @started would be in !clicks in n
```
## B. Prior art

Each entry says what the system contributed and sorts its claims into two tiers. *Verified* means the quote is in the book's research notes or in the Reflex sources kept beside them, copied exactly; a quoted sentence under that heading can be relied on. *From memory* means the claim is general knowledge about the system with no quoted source behind it: it is stated as a paraphrase, never in quotation marks, and a URL after it is where a reader who wants to rely on the claim should look. The design leans on the from-memory tier for its intuitions, not for any law, and nothing in Part II depends on a sentence in that tier being exact.

### Elliott, push-pull FRP

**Contribution.** The denotations. A behavior is a value over time, an event is a sequence of timed occurrences, and they are different types; piecewise-constant behaviors, a value plus the discrete changes, are what pushing can implement, and continuous time is what forces pulling. Prism's `$` is Elliott's reactive value restricted to the piecewise-constant case, which is why it can be implemented by propagation and why the language has no notion of a behavior sampled between occurrences. The restriction is on purpose and is a divergence from Elliott's own position, which treats continuous time as essential.

**Verified.** Elliott, "Push-pull functional reactive programming", Haskell Symposium 2009, pp. 25–36 (http://conal.net/papers/push-pull-frp/). Abstract: "Functional reactive programming (FRP) has simple and powerful semantics, but has resisted efficient implementation."; "most past implementations have used demand-driven sampling, which accommodates FRP's continuous time semantics and fits well with the nature of functional programming. Consequently, values are wastefully recomputed even when inputs don't change, and reaction latency can be as high as the sampling period. This paper presents a way to implement FRP that combines data- and demand-driven evaluation, in which values are recomputed only when necessary, and reactions are nearly instantaneous."; the paper introduces "reactive normal form" among its new friends. The Hackage `reactive` package (https://hackage.haskell.org/package/reactive) describes itself as having "a hybrid demand/data-driven implementation" "as described in the paper". Fran (Elliott and Hudak, ICFP 1997), via a published summary: "Behaviors are first-class values, and are built up compositionally"; the design aims to "clearly distinguish between modeling and presentation".

**From memory.** The paper body's `Reactive a = a 'Stepper' Event a`, its description of a reactive value's meaning as a step function fully defined by an initial value and discrete changes, and its remark that reactive values can be evaluated in push style, leaving pull for time functions. Elliott's position that discrete-only systems are not FRP in his sense, defended on his blog.

### Reflex

**Contribution.** Spider's frames as atomic transactions; `Dynamic` as behavior plus event with the rule that the behavior changes exactly when the event fires; the hold law (a sample in the frame that sets the behavior sees the old value), which is Prism's tag law; non-prompt switching as the recommended form; `dyn` and `networkView` firing in the rebuild frame; `subscribeAndRead` as the mechanism for construction in the middle of a frame; `listWithKey`; `Requester`'s statement that a response is later than its request. Also the negative lessons: exposed `current` and `sample`, `PostBuild` whose frame depends on where it is used, and `MonadQuery` whose results have no first value. Appendix A has the row-by-row account.

**Verified.** From the Reflex sources (github.com/reflex-frp/reflex, develop branch, post-0.9; the version number is not stated in the files read) and reflex-dom's `Basic.hs`:

- `Class.hs`: Behavior, "A container for a value that can change over time. 'Behavior's can be sampled at will, but it is not possible to be notified when they change"; Event, "A stream of occurrences. During any given frame, an 'Event' is either occurring or not occurring"; Dynamic, "Basically a combination of a 'Behavior' and an 'Event', with a rule that the 'Behavior' will change if and only if the 'Event' fires"; `hold`, "if the occurrence that sets the 'Behavior' (or one that is simultaneous with it) is used to sample the 'Behavior', it will see the __old__ value of the 'Behavior', not the new one"; `tag`, "Replace each occurrence value of the 'Event' with the value of the 'Behavior' at the time of that occurrence"; `leftmost`, "If multiple occur at the same time the value is the value of the leftmost event"; `coincidence`, "You maybe looking for '@switchHold@ @never@' instead."; `switchHold`, "Only the old event is considered the moment a new one is switched in; the output event will fire at that moment only if the old event does", and "Use this rather than 'switchHoldPromptly' and 'switchHoldPromptOnly' unless you are absolutely sure you need to act on the new event in the coincidental case."
- `Dynamic.hs`: `switchPromptlyDyn`, "The timing requirements that switching before imposes are likely to bring down your app unless you are very careful."; `tagPromptlyDyn`, "__When creating__ __cyclic data flows, generally @/tag (current d) e/@ is preferred.__"
- `Requester/Class.hs`: `requesting :: Event t (Request m a) -> m (Event t (Response m a))`; "Semantically, the response event occurs at a later time than the request event."; module doc, "collect many potential sources of events and send them over a single channel, then distribute the results back out efficiently to their original request sites".
- `Query/Class.hs` and `Query/Base.hs`: `askQueryResult :: m (Dynamic t (QueryResult q))`; `queryDyn q = do tellQueryDyn q; zipDynWith crop q <$> askQueryResult`; `runQueryT :: (MonadHold t m, Commutative q, Group q, Reflex t) => QueryT t q m a -> Dynamic t (QueryResult q) -> m (a, Incremental t (AdditivePatch q))`.
- `PostBuild/Class.hs`: "the intent is to allow 'Behavior's and 'Dynamic's to be safely sampled, regardless of where they were created, when the post-build 'Event' fires. The post-build 'Event' will fire exactly once for an given action."
- `Network.hs`: `networkView :: (NotReady t m, Adjustable t m, MonadHold t m) => Dynamic t (m a) -> m (Event t a)`, "The returned Event of network results fires at post-build time and when the 'Dynamic' updates"; `networkHold`'s own documentation suggests flattening its result with `switchPromptlyDyn`, the function whose documentation says it is likely to bring down your app. reflex-dom `Basic.hs`: `dyn = networkView`, `widgetHold = networkHold`.
- `Adjustable/Class.hs`: `runWithReplace :: m a -> Event t (m b) -> m (a, Event t b)`; the class is for actions that "adjust itself so that its net effect is as though it had originally been run with the new value", with the documentation itself flagging "persistent side-effects: obviously, IO" as the problem case.
- `Spider/Internal.hs`: `subscribeAndRead :: Event x a -> Subscriber x a -> EventM x (SubscribeResult x a)`, where `SubscribeResult` holds `subscribeOccurrence :: Maybe a` (a reading of the record: the occurrence value if the event already fired in the frame); the comment "hold cannot examine its event until after the phase is over"; `EventLoopException` shown as "causality loop detected"; `coincidence` needs its own machinery in the propagation loop (`SubscriberCoincidenceOuter`/`Inner`, `SomeResetCoincidence`), which is evidence that prompt switching costs the engine re-subscription and height resets mid-frame.

**From memory.** The `Reflex` class requiring `Monad (Dynamic t)`, which is why `join` on dynamics exists. `listWithKey`'s full constraint list and `simpleList` keyed by index (`Collection.hs`). `fireEventsAndRead` and `newEventWithTriggerRef` in `Host/Class.hs`; `performEvent`'s documentation saying its output generally occurs later than its input because most actions cannot be performed during propagation; `newTriggerEvent :: m (Event t a, a -> IO ())`. `MonadDynamicWriter` collecting dynamics with `tellDyn` and combining them monoidally. The readthedocs description of `RecursiveDo` as the way to make a cyclic graph, and of `networkView`/`networkHold` as the equivalents of `dyn` and `widgetHold`. reflex-dom's `Builder/Immediate.hs` building an input element's value as a hold over the leftmost of a set-value event and a UI event, with an initial value read from the DOM at build (the build-time read that Prism does not have). `switchPromptlyDyn`, `fanMap`, `select`, and `listWithKeyShallowDiff` as described in the correspondence table.

### Esterel, Lustre, Lucid Synchrone

**Contribution.** The instant as the unit in which nothing half-done is observable. A synchronous program is a totally ordered sequence of logical instants; a signal is present or absent in an instant and has one value there; a signal's value may not be used in its own computation in the same instant, and the compiler rejects the attempt as a causality cycle. Prism's occurrence is that instant, its glitch-freedom law is the consistency the instant gives, and P8 is a small causality analysis: a cycle is legal only through a delayed edge, which is Lustre's `pre`. The synchronous languages are designed for controllers with fixed topology; Prism keeps the law and adds `dyn` and `for`.

**Verified.** Edwards, "The Synchronous Language Esterel", Columbia lecture notes (https://www.cs.columbia.edu/~sedwards/classes/2002/w4995-02/esterel.pdf): "Esterel's model of time is synchronous like that used in hardware. Threads march in lockstep to a global clock."; "Time divided into sequence of discrete instants"; "Each signal is either present or absent in each cycle"; "Can't take multiple values within a cycle"; "A signal emitted in a cycle is visible immediately"; contradictory programs "are erroneous; the Esterel compiler refuses to compile them."; causality is a "Dynamic concept: whether there's a "guess-free proof" that concludes a signal is absent". Von Hanxleden, Kiel "Synchronous Languages" lecture 2 (https://www.informatik.uni-kiel.de/~rvh/ss20/v-synch/lectures/lecture02.pdf): "Computations considered to take no time (synchrony hypothesis)"; "A system works in perfect synchrony, if all reactions of the system are executed in zero time."; "In practice, 'zero time' means before the next interaction"; "Each signal is only present or absent in a cycle, never both"; "All writers run before any readers do"; `present A else emit A end` is "an erroneous program."; "Fully deterministic behavior". Palshikar, "An Introduction to Esterel", embedded.com (https://www.embedded.com/?p=4024644): "the underlying machine is infinitely fast and, hence, the reaction of the system to an input event is instantaneous."; the "hypothesis of atomic reactions"; "Esterel has no facility to refer to any of the past or future instants."

"All writers run before any readers do" is the synchronous-language statement of the occurrence as a transaction; `present A else emit A end` is the synchronous form of a prompt cycle, and the compiler's refusal is the static version of the acyclic-prompt-graph rule.

**From memory.** Berry's *Esterel v5 Language Primer* and its phrases for the perfectly synchronous or zero-delay model, for a valued signal having one status and one value at a time, and for the causality cycle (https://www.rw.cdl.uni-saarland.de/people/kaestner/private/es0203/esterel_primer.pdf). The primary papers: Berry and Gonthier, Science of Computer Programming 1992; Halbwachs, Caspi, Raymond and Pilaud on Lustre, Proc. IEEE 1991; Benveniste et al., "The synchronous languages 12 years later", Proc. IEEE 2003. Lustre's `pre` (the previous cycle's value) and `->` operators. Lucid Synchrone's `when` as a sampler between fast and slow processes, its clocks as boolean sequences, and `current` holding a signal between samples with a default used before the first sample (https://www.di.ens.fr/~pouzet/lucid-synchrone/manual_html/manual006.html); that default-before-first-sample is the hold's initial value.

### STM and transactional memory

**Contribution.** Buffered writes, commit at the end, cheap abort, and one snapshot per transaction. The occurrence is a transaction in exactly this sense: holds buffer their new values and commit at the end, an aborted occurrence leaves the graph as it was, and the World answers the oracle from one snapshot per occurrence. Both the error story (abort the occurrence, report, continue) and the parallelism story (optimistic execution across occurrences, later) lean on this.

**From memory.** Haskell's `stm` and `retry`, which abandons the current transaction because it has seen values that mean it should not continue (https://hackage.haskell.org/package/stm/docs/Control-Monad-STM.html); Harris, Marlow, Peyton Jones and Herlihy, "Composable Memory Transactions", PPoPP 2005, as its origin. PostgreSQL's Repeatable Read, which sees a snapshot as of the transaction's first statement and asks applications to be prepared to retry on serialization failure (https://www.postgresql.org/docs/current/transaction-iso.html). Datomic's database value as an immutable point in time, so that several queries against one value are known to see the same data (https://docs.datomic.com/whatis/data-model.html). The design uses only the common idea: a snapshot per transaction, writes visible at commit, abort as a cheap and complete undo.

### Edge and level: epoll, kqueue, interrupts, inotify, ZooKeeper, Kubernetes, Debezium, Kafka, market data

**Contribution.** Every one of these systems rediscovered the same two facts. A subscriber needs a snapshot and the changes since, consistent with each other, keyed by a version so that the seam between them is exact. And "you will get a change notification eventually" is not a value: an edge without the level it refers to is a wake-up, after which the program must read. Prism's state elements are that pattern as a language rule: the oracle is the snapshot, the pushes are the changes, the occurrence is the version, and a slot created in n reads the oracle (before n) and then receives n's push, so it ends n holding what every other slot holds. The creation rule is the other side: a stream element reports changes after it exists and nothing at start, because a first event that said "here is what it is now" would be a query answering a question nobody asked.

**Verified.** Kafka Streams, Confluent documentation (https://docs.confluent.io/platform/current/streams/concepts.html): "A stream can be considered a changelog of a table"; "A stream is thus a table in disguise"; "A table can be considered a snapshot, at a point in time, of the latest value for each key"; "A table is thus a stream in disguise"; "Essentially, this duality means that a stream can be viewed as a table, and a table can be viewed as a stream."; "A KTable is an abstraction of a changelog stream, where each data record represents an update." Hanlon, "What is tearing?", React 18 working group (https://github.com/reactwg/react-18/discussions/69): "In a user interface, by "tearing" we mean that a UI has shown multiple values for the same state." (the UI-side name for a subscriber whose snapshot and changes disagree).

**From memory**, each with where to check it. The design leans on this entry hardest, so a reader who wants to quote any of it should fetch the source first; the numbers below in particular are approximate.

- epoll (https://man7.org/linux/man-pages/man7/epoll.7.html): the interface behaves as either edge-triggered or level-triggered; in edge-triggered mode an event is delivered only when the descriptor changes, so the man page's pipe example, where more is written than is read, hangs on the next wait despite data still buffered; the recommended edge-triggered use is nonblocking descriptors read until `EAGAIN`; level-triggered is described as a faster `poll`.
- kqueue (https://man.freebsd.org/cgi/man.cgi?query=kqueue): `EV_CLEAR` resets an event's state after it is retrieved, for filters that report transitions rather than current state; a socket read filter also reports how many bytes are available, which is the level beside the edge.
- Interrupts (https://en.wikipedia.org/wiki/Interrupt): an edge-triggered interrupt can be missed while interrupts are masked, unrecoverably unless hardware latches it, and modern hardware has status registers that edge-driven handlers check so nothing is missed. The Linux generic IRQ layer's edge handler loops while the pending bit is set and has a delayed-disable scheme against losing edges while disabled (https://docs.kernel.org/core-api/genericirq.html).
- inotify (https://man7.org/linux/man-pages/man7/inotify.7.html): the event queue can overflow and then events are lost, with a single overflow event always generated; robust applications must handle lost events, possibly by rebuilding their cache; after adding a watch on a new directory, scan it at once; pairing move-from and move-to events is inherently racy.
- ZooKeeper (https://zookeeper.apache.org/doc/current/zookeeperProgrammers.html): a watch is a one-time trigger; a client sees the watch event before the new data, and never sees a change it watched for without first seeing the event; a client cannot reliably see every change; the zxid exposes the total order of all changes.
- Kubernetes (https://kubernetes.io/docs/reference/using-api/api-concepts/): the watch mechanism lets a client fetch current state and then subscribe to changes without missing any; the server keeps a few minutes of history by default; on `410 Gone` the client clears its cache, re-lists, and watches from the returned resource version; the Go client's Reflector is the standard list-then-watch tool. The controllers guide (https://github.com/kubernetes/community/blob/master/contributors/devel/sig-api-machinery/controllers.md): level driven, not edge driven; an object seen with a marker of true gives no assurance of having seen it turn from false; distinct watches update independently. The API conventions: behavior is level-based rather than edge-based, for robustness against missed intermediate states.
- Debezium's PostgreSQL connector (https://debezium.io/documentation/reference/connectors/postgresql): the initial snapshot records the log position, then scans the tables, then streams from the recorded position, so no change made during the snapshot is missed.
- Binance's guide to a local order book (https://github.com/binance/binance-spot-api-docs/blob/master/web-socket-streams.md): buffer the stream, fetch a depth snapshot, discard buffered events at or before the snapshot's update id, restart if the first buffered event is newer than the snapshot, and restart if a gap appears in update ids; the snapshot has a fixed depth per side, so levels outside it are learned only when they change. CME MDP 3.0 market recovery: a last-sequence-number field on the snapshot against which cached incremental updates are dropped, and packet loss indicated by a sequence gap. Coinbase Exchange: queue messages, fetch the REST snapshot, play back the queue discarding sequence numbers at or before the snapshot's. FIX market data requests: a subscription type of snapshot, or snapshot plus updates, the latter sending the current state and then updates as they occur.

### React

**Contribution.** Three places where React's later additions mark where the model leaked, used in Part I as the comparison: `useSyncExternalStore` is a consistency check with a blocking restart, added after tearing was named; Suspense is a mechanism that interrupts a render when a value is not there yet and retries the subtree from scratch; the compiler is manual memoization admitted to be tedious and easy to get wrong. The retry-from-scratch of a suspended render is also the closest cousin of Prism's rule that a widget built in n behaves as if built before n's inputs: a render that cannot finish consistently is thrown away and rerun, not resumed.

**Verified.** `useSyncExternalStore` (https://react.dev/reference/react/useSyncExternalStore): "While the store has not changed, repeated calls to `getSnapshot` must return the same value."; "The store snapshot returned by `getSnapshot` must be immutable."; during a transition "React will call `getSnapshot` a second time just before applying changes to the DOM. If it returns a different value than when it was called originally, React will restart the update from scratch, this time applying it as a blocking update, to ensure that every component on screen is reflecting the same version of the store." The hook's page does not use the word "tearing"; the React 18 working group's discussion defines it (quoted under the edge-and-level entry), and the React 18 release post says the hook works "by forcing updates to the store to be synchronous". Suspense (https://react.dev/reference/react/Suspense): "`<Suspense>` lets you display a fallback until its children have finished loading."; "React does not preserve any state for renders that got suspended before they were able to mount for the first time. When the component has loaded, React will retry rendering the suspended tree from scratch." Effects (https://react.dev/reference/react/useEffect): "**Notice that you can't "choose" the dependencies of your Effect.** Every reactive value used by your Effect's code must be declared as a dependency."; "By suppressing the linter, you "lie" to React about the values your Effect depends on."; Strict Mode "will **run one extra development-only setup+cleanup cycle** before the first real setup." React Compiler (https://react.dev/learn/react-compiler/introduction): "This manual memoization is tedious, easy to get wrong, and adds extra code to maintain."; the compiler "automatically applies the equivalent of manual memoization, ensuring that only the relevant parts of an app re-render as state changes, which is sometimes referred to as 'fine-grained reactivity'." Batching (https://react.dev/learn/queueing-a-series-of-state-updates): "**React waits until *all* code in the event handlers has run before processing your state updates.**"

**From memory.** The implementation of Suspense as a thrown promise, with `ReactFiberThenable.js`'s `SuspenseException` describing itself as not a real error; the documentation names only `use` and `lazy` as activators and does not promise the mechanism. The "Keeping components pure" page's description of Strict Mode calling each component's function twice in development.

### Solid and signals

**Contribution.** Fine-grained dependency tracking without re-rendering, which is the closest mainstream thing to `$`; and the observation that glitch freedom is a property each implementation engineers rather than one the model gives, which is the reason Prism states it as a law and orders propagation by the prompt graph. A signal graph has behaviors and no events; Rx has events and no behaviors; FRP has both as types.

**Verified.** Solid documentation (https://docs.solidjs.com/concepts/intro-to-reactivity, https://docs.solidjs.com/advanced-concepts/fine-grained-reactivity): "Signals are like mutable variables that can point to a value now and another in the future."; "Effects are functions that are triggered when the signals they depend on point to a different value."; "During this run, if the effect accesses any signals it is then registered as a subscriber to those signals."; "When a signal changes, any corresponding subscribers are immediately updated in an ordered manner."; "`batch` can be used to delay an update so the subscriber runs after each signal has been updated." Neither page uses the words "glitch" or "topological"; consistency is told through `batch` and "ordered manner". TC39 Signals proposal (https://github.com/tc39/proposal-signals): "Computation is glitch-free, meaning no unnecessary calculations are ever performed." (a definition of glitch-free as no wasted work, not as never observing an inconsistent state); "A computed Signal always observes the Signal graph in a consistent state,"; "Computed signals are lazy, i.e., pull-based"; "The Signal API does not include any built-in function like `effect`."; the `Signal.subtle` namespace holds "advanced" features "better to leave for framework authors". The proposal's issue #178 is an open argument between eager tree-ordered evaluation and lazy dirty-marking, which is scheduling left to each implementation. Leptos's description of the Solid-family push-pull scheme (https://book-ru.leptos.dev/appendix_reactive_graph.html): "A naive, push-based reactive implementation would cause this effect to run twice, which would be bad."; node states "`Clean`", "`Check`", "`Dirty`"; "It pushes the `Check` status all the way down the graph, but then "pulls" its way back up."

**From memory.** Svelte 5 runes, Vue's `ref`/`computed`/`watchEffect`, Angular's `signal`/`computed`/`effect` and Preact Signals, as names only; their schedulers are not described anywhere in this book.

### Rx and streams

**Contribution.** Events as values with good combinators, which is half of what is needed, and the clearest example of what a pushed level looks like: a `BehaviorSubject` fires its current value to every new subscriber, which is the form Prism's creation rule forbids. The stream-table duality (Kafka, above) is what happens when there are streams and no behaviors.

**Verified.** RxJS manual (https://reactivex.io/rxjs/manual/overview.html): "Observables are lazy Push collections of multiple values"; "each subscribed Observer owns an independent execution of the Observable"; "A Subject is like an Observable, but can multicast to many Observers"; "Observables are able to deliver values either synchronously or asynchronously."; a `BehaviorSubject` "has a notion of "the current value"", and a new observer "immediately receives the current value" (https://www.learnrxjs.io/learn-rxjs/subjects/behaviorsubject).

**From memory.** The rxjs.dev subject guide's example that a stream of birthdays is a Subject while a person's age is a BehaviorSubject (https://rxjs.dev/guide/subject). Akka Streams, as a name.

### OCaml's extensible variants

**Contribution.** The element typing: `type Element = choice ..` is OCaml's `type t = ..`, with constructors declared across modules, each with concrete field types, and a `match` that must end in a catch-all because the set of alternatives is open. Prism adds the tag's three parts (`requests`, `responds`, `state`) and the namespacing.

**From memory.** The OCaml manual's chapter on extensible variant types (https://ocaml.org/manual/extensiblevariants.html); the OCaml sources named under the compiler entry are about values and bytecode, not variants.

### Plotkin–Pretnar, OCaml 5 effects, Koka

**Contribution.** Considered for interceptors and not used. An effect handler is a dynamic-scope construct: a `perform` finds the innermost handler on the stack, which gets a delimited continuation and may resume it. An interceptor is a widget in the tree: its scope is lexical in the widget tree, its "handling" is supplying a dynamic or a response by path, and it never resumes anything, because nothing in Prism suspends. The design borrowed the vocabulary of handling and forwarding (a catch-all arm that re-emits is OCaml's `| _ -> None`, read as forwarding the unhandled effects to the outer handler) and rejected the mechanism, because a handler on a stack can be reached from anywhere below it and the sandbox invariant needs the opposite: nothing reaches past an interceptor unless Prism code sends it. The one idea that survives from Koka is that a handler which answers at once needs no captured continuation; an interceptor's state supply is that case, and its request answering goes through `Defer` rather than through a continuation.

**From memory.** Plotkin and Pretnar, "Handling Algebraic Effects", LMCS 9(4:23), 2013 (https://arxiv.org/abs/1312.1399), the journal version of the ESOP 2009 paper, whose handlers generalise exception handlers with timeouts, rollback and stream redirection among the examples. The OCaml 5 manual's effects chapter (https://ocaml.org/manual/5.2/effects.html): the handler receives the delimited continuation representing the suspended computation between `perform` and the handler; `| _ -> None` reads as forwarding to the outer handler; an unhandled effect raises at the `perform`; every captured continuation must be resumed exactly once. Koka's evidence passing, under which a tail-resumptive operation is called directly as a function without capturing the stack; Koka's README describing exceptions, async/await and probabilistic programs as user libraries. Bauer and Pretnar's Eff.

### GHC's core-lint and GHCi

**Contribution.** The lint pass of the Rust compiler: a typed intermediate form that is re-checked after every pass, so that a conversion that breaks a type is caught at the pass that broke it rather than at runtime. The typed AST is the form everything else is checked against, as Core is in GHC. GHCi is the model for an interpreter that runs beside compiled code against one specification.

**From memory.** The GHC user's guide (https://downloads.haskell.org/ghc/latest/docs/users_guide/debugging.html): `-dcore-lint` as intra-pass sanity checking at the Core level, `-ddump-simpl` for simplifier output, `-ddump-bcos` for the byte-code objects. GHCi's documentation: a loaded module is converted to byte-code and interpreted; interpreted code runs alongside compiled code; compiled code is roughly an order of magnitude faster. The recursive-do extension's `mfix` and its description of a `rec` block as telling the compiler where the recursive knot is tied. The STG notes describing STG as a variant of administrative normal form, every `let` as a closure allocation, and the main work of Core-to-STG as computing free variables to construct closures, which is the closure conversion the Prism compiler also has to do.

### Lua 5.0, the WebAssembly rationale, OCaml's bytegen, Cranelift

**Contribution.** Studied for the compiler and set aside. A register VM is a dispatch-cost design, and Prism's per-occurrence cost is at the runtime boundary rather than in dispatch; structured control flow is kept for `match` because it makes verification one pass; OCaml's two backends sharing one Lambda tree is the model for two Prism implementations sharing one typed-AST text form; Cranelift's SSA with block parameters is where a native backend would start if one is ever built. Two things survived into the plan: the closure compiler over the typed AST, and the principle that running Prism code produces a graph.

**From memory.** Lua 5.0's paper (https://www.lua.org/doc/jucs05.pdf) describing its virtual machine as the first widely used register-based one, register code avoiding the push and pop instructions of stack code, and the new machine as considerably faster than the old. The WebAssembly design rationale (https://github.com/WebAssembly/design/blob/main/Rationale.md): started from an AST and generalised to a structured stack machine; restriction to structured control flow simplifies one-pass verification by avoiding a fixpoint computation. OCaml's `bytecomp/bytegen.mli` (generation of bytecode from lambda terms) and `middle_end/closure/closure.mli` (introduction of closures, uncurrying, recognition of direct calls); Real World OCaml on bytecode as a close mapping to the lambda form, whose advantage is simplicity, portability and compilation speed. Cranelift as a low-level retargetable code generator with several backends, and `cranelift-frontend` as the API that turns mutable variables into SSA (https://github.com/bytecodealliance/wasmtime/tree/main/cranelift). Crafting Interpreters on walking the AST as the wrong execution model, for locality and speed (https://craftinginterpreters.com/chunks-of-bytecode.html). Feeley and Lapalme, "Using Closures for Code Generation", Computer Languages 12(1), 1987, as the technique of compiling each node to a host closure. Rustlantis (https://github.com/cbeuw/rustlantis) treating any discrepancy between backends as a bug in one of them, the model for the differential test. LLVM's developer policy stating that the textual IR is not backwards compatible, one reason LLVM is not a target.

### Squeak

**Contribution.** An implementation written in a subset of the language it implements, debugged inside the language's own environment, as the model for keeping two Prism implementations honest against one specification: the Haskell implementation exists to read Part II a second time, and the differential test is the Interpreter Simulator's job done between two readers.

**From memory.** Ingalls et al., "Back to the Future: The Story of Squeak", OOPSLA 1997 (https://ftp.gwdg.de/pub/languages/smalltalk/st.cs.uiuc.edu/Smalltalk/Squeak/docs/OOPSLA.Squeak.html): the VM written entirely in Smalltalk, in a subset that maps directly onto C; a subclass, the Interpreter Simulator, running the same code inside a Smalltalk environment and serving as the basis for debugging the system into existence.

### Also consulted

Each of these contributed one idea, named here so the reference says why and not only where. All are from memory.

- Jane Street's Incremental (https://github.com/janestreet/incremental): a variable set during stabilization takes effect at the next one, and a node created during a stabilization has its initial value for that stabilization, which is the mid-occurrence construction rule in another system.
- Differential dataflow (https://github.com/TimelyDataflow/differential-dataflow): `advance_to` as a promise to stop changing an input at all times before a given one, the version as the seam.
- Verilog elaboration, Nix instantiation and Bazel's analysis phase: three systems in which evaluating the program produces a graph that something else then runs, the principle behind "building is the only doing".
- winit (https://docs.rs/winit): the event loop must be created on the main thread and only once per application, a constraint every GUI World inherits and the reason the turn loop lives on the platform's main thread.
- AccessKit (https://github.com/AccessKit/accesskit): what an accessibility tree wants from a draw list, which is why `Gui.Scene` is data a World can read rather than a sequence of calls.
- wasi-http (https://github.com/WebAssembly/wasi-http): a request/response boundary with no callbacks, the model for a World whose whole life is one turn.
- Elm's `Cmd`/`Sub`, Cycle.js drivers, Haxl's batching, and the Rust and C# `async` state machines, as the other designs in which a program describes effects and something outside it performs them.
## C. Syntax quick reference

### Sigils

| Sigil | On a value | On a function name | On a type |
| --- | --- | --- | --- |
| `!` | an event | returns an event | `!A`, event of `A` |
| `$` | a dynamic | returns a dynamic | `$A`, dynamic of `A` |
| `@` | a widget | returns a widget | `@A`, widget returning `A` |
| `?` | an optional | returns an optional | `?A`, `Nil` or `Some a` |
| none | a plain value | returns a plain value | a plain type |

Sigils nest leftmost-outermost: `?$user`, `!!rendered`, `@!unit`, `$@a`, `$(Key ==> $Item)`. A sigil is attached (part of the token) when followed by a name character, `_`, `"` or another sigil; otherwise it is a prefix operator over the whole expression to the end of the call: `$ x` is the constant dynamic, `@ v` the widget returning `v`, `@ ((!a, $b))` a bundle. `$_`, `!_`, `@_` are the pronoun at that kind. `&` is reserved for future work; `%` does not exist. Names: `snake_case` for values, `UpperCamelCase` for functions and types; a camelCase value name such as `isClick` is a lint warning (write `is_click`), not a compile error. A dotted name such as `Gui.Button` lexes as name, `.`, name, and `Gui.*` in a pattern as name, `.`, `*`.

### Lexical

Comments `#` to end of line. UTF-8 source. Layout is the offside rule with `:` as the block opener: a `:` ending a line opens a block on the next, more-indented line; a `:` with tokens after it on the same line opens an inline block of that line, and `else` terminates an inline block, so `if c: a else: b` is one line with two inline blocks; inside `( ) (( )) [ ] [[ ]] { }` layout is suspended and `:` is the map key separator. Inline `:` is a plain token the parser interprets, not a layout token. `;` separates statements in an inline block: `type Pair<a, b> = record: first: a; second: b`, `if c: Log x; Log y else: Log z`; a layout block never needs it. `((` and `))` open and close a tuple, `[[` and `]]` an array literal; single `( )` is grouping and single `[ ]` is indexing or an array type. A trailing lambda as the last argument absorbs the rest of the line, so `MapValues !xs, \|x| x + 1` needs no parentheses around the lambda. Keywords: `let type tag of fn match if else for dyn intercept widget record choice map array unit test ignore on not and or lt le gt ge key empty`; keywords are reserved everywhere (none is contextual), and there are no index variables `ix`, `jx`, `kx`: a loop names its own; `read` and `update` are reserved for future work. `never started main Switch Leftmost Join Filter Updated Distinct Element Kind` are prelude names written with their sigils where they have one: `!never`, `@started`, `@main`, `!Switch: fn !!A -> !A`, `!Leftmost: fn [!A] -> !A`, `$Join: fn $$A -> $A`, `!Filter: fn !?A -> !A`, `!Updated: fn $A -> !A` (fires with the new value in every occurrence the dynamic changes; never at build), `$Distinct: fn $A -> $A` (needs equality; suppresses equal-valued changes, which nothing else does), `@HoldFirst: fn !A -> @$A` (a dynamic that holds its event's value, with the first firing's value, in the build occurrence, as its initial value), `@Element`, `Kind: fn Element -> Kind`, and the optional default `Or: fn ?A, A -> A` (`Or ?x, default`; a function, unrelated to the keyword operator `or`). There are no prelude `Not`, `And` or boolean `Or` functions: `not`, `and`, `or` are operators. `Wrap.*` and `Checked.*` are the prelude's wrapping and checked arithmetic (`Wrap.Add: fn Nat, Nat -> Nat`, `Checked.Add: fn Nat, Nat -> ?Nat`, and likewise `Sub`, `Mul` and the `Int` forms); the bare operators abort the occurrence with an `Overflow` error.

Literals: an integer-spelled literal is `Nat` or `Int` by inference, and `Int` when nothing constrains it (`$n <- 0 <= $n + 1 on !tick` is an `Int` counter); `-1` is an `Int`; a `Nat` is pinned by annotation; a decimal-spelled literal is `Float`. `"text"` with the escapes `\\ \" \n \t \{ \} \u{hex}` and interpolation `{expr}` only (`"hi {name}"`, `"count: {$count}"`); a `$` in text is always a literal `$`, so `"$count"` is plain text, and a literal brace is `\{` or `\}`. Interpolating a dynamic or an event lifts the whole literal (`"Count: {$count}"` is a `$Text`; `"got {!line}"` is a `!Text`). `'c'`, `unit`; `[[1, 2, 3]]` for an array, `[[]]` or `empty` for the empty one; `{k: v}` for a map, `{}` for the empty one; `((a, b))` for a tuple. The spacing rule: a token that can be prefix or binary is prefix when preceded by whitespace and followed immediately by a non-space (`F -1` passes negative one; `F - 1` subtracts). Wherever `_` is in scope (a `\` body, a `for` body, any pronoun-binding construct) a prefix `.x` is `_.x` and a prefix `[i]` is `_[i]`; with no `_` in scope it is an unbound-pronoun error, and a `.Method` that starts a continuation line is a chain step.

### Statements in a widget block

| Form | Meaning |
| --- | --- |
| `pattern <- widget` | build the widget as a child of this node; bind its result |
| `widget` | build it; discard the result |
| `let pattern = expr` | bind a value of any kind; builds nothing |
| `$x <- v <= !e` | hold: `$x` is `v` until `!e` fires, then `!e`'s value; owned by this node; a hold set to the value it already has still changes |
| `!!r <- dyn $x: block` | rebuild the block on each change of `$x`; `_` is the plain value; event fires in every build occurrence |
| `!!r <- dyn p <- $x: block` | the same, destructuring `$x`'s value with pattern `p` |
| `r <- for k, $v <- $m: block` | one block per key of `$(K ==> V)`; `k` is the key and `$v` the value as a dynamic; consolidated by result kind, promptly |
| `r <- for $m: block` | the shorthand: `key` is the key and `$_` the value as a dynamic; `for $v <- $m:` names only the value |
| `((r, $slots)) <- intercept @child, !responses, $supplier` | the interceptor primitive (below) |
| last statement | the block's result; `@ v` returns a plain value |

Bindings in a block are mutually recursive; a cycle must pass through a delayed edge (a tag, `!Switch`'s hold, a hold's value side). A block body is `@name:` at top level, `let @w = @widget:` inline, or `\@widget:` as a lambda body. A plain `for k, x <- xs: body` (or `for xs: body`) outside a widget block maps over a plain array or map and is an expression; over an array the key is the index.

A top-level definition is `name: type = expr`, with the annotation required and the name's sigil saying what kind of thing it is: `@Button: fn $Text -> @!Click = \|$label| @widget: ...`, `IsOne: fn Int -> ?unit = \|n| ...`, `@main: @unit = @widget: ...`. The shorthand `@name:` opens the widget block of a definition of type `@unit` that takes nothing. `let` is the binding form inside a block and never appears at top level.

### Operators, loosest to tightest

| Level | Operators | Notes |
| --- | --- | --- |
| 1 | `<-`, `let ... =` | statement level |
| 2 | `<=` | hold |
| 3 | paren-less application | extends to the end of the expression: `F a + 1` is `F(a + 1)`, and `(F a) + 1` is the other grouping; commas belong to the outermost call on the line or in the enclosing single-paren group; a trailing lambda absorbs the rest of the line |
| 4 | `\|` | leftmost merge of events; associative; `!never` is its identity |
| 5 | `on` | `v on !e` fires with `!e`, value `v` (dynamic `v` read before the occurrence) |
| 6 | `or` | boolean or; associates left; short-circuits when plain |
| 7 | `and` | boolean and; binds tighter than `or`: `x or y and z` is `x or (y and z)`; short-circuits when plain |
| 8 | prefix `not` | boolean negation; looser than comparison, so `not a == b` is `not (a == b)` and `not a lt b and c` is `(not (a lt b)) and c` |
| 9 | `lt le gt ge`, `== !=` | comparison; `a lt b` through the ordering trait, `a == b` through the equality trait; `Bool` result; does not associate |
| 10 | `+ -` and `+~ -~` (Int), `+. -.` (Float) | suffixed spellings only when inference cannot decide between `Int` and `Float`; there are no suffixed `Nat` spellings, a `Nat` is pinned by annotation |
| 11 | `* /` and suffixed | `/` on `Nat` and `Int` returns `?`; on `Float` it follows IEEE (infinities, NaN) and never fails |
| 12 | prefix `+ -`, free sigils `! $ @ ?`, `+?` | prefix `+` is numeric identity; `+?: fn Int -> ?Nat` converts, `Nil` below zero |
| 13 | `.field`, `[index]`, attached sigils, `.F args` method form | `x.F y` is `F x, y`; chains associate left, so `x.F.G` is `G (F x)`; a `.F` that starts a continuation line closes the call above it and takes it as receiver; `a[i]` and `m[k]` return `?` |

`not`, `and` and `or` are keywords over `Bool`, lifted by the sigils like any operator: `$done or $count gt limit on !click` is `($done or ($count gt limit)) on !click`, and `a lt b and c lt d` needs no parentheses. Plain `and` and `or` short-circuit, so an overflow in the operand that was not needed aborts nothing; lifted over dynamics or events they are lifts of the same function and fire by the lifting rule.

Parentheses: a single-paren group whose head is a call is that call with all its commas, so `(F a, b)` is `F(a, b)` and `G x, (F a, b)` gives `F` two arguments. Tuples are double parens: `((a, b))` is a pair, `((F a, b))` the pair of `F a` and `b`, and a multi-argument call inside a tuple takes its own single parens, `(((F a, b), c))`. Array literals are double brackets, `[[1, 2, 3]]`; single `[i]` is always indexing. A chain is written one step per line, and indentation decides where each call ends:

```prism
items.Filter IsDone
    .MapValues Label            # MapValues (Filter items, IsDone), Label
```

On one line, `x.F y` is method form and a spaced `.F` is not a chain step. Comparisons are the infix keywords `lt`, `le`, `gt`, `ge` (`a lt b`), resolved through the ordering trait, and `==`, `!=`, resolved through the equality trait. Arithmetic, equality and ordering are the three built-in traits; nothing else is overloaded. Records have equality when every field does, and ordering, lexicographic in field declaration order, when every field is ordered; tuples, arrays, primitives and text are ordered; `Float` has a total order (IEEE totalOrder) and can key a map; the alternatives of a closed `choice` are ordered by declaration order; element equality is the tag plus the plain fields, never the request event, and elements are ordered by tag name then plain fields, so they can key sorted maps.

### Lifting

Read off the operands' sigils at every operation and application. All plain: plain. Some `$`, no `!`: a dynamic, prompt. Some `$` and one `!`: an event firing with the `!`, the dynamics read before the occurrence (tag law). Two `!` in one operation: an error naming both. Field access and indexing lift like application: `!!panes.!inbox` and `$users[id]` are fine. `@` operands are not lifted; `@` as a prefix lifts a plain value into a widget. `?` lifts at the plain layer like an applicative `Maybe` (`a / b + 1` is `?Nat` for `Nat` operands; `Float` division has no `?`), and `match` on a `?` is its eliminator. Lifting never crosses a sigil boundary; `!Switch` (non-prompt) and `$Join` (prompt) do. `match $x:` and `match !e:` lift like application when all arms are plain; when any arm is a widget the scrutinee must be plain. A derived dynamic recomputed to an equal value still changes; `$Distinct` is the one place equal values are suppressed.

Binding a widget application lifts by the result: `!x <- @Button $label` is `dyn $label:` around the build with `!x` as the `!Switch` of the result; a state-returning element under a `dyn` binds as a hold of dynamics joined with `$Join`, with the first build's result as the hold's initial value; a widget application whose result is plain binds as a `$` holding the first build's value. An unannotated `let` lambda is a plain function lifted afresh at each application; a lambda mentioning `$_`, `!_` or `@_` takes that kind and is not lifted over it.

### Lambdas and patterns

`\body` takes `_` (or the sigiled pronoun its body uses); `\|a, b| body` names arguments; `\|((x, y)), k| body` destructures a pair argument; `\@widget:` opens a widget block. `ignore` as an argument discards it. Wherever `_` is in scope (a `\` body, a `for` body, any pronoun-binding construct) a prefix `.x` is `_.x` and a prefix `[i]` is `_[i]`, so `\.done` and `\[0]` are lambdas.

Patterns: a literal; a name (binds); `ignore`; `((p, q))` for a tuple; `Some p`, `Nil`; `Type p` for a newtype or payload alternative, `Alt p, q` for an alternative with several payloads, comma-separated as in its construction; `Type(f1, f2=p, ..)` for a record, positional or by name, `..` for the rest; `Ns.Tag(field=p, ..)` for an element by tag; `Ns.*(..)` for any tag in a namespace. `[[]]` matches the empty array, `[[a, b]]` exactly two elements, and `[[head, ..rest]]` a non-empty one (`..` takes the rest and comes last). `match` arms are tried in order; an arm may have a guard, `pattern if cond: body`, and a guarded arm does not count toward exhaustiveness (an extensible variant needs an unguarded catch-all). A closed choice must be exhaustive, arrays included. `if c: a else: b` is `match` on `Bool`, and `if c: a` with no `else` is `?T`: `Some a` when `c` holds and `Nil` otherwise (`?unit` when `a` is `unit`), a `$?T` over a dynamic condition.

### Types

```prism
type Vector = record:            # record; constructor is the type name
    x: Float
    y: Float

type Figure = choice:            # choice; a payload alternative is a newtype
    Circle Float
    Square Float
    Line Point, Point            # several payloads: the alternative holds the tuple ((Point, Point))

type Bool = choice True | False  # one-line choice
type Meters = Meters Float       # newtype, unwrapped with .value
type Click = click               # unit type, one value, lowercase
type Pair<A> = record: ...       # generics, erased; no type variables in tag fields
fn A, B -> C                     # function type; fn A -> C for one argument; fn unit -> C for none
fn ((A, B)) -> C                 # one argument, a pair
fn (fn A -> B), [A] -> [B]       # a function-typed argument or result is parenthesised
fn A -> (fn B -> C)
[T]   K ==> V   ((A, B))   ?T    # array, sorted map, tuple, optional
```

Primitives: `Nat Int Float Text Char Bytes unit`; `Int` is 64-bit signed and `Nat` 64-bit unsigned. A record with sigiled fields is a bundle, read with `.!changed`:

```prism
type TextInput = record:
    !changed: Text
    $value: Text
```

Fields of a record or tag go one per line in a layout block, or separated by `;` in an inline block. The prelude's `Kind` is a record, `type Kind = record: requests: Bool; responds: Bool; state: Bool`, and `Kind e` says which parts the element `e`'s tag has.

### Tag declarations

```prism
type Element = choice ..                 # the extensible variant, once

tag Ns.Name of Element:                  # a tag, in any module
    field: PlainType                     # fields: plain data, concrete types, fixed for the slot's life
    requests: !R                         # optional: an event the program supplies; firings go out
    responds: R                          # optional: the element returns !R; firings come in later
    state: S                             # optional: the element returns $S; a value from creation
```

Construction spells the request event as a named field: `Cli.Stdout(requests=!line)`, `Fs.File(path=p, requests=(Read on !started))`. The request event is excluded from equality, so two slots differing only in their request events are equal elements and share state. `@Element e` returns `@unit` (fields only, or requests only), `@!R` (responds), `@$S` (state, with or without requests), `@((!R, $S))` (responds and state); in a catch-all arm the result is the generic `@((!Response, $State))`, checked per slot at runtime, and inside a tag-specific arm `@Element _.element` has that tag's types. A dynamic where a field goes lifts to a `dyn` with a fresh slot per change. An event where a field goes is an error unless the field is `requests`. Responses are addressed by path; state is shared by element value. Elements never fire in the occurrence they were created; a state element has a value in it. There is no `default:` line in a tag: a World that cannot answer the oracle for a state tag makes the slot's creation an `UnsuppliedState` error, and a test gives the value with a `State` step before the build.

### `intercept`

```prism
intercept: fn @a, !(Path ==> Response), $(fn Element -> @$State) -> @((a, $(Path ==> Slot)))

type Slot = record:
    element: Element
    !request: Request        # typed by the tag inside a match arm on element
```

Builds `@a` as a child and captures its element list. Down: `!responses` by path, routed against the list as the occurrence opened; the supplier, read at each state slot's creation, applied to the element, its widget built in the interceptor's node once per distinct element value, its result the slot's dynamic. A new child slot whose element value already has a live supplied widget joins it, whatever the supplier dynamic holds now. Up: whatever the interceptor makes with `@Element`. Forwarding is `\|e| @Element e`. Answering a request yourself goes through `Defer`. Pure code firing at a path created in the same occurrence is an `UnknownPath` error.

### Test scripts

A test is a top-level statement that the suite's harness runs: a function from a widget and an array of steps to a transcript and a verdict, under the Test World, which accepts every tag.

```prism
test name: Test @widget: array:
    Expect pattern                  # some slot at any depth matches
    ExpectTop pattern               # some root-level slot matches
    ExpectNone pattern              # no slot matches
    ExpectTopNone pattern           # no root-level slot matches
    ExpectCount pattern, n          # exactly n match
    Send pattern, response          # exactly one slot matches; deliver; run a turn
    Together:                       # several Sends in one occurrence
        Send pattern, response
        Send pattern, response
    State pattern, value            # set the snapshot for a state element; next turn's oracle and push
    Push pattern, value             # State, then a turn with no other input
    Returns pattern                 # the widget's result matches (a dynamic by current value); on an event result, a script error (use Fired/Quiet)
    Fired pattern                   # the result event fired in the last turn with a matching value
    Quiet                           # the result event did not fire in the last turn
    Requested pattern, value        # a matching slot fired its request with a matching value
    Unrequested pattern             # no matching slot fired its request in the last turn
    Errors pattern                  # the last turn ended in a matching runtime error
```

The build is turn zero. Each `Send`, `Together` or `Push` is one turn including its `Defer` occurrences; independent `Defer`s fired in one occurrence are answered together. `State` may appear before the build and must for every state tag the program creates at build, since the oracle has to answer in occurrence zero. Patterns are element patterns; `Errors` patterns are `Error(kind=Cycle, ..)` and the like.

Transcripts and `Send` targets spell paths canonically: a slot or child by its index in the parent's child list, a `dyn` generation as `#n`, a `for` key as `[k]`, and a supplier-built slot as `s/n/...` with the supplied widget's own segments after the prefix. Only slots, `dyn` generations, `for` keys and intercept children take segments; a plain widget-function application makes a node but adds no segment, so a `@Button` whose body holds the `dyn` prints as `0/#0/0`, and the third row of a keyed list's second generation prints as `1/#1/[3]/0`.
## D. The element vocabulary

Every tag the design uses, in one place, so that a chapter, a test, a World and a library agree on the names. The language fixes the kinds (which parts a tag has) and the laws (creation rule, routing by path, identity by value); the names and fields are the vocabulary the book's examples and Worlds speak. A World accepts a subset of these natively and says which through `World.Tags`; everything else under it is Prism, as interceptors over the tags it does accept, or an `Unsupported` response.

Four things apply to every row.

*Fields* are plain data fixed for the slot's life; a dynamic in a field position is a `dyn` with a fresh slot per change.

*Identity* is by path for responses and by element value for state, and equality of elements is the tag and the plain fields, never the request event. Two slots with equal elements observe one state, supplied once. A tag with both parts has one slot and two identities: `Net.Socket(address="db:5432")` in two places is two conversations (each slot's sends and receives are addressed to its own path) over one connection (both slots read one `Link` state, because their elements are equal). The rule is the same for every row that has both a `responds` and a `state` column, and the Identity column says "path; value" for it.

*Creation*: nothing fires in the occurrence a slot is created. The runtime enforces half of this, since it has no response to deliver in a slot's creation occurrence and a firing from pure code at a path created in the same occurrence is an `UnknownPath` error; the World is bound to the other half by its obligation never to answer a request in the occurrence it was made. A state element has its value in its creation occurrence, from the snapshot as the occurrence opened; the World is bound to answer the oracle synchronously and consistently, and the runtime raises `UnsuppliedState` on the oracle's refusal.

*Teardown*: a response addressed to a slot that no longer exists is discarded, by the World under its obligation to send only to known paths, and a request fired by a slot torn down in the same occurrence is reported to the World with its path, performed if the World can, and its response discarded. Anything a World started on a slot's behalf and that cannot outlive the slot (a child process, a watch, a listener) is stopped under the World's cancel-on-teardown obligation. No row below is a trigger by presence or absence: appearing is a description, disappearing is the end of delivery, and anything a program wants done is a request it fires.

File paths are `Text` throughout, because the name `Path` is the type of positions in the element tree.

### Value types

The value types the tags refer to are declared once here and reused. A choice alternative with several payloads holds the tuple of them.

```prism
type Click = click
type Tick = tick
type Code = Code Int
type Instant = Instant Int                 # milliseconds since the World's epoch
type Duration = Duration Int               # milliseconds

type Point = record:
    x: Float
    y: Float

type Vector = record:
    x: Float
    y: Float

type Extent = record:                      # a width and height, the only size type: a window's size, a text's measure (what Gui.Metrics answers)
    w: Float
    h: Float

type Rect = record:
    x: Float
    y: Float
    w: Float
    h: Float

type Color = record:
    r: Float
    g: Float
    b: Float
    a: Float

type Font = record:                        # named in the value that draws it; the World resolves it
    family: Text
    size: Float

type ImageId = ImageId Nat                 # an opaque handle the World allocates

type Button = choice Primary | Secondary | Middle

type Pointer = choice:
    Moved Point
    Down Point, Button
    Up Point, Button
    Scroll Point, Vector

type NamedKey = choice Left | Right | Up | Down | Backspace | Delete | Tab | Enter | Escape

type CompositionEvent = choice:            # an input method's composition
    Start
    Update Text, Nat
    Commit Text
    End

type Key = choice:
    Char Char
    Named NamedKey
    Composition CompositionEvent

type WindowEvent = choice:                 # what a Gui.Window responds with
    Resized Extent
    CloseRequested
    Pointer Pointer
    Pressed Key

type Draw.Rect = record:                   # draw commands are values, not tags
    rect: Rect
    color: Color

type Draw.Text = record:
    origin: Point
    text: Text
    font: Font
    color: Color
    ime_at: ?Nat                           # offset into text where an input method's candidate window goes

type Draw.Image = record:
    rect: Rect
    image: ImageId

type Draw.Clip = record:                   # children drawn clipped to the rectangle
    rect: Rect
    draw: [Draw]

type Draw = Draw.Rect | Draw.Text | Draw.Image | Draw.Clip

type Frame = record:                       # one displayed frame
    now: Instant

type ChildInput = record:                  # what a Ui container responds with
    path: Path
    response: Response

type FsError = choice NotFound | Denied | TooLarge | Other Text

type Contents = choice:
    Unavailable FsError                    # the reason
    Known Text

type FsRequest = choice:
    Read
    Write Text
    Append Text
    Delete

type FsResponse = choice:
    Contents Text
    Written
    Unavailable FsError

type Listing = choice:
    Unavailable
    Known [Text]

type Stamp = choice:                       # a file's modification stamp as state
    NoStamp
    Modified Instant

type Link = choice:
    Connecting
    Connected
    Disconnected Text

type SocketEvent = choice:
    Received Text
    Closed

type HttpRequest = record:
    method: Text
    url: Text
    headers: Text ==> Text
    body: Text

type HttpResponse = choice:
    Answer Nat, Text ==> Text, Text        # status, headers, body
    Failed Text

type Accepted = Accepted Nat               # a connection id
type ConnEvent = choice:
    Received Text
    Closed
type ConnRequest = choice:
    Send Text
    Close

type SpawnRequest = record:
    program: Text
    args: [Text]
    env: Text ==> Text
    cwd: ?Text

type SpawnResponse = choice:
    Spawned Nat                            # the child's id, the World's to allocate
    NotFound
    Denied

type ProcessRequest = choice:
    Input Text
    CloseInput
    Signal Signal

type ProcessEvent = choice:
    Output Text
    Error Text
    Exited Code

type ProcessState = choice:
    Running
    Exited Code
    Gone                                   # no such child; the id is stale

type StdinEvent = choice:
    Chunk Text
    End

type WriteFailed = choice:                 # what Cli.Stdout and Cli.Stderr respond with
    Closed
    Failed Text

type Signal = choice Interrupt | Terminate | Hangup | WindowChanged

type Stored = choice:                      # a Store.Value's state
    Absent
    Present Bytes

type PutResult = choice:
    Stored
    Failed Text

type ErrorKind = choice Cycle | Overflow | UnknownPath | UnsuppliedState | DeferBackwards | Component | Build

type Span = record:
    file: Text
    line: Nat
    column: Nat
    name: Text                             # the binding's name

type Unsupported = unsupported
```

`Bytes` is the byte-string primitive. `Response` and `State` are the generic response and state types an interceptor sees in a catch-all arm; `Path` is the position type. `ErrorKind` is a closed choice on purpose: a program that matches on `Error`'s `kind` is exhaustive, and a new kind of failure is a change to the language, not to a World. `Error` itself is a tag (the last table below), so one value serves as the presence element a failed build leaves and as the response `Errors` delivers; `RuntimeError { kind, spans, occurrence, .. }` is the same record at the Rust boundary, where the runtime's `Outcome` lists the contained build failures.

### Screen: `Gui.*` and `Ui.*`

GUI is not a World concept. `Gui.*` is the primitive vocabulary every GUI World speaks: five tags, and a World that knows about screens accepts these and nothing more. `Ui.*` is the Prism GUI library's own vocabulary, handled by the library's interceptors; the rows are listed because the examples use them, and they reach a World only if that World accepts them (the Test World accepts every tag; a World wrapping a native toolkit may accept some). Under a World that accepts only the primitives, `Ui.Button` is an interceptor's business. The World does no geometry: layout, hit-testing, focus traversal and text editing are Prism over the five primitives. Draw commands (`Draw.Rect`, `Draw.Text`, `Draw.Image`, `Draw.Clip`) are plain values in a `Gui.Scene`'s `draw` list, never tags.

| Tag | Fields | Requests | Responds | State | Identity | Meaning |
| --- | --- | --- | --- | --- | --- | --- |
| `Gui.Window` | `title: Text`, `size: Extent` | | `WindowEvent` | | path | *Primitive.* One top-level window per slot; `size` is what the program asks for when it opens. Every pointer event over the window arrives here, and every key (`Pressed`), as does the size the platform gave (`Resized`) and `CloseRequested`, which the program answers by tearing the window down, or not. The library hit-tests against its own layout, routes pointer hits inward, and routes each key to the input whose name is the focus. |
| `Gui.Scene` | `window: Text`, `draw: [Draw]` | | | | path | *Primitive. Presence.* The draw list for the window of that title, as data in paint order, read by the World from the committed list after each turn. A picture that changes is a fresh slot per change, at the same tree position; nothing is in flight on a picture, so that is right. |
| `Gui.Focusable` | `name: Text` | | | | path | *Primitive. Presence.* The name of the focused region and nothing else; keys do not go here but to `Gui.Window`. It lets a World show the platform focus ring, place an input method candidate window and inform a screen reader. Which name is focused, what Tab does and where the caret is are holds in the library, which emits at most one `Gui.Focusable` at a time. No rectangle: the World does no geometry. |
| `Gui.Metrics` | `text: Text`, `font: Font` | | | `Extent` | value | *Primitive.* The width and height of a text in a font, answered synchronously from the oracle, so a layout has its measurement in the occurrence that asks. Every slot asking about the same text and font shares one answer. |
| `Gui.Frames` | | | `Frame` | | path | *Primitive.* One `Frame(now: Instant)` per displayed frame while the slot exists, for animation. Put the slot under a `dyn` over whether anything is animating, so that when nothing is, the World stops asking for frames. |
| `Ui.Label` | `text: Text` | | | | path | *Library.* Text at the position layout gives it. |
| `Ui.Button` | `label: Text` | | `Click` | | path | *Library.* A button; responds `click` in the occurrence the library routes the press to it, which is later than the press's own occurrence. |
| `Ui.TextInput` | `name: Text`, `text: Text`, `cursor: Nat` | | `Key` | | path | *Library.* A focusable editing region; the text and cursor are holds in the Prism widget around it, mirrored into the fields for drawing, and there is no second copy in a World. |
| `Ui.VBox` | `children: Path ==> Element` | | `ChildInput` | | path | *Library.* Vertical layout container; routes any child-addressed response inward by path through `ChildInput(path, response)`, one level per container. |
| `Ui.HBox` | `children: Path ==> Element` | | `ChildInput` | | path | *Library.* Horizontal layout container; routes as `Ui.VBox` does. |
| `Ui.Scroll` | `name: Text`, `height: Int`, `children: Path ==> Element` | | `ChildInput` | | path | *Library.* A clipped, scrollable container, a `Draw.Clip` and an offset the library holds. |

A `Gui.Scene` appearing is a description the World draws; a `Ui.Button` disappearing stops nothing but the delivery of clicks to a path that no longer exists.

### Files: `Fs.*`

| Tag | Fields | Requests | Responds | State | Identity | Meaning |
| --- | --- | --- | --- | --- | --- | --- |
| `Fs.File` | `path: Text` | `!FsRequest` | `FsResponse` | | path | One file handle's conversation: each request firing is one operation; each response answers one, in a later occurrence, in order. |
| `Fs.Contents` | `path: Text` | | | `Contents` | value | The file's contents as a value over time. `Unavailable(reason)` with a reason from the closed `FsError`: `NotFound` when it does not exist, `Denied` when it cannot be read, `TooLarge` when it is larger than the World's synchronous size cap, `Other(text)` for any other failure; pushed when the World sees a change. Shared by every slot with the same path. |
| `Fs.Dir` | `path: Text` | | | `Listing` | value | A directory's entries, sorted, as state. |
| `Fs.Watch` | `path: Text` | | | `Stamp` | value | The file's modification stamp as state: a real stat answers the oracle at creation (`NoStamp` or `Modified` with the time), so there is no unknown case; pushed when the World sees a change. A program that wants the contents re-reads through `Fs.Contents` or `Fs.File` when the stamp changes, so the World never needs to say it lost changes. |

`Fs.Contents` and `Fs.Watch` are both levels: one is the file, the other is the fact that it changed, and a program that only wants to know *that* something changed watches the stamp rather than holding the contents in memory.

### Time

| Tag | Fields | Requests | Responds | State | Identity | Meaning |
| --- | --- | --- | --- | --- | --- | --- |
| `Clock` | | | | `Instant` | value | The current time as state, pushed at most once per turn. Every clock in the program is one clock. Read it through a tag at an event; a derived dynamic over it changes once per push. |
| `Time.After` | | `!Nat` | `Nat` | | path | A request: each firing is a delay in milliseconds and asks for one wake-up that long from now; the response echoes the number back, so a slot with several delays in flight can tell the wake-ups apart by their delays; a program that needs more than that uses one slot per timer in a `for`. |
| `Time.Every` | `period: Duration` | | `Tick` | | path | A periodic stream: one `tick` per period while the slot exists; nothing at start; the phase restarts if the slot is rebuilt. Missed ticks are skipped: at most one `tick` per turn, never a backlog. |

A timer is not a request-with-response-on-creation: `Time.Every` fires its first tick one period after it exists, never in its creation occurrence, and `Time.After` fires nothing until asked.

### Network: `Net.*`

| Tag | Fields | Requests | Responds | State | Identity | Meaning |
| --- | --- | --- | --- | --- | --- | --- |
| `Net.Socket` | `address: Text` | `!Text` | `SocketEvent` | `Link` | path; value | One connection per distinct address, shared as state; each slot's sends and receives are its own conversation on it. `Closed` is a response; `Disconnected` is the state afterward. |
| `Net.Http` | | `!HttpRequest` | `HttpResponse` | | path | Outgoing HTTP: each firing is one request, each response answers one, in order. A widget that wants one in flight per key uses a `for`. |
| `Net.Listener` | `port: Nat` | | `Accepted` | | path | Listen while the slot exists; each accepted connection is a response with an id the World allocated. Stopped on teardown under the cancel obligation. |
| `Net.Conn` | `id: Nat` | `![ConnRequest]` | `ConnEvent` | | path | One accepted connection by id; sends and closes are requests, received data and the peer's close are responses. Created in a `for` over the accepted ids, torn down when the id leaves the map. |

### Processes: `Process.*`

The process tags are what every program has, a GUI program included: its own arguments, environment and exit, and the children it starts. They are not under `Cli` because a sandbox wants to give a subtree different ones, and because exit is a process thing whether or not there is a shell.

| Tag | Fields | Requests | Responds | State | Identity | Meaning |
| --- | --- | --- | --- | --- | --- | --- |
| `Process.Spawn` | | `!SpawnRequest` | `SpawnResponse` | | path | Spawning is a request: each firing starts one child; the response is `Spawned id`, `NotFound` or `Denied`, in a later occurrence. |
| `Process.Child` | `id: Nat` | `!ProcessRequest` | `ProcessEvent` | `ProcessState` | path; value | The child's existence is state keyed by the id the World allocated. Input, closing input and signals are requests; output chunks, error chunks and exit are responses; `Running`, `Exited` or `Gone` is the state. The program keys a `for` on the ids it was given; when no slot names an id, the World kills the child under its cancel-on-teardown obligation. There is no `detach` field. |
| `Process.Exit` | | `!Code` | | | path | Exit is a request: the run ends after the turn in which this fires, after the turn's writes. Two firings in one turn: the first in occurrence-then-list order wins. The element being present does nothing. |
| `Process.Args` | | | | `[Text]` | value | The program's arguments as state; one value for the whole program; an interceptor may supply a subtree different ones. |
| `Process.Env` | | | | `Text ==> Text` | value | The environment as state; pushed if the World can see it change. |

A run-to-completion helper (`@Run: fn SpawnRequest -> @$ProcessState`) is a library widget over `Process.Spawn` and `Process.Child`: fire the spawn on `@started`, hold the id, and build the child under a `dyn` over it. It is not a tag.

### The shell: `Cli.*`

The `Cli` tags are what a process has when it was started from a shell, and nothing else: four tags. A CLI run ends on `Process.Exit` or when nothing can cause another occurrence; stdin until its end, signals, in-flight requests and timers keep it alive, and args, env, the clock and a watch do not.

| Tag | Fields | Requests | Responds | State | Identity | Meaning |
| --- | --- | --- | --- | --- | --- | --- |
| `Cli.Stdin` | | | `StdinEvent` | | path | Chunks then `End`; read only while a slot exists. A chunk and the end are always in different occurrences, since a slot gets at most one response per occurrence. A slot that is dropped delivers nothing further, and the World keeps the partial chunk with the stream. |
| `Cli.Stdout` | | `!Text` | `WriteFailed` | | path | Each firing is written after the turn, in occurrence order then list order, then flushed. A write the World could not complete comes back as `WriteFailed` in a later occurrence. |
| `Cli.Stderr` | | `!Text` | `WriteFailed` | | path | As `Cli.Stdout`, to the error stream. |
| `Cli.Signals` | | | `Signal` | | path | Each signal received while a slot exists; with no slot, the OS defaults apply. |

### Persistence: `Store.*`

Chapter 7 declares the persistence vocabulary; the rows are repeated here so the table is complete.

| Tag | Fields | Requests | Responds | State | Identity | Meaning |
| --- | --- | --- | --- | --- | --- | --- |
| `Store.Value` | `key: Text` | | | `Stored` | value | A stored value as state: `Absent` or `Present` with the bytes, loaded into the World's snapshot when the key is first asked about, pushed when any writer changes it. |
| `Store.Put` | `key: Text` | `!Bytes` | `PutResult` | | path | Writes are requests; each firing stores one value under the key and is answered `Stored` or `Failed` in a later occurrence. |

### Every World: `Defer`, `Errors`, `World.Tags`

| Tag | Fields | Requests | Responds | State | Identity | Meaning |
| --- | --- | --- | --- | --- | --- | --- |
| `Defer` | | `!unit` | `unit` | | path | Answered in a later occurrence of the same turn, at most once per slot per turn; firing again after being answered in the same turn is a `DeferBackwards` error. A `Defer` reborn by a `dyn` at the same tree position counts as the same `Defer` for the once-per-turn bound; independent `Defer`s fired in one occurrence are answered together. How an interceptor answers a request itself. Every World provides it. |
| `Error` | `kind: ErrorKind`, `spans: [Span]`, `occurrence: Nat`, `turn: Nat`, `message: Text`, `owner: ?Path`, `path: ?Path` | | | | path | *Presence.* The error value: what a failed build or a quarantined subtree leaves in the element list, and what `Errors` delivers, so one `match` reads both. One tag with the kind as a field. |
| `Errors` | | | `Error` | | path | A stream a program may subscribe to: each runtime error the World would otherwise only report, with its kind, spans and occurrence. A failed `dyn` build does not fire the `dyn`'s event and is reported here. A program cannot catch an error any other way. |
| `World.Tags` | | | | `[Text]` | value | The tags this World accepts natively, as state, so a program can read it in occurrence zero and build under a `dyn` over it; what a library consults before deciding which interceptors to install. An interceptor supplies a sandboxed child a narrower one. The Test World's lists every tag. |

`Unsupported` is not a tag; it is the response a World gives, in the next turn, to a request on a tag it does not accept, so that a `Ui.Button` under a CLI World is a program-visible fact. A World component that crashes is a `Component` error through `Errors`, never an `Unsupported` response. A state tag the World does not accept is an `UnsuppliedState` error raised by the runtime at the slot's creation, on the oracle's refusal, since nothing later can give it a value; a presence tag it does not accept is reported and ignored.

### What the vocabulary excludes

No tag has a dynamic field; what would be one is a fresh slot per change, and the one case where continuity matters (a native text input) is not native. No tag fires on creation; what would be one is `@started` inside the program firing a request. No tag is a trigger by presence or absence; exit is a request, spawning is a request, and a child's death on teardown is the World's cancel obligation, not a meaning of the element. No draw command is a tag; the picture is a value in `Gui.Scene`, so a World reads it and an accessibility tree can read it too. No tag's state is `Pending` or `Fetching`; unavailability is a case of the state's own type (`Unavailable`, `Connecting`, `Disconnected`, `Absent`, `Missing`). No request can be answered in its own occurrence, by a World or by an interceptor; the shortest path from a request to its answer is one `Defer`. Each exclusion is what makes the corresponding law a law rather than a convention: nothing in the vocabulary can be used to break it.
