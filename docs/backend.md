# The back end: what the templates emit, and what `prism_rtl` provides

*Written by Claude, 1 November 2026, on the `claude-self-hosting` branch. Work in progress
and subject to change: implementers use their discretion. The specification is the
Compiler book, chapters 3 to 5, with Graph chapters 3 and 4; this document says how this
implementation lays them out under Jimmy's rulings of today, which replace
`decisions.md` D16. The library is `prism_runtime_lib/` (crate `prism_rtl`); the counter
and `@Normal` in its `tests/` are hand-written in the form below.*

## Why the line sits where it does

A compiled counter should read as the class a person would write: two button methods, a
hold, a label (Compiler ch. 3). A dynamic `Value` and an apply-with-lifting layer under
it make every program an interpreter run, and every type mistake a runtime fault. So:

- **Emitted code is typed Rust.** One Rust type per Prism type, typed closures, lifting
  resolved at compile time.
- **Each widget's graph is emitted.** Nodes, heights, propagation, holds, states and
  transitions come out of the templates per block, as Compiler ch. 3 and 4 describe.
- **The library is what is the same in every program:** numbers and collections, the
  World protocol and the Test World, the printer, the patch diff, `@Defer` and `pending`,
  the zombie collector, and the node primitives a template would emit word for word in
  every program (`Hold<T>` is twenty lines either way, Compiler ch. 5).
- **Templates decide nothing** (K9): every choice below is the class model's.

## The program

- **One `impl Program` per root**, around an `rt::Runtime` and the root node:
  - `build(args: &[Pod], ready) -> (Self, TickOut)`: `let mut cx = rt.build(ready)`, the
    root's constructor, `rt.close(cx)`; the arguments arrive as data (`FromPod`);
  - `tick(&mut self, input, ready)`: `rt.tick(&root, input, ready)`;
  - `compact(&mut self)`: `rt.compact()`.
