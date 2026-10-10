# Chapter 16 — Future work and research

Nothing in this chapter is part of the specification, and nothing in it may change a transcript. That second sentence is the whole discipline of the chapter: every item below is either a representation or scheduling choice behind semantics that are already fixed, or a language extension that adds programs without changing what any existing program means. Each item says what it is, the afternoon that makes someone want it, its design as far as it has been taken, the things it must leave alone, and what would make it worth starting. The order is roughly the order in which the pressure to do each one will arrive: a profile first, then a large record, then a binary to ship, then a cycle found in production, then a program too big for one file.

## How to read a future-work item

A reactive language with two implementations and a conformance suite has one asset that nothing here may spend: the transcript. A Prism program under the Test World produces a sequence of element lists, request firings and results, determined by its inputs and nothing else. The Rust runtime and the Haskell evaluator are checked against each other by diffing those transcripts, and the suite is what makes "the spec is the thing" more than a slogan. So the test every item below has to pass before any other is: does the differential test still come out empty? For the representation and backend items the answer must be yes by construction, since Prism values have no identity and a program cannot tell whether a value was copied, shared, interned or reused. For the language extensions the answer must be yes for every program that compiled before.

The second thing each item has to answer is what triggers it. The plan chapter ends with a list of things that are not on it, and the reason is not that they are unimportant but that each one waits on a fact that does not exist yet: a profile showing the value enum is the cost, a program that cannot be written without `update`, a World that cannot ship with the frontend inside it. Doing any of them before the fact is in hand means designing against a guess, and every one of them is large enough that a wrong guess is expensive. The triggers are written down so that when the fact arrives, the decision is already made.

## Values representation

You have profiled the runtime and the top of the profile is not propagation. It is `clone` and `drop` on a 24-byte enum, an atomic increment for every event payload that passes through a map, and an allocation every time a float crosses from one node to the next. The graph is doing what it should; the values riding on it are the cost. This item is the design of the value representation that replaces the enum, in full, with the small part that should be done first and the longer list of things that become possible once the base is in place.

### What the runtime has and what it must keep

Every value the runtime touches goes through one module. The rest of the runtime sees a typed wrapper and an erased form, and reaches the inside only through checked downcasts. The source today spells those `Value<T>` and `AnyValue`; the boundary's names, and the names this chapter uses for the design, are `Typed<T>` for the wrapper and `Value` for the erased word. Inside is an enum over an `Arc<dyn HeapValue>`, a `u32`, an `i32` and a unit. It is 24 bytes on x86_64. It was written to be replaced: the module's own comment says the implementation is meant to change in a way the rest of the runtime adapts to, and the API it exposes has no introspection and no fallible downcast, so nothing outside the module depends on how a value is laid out.

Three properties of that API are the contract the replacement keeps. A downcast to the wrong type is an error, not a reinterpretation; events and dynamics are values like any other and can sit inside records and element fields; and generic code in the runtime works on `Value` without knowing what it holds. The replacement is free to make all three cheaper. It is not free to weaken the first in a release build, because the producers of values include the frontend, the closure compiler, native components and whatever comes back from a World, and the first one to get a type wrong should hit a check, not a wild pointer.

### One word, three tag bits

A Prism value is one machine word. Its low three bits say what it is: zero means the rest of the word is the address of a heap object; the other seven patterns are immediate kinds with the payload in the high bits.

| low bits | kind | payload |
|---|---|---|
| 0 | heap pointer | address of a header |
| 1 | `Int` | signed integer |
| 2 | `Nat` | unsigned integer |
| 3 | `Char` | code point |
| 4 | `Bool` | 0 or 1 |
| 5 | tag | unit, or a payload-free choice variant's index |
| 6 | `Nil` | none |
| 7 | `Float` on 64-bit targets (below); unassigned on wasm32 |

Heap objects are allocated with eight-byte alignment on every target, because they all start with one header type declared `align(8)`; wasm32 would otherwise give four and leave only two bits. So a heap pointer ends in three zero bits on both targets and the seven other patterns are free. The zero word is never a valid value, so `Option<Value>` is one word through the same niche `NonZeroUsize` has, which matters because optionals are everywhere in node state.

```rust
pub struct Value {
    word: NonNull<()>,
    #[cfg(debug_assertions)]
    shadow: &'static TypeInfo,
}
```

The word is a `NonNull<()>` and not a `usize` so that heap pointers keep their provenance. Heap words come straight from the allocation. Immediates are built from an integer address and retagged with the pointer's own provenance, the strict-provenance style (`NonNull::without_provenance` and `map_addr`, from memory) that Miri checks, and Miri is the test that stands between this module and the rest of the runtime.

`Bool` has a kind of its own because it is the most common choice type and a `match` on it should be a mask and a compare. `Nil` has a kind of its own for the optional rule below.

Why three bits and not one. A one-bit scheme, the kind OCaml uses (from memory), lets the static type say which immediate a word is. Three is better here because a release build never decodes a word as the wrong kind, whatever produced it. For an immediate a kind check is a mask and a compare on a word already in a register, so there is no speed case for dropping it, and three bits also let a field of an element record say its own kind without a type beside it. What three bits cost is two bits of integer range, and the integer scheme below makes that a matter of when an integer goes to the heap, not of what integers the language has.

### Integers keep their meaning

`Nat` is `u64` and `Int` is `i64` on every target, overflow panics, and none of that changes. Tag bits belong to the representation, and the language does not shrink to make room for them.

The runtime keeps an `Int` in the word when it fits beside the tag: −2^60 through 2^60 − 1 on 64-bit targets, −2^28 through 2^28 − 1 on wasm32. Otherwise the `Int` is a heap object with an `Int` descriptor holding the full `i64`. `Nat` works the same way on its own tag. The form is canonical: a value that fits is always immediate, never boxed. So two immediates are equal exactly when their words are, a boxed integer never equals an immediate one, and equality on integers is a word compare followed, only when both words are heap pointers, by a descriptor call.

The worry is that every arithmetic operation now needs its own overflow logic, since Rust's checked operations know nothing about a 61-bit bound. They do not, because the payload sits in the high bits, so the tagged word overflows exactly when the payload does. If `a = 8x + 1` and `b = 8y + 1` then `(a ^ 1) + b = 8(x + y) + 1`, and that overflows the machine word exactly when `x + y` leaves the immediate range:

```rust
const INT: isize = 0b001;

fn int_add(a: &Value, b: &Value) -> Value {
    if let (Some(x), Some(y)) = (a.small_int_word(), b.small_int_word()) {
        if let Some(sum) = (x ^ INT).checked_add(y) {
            return Value::from_int_word(sum);
        }
    }
    let sum = a.int().checked_add(b.int()).expect("Int overflow");
    Value::from_int(sum) // immediate if it fits, boxed otherwise
}
```

Subtraction is `a.checked_sub(b ^ INT)`; multiplication is `(a >> 3).checked_mul(b ^ INT)` with the tag put back. When the fast path overflows, or either side is boxed, the slow path works at the full 64 bits and panics only where the specification says to. Dividing two immediates always gives an immediate except −2^60 / −1, which boxes like any other large result; the only overflow left in division is `i64::MIN / -1`, as in Rust.

The cost is allocation, and it is worth being exact about where. On 64-bit, an integer allocates only past 2^60, which is hashes and little else. On wasm32, anything past about 268 million allocates, and that includes Unix time in seconds, so in a browser World those values are heap objects. One tag bit would not save them: 2^30 is about 1.07 billion, and current Unix time is past 1.79 billion. `Int` is 64 bits on every World, so a program means the same thing in a browser as on a desktop; on wasm32 the values past 2^28 box, and the cost is time, never meaning.

### Heap objects

A heap object begins with a reference count and a `&'static Descriptor`. The descriptor holds a type id, a debug name, drop, eq, hash and debug functions, and a layout saying which of the object's words are values to be counted and which are raw data.

Clone never calls through the descriptor. It tests the low bits: nonzero means copy the word; zero means increment the count at a fixed offset from the pointer, inline. That is the increment `Arc` does, with the same release decrement and acquire fence on drop (from memory). The descriptor is called only when a count reaches zero and for eq, hash and debug. Whether the count has to be atomic at all is the first entry under the optimisations below, and the answer is the same for one word, two words or an enum.

Text, arrays, maps, records, choice variants with payloads, closures (a code id plus captures, one descriptor per closure site), large integers and boxed floats are heap kinds. Events and dynamics are boxed today and stay boxed, with a header and descriptor where `Arc<dyn HeapValue>` is now. Text is one allocation: header, length, bytes. Not a header pointing at a separate buffer. Text literals become static objects with no allocation at all, which is most of the text in a user interface.

