# The back end: what the templates emit, and what `prism_rtl` provides

*Written by Claude, 1 November 2026, on the `claude-self-hosting` branch. Work in progress
and subject to change: implementers use their discretion. The design it follows is the
Compiler book, chapters 3 to 5, with Graph chapters 3 and 4; this document says how this
implementation lays them out under Jimmy's rulings of today, which replace
`decisions.md` D16. The library is `runtime/rust/` (crate `prism_rtl`); the counter
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
- **A local is its binder's name and id** (prismc/README.md, pick UN): the model's
  `XVar` of a program's local is `name_<kind><id>`, the id the checker gave the binder and
  recorded at its every use, so no two locals of a declaration share a spelling and none
  hides a function, a runtime name or a keyword.
- **A node is a plain struct of library cells behind an `Rc`.** Each cell borrows only
  inside its own methods, so no borrow is held across a firing (Compiler ch. 4, "The
  ownership layer") and node methods take `&self`.

## Types

- `Nat`, `Int` → `rt::Nat`, `rt::Int` (bignums with an `i64` fast path); `U8` → `u8`;
  `Float` → `rt::Float` (an `f64` whose `==` is bit identity and whose `Ord` is
  `total_cmp`, so it is `Eq`, `Ord` and `Hash`: prismc/README.md, pick BF); `Bool` → `bool`; `Char` → `char`; `Text` → `rt::Text` (UTF-8: a
  byte range of a shared `Rc<str>`); `Cursor` → `rt::Cursor` (the text and a byte
  index); `unit` → `()`.
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
- A trait → a Rust trait; `derive` → impls (`^Eq` → `PartialEq, Eq`, `^Ord` →
  `PartialOrd, Ord`, whatever the fields: every `^Eq` is reflexive and every `^Ord`
  total); generics → Rust generics bounded by `Eq` and `Ord` for `^Eq` and `^Ord`,
  monomorphized by rustc (doubt MO).
- An existential `any ^Show` → `Rc<dyn Show>`.
- An element tag → a struct per tag holding its `Level` fields, with `ToPod`, and the Rust
  type of its output.
- A row kind other than `Fault` (`type Draw = row`) → an `enum` with a variant per tag of
  the kind, each holding its tag's struct; `Elem r`, a document's element → `crate::Elem`,
  a variant per element tag's `Level`; `Element r o` → `rt::Element<Elem>`, either a live
  element, `Live` (its level at the build, tag and plain fields, and its level over time, a
  `Dynamic<Elem>`), or a description, `Described` (prismc/README.md, pick RV; gap LE).
- An element tag also → a description record, `{Tag}_desc`: each field as an application
  gives it, a `$` field its `Dynamic`, an event field its `Event`, a plain field its value
  (`DescDefOf`, from the tag's declaration).

## Functions and lifting

- **A plain function** → a Rust `fn` taking references, its clauses one `match`.
- **Lifting is resolved by the checker**, so the emitted code is the lifted form: a pure
  transform of an event calls straight through; `$x on !e` samples at the read point;
  `$x + 1` is a getter, or an `rt::Cache` field when several readers share it; a lifted
  `match` or `if` is a computed behavior over its scrutinee.
- **`?` and `~` at the plain layer** (Type System ch. 4): the checker notes each
  application, operator or field read it lifts over a `?` or `~`, with the operands it
  peeled (`TProgram.lifts`); lowering makes it a `match` of each operand in turn, left to
  right, whose present arm is the plain expression over the peeled values, wrapped back
  (`match get(ps, k) { Some(o) => Some(o.a.clone()), None => None }`), and whose absent arm
  is `None` (the leftmost `Err`). An unbounded call lifted so is a frame that matches first (below).
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
  event is a builder and a switch in one path (round 46 RD). One returning a dynamic,
  `$$a`, gives the current generation's result as an `rt::Dynamic` (the generation's
  `result_dynamic`), a structural dynamic of the block (below): the build keeps the
  newborn's dynamic and marks the snapshot, taken after the generation is built.
- **Structural dynamics** (Semantics ch. 7, "Structural dynamics"; round 3 ST): a `for`'s
  gathered `$(K ==> T)`, a host's `$doc` (its outer, and each place's entry, below) are
  lawful dynamics one tick behind the structure they describe. Each is an `rt::Snapshot`
  field. The live computation (`rows_all_now`, `host_k_doc_now`) is read once per tick, in
  the node's construction step where the structure behind it has settled (a gathered
  dynamic after its builder; `$doc` after the child's builders, before the host's `for` over
  it), when a write marked it: the snapshot S(n). The getter (`rows_all`, `host_k_doc`) answers the dynamic, whose value
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
- **`for k, $v <- $each:` over a map of dynamics** (`$(k ==> $v)`: a host document's
  entries `$(Path ==> $Elem c)`, `@PerKey`'s output, a `for` gathering `rebuild`s'
  dynamics; ruling FV, prismc/README.md pick FV) → the row's binder is its entry
  flattened (`ForLine` makes it an `EntryFlatten` when the source's value type is a
  dynamic): the current inner `Hold<Option<Dynamic<V>>>` (`{v}_inner`), set at the row's
  construction from the `value` its builder hands it, and an `rt::Follow` (`{v}_follow`),
  the subscription to the inner's update port, which calls `{v}_moved`: what reads `$v` is
  marked as for a write of the inner, in the tick the inner moved. The row's entry is not
  among its place reads (`RowPlaceReads`), so a write of the outer wakes no kept row; the
  `for`'s builder (`ForBuild`, both printers) walks the kept keys and compares each one's
  dynamic in the new source with the one in the map the rows were last built from
  (`{name}_from`; a `Dynamic` equals only itself) and calls that row's `{v}_replaced`
  (`FlattenMethods`; `ForDef.replaced` lists them), which queues `{v}_adopt` with
  `cx.answer_later`, so the adoption is among the next tick's first firings, which is owed
  (rulings VL, TK). The row is kept, never rebuilt for a replaced dynamic (Semantics ch.
  11). A `$$d` binder with `@Flatten $$d` below it is a `mark-stack` in the checker
  (`tests/checker/neg/for-double-dynamic`); `sem-ch04-for-replaced-inner`.