- **The tick** (`Runtime::tick`, Graph ch. 4's order): last round's undone clean-ups; the
  started list's `Node::started` and the armed `Node::deferred`; the computed answers a
  host's supplier queued last tick (`Cx::answer_later`); answers through
  `root.answer(path, v)`; pushes through the push index; wakes by height; the read point
  moves past the tick; the build, swap, leave and patch lists; the patch diffed against
  the document last sent; requests sorted into document order; `pending` (a started node,
  an armed `@Defer`, or a computed answer due).
- **The node interface** (`rt::Node`): `wake`, `started`, `deferred`, `answer`, `push`,
  `build`, `swap`, `leave`, `patch`, `clean_up`, `edges`, `release`, each defaulting to
  nothing; the tick's methods take `self: Rc<Self>` and `&mut Cx`, so a node can put
  itself on a list.
- **A node is a plain struct of library cells behind an `Rc`.** Each cell borrows only
  inside its own methods, so no borrow is held across a firing (Compiler ch. 4, "The
  ownership layer") and node methods take `&self`.

## Types

- `Nat`, `Int` → `rt::Nat`, `rt::Int` (bignums with an `i64` fast path); `U8` → `u8`;
  `Float` → `f64`; `Bool` → `bool`; `Char` → `char`; `Text` → `rt::Text` (a rope);
  `unit` → `()`.
- A record → a `struct`, fields in declaration order; a choice → an `enum`, recursive
  payloads behind `Rc`; a newtype → a tuple struct; a unit type (`click`) → a unit
  struct.
- `?a` → `Option<A>`; `~a` → `Result<A, rt::Error>`; `((a, b))` → `(A, B)`; `[[a]]` →
  `rt::Array<A>`; `k ==> v` → `rt::Map<K, V>`.
- `fn a -> b` as a value → `Rc<dyn Fn(A) -> B>`, captures cloned in (values are cheap to
  clone); a named function → a Rust `fn`.
- `unbounded fn a -> b` as a value → `fn(A) -> Rc<dyn rt::Job<B>>`, its `Start`.
- `!a`, `$a` where they cross a node boundary or sit in a value → `rt::Event<A>`
  (`owner` + `Rc<Port<A>>`), `rt::Dynamic<A>` (`owner` + typed read + an update port of `()`, which says the value moved).
  Inside one node they are methods and fields, never values.
- `@a` as a value → `Rc<dyn Fn(Path, &mut Cx) -> (Rc<dyn Node>, A)>`.
- A trait → a Rust trait; `derive` → impls; generics → Rust generics, monomorphized by
  rustc (doubt MO).
- An existential `any ^Show` → `Rc<dyn Show>`.
- An element tag → a struct per tag holding its `Level` fields, with `ToPod`, and the Rust
  type of its output.
- A row kind other than `Fault` (`type Draw = row`) → an `enum` with a variant per tag of
  the kind, each holding its tag's struct; `Elem r`, a document's element → `crate::Elem`,
  a variant per element tag's `Level`; `Element r o`, a live element → `rt::Live<Elem>`,
  its level at the build (tag and plain fields) and its level over time, a `Dynamic<Elem>`
  (prismc/README.md, pick RV).

## Functions and lifting

- **A plain function** → a Rust `fn` taking references, its clauses one `match`.
- **Lifting is resolved by the checker**, so the emitted code is the lifted form: a pure
  transform of an event calls straight through; `$x on !e` samples at the read point;
  `$x + 1` is a getter, or an `rt::Cache` field when several readers share it; a lifted
  `match` or `if` is a computed behavior over its scrutinee.
- **`&` blocks** → native mutation of a fresh or unique value: `Rc::make_mut`, and the
  persistent collections' in-place updates (Compiler ch. 3, "Unique values").
- **Fault `~`**: a fallible expression is a `Result<_, rt::Error>` and `?` propagates;
  a failed guard gives `Prelude.Unmet` with its source text (D11). An exception that
  escapes a widget method returns `Err(rt::Fault)`, which fails the tick: the tick never
  happened, and the World decides (the CLI World prints and exits 1).

## A widget block

- **Storage.** A block → `struct Name` (Compiler ch. 4's naming: the binding's name,
  anonymous `anon_0`, suffixes per state) with `path: Path`, one `rt::Hold` per hold,
  one `rt::Stamped` per input a combinator reads, one `rt::Cache` per shared computed
  behavior, merged children's fields, and a `state` enum when the block has more than one
  state.
- **The constructor** `fn build(path, args, cx: &mut Cx) -> Rc<Self>` runs after
  propagation: holds take their initial values (`Hold::new(v, cx)`, stamped with the
  build tick), the hosts are linked, the builders run and the structural dynamics'
  snapshots are taken (Dynamic structure, below), then each place goes out with
  `cx.added(path.at(n), level.to_pod())`, its level the program's `Elem` (a constant for a
  place with no live fields, the place's getter `n.getter(cx)` otherwise, which the patch
  pass reads again after the tick); a place with
  a first value asks `cx.first_value(&level)`, a block that binds `@started` calls
  `cx.on_started(node)`, and nothing fires. A place in a hosted scope goes out nowhere: it
  is an entry of its host's document (Hosting, below).
- **Binding groups, sorted by prompt edges.** The class model takes the block's
  bindings as a graph whose edges are prompt (same-tick) reads; an edge through a hold's
  `then` is delayed and cuts nothing. A prompt cycle is the checker's error, so the graph
  sorts. The sort gives each combinator its distance from the node's inputs, and the
  bindings with the same inputs share one height (Q-B8). Heights are emitted constants.
- **Inputs.** An event input is a method. One that feeds only pure transforms calls
  straight through (`increment()` calls `delta()` calls the setter); one that feeds a
  combinator records itself in its `Stamped` and asks `cx.wake(HEIGHT, self)`.
- **`wake`** runs the node's combinators in distance order, once per height, by direct
  calls. A merge `<|` reads its left stamp first (CXb). The root wakes through the height
  buckets like every node (CXi).
- **Holds.** `$x <- init then !e` → the firing's method calls `x.set(v, cx)`; every read
  is `x.get(cx)`, before the tick during propagation and after it from then on, so the
  tag law needs no commit pass.
- **Marks.** A write marks its readers at the write: `cx.dirty(self)` for a place whose
  live field reads it, `Cache::mark` for a computed behavior in the node, a fire on the
  update port for a reader in another node, and the `_moved` flag of an `!Updated` node
  over it, with a wake at that node's height.
- **`!Updated $d`** over holds is a combinator decided in the wake, one above the writers
  of the holds `$d` reads: when its flag was set this tick it fires `$d`'s value after the
  tick, read under `Cx::after` (the read point moved to n+1 for the read), so the event
  half fires at n with the value after n while a sample in n reads the opening value
  (prismc/README.md, pick UP). A structural dynamic is written the same way, published at
  the start of the tick after its structure moved (Structural dynamics, below).
- **The patch pass.** `Node::patch` calls `cx.changed(path, level)` for each dirty place;
  `rt::Document::diff` drops a `Changed` whose `Level` did not move.
- **Answers.** `Node::answer` matches the path's segments: a position in this node calls
  the input method with the decoded value (`FromPod`); a segment that names a child node
  forwards the rest of the path to it. The path index is this dispatch.
- **Pushes.** An external behavior is a `Hold` written by `Node::push`; its place
  registers with `cx.listen(level, node)`, and the runtime's index finds every place with
  that element.
- **Requests.** A request field's firing → `cx.request(path, field, edge)`, `field` set
  when the tag has several.
- **The root's result** → `cx.result(Outcome::Value(..) | Outcome::Fired(..))`.
- **Demand.** A path that feeds only an output with no subscription is gated by its
  port's `demand()`, keeping its value at the fork point in a `Stamped` (Graph ch. 4,
  "Paths inside a widget node").