Where a `&'static Descriptor` comes from depends on who made the value, and the first reader implementing this hits the question at closures. The runtime's own kinds (text, arrays, maps, events, dynamics, boxed numbers) have descriptors written as Rust statics. A closure site, a record type or an element layout from a compiled program needs a descriptor the Rust source never saw. The closure compiler builds those at load, one per closure site and one per interned layout, into a table it leaks for the life of the program; leaked memory is `'static`, the program is loaded once per process, and a World that loads many programs leaks one table per program, which is bounded by the number of programs and not by the number of values. A bytecode-loaded program is no different: its load step is the same build of the same table. The emitted-Rust backend below writes the same descriptors as statics in the generated source, so under that backend nothing is leaked and nothing is built at load. Either way, every descriptor pointer a value holds outlives every value that holds it, which is the invariant the drop path relies on and the loader is bound to.

### Records keep their floats raw

Floats are the strongest case against one word, and the design has to meet it head on. In a UI language the numbers that change most are coordinates, sizes, colors, opacities and animation progress, all `f64`. An `f64` cannot sit beside a tag in a 64-bit word and does not fit a wasm32 word at all, so a float stored as a bare word is a heap object. If every UI float went through a word, every frame would allocate all over.

Most of them do not go through a word, because they live in records. A record's descriptor already needs a layout so that clone and drop know which fields to count, and that layout can mark a `Float` field as eight raw bytes (two words on wasm32). That is data, not code: one clone and one drop routine per record type, walking a list, which is the struct layout any compiled language has. A rectangle, point or color is one heap object with raw floats inside, cloned with one increment and no allocation.

Elements and the responses a World sends get the same treatment. An element value is a heap object: header, interned tag, a pointer to its layout, then the fields. The layout lists each field's name, type and storage, and `Float` fields are raw. The layout comes from the tag declaration, so a `Draw.Rect(x=rx, y=ry, w=rw, h=rh)` in a `Gui.Scene`'s draw list, with its fields statically `Float`, has a layout known at compile time, and so does the pointer record `Gui.Window` responds with. A `match` arm that names the pointer record and reads `x` as a `Float` checks the layout pointer and loads the raw bits, with no box anywhere between the World's input and the arithmetic. Layouts are interned for the life of the program; records with the same tag and fields share one wherever they were built; matching compares layout pointers and the result can be cached per pattern site. There is no second representation at the boundary: `@Element` stores the same words and raw floats the rest of the code uses, and an event or an element list held in a field is the same heap pointer it was outside, which is how `requests: !R` fields and the `children` field of `Ui.VBox` come for free.

### Where floats still box, and the immediate float

A bare `Float` travelling alone as the payload of `!Float` or `$Float`, or as the element of a generic array, is a heap object. An animated `$Float` allocates and frees once per firing. That is acceptable as a base, for three reasons, and then fixed on 64-bit by the last tag.

The first reason is proportion: a firing already propagates through the graph, runs subscribers and updates a held value; one allocation is in scale with that. The second is where floats usually live: a uniform representation can box a scalar as long as the places the scalar usually sits do not, and in Prism a float in a typed register inside a leaf or in a record field is not boxed, so the lone float is the exception and not the rule. The third is what the alternatives charge, below.

The fix: an `f64` has 11 exponent bits. Drop three, and any double whose exponent falls in an 8-bit range, which is roughly the range of an `f32` with the full 52-bit mantissa, fits beside tag 7 exactly, with no rounding. Zero gets its own encoding. Every other double (huge, tiny, infinities, NaN) stays boxed, and the form is canonical the same way integers are: a float that fits is always immediate. Coordinates, sizes, colors, opacities and animation progress all fit. Smalltalk-family 64-bit VMs have immediate floats on this principle (from memory). Decoding is a shift and a rotate; float equality goes through the decoded value, with NaN and −0.0 handled the way they have to be handled anywhere. It does nothing for wasm32, where 29 bits cannot hold a useful double, so browser Worlds keep boxing lone floats; and it uses the last free tag, which is why short strings do not get it (a 7-byte inline string on 64-bit and a 3-byte one on wasm32 is worth less than every animated value in the program not allocating).

The alternatives are two other layouts, and both are worth taking seriously before rejecting. Two words per value, a descriptor pointer and an 8-byte payload, is 16 bytes on both targets, never allocates an integer, never boxes a float, knows its own full type, and makes debug and release the same program. What it charges is 8 bytes on every value in every node's state, every event payload and every record field, which doubles a record of integers, and the second word repeats what a heap object's header already says. NaN-boxing makes every value 8 bytes on both targets, which is twice the word on wasm32; it narrows integers to fewer than 64 bits and ties pointers to the address bits a NaN payload has room for (48 or so, from memory), and it builds the whole encoding around doubles, which fits a language whose numbers are all doubles and not one with `Nat` and `Int` beside `Float`. Immediate floats on tag 7 get most of what either would give, every UI-range double on 64-bit, while keeping one tag scheme on both targets. NaN-boxing stays as the comparison point in the benchmark, so the choice is measured and not argued.

### Where generic code gets its types

Layouts need a type for each field, and a single language rule makes sure they have one: a field of an element may not have a type that mentions a type variable (bare `a`, `[a]`, `?a`, `fn a -> b`), and breaking that is a compile error. Every tag declaration therefore gives complete field types, and every element layout is known where the element is built. The case that would have needed help, a function generic in `a` putting an `a` into an element where the word cannot say `?Nat` from `Nat` or which choice type a tag index belongs to, cannot be written.

So generic code is erased. A register whose type is a type variable holds the one-word value and clones and drops it by its tag bits; the kind bits and a heap value's descriptor are all that clone, drop and downcast need. There are no hidden descriptor arguments and no monomorphization. The cost is that generic library code cannot wrap its own type parameter into an element; it takes elements already built, or is written at a concrete type. Elements are strongly typed by their tags, so handing a value of the wrong type to a tag's field is a runtime error at the element.

Suppose instead the language allowed `a` in an element field and had the compiler pass a type descriptor for `a` as a hidden argument to any function where a value of a type variable reaches an element constructor, spreading the requirement to callers the way a type-class constraint spreads. That is Haskell's dictionary passing, it works, and it does not reintroduce two words: the descriptor is an argument used once to pick a layout, not a second word on every value. What it costs is a second calling convention in the closure compiler and a requirement that spreads through signatures for the benefit of a use (a generic function building elements out of its type parameter) that no library in the GUI corpus has needed. If one appears, this is the design to reach for, and the rule above is the line it would move.

### Debug builds are two words

In a debug build a value is the release word, unchanged, plus a pointer to its full static type. Nothing needs an ABI yet, so there is no reason to squeeze the shadow into the word. Debug runs every check release does and also checks the full type at every downcast.

That makes debug and release slightly different programs, and the gap should be named exactly. The first word is bit-identical in both. Release checks every kind and, through the descriptor's type id, the full type of every heap value. What release cannot catch is confusion within an immediate kind, and the whole list is short:

| Within-kind confusion | Debug | Release |
|---|---|---|
| Two payload-free choice types swapped (a `Color` index read as a `Side`) | Shadow mismatch, panic at the downcast | Wrong answer: the index is read as the other type's variant; `match` bounds-checks it, so an index past the other type's last variant is a panic, not a jump |
| A newtype read as its base (`Px` as `Nat`) | Panic at the downcast | Wrong answer with the right bits |
| `?Nat` read as `Nat` where the value is `Some n` | Panic at the downcast | Wrong answer: `Some n` is `n`'s word, so the read succeeds; a `Nil` where a `Nat` is expected is a cross-kind confusion and panics in both |
| A boxed `Int` read as a boxed `Nat` | Panic at the downcast | Panic in release too: both are heap objects and the descriptor's type id differs |

Every entry is a wrong answer or a panic, never a memory error, because the kind bits say what may be done with a word and the one place an immediate indexes anything, a payload-free `match`, is bounds-checked with one compare. CI runs the suite in both profiles, and the tests that pin each row are in the table at the end of this section.

### Optionals and choices

`Nil` is its own kind. `Some x` is `x`'s word, unless `x` is `Nil` or an already boxed `Some`, in which case it is a one-field box. The rule depends on the value and not the type, so generic code over `?a` works without knowing `a`, and only nested optionals with a `Nil` inside, `Some Nil` in a `??T`, ever allocate. When `x`'s static type is neither an optional nor a type variable, `Some x` is the word with no check at all. Integer division returns `?Int` and allocates nothing on immediates, which matters because division is in every layout computation.

Units and payload-free variants are tag immediates holding the variant index. Variants with payloads are heap objects whose descriptor names the choice type. The kind check catches `Nil` where a value belongs and a value where `Nil` belongs; telling `?Nat` from `Nat` is the static type's job, the debug shadow's, or an element layout's.

### The urgent subset

The full design is worth doing only with a benchmark in hand, and the trigger below says what that benchmark is. Two parts of it pay before any benchmark, because they remove costs the current enum pays on every program and need no unsafe code:

- Inline `i64` and `f64` in the enum in place of the `u32` and `i32`. The enum grows to no more than its current 24 bytes, since the `Arc` variant already sets the size, and every integer and float stops allocating.
- Text as one allocation, with literals static.

Both are changes inside the value module behind the API the runtime already uses. Both are also steps one and two of the plan for the full design, so nothing done for them is thrown away.

### The plan for the full design

1. **Enum as reference.** Extend the enum to every kind: full-width `Int` and `Nat`, `Char`, `Bool`, tags, `Nil`, `Float`, `Text`. Put it behind a cargo feature. Write property tests against the public API only, so they run unchanged against the word later. Done when the runtime's suite passes on it.
2. **Header and descriptor.** Replace `Arc<dyn HeapValue>` with the header-and-descriptor object, still inside the enum; move events and dynamics onto it without changing their APIs; text becomes one allocation. Done when no `Arc<dyn ...>` is left in the value path.
3. **The word.** The `NonNull<()>` implementation: tag table, inline clone, drop through the descriptor, integers with the tagged fast path, the full-width slow path and canonical boxing. Done when the suite passes on both implementations, every `i64` and `u64` edge value round-trips boxed exactly when out of range, and Miri is clean with strict provenance.
4. **Debug shadow.** CI in both profiles from here. Done when a within-kind mismatch panics in debug and a cross-kind one panics in both.
5. **Optionals.** `Nil`, the `Some`-box rule, division returning a word. Done when nested-optional round trips pass and dividing immediates allocates nothing under a counting allocator.
6. **Records and choices.** Raw `Float` fields, payload-free variants as immediates, payload variants on the heap. Done when a record of four floats clones with one increment and no allocation.
7. **Element layouts in the compiler.** Interned layouts with field types and storage, interned symbols for tags and field names, matching by layout with a per-site cache, and the compile error for a type variable in a tag's field. Done when a pointer element goes from World to `match` arm without boxing a float.
8. **Measure, then switch.** Benchmark clone, drop, allocations and time per occurrence on the runtime harness for both implementations, with a float-heavy animation, a large keyed `for` list and a wasm32 build. The word becomes the default when its time per occurrence is no worse than the enum's on every case; the enum stays as the oracle.
9. **Immediate floats on tag 7**, benchmarked against a NaN-boxing prototype on the animation case.

The enum is the safe version. The unsafe code sits in one module behind the API the runtime already uses, and the enum stays as the thing the word is checked against. That is the normal order, ship safe and measure before going unsafe, and the plan keeps it.

### Other optimisations in the same bucket

The one-word design is a base. Each item below says what it buys, what it costs, and what has to exist first. None of them changes what a program means, because Prism values have no identity and nothing a program can do distinguishes a value that was reused from one that was copied. None of them should land before the measurement step above.

**Non-atomic reference counts.** Clone and drop of a heap value become a plain add and subtract instead of locked atomics; they are the commonest operations on heap values, so this touches everything. The cost is that a non-atomic count makes values `!Send`, which cannot be a quiet switch inside the value module. Two ways to do it: a cargo feature making the whole runtime single-threaded, with two builds to test; or a per-object sharing mark, where an object uses a plain count until it is shared with another thread and is marked for atomic counting then, which is one build and one branch per count update (Lean 4 does something of this form, from memory). The runtime is single-threaded, and the parallel variants later in this chapter are separate runtimes, so the feature is the fit, and it comes right after the `Rc`/`RefCell` conversion the runtime plan already has.

**An incremental element list.** The element list of a node is the concatenation of its own slots and its children's lists in tree order, and the first implementation rebuilds the whole thing when any slot changes. That is linear in the program's element count per occurrence that touches the list. The incremental version keeps the list as a persistent ordered map keyed by path, so that a slot created or removed in a `for` or `dyn` is one insertion or deletion, and the interceptor and World above see a map whose unchanged subtrees are pointer-equal to last occurrence's. The dynamic's value and change event are the same as before; only the cost changes. Paths are already ordered in tree order, so the map's order is the list order with no sort. This pays as soon as any program has a list of a few hundred items and a label that changes per keystroke, which is any text editor.

**Interned tags and field names.** `Ui.Button` and `label` become pointers to interned symbols, so matching a tag is a pointer compare and finding a field by name happens once per layout rather than per access. Tags written in source are interned at load; if code ever builds a tag from runtime text, that needs a runtime intern step with a table that grows. Step 7 assumes it.

**Arena allocation of nodes per widget.** Graph nodes are `Rc<dyn Impl>` with weak subscriber lists, and a `dyn` rebuild frees a subtree by dropping counts one at a time. Nodes held in a per-node-tree arena, addressed by generational index, drop the per-node count traffic and give locality, and node lifetime follows build structure exactly: a rebuild frees its subtree by freeing its arena block. Reference-counted graphs with feedback also have to be careful about cycles, which is why a node owns its cyclic placeholders as a unit; an arena has no such care. The cost is a large refactor of the runtime's core, and event and dynamic values in Prism still have to be one word, so the value becomes a heap object holding a node index. Of everything here this is the biggest change and the furthest out; it waits on a benchmark showing node churn from `dyn` and `for` is a cost.

**A per-occurrence arena for temporaries.** Values allocated during an occurrence that do not outlive it, a lone float consumed by the next node, could come from a bump allocator reset at commit. Anything that escapes into a hold, a switch target or node state must be copied out, so every store into persistent state needs a check, and holds commit at exactly the moment the arena would reset, so the order has to be right. This is the item most likely to turn out unnecessary once immediate floats and fusion are in.

**A fast path for `for` over a map that changed in one key.** The keyed list widget diffs the old map against the new one on every change, which is linear in the map. With persistent maps the diff can skip any subtree that is the same pointer in both, which makes the diff proportional to what changed. The common case, a hold folding one insertion or one update into a map, then costs one path from root to the changed key. This needs the collection representation below and the runtime's keyed list to use pointer equality on subtrees; it changes nothing about which blocks are built or torn down.

**Persistent maps and arrays.** Persistent maps and vectors (the `im` crate is the implementation choice; its structures give logarithmic updates with the rest shared, from memory) cover the hold case that in-place reuse cannot (below), and give the `for` diff above. Small maps should stay flat and switch representation past a size threshold, since persistent structures lose on point reads and small sizes. This is the one representation choice here that a World can observe indirectly, through the time a turn takes with a large list, which is to say not at all in a transcript.

**Copy-on-write in library collection operations, then compiler reuse.** An array push that checks the count and mutates when it is 1 is what `Arc::make_mut` does and needs no compiler help. Its limit is the hold: a fold over an event runs during the occurrence while the old value is still readable through the hold's tag, so the hold's own reference keeps the count at 2 and nothing is reused; the first write through the root copies the top level, one copy per occurrence per level touched, and every later write through the same level finds a count of 1. Reuse past that needs the runtime to know that the hold's only reader this occurrence is its own update, which is knowable from the graph in many cases, and a hook letting the hold hand its value to the update. The general form, compiler-inserted ownership so that a last use hands its value on instead of copying, is what Koka's Perceus and Lean 4 do (from memory); it needs the closure compiler to know which uses are last. The `update` blocks later in this chapter are the language feature this optimisation exists to make cheap.

**Fused map chains.** `f` mapped over `g` mapped over an event becomes one node running `f . g`: one subscription, one propagation step, and no intermediate payload, which removes the box when the intermediate is a lone float. It is safe only when the intermediate has no other subscriber, which the compiler sees at elaboration in most cases and the runtime's builder sees when a map subscribes to a map with one subscriber. The compiler already lowers one lifted expression to one leaf, so `$a * 2 + 1` is one map with one closure; fusion extends that across bindings.

**Typed numeric arrays.** An array whose element type is statically `Float` stores raw `f64`s with a descriptor saying so, the way records do. Vertex lists, plot data and audio buffers stop being arrays of boxes. Generic code is erased, so an array it builds gets the general layout; raw arrays come from code where the element type is static. Waits on the record layout machinery and a program that holds a large numeric array, which a form does not and a chart does.

**Layout-keyed field access caches.** Each field-access site remembers the last layout it saw and the field's offset, so a repeat read is one pointer compare and a load, which is the inline cache of the Self and JavaScript VMs (from memory). A small mutable cache per site in compiled code, which is fine on a single-threaded runtime and one more reason the threading decision comes first.

### Testing the representation

The enum is the oracle: every test runs against both implementations through the public API, so the word is checked against something simple enough to trust.

| Test | What it catches |
|---|---|
| Property tests on clone and drop, counted with a test descriptor recording drops | Leaks and double frees in the inline clone and the descriptor drop path; a heap value dropped while an immediate copy of the word is still around |
| Integer round trip for every `i64` and `u64` edge value (0, ±1, the immediate bounds and one past, MIN, MAX) | Off-by-one in the immediate range; a value that fits but was boxed, or the reverse, which breaks canonical form and therefore equality |
| Tagged `+ - * /` against an `i128` oracle on random mixed-width pairs and all edge pairs | Fast-path overflow detection disagreeing with the specification; −2^60 / −1 not boxing |
| Eq and hash across immediate and boxed integers, and across both implementations | A boxed integer equal to an immediate; hash disagreeing with eq |
| `size_of::<Option<Value>>() == size_of::<Value>()` in both profiles | Losing the niche, which would quietly grow every optional |
| Downcast mismatch: cross-kind in both profiles, within-kind in debug, boxed `Int` as boxed `Nat` in both | A release build reading a word as the wrong kind; the shadow not checking the full type; a descriptor type id that does not distinguish two boxed kinds |
| `match` on a payload-free variant index out of range, and `Some n` read as `n` in release | A within-kind confusion in release becoming a memory error instead of a wrong answer; the gap table above being wrong about any row |
| Nested optionals, generic and monomorphic | The `Some`-box rule collapsing levels that must stay distinct |
| Allocation counts under a counting allocator: immediate division, record of four floats, pointer element from World to arm | Hidden boxing where the design promises none |
| Miri with strict provenance over the module and its tests | Pointers rebuilt from integers, misaligned headers on wasm32, use after free in drop |
| The conformance suite on enum and word, debug and release | Anything the unit tests miss that changes a transcript |
| A wasm32 build of the suite | Assumptions about 64-bit words, 61-bit payloads or eight-byte native alignment |

Each optimisation brings its own test before it lands: immediate floats round-trip every double with the right encoding; reuse and copy-on-write are checked by keeping a second reference, updating, and comparing the old value; persistent maps run the same `for` diff tests as flat maps; fused maps produce the same event stream as unfused ones on the harness; the incremental element list is diffed against the rebuilt one on every occurrence of every suite program.

### What it must not change, and what triggers it

Nothing here may change a transcript, a type, or the meaning of overflow. The trigger is a profile: a suite program, or a real program under a GUI World, whose time per occurrence is dominated by the value module rather than by propagation. The urgent subset can go in whenever someone is in the module anyway. The full design waits for that profile, because without it the benchmark in step 8 has nothing to compare against and the word would be adopted on faith.

## `&` references, `read` and `update` blocks

You have a record with a record in it with a map in it, and you need to add one to a counter three levels down. In an imperative language it is a line. In a pure language it is a `let` per level, each one rebuilding the enclosing record with one field replaced, and the line that does the work is at the bottom of a staircase of plumbing. This is the one place a pure language is worse to write than an imperative one, and it is the one `update` blocks exist to fix. They are future work because nothing in the specification needs them, and because the semantics of references under erasure and copy-on-write are the part that has to be designed before the Rust implementation can commit to a representation.

### What the feature is

Three pieces, each small on its own.

A `read` block gives the pronoun over a value. `\read _:` opens a block in which `_` is the lambda's argument, `.x` reads a field of it, and `[k]` indexes it, as in any lambda or `for` body; `read v:` gives the same prefix `.x` and `[k]` over any value `v`, and an `update` block gives them over the value being updated, and `read p <- v:` binds a pattern. Only `let` statements and one result statement are allowed. The point is not saving characters. A function over a large value should read like a description of the value:

```prism
type Vector = record:
    x: Float
    y: Float
    z: Float

