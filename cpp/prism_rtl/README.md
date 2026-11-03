# `prism_rtl` in C++

*Written by Claude, 3 November 2026, on the `claude-self-hosting` branch. Work in
progress and subject to change: implementers use their discretion.*

This is the runtime library for the C++ target. It ports the Rust crate `prism_rtl`
(`prism_runtime_lib/`), whose contract with the emitter is `docs/backend.md`. Names,
structure and semantics follow the Rust library. The headers carry the Rust module
headers' explanations, adapted to C++. The emitter that targets C++ reads this file,
then that contract.

- **C++20, no dependencies.** It builds with g++ 13 and clang++ 18, with
  `-Wall -Wextra -Werror`.
- **Header-mostly.** The templates (cells, hosting, the `^Pod` and `^Show` traits,
  collections) live in `include/prism_rtl/*.hpp`. Everything else lives in `src/*.cpp`
  and builds into `libprism_rtl.a`.
- **One include:** `#include "prism_rtl/prism_rtl.hpp"`. The namespace is `prism_rtl`.

```
make -C cpp/prism_rtl test    # g++, clang++, then ASan+LSan+UBSan (g++; clang++ too
                              # when its sanitizer runtime is installed)
make -C cpp/prism_rtl check   # once, with $(CXX), into build/$(CONFIG)
make -C cpp/prism_rtl lib     # build/$(CONFIG)/libprism_rtl.a
```

## Memory and the C++ forms of Rust's

| Rust | C++ |
|---|---|
| `Rc<T>`, `Weak<T>` | `std::shared_ptr<T>`, `std::weak_ptr<T>` |
| `Rc<dyn Node>` | `NodeRef` = `std::shared_ptr<Node>`; `Node` is an abstract class with virtual methods |
| `self: Rc<Self>` in a node method | a virtual method on `this`; `me()` (or `me_as<T>()`) is the node as a `NodeRef`, for putting it on a list. Nodes are always made with `std::make_shared` |
| `RefCell<T>`, `Cell<T>` | plain members (single-threaded); `Cx`'s read point and late fault are `mutable` |
| `Result<T, Fault>` | `Result<T>` = `Expected<T, Fault>` (result.hpp: clang 18 over libstdc++ 13 has no `std::expected`) |
| `Result<(), Fault>` (`Done`) | `Done` = `Expected<void, Fault>`; `return {};` is `Ok(())` |
| `Err(f)` | `return unexpected(f);` |
| `?` | `PRISM_TRY(expr);` and `PRISM_TRY_LET(name, expr);` |
| `panic!` (a bug of the compiler) | `panic(msg)` throws `Panic` |
| `Box<dyn FnOnce(&mut Cx) -> Done>` (`Later`) | `std::function<Done(Cx&)>` |
| `Rc<dyn Fn(..)>` (subscribers, reads) | `std::shared_ptr<const std::function<..>>` |
| `Rc::new_cyclic` | `make_shared`, then the weak self set afterwards (`HostRow::make`) |

## Module and type map

