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
- Any other existential → `prism_rtl::Packed`: the value behind `Rc<dyn Any>` and the
  impls its packing found. A trait of the prelude is the packing's own, spelled from
  its name (`?PreludeTrait`): `with_show`, `with_eq`, `with_ord` are prism_rtl's;
  `with_num` and `with_pod` are the emitted Rust prelude's (`PreludePacking`), `^Num`
  keeping its operators at the hidden type as a dictionary (`NumPacked`, with `Num` for
  `Packed` over it) and `^Pod` how the hidden value is told as plain data (`ToPod` for
  `Packed`). In C++ `with_num` and `with_pod` keep nothing: such an existential is
  held, passed and counted, and has no arithmetic there. A program's trait is a dictionary the compiler emits beside the trait (`ItPacked`,
  `PackedDef`): `Weighed_t_packed`, a struct of one function per method over packed
  values; `weighed_t_packed_of`, which makes it at a type from a value of it (each entry
  reads the hidden values with `Packed::held::<T>()`, calls the impl at `T`, and hides a
  result of the trait's type again with `Packed::repacked`); and `impl Weighed_t for
  Packed`, each method asking the dictionary found in its first hidden argument
  (`Packed::dictionary::<Weighed_t_packed>()`). `any x` packs with
  `.with_dictionary(weighed_t_packed_of)` per program trait among its bounds and the
  traits above them, so an impl of any head, generic ones too, is kept: the dictionary is
  made where the value's type is known. A trait has a dictionary when each method has no
  type parameter of its own, takes the trait's type as at least one whole parameter and
  names it nowhere else but as its whole result (`?PackedDefOf`). In C++ the dictionary
  holds function pointers, `weighed_t_packed_of` is a function object whose call is a
  template among the generic definitions, and the impl is an overload in the trait's
  namespace.
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
- **A guard no checked program reaches** is an internal error: `InternalBug`,
  `InternalBugDo`, `InternalBugType` put "a prismc bug: .." where a refusal's words go, and
  `CompileProgram` reports it as `error[internal]`.
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
- **A function that takes time whole** (`IsWholeFn`: a `$` or `!` below the top of a
  parameter or of its result, `fn !Fired ((!made, more))`) takes and gives `Dynamic`s and
  `Event`s. A `$` or `!` binder of a pattern in its body names the `Dynamic` or `Event`
  itself (`RustPat`), and an application the checker lifted over one maps it
  (`EventValueOf`, `DynamicLift`).
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
  A tick holds every result told in it, in the order told (`TickOut.results`): a root
  that ends on a bundle tells each part by itself, a dynamic part at the build and in
  each tick it moves (`result_value`, `result_value_1`, .. each with its own
  `result_last`).
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
  positions, answered there in the World's scope and in a hosted one. Places are counted
  as they are built (rulings since the Bible, TC): see "Positions" below. An arm of a `match` reads the
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
- **A building `match` whose plain result a line above takes as a value at the build** (a
  widget's plain argument, a builder's rows) is built early instead, in the block's own
  build before the node, as a clause widget with a plain result is (`EarlyChoiceLine`,
  `IsEarlyChoiceAt`): where no arm has a guard or reads a name of the block, since the
  node does not exist yet when the arm is built. The result is then a value of the
  build and the block is not split at the line.
- **An `update` block's bare `for`** (`for &_.xs:`, `for xs:`; Semantics ch. 5,
  "Statements") rebinds `&_` to the entry: over a reference path the element in place
  (`UpdForRefs` under the key `&_`), over a plain value a `mut` copy of the turn's
  (`UpdForValues`). In a job (`ForJob`) the entry is a state of the turn's own, written
  back by its key over a reference path, and the block's own `&_` stays a state of the
  turn under `&_#block{place}`, so a job inside the turn takes it with the rest.
- **A constructor's `$` field** takes its argument whole (`CtorArgs`,
  `IsDynamicFieldCtor`): a reader's live `Dynamic`, which the line makes as it does for a
  function that takes time whole (`WholeArgsAt`).
- **`any x` of `^Show` alone** is `Rc::new(x)` cast to `Rc<dyn Show>` where it is made
  (`AnyValue`): an argument behind a reference and an array's element are not coerced.
- **A module's lowercase declaration written qualified** (`Words.base`) is parsed as a
  field read; the checker and the lowering read it as the qualified name where no value
  `Words` is in scope and the module declares the value (`?QualifiedField`).
- **A generic widget's type variable in an element parameter's output** (`Element Ui.*
  o`) is matched at the call as any parameter's is (`?ParamMatch`, `Unrigid`, `RigidsIn`).
- **A bundle with a late plain part bound whole to one name** is the name's parts,
  bound before any line of the block is lowered (`PreBindWhole`): an event part a node
  of the block that the line's event feeds once the line is built (`BindWhole`), a
  dynamic part a getter defined then, a plain part a getter of the node over its read,
  with no value at the build. The name is late (`{name}#late`): a building line above
  the split that reads it (`@Counter both` above `both <- @Choose mode`) is built after
  the row that gives the part (`DeferredLine`) and reads the parts through the node, as
  the lines below the split do (`RowPartsBn`). Another name for it (`let same = both`)
  is late as it is. A late plain part bound by a pattern of its own gives each name a
  getter that matches the part by the pattern (`LateName.within`).