let Magnitude: fn Vector -> Float = \read _:
    let x = .x
    let y = .y
    let z = .z
    Sqrt (x * x + y * y + z * z)
```

An `update` block is an expression that evaluates to an updated copy of a value. Inside it, `&_` is a reference to the value being updated and `&_.x`, `&_[k]` are references into it. A reference can be set with `<=`, or combined with an operator: `+=`, `-=`, and so on for every operator the type has:

```prism
let Add: fn Vector, Vector -> Vector = \|a, b| update a:
    &_.x += b.x
    &_.y <= .y + b.y      # the same thing, spelled out
    .z += b.z             # the pronoun is implicit on the left too
```

The original `a` is unchanged. The block's value is the updated copy. Statements run in sequence, and a later statement reads the value as earlier statements left it, which is what makes the block worth having: `&_.x += 1` followed by `let t = .x` sees the new `x`.

References, `&T`, exist only inside an `update` block. A function may take a reference and return one, `fn &Vector -> &Float`, and that is a lens: it says where inside a `Vector` a `Float` lives, and applied inside an `update` it gives a reference you can set. A function may not take two references, return a reference it did not derive from its argument, or store one anywhere; a reference is a path into the value being updated, and a path only means something while the update is happening. `*r` dereferences for reading and gives the current value at the path.

Two conveniences fall out. A function whose result type is its first argument's type, `fn Vector, Vector -> Vector`, is an *update function*, and inside an `update` block it may be used as a statement with its first argument implied: `Add b` inside `update a:` means `&_ <= Add _, b`. And `for` inside an `update` block over a reference to an array or map yields a reference per element:

```prism
let AddByElement: fn [Float], [Float] -> [Float] = \|a, b| update a:
    for &_:               # `&_` inside the loop is a reference to a[ix]
        &_ += b[ix]