- **`@Switch $!a`** → a field with the current inner `Event` and its `Handle`, and an
  invalidation edge on the outer whose mark calls `cx.swap_later(self)`. `swap` pulls
  the outer after the tick; on a new inner it subscribes first, with no catch-up, then
  unsubscribes the old.
- **`@Flatten $$a`** over a `rebuild`'s dynamic → a current-inner `Hold<Option<Dynamic<A>>>`
  (`{name}_inner`), adopted at the build; when the outer is published (n), the adoption
  (`{name}_adopt`) is queued with `cx.answer_later` among the first firings of n+1, which is
  owed, and writes the hold, so the `@Flatten` reads the new inner after n+1 (Semantics
  ch. 7: after n, the inner its outer held after n-1). The getter samples the inner at the
  read point; a write behind the current generation's result marks the `@Flatten`'s
  readers (`{gen}_result_moved`), and that method also fires the update port of the
  `Dynamic` kept for the result (`{gen}_result_live`), so a follower outside the node
  sees it move: a `for` gathering `$$r`s into `$(k ==> $r)`, whose rows' `$v` follow
  their inners (`sem-ch04-for-replaced-inner`).
- **`@Flatten $$a`** over any other dynamic of dynamics (`HeldFlatten`; the coordinator's
  ruling FV) → the outer as the live `Dynamic<Dynamic<A>>` its line made (`WholeArgs`,
  `whole_{x}_dyn`), a current-inner `Hold<Option<Dynamic<A>>>` and two `Follow`s: the
  outer's update port (`{name}_outer_moved`) queues `{name}_adopt` with
  `cx.answer_later`, owed, as the `rebuild` form does, and the adoption takes the inner
  the outer holds after n, follows the inner's port (`{name}_moved`, a write of the
  inner) and fires. No dynamics are compared; an adoption of the same inner fires again
  (`sem-ch07-flatten-held`, `sem-ch07-flatten-mapped-constants`). A live dynamic's port
  fires once a tick however many writes lie behind it (`{name}_once`).
- **`!MergeAll` of a plain list that is not a literal** (`MergeListEvent`) → a
  `MergeList<T>` field (prism_rtl, both runtimes) that hears each `Event` of the list,
  read once the node is made (`{nm}_events`), into a stamped input and tells the node
  (`{nm}_in`, the wake asked); the wake takes the values fired this tick in the list's
  order (`GnMergeList`). Over a dynamic list `!MergeAll` is lifted, a `$![a]` (Type
  System ch. 4), whose inners are `merge_all` event values: prism_rtl's own `MergeNode`
  decides each tick's firing, owned by the event.
- **`@Switch $!a`** over any dynamic of events that is not a `rebuild`'s (`HeldSwitch`) →
  the outer's live `Dynamic<Event<A>>`, made once the node is (`{name}_outer`), a port node
  `{name}` of the graph, and a `Listen<A>` (prism_rtl) that keeps the current inner event
  whole and tells the port node. The outer's update port (`{name}_ofollow`) calls
  `cx.swap_later`, and `swap`, after the tick's propagation, listens to the inner the
  outer then holds: heard from n+1, with no catch-up (`sem-appB-merge-all-switched`).
- **A generation's own builder over a name of its block** (gap DY): the generation hears
  every block name its `!Updated` nodes or its own builders read (`RowHears`), through
  the block's `{gen}_wrote_{m}_port` (`sem-ch07-generation-rebuild`). A write behind a
  name that the `rebuild`'s source reads too lists both the block and the old generation
  for the build, in either order. The generation's `build` runs only while `current()`
  holds: it has not left (`left`, set as its places are torn down), its block's
  `{gen}_stale` is clear, and a generation block is current in turn. So a generation the
  tick replaces builds nothing (`sem-ch07-paged`).
- **`!Updated` in a generation over its block's names.** The block holds its current
  generation as a `Row`, which names no method of it, so each name the generation hears
  is a port of the block, `{gen}_wrote_{m}_port`, fired where a `for` row's
  `parent_wrote_{m}` is called; the generation subscribes to it (`GenHears`,
  `subscribe_row`) and hears it as `parent_wrote_{m}` (`sem-ch07-generation-updated`).