## Dynamic structure

- **`for k, $v <- $xs:`** → a builder node: a `Map<K, Rc<Row>>` of rows, a mark on `$xs`
  calls `cx.build_later(self)`, and after propagation `build` detaches the rows whose
  key left (`cx.removed` for its places, `cx.leave_later(row)`) and builds each new
  key's row at `path/[k]` from the values after the tick (R7); a key that leaves ends
  its row at the end of that tick (R8). The `for` is a projection of `$xs`: a row's
  `$v` is `$xs[k]`, a derived dynamic, never state. The block keeps the source as a
  getter and a row's `entry` reads it at the row's read point by its key, so a kept row
  reads its entry as any `$` (the opening value during propagation, the value after the
  tick from then on), nothing writes it, and `!Updated $v` is a combinator of the row
  one above the block's writers of the holds behind `$xs`, told of the write by the
  block (`parent_wrote_{m}`), firing the entry after the tick in the tick of the change.
  A `for` returning an event consolidates its rows' firings into `!(K ==> T)`, one a
  tick; one returning a dynamic gathers the rows' results into `$(K ==> T)`, a structural
  dynamic (below). (prismc/README.md, pick RE.)
- **`rebuild $x:`** → the same with one generation at `path/#g`; a `rebuild` returning an
  event is a builder and a switch in one path (round 46 RD).
- **Structural dynamics** (Semantics ch. 7, "Structural dynamics"; round 3 ST): a `for`'s
  gathered `$(K ==> T)` and a host's `$doc` are lawful dynamics one tick behind the
  structure they describe. Each is an `rt::Snapshot` field. The live computation
  (`rows_all_now`, `host_k_doc_now`) is read once per tick, in the node's construction step
  where the structure behind it has settled (a gathered dynamic after its builder; `$doc`
  after the child's builders, before the host's `for` over it), when a write marked it:
  the snapshot S(n). The getter (`rows_all`, `host_k_doc`) answers the dynamic, whose value
  after m is S(m−1) (after the build, the build's snapshot). When S(n) differs from S(n−1)
  (compared where the type has equality, else whenever taken) the node queues
  `{name}_publish` with `cx.answer_later`, once a tick, and at n+1, among that owed tick's
  first firings, it marks everything over the dynamic as a write of a hold does: the
  places over it are dirtied, the builders over it listed, and `!Updated` over it, or over
  a dynamic lifted from it, fires in n+1's wake with S(n) while every sample in n+1 reads
  S(n−1). The patch pass and the builders of n+1 read S(n). The host's own rows read the
  structure itself, `host_k_doc_direct` (S(n) from n's construction step on; a read of a
  marked snapshot there before it is taken is a panic, not a value), so a newborn child
  place gets its row in its birth tick (prismc/README.md, pick ST).
- **`@Switch $!a`** → a field with the current inner `Event` and its `Handle`, and an
  invalidation edge on the outer whose mark calls `cx.swap_later(self)`. `swap` pulls
  the outer after the tick; on a new inner it subscribes first, with no catch-up, then
  unsubscribes the old.
- **`@Flatten $$a`** → the same, with a current-inner `Hold`; the adoption puts the node
  on the started list, so it fires at n+1 (Graph ch. 4).
- **Leaving.** `leave` writes the `state` enum (live, detached, zombie, dead), turns
  holds into their `newest()` constants and the node's own events into `never`, then
  `cx.clean_up_later(self, zombie)`. A `for` row's entry, a projection whose key the
  source no longer has, becomes the constant it was as the leaving tick opened (read
  under `Cx::before`, the read point moved back to the tick), since the entry after that
  tick does not exist. In the compact round `clean_up` unsubscribes what the
  new state no longer reads; a zombie's `edges` and `release` let the collector break the
  one loop counts cannot (Graph ch. 4, worked examples).
- **States and transitions** are emitted: a `state` field per node, the transition as
  the code that writes it, and every method matching on it.

## Hosting

- **A host is Prism inside the same graph and the same tick** (Semantics ch. 9 as ruled).
  `@HostDocument @child, supplier` inlines the child into the node of the host's block (its
  owner) in a scope of its own: the child's places are numbered from 0 in that scope, keyed
  by their path relative to the child (`0`, `1`, `2/[k]/0`), and emit into no document.
  There is no protocol and no plain data at the host: the owner reads the child's places
  as values and answers them by calling their input methods.