```

`ix` is the index of the current iteration; nested loops get `jx` and `kx`, and past that a helper function is better than another keyword.

### Why it is wanted

The use cases are the ones where a program keeps a large value in a hold and folds small changes into it. A text editor's buffer, a spreadsheet's cell map, a drawing's list of strokes, a game's entity table, a form with forty fields. Each one is a hold whose event is a merge of small edits, and the fold is a function from the old value and an edit to the new value. Today that function is written with functional update at every level:

```prism
type Doc = record:
    lines: Nat ==> Text
    cursor: Cursor
    dirty: Bool

let Insert: fn Doc, Edit -> Doc = \|doc, e|
    Doc(
        lines = Set doc.lines, e.line, (Splice doc.lines[e.line], e.col, e.text),
        cursor = Cursor(line = e.line, col = e.col + Len e.text),
        dirty = True)
```

Every field has to be named even when two of them are unchanged, the nested `Splice` is hidden inside the `Set`, and adding a field to `Doc` touches every function that builds one. The same function as an update:

```prism
let Insert: fn Doc, Edit -> Doc = \|doc, e| update doc:
    &_.lines[e.line] <= Splice .lines[e.line], e.col, e.text
    &_.cursor <= Cursor(line = e.line, col = e.col + Len e.text)
    &_.dirty <= True
```

The goal this serves is the one about seeing what moves. The hold's definition still lists every event that can change the document; nothing about that changes. What changes is that the function the hold applies says what it changes and nothing else, so reading it is reading the diff, which is what you wanted to read.

The second reason is performance, and it is the same reason reuse is in the optimisations list above. A functional update that rebuilds a record of forty fields to change one copies forty words; a map insertion copies the map unless the map is persistent. An `update` block that knows which path it changed can, when the value's count is 1, change exactly those words in place. The feature and the optimisation are one design seen from two sides.

### The semantics as far as they go

Everything about references can be said without mutation, and should be, because that is what makes them safe to add to a language with no assignment. An `update` block is a function from a value to a value. A reference is a lens, a pair of a getter and a setter over the root value. A statement `&p <= v` is `root <- set_p root v`. A statement `&p += v` is `root <- set_p root (get_p root + v)`. A `let` inside the block reads the root as it is after the statements before it. The block's value is the root after the last statement. Under this reading an `update` block is a fold of lens applications over the root, every value is immutable, and the block could be elaborated into exactly the staircase of functional updates it replaces. That is the reference semantics, and the Haskell implementation runs it directly.

Lenses nest. `&_.lines[e.line]` is the lens for `lines` followed by the lens for key `e.line`. A user function `fn &Doc -> &Text` is a lens written by hand, and inside an `update` it is applied like any lens: `&(CurrentLine _) <= "..."`. The map indexing lens has the insertion rule from the aggregate types: setting `&m[k]` on a map inserts if the key is absent, and reading `*&m[k]` gives a `?V`. The array indexing lens is a plain index, and setting it out of range is the same error as reading out of range would be, a `?` on the read side and an abort on the write side, which is the checked-arithmetic discipline applied to indexing.

Then the borrowing rules, which exist for one reason: so that an implementation may mutate in place. If two live references could alias, an implementation that wrote through one would have to know whether the other would see it, and the only safe answer would be to copy. So, inside one block: a reference is live from its creation until its last use; two live references must be disjoint, and the compiler must be able to see that they are. Two different fields of a record are disjoint. Two different literal keys or indices are disjoint; two keys that are variables are not known to be, and the second reference is an error while the first is live. A reference obtained through a user lens function borrows the lens's whole argument for its life, because the compiler cannot see inside the function. These are Rust's rules for `&mut` reduced to a block with no loops but `for`, no conditionals that keep a reference across arms, and no escape, which is a small enough fragment that the check is a pass over the block and not a borrow checker.

Copy-on-write is the implementation side, and the thing the Rust implementation has to get right. When the block starts, the root value has some reference count. If it is 1, which it is when the caller passed the block its last reference, the block owns it and every `<=` through a path with count 1 at each level writes in place. If any level's count is greater than 1, that level is copied once, the copy is written, and the parent is pointed at the copy; later writes through the same level find a count of 1. This is `Arc::make_mut` applied along a path. The observable behaviour is identical to the functional reading because no one else can see the write, and the borrowing rules are what guarantee that "no one else" includes the block itself.

The hold case is the one the reuse entry above describes: one copy per occurrence per level touched, which is already far better than a functional update's copy per field, and the runtime hook that lets a hold hand over its value when its only reader this occurrence is its own update removes the last copy. Nothing about `update` depends on that hook existing.

### What it must not change

An `update` block is an expression with a value. No reference escapes it; a reference is not a value and has no type outside a block. Nothing about occurrences is involved: an `update` block that appears inside a lifted expression runs when that expression's node runs, once per occurrence, over the value it was given, and the block cannot see or touch a hold, an event or a dynamic. Widget blocks do not get mutation; the only state in a program is a hold and the only way to change one is its event. A program that compiles today compiles unchanged, because `&`, `read` and `update` are new syntax and no old program has them.

The erasure question is the one that is not yet answered and is why the item is research: a lens through a type variable, `fn &[a] -> &a`, needs to write into a value whose layout generic code does not know. Under the one-word representation, where generic code clones and drops by tag bits and a record points at its layout, the setter can write a word into a field slot without knowing its type, so erasure and `update` are compatible, but the proof is a working implementation and not this paragraph.

### Why it belongs in the Reflex implementation first

The Haskell implementation interprets the typed AST, and a lens in Haskell is a first-class value with exactly the getter-setter semantics above: two lenses nest the way two functions do, and reading through one, writing through one and mapping through one are the three operations an `update` block needs (the `lens` library is the implementation choice; the form of its API is from memory). Adding `update` to the Haskell evaluator is adding an elaboration case that turns a block into a fold of lens applications, and the library does the rest. That makes the experiment cheap: the syntax, the pronoun rules, the typing of `&`, the borrowing check and the `for` form can all be tried on real programs and tested through the conformance suite before the Rust implementation commits to copy-on-write along paths and the compiler to a borrow pass. If the semantics change during the experiment, nothing in Rust has to change back. Once the suite has `update` programs with golden transcripts, the Rust implementation has an oracle, and the differential test says when its in-place version is right.

### Trigger

A program in the suite, or a library, whose fold functions are more plumbing than logic. The text editor is the likeliest first case, because its buffer is a map of lines and every keystroke is a nested update. Before that, functional update is adequate and the feature would be a guess about ergonomics.

## Native backends

A Prism program ships today as source plus the frontend, and runs through the closure compiler: a `Program { functions, blocks, tags, constants }`, a table of compiled blocks and leaf functions, and an `IrWidget { program, block, captures, span }` per widget whose `build` runs its block's steps in order against a builder, each step calling a runtime constructor and writing into an environment vector. That is the right first implementation because it has one intermediate form, the typed AST, and no second syntax. It stops being the right one in two situations. A World has to ship a program without a frontend in it: a browser bundle that should not include a parser, a device that cannot afford one, or a World that receives programs it did not compile and wants to check them before building. Or the closure compiler is measured to be the cost: a leaf that runs per occurrence through a tree walk and an environment lookup, when it should be three machine instructions. This item is what a native backend is, lowered from what, and what the differential test gives once there are two ways to run the same program.

### Register code as a lowering

The typed AST after closure conversion says what every binding means, with every lift explicit, every lambda a function id plus a capture list, every cyclic group with its placeholder and close nodes placed, and every binding marked build-time or occurrence-time. That is the one form the lint checks and the goldens diff. A native backend starts from it and nowhere else.

The tree does not say one thing flat, and a backend needs it: order. A widget function is a work order to the runtime, bind this, hold that, open a placeholder, close it, and the order is meaning. Register code is the tree with the nesting taken out of widget bodies: per function, a flat list of steps, every intermediate value in one named, typed register, builder calls in the order the builder sees them, and `match` kept structured with arms as blocks the way WebAssembly keeps control flow structured. It is a flattening of a compiled block's step list, not of a closure tree: the closure compiler's `CompiledBlock` already has its steps in group order over an environment indexed by position, and register code gives each environment position a name and a type and each leaf a signature over machine values. The counter's knot, the part of its work order where order is everything:

```
%count: $Int  = cyc.dyn "count"
%next:  !Int  = ev.tag_map %delta, %count, @add_int
%held:  $Int  = dyn.hold 0, %next
cyc.close %count, %held
%label: $Text = dyn.map %count, @fmt_count
```

Open, use, hold, close, then read. The tag reads the placeholder's old value, which is what keeps the hold from feeding on itself, and the listing shows it. Each register is named once and has one type, which is an SSA value; a widget function is a basic block; a leaf is a small function over typed registers that keeps `Int`, `Nat` and `Float` as plain machine values until they cross into the runtime as `Value`.

The instruction families are the runtime's combinators one for one: `bind` on a native or compiled widget; `ev.map`, `ev.tag_map`, `ev.leftmost`; `dyn.hold`, `dyn.map`, `dyn.map2`, `dyn.join`; a switch over an event of events; `cyc.dyn`, `cyc.event`, `cyc.close`; slot creation, the keyed list, `dynamic_widget` and `intercept` as natives by table index; `rec.make` and `rec.match` for elements, with layouts from the tag table. A text form prints and parses from the same instruction enum, so a person can write a fixture before the lowering exists and a golden can diff the lowering's output. A verifier runs after every lowering: register types agree, every open has one close, nothing is sampled while open, every capture names a free register. It is the lint of the closure compiler applied to the flat form, and it is what makes the lowering mechanical in the only sense that matters, that a mistake in it is caught before anything is built.

Lowering is where the closure compiler and the native backend part ways and it is deliberately late. Everything upstream, elaboration, closure conversion, the lint, the typed-AST text form, is shared. A bug found by the differential test is therefore in the lowering or the backend, never in the meaning, because both executors got the same typed AST.

### Cranelift

Cranelift is the native option for a desktop or server World that wants machine code without a C toolchain. Its IR is SSA with block parameters in place of phi nodes (from memory), which is a short step from register code: each register is a Cranelift value, a widget function is one block of calls, and a structured `match` becomes a conditional branch or a jump table on the tag with one block per arm and a join block taking the arm's result as a parameter. The frontend builder crate does the SSA construction (from memory); the emitter never writes a phi.

Every runtime call goes through a shim, one `extern` function per combinator, instantiated at `Value`:

```rust
pub type Leaf2 = unsafe extern "C-unwind" fn(
    env: *const Value, a: *const Value, b: *const Value, out: *mut Value,
) -> bool;

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn prism_event_tag_map(
    e: *const Event<Value>, d: *const Dynamic<Value>,
    f: Leaf2, env: *const Rc<[Value]>,
) -> *mut Event<Value> {
    // clone env; call e.tag_map with a closure that calls f
}
```

(The `"C-unwind"` ABI string and the `#[unsafe(no_mangle)]` attribute spelling are from memory; whichever spellings the toolchain in use takes, the requirement is the one below: a Rust panic must be able to cross the emitted frame.)