- **`$$a` is a dynamic of `Dynamic`s.** `Peel` takes one time layer off, so a `$$a`
  binder's value is a `Dynamic<A>`: a hold of dynamics (`$$cur <- $a then ..`) takes its
  initial value whole; a function is whole when a call takes time below the top of what
  it gives (`$Sel (True on !swap), $a, $b`, a `!$Nat`), and such a lifted call takes its
  sigiled arguments whole (`ArgsFor`); `$ $q` is a map of `$q` to constants (Type System
  ch. 4: a bare type variable lifts), and a lifted `if` or `match` maps its arms' results
  too (`MappedOperands`).
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
- **A widget of several clauses in a static position**, and **a building `match` or
  `if`** (Semantics ch. 4: the arm is fixed for the life of the block and adds no
  segment), are one generation built once, transparent: the row the arguments (or the
  arm's pattern and guard) choose stands at the block's path, its places at the block's
  positions, the widest clause's or arm's width reserved (pick CS), answered at those
  positions in the World's scope and in a hosted one. An arm of a `match` reads the
  block's names through its `parent`; a clause reads only its own, a `$` parameter being
  the caller's dynamic read through the parent (`ClauseDynArg`), never its value at the
  build. The result:
  - an event fires the line's event;
  - a dynamic (`result_dynamic`) is kept by the block in `{gen}_res` and read through the
    line's getter; a write behind it in the row calls the block's `{gen}_result_moved`,
    which marks the block's readers (`KeptDynResult`);
  - a plain one, which the block reads in its build: the clause's row is built in the
    block's own build, before its node, at the line's point (`GenDef.early`), holding
    the block through a `prism_rtl::Link` linked once the node is made, and hands back
    its `result_value` with itself (`EarlyClauseLine`).
- **A building `match` or `if` whose arm ends on a plain result splits its block's build**
  (Semantics ch. 4 and ch. 8: the arm reads the block's names, and places are born in
  source order). The arm's row is built once, after the node, as any arm's; the node
  keeps its `result_value` in `{arm}_res` from then on. That result is late (`WrLate`): an
  expression of the node's methods, with no value among the build's locals. The lines
  below the line that binds it are one more generation of the block, built once after
  the arm's, transparent, at the block's next positions: the line's pattern is its
  binder, the kept result its value, and it reads the block's names through the node as
  an arm does (`SplitBlockInto`). Its result is the block's: an event's or a dynamic's as
  an arm's is, a plain one late again. So a late result travels up: a caller's line that
  binds it splits the caller's block in turn (`IsLateExpr`), a row keeps it behind its
  `result_value`, and the root tells the World in its build once its rows are built.
  A line above the split does not read a name bound below it.
- **A widget's plain result bound by a pattern** (`((Extent(wide=w, high=h), ((d, t)))) <-
  @Measure 3`): a name or a tuple of names takes its parts as locals of the build, each
  moved into its field (`BindPlainResult`); any other pattern is matched once in the build
  against the result, and each name it binds is moved into the field its prebinding made
  (`BinderNames`, `BindPatternResult`).

## Hosting

- **A host is Prism inside the same graph and the same tick** (Semantics ch. 9 as ruled).
  `@HostDocument @child, supplier` inlines the child into the node of the host's block (its
  owner) in a scope of its own: the child's places are numbered from 0 in that scope, keyed
  by their path relative to the child (`0`, `1`, `2/[k]/0`), and emit into no document.
  There is no protocol and no plain data at the host: the owner reads the child's places
  as values and answers them by calling their input methods.
- **The child's document `$doc` is `$(Path ==> $Elem c)`** (Jimmy, 6 November 2026): an
  outer dynamic that moves when a place comes or goes, and one dynamic per place that moves
  when that place's element changes. Both levels are structural (Dynamic structure, above).
  - **Each hosted place is an `rt::Entry<Elem>`**, a field of the node the place is in (the
    owner for the child's own places, a row for a row's), made before the node (a
    first-value query hands it to the host) and holding the place's own snapshot. A live
    place's entry takes its first snapshot at the build in its scope's turn (after the
    scope's builders, before the host over it builds its rows); a constant place's at once.
    A write behind a hosted place dirties its node, as for a World place, and the patch
    pass takes the entry (`Entry::take`, compared on the level, a dynamic it holds equal
    only to itself: doubt EQ); a take that moved it dirties the host rows reading the place
    as it stands and queues its publication, so the entry's dynamic, which samples the
    snapshot of the tick before, fires its update at the start of the next tick, owed.
  - **The outer is the snapshot `host_k_doc_snap` of an `rt::HostDoc<Elem>`**: each place's
    dynamic by its path relative to the child (`doc`, what the program's `$doc` reads,
    `host_k_doc`) and each place's entry (`places`, what the host's own `for` reads,
    `host_k_doc_direct`). `host_k_doc_now` gathers the entries (the child's own places,
    and under each `for` and `rebuild` of the child its rows' `Row::document`, recursively;
    a host's own rows give their forwarded places' entries, `HostRow::entries`). Nothing a
    place reads marks it: a builder or a generation of the child whose rows came or went
    calls `host_k_moved` (or, in a hosted row, `document_moved`, which its parent carries
    to the host), which marks the snapshot and lists the build; it is taken after the
    child's builders, compared by the entries it holds, and published the tick after, when
    `!Updated $doc` fires. A value change of a place never moves the outer.
- **The host is `for p, e <- $doc: supplier p, e`** in the owner: an ordinary builder
  (`host_k_rows_build`, the `for` template) over the getter, whose rows are `rt::HostRow`s,
  one per child place, built at `hostpos/[p]` when the key is in `$doc` after a tick and not
  before (R7), kept while it stays, and torn down at the end of the tick its key leaves
  (R8). The owner holds the host as `rt::Host<Elem, Answer, Owner>`: the supplier, the rows,
  the enclosing host when the line stands in a hosted scope, and the owner's emitted typed
  dispatch, linked to the node once it is made. The
  host holds its owner and each row holds the host weakly (the host outlives its rows), so
  no count cycle runs through a host and a torn-down host is freed with its rows (D5).
- **`e` is the live element**, `rt::Element::Live`: its identity (the level at the build),
  its level over time (a `Dynamic<Elem>` reading the place's entry as it stands, whose
  update is the host row's patch), and the entry itself, which a forward into an enclosing
  host's document hands on as the same place.
- **An element applied as a plain value is a description** (gap LE; Type System ch. 2,
  "Names and sigils"), `rt::Element::Described`: the tag's `_desc` record, its level over
  time mapped from its `$` fields, and each event field's firings as the field's request
  (`ElementValue`). It reads nothing until it is placed. `@Element d` places it
  (`described_blueprint`): its level is read at the build and sent (`cx.added`), or taken
  into an entry of its own in a hosted scope (`Supplied.entry`, which the enclosing host's
  document holds as the row's place at `0`, `HostRow::entries`; the entry takes the level
  again each time it moves, and an enclosing host's answer there goes to the child's
  place). In the World's scope each move of its level sends it again, and its answers and
  pushes are the child place's. A description with an event field placed in a hosted
  scope is a fault at the build (doubt DQ).
- **A pattern sees either form as the tag's description** (`DescView`,
  `rt::element_view`). A description of the tag matches as it is. A live element of the tag
  matches as the description of its level, each `$` field its level projected. So a `$`
  field binds a `$T` in both forms. A function with an element parameter takes its time
  whole (`HasElementParam`), so its callers read what it gives.
- **`@Element (Tag(..))` in a block** is the element statement at the line's own position
  (`ElementCallLine`; Semantics ch. 4).
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
    (the row is dirtied by its place's entry when a take moved it; its patch fires the
    level's update port and the forward re-emits with `cx.changed`), and hands every answer and push the World
    sends it to the sink **in the same tick**, as the tick's inputs. Its first value is the
    World's. A host that forwards everything is observably not there. A forward in a hosted
    scope (nested hosts) emits nothing: its place is the child's own entry in the enclosing
    host's document (`HostRow::entries`), and its first value is the enclosing host's.
  - **A level that holds dynamics** (a container whose children are its document) goes to
    a World deep: `Elem` implements `rt::Level`, each dynamic sampled at the read point
    (`level_pod`), and a World place over one keeps an `rt::Watch` on the dynamics it holds,
    so a move of a nested entry sends it again (doubt WD). A forward re-emits through the
    same `level_pod`.
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
  (`host_k.first_value(&place, &elem, &entry, cx)`), before the owner's node exists: the host builds
  the supplier row for the place early and keeps it in the rows, where the builder finds it
  present. This is the only early construction; it is why `Host` reaches its owner through
  a link filled in after `Rc::new`.
- **Requests** of a hosted place go to the host (`host_k.request`): out with the forwarded
  element, or nowhere when the supplier answered the place.
- **`@HostSome`** is `@HostDocument` through `rt::forward_on_none` (`nil` forwards).
- **`@HostList @child, !answers, supplier`** is `@HostDocument` whose owner also hears
  `!answers` (`Host::hear`): each firing's map hands the place at each key its answer,
  decoded by the place's element as a World's answer is (`AnswerOf::decode`, a tag that
  does not match is a fault of the tick), as a computed answer, the next tick's input
  (`HostRow::answer_later`); a key with no place is dropped. `Answer c` is plain data
  (`Pod`), made by the prelude's `AnswerOf`; `Wired c` is `Elem c` (prismc's first ruling HL).
- **A bundle in a hosted scope** (a tag whose output is plain parts and one event part):
  `crate::Answer` has a second variant per bundle tag, `{Tag}First`, its plain parts; the
  place's first value is that variant (`{tag}_first()`), from a supplier's
  `@ ((x, !e))` (`rt::bundle_blueprint`: `x` once, each firing of `e` a tick late) or from a
  World's first value to a forwarded element (`AnswerOf::decode_first`); its answers are
  the event part's variant, as in the World's scope.
- **A named widget function as a supplier** (`@HostDocument @child, @Supply`): a widget
  function shaped as a supplier (a parameter `Element c o`, the result `@o`) whose clauses
  build no place (`@ x`, `@Element e`, a `match` or `if` of them, after `let`s) is emitted
  as a function giving the blueprint (`IsValueWidget`), which the supplier, a lambda calling
  it, or a value naming it calls. Any other widget function used as a value is refused.

## Local functions, `@Machine`, durations

- **A local function** (`fn Add a, x = ..` in a block) is inlined where it is called, its
  parameters bound to the arguments (a pattern parameter matched against its argument),
  and is a closure of its clause where it is named as a value, at the type it is used at
  there; a generic one as a value, and one that calls itself, are refused.
- **`@Machine step, s, !e`** calls a named step by its path; any other step is a function
  value made at the build and kept in a field (`{m}_step`).
- **A duration literal** is `Duration(ns)`, its signed nanoseconds (`DurationNanos`); in C++
  `prism_rtl::Nanos`, an `__int128`, built from `long long` parts (`CppNanos`). Both
  runtimes print and read minutes as `min`, the literal's unit.

## Unbounded functions and `@Run`

- **A step machine per group of unbounded functions** (D6). Its frames are the cycle
  points of the canonical rule, a call of an unbounded function from an unbounded
  function, and the job implements `rt::Job<B>::cycle`, which runs bounded code to the
  next cycle point (`Turn::More(next)`) or the result (`Turn::Finished(b)`).
- **A tail call** leaves no frame (`Normalize`); any other call pushes the caller's
  continuation as a frame inside the job. A call lifted over `?` or `~` matches its peeled
  arguments first: with all present the job is entered and its result, wrapped, resumes the
  source; an absent one resumes it at once with `nil` or the leftmost `Err`
  (`LiftedFrame`).
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
- `regex`: `Regex` (`new`, `lit`, `parse`, `match_at`, `find`, `matches`, `split`),
  `RegexReplacement` (`new`, `lit`, `replace`), `Group`, `Match`, `RegexError`,
  `RegexPart`. A literal `%/p/f` is `Regex::lit(p, f)` and `%/p/r/f`
  `RegexReplacement::lit(p, f, r)`, `p` and `r` as written between the delimiters, each
  built once in a `thread_local!` and cloned at each use; the C++ runtime's spelling is
  `Regex::lit`, `RegexReplacement::lit` in a function-local `static`, and `compile` for
  `new` (runtime/cpp/README.md, "Regular expressions").
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

## The C++ side

`prismc compile --target cpp` prints the same model as C++20 over `runtime/cpp` (the C++
`prism_rtl`, whose README maps each Rust type of the library to its C++ form), through
prismc/emit/cpp/ and its templates (`Emit.Cpp.Templates`). Everything above holds construct for construct;
what differs is how C++ spells it.

- **The program** is `Makefile`; per module, in its directory (`src/DIR/`, `check/env`
  for `Check.Env`), the headers `decl.hpp` (its types declared),
  `types.hpp` (its value types, choices with their cases among them, each after the types
  it holds by value), `module.hpp` (prototypes, nodes and jobs) and, when it has templates,
  `generic.hpp`, each including only the
  layers of the modules it needs (prismc/README.md, pick MH); `src/elem.decl.hpp` and
  `src/elem.types.hpp` for `crate::Elem` and `crate::Answer` (declared, and defined with
  their cases); `src/programs.hpp` (the
  program classes); `src/main.cpp` (`Elem`'s and `Answer`'s definitions, `test_world()`,
  `main`), `src/prelude.hpp` and the modules' definitions in translation units,
  `src/unit_N.cpp`, several small modules to a unit (prismc/README.md, pick SF). The
  modules are the namespaces `crate::check::env`, nested as their names are, and every
  reference to another module's item is from `crate::`. A program class has the static `build(args, ready)`
  and the virtual `tick` and `compact`; `main` runs `test` under the C++ Test World and a
  `@main` root under the C++ CLI World (worlds/cli/cpp).
- **Types.** A record, a tag's `Level`, a newtype and a unit type are structs (fields in
  order, designated initializers); a choice and a row kind's values are a class holding a
  `std::variant` of nested case structs (`Term::Var`) by value, as the Rust enum is; a
  recursive payload (`Rc<T>` above) is a `prism_rtl::Rc<const T>`, the runtime's one `Rc`,
  whose `==` and `<=>` are its value's exactly when the value has them, the pointers
  compared first as Rust's `Rc<T: Eq>` does, so that a payload
  compares, orders, prints and goes out as plain data by its value, as Rust's `Rc<T>` of a
  value does, and a node's `Rc` compares not at all (identity is `ptr_eq`; prismc/README.md,
  pick CH). The model's `*x` is `*x` where the printer knows `x` is an `Rc`, `x` where it
  knows `x` is a reference, and `prism_rtl::deref(x)` where it knows neither (pick PT). `?a` is
  `std::optional`, `~a` `Fallible`, a tuple `std::tuple`, `Rc<dyn Fn>` `prism_rtl::Fn`, a
  job `JobRef`; `Float` is `prism_rtl::Float`, a `double` whose `==` is bit identity and
  whose `<=>` the `std::strong_ordering` of IEEE's totalOrder, so the defaulted comparisons
  of a struct, a variant or a tuple holding one are bit identity and total too (pick BF).
  A node is a class deriving from `Node` (or the prelude's `Row`), held by
  `prism_rtl::Rc` (plain counts, one thread); its constructor is `make` and its methods
  are members.
- **Expressions.** A block, a `match`, an if-let chain and a labelled block are GNU
  statement expressions (g++ and clang++ both take them); a `match`'s value is a
  `prism_rtl::Slot` of the type the model gives it, set by the arm that holds, which then
  leaves by `goto`, except that a function's body ending in a `match` returns from each
  arm instead (pick CB of prismc/README.md); `break 'l v` sets the label's slot and jumps. `?` is `TRY(..)`, a GNU
  statement expression too, which moves out of its operand: lowering tries only
  temporaries, and refuses `?` of a place. A
  pattern tests with `prism_rtl::is<Case>` and binds `prism_rtl::as<Case>(v).aN`.
- **What the library adds for the emitter** is `prism_rtl/lang.hpp`: `Slot`, `TRY`, the
  pattern helpers, `rc` (`Rc::new`: a value as an `Rc<const T>`, a node or a cell as an
  `Rc<T>`), `ptr_eq` (rc.hpp: `Rc::ptr_eq`),
  `share`/`weaken`/`upgrade`, `Fn`, `str`/`cat` (`format!`), `deref` (the model's `*x` of
  unknown type, and Rust's auto-deref of a cell), `host_new`, and the constructors whose type the context gives (`make_new`,
  `snapshot`, `finished`).

## Regular expressions

The types, the literal and the prelude's functions (Semantics ch. 2 and Appendix B,
Regular expressions; the dialect, `claude/regex-dialect.md`, checked by prismc before
tick 0 and again by each runtime).