- **A `let` of a bundle written out whose plain part reads a late name** (`let pair =
  ((n + 1, !go))` above `n <- match ..`) is late too (`MarkLateLets`, `MarkLateBundle`):
  the plain part is a getter computed from the node, with no value at the build
  (`LateBundleLet`); the other parts are bound as any bundle's; a building line that
  reads the name waits for the late result (`DeferredLine`). A `let` whose tuple
  pattern takes a late bundle apart (`let ((step, !go)) = both`) names each plain part
  as a late name (`LatePartNames`): a getter of the node that the `let`'s line gives
  the part's read to (`BindNamedPart`), with no value at the build. A late bundle read
  as one value from the node's methods (a place's field over `StepOf both`, a late
  `let`, a hold's step) is a getter of the node bound under `{name}#value`
  (`LateBundleValue`): the tuple of a plain part's read, an event part's event over a
  port its node fires, a `$` part's live `Dynamic`. Refused: a late bundle read as one
  value at the build (a supplier's `@ both`).
- **A `let` that names a bundle stands above the first line that reads the name**
  (`BundleLetsEarly`, in `LinesInto`): a bundle's name is its parts, bound where its
  `let` is lowered, and a `let` builds no place, so a line written above it (`@Counter
  pair` above `let pair = ((3, !go))`) is lowered after it and keeps its position. The
  block's result is still its last line as written (`IsLastLine`).
- **The split block's lines read each other across the split** (Semantics ch. 4: line
  order decides positions and nothing else):
  - the lines below that build no place (a hold, a `let`, a local function, a local
    widget) and read nothing a lower place line binds but events are lowered into the
    node with the lines above (`HoistedLines`);
  - an event a lower place line binds is a node of the block too, prebound, and the
    lower generation's own event fires it (`PreBindUps`, the row's `SkUp`);
  - the split line's names are getters of the node over the kept result (`late_{name}`,
    `BindLate`), which a hold's step, a dynamic `let` and a place's live field read;
  - a hold whose first value reads one (`$h <- n then ..` above `n <- match ..`) is a
    `Hold<Option<T>>`, empty until the row that gives the result is built, set in that
    generation's build right after the row (`WithLateInits`), before the lower
    generation is built and before any place goes out;
  - a place of the node whose field reads a late name is live over the node's getters
    (`ElementLine`): it goes out where every place does, after the node's rows are
    built, its level read from the node;
  - a plain `let` of the node that reads a late name is late too (`MarkLateLets`): a
    getter of the node that computes it (`late_{name}`, `LateLetGetters`), bound under
    `{name}#late` as the split line's names are, so what reads it reads it late in turn;
    a `let` of a pattern whose names are plain (`let ((low, high)) = Sides n`) is late
    with every name it binds: one getter computes the whole value (`late_whole`), and
    each name's getter matches it by the pattern;
  - a building line of the node that reads a late name and is no element's (a widget's
    plain argument, a `for` above the split, a building `match` over the name) is a
    generation of one arm at its own positions (`DeferredLine`, `IsLateTaker`): its row
    reads the block through the node, the late name through its getter, and is built
    after every other builder and generation of its scope (`Wx.deferredGens`,
    `BuildSteps`), the row that gives the late result among them. A plain `let` below
    the split that such a line reads is the node's late `let` (`HoistedLines`). Two
    such lines are refused: one whose places are counted as built (a building `match`
    or `if` inside it: the rows after it were numbered before it was built), and one
    that gives a plain result of its own;
  - a building `match` or `if` above that reads the plain result of a building `match`
    or `if` (`if n > 15:` above `n <- match mode:`) needs the result before either row
    is numbered. Where every arm gives its result from what it reads of the block, its
    own pattern and its own plain `let`s, and from nothing its own lines build
    (`IsPureChoiceAt`), the result is computed in the block's build as a plain `match`
    or `if` over the arms' plain `let`s and results (`PureChoiceValue`): the line is
    not late, the block is not split at it, and both generations are built in place
    order (ruling TC), the row's own copy of the result unread. An arm whose result
    reads what its own lines build is the refused case above.
- **A line is late** (`IsLateExpr`) when it is a building `match` or `if` with a plain
  result, a widget block or a widget of one clause whose block ends late, a widget
  parameter applied whose argument is late (read in the caller's environment), or a
  local widget whose block is. A late bind on a block's last line splits it too, with
  no lower generation.
- **A building `if` with no `else`** (ruling IE2) is a building `if` whose other arm is
  an empty block: it builds its arm or nothing, and the later places count from what was
  built.
- **A widget function parameter** (`Inner: fn Nat -> @unit`, given `@Row`) is bound as a
  blueprint parameter is, the argument's text with its caller's environments; `Inner 1`
  applies the function the argument names to the line's arguments, read in the line's
  environments (`ApplyWidgetParam`). Given a lambda (`@Use (\|F| @Shower label, F)`),
  the lambda's body is lowered at the line, in the lambda's own file: its parameters
  are bound to the line's arguments by layer as an inlined widget's are (`BindParam`),
  over the environments the lambda closes over, and its names are kept apart for each
  application by the line's call (`Lx.inlined`). Any other function value (a call's
  result, a field) is read where the argument stands and applied to the line's
  arguments, and the blueprint it gives is applied as any blueprint value
  (`AppliedBlueprintLine`).
- **A clause widget's blueprint parameter** (`fn @Show True, @w:`), and a widget
  function parameter of one, is bound in the clause's row as an inlined widget's is
  (`BindParam`): the caller's argument's text with the caller's environments as the row
  sees them (`ArmRowOf`'s `widgets`, `RowEnvNames`), built where the clause applies it,
  at the row's next positions. The argument is no value: the clauses' patterns match
  nothing in its place (`ClauseScrutinee`). So a widget defined in the caller's block is
  passed as any blueprint is.
- **A clause widget's `$` argument that its caller reads through its own parent** (the
  caller a row, the dynamic its block's) is a getter of the caller (`clause_dynamic`),
  read by the clause's row through its parent; the caller hears its block's write behind
  it and marks the row (`ClauseDynArg`).
- **The root's bundle result** is told part by part (`RootResultOf`): each plain part at
  the build, each firing of an event part after, one dynamic part as a root's dynamic
  result is.
- **A `let` of a name bound to a bundle** (`let same = both`) binds the same parts
  (`?BundleNamed`).
- **A row that ends on a bundle hands it to its block part by part** (`RowPartsOut`,
  `KeepParts`; a clause of a widget of several clauses, an arm of a building `match` or
  `if`, the lower generation of a split block). Part `i` is `p{i}`, a part of a bundle
  inside the bundle `p{i}_{j}`:
  - an event part fires the block's `{generation}_p{i}_up`, a node of the block's graph;
  - a dynamic part is the row's `result_p{i}` and `result_dynamic_p{i}`, kept by the
    block in `{generation}_p{i}_res` and read through `{generation}_p{i}_value`; a write
    behind it in the row calls `{generation}_p{i}_result_moved` (`KeptDyn`);
  - a plain part is the row's `result_value_p{i}`, kept by the block in
    `{generation}_p{i}_res` from the row's build on, and late (`WrLate`).
  A widget of several clauses that ends on a bundle is built after the block's node
  (`BundleClauseLine`), whatever its parts.
- **A line that binds a bundle with a plain part from a row is late** (`IsPlainResult`,
  `PlainIn`), and its block is split at it. The bundle's events and dynamics are names
  of the node, prebound as any line's (`PreBind`) and bound where the line is lowered
  (`BindParts`); its plain parts are the late names (`BindLate`, each read by itself,
  `LateParts`), and the lower generation matches the plain parts' patterns against
  their reads (`LowerBinder`). A bundle with no plain part splits nothing.
- **A `for` whose rows end on a bundle** gathers it part by part (`ForParts`): an event
  part into `{name}_p{i}_all`, one firing a tick by key, a dynamic part into the
  structural dynamic `{name}_p{i}_all`, a plain part as a dynamic no write moves. The
  row has one `result_p{i}` per dynamic part and tells the block of a write behind each
  (`Wx.resultMoved`, a list); the block's gathers of one row are `PendingRow.gathers`.
- **A `rebuild` whose block ends on a bundle** gives a dynamic of the bundle as one
  value (`WholeResultOf`, `Wx.wholeResultAt`): the generation's plain result is the
  tuple with each event as its port's `Event` and each dynamic it names as the live
  `Dynamic` made for it (`WholeArgsAt`). A bundle a line named whole (`@ both`) is the
  tuple of its parts by their bindings (`WholeNamedOf`, `WholePartsOf`): an event the
  `Event` of a port it fires, a dynamic a live `Dynamic` over the node's link
  (`LiveDynamic`), a plain part its value at the build, a bundle inside the tuple of its
  own parts, made the same way. A named bundle inside a tuple written out (`@ ((_,
  both))`) is that value under `{name}#value` (`WholeWrittenOf`). A block that ends on a
  line that gives a bundle with no name between (`@Counter _` last) gives the same value
  from the line's parts (`WholeOfResults`); with a late plain part inside, the value is
  read from the row once the part's row is built, every other part kept in a field.
- **A name bound to a whole bundle is its parts** (`BnParts`; `both <- @Counter 1`, `let
  pair = ((..))`, a name inside a tuple pattern over a bundle). Each plain part with a
  value at the build is a field of the node (`FieldedParts`): the build reads the local,
  the methods the field, a row the field through its parent. A plain part bound by a
  tuple pattern's name is moved into the field its name was prebound to (`BindParts`,
  `BindPlainResult`). A `let` whose tuple pattern takes a named bundle apart (`let ((ten,
  ((!more, $guests)))) = all`) binds each name to the part at its place
  (`BindNamedParts`): an event part feeds the name's node, a dynamic part is the name's
  getter, a plain part the name's field, a bundle inside is taken apart in turn or
  bound whole.