| Rust (`prism_rtl::`) | C++ (`prism_rtl::`) | Header |
|---|---|---|
| `cells::Tick` | `Tick` (`std::uint64_t`) | result.hpp |
| `()` | `Unit` | result.hpp |
| `num::Int`, `num::Nat` | `Int`, `Nat`: `div` → `std::optional`, `modulo`, `checked_sub`, `from_int`, `as_int`, `to_i64`, `+ - *`, `<=>`; `Int::parse` for literals | num.hpp |
| `num_bigint::BigInt` | `BigInt` (this library's own: 32-bit limbs, Knuth D) | num.hpp |
| `collections::Text` | `Text` (characters as `char32_t`; `len`, `take`, `drop_front`, `concat`, `char_at`, `chars`, `to_string` in UTF-8) | collections.hpp |
| `collections::Map<K, V>` (`im_rc::OrdMap`) | `Map<K, V>`: `get` → pointer, `contains`, `insert`/`remove` (in place), `inserted`/`removed` (new value), `unit`, ordered iteration | collections.hpp |
| `collections::Array<T>` (`im_rc::Vector`) | `Array<T>`: `get`, `[]`, `push_back`/`set` (in place), `pushed`/`updated`, `concat`, `take`, `drop_front` | collections.hpp |
| `pod::Pod` | `Pod`: factories `unit`, `boolean`, `integer`, `int64` (Rust's `Pod::int`), `floating`, `character`, `text`, `duration`, `record`, `choice`, `name`, `element`, `tuple`, `array`, `map`, `nil`, `some`, `ok`, `failed`, `path`, `shown`; `kind()` and `if_<kind>()` accessors; `structure`, `field`, `tag`, `as_text`, `as_f64`, `elements`, `to_string` | pod.hpp |
| `pod::F64`, `pod::ShownText` | `F64` (total order); a `Shown`'s text is `Pod::Shown::text` | pod.hpp |
| `pod::Segment`, `pod::Path` | `Segment` (`pos`, `gen`, `key`, `kind`, `number`, `key_value`, `is_pos`), `Path` (`root`, `child`, `at`, `join`, `segments` → `std::span`, `len`, `to_string`); `path_of(span)` is Rust's `Path(at.to_vec())` | pod.hpp |
| `pod::Fault` | `Fault` (`kind`, `message`, `answer`, `to_string`) | result.hpp |
| `pod::Error`; `Result<T, Error>` (`~a`) | `Error`; `Fallible<T>` = `Expected<T, Error>` | pod.hpp |
| `pod::ToPod`, `pod::FromPod` | the class templates `ToPod<T>`, `FromPod<T>` (specialize them, or give the type a member `Pod to_pod() const` and a static `Result<T> from_pod(const Pod&)`), with the helpers `to_pod(x)` and `from_pod<T>(p)`; for `Unit`, `bool`, `Int`, `Nat`, `uint8_t`, `double`, `char32_t`, `Text`, `Path`, `Pod`, `Error`, `std::optional`, `Fallible`, `std::shared_ptr<const T>`, `Array`, `Map`, `std::tuple` of any arity | pod.hpp |
| `pod::mismatch` | `mismatch<T>(want, got)` | pod.hpp |
| `print::Show` | the class template `Show<T>` (specialize it, or give the type a member `void show(Printer&) const`); `ShowRef` is `&dyn Show`; an `any ^Show` is `std::shared_ptr<const DynShow>` | print.hpp |
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
| `host::Host`, `HostRow`, `HostOf`, `Rows`, `Scope`, `Link`, `Sink`, `Supplied`, `Blueprint`, `Supplier` | the same names. `Host::new` is `Host::make`, `HostRow::build` is `HostRow::make`. `Rows<E, A>` is a `shared_ptr<Map<..>>` (`new_rows<E, A>()`). `Scope` is `{hosted}`, with `world()`, `in(h)`, `is_world()` | host.hpp |
| `host::AnswerOf<E>` | the concept `AnswerOf<A, E>`: `static Result<A> A::decode(const E&, const Pod&)` | host.hpp |
| the blueprint forms, `forward_on_none` | `plain_blueprint<E, A>`, `never_blueprint`, `event_blueprint`, `dynamic_blueprint`, `element_blueprint`, `forward_on_none` | host.hpp |
| `world::Program` | `Program` (virtual `tick`, `compact`); a program type `P` also has `static Built<P> build(std::span<const Pod>, const ReadyState&)`, where `Built<P>` = `Result<pair<unique_ptr<P>, TickOut>>` | world.hpp |
| `world::TickIn`, `TickOut`, `Outcome`, `ReadyState`, `ReadyMap`, `World`, `drive`, `EXIT_*` | the same; `Outcome::value(p)`/`fired(p)` with `kind` and `pod`; `ReadyMap::entries` | world.hpp |
| `world::scene::find_clickables`, `Clickable` | the same | scene.hpp |
| `world::test_world::TestWorld`, `Report`, `XorShift`, `print_tick`, `diff`, `strip_comment_header`, `COMPUTE_PATIENCE` | the same; `TestWorld().program<P>("name").check("name", lines)` | test_world.hpp |

**For the emitter.** An emitted node's static constructor is `make`, not `build`. In C++
`build` is the virtual build-list method, and a static of the same name would hide it
(g++'s `-Woverloaded-virtual`).

## Where it stands

- **Everything in `prism_runtime_lib/src` is ported.** That covers cells, cx, doc, gc,
  num, collections, pod, print, read, host, the World protocol, the scene and the Test
  World. The CLI and canvas Worlds are crates of their own in Rust (`worlds/`), and
  are not part of this library.
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
- **All 38 tests pass** under g++ and clang++. They also pass under AddressSanitizer,
  LeakSanitizer and UBSan with g++, and LSan reports no leaks. That includes the zombie
  loop, which the collector frees, and the host teardown. Clang's sanitizer runtime
  (`libclang-rt-18-dev`) is not installed on this machine; `make test` runs clang under
  the sanitizers when it is.

## Doubts

- **HW: a row holds its host weakly.** In Rust, `HostRow::host` is an `Rc`, which closes
  the count cycle Host → rows → HostRow → Host. That cycle leaks under D5. Here the
  back-pointer is a `weak_ptr`: the host outlives its rows, and a row that outlives its
  host panics as a bug. The test `a_host_torn_down_with_its_rows_frees_everything` holds
  it. *Ruling (the coordinator's):* weak, and Rust gets the same fix.
- **CW: copy-on-write collections.** Rust's `Text` is a rope and its `Map` and `Array`
  are persistent trees, which keeps Appendix B's logarithmic `Take`, `Drop` and updates
  under sharing. Here a copy is a pointer copy, and an update of a shared value copies
  the contents, which is linear. An update of a unique value works in place, which is
  what `&` blocks need. *First ruling (Claude's):* copy on write until a program shows
  the cost. *If otherwise:* a rope and HAMT/RRB structures behind the same interfaces.
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