- **Types.** `Regex` → `prism_rtl::Regex` and `RegexReplacement` →
  `prism_rtl::RegexReplacement` (`OutRegex`, `OutReplacement`), opaque, an `Rc` of the
  compiled form in both runtimes; `Group` and `Match`, records the prelude declares, are
  the runtime's own structs (Rust: `pub use prism_rtl::{Group, Match}` in the emitted
  prelude; C++: seen through `using namespace prism_rtl`), fields as Appendix B names them.
- **A literal is built once and shared.** `%/p/f` → Rust
  `{ thread_local!(static R: Regex = Regex::lit("p", "f")); R.with(Regex::clone) }`,
  C++ `([]() -> const Regex& { static const Regex r = Regex::lit("p", "f"); return r; }())`;
  `%/p/r/f` the same with `RegexReplacement::lit("p", "f", "r")`. The pattern and the
  replacement are the texts as written between the delimiters (`\/` stays `\/`), which
  the runtime reads by the dialect. Each literal site is its own static: a Rust block's
  item, a C++ lambda's function-local static (initialized on first call, thread-safe,
  alive to exit; a copy holds its own count, so no destruction order matters). The
  arguments are string literals of static storage. `lit` treats a refusal as a fatal
  error, which prismc's check makes unreachable.
- **The prelude.** `?Regex` → `regex` (`Regex::parse`), `?Match` → `match_`
  (`match_at`), `?Find` → `find`, `Matches` → `matches`, `Replace` → `replace`, `Split` →
  `split`, each the runtime's method; `SplitOn`, a text's separator split, is `split_on`.
  A prelude call whose name is a segment of a module's path is written
  `crate::prelude::name`, since in C++ that module's namespace would hide it.