- **The child's document `$doc` is a structural dynamic** of the owner (Dynamic structure,
  above): the construction step takes the snapshot `host_k_doc_snap` from `host_k_doc_now`,
  every child place as the program's `Elem` enum (the static places by their getters or
  constants, and under each `for` and `rebuild` of the child the rows' own
  `Row::document`, recursively), after the child's builders have run and before the host's
  `for` over it builds its rows. The host's `for` and its rows' entries read the structure,
  `host_k_doc_direct(&self, cx) -> Map<Path, Elem>`; the program's `$doc` is the getter
  `host_k_doc`, one tick behind. The live computation's reads are everything the child's
  places, builders, generations and rows read, so a write behind any of them marks the
  snapshot, lists the build and dirties the host's rows, and touches no reader of `$doc`;
  when the snapshot moved, `host_k_doc_publish` at the start of the next tick marks the
  owner's places over `$doc` and `!Updated $doc` fires there with the document after the
  tick before. A row of the child that writes a hold of its own tells the owner through
  `document_moved` → `host_k_moved`, which marks the same way.
- **The host is `for p, e <- $doc: supplier p, e`** in the owner: an ordinary builder
  (`host_k_rows_build`, the `for` template) over the getter, whose rows are `rt::HostRow`s,
  one per child place, built at `hostpos/[p]` when the key is in `$doc` after a tick and not
  before (R7), kept while it stays, and torn down at the end of the tick its key leaves
  (R8). The owner holds the host as `rt::Host<Elem, Answer, Owner>`: the supplier, the rows,
  the enclosing host when the line stands in a hosted scope, and the owner's two emitted
  methods, the document and the typed dispatch, linked to the node once it is made. The
  host holds its owner and each row holds the host weakly (the host outlives its rows), so
  no count cycle runs through a host and a torn-down host is freed with its rows (D5).
- **`e` is the live element**, `rt::Live<Elem>`: its identity (the level at the build) and
  its level over time, a `Dynamic<Elem>` whose read is the row's entry, the projection
  `$doc[p]` through the owner.
