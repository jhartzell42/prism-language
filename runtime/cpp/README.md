# `prism_rtl` in C++

*Written by Claude, 3 November 2026, on the `claude-self-hosting` branch. Work in
progress and subject to change: implementers use their discretion.*

This is the runtime library for the C++ target. It ports the Rust crate `prism_rtl`
(`runtime/rust/`), whose contract with the emitter is `claude/backend.md`. Names,
structure and semantics follow the Rust library. The headers carry the Rust module
headers' explanations, adapted to C++. The emitter that targets C++ reads this file,
then that contract.

- **C++20, two libraries.** It builds with g++ 13 and clang++ 18, with
  `-Wall -Wextra -Werror`, over RE2 (regular expressions, "Regular expressions" below) and,
  for every program and test binary it links into, jemalloc (pick AL, "The allocator"
  below), both found with pkg-config: `apt install libre2-dev libjemalloc-dev pkg-config`
  (Debian, Ubuntu), `brew install re2 jemalloc` (macOS).
- **Header-mostly.** The templates (cells, hosting, the `^Pod` and `^Show` traits,
  collections) live in `include/prism_rtl/*.hpp`. Everything else lives in `src/*.cpp`
  and builds into `libprism_rtl.a`.
- **One include:** `#include "prism_rtl/prism_rtl.hpp"`. The namespace is `prism_rtl`.

```
make -C runtime/cpp test    # g++, clang++, then ASan+LSan+UBSan (g++; clang++ too
                              # when its sanitizer runtime is installed)
make -C runtime/cpp check   # once, with $(CXX), into build/$(CONFIG)
make -C runtime/cpp lib     # build/$(CONFIG)/libprism_rtl.a
```

## Memory and the C++ forms of Rust's