- **Plain data.** Neither type has `^Pod`, but an emitted record derives the runtime's
  `ToPod` and `FromPod` whatever its fields: a regex, a replacement, a group and a match
  go out as plain data (a regex as its spelling, `Pod::Shown`), as a `Cursor` does, and
  `FromPod` is a mismatch.
- **The C++ link.** RE2 is a prerequisite of every C++ build (`runtime/cpp/README.md`);
  the emitted Makefile links `$(shell pkg-config --libs re2)` after `libprism_rtl.a`,
  in the CLI and the canvas variant alike.

## Negative space

- **No dynamic `Value`.** *Buys:* type errors at rustc, not in a tick; a counter that
  reads as a class.
- **No graph engine in the library.** *Buys:* a static widget with no lists, no ports and
  no subscriptions; the library's size does not grow with the language.
- **No commit list, no clear pass.** Stamps do both jobs. *Buys:* K3 at every tick's end.
- **No heap of our own, no compaction, no checkpoints** (D4). *Buys:* `Rc` and rustc's
  drop glue in place of a copying collector; the price is doubts AB and GC.
- **No CLI World in this crate.** Worlds sit over the protocol in crates of their own,
  as `worlds/canvas/rust` does. *Buys:* a library compiled programs link without a file system.

## Doubts