- **A row reads a named bundle of its block** part by part (`RowPartsBn`; a `for`'s row
  takes them from its port environment, `BundlesOf`): an event through a port of the
  row, a dynamic and a plain part through the parent, in the build through the `parent`
  argument (`PartsAtBuild`).
- **A row of a row reads a dynamic of the block around both** through a getter of the
  row between (`through_{name}`; `RowReadGetters` for a `for`'s rows, `ThroughGetter`
  for a generation's and for a bundle's part): the block's own read of its parent,
  which the inner row reads through its parent as any dynamic `let` of the block, so
  the outer block's writes reach the inner rows through the row between (`RowHears`,
  `parent_wrote_{m}`).
- **A widget's parameter that is a pattern** (one clause, so a pattern that cannot
  fail; `BindParamPattern`). A tuple over a bundle takes the argument apart
  (`BindBundleParam`): written out, each pattern of the tuple takes its part's
  expression as a parameter of its own takes its argument; a name bound to a bundle
  gives each pattern the part's binding (`BindPartsParam`), an event with its value
  form (`BindEventParam`), a plain part as a field. Any other pattern is over a plain
  value: the argument is a local of the build, matched once there, each name a field
  (`BindPlainPatternValue`). A pattern that binds an event or a dynamic out of a value
  (a record's `$` field, `fn @Shown Gauge($level=$l, name=nm):`; a tuple over a bundle
  that is a function's result) binds its names as a `let`'s are and takes the argument
  apart as a `let`'s value is (`BindTimePatternParam`, `PatternDynamics`): a getter of
  the node reads the argument in its caller's environment, each dynamic's getter
  matches it and samples what the binder holds, each event's node hears the event the
  value holds. Refused: a name that is a bundle of its own inside such a value.
- **`@Element made` as a line, `made` a `let` of the block bound to a tag applied**
  (Semantics ch. 8, "Elements and the element statement") is that element's line at
  the placing line's position, lowered in the placing line's environments
  (`WithElementLets`, `ElementCallLine`); a row that places it sees what its fields read
  through the parent (`RowEnvNames`). A `let` no line reads but to place it is no value
  of the node. `@Element` of any other element value in a block (a parameter, a call's
  result) applies the value's blueprint (`prism_rtl::element_blueprint`: a live element
  forwarded, a description placed) as a blueprint value is applied
  (`AppliedBlueprintLine`).
- **An element applied in a value's place** (`@Framed (Ui.Button($label=$"go {$n}"))`:
  an argument of a call that is not `@Element`'s, a part of a tuple or an array, the
  operand of `?`; `ElementValuesIn`) holds its `$` fields as `Dynamic`s, so its line makes
  a live dynamic for every reader its fields are written over (`WholeArgsAt`), and a
  field that is an expression is a map over them (`DynamicLift`).
- **A blueprint a `let` of the block names** (`let @w = @Tile k`; Semantics ch. 8: a
  blueprint builds nothing until a block applies it) is its expression, kept under
  `{name}#blueprint` (`WithElementLets`) and built by each line that applies the name,
  in that line's environments, at the node's next positions (`LetBlueprintLine`); a row
  that applies it sees what it reads through the parent (`RowEnvNames`). The `let` is
  no value of the node; where the name is read as a value it is the expression in its
  place (`NameValue`).