- **The way back is typed.** `crate::Answer` is emitted beside `crate::Elem` in `main.rs`:
  one variant per element tag carrying that tag's output type, an accessor per variant
  (`ui_button() -> Result<Click, Fault>`), and the decode of a World's answer by the
  element's tag (`rt::AnswerOf<Elem>`). The owner's `host_k_answer(place, &Answer, cx)`
  dispatches from a child place's path into the child's input method (a hold the place's
  element answers is written as a push writes it, with every mark a hold's sink makes), or
  to the row of a `for` or `rebuild` under it, whose `Row::deliver` carries it on. A
  variant of another tag is a fault of the tick.
- **The blueprint forms decide when an answer arrives** (`rt::Blueprint<Elem, Answer>`,
  built at the row's path with a `Sink<Answer>`, the typed delivery, in the host's
  `Scope`):
  - `@Element e` (`rt::element_blueprint`) is re-emission: a `Forward` node whose place
    `hostpos/[p]/0` goes out with `cx.added` (the identity's level), follows `e`'s level
    (the row is dirtied by the owner's marks; its patch fires the level's update port and
    the forward re-emits with `cx.changed`), and hands every answer and push the World
    sends it to the sink **in the same tick**, as the tick's inputs. Its first value is the
    World's. A host that forwards everything is observably not there. A forward in a hosted
    scope (nested hosts) emits nothing: it is an entry of the enclosing host's document
    (`HostRow::document`), and its first value is the enclosing host's.
  - `@ x` and a constant `@ $ v` (`rt::plain_blueprint`) are the first value once; `@ never`
    (`rt::never_blueprint`) answers nothing.
  - `@ !e` (`rt::event_blueprint`) and `@ $d` (`rt::dynamic_blueprint`, `$d` any dynamic
    of the owner, its first value read at the row's build, synchronously) are **computed
    answers, delayed**: each firing, and each tick with a change behind `$d`, is queued
    with `Cx::answer_later` and delivered to the child's place as an input of the next
    tick, which is owed (`pending`, like an armed `@Defer`); a dynamic's push samples it
    when delivered, so it carries the value after the tick of the change. The owner emits a
    getter per dynamic answer and fires its update port at every write behind it; the
    supplier reads the getter through an `rt::Link` to the owner, and the dynamic's value
    at the build while the owner is being made. A computed answer in flight to a place
    whose key left is dropped. Nested hosts add one tick per computing layer and none per
    forwarding layer.
- **First values.** A child place with a first value asks the host at its build
  (`host_k.first_value(&place, &elem, cx)`), before the owner's node exists: the host builds
  the supplier row for the place early and keeps it in the rows, where the builder finds it
  present. This is the only early construction; it is why `Host` reaches its owner through
  a link filled in after `Rc::new`.
- **Requests** of a hosted place go to the host (`host_k.request`): out with the forwarded
  element, or nowhere when the supplier answered the place.
- **`@HostSome`** is `@HostDocument` through `rt::forward_on_none` (`nil` forwards);
  `@HostList` is not lowered yet.

## Unbounded functions and `@Run`

- **A step machine per group of unbounded functions** (D6). Its frames are the cycle
  points of the canonical rule, a call of an unbounded function from an unbounded
  function, and the job implements `rt::Job<B>::cycle`, which runs bounded code to the
  next cycle point (`Turn::More(next)`) or the result (`Turn::Finished(b)`).
- **A tail call** leaves no frame (`Normalize`); any other call pushes the caller's
  continuation as a frame inside the job.
- **`@Run`** is Appendix B's Prism definition over `@Machine` and `@Defer`, compiled like
  any widget and inlined into its caller's node: the job is a `Hold`, each cycle fires
  `!turns`, a `More` arms the `@Defer`, a `Finished` fires the result.
- **`@Defer !e`** → an `rt::Defer<A>` field: `arm(v, node, cx)` in the firing tick, and
  in the next one `Node::deferred` reads `fired(cx)`. An armed `@Defer` or a non-empty
  started list sets `TickOut::pending`.

## Printing

- **`derive ^Show`** → `impl rt::Show`, one printer call per constructor:
  `p.choice("App", &[f, x])`, `p.record("Card", &[("rank", r), ..])`, `p.tuple`,
  `p.array`, `p.map`, `p.option`. The printer decides the parentheses.
- **A program's own `impl ^Show`** → `p.raw(&text)`.
- **`derive ^Pod`** (and every type that crosses to a World) → `impl rt::ToPod`,
  `impl rt::FromPod`; a type with its own `^Show` crosses as `Pod::Shown(text, structure)`,
  so a key prints `0/[e2]/0` and still matches structurally.
- **`Show x`** → `rt::show_text(&x)`.
- **The transcript** is the Test World printing the `Pod`s it receives with the same
  printer, in Conformance ch. 1's sections.

## The library's API

- `num`: `Int`, `Nat` (`checked_sub`, `div`, `modulo`, `from_int`).
- `collections`: `Text`, `Map`, `Array`.
- `pod`: `Pod`, `Path`, `Segment`, `ToPod`, `FromPod`, `mismatch`, `Error`, `Fault`.
- `print`: `Show`, `Printer`, `show`, `show_text`; `read`: `value`, `pattern`, `path`,
  `Pattern`.
- `cells`: `Tick`, `Hold`, `Stamped`, `Cache`, `Snapshot` (`mark`, `marked`, `take`,
  `take_compared`, `queue`, `published`, `get`, `current`), `Port` (`subscribe`, `unsubscribe`, `lien`, `release`,
  `demand`, `fire`, `kept`), `Handle`, `Demand`, `Event`, `Dynamic`, `Defer`, `Job`,
  `Turn`.
- `cx`: `Node`, `Done`, `Cx` (`tick`, `read_point`, `propagating`, `after`, `before`, `wake`, `build_later`, `swap_later`,
  `leave_later`, `dirty`, `on_started`, `arm`, `clean_up_later`, `added`, `removed`,
  `changed`, `request`, `answer_later`, `result`, `first_value`, `listen`), `Runtime`
  (`build`, `tick`, `close`, `compact`, `document`).
- `host`: `Host`, `HostRow`, `HostOf`, `Link`, `Rows`, `Scope`, `Blueprint`, `Supplier`, `Sink`,
  `Supplied`, `AnswerOf`, the blueprint forms (`plain_blueprint`, `never_blueprint`,
  `event_blueprint`, `dynamic_blueprint`, `element_blueprint`), `forward_on_none`.
- `doc`: `Document` (`diff`, `apply`), `Edit`, `Patch`, `Request`.
- `gc`: `Collector`.
- `world`: `Program`, `TickIn`, `TickOut`, `Outcome`, `ReadyState`, `ReadyMap`, `World`,
  `drive`; `test_world::TestWorld` (`program::<P>(name)`, `check`, `run`, `verify`);
  `scene::find_clickables`.

## Negative space

- **No dynamic `Value`.** *Buys:* type errors at rustc, not in a tick; a counter that
  reads as a class.
- **No graph engine in the library.** *Buys:* a static widget with no lists, no ports and
  no subscriptions; the library's size does not grow with the language.
- **No commit list, no clear pass.** Stamps do both jobs. *Buys:* K3 at every tick's end.
- **No heap of our own, no compaction, no checkpoints** (D4). *Buys:* `Rc` and rustc's
  drop glue in place of a copying collector; the price is doubts AB and GC.
- **No CLI World in this crate.** Worlds sit over the protocol in crates of their own,
  as `worlds/canvas` does. *Buys:* a library compiled programs link without a file system.

## Doubts

- **MO Monomorphization against one class per block** (Q-B9). The book keeps one node
  class per block so a checkpoint names blocks; with no checkpoints rustc monomorphizes.
  *First ruling (Claude's):* rustc's generics. *If otherwise:* trait objects per block.
- **AB A tick that aborts.** Conformance's AB goes on from the state before the tick;
  with no checkpoint there is none. *First ruling (Claude's):* the program is dropped,
  the Test World prints `aborted`, and later steps fail. *If otherwise:* `Program` gains
  a snapshot, or the World rebuilds and replays the input log.
- **GC The collector's watch list.** Trial deletion runs over zombies whose count
  dropped; `Rc` has no hook for that, so every live zombie is watched until it dies.
  *First ruling (Claude's):* watched until death. *If otherwise:* zombies count their
  own releases, and the round visits only those.
- **FV The sharing rule for first values.** Graph has a newborn take an equal older
  place's value. *First ruling (Claude's):* the ready state with the tick's pushes on top
  stands in, since a World's pushes keep its ready state equal to its places.
- **SC The scope.** A host sees the elements its child emits typed, as the program's
  `Elem`, through the owner's document getter, and a row of a hosted child takes the
  enclosing host (`Rc<dyn HostOf>`) as a parameter of its `build`. *First ruling
  (Claude's):* the scope is known at compile time for every emitted node and rides on the
  rows as a parameter; only the library's `HostRow` asks its host for it (`Scope`). *If
  otherwise:* a `Scope` parameter on every constructor.
- **SZ The size.** The library proper (everything but the Test World, its reader and
  the scene) is about 2,000 lines; the Test World adds about 1,300. *First ruling
  (Claude's):* both stay in this crate, as the brief asks. *If otherwise:* the Test World
  moves to `worlds/test`, and `prism_rtl` is the library alone.