- **MO Monomorphization against one class per block** (Q-B9). The book keeps one node
  class per block so a checkpoint names blocks; with no checkpoints rustc monomorphizes.
  *First ruling (Claude's):* rustc's generics. *If otherwise:* trait objects per block.
- **AB A tick that aborts.** Conformance's AB goes on from the state before the tick;
  with no checkpoint there is none. *Ruled (coordinator, 9 November):* the Test World
  logs each tick that closes, its input and the ready state it ran under, and after a
  tick that aborts builds the program again and replays the log, both runtimes
  (`conf-ab-goes-on`, a job pending across the abort). *If otherwise:* `Program` gains a
  snapshot.
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

## Rust or C++ as the default back end

prismc emits either, and both reach a fixed point; `./prism compile` takes `--target rust`
when none is named, and the question is whether it should. This is Claude's recommendation,
Jimmy's to overrule: **Rust as the default**, C++ kept as the second target. The numbers are from Jimmy's Mac M2 Pro (12
cores, 16 GB); the building figures and the GUI table were measured with Low Power Mode on,
so every absolute time is pessimistic and the ratios between the two targets are the
evidence. Nothing in this section was measured on Linux. Where another figure comes from a
run on the same Mac under load, it says so.

### Running

The GUI table (bench/gui): js-framework-benchmark's operations on one table, median wall
time in ms over 10 runs, Mac M2 Pro with Low Power Mode on, the Prism rows before the
table's rows went over `@PerKey`; GTK 3 is the toolkit written in C beside it.

| Operation | Prism C++ | Prism Rust | GTK 3 |
|---|---|---|---|
| create 1,000 rows | 50 | 44 | 279 |
| replace 1,000 rows | 67 | 60 | 320 |
| update every 10th | 5.4 | 7.1 | 34.5 |
| select a row | 3.1 | 4.8 | 1.0 |
| swap two rows | 3.4 | 4.8 | 61 |
| remove a row | 12.2 | 10.2 | 5.9 |
| create 10,000 rows | 585 | 514 | 11,659 |
| append 1,000 rows | 59 | 48 | 390 |
| clear | 20 | 17 | 42 |

- **Rust wins the bulk operations by a tenth to a fifth**: create 1,000 rows 44 ms against
  50, replace 60 against 67, create 10,000 rows 514 against 585, append 48 against 59, clear
  17 against 20, remove 10.2 against 12.2. Both beat GTK 3 by five to twenty times on the
  creates, the replace and the append (create 1,000 rows: 279 ms in GTK) and by two times
  on clear; GTK removes a row faster (5.9 ms against 10.2 and 12.2).
- **C++ wins the three smallest operations by a quarter to a third**: update every 10th 5.4
  ms against 7.1, select 3.1 against 4.8, swap 3.4 against 4.8. GTK 3 selects in 1.0 ms,
  three to five times faster than either.
- **The small-operation gap is the Rust World's frame, not the emitted code.** A select
  after `@PerKey` retires 20.5 million instructions in C++ and 30.5 million in Rust (Mac M2
  Pro, Low Power Mode). The phase table (`bench/gui/prism/check.sh`, same Mac, one run
  each, claude/bugs/window-relayout-every-change.md) puts a Rust select at 31.1 M with 12.7
  M of it painting and a C++ select at 20.6 M with 1.4 M painting: the program's own ticks
  are 18.4 M in Rust against 19.2 M in C++, equal within 5%. About 11 M of every Rust
  frame is softbuffer presenting the whole frame on macOS, a new zeroed buffer per frame
  copied whole and made a `CGImage` (worlds/canvas/rust/README.md, "Painting"); the C++
  World sets its layer's image from its own buffer, and on X11 presents only the damaged
  boxes. That is a change to `worlds/canvas/rust`, which touches no template; if the Rust
  World presented as the C++ one does, the select column would read, by the instruction
  counts, about 3 ms on both.
- **Remove runs the other way: C++ 128 million instructions against Rust 80 million** (Mac
  M2 Pro, Low Power Mode), 1.6 times. The column's document is gathered an insert at a
  time (claude/bugs/window-relayout-every-change.md). A runtime matter, in `runtime/cpp`,
  which stands either way; the C++ `Map` changing its own nodes in place, as Rust's does,
  took about a seventh off remove's build on the Linux machine, and the rest is unprofiled.
- **prismc as the workload: Rust does less work.** Each compiler compiling itself (Mac M2
  Pro, 5 November, other jobs at load 3 to 13, prismc/README.md "Arrays and maps: what
  they hold"): the Rust prismc 586 G instructions, 45 to 46 s, 275 MB peak footprint; the
  C++ prismc 738 G, 65 to 68 s, 310 to 324 MB. Rust retires 21% fewer instructions and
  holds about 10% less memory on a compiler-shaped program.
- **Memory on the GUI table goes to C++.** On the table's small operations (Mac M2 Pro, 5
  November, other agents building, claude/benchmarks-preliminary.md "After the window
  fix") C++ peaks at about 172 MB and Rust at about 208 MB: C++ holds about a sixth less.
  The two memory results point opposite ways and neither gap is large.
- **Taken together:** one of the two models is ahead by a tenth to a fifth on everything
  but three operations, and those three come down to the frame present of one World. On
  running alone the two are close enough that running does not decide; it only rules out
  C++ being the faster target by nature.

### Building

Building prismc from clean, each pipeline all one language, Mac M2 Pro with Low Power Mode
on (`bench/building/run.sh`):

- **Rust: emit 65.9 s + build 100.1 s = 166.0 s, a 25.9 MB binary. C++: emit 86.2 s +
  build 134.6 s = 220.8 s, a 63.8 MB binary.** C++ takes a third longer on the wall and the
  binary is 2.5 times the size.
- **The emit itself favours Rust:** 65.9 s against 86.2 s. The Rust printer writes about
  115,000 lines for prismc; the C++ printer writes 185,385 lines across 63 translation
  units and 280 headers (prismc/README.md, "The C++ bootstrap"): per-module headers, each
  type declared, defined and its cases laid out in separate files, where Rust's `mod` needs
  one file.
- **The build's CPU, not its wall time, is the large gap.** On 12 cores (Mac M2 Pro, 4
  November, other jobs running, prismc/README.md "Building prismc, C++ against Rust")
  cargo spends 255 s of CPU on the emitted crate with its dependencies, 2.8 cores busy on
  average; `make -j12 CXX=clang++ OPT=-O3` spends 1,003 s, about 10 cores busy throughout,
  and with the module tree's 62 units 1,306 to 1,314 s (5 November, pick SF's table).
  Four to five times the CPU for the same program: every unit re-parses the standard
  library and `prism_rtl`'s headers (about 114,000 preprocessed lines for a module with
  no definitions) and re-optimizes every inline function and template it instantiates.
  On a machine with fewer cores, or a laptop on battery, this is the number that shows:
  the C++ build's wall time rises toward its CPU time where Rust's does not.