- **A widget with places as a blueprint value** (Type System ch. 2: a blueprint is an
  opaque handle and still a value; `?Pick b, @Pair`, a supplier's `Pad.Ask(name=n):
  @Answerer n`). The program is read once for the widgets that stand in a value's place
  (`ValuedWidgets`, `Lx.valued`): an argument of a function that is no widget or of a
  constructor, a host's supplier, what a lambda gives, the operand of `?`, a part of a
  tuple or an array, the value of a `let` (`LetValues`: `let @w = @Pair`, read where
  `@w` is), and a blueprint argument of a widget that stands in one (`@Frame @Pair`).
  A widget can be one when it has no widget function parameter and its `$` and event
  parameters have no time below their top, whatever it gives back (`?ValuedCases`).
  - A generic widget is one for each set of types the program's readings give its type
    parameters (`?ValuedTypes`: read off the arguments' types as an inlined call reads
    them, and off the widget's own type where it is read). `Lx.valued` is keyed by the
    widget and those types (`ValuedKey`), `Lx.valuedAt` lists a widget's readings in
    the order the program has them; each has rows of its own, lowered with the types in
    the substitution (`AtInstance`, `ValInfoAt`) and named `{Widget}_value_at{k}_{i}`
    (`ValuedStemAt`). A reading that leaves a type parameter open stands in a generic
    function, whose own parameter it is given (`fn ?Wrapped x = ?Pick True, (@Shown x,
    "n")`): it has rows for the types each of that function's instantiations gives it
    (`PlacedWidget`). A definition's instantiations are the types its uses give its
    type parameters over the whole program, a use in another generic definition's body
    at each of that one's own (`?DefInstances`; `Lx.inDefinition` names the definition
    being lowered). The function is a target generic, lowered once, so where it runs
    it chooses among those rows by its type parameters (`?OpenWidgetBlueprintValue`:
    prism_rtl's `type_key` of each against the instantiation's types; two
    instantiations of one target type are one choice); the arguments are read at the
    function's own types and are the row's by the choice made (`recast`). A function
    the program never uses chooses nothing. Where the instantiations cannot all be
    read (a use in a method's body at the impl's type parameters, a definition that
    uses itself at other types than its own, more than eight definitions deep) the
    reading is refused.
  - A `$` or an event parameter is the caller's live `Dynamic` or `Event`, which the
    blueprint keeps and each row it builds holds (`LiveParams`, as the shared rows of a
    widget that calls itself do); the line that reads the widget makes the dynamic
    (`WholeArgsAt`, `SharedLiveCalls`). A blueprint parameter is a value of the row,
    applied as any blueprint value is.
  - An event result is a port of the row's that the event fires
    (`ValuedEventResult`, the row's `result_event`); prism_rtl's
    `widget_event_blueprint` subscribes the sink of what applied the blueprint, each
    firing an answer under the arm's tag or outside any, as a plain result is.
  - A dynamic result and a bundle are one value the row's `result_value` gives
    (`ValueRowWhole`): a dynamic is a live `Dynamic` over the row, its update event a
    port the writes behind it fire (`LiveDynamic`); a bundle is the tuple of its parts,
    each event the event of a port of the row's that it fires, each dynamic such a live
    `Dynamic`, each plain part its value. Outside a tag's arm the blueprint answers
    with that value held as it is, once, at its build (`crate::Answer`'s case
    `Held_a`, the prelude's `answer_held`), and so it does with a plain value that is
    no plain data (a function): the line that applies the blueprint reads the widget's
    own dynamic, events and value, so they move in the tick the widget's do. Under a
    tag's arm a dynamic result is the place's first value and each move of it a push
    of the next tick, as `@ $d` is (prism_rtl's `widget_dynamic_blueprint`), and a
    bundle is answered as a bundle that is a value already is (`@ pair`,
    `PartsAnswerOf`): prism_rtl's `widget_parts_blueprint` builds the row and makes
    the `PartsAnswer` of its one value, the plain and `$` parts the tag's state, each
    event part the tag's answers, all but the first value inputs of the next tick.
  - A widget whose block ends on a host's line gives the host's output: the tuple of
    the child's result and the child's document, the document a live `Dynamic` over the
    row as any dynamic of the result is (`ValueRowWhole`, `WrHosted`).
  - Each such widget is lowered once more as rows of its own, items of its module
    (`ValuedRows`): one row type a clause, `{Widget}_value{i}`, its places numbered from 0
    and built from a base, holding no block, telling how many places it built (`built`),
    as the rows of a widget that calls itself are. Each clause has a second row,
    `{Widget}_value{i}_hosted`, for a hosted scope: its places are entries of a host's
    document, and it is built with that host (`enclosing`) and what tells the host that
    a place of the row's came or went (`moved`), as a shared row in a hosted scope is.
    A blueprint value is built in the scope of what applies it, which no value site
    knows, so both rows are emitted.
  - The value is a `prism_rtl::Blueprint` made where the widget is read
    (`WidgetBlueprintValue`, prism_rtl's `widget_blueprint`): the arguments are read
    once and kept; each build chooses the clause by them and builds its row at the path
    and base it is given: the World's row in the World's scope, the hosted row under a
    host, given the host of the scope and `prism_rtl::host_moved` of it (`HostOf::moved`,
    the owner's `host_k_moved`). A plain result is the first value of the place the
    blueprint answers, under the arm's tag as `@ x` is.
  - What a hosted row built is read through `Supplied::document` (its places' entries
    by their paths under the path it was built at) and answered through
    `Supplied::deliver` (the prelude's `built_widget` makes both of the row's `Row`
    methods). A host's row gives them to the enclosing host (`HostRow::entries`,
    `HostRow::deliver`); a block that applied the value gives them as its generation's
    (`AppliedBlueprint`).
  - A widget with a widget function parameter is not among them, and is refused where
    it is read as a value.
- **A widget defined in a block read as a value** (`Card(@body=@line)`, `?Pick True,
  @line`) is a `Blueprint` its line makes at the build before the node, kept in a field
  (`BlueprintValuesAt`, `blueprint_{name}`): it builds the widget's block as a row of its
  own (`{Node}Value`), numbered from 0, which holds the node through its link and reads
  the block's names through it as an arm's row does. Bound under `#whole@{span}`, so a
  row and a closure of the line see it as they see a live dynamic.
  - The widget has two rows (`ValueRowOf`): `{Node}Value` for the World's scope and
    `{Node}HostedValue` for a hosted one, built with the host and its telling as a
    valued widget's hosted row is (`Wx.?valuedAs` tells `ArmRowOf` which, whatever
    scope the block stands in).
  - The node keeps the rows the blueprint built, weakly (`prism_rtl::WeakRows`,
    `{blueprint}_rows` and `{blueprint}_hosted_rows`; a blueprint builds as often as it
    is applied). A write behind what the rows' places read of the block dirties each
    row that lives (`RowMarks` of kind `ValueRow`, `RowWritten`), so a place that reads
    a dynamic of the block changes in place.
  - What the widget gives back it answers with as a top-level widget read as a value
    does outside a tag's arm: plain data outside any tag; an event through a port of
    the row's (`ValuedEventResult`, `widget_event_blueprint`); a dynamic, a bundle and
    a value that is no plain data held whole (`ValueRowWhole`, `answer_held`).