The closure the shim builds holds a function pointer and a captures array, which is the closure record the typed AST already produces with the function id turned into an address. A compiled widget is a `Widget` whose `build` calls the emitted build function and whose output is `Value`, so it sits inside a `dyn` beside closure-compiled and native widgets and the runtime cannot tell them apart.

The contract on emitted code is two rules, and the shim is the only place they are checked: emitted code borrows values through pointers and never owns one, so reference counts change only in Rust; and it initialises `out` exactly when it returns true. Emitted frames therefore own nothing and need unwind tables but no landing pads. A panic inside a leaf, an `Int` overflow, raises in Rust, unwinds through the emitted frame (which the unwinding ABI permits and the plain C ABI does not, from memory), and is caught by the runtime's `catch_unwind` like any other, which is what keeps the occurrence abort (the errors chapter) working unchanged under a native backend. Build-time handles that emitted code receives belong in an arena the Rust adapter owns for the length of `build`, so a panic mid-build drops them on the Rust side.

Cranelift has no wasm backend (from memory; its targets are the common native architectures). A browser World runs the closure compiler, which it would anyway because a playground that compiles Prism in the page needs the frontend there. Cranelift also offers a JIT, which is what a REPL or the hot reload item later in this chapter would use if the closure compiler's build time were ever the thing in the way; it is not expected to be.

### Emitted Rust

The other native option is to print the register code as Rust: each register a `let`, each structured `match` a Rust `match`, each builder call the same call the closure compiler makes, and each widget function a `build` method on a generated type implementing the runtime's widget trait. The output compiles with the runtime into one binary, and `rustc` inlines across the boundary, so a leaf that adds two integers is an add with an overflow check and nothing else. This is the backend for shipping: one `cargo build` produces a World with its program inside it and no frontend.

`rustc` checks the emitter's Rust and nothing else: that the generated code type-checks against the runtime's API. It does not check Prism's meaning. An emitter that writes `wrapping_add` where Prism means checked addition, or swaps the operands of a leftmost merge, produces Rust that compiles and a program that is wrong, and nothing in the Rust toolchain can say so. That is why emitted Rust is a differential backend and not an oracle: its correctness comes from running the suite and matching the closure compiler's transcripts, the same as Cranelift's.

### Other targets

LLVM is one possible target, and the only one that would reach every World including the browser, since the runtime's own wasm build goes through LLVM (from memory, by way of `rustc`). The form it would take is clear: the register code prints almost directly as LLVM IR, with each widget function a list of calls into the same unwinding shim and each leaf a function over `i64` and `double`. The pin on an LLVM version, the toolchain in the edit loop, and a second unsafe surface are costs that nothing yet justifies, and a program that has to be native on the desktop and also run in a browser is served by Cranelift for one and the closure compiler for the other. If a native backend is ever built, it starts from the register code and picks its target then; the choice of target is the last decision, not the first.

Bytecode is a different question and a smaller one. If compiled code ever has to outlive the process that compiled it, cached builds, precompiled libraries, a World that loads programs, the format is a serialization of the converted typed AST with a version and a load-time lint, decoded back into the tree the closure compiler already takes. Nothing has to be designed for that beyond a version field.

### What the differential test gives

The conformance suite runs each program under the Test World and records its transcript. With two executors of the same typed AST, every program in the suite runs under both and the transcripts are diffed, exactly as they are between the Rust and Haskell implementations. This is the reason a second backend is cheap to trust.

A mismatch between the closure compiler and a native backend is in the lowering, the emitter or the shim, because both executors share everything above the typed AST. A mismatch between the Rust implementation (either executor) and the Haskell one is in a frontend or in the semantics as one of them understood it. Those two diffs localise a bug to one of three places before anyone reads a line of code. And the suite's programs are the specification's programs, so a native backend that passes it has been checked against the laws, not against the closure compiler's accidents. The register code's text form adds a third, cheaper check: goldens on the lowering's output, compared through a normalising printer so that renamed registers do not show as diffs, catch a lowering change as a diff someone reads before any backend has run it.

### What it must not change, and what triggers it

The typed AST is the form everything is checked against, and no backend may need something from the frontend that the typed AST does not say. Transcripts are unchanged. The occurrence abort still works, which is the unwinding requirement on every shim. The runtime's API does not grow a second entry point for native code; the shim is a client of the API the closure compiler uses.

The trigger is one of the two situations the section opened with: a World that has to ship without the frontend, or a profile under a real World where the closure compiler's leaves are the cost after the value representation has been fixed. The first is the likelier, and it points at emitted Rust. Until one of them arrives, register code is a stage with no consumer, and a stage with no consumer is a form that drifts.

## Causality in types

A cycle in the prompt graph inside one widget block is a compile error with every binding in it listed, and that check has been enough for every program in the suite. Then you write a layout library. `@Pack` takes a list of children's size dynamics and returns a dynamic of positions; a child takes its position and returns its size; both are widget functions, each is acyclic inside its own block, and the first time a program wires a child's size from its own position through both of them, the cycle exists only at the call site, where neither function's body is visible. The compiler says nothing. The runtime finds it the first time the occurrence runs, a node scheduling below the current height, and aborts the occurrence with the spans of the two bindings at the call site and no explanation of why they form a cycle. This item is a type-level record of which arguments a function's result depends on promptly, so that the check inside a block can cross a function boundary and the runtime's detection becomes a backstop. It is an effect system in a small way, and it is research.

### The check as it stands

Inside a widget block, every binding is a node and every use is an edge, labelled prompt or delayed by the construct that makes it: lifting over an event or a dynamic, `|`, `for` consolidation and `dyn`'s rebuild are prompt; a tag, `!Switch`'s choice of inner event and a hold's value side are delayed. The resolver finds the strongly connected groups; the elaborator rejects any group whose cycle has no delayed edge. A widget function's body is one such block, so its own cycles are caught. What the check cannot see is an edge from an argument to the result: `@F !a` returns `!b`, and whether `!b` fires promptly on `!a` is a fact about `@F`'s body that its type `fn !A -> @!B` does not state. At the call site the elaborator must assume either that every result depends promptly on every argument, which rejects the layout library and most of the GUI corpus, or that none does, which is what it assumes now and why the cycle above is found at runtime.

### The annotation

A function type gains, for each sigiled result, the set of arguments it depends on promptly. Written out, in a form a signature could use:

```prism
@Button: fn $Text -> @!Click                       # result depends promptly on nothing
Scale:   fn !Int, $Int -> !Int    prompt !_1       # result fires promptly on the first argument, reads the second through a tag
@Pack:   fn [$Size] -> @$[Pos]  prompt $_1         # positions change promptly when any size changes
```

The default, with no annotation, is that the compiler infers the set from the body: a function's result depends promptly on an argument if there is a path of prompt edges from the argument to the result inside the body. Inference is a reachability pass over the same labelled graph the cycle check already builds, so it costs nothing new. Written annotations are for signatures the author wants to pin, the way a type annotation pins an inferred type, and the body is checked against them.

At a call site the elaborator then adds, for each argument the result depends on promptly, a prompt edge from that argument to the binding of the result, and the group check runs as before. The layout cycle becomes:

```prism
$positions <- @Pack [[$size]]        # prompt on $size, by @Pack's type
$size <- @Child (First $positions)   # prompt on $positions, by @Child's type
```

`First: fn [Pos] -> Pos` is a plain library function, lifted over `$positions`, and the lifted application is prompt like any lift. Two prompt edges and no delayed one, rejected with both bindings' spans and the two signatures that make the edges prompt. The fix is the one the author would have reached for after the runtime abort, a tag or a hold between the two, and now the message says so before the program runs.

The same cycle through an element is the case the annotation has to be exact about, because it is the case the GUI library actually has. The library's `@HBox` is `fn @a -> @a`: an interceptor over its child, with the child's `Ui.*` elements in a `Ui.HBox(children=...)` element it puts out. A layout interceptor above it supplies positions as state to the children, so a child that wants its position asks for it through a tag:

```prism
tag Ui.Placed of Element:
    size: Size
    state: Pos
```

A child writes `$pos <- @Element Ui.Placed(size=$size)`, and a dynamic in a field lifts into a `dyn`: a fresh slot per size change, with the slot's state read from the supplier the layout interceptor built. If the child then computes `$size` from `$pos`, the loop runs through the element list: `$size` changes, the `dyn` rebuilds the slot promptly, the interceptor's `$slots` changes promptly, the supplier's state dynamic for the new slot is built from `$slots` and read into `$pos`, and `$pos` feeds `$size`. Every edge in that loop is prompt, including the one that looks as if it might not be, inside `intercept`: the state a slot sees is the supplier widget's result, and the supplier is built in the interceptor's node at slot creation, as a widget built mid-occurrence that sees the occurrence's inputs, so it is prompt on `$slots`. So the loop is a real cycle and the runtime finds it today. The annotation that catches it at compile time is on `@Element`: for a state tag, the returned `$State` is prompt on the slot's element value, and the element value is prompt on every dynamic that went into a field, because each one is a `dyn`. With that one rule the elaborator sees `$pos` prompt on `$size` across the `@Element` boundary, and the message names the field. The two-function form above and the element form are the same cycle; the element form is the one where the type has to say something at a boundary the program did not write.

### What makes it research

Three places where the small version stops being small.

*Higher-order functions.* `Map: fn (fn a -> b), !a -> !b` is prompt on its event argument, and whether it is prompt on anything else depends on the function passed in, which has no sigiled arguments and so no dependency set of its own. The clean answer is that a plain function's dependency is empty, since a plain value has no time in it, and the only prompt dependencies come from sigiled arguments. That holds for every prelude function. It breaks when a function argument is a widget function, `fn (fn $A -> @!B), $A -> @!B`, whose dependency set is a variable the caller fills in. That is dependency polymorphism, and it is the point at which the annotation becomes an effect system with variables and inference becomes unification over sets. It is tractable; it is also where the design has to stop being a reachability pass and become a type system, and the payoff should be measured against how often widget functions take widget functions in real libraries.