- **Small programs are close, Rust slightly ahead:** chess 19.5 s against 23.2 s, the GUI
  table 19.7 s against 23.5 s (Mac M2 Pro, Low Power Mode), most of either being the
  runtime library and the World built from clean. A second build that reuses the runtime
  library is not measured.
- **Taken together:** building favours Rust on every axis measured, by a third on the wall,
  four to five times on CPU, and 2.5 times on binary size.

### What the numbers do not say

- **rustc checks the emitted code; clang does not.** A template that reads a moved value,
  misses a `match` arm, or holds a borrow across a firing is a compile error in Rust. In
  C++ at `-O3` the same slip is undefined behaviour, which the compiler may silently turn
  into anything (Jimmy's rule: reason about undefined behaviour in C++). The sanitizer runs
  (ASan and UBSan under Homebrew clang on macOS, g++ on Linux; `runtime/cpp` and
  `worlds/canvas/cpp` `make test`) are the check the C++ side has, and a sanitizer finds
  only what a test exercises. The Rust runtime has no `unsafe` (runtime/rust/README.md);
  the Rust canvas World has one `unsafe` block, the macOS colour-space call. So a wrong
  template fails loudly on the Rust side and may run quietly on the C++ side; with Rust as
  the default, every compiled program is checked by rustc before it runs, and the C++
  target is checked against it (the C++ prismc emits the Rust prismc's Rust byte for byte).
- **C++'s emitted code leans on GNU extensions.** Every block, `match` and `TRY` is a GNU
  statement expression (claude/handoff.md; prismc/README.md picks SE and TR): g++ and
  clang++ take them, and a compiler without the extension cannot build the output. Rust's
  emitted code is Rust.
- **Portability goes two ways.** The C++ canvas World has a Cocoa path and an X11 path
  written here (`app_macos.mm`, `app.cpp`), each a window system done by hand, and its
  X11 path is the only one that presents just the damage; a third window system would be a
  third file. The Rust canvas World sits on winit, softbuffer, tiny-skia and fontdue, which
  cargo fetches and builds, so another window system is the crates' to support, not a
  file of ours. The C++ build needs RE2, cairo, FreeType and the X11 or Cocoa headers from
  the system (Homebrew or apt); the Rust build needs cargo and a network, or a vendored
  registry. A target with a C++ compiler and no Rust toolchain would turn this around, and
  the C++ target exists for it.
- **Binary size and stack.** The C++ prismc is 63.8 MB against Rust's 25.9 MB (Mac M2 Pro,
  Low Power Mode). The emitted C++ runs its program on a 4 GB stack, the Rust program on a
  1 GB one (pick BS): a C++ frame of the emitted code is larger.
- **Two targets are worth more than one.** Whichever is the default, the other is the
  check that the model is the model and not one language's habits; the byte-equal Rust out
  of the C++ prismc is that check, and the sanitizer run is the C++ side's. Dropping
  the C++ target would save the 220.8 s build in the bootstrap and nothing in the design.

### The recommendation, and what would change it

- **Rust as the default back end** (Claude's pick, Jimmy's to overrule). Reasons, in
  order of weight: the build costs a third less wall time and a quarter of the CPU and
  gives a binary two fifths the size (Mac M2 Pro, Low Power Mode; the CPU from the 4
  November run); rustc checks every emitted program where C++ at `-O3` runs it; running is
  a tie or a Rust win everywhere but three small GUI operations, and those are the Rust
  World's frame present (11 M instructions a frame), not the emitted code.
- **C++ as the default would be the pick if latency on small interactive steps were the
  one thing that mattered** and the Rust World kept presenting whole frames: 3.1 ms
  against 4.8 ms on a select is a difference a user cannot see, but a design that counted
  every millisecond under a pointer would take it. It would pay four to five times the CPU
  on every build for it.
- **What would change the recommendation**, each a measurement on the same machine under
  the same conditions for both targets:
  - **A Rust select still a third slower than C++ once the Rust World presents only the
    damage.** As measured, the program's own ticks are 18.4 M instructions in Rust against
    19.2 M in C++ (the phase table above); if, with painting taken out, Rust's select read 1.33
    times C++'s or more, the emitted Rust would be the slow half, and that is the back
    end's.
  - **The C++ prismc build from clean at or under Rust's 166.0 s wall and under twice
    Rust's CPU** (so under about 510 s against 255 s, or the same ratio against a fresh
    Rust figure): a C++ build that cheap, by precompiled headers, C++ modules or far fewer
    units, would take the building argument away, and the sanitizer run and rustc's
    checking would be weighed alone.
  - **The Rust prismc compiling itself in more instructions than the C++ one** (as
    measured, 586 G against 738 G): if the Rust model fell behind on a compiler-shaped workload by any
    margin, running would stop being a tie and start arguing for C++.
  - **Rust peaking above C++ by half or more on the compiler workload** (as measured,
    275 MB against 310 to 324 MB): memory is close both ways, and a Rust footprint of 1.5 times
    C++'s would matter on a 16 GB machine building beside other jobs.
  - **A requirement, not a measurement:** a platform Prism must run on that has a C++
    compiler and no Rust toolchain, or a host program that links Prism as C++ (an existing
    C++ application embedding a Prism widget). Either makes C++ the default for that
    platform and leaves Rust the default elsewhere.