- **A blueprint value applied by a block** (`? @t: @t`, `ignore <- card.@body`;
  Semantics ch. 8: at the applying block's next positions) is the node's generation at
  the line (`ValueBlueprintLine`): the prelude's `apply_blueprint` builds the value at
  the node's path from the line's position as built, in the node's scope (the World's,
  or the host's whose document the node's places at the line are entries of), and wraps
  what it built as a `Row` (`AppliedBlueprint`). In a hosted scope that row's `document`
  is what the blueprint built (a widget's hosted row's places, or the one entry of an
  element forwarded or placed, at the base) and its `deliver` hands a host's answer to
  the widget's row or to the blueprint's sink; the generation tells the host that the
  document moved as any generation of a hosted node does. The generation is transparent,
  `wideGeneration` positions wide and gappy, as a call of a widget that calls itself is,
  so the later places are counted as built and an answer under it is handed on with its
  position counted from the generation's first.
  - A line that gives an event back (`!go <- @Element e`) hears the blueprint's sink
    (the prelude's `apply_blueprint_heard`): each answer, a `crate::Answer` under the
    tag of the element answered, is taken out at the line's type by that tag's accessor
    (`AnswerSink`: the program's tags whose answers are of that type, and its generic
    ones) and fires the line's event through the node's `{generation}_up`, as a
    generation's row fires its block's.
  - A line that binds a plain result (`n <- @t`) takes the blueprint's first answer
    (the prelude's `apply_blueprint_answered`), read at the line's type (`FirstAnswerTaken`): by the tag
    it is under where a tag of the program answers with that type (a blueprint made in
    a supplier's arm answers under the arm's tag; that tag's accessor reads an answer
    made outside any tag too), else as plain data (`answer_plain`, `Answer::plain`); a
    type that is no plain data (a function) is read from the answer held whole
    (`held_answer`, `Answer::held`). The node keeps it in `{generation}_res`
    from the build on: the result is late (`WrLate`, `IsLateExpr`), as a call of a
    widget that calls itself keeps its own.
  - A line that binds a dynamic (`$d <- @$t`, `$level <- @Element e` of an element
    value whose output is a dynamic) takes the dynamic the prelude's
    `apply_blueprint_dynamic` makes of the blueprint's answers: a hold whose value at
    the build is the first answer and which each later answer writes, in the tick the
    answer comes, read at the line's type as a bound first answer is. The node keeps it
    in `{generation}_res` as a generation's dynamic result is kept (`KeptDyn`) and
    follows its update event (`prism_rtl::Follow`), each firing marking what reads it
    (`{generation}_result_moved`). A first answer that holds a `Dynamic` whole (a
    widget's dynamic result) is the line's dynamic itself.
  - A line that binds a bundle (`((step, !go)) <- @t`, `((!rx, $link)) <- @Element e`;
    `AppliedBundleLine`) takes it as one value, its events and dynamics whole, from the
    prelude's `apply_blueprint_whole`: a first answer that holds the value whole (a
    widget's bundle) is it; any other blueprint answers as a bundle's tag does, and the
    value is made of those answers: the state (the tag's first-value case, or its own
    when it has no event part) a hold the first answer makes and each later one writes,
    its `$` parts dynamics over it (`state_part`), its plain parts read at the build;
    the event parts the event the tag's answers fire, of several the firings of the
    sparse tuple that hold the part (`event_part`). The tags are the program's whose
    output is the line's bundle (`BundleTagsOf`). The node keeps the value in
    `{generation}_whole` and binds its parts (`AppliedPart`): a plain part is late,
    read from the kept value; a dynamic part is kept and followed as a dynamic result
    is (`{generation}_part{place}_res`); an event part is heard (`hear_part`) and fires
    an event of the node through `{generation}_part{place}_up`.
- **`prism_rtl::Blueprint`** is built with a base beside its path, sink and scope: 0
  under a host's row, the line's position as built in a block. `Supplied` tells how many
  places were built (`built`), how a widget's row leaves (`leave`), and, for a widget's row
  built in a hosted scope, its places' entries and a host's answer to one (`document`,
  `deliver`). In a hosted scope a forwarded element's `entry` is the child place's own. The element forms
  (`element_blueprint`, `described_blueprint`) put their one place at the base and
  answer at the first position under what they built, so a block applies them as it
  applies a widget's.
- **A widget defined in a block** (`@line:` and its block) is bound to its block
  (`BnLocalWidget`) and lowered at each line that applies it, in that line's
  environments, at the node's next positions (`LocalWidgetLine`).
- **A row's view of its block** (`RowEnvNames`) carries the block's local functions and
  local widgets as they are, with the names their bodies read, and a widget parameter
  with its argument's environments seen through the row: the caller's events as ports
  of the row, its values and dynamics through the parent. A `for` row takes its widget
  parameters the same way (`EventsOf`). A `for` row's plain locals of its block
  (`RowLocals`) are the ones it names and the ones a function or a widget of the block
  it calls reads (`RowNamesClosed`); a dynamic of the block that is no getter and no
  hold (a hold that starts from a late result) is read through the parent as it stands
  (`RowReads`, `OverBlock`).
- **A widget's plain result bound by a pattern** (`((Extent(wide=w, high=h), ((d, t)))) <-
  @Measure 3`): a name or a tuple of names takes its parts as locals of the build, each
  moved into its field (`BindPlainResult`); any other pattern is matched once in the build
  against the result, and each name it binds is moved into the field its prebinding made
  (`BinderNames`, `BindPatternResult`).

- **A live dynamic** (`WholeArgs`: a reader of the node passed whole to a function, the
  outer of a `@Flatten` written in place, `$d` under `!Updated` in a lambda of the line) is
  a `Dynamic` made before the node whose read goes through a `prism_rtl::Link` to the
  node's getter (`whole_{x}`), its update event a port the writes behind it fire.
  - With a value at the build it reads that value until the node is linked.
  - With none (a clause widget's result, a place's dynamic, a name of the parent), and in a
    hold's step, it reads through the link alone; a read before the link is
    `unreachable!()`. The links are made before every line's `let`s and linked first once
    the node is made, before the builders.
  - A row reads its block's live dynamics through its `parent` (`WholeOfBlock`), and hears
    the block's writes behind its own (`RowHears`).
  - A global dynamic passed whole is the constant its getter gives.
- **`!Updated $d` as an event value** (in a supplier's closure, in a function taking `$d`
  whole): the prelude's `updated`, which fires `d`'s value after the tick when `d`'s update
  event fires. A widget's event parameter has a value port as a bound event has.

## Positions

- Places are counted as they are built (ruling TC): after a building `match` or `if`, or
  a widget of several clauses, the block's later places take the next numbers after the
  places the chosen row built.
- Inside a node the lowering still writes each position as a fixed number, the one the
  place has when every transparent generation before it builds its widest row. That
  keeps every match on a position, every forward to a row and every place's name static.
- A transparent generation whose rows differ in what they build is gappy
  (`GenDef.gappy`). The node keeps how many of its positions the chosen row left unbuilt
  in `{name}_gap`, set where the row is built: the row's shortfall against the widest
  row, plus what the row's own gappy generations left (`inner_gap`).
- Going out, every path is spelled through `place(v)` (`PlaceNumber`): `v` less the gaps
  of the generations that end at or before it. A transparent row is built with `base`,
  the number its first position has as built, and counts from it. A node with no gappy
  generation and no base spells the number itself, so its code has no `place`.
- Coming in, a node with gaps of its own puts them back before it matches (`seat`, the
  prelude's `seated`): a World's answer in `answer`, a host's in `host_k_answer` and
  `deliver`. A transparent row is handed the path with its block's gaps already put
  back, so each node undoes only its own.
- A hosted scope has its own sequence, so its methods are `place_k` and `seat_k`.
- A place read at the block's build before the node exists (a first-value query, a host
  made in the build) after a gappy generation built after the node has no number yet;
  that is refused loudly. A generation built early gives its gap as a local
  (`{name}_gap_now`).
- **A widget that calls itself**, of one clause or of several (plain parameters, no type
  parameter, nothing or a plain value given back), is as deep as
  its data (`SharedClauseLine`). A plain result is kept by each row (`result_value`) and
  by the calling block in `{generation}_res` from the row's build on, late; a block that
  binds it is split at that line (`IsRecursingCall`, `Lx.recursing`). The outermost call is
  lowered as any; a call inside the widget's own clause makes the clauses once more as
  rows shared by every further call (`Lx.shared`): one row type per clause, numbered
  from 0 (`zeroBased`), built with `base`, holding no block, each with `built()`, how
  many places it built (its positions less what its own generations left unbuilt).
- Such a call is a transparent generation `wideGeneration` (2^20) positions wide in its
  block's numbering, gappy, its gap what the row did not build of them, so every
  position after it is counted as built by the same `place` and `seat` as any gappy
  generation's. A call builds fewer than 2^20 places.
- A generation that wide (one that holds such a call is too) has no arm per position:
  the node's last answer arm takes any path (`WideForward`), finds the generation whose
  positions the first segment is among (the prelude's `within_positions`) and hands the
  answer on, to a shared row with the position counted from the generation's first
  (`rebased`, `GenDef.rebased`).
- A blueprint parameter of such a widget, where the widget gives nothing back and the
  call's argument is a blueprint value the lowering can make (`IsBlueprintValued`: a
  widget in `Lx.valued`, named in place or through the caller's own parameter, or a
  blueprint the node holds), is a value of the shared row: a field holding the
  `prism_rtl::Blueprint`, matched and passed on as a plain argument is
  (`SharedScrutinee`), applied as any blueprint value (`ValueBlueprintLine`).
  `ValuedWidgets` reads the blueprint arguments of a call of a widget that calls itself.
- A `$` or an event parameter of such a widget, where the widget gives nothing back, is
  the caller's live value held by the shared row (`IsLiveParam`, `LiveParams`): a
  `Dynamic` the call's line makes for the reader it is given (`WholeArgsAt`,
  `SharedLiveCalls`), an `Event` made of the event's port. Every call of such a widget
  is lowered as shared rows, the outermost too (`SharesAt`), so each level is handed
  the value itself.
  - The row reads a `$` parameter through its getter `{name}_param_value`, which
    samples the dynamic at the read point, and follows the dynamic's update event
    (`prism_rtl::Follow`): each firing marks what reads the getter
    (`{name}_param_result_moved`).
  - An event parameter is a port node of the row's graph, fed by a subscription the
    build makes to the event's port (`subscribe_row`).
  - The value is bound under `#whole#{name}`: a call inside hands it on as it is
    (`SharedScrutinee`), and an arm's row of the row's reads it through its parent
    (`WholeOfBlock`).
- In a hosted scope a shared row is given its host (`enclosing`, as any hosted row) and
  `moved`, a `prism_rtl::Moved` made of the node that builds it (`moved_by`: that node's
  `{host}_moved` or `document_moved`): the row holds no block, so its `document_moved`
  calls that (`sharedMoved`, `DocumentMoved`). A host's typed answer reaches a
  generation that wide through one last arm of the typed dispatch (`TypedWideForward`),
  as the World's does through `WideForward`.
- Any other widget that calls itself (a `$` or event parameter with time below its top
  or beside a result, a blueprint parameter whose argument is no such value or beside a
  result, an event, dynamic or bundle result, a type parameter) is lowered level by
  level, to the sixteen levels an inlined widget is (`TooDeep`: a tree deeper than that
  faults at its build).

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
  pushes are the child place's. Each firing of an event field is the place's request:
  out to the World, or in a hosted scope a request of the enclosing host's child
  (`HostOf::request_from`).
- **A request rises host by host.** A hosted place's request goes to its host's row
  (`Host::request`), which the live element's event fields hear; a row that forwards the
  element hands it on as the host's own at `0` under the row, to the World or to the
  enclosing host's row for that place, which decides again.
- **A pattern sees either form as the tag's description** (`DescView`,
  `rt::element_view`). A description of the tag matches as it is. A live element of the tag
  matches as the description of its level, each `$` field its level projected. So a `$`
  field binds a `$T` in both forms. A function with an element parameter takes its time
  whole (`HasElementParam`), so its callers read what it gives.
- **An event field in an element pattern** (`Pad.Bell(!rings=!r, ..)`, ruling EP) binds an
  `Event`: the description's own, or, for a live element, the requests its place makes
  through that field (`Live::requested`: the host row's `requests` port, filtered by the
  field's name and decoded), fired in the tick of the request.
- **A tag's answer constructor named, not applied**, is a closure building the variant.
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
    at the build while the owner is being made. A dynamic read through the owner's parent
    (a host in a row answering with the block's `$n`) has its value at the build taken
    at the host's line, through the parent, into a local the supplier copies
    (`{getter}_at_build`, `AnswerAtBuild`); the block's write tells the row, which fires
    the getter's port (`RowHears`). An answer through a blueprint a `let` names (`let
    @$live = @ $level`) is its expression in the name's place: `DynamicAnswers` reads
    the `let`'s expression, and the supplier's closure captures what it reads
    (`WithBlueprintReads`). An answer through a widget's blueprint parameter (`fn
    @Hosting @answer:` whose supplier's arm gives `@answer`) is the argument in the
    name's place, read in its caller's environment and context under the arm's tag and
    the supplier's link (`ParameterEnv`): `DynamicAnswers` reads the argument with its
    caller's environments (`AnswerTexts`, a parameter handed on followed to its own
    argument), and the supplier's closure captures what the argument reads from the
    caller's environment (`LambdaCaptures`). A computed answer in flight to a place whose key left is
    dropped. Nested hosts add one tick per computing layer and none per
    forwarding layer.
- **First values.** A child place with a first value asks the host at its build
  (`host_k.first_value(&place, &elem, &entry, cx)`), before the owner's node exists: the host builds
  the supplier row for the place early and keeps it in the rows, where the builder finds it
  present. This is the only early construction; it is why `Host` reaches its owner through
  a link filled in after `Rc::new`.
- **A supplier's dynamic answer with no value while the owner is being made** (it reads a
  structural dynamic of the owner's, a `for`'s gathered result or a `rebuild`'s;
  Semantics ch. 7, "Structural dynamics": a builder in a tick's construction step and
  the tick's outputs read the value after it) is read through the owner's link alone
  (`#dynlate`, `DynamicAnswers`), and the host's scope is late (`Wx.lateHosts`). The
  child's places with a `$` output have holds that are empty at the node's making
  (`Hold<Option<T>>`, read as the value they hold, `LatePlaceBn`; a push writes the
  present value) and ask their first values in the construction step where it reaches
  the host's line (`LateFirstPlace`, `Wx.lateFirsts`, `ScopeSteps`): the owner's
  builders above the host have run and their snapshots are taken, so the row the host
  builds for the place reads the gathered dynamic over the rows built above it. A late
  scope's own builders, generations and entries follow its first values, just before
  its host's rows; any other hosted scope is built before the owner's own, as before.
  Refused there: a bundle tag's plain part beside a `$` part (the build reads a plain
  part once, before the node).
- **Requests** of a hosted place go to the host (`host_k.request`): out with the forwarded
  element, or nowhere when the supplier answered the place.
- **`@HostSome`** is `@HostDocument` through `rt::forward_on_none` (`nil` forwards).
- **`@HostList @child, !answers, supplier`** is `@HostDocument` whose owner also hears
  `!answers` (`Host::hear`): each firing's map hands the place at each key its answer as
  a computed answer, the next tick's input (`HostRow::answer_later`); an answer of a tag
  the place is not of is a fault of the tick the place reads it in; a key with no place
  is dropped. `Answer c` is `crate::Answer`, and a tag's answer constructor
  (`Ui.Button.Answer click`, rulings round 12, AQ) is the tag's variant of it over what
  the tag's answer carries (`AnswerValue`); `Wired c` is `Elem c` (prismc's first ruling HL).
- **An element's level holds the events of its event fields** (`LevelEvents`, ruling EP):
  the tag's struct has a field per event field, the event the element was made with (a
  port the place's event fires, `request_{pos}_{field}_port`, and the `Event` of it made
  once, so the level holds the same event while the place lives; a description's own
  event). Events compare by their port, as dynamics by their read. They are no part of
  the element's plain data: a World sees a firing as a request, and an element read back
  from plain data has events that never fire. So a pattern over a document's entry
  (`Elem r`) binds the event, and a host's body hears its child's requests out of `$doc`.
  The core prelude's own element tags hold none.
- **A generic tag holds each of its variables as plain data** (`TagFieldOut`): `crate::Elem`
  has one variant per tag whatever it is placed at, so in the tag's level, its description
  and its `Answer` variant a type variable of the tag is a `Pod`. A place recodes the
  fields it gives (the prelude's `recode`, by `ToPod` and `FromPod`; a `$` or event field
  mapped by it) and the answer it reads (`RecodedAnswer`), and a host's arm reads a field
  as the plain data it is.
- **An answer made outside a tag's arm** (a host's `let @seven = @ 7`) is plain data in
  `Answer`'s case of no tag, `Untagged_a`; each accessor of `Answer` reads that case at its
  own tag's type, so the place that takes the answer decides what it is.