| Rust | C++ |
|---|---|
| `Rc<T>`, `Weak<T>` | `prism_rtl::Rc<T>`, `prism_rtl::Weak<T>` (rc.hpp): plain counts, not atomic ("Threads" below), the value and its counts in one allocation (`make_rc<T>(..)`, or `RcPlace<T>` with a constructor the caller writes); `std::shared_ptr`'s spelling (`get`, `reset`, `use_count`, `lock`, `expired`, `rc_static_cast`), and `RcFromThis<T>` for `std::enable_shared_from_this` |
| `Rc<dyn Node>` | `NodeRef` = `Rc<Node>`; `Node` is an abstract class with virtual methods |
| `self: Rc<Self>` in a node method | a virtual method on `this`; `me()` (or `me_as<T>()`) is the node as a `NodeRef`, for putting it on a list. Nodes are always made by `make_rc` or `rc_placed` |
| `RefCell<T>`, `Cell<T>` | plain members (single-threaded); `Cx`'s read point and late fault, and a `Cache`'s value and stale point (its `get` is `const`, as Rust's is `&self`), are `mutable`. A node's fields are not: its `const` methods (level readers such as `label`) write none of them |
| `Result<T, Fault>` | `Result<T>` = `Expected<T, Fault>` (result.hpp: clang 18 over libstdc++ 13 has no `std::expected`) |
| `Result<(), Fault>` (`Done`) | `Done` = `Expected<void, Fault>`; `return {};` is `Ok(())` |
| `Err(f)` | `return unexpected(f);` |
| `?` | `TRY(expr)`, an expression (a GNU statement expression, which g++ and clang++ take); its operand is moved from, so it is a temporary (lowering refuses `?` of a place) |
| `panic!` (a bug of the compiler) | `panic(msg)` throws `Panic` |
| `Box<dyn FnOnce(&mut Cx) -> Done>` (`Later`) | `std::function<Done(Cx&)>` |
| `Rc<dyn Fn(..)>` (subscribers, reads) | `Rc<const std::function<..>>` |
| `Rc::new_cyclic` | `rc_placed` (over `RcPlace`), then the weak self set afterwards (`HostRow::make`) |

## Sharing

How a Prism value is shared between the places that hold it, and what the emitted code
pays for it (prismc/README.md, pick CH; the Rust runtime's forms in claude/backend.md,
"Types"):

- **A choice is its variant.** An emitted choice class holds `std::variant<Cases...>` by
  value, the case structs nested in it; `cases()` is the variant by reference, and a copy
  of the choice copies the variant, case struct and all, as Rust copies an enum. No
  allocation and no count belong to the choice itself. A record is a plain struct, copied
  by value.
- **One `Rc`, as in Rust.** A recursive payload (what the lowering puts behind `Rc`
  because the payload's type reaches its own, `Lam(Rc<Term>)`) is an `Rc<const T>`
  (rc.hpp), the same counted pointer that holds nodes and cells. `==` and `<=>` on an
  `Rc<T>` are `T`'s, through the pointer, and defined exactly when `T`'s are (a
  requires-clause), as Rust's `impl<T: PartialEq> PartialEq for Rc<T>` is: a value's `Rc`
  compares and orders by the value, so sharing stays invisible (Semantics ch. 11), and a
  node's or a cell's `Rc` has no `==` at all, since the node or cell has none. No pointer is
  compared first (a NaN stays unequal to itself). `<=>` is `std::strong_ordering`
  (`as_strong`), as the emitted comparisons are throughout: Rust's `Ord` is total. `Show`,
  `ToPod` and `FromPod` of an `Rc<const T>` go through to the value; `IsValue<Rc<const T>>`
  is `IsValue<T>`, and `rc(x)` of a value makes an `Rc<const T>`, of a node or a cell an
  `Rc<T>`. The alternative refused: a second class for values (`Val<T>`, wrapping an
  `Rc<const T>`) beside an `Rc` that compared pointers; two names for Rust's one type, and
  an `==` on nodes that compiled and meant something no Prism program asks.
- **Identity is `ptr_eq(a, b)`** (Rust's `Rc::ptr_eq`), always written out: a `Dynamic`'s
  equality (the same read and the same update event), a `Map`'s `same` and its equality's
  shortcut for one root, a compiled `Regex`'s `same`, a host document's entries, a
  per-key inner's cell. These are every place the runtime, the Worlds and the emitted
  programs compared `Rc`s, found by compiling all of them with `Rc`'s comparisons deleted.
- **`deref(x)`** is the model's `*x` where the printer does not know `x`'s type (a
  pattern's binder, a receiver), and Rust's auto-deref of a cell: through an `Rc`, the
  value behind it; anything else, `x` itself (Rust's `*` of a reference, which C++ has no
  need of). A temporary `Rc<const T>` comes back as a copy of the value (it may be the
  value's last owner), a temporary `Rc<T>` of a node or cell as the place its owner keeps.
  Where the printer knows the type, it writes `*x` (prismc/README.md, pick PT).
- **What the alternative cost.** Until 7 November 2026 a choice was a handle, a class
  holding `Rc<const std::variant<..>>`, and `Rc<T>` of any value type was `T` itself, so no
  payload needed a box and every `*` of the model was a no-op: one allocation and one count
  per choice value, three for `Pair (Num 2), (Num 5)` where Rust has two, and a record of
  choices was that many pointers.

## The allocator

- **AL: jemalloc for every C++ program.** An emitted program's Makefile links
  `$(pkg-config --libs jemalloc)` after everything else, as do this library's test binary,
  both Worlds' test and smoke binaries and the GUI bench's driver: jemalloc's `malloc`,
  `free` and `operator new`'s calls of them stand in for the C library's for the whole
  process (on Linux by symbol interposition: `libjemalloc.so` is a `NEEDED` library before
  libc). Nothing in the runtime or the emitted code names it; it is a link-time choice.
  The Rust target makes the same choice in its own way (runtime/rust/README.md, pick AL).
  What it buys: a third of prismc's self-compile was glibc's `malloc` and `free` (their
  arenas, `malloc_consolidate`, `unlink_chunk`), the allocation of choice payloads, map
  and array nodes, texts, counts' blocks and job frames, which jemalloc's thread caches
  and size classes serve faster (claude/benchmarks-preliminary.md, "jemalloc (C++)").
- **No silent fallback.** Where pkg-config does not find jemalloc the build stops and
  names the package: the emitted Makefile with `$(error ..)` when it is read, since its one
  product links; this library's, the Worlds' and the GUI bench's Makefiles at the link of
  a binary (`ALLOC_READY`, as `RE2_READY` is), since their libraries (`make lib`) link
  nothing and need no allocator.
- **Sanitizer builds keep the C library's.** AddressSanitizer replaces `malloc` with its
  own to find what it finds, and jemalloc beside it would take the allocations away from
  it: a build with `-fsanitize` in its flags links no jemalloc. The Makefiles here tell by
  `SANITIZE` (the `make test` sanitizer runs set it), the emitted Makefile by
  `-fsanitize` in `OPT` (prism's `PRISM_SANITIZE=1` puts it there). UBSan alone would run
  with jemalloc; every sanitizer run here is ASan and UBSan together, so the rule is the
  one flag.
- **The alternative refused:** `LD_PRELOAD` of jemalloc at run time, which a program run
  without it silently lacks, and which the bootstrap, the tests and the benchmarks would
  each have to remember.

## Threads

`Rc`'s counts are plain integers, as Rust's are, because no two threads ever touch a
counted value at once (D4: a program is single-threaded). Read in the C++ runtime, both
Worlds and the emitted `main`, the only threads are these, and each waits while the other
runs:

- **The emitted `main`'s big stack** (`on_big_stack`, templates/cpp_prelude.prism): the
  program runs on one thread with a 4 GB stack while the main thread blocks in
  `pthread_join`; nothing counted crosses (the arguments are a `std::vector` of
  strings).
- **The canvas World's `BigStack`** (worlds/canvas/cpp, `src/big_stack.cpp`): every tick
  and the program's teardown run on its thread, and the window thread (Xlib or AppKit)
  hands each job over under a mutex and waits on a condition variable until it is done,
  so the two never run at once and the mutex orders every count's change between them.
  The window thread reads the scene and the shared input queue (`Shared`) only between
  jobs.
- **The Xvfb smoke test's robot** (worlds/canvas/cpp/tests, Linux only) drives X from a
  thread of its own and touches no value of the program.

A World that ran a tick beside another thread holding counted values would need atomic
counts again.

## Module and type map

| Rust (`prism_rtl::`) | C++ (`prism_rtl::`) | Header |
|---|---|---|
| `cells::Tick` | `Tick` (`std::uint64_t`) | result.hpp |
| `()` | `Unit` | result.hpp |
| `num::Int`, `num::Nat` | `Int`, `Nat`: `div` → `std::optional`, `modulo`, `checked_sub`, `from_int`, `as_int`, `to_i64`, `+ - *`, `<=>`; `Int::parse` for literals | num.hpp |
| `num_bigint::BigInt` | `BigInt` (this library's own: 32-bit limbs, Knuth D) | num.hpp |
| `collections::Text` | `Text` (UTF-8: a byte range of a shared `std::string`; `len` in bytes, `view`, `concat`, `chars` decoded, `to_string`) | collections.hpp |
| the prelude's `upper`, `lower` (`str::to_uppercase`, `to_lowercase`) | `text_upper`, `text_lower` ("Upper and Lower" below) | text_case.hpp |
| `collections::Cursor` | `Cursor` (a `Text` and a byte index on a character boundary; `front`, `back`, `at_byte`, `at`, `step`, `step_back`, `before`, `after`, the last two views of the same buffer) | collections.hpp |
| `collections::Map<K, V>` (a persistent AVL tree, ported from this one) | `Map<K, V>` (a persistent AVL tree whose updates change this map's own nodes in place and copy only a node another map shares, as Rust's `Rc::make_mut` does): `get` → pointer, `mut_get` (the path owned), `contains`, `insert`/`remove` (this value), `inserted`/`removed` (new value), `unit`, ordered iteration | collections.hpp |
| `collections::Array<T>` (a persistent balanced tree of chunks, the same design) | `Array<T>` (a persistent balanced tree of chunks with a tail): `get`, `[]`, `mut_at`, `push_back`/`set` (this value), `pushed`/`updated`, `concat`, `take`, `drop_front` | collections.hpp |
| `pod::Pod` | `Pod`: factories `unit`, `boolean`, `integer`, `int64` (Rust's `Pod::int`), `floating`, `character`, `text`, `duration`, `record`, `choice`, `name`, `element`, `tuple`, `array`, `map`, `nil`, `some`, `ok`, `failed`, `path`, `shown`; `kind()` and `if_<kind>()` accessors; `structure`, `field`, `tag`, `as_text`, `as_f64`, `elements`, `to_string` | pod.hpp |
| `pod::F64`, `pod::ShownText` | `F64` (total order); a `Shown`'s text is `Pod::Shown::text` | pod.hpp |
| `pod::Segment`, `pod::Path` | `Segment` (`pos`, `gen`, `key`, `kind`, `number`, `key_value`, `is_pos`), `Path` (`root`, `child`, `at`, `join`, `segments` → `std::span`, `len`, `to_string`); `path_of(span)` is Rust's `Path(at.to_vec())` | pod.hpp |
| `pod::Fault` | `Fault` (`kind`, `message`, `answer`, `to_string`) | result.hpp |
| `pod::Error`; `Result<T, Error>` (`~a`) | `Error`; `Fallible<T>` = `Expected<T, Error>` | pod.hpp |
| `pod::ToPod`, `pod::FromPod` | the class templates `ToPod<T>`, `FromPod<T>` (specialize them, or give the type a member `Pod to_pod() const` and a static `Result<T> from_pod(const Pod&)`), with the helpers `to_pod(x)` and `from_pod<T>(p)`; for `Unit`, `bool`, `Int`, `Nat`, `uint8_t`, `double`, `char32_t`, `Text`, `Path`, `Pod`, `Error`, `std::optional`, `Fallible`, `Rc<const T>`, `Array`, `Map`, `std::tuple` of any arity | pod.hpp |
| `pod::mismatch` | `mismatch<T>(want, got)` | pod.hpp |
| `print::Show` | the class template `Show<T>` (specialize it, or give the type a member `void show(Printer&) const`); `ShowRef` is `&dyn Show`; an `any ^Show` is `Rc<const DynShow>` | print.hpp |
| `print::Printer` | `Printer`: `raw`, `text`, `choice(ctor, {args})`, `record(name, {{"f", v}})`, `tuple`, `array`/`array_of`, `map`/`map_of`, `option`, `prefixed`, `float64` (Rust's `float`), `duration` | print.hpp |
| `print::show`, `show_text`, `path` | `show(x)`, `show_text(x)`, `print_path(p)`; `float_digits(f)` is Rust's `{}` for `f64` | print.hpp |
| `read::value`, `pattern`, `path`, `Pattern` | `read::value`, `read::pattern`, `read::path` (each `Expected<_, std::string>`), `Pattern` | read.hpp |
| `cells::Hold`, `Stamped`, `Cache`, `Snapshot` | the same names, same methods (`Hold(init, cx)` for `Hold::new`) | cells.hpp |
| `cells::Port`, `Handle`, `Demand`, `Subscriber` | the same; `Port::subscribe` takes any callable `Done(const T&, Cx&)` | cells.hpp |
| `cells::Event`, `Dynamic`, `Live` | the same (aggregates `{owner, port}`, `{owner, read, updates}`, `{identity, level}`); `Event<Option<U>>::filter_some` is the free function `filter_some(e)` | cells.hpp |
| `cells::Defer` | `Defer` | cells.hpp |
| `cells::Job<B>`, `Turn<B>`; `Rc<dyn Job<B>>` | `Job<B>` (virtual `cycle() const`), `Turn<B>` (`more`, `finished`, `if_more`, `if_finished`); `JobRef<B>` | cells.hpp |
| `cx::Node`, `Done`, `Later` | `Node`, `Done`, `Later` (`answer` takes `std::span<const Segment>`; `edges` takes a `Visit`) | cx.hpp |
| `cx::Cx` | `Cx`, every method as in Rust; `request`'s field is `std::optional<std::string>` | cx.hpp |
| `cx::Runtime` | `Runtime`: `build`, `tick`, `close(std::move(cx))`, `compact`, `document`, `collector` | cx.hpp |
| `doc::Document`, `Edit`, `Patch`, `Request` | the same; `Edit::added`/`changed`/`removed` | doc.hpp |
| `gc::Collector` | `Collector` | gc.hpp |
| `host::Host`, `HostRow`, `HostOf`, `Rows`, `Scope`, `Link`, `Sink`, `Supplied`, `Blueprint`, `Supplier` | the same names. `Host::new` is `Host::make`, `HostRow::build` is `HostRow::make`. `Rows<E, A>` is an `Rc<Map<..>>` (`new_rows<E, A>()`). `Scope` is `{hosted}`, with `world()`, `in(h)`, `is_world()` | host.hpp |
| `host::AnswerOf<E>` | the concept `AnswerOf<A, E>`: `static Result<A> A::decode(const E&, const Pod&)` | host.hpp |
| the blueprint forms, `forward_on_none` | `plain_blueprint<E, A>`, `never_blueprint`, `event_blueprint`, `dynamic_blueprint`, `element_blueprint`, `forward_on_none` | host.hpp |
| `host::Entry`, `HostDoc`, `Follow`, `Watch`, `Level`, `dynamics_pod`, `dynamics_ports` | the same names (`Entry::new` is `Entry::make`, or `entry_new(what)` where the type is the context's); `Level` is a member `level_pod(cx)` (and `level_ports`) the free `level_pod(e, cx)` calls, a type without one going as its `to_pod` | host.hpp |
| `Fault::too_deep` | the same | result.hpp |
| `world::Program` | `Program` (virtual `tick`, `compact`); a program type `P` also has `static Built<P> build(std::span<const Pod>, const ReadyState&)`, where `Built<P>` = `Result<pair<unique_ptr<P>, TickOut>>` | world.hpp |
| `world::TickIn`, `TickOut`, `Outcome`, `ReadyState`, `ReadyMap`, `World`, `drive`, `EXIT_*` | the same; `Outcome::value(p)`/`fired(p)` with `kind` and `pod`; `ReadyMap::entries` | world.hpp |
| `world::scene::find_clickables`, `Clickable` | the same | scene.hpp |
| `world::test_world::TestWorld`, `Report`, `XorShift`, `print_tick`, `diff`, `strip_comment_header`, `COMPUTE_PATIENCE` | the same; `TestWorld().program<P>("name").check("name", lines)` | test_world.hpp |

**For the emitter.** An emitted node's static constructor is `make`, not `build`. In C++
`build` is the virtual build-list method, and a static of the same name would hide it
(g++'s `-Woverloaded-virtual`).

## Where it stands

- **Everything in `runtime/rust/src` is ported.** That covers cells, cx, doc, gc,
  num, collections, pod, print, read, host, the World protocol, the scene and the Test
  World, and `Snapshot` as the Rust one has it now: the structure (`current`) and the
  dynamic one tick behind it (`get`, `queue`, `published`). The CLI World in C++ is
  `worlds/cli/cpp` and the canvas World `worlds/canvas/cpp` (an X11 loop and a Cocoa
  loop), both beside this library; `./prism run --target cpp` runs a root under the one
  its row asks for.
- **The emitter's C++ is `prismc compile --target cpp`** (prismc/README.md, "The C++
  target"; claude/backend.md, "The C++ side"). What it needs beyond the library's types is
  `prism_rtl/lang.hpp`: the value slot of a `match`, `TRY`, the pattern helpers, `rc`,
  `share`, `upgrade`, `Fn`, `str`/`cat`, `deref`, `host_new`, and the constructors whose
  type the context gives. For it the library also has `Turn::Value`, `Port::Value` and
  `Job::Value`, and `Array::mut_at`/`Map::mut_get` (an `&` block's update in place).
- **The Test World prints byte for byte what the Rust one prints.** The hand-written
  counter, half counter and `@Normal` (Rust's `tests/counter.rs` and
  `tests/normalize.rs`, in C++) verify against `tests/conformance/*.expected`.
- **The tests** cover all of Rust's `tests/*.rs` and the modules' unit tests, ported to a
  small harness (`tests/check.hpp`). They add:
  - bignums against 400 Python vectors built to reach Knuth D's corrections
    (`tests/gen_bignum_vectors.py`);
  - bignums against `__int128` on 20,000 random operands, and identities on 2,000
    large ones;
  - Float spelling against 728 vectors from Rust's `{}` (`tests/gen_float_vectors.rs`),
    each read back to the same bits;
  - a forwarding host;
  - a host torn down with its rows, which frees everything.
  - persistence: a map's and an array's update never reaches a value sharing them;
  - `lang.hpp`: `TRY` on a named value leaves it as it was, a `Slot` of an option holds
    `nil` as a value.
- **All 57 tests pass** under g++ and clang++. They also pass under AddressSanitizer,
  LeakSanitizer and UBSan with g++, and LSan reports no leaks. That includes the zombie
  loop, which the collector frees, and the host teardown. Clang's sanitizer runtime
  (`libclang-rt-18-dev`) is not installed on this machine; `make test` runs clang under
  the sanitizers when it is.

## Arrays and maps: the costs

Each of Semantics Appendix B's promises for arrays and maps (the book's words first), `n`
the size of the collection and `m` of the second, against this library
(`collections.hpp`). The Rust runtime's `Array` and `Map` are the same designs
(runtime/rust/README.md).

- **An `Array`** is a persistent balanced tree of chunks with a tail. The tail is a chunk
  of the last elements, at most 64, in a vector that grows 1, 2, 4, ..., 64 slots while
  the array is small; a full tail goes into the tree whole. The tree's leaves are chunks
  of 1 to 64 elements and its nodes have two children, each with its length and height,
  balanced as an AVL tree is; two trees are concatenated by the AVL join and cut by joins
  along the path to the cut, and a single chunk that fits beside its neighbour merges
  into it. A binary tree of chunks rather than a radix tree: the book asks for `Append`
  "log (a relaxed radix balanced tree or the like)", and the AVL join keeps its balance by
  a local rule.
- **Storage:** at most `2n + 1` slots while the array is small, `n + 64` beyond, plus a
  node per chunk; an array from a `std::vector` holds exactly `n`, each element moved once.
- **`Len`, "constant":** kept (a count).
- **Indexing, "constant or log":** kept: constant in the tail (an array of 64 elements or
  fewer is all tail), log in the tree (its height, about 1.44 log2 of the chunks).
- **`Append`, "log":** kept: the AVL join, copying two chunks at most (a tail that fits
  beside its neighbour).
- **`Push`, "amortised constant":** kept, onto any array: at most the tail (64 elements)
  copied when another array shares it, and once every 64 pushes the full tail joined to
  the tree (one path) with no element copied.
- **`Each`, `Fold`: "`n·t`"; `Sum`: "linear":** kept (the iterator walks the chunks; an `&`
  block's `iter_mut` is linear too, every node made the array's own once).
- **`Take`, `Drop`, an element's update** (`set`, `updated`, `mut_at`: `&xs[i] <= v` in an
  `&` block): Appendix B lists no cost; Semantics ch. 5 asks that a statement on a shared
  part copy "the path from `&_` to the part once per block". Each is log here and copies
  one path and at most one chunk, the path once (after it the nodes are the array's own).
- **References:** an element's address holds while the array is read and while its other
  elements are updated in place; a push may move the tail's elements as its vector grows.
- **A `Map`:** an AVL tree of counted nodes, one per entry. `Insert`, `Remove`, `?Get` and
  indexing, `?First`: "log", kept (an update touches the path to its key, copying only a node another map shares). `Keys`,
  `Values`: "linear, in key order", kept. `Size`: "constant", kept (the root's count).
  `Map` of pairs: "`n` log `n`, the later pair winning", kept. `Union`: "the left
  winning; `m` log `(n+m)`", kept: `Map::united` puts the smaller map's entries into the
  larger, whichever side it is (`tests/test_maps.cpp` counts the key copies).
- **`==` on a `Map` or an `Array`** (Appendix B states no cost): linear at worst, and a
  node both sides share at the same place answers at once once its values are known to
  equal themselves, which the node remembers (`same`, forgotten by any change through
  `own`); so a value compared with a clone, or with an update of itself, costs about the
  path the update copied after the first comparison. A shared node holding a NaN (`Float`
  `==` is IEEE equality, Type System ch. 2) stays unequal, as the element-by-element
  comparison has it: pointer equality is never the answer, so sharing stays invisible
  (Semantics ch. 11). `tests/test_maps.cpp` and `tests/test_arrays.cpp` count the values
  compared.
- **The tests** (`tests/test_arrays.cpp`) check the behaviour at the sizes around a
  chunk's edges, the storage (`capacity`), and the costs, by counting the copies and moves
  of an element type that counts them.

## Doubts

- **HW: a row holds its host weakly.** In Rust, `HostRow::host` is an `Rc`, which closes
  the count cycle Host → rows → HostRow → Host. That cycle leaks under D5. Here the
  back-pointer is a `Weak`: the host outlives its rows, and a row that outlives its
  host panics as a bug. The test `a_host_torn_down_with_its_rows_frees_everything` holds
  it. *Ruling (the coordinator's):* weak, and Rust gets the same fix.
- **CW: persistent collections.** A `Map` is a persistent AVL tree (an update changes
  this map's own nodes on the path to its key in place, and copies a node another map
  shares, as Rust's `Rc::make_mut` does); an `Array` is a persistent balanced tree of
  chunks with a tail
  (this README's "Arrays and maps: the costs"). Copy on write, as this library
  first had it, made prismc compiled to C++ quadratic in its own lexer and checker: the
  program showed the cost. *Ruling (Claude's):* these two structures. A `Text` is a
  byte range of a shared UTF-8 `std::string` (a literal's, `Text::lit`, of the program's
  own static bytes, with no buffer), as Rust's is of an `Rc<str>` (Semantics
  Appendix B, Text: a cursor's `Before` and `After` are constant views); concatenation
  copies. A text is made only from well-formed UTF-8, as Rust's `str` is: other bytes
  are a fatal error (a `Panic` naming the byte), never U+FFFD, and the CLI World checks a
  file's bytes before it makes a text of them (`Fs.NotText`), so a cursor never meets a
  broken sequence (doubt TXU in prismc/README.md).
- **FT: Float ties.** On an exact tie between two shortest spellings, `std::to_chars`
  rounds to even and Rust rounds up. `float_digits` checks the exact expansion and
  rounds up. 300,000 random floats, ties included, agreed with Rust byte for byte; the
  committed vectors keep 728 of them.
- **UC: Unicode character classes.** The reader and the Test World use Rust's
  `is_whitespace`, `is_alphabetic` and `is_uppercase`. Here they are exact for ASCII.
  Above ASCII, the Unicode spaces count as whitespace, every other character counts as
  alphabetic, and none counts as uppercase. Transcripts' names are ASCII.
- **DB: failure wording.** A failed check's message names the check after Rust's
  `{:?}`, with patterns shown in the transcript's spelling rather than Rust's `Debug`.
  Failures sit beside the transcript, never inside it.
- **TW: a hold written twice.** The Rust debug build asserts as well as failing the tick.
  Here the tick fails (`Runtime.TwoWrites`) with no assertion.

## Upper and Lower

`include/prism_rtl/text_case.hpp` and `src/text_case.cpp`: `text_upper` and `text_lower`,
which the C++ prelude's `upper` and `lower` call (Semantics Appendix B, Text).

- **The statement** is the one both targets meet (`tests/text/README.md`): Unicode's
  default full case mapping, `SpecialCasing.txt`'s unconditional mappings over
  `UnicodeData.txt`'s simple ones (`ß` → `SS`, `ﬁ` → `FI`, `İ` → `i̇`); in `Lower`, `Σ` is
  `ς` where it is Final_Sigma (a `Cased` character before it and none after it,
  `Case_Ignorable` characters skipped both ways) and `σ` elsewhere; no locale's tailoring
  (no Turkish dotless `ı`) and no normalization. It is what Rust's `str::to_uppercase`
  and `str::to_lowercase` do, which the Rust prelude calls.
- **The tables** (`src/case_tables.inc`, 55,868 bytes) are generated by
  `tools/gen_case_tables.py` from the Unicode Character Database 17.0.0, the version of
  Rust 1.98's `std` (`char::UNICODE_VERSION`); `tools/README.md` says how to regenerate
  them. Two stages: a byte per block of 128 code points, then a record index per code
  point; a record holds each mapping as a delta or an index into the sequences of two or
  three code points, and the character's class for Final_Sigma. Every index is checked
  in bounds at compile time (`static_assert`).
- **The cost:** linear, one pass. Each character is decoded once and looked up in
  constant time; the result is one `std::string`, reserved once and moved into the
  text's buffer (`Text::of_utf8`), with no `Text` per character; the bytes before the
  first character that changes are copied when it is met, and a text in which none
  changes is answered itself, with no copy. Final_Sigma looks ahead over the
  case-ignorable characters after a `Σ` only, so each character is passed at most twice.
- **Undefined behaviour:** a `Text` is well-formed UTF-8, so a lead byte's length never
  runs past the end (checked anyway: a `Panic`, never a read past the buffer); bytes are
  read as `unsigned char`; a code point at or past the table's end (U+E0200) is not
  looked up; a delta is an `int32_t` sum of two values below 2^21, and the generator
  refuses a mapping that is not a scalar value.
- **The tests:** `tests/test_case.cpp` reads the shared `tests/text/case.txt` (the twin
  of `runtime/rust/tests/case.rs`), checks that an unchanged text comes back as itself
  and that the tables are 17.0.0's.

## Regular expressions

`include/prism_rtl/regex.hpp` and `src/regex.cpp`, the twins of the Rust runtime's
`regex.rs` and `regex_dialect.rs`: `Regex`, `RegexReplacement`, `Group`, `Match`, and
`match_at`, `find`, `matches`, `split`, `replace` behind `?Match`, `?Find`, `Matches`,
`Split` and `Replace`, with `Regex::parse` behind `?Regex`.

- **The dialect** is checked by this runtime's own parse (`claude/regex-dialect.md`, the
  Rust runtime's procedure function for function), which then spells the pattern for RE2;
  RE2 never sees the pattern as written. Its diagnostics are the Rust runtime's, byte for
  byte: the shared corpus `tests/regex/corpus.txt` is `tests/test_regex.cpp`'s.
- **RE2** comes from pkg-config (`brew install re2`; `apt install libre2-dev pkg-config`),
  and the build stops naming the package when it is missing. Only `src/regex.o` sees
  RE2's headers (as system headers, so `-Werror` does not judge them); `regex.hpp`
  includes none of them, so a program compiles without RE2's flags and links
  `$(pkg-config --libs re2)` only when it uses a regex.
- **RE2 under the sanitizers.** Homebrew's RE2 and abseil are built against the
  system's libc++, and the sanitizer run links Homebrew LLVM's: the test binary loads
  both. What crosses between them is `absl::string_view` (which is `std::string_view`)
  and RE2's `const std::string&` error, read in place; libc++'s ABI is the same in both
  (version 1); and every allocation is freed by the side that made it: this library
  allocates and deletes the `RE2` object, whose constructor and destructor, RE2's code,
  manage its insides. The corpus passes under ASan and UBSan that way.
- **Lifetimes.** RE2 sees a searched text as a view into its `Text`'s buffer for one
  `RE2::Match` call, which the caller's `Text` outlives; the submatches become byte
  offsets before the call returns; every text and cursor a match holds is a counted
  view of that buffer. An empty match RE2 reports inside a character (`\B` in `é`) is
  skipped, as the books say, and the search goes on at the next character.
- **The costs** are the Rust runtime's (its README): a search `p·n`, an anchored one `p`
  times the bytes it reads, iteration `p·n·(k+1)` at worst; RE2's `max_mem` is 1 GiB so
  that every pattern in the dialect compiles (the bound's worst, 10,000 atoms of
  `(?i)\pL`, takes about 3.5 s).