*Switching.* A `dyn` over `$x` returns an event that fires promptly on `$x`'s change; the block's result is built fresh each time, and what the result depends on inside the block is the block's business and acyclic by the block check. So `dyn` contributes exactly one prompt edge, from its dynamic to its event, and the block's dependency on captured outer dynamics is prompt through them as well. `!Switch` is delayed by definition and contributes nothing. `for` is prompt on its map and its consolidated result is prompt on whatever the block's result is prompt on, through the per-key `$_`. None of these needs the annotation to say anything new, but the inference pass has to understand them, and the construction-in-the-middle rule means a cycle formed by a rebuild is a cycle the new block's result makes at the `dyn`'s call site, which the annotation on the block's captured names covers.

*Interceptors.* The recursive binding every interceptor has, `!down` defined from `$slots` which `intercept` produces from `!down`, is legal because routing reads the list as the occurrence opened, which is a tag. `intercept`'s type has to say so: its result list is prompt on the child and delayed on the responses argument. That is one annotation on one primitive and it is exact.

### What it must not change

No program that was legal and never hit a runtime cycle becomes illegal, which is soundness in the direction that matters, and it holds wherever the inference can see a body: there the annotation only adds prompt edges that exist, so a rejected program is one that would have aborted on the first occurrence to exercise the cycle. Where the inference cannot see a body it over-approximates, and the two places are named so a reader knows where a rejection may be of a program that would never have aborted. A widget parameter, `@child` in `fn @a -> @a`, has no body at the call site, so its result is treated as prompt on every sigiled argument the caller passed it. A `for` over a widget parameter whose body is not visible is treated as able to fire at birth, which is the rule the static cycle check has already and this pass inherits: that adds a prompt edge from the collection's change to the consolidated result that may not exist. In both cases the written annotation on the parameter is what the author writes to get the program accepted: `fn (@a prompt nothing) -> @a` says the child's result depends promptly on nothing, the elaborator checks the argument's inferred set against it at the call site, and the body of the function is checked under that assumption. That is the same bargain a type annotation on a higher-order parameter makes, and it keeps the default conservative without making it final. The runtime's detection stays, because a cycle formed through a path the type system does not track, a function value stored in a record and applied later, for instance, still has to be found and still has to abort the occurrence rather than loop. The typed AST gains a field on function types and nothing else changes in it; the Haskell implementation can ignore the field, because Reflex finds the cycle at runtime the same way the Rust runtime does, and the differential test is unaffected because an accepted program means the same under both.

### Trigger

Runtime cycle errors in practice: a library whose users hit the abort at call sites and have to read two function bodies to understand it. The GUI library's layout widgets are where it will happen first if it happens. Until then the block check plus the runtime backstop is the design, and the annotation is a note about where the type system would go next.

## User-defined traits and modules

Two pieces of ordinary language work, with no bearing on the semantics of time, deferred for exactly that reason: they can be added when a program needs them without any law changing, and a design that lands them early is a design that spends its attention on the wrong thing while the laws are still being tested. What follows is what each would look like, so that when the need arrives the work is known.

### Traits

You have a `Money` type, or a `Vector`, and every sum in the program is spelled `Add a, b` where you would write `a + b`, because the numeric trait reaches only the built-in number types. Comparison is already there: a record is ordered by its fields in declaration order, so `a lt b` on two `Money` values works today, with the derived order rather than one the type's author chose. The language has three built-in traits, equality, ordering and numeric, which the checker resolves at each use to the implementation for the operand type and passes as a hidden argument where a generic bound needs one. That mechanism is a trait system with no syntax for declaring a trait or adding a type to one. User-defined traits are the syntax.

```prism
trait Show a:
    Show: fn a -> Text

impl Show Vector:
    Show = \read _: "(${.x}, ${.y}, ${.z})"

impl <a: Show> Show [a]:
    Show = \|xs| Concat [["[", (JoinText ", ", (for xs: Show _)), "]"]]
```

The elaborator treats a trait as a record of functions, an `impl` as a value of that record, and a bound `<A: Show>` on a generic function as a hidden record argument, resolved at each call site by the operand's type. That is dictionary passing, the same thing the three built-ins do, and it is why generic code stays erased under the one-word representation: a trait method on a type variable is a call through the dictionary, which is a plain record of closures, and the value itself needs no descriptor.

What traits do not do, and the exclusion is the design: no traits over the functors. There is no `Functor` or `Monad` trait, no user-defined block kind, no instance that says how `!` lifts. The sigils fix the set of functors and the lifting rule is the language's; a trait that could change how `$a + !b` elaborates would make the sentence "you can see what moves" false, because what moves would depend on an instance in scope. Traits are over plain types only, and a sigiled type (`!Int`) may not be an instance. Coherence is the simple rule: one `impl` per trait per type in a program, and overlap is an error at the point both are in scope, which the module system below makes a concrete place.

What traits buy, in order of how soon it will be wanted: `Show` for debugging and labels, `Default` for a record with many fields, a `Draw` trait in the GUI library so that a container can take `[Drawable]` without a closed choice of what can be drawn, and the user-defined numeric types (a `Money`, a `Px`) that make `+` mean something for them without a suffix. The last is also the one place the deferral has a visible cost today: a program with a `Vector` type writes `Add a, b` where it would write `a + b`, which is where this section began.

### Modules

One file plus the prelude is the whole of the current module story. Anything larger than one file needs a way to split, and anything meant to be reused by another program needs a way to say what it exports.

```prism
module Ui.Layout

import Draw (Rect, Color)
import Ui.Focus as Focus

export @HBox, @VBox, Size, Pos
```

A file is a module and names it on its first line. Imports bring in a module's exports, optionally renamed; exports list what is visible outside. Tags are already namespaced, `Ui.Button`, and a module's tag declarations live in its namespace, so `tag Button of Element` inside `module Ui` declares `Ui.Button`, which makes the tag namespaces and the module namespaces one thing. Instances are exported with their trait or their type, whichever the module defines, and the overlap check runs at import.

What the typed AST needs: qualified names. Every binding in the text form is already named; a module system makes the name `Gui.Layout.HBox` instead of `HBox`, and nothing else in the form changes. That is the test for whether modules are ordinary language work: if the typed-AST text form had to gain a construct, the deferral would be hiding a semantic question. It does not.

The thing deliberately left out: no module-level state and no module initialisation. A module is a set of definitions, a program is `@main`, and the first thing that happens is occurrence zero building it. A module that wanted to "run something at load" would be a widget being built by nothing, which the language does not have, and this is the same rule as "nothing happens because an element appears" applied to source structure.

### Trigger

A program that does not fit in one file, or a library (the GUI library is the candidate) that a second program wants to import. Both will arrive during the conformance work, because the GUI library in Prism is a module the twelve programs share, and the moment it is shared it needs a name. Traits come when the second user-defined numeric type appears.

## Incremental compilation

You change a label in one widget and wait for the whole program to be re-elaborated. For the suite's programs that wait is unmeasurable. For a program the size of a real application with a GUI library under it, it is the edit loop, and the edit loop is the thing a language is judged on by the people who use it every day. Incremental compilation is making the frontend redo only what the edit touched. It is deferred because the typed AST was designed so that it needs nothing new, and the argument that this is true is the item.

### The unit

The typed AST's text form is one binding per line: name, sigil, type, mark, group, expression with every lift explicit, span. A top-level definition elaborates to a set of such lines and depends on the types of the top-level names it uses and nothing else about them. That makes the top-level definition the unit of incremental work, and its interface the types of its free top-level names.

The pipeline per definition is lex, parse, resolve, elaborate, convert, lint, compile. The first three are per file and cheap. Elaboration is per definition given the types of its dependencies, because Hindley–Milner on the value layer needs the type of each free name and the sigil of each is in the name. Conversion and the lint are per definition. The closure compiler produces one `CompiledBlock` per widget block and one `CompiledFunction` per leaf in the `Program`'s tables, indexed by position, which is per definition too once the tables are appended per definition and the indices are stable across an edit (a recompiled definition reuses its old slots and a new one appends). So the dependency graph of the incremental build is the dependency graph of top-level names, and it is already computed by the resolver for the group analysis.

### The sketch