- **A place whose element answers a dynamic has its value at the build**: its first
  value stands in the local `{f}_init` (`DynamicPlace`), bound as the name's value at the
  build (`PreInit`), so a supplier's dynamic answer over the host's own pushed place
  reads it while the node is being made.
- **A tag whose output is a bundle** (Type System ch. 2, "Bundles and sparse tuples":
  `((WindowId, !WindowInput))`, `((!Text, $Text))`, `((Nat, $Text))`, `(($Text, $Nat))`,
  `((!Text, !Nat))`) is one mechanism for any mix of plain, `$` and event parts
  (`BundleOutputOf`, the node's `BundlePlace`s).
  - **The state** is the plain and `$` parts' values, one part itself, several a tuple in
    the tag's order. It is the place's first value (the World's first-value query, or the
    enclosing host's) and what each push after it holds, keyed by the element. A plain
    part is a field read from the first value once. A `$` part is a hold made with its
    part of the first value (the local `{f}_init`, the name's value at the build,
    `PreInit`); a push writes every `$` part's hold with its part of the state
    (`BundleStateWrite`), as any hold is written.
  - **The event parts' answer** comes by path. Of one event part it is the part's value;
    of several it is a sparse tuple, one `prism_rtl::SparsePart` per event part, and each
    part that is there fires its event (`BundleEventsFire`). As plain data an absent part
    is `Pod::Absent`, written and printed `-` (`(("hello", -))`). A bundle with no event
    part is answered by path with its state.
  - **In `crate::Answer`** the tag's variant carries the event parts' answer (`AnswerTy`),
    and a tag with both event parts and a state has a second variant, `{Tag}First`, the
    state (`?BundleFirstTy`, `{tag}_first()`); a bundle of state parts alone has its
    state in the tag's own variant. A World's first value and its pushes to a forwarded
    element decode to the state's variant (`AnswerOf::decode_first`), its answers to the
    tag's own.
  - **The child's typed dispatch has one arm for a bundle's place** (`BundleTypedArm`):
    the state's variant is written to the `$` parts' holds, any other answer fires the
    event parts.
  - **A supplier's bundle answer** (`@ ((a, $b, !c))`, or `@ pair` of a bundle that is a
    value) is `rt::parts_blueprint` over an `rt::PartsAnswer` (`PartsBlueprint`). The
    state is one `Dynamic` of the state's variant: a constant when no `$` part is live
    (a constant `$ v` is its value), else read at the read point from the plain parts,
    cloned, and each live `$` part's own `Dynamic` (read through the owner's link as a
    dynamic answer's is, `DynamicAnswers` keys the getter by the part, `?PartDynamic`),
    and moved when any of them moves (`moved_by`). Its value at the row's build is the
    place's first value; each later move is one push of the whole state, delivered the
    next tick. Each event part is mapped to the tag's answer (of several event parts, a
    sparse tuple holding that part alone); each firing is an answer a tick late, so two
    parts that fire in one tick are two answers of the next.
- **A named widget function as a supplier** (`@HostDocument @child, @Supply`): a widget
  function shaped as a supplier (a parameter `Element c o`, the result `@o`) whose clauses
  build no place (`@ x`, `@Element e`, a `match` or `if` of them, after `let`s) is emitted
  as a function giving the blueprint (`IsValueWidget`), which the supplier, a lambda calling
  it, or a value naming it calls. Any other widget function used as a value is refused.

## Local functions, `@Machine`, durations

- **A fault kind as a value** (`Fs.NotFound` named, an `Error`'s `kind` read) is its name
  as plain data, a `Pod` choice with no arguments (the prelude's `fault_kind`, `kind_of`);
  `Error(kind=k, ..)` takes the name back (`error_of`). An `Error`'s `message` is a `Text`.
- **The prelude's `Turn`** is `PreludeTurn` in both emitted preludes (`TypeItemName`): the
  runtime's `Turn` is the job's. In C++ it has the form of a program's own choice (a case
  struct each, positional fields `a0`).
- **A trait method named as a value** is a closure calling it (`MethodFn`).
- **A function value of its callee's own color in a call's result** (`[u fn Nat -> Nat]`
  at a call that makes `u` bounded) is a `Start` in the callee's signature; the call's
  value is made again by one walk over its two types, the signature's and the call's
  (`Recolored`): an array is mapped (the prelude's `each`), an event and a dynamic
  are mapped (`Event::map`, `Dynamic::map`), an optional and a `~T`
  matched, a tuple taken apart, a map made again with `fold_map` and `insert`, a record
  made from its fields and a choice by a `match` over its cases, each field at its type
  under the signature's type arguments and under the call's (`NamedRecolored`), to any
  depth and in any nesting. A function is made again (`FunctionRecolored`): a `Start`
  where a bounded function is wanted runs each application's job to its end; a
  function's own parameter is walked the other way, where a bounded function is wrapped
  as a `Start` that answers in its first cycle; a `Start` that stays one and whose own
  parameter or result holds the function is a `Start` that hands the held one its
  arguments walked and maps its job's result (the prelude's `map_job`). The walk
  starts from the signature's result at the types the call gives the callee's type
  parameters, so the two types differ in color alone, and it is made whenever the two
  differ: a function beside it that stays a `Start` is itself. A blueprint that holds
  the function (`?@(u fn Nat -> Nat)`) has its answers made again (prism_rtl's
  `answers_mapped`, the prelude's `held_answers_mapped`): its first value and each
  answer it gives, one that holds the output whole or one firing's or one move's value
  (`answer_held`). A blueprint's target type names no output, so the walk is found by
  the two types themselves (`HeldDiffers`, `BlueprintOutputs`). A call that is a frame
  of an unbounded function's body walks the job's result the same way before the rest
  of the source runs (`FrameTaker`, `?SignatureResult`).
- **A function value applied** (`Take (\|n| n * k)`, `Take Double`, `Take` a `let` or a
  parameter) takes each argument of a function type as a function value
  (`ValueArgument`); a named function takes a borrowed closure or a path there.
- **`@Run` of a function value that is no function the program defines by clauses** (a
  lambda, a local, a call's result, a constructor, a trait's method, a function of the
  prelude) takes the `Start` it is or is made (`AsJobValue`); a function the program
  defines, bounded or not, is named by its `start_..` (`JobRoots` gives it a job).
- **A local function that is not bounded, as a value** is a `Start` as a job lambda is
  (`JobLocalFn`): its job is the clauses as one job body, the arguments matched against
  each clause's patterns in order, and entering it is a cycle point.
- **A prelude trait's impl** names its functions after the method and the head
  (`HeadWord`: a named type's name, a collection's shape with its parts).
- **A widget block's `let` of a record's or a constructor's pattern** computes the value
  at the build, matches it once there and moves each name it binds into its own field
  (`BindPatternResult`, as a widget's plain result bound by a pattern). A pattern that
  binds a dynamic or an event inside it (`LetPatternDynamics`) keeps the whole value as
  a getter of the node: a `$` binder's getter matches it and samples the dynamic it
  holds; an event binder's port node is subscribed at the build to the event the value
  holds (read out by the pattern, `{heard}_event`) and feeds the name's own node.
- **`for &v <- &m:` over a map** (ruling FM) takes the keys before the first step and
  reads each entry's place through its key (`keys`, `at_mut_map`), in key order.
- **A `while` or an `if` bind in an `update` block whose pattern has conditions** (a
  literal, a payload behind an `Rc`): the `while` leaves the loop at the first that fails
  (`let .. else` for a payload), the `if` is a value `match`'s arm with its `else` the
  other.
- **A cycle-point `while` inside `for &s <- &xs:`** (`ForJob`): a job keeps no reference
  across a cycle, so the entry is a state of the turn's own, which the inner loop's job
  takes with the block's state, written back to its place by its key as the turn ends.
- **An interpolation over a `?` or `~` hole** is a plain-layer lift: the checker notes it
  with its holes, and the lowering matches each (`DirectOperands`).

- **A local function** (`fn Add a, x = ..` in a block) is inlined where it is called, its
  parameters bound to the arguments (a pattern parameter matched against its argument),
  and is a closure where it is named as a value, at the type it is used at there: its
  clause as a lambda, or with several clauses one closure whose body matches its
  arguments against the clauses' patterns in order (`LocalClausesClosure`). One the
  checker generalized is lowered at each use's instantiation (`LocalFnLx`), so a
  closure of it is made per use, at the type used there. One whose body reaches its own
  name (itself, or through the block's other local functions) is a recursion
  (`RecursiveLocalFn`): one closure that takes itself first and its arguments owned as
  one value, given to the prelude's `recursive` with the first arguments
  (`recursive::<(Nat, Nat), Nat>(&|go, (k, acc)| .., (n, 0))`); its body calls `go` for
  the function's own name. A closure cannot name itself, and a nested `fn` item could
  read neither the block's locals nor the enclosing function's type parameters.
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
- **A generalized `let` in an unbounded function's body** is lowered at each of its
  instantiations, calls and all (`GenericLetTail`): a call its value makes is a frame at
  that instantiation, where a color no use decides is bounded, and a call in a branch
  ends the branch in a continuation over that instantiation's value (`KValue`, which a
  statement's own branches use too, `KStmt`). The places one instantiation's frames
  bound are unbound before the next lowers the same expression.
- **`@Run`** is Appendix B's Prism definition over `@Machine` and `@Defer`, compiled like
  any widget and inlined into its caller's node: the job is a `Hold`, each cycle fires
  `!turns`, a `More` arms the `@Defer`, a `Finished` fires the result.
- **`@Defer !e`** → an `rt::Defer<A>` field: `arm(v, node, cx)` in the firing tick, and
  in the next one `Node::deferred` reads `fired(cx)`. An armed `@Defer` or a non-empty
  started list sets `TickOut::pending`.
- **`@Later !e`** → a `Time.After` at the block's next position (a place of the block,
  as an inlined widget's is) and a `prism_rtl::LaterQueue<A>` field, the firings whose wake
  has not been answered, oldest first. Each firing of `!e` calls `queue(v)` and asks the
  place for a wake, the request field's record with a duration of nothing and the empty
  context; each answer at the place calls `answered()` and fires the oldest value. The
  place is made by `ElementPlaced`, the part of an element's line after its tag is
  known, which takes the request field's event from `LaterLine`. An answer with nothing
  queued panics.

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