A query-based build (the `salsa` crate is the implementation choice; that it and `rustc`'s own build work this way is from memory): every stage is a function from inputs to output, memoised by a fingerprint of its inputs, and an edit invalidates exactly the queries whose inputs changed.

- `parse(file)` depends on the file's text.
- `resolve(file)` depends on `parse(file)` and the export lists of imported modules.
- `signature(name)` depends on the definition's annotation, which is why top-level annotations are required: the signature of a definition is readable without elaborating its body, so an edit to a body never invalidates its dependents.
- `elaborate(name)` depends on `parse`, `resolve`, and `signature` of every free name.
- `convert(name)`, `lint(name)`, `compile(name)` depend on the stage before.

An edit to a widget's body re-elaborates that definition and recompiles its blocks and leaves in place, and nothing else, because its signature did not change. An edit to a signature re-elaborates its dependents, which the resolver's graph names. The typed-AST text form is the cache: each definition's elaborated lines are written beside the source with their fingerprint, so a cold start with an unchanged file reads the cache and skips to compile.

The cross-cutting checks are the part that needs care. Exhaustiveness and the cycle check are per definition, since both work on one block's graph. Trait coherence and tag uniqueness are per program, but they are checks over tables that are themselves incremental (each module contributes its instances and tags; the check is over the union), and a violation is reported at the import that caused it. Nothing in the frontend needs whole-program information that cannot be expressed as a union of per-module tables, which is the property that makes the module design above and the incremental design here the same design.

### What it must not change

The output: the typed AST of an incrementally built program is byte-identical to a clean build's, and the goldens check it, since the golden test is exactly "elaborate this program and diff the dump." A test that builds every suite program clean and then incrementally after a no-op edit, a body edit and a signature edit, and diffs the dumps, is the whole correctness test. Transcripts cannot change because the typed AST did not.

### Trigger

An edit loop someone notices. Not before, because the memoisation adds a fingerprint to every stage and a cache to every build, and a frontend that is fast enough clean is simpler without them.

## Parallel runtimes

The runtime is single-threaded: `Rc`, `RefCell`, one entry point, no locks. That is a property of the first implementation and not a claim about the model, and the model has three kinds of parallelism in it, each visible in the structure of the runtime as it stands. None of them is needed to make the language exist. Each is a second runtime variant or a World arrangement, and none touches a program. Each is taken here far enough to say what it costs and what has to be true for it to pay.

### Fork-join per height

Within an occurrence, every node at one height depends only on nodes at lower heights, so every node at one height is independent of every other at that height. Propagation is "pop the lowest height, run every action in it," and the actions in one height can run on as many threads as there are, with a join before the next height. That is the whole idea, and the runtime's action map, a `BTreeMap<usize, Vec<Action>>`, is already the work list it needs.

What has to change for it to be safe:

- Graph types become `Sync`. Subscriber lists are read during propagation and written only at build, so they can be behind a read-write lock that is write-locked only when a `dyn` or `for` builds mid-occurrence. Node state that an action writes, a fired mark and a cached current-occurrence value, is per node and written by exactly one action per occurrence, which is the at-most-once law doing work for the implementation: no two actions in one height write the same node, so the writes need no lock, only the memory ordering a join gives.
- Buffered commits stay buffered. Holds and switch targets commit at `usize::MAX`, which is one height and runs after the join, so every write that could conflict is already serialised by the design that makes occurrences transactions.
- Construction mid-occurrence is the hard case. A `dyn` rebuilding at height h builds nodes that schedule at heights above h, and building writes subscriber lists. Two `dyn`s at the same height building at once would both write the action map. The simple rule: an action that builds takes a global build lock for the length of its build, and the catch-up logic that gives the new nodes the occurrence's earlier firings runs under the same lock. Builds are rare compared with propagation, so the lock is cheap; and a World whose occurrences are all rebuilds is not the World that wanted parallel propagation.
- The scheduling check, a node scheduling at or below the current height is a cycle, stays as is: the current height is a shared atomic read, and the comparison is the same.
- Closures are `Send + Sync` in this variant, and values use atomic counts, or the per-object sharing mark described under the representation above, which is the one place the two variants' value modules differ.

The cost is a second runtime crate or a feature flag over the first, with the suite run under both. The payoff exists only when heights are wide: a program with a thousand independent nodes at one height, which is a large `for` over a list whose entries each do a little work. A form with forty fields has heights of width two or three, and a thread pool would make it slower. That is why this is a variant and not the runtime: it has to be measured on the program that wants it.

### Optimistic occurrences

Across occurrences, an occurrence is a transaction with buffered writes. Its read set is every tag read, every hold's value side read, and the element list as it opened; its write set is every hold commit, every switch target and every list change. Two occurrences whose sets do not overlap could run at once and commit in order, and one whose read set overlaps the other's write set has to run after it, which is ordinary optimistic concurrency with the runtime's rollback path as the retry.

The sketch: the World hands the runtime a queue of input sets instead of one. The runtime runs occurrence n on one thread and speculatively starts n+1 on another against the same graph, with n+1's reads recorded. When n commits, its write set is compared with n+1's read set; if disjoint, n+1 commits next; if not, n+1 is rolled back (drop its buffer, clear its fired marks, drop any nodes it built) and rerun against the committed state. Reads during n+1 see the state before n, which is wrong only if n writes something n+1 read, which is exactly the overlap test.

What it needs that the runtime does not have: versioned node state, so that n+1's reads of a hold see the value before n while n's commit is being prepared, which is one extra slot per hold; per-occurrence fired marks instead of one, which is a small array per node; and the rollback path generalised from "the whole occurrence failed" to "this one of two." It also needs the World's queue to be deep, which it is for a server World with many connections and is not for a GUI World that runs one occurrence per input event.

It preserves the transcript. Occurrences commit in the order the World queued them, and a speculative occurrence that conflicted is rerun before it commits, so the sequence of committed states is the sequence a sequential runtime would produce. Determinism is the law that makes optimistic execution invisible, and it is why this variant can exist at all.

### Shards

Across the program, independent subtrees connected only through elements can be separate runtimes, each on its own thread, with a World that routes between them. A server with one runtime per connection, a shell that hosts other programs, a GUI with one runtime per window: in each case the subtrees share nothing but what passes through the element list, and the run call is cheap enough that a World can make many of them.

This one needs no runtime support at all. It needs a World that keeps a runtime per shard, routes each input to the shard whose list has the path, and merges the shards' element lists and request firings for its own components. State elements are the one subtlety: a state element's value is identified by its element value and shared by every slot with that value, and two shards with a `Clock` slot each must see the same clock in the same turn. The World's snapshot rule already says what to do, one snapshot per occurrence answered the same to everyone; a sharded World takes the snapshot once per turn and answers every shard's oracle from it. The equality law then holds across shards because the World made it hold, which is the law's intent: it binds the World, not the runtime.

Communication between shards is the interesting part and it falls out of interception. A program running as a shard that wants to talk to another shard does so through elements: it creates a request element the World routes to the other shard as a response, which is how two programs under a shell talk already. Nothing new. What a shard cannot do is share a dynamic with another shard, because a dynamic is a node and nodes do not cross the run call, and that is the right restriction: the moment two shards share a node they are one runtime.

### What they must not change

The transcript, the laws, the run call. A program cannot tell which variant it runs under, and the suite run under every variant is the test. The language has no threads, no priorities and no phases, and none of these variants adds one.

### Trigger

Fork-join: a profile under a GUI World showing wide heights, which means a large `for`. Optimistic: a server World with a deep input queue and occurrences that rarely conflict, which is a server whose connections are independent, which is most servers, and which is also served by shards with no runtime work at all, so shards first. Shards: the first World that hosts more than one program or serves more than one connection, which is the shell in the examples chapter and a server World after it.

## Hot reload of a widget subtree

You are tuning the padding on a dialog that takes six clicks to reach. Each change is edit, rebuild, restart, six clicks. You want to change the dialog's code and see the running program pick it up with its state intact. Prism has most of this already, because `dyn` replaces a subtree when a dynamic changes and nothing in the language cares what the new subtree's code is. Hot reload is making the code a dynamic.

### The design

Two parts, neither of which touches the language.

A World component. A development World offers a state tag whose value is compiled code:

```prism
tag Dev.Code of Element:
    name: Text
    state: Unavailable | Known Code
```

`Code` is an opaque plain value, a handle to a compiled `IrWidget` constructor, and `Dev.Code(name="Dialog")` is the current compiled form of the top-level widget named `Dialog`. When the file changes the World recompiles the definition (incrementally, if the frontend above exists; from scratch otherwise), and pushes the new `Code` as a state change in the next turn. It is a state element because it is a value that changes and whose current value every slot asking for it must see the same: two `@Reloadable "Dialog"` on screen reload together.

A library widget, in two layers. The inner one is the reload itself:

```prism
@Live: fn Text, @a -> @!a = \|name, @fallback| @widget:
    $code <- @Element Dev.Code(name=name)
    !built <- dyn $code:
        match _:
            Known c: @Build c
            Unavailable: @fallback
    @ !built
```

`@Build: fn Code -> @a` is a primitive that builds the compiled widget the handle names, and is the only new primitive, a native in the closure compiler's table. Under the development World, every save of the file rebuilds the subtree under the `dyn`, in the occurrence of the push, with the new code, and `!built` hands the parent the new subtree's result each time; a parent that wants a continuous event or dynamic across reloads puts a `!Switch` or the hold-and-`$Join` idiom over it, as it would over any `dyn`.

The outer layer is what makes the same program run under a production World, and it cannot be left to tolerance. A World that does not accept a state tag reports an unknown state tag as an error at the slot's creation, which is the obligation that keeps a misspelled tag from being quietly `Unavailable` forever; so a production World handed `Dev.Code` would abort the build, and `@Live` on its own is a development-only widget. The library therefore asks first. `World.Tags` is the core-provided state element listing the tags the World accepts, valued in its first occurrence like any state element, and `@Reloadable` reads it before it creates anything:

```prism
@Reloadable: fn Text, @a -> @!a = \|name, @fallback| @widget:
    $tags <- @Element World.Tags
    !gen <- dyn $tags:
        match Accepts _, "Dev.Code":
            True: @Live name, @fallback
            False: @Once @fallback
    @ !Switch !gen

@Once: fn @a -> @!a = \|@w| @widget:
    !built <- dyn (Const unit): @w       # the construction @started is: a dyn over a constant, built once
    @ !built
```

`Accepts: fn [Text], Text -> Bool` is a plain function over the tag list. Under a production World `$tags` lacks `Dev.Code`, the `dyn` builds the `False` arm, and `Dev.Code` is never created, so no obligation is tested and nothing is tolerated: the production path is the fallback compiled into the program, built once and never rebuilt. Under the development World the `True` arm builds `@Live`, and the `!Switch` over the outer `dyn`'s event flattens the two layers so the parent sees one `!a` either way. The tag list does not change during a run, so the outer `dyn` builds once; it is a `dyn` and not a `match` on a plain value only because `World.Tags` is a state element and its value arrives as a dynamic.

The typing is the part that needs a sentence. `@Build c` returns `@a` for whatever `a` the handle's widget returns, and the handle is opaque, so the check is at runtime: the World knows the compiled widget's result type from the typed AST, the `Reloadable` site knows the `a` it expects, and a mismatch is a build error in the occurrence of the push, reported like any build error, with the program continuing to run the old subtree. That is the construction abort from the errors chapter doing its job for a subtree that was being reborn.

### What survives and what does not

Holds inside the reloaded subtree reset, because a `dyn` rebuild makes new holds with their initial values. That is the semantics, and it is the honest answer: the new code may have changed what the holds mean. State that should survive a reload lives outside the `dyn`, which is where state that should survive any rebuild already lives, and a program written for reload keeps its model above its views for the same reason a program written for `dyn` does. A `$Join` over a hold of the subtree's result dynamic gives the parent a continuous value across reloads.

Suppose instead the runtime tried to migrate holds by name across the rebuild, matching the old node's holds to the new node's by binding name and copying values where the types agree. It would make reload feel better in the common case, and it would make it lie in the uncommon one: a hold whose meaning changed but whose name and type did not would start the new code in a state the new code never produced. Reload is `dyn`, nothing more, and that is a feature: the behaviour is the one the program already has and the test suite already checks.

### What it must not change

Nothing in the language; a program with `@Reloadable` in it is an ordinary program with a `dyn` over `World.Tags` and, under a World that lists `Dev.Code`, a second `dyn` over a state element; under any other World it is the fallback, built through `@Once`, and `Dev.Code` is never created. The production World core need not know `Dev.Code` exists, because the library never asks it to; a World that wanted to accept the tag and always answer `Unavailable` could, and `@Live` would then run under it with the fallback, but nothing requires that and the library does not rely on it. Paths are the other thing to say once: a reload is a `dyn` rebuild, so every slot under it gets a fresh generation segment and a fresh path, and a World holding a native widget keyed by path (a GTK button for a `Ui.Button` it accepted directly) sees a teardown of the old path and a creation at the new one, exactly as it does for any rebuild. The World discards late responses for the old paths under its obligation for gone slots, and nothing about reload asks it to recognise the new slot as the old one.

### Trigger

A GUI World and a program big enough that the restart is noticed. The whole thing is a World component, a prelude widget and one native, so it can be done the week the GUI World works.

## Persistence of holds

Close the program and open it again and everything is gone: the document, the scroll position, the half-filled form. Every application solves this and most solve it twice, once for the real model and once for the dozen pieces of UI state nobody thought of until a user complained. In Prism all state is holds and every hold is owned by a node with a path, so there is a temptation to snapshot the whole tree, write it out, and read it back. The right design is smaller than that and goes through the element list; the whole-tree snapshot is a different thing, a World feature for hosting programs, and is taken up after it.

### Persistence through elements

A hold that should persist is a hold whose initial value comes from outside and whose changes go outside. Both halves are elements, and the worlds chapter declares them: `Store.Value(key)` is a state element whose state is `Stored`, `Absent | Present Bytes`, and `Store.Put(key, requests=!Bytes)` is a request element responding `PutResult`. A `Store.Value` slot has, in the occurrence it is created, the value the store holds for that key, because it is a state element and state is there in its first occurrence; a `Store.Put` firing writes a new value and is answered in a later occurrence. A library widget makes a persistent hold out of the pair:

```prism
@Persisted: fn Text, a, !a -> @$a = \|key, default, !change| @widget:
    $stored <- @Element Store.Value(key=key)
    !written <- @Element Store.Put(key=key, requests=!write)
    let $loaded = match $stored:            # lifts over $; valued in occurrence zero
        Present b: Decode b
        Absent: default
    $$current <- $loaded <= Const !change   # a hold of dynamics
    let !write = Encode !change
    @ $Join $$current
```

The hold is the cache idiom, a hold of dynamics joined with `$Join`, and the reason is occurrence zero. Suppose instead the widget held a plain value, `$value <- default <= !loaded | !change`, with `!loaded` read from `$stored on !first` through a tag at `@started`. That reads the oracle's answer correctly, but the hold commits at the end of occurrence zero, so inside occurrence zero `$value` is `default`: a `dyn` over `$value` in the parent builds under `default`, that slot is in the element list the World reads after occurrence zero, and the `dyn` rebuilds in occurrence one with a fresh path. A one-occurrence hiccup, visible in the transcript as a slot that exists for one turn under the wrong value. With a hold of dynamics the initial value is `$loaded` itself, a derived dynamic that is prompt on `$stored` and so has the decoded stored value from the first instant of occurrence zero; `$Join` follows it, and nothing anywhere sees `default` unless the store was empty. `Const: fn a -> $a` is the prelude's constant dynamic, lifted over `!change`, so each change replaces the inner dynamic with a constant one holding the new value. The widget's contract is "start from what was saved, then follow the program's edits": before the first edit it follows the store, which is right for a document another program may be writing; after the first edit it is pinned to this program's values. A widget that wants to follow the store after edits too is a different widget, a `dyn` over `$stored`, and a program that wants the one-shot stored value as an event still has `$stored on !first`, the tag read at `@started`. A program then writes `$doc <- @Persisted "doc", EmptyDoc, !edit` and the document survives a restart. `!written` is the `PutResult` stream; this widget ignores it, and a stricter one surfaces it. `Encode` and `Decode` are the program's; a `Show`-like trait pair would make them derivable once traits exist.

Why this and not a tree snapshot. Three reasons, each a law.

A hold's identity is its path, and paths change when code changes: insert a bind above the hold and its path moves. A snapshot keyed by path is invalid after any edit, and a program that persists state across versions of itself needs a stable key, which is what `key` is. The program names what persists, which is the goal about one way to change anything written next to the thing: the hold's line says it is persisted, under what key, and nothing else in the program can make a hold persist.

Nothing happens because a widget exists. A tree snapshot taken by the World "when the program exits" is an effect caused by the program's structure, which is the thing the language refuses. A `Store.Put` write is a request fired by an event the program named.

Persistence is a World policy. Whether `Store` is a file, a browser's local storage, a database or a test fixture is the World's business, and a sandbox interceptor can give a subtree a different store or no store by supplying `Store.Value` itself, which is how a test runs a program against a known saved state: the Test World's script sets the oracle's answer for `Store.Value(key="doc")` with a `State` step before the build, the program starts from it, and a `Requested` step checks the `Store.Put` firing after an edit.

### Whole-program snapshots

There is a second thing the word means, and it is a World feature rather than a program one. A World that hosts programs, a shell, a server that migrates sessions between machines, a debugger, wants to freeze a running program and resume it later, code and all. That is a snapshot of the runtime: every node's state, every hold's value, every switch's target, the element list, keyed by path, plus the fingerprint of the typed AST that built it.

It is possible because of what the runtime does not have. No threads, no pending effects, no closures with hidden state: between turns a runtime is a tree of nodes and plain values, and a hold's value is a plain value, an event or a dynamic, each of which is either data or a node reference that the tree also owns. Serialising the tree is a walk. Resuming is building the program's `@main` under a fresh runtime and overwriting every hold's value and every switch's target from the snapshot before the first occurrence, which is legal because occurrence zero has no inputs and the snapshot was taken between turns, when every commit was done.

Its constraints are the ones the path-keyed design has: the typed AST must be the same, since paths are bind positions, and the snapshot is a World's artefact and not a program's, invisible to the program and not part of any transcript. Closures in values are the one representation question: a closure is a code id plus captures, and the code id is stable across runs of the same typed AST, which is one more reason the fingerprint is part of the snapshot.

### What it must not change

The creation rule: a `Store.Put` slot does not write in the occurrence it is created, and a `Store.Value` slot's state is the oracle's answer, never a value the program pushed in that occurrence. No hold persists without a line saying so. A whole-program snapshot is a World feature that produces a runtime indistinguishable, to the program, from one that never stopped.

### Trigger

Element-level persistence: the first program with a document. It needs a `Store` component in the CLI World and the GUI World and a prelude widget, which is an afternoon once the world core exists. Whole-program snapshots: a World that hosts programs and wants to migrate or debug them, which is far off and is listed so that nothing done before it makes it impossible.

## A package system

A second repository wants the GUI library. Someone copies the directory. Three weeks later the two copies disagree on `Ui.Button`'s fields, a program that imports one of them is handed elements by a widget written against the other, and the first sign is a `Build` error in a tag the author never touched. A module system says how one program is split into files. A package system says how code written by someone else gets into a program, with a version, and how a program says which versions it is compatible with, so that the disagreement is a version mismatch the loader reports and not an afternoon. It comes after modules and is the last item here because it is the one with the least to do with the language and the most to do with everything around it.

### What a package is

A named set of modules with a version, a manifest, and its own conformance scripts:

```
name    = "gui"
version = "0.3.0"
prism   = ">=1.0"           # the specification version the package is written against
depends = { "draw" = "^0.2" }
```

Modules in a package live under its namespace, so a package `gui` provides `Gui.Layout`, `Gui.Focus`, and so on, and its tag declarations are in the `Gui` namespace. That makes the tag namespaces, the module namespaces and the package namespaces one thing, and it makes "who owns `Gui.Button`" a question with one answer. A World's components are packages too: the CLI World's tags are the `cli` package's declarations, and a program that imports `Cli` is declaring which World components it needs, which a World can check at load.

Two things a package may not contain, and the loader is what enforces both. No native code: a package is Prism source and the manifest and nothing else, so that the Rust and Haskell implementations read the same package and the differential test covers it. Native code lives in Worlds, which are host-language programs, and a package that needs something native declares a tag and a World provides it. And no build scripts: a package is read, not run, which is the "nothing happens because something exists" rule applied once more. Each implementation's loader rejects a package containing any file that is not Prism source or the manifest, before any of it is parsed, and both loaders run the same check from the same rule, so a package that one implementation accepts and the other refuses is a differential-test failure like any other. The package author is bound by the rule; the loader checks it; nothing downstream of the loader needs to.

### Versions and the conformance suite

A package's version is a claim about its exported signatures and its behaviour, and the second half of that claim is testable. A package ships its own test scripts, written in the conformance suite's script language against the Test World, and a dependent program's build runs them under the implementation it uses. A package that passes its own scripts under both implementations is a package the differential test has checked; that is a stronger guarantee than most ecosystems offer, and it costs nothing beyond what the suite already is.

Resolution is the ordinary thing: a lockfile with exact versions, semantic versioning on the manifest, one version of a package per program (no diamond with two versions, because tag declarations would collide and because the one-impl-per-trait-per-type rule has to hold program-wide). The compatibility question that is Prism's own is the `prism` field: the specification version. The typed-AST text form has a version, and a package compiled against one specification version has a dump that a later frontend can read or reject, which is the same mechanism incremental compilation's cache uses.

### What it must not change

The language, the typed AST beyond qualified names, the laws. A program with packages is a program with more modules.

### Trigger

The second program that wants the GUI library, from a different repository. Before that, a module system and a path on disk are enough.

## What the list has in common

Every item above waits on a fact. None of the facts exists yet, and the specification is complete without any of the items, which is the test each had to pass to be in this chapter rather than in the specification chapters.

What no Trigger subsection says is the order the facts are likely to arrive in and which items block which, and that is worth one paragraph, because an item started out of order is designed against a guess. The world core comes first and is on the plan already; persistence needs it, since `Store.*` is a component the core hosts, and hot reload needs the GUI World built on it. Modules come before packages, because a package is a namespace of modules and has nothing to name without them; traits come beside modules, since coherence is checked at import. The value representation's urgent subset can go in at any time; its full design needs the threading decision made as a feature flag, because the non-atomic count and the per-site caches both assume one thread, and fork-join needs that same flag decided the other way, so the two are one decision with two builds. `update` reuse needs the value representation, since copy-on-write along a path is a property of the heap object's count and layout. Native backends need nothing above them but a consumer, and that consumer is the likeliest to be a World that ships without the frontend, which is a GUI World. Causality in types needs only the GUI library's layout widgets to exist and be used, and is the one item here whose trigger is a bug report. Incremental compilation needs modules, because the unit of invalidation is a definition and a definition's dependencies cross files. Shards need nothing; optimistic occurrences need shards to have been tried first. So the likely sequence is: world core, persistence, modules and traits, the urgent subset, a GUI World, hot reload, and only then anything that needs a profile.
