// What the emitted C++ uses besides the library's types:
// the spellings of the model's operations that C++ has no syntax for. A Rust
// `match` or labelled block is a GNU statement expression whose value is a `Slot`; a
// pattern tests with `is<C>` and takes a payload with `as<C>`; `?` is `TRY` (result.hpp); a shared
// function value is `Fn`; the model's names for the Rust runtime's constructors and
// methods (`Rc::new`, `Rc::downgrade`, `upgrade`, `Port::new`) are the functions below.
//
// Written by Claude, 3 November 2026, on the claude-self-hosting branch. Subject to change.

#pragma once

#include "prism_rtl/float.hpp"
#include "prism_rtl/rc.hpp"
#include <charconv>
#include <functional>
#include <memory>
#include <optional>
#include <string>
#include <type_traits>
#include <typeindex>
#include <typeinfo>
#include <utility>
#include <variant>

#include "prism_rtl/prism_rtl.hpp"

namespace prism_rtl {

// ---- Values ------------------------------------------------------------------------------

/// The value of a `match`, an if-let chain or a labelled block, set by the arm that
/// matched (or the `break` that left) and taken once at the end. `std::optional`'s own
/// assignment would read `nullopt` into an `optional<optional<T>>` as "unset".
template <class T>
class Slot {
public:
    template <class U>
    void set(U&& x) {
        v_.emplace(std::forward<U>(x));
    }
    T take() { return std::move(*v_); }

private:
    std::optional<T> v_;
};

/// `unreachable!()`, as an expression of any type.
[[noreturn]] inline void unreachable(const char* what = "entered unreachable code") { panic(what); }

/// `unreachable!()` where a local of type `T` is declared (`let n: T = unreachable!()`): a
/// throw expression is `void`, so the declaration takes a call that has the type.
template <class T>
[[noreturn]] T unreachable_as(const char* what = "entered unreachable code") { panic(what); }

/// A type as a key (a generic function chooses by its type parameters what was made for
/// each set of types the program gives them: a widget's rows read as a value).
template <class T>
std::type_index type_key() { return std::type_index(typeid(T)); }

/// A value at the type `To`, which is its own type (a generic function's value where the
/// function's type parameters are known, `type_key`): the value itself.
template <class To, class From>
To recast(From value) {
    if constexpr (std::is_same_v<To, From>) return value;
    else panic("a prismc bug: a value read at a type that is not its own");
}

/// Every type `C` is one of `Cs`: a choice's constructor takes one of its cases.
template <class C, class... Cs>
concept one_of = (std::is_same_v<std::remove_cvref_t<C>, Cs> || ...);

// ---- Patterns ----------------------------------------------------------------------------

template <class T>
bool is_some(const std::optional<T>& o) { return o.has_value(); }
template <class T>
bool is_some(const Rc<T>& p) { return p != nullptr; }
template <class T>
bool is_none(const std::optional<T>& o) { return !o.has_value(); }
template <class T>
bool is_none(const Rc<T>& p) { return p == nullptr; }
template <class T, class E>
bool is_ok(const Expected<T, E>& e) { return e.has_value(); }
template <class T, class E>
bool is_err(const Expected<T, E>& e) { return !e.has_value(); }

/// A case of a choice (`Shape::Circle`) against a value of the choice: the choice holds its
/// `cases()`, a `std::variant`, by value; `as` is a reference into the value tested.
template <class C, class V>
struct Case {
    static bool is(const V& v) { return std::holds_alternative<C>(v.cases()); }
    static const C& as(const V& v) { return std::get<C>(v.cases()); }
};

template <class C, class V>
bool is(const V& v) { return Case<C, std::remove_cvref_t<V>>::is(v); }
template <class C, class V>
decltype(auto) as(const V& v) { return Case<C, std::remove_cvref_t<V>>::as(v); }

/// The runtime's constructors in patterns: a path segment's kinds.
namespace pat {
struct Pos { std::uint32_t a0; };
struct Gen { std::uint32_t a0; };
struct Key { Pod a0; };
}  // namespace pat

template <>
struct Case<pat::Pos, Segment> {
    static bool is(const Segment& s) { return s.kind() == Segment::Kind::Pos; }
    static pat::Pos as(const Segment& s) { return {s.number()}; }
};
template <>
struct Case<pat::Gen, Segment> {
    static bool is(const Segment& s) { return s.kind() == Segment::Kind::Gen; }
    static pat::Gen as(const Segment& s) { return {s.number()}; }
};
template <>
struct Case<pat::Key, Segment> {
    static bool is(const Segment& s) { return s.kind() == Segment::Kind::Key; }
    static pat::Key as(const Segment& s) { return {s.key_value()}; }
};

// ---- Sharing -----------------------------------------------------------------------------

/// Whether a type is a Prism value, as opposed to a node or a cell: `Rc::new` of a value
/// makes an `Rc<const T>` (rc.hpp), an immutable shared value that compares by its value;
/// of a node or a cell, an `Rc<T>`, which has `T`'s comparisons and so none. Emitted types
/// say so with `prism_value`.
template <class T>
struct IsValue : std::bool_constant<std::is_arithmetic_v<T> || requires { typename T::prism_value; }> {};
template <> struct IsValue<Unit> : std::true_type {};
template <> struct IsValue<Text> : std::true_type {};
template <> struct IsValue<Int> : std::true_type {};
template <> struct IsValue<Nat> : std::true_type {};
template <> struct IsValue<Pod> : std::true_type {};
template <> struct IsValue<Path> : std::true_type {};
template <> struct IsValue<Error> : std::true_type {};
template <class T> struct IsValue<std::optional<T>> : std::true_type {};
template <class T> struct IsValue<Array<T>> : std::true_type {};
template <class K, class V> struct IsValue<Map<K, V>> : std::true_type {};
template <class T, class E> struct IsValue<Expected<T, E>> : std::true_type {};
template <class... T> struct IsValue<std::tuple<T...>> : std::true_type {};
template <class T> struct IsValue<Rc<const T>> : IsValue<T> {};

/// `Rc::new(x)`: a value as an `Rc<const T>` (a recursive payload); a closure (which a `Fn`
/// takes) as it is; a node or a runtime cell on the heap, as an `Rc<T>`.
template <class T>
auto rc(T x) {
    if constexpr (std::is_pointer_v<T> || requires { &T::operator(); }) return x;
    else if constexpr (IsValue<T>::value) return make_rc<const T>(std::move(x));
    else return make_rc<T>(std::move(x));
}

/// `Rc::new(T::new())` with `T` the type the context asks for.
struct MakeNew {
    template <class T>
    operator Rc<T>() const { return make_rc<T>(); }
};
inline MakeNew make_new() { return {}; }

/// `self` as a shared pointer, and a weak one.
template <class T>
Rc<T> share(T* self) { return rc_static_cast<T>(self->rc_from_this()); }
template <class T>
Weak<T> weaken(T* self) { return share(self); }
template <class T>
Weak<T> weaken(const Rc<T>& p) { return p; }
/// `Weak::upgrade`: the node, if it lives.
template <class T>
std::optional<Rc<T>> upgrade(const Weak<T>& w) {
    if (auto p = w.lock()) return p;
    return std::nullopt;
}

/// The model's `*x` where the printer does not know `x`'s type (a pattern's binder, a
/// receiver), and a runtime cell reached as Rust auto-derefs it: through an `Rc`, the value
/// behind it; anything else, the place itself (Rust's `*` of a reference, which C++ has no
/// need of). A temporary `Rc<const T>` of a value comes back as a copy of the value, since
/// it may be the value's last owner; a temporary `Rc<T>` of a node or a cell as the place,
/// which its owner keeps; a temporary of anything else as itself, moved.
template <class T>
struct IsShared : std::false_type {};
template <class T>
struct IsShared<Rc<T>> : std::true_type {};
template <class T>
    requires(!IsShared<std::remove_cv_t<T>>::value)
T& deref(T& x) noexcept { return x; }
template <class T>
    requires(!IsShared<std::remove_cv_t<T>>::value && !std::is_lvalue_reference_v<T>)
T deref(T&& x) { return std::move(x); }
template <class T>
T& deref(const Rc<T>& p) noexcept { return *p; }
template <class T>
T deref(Rc<const T>&& p) { return *p; }

/// `Some(p)`'s payload: an option's value, a live node's pointer (`Weak::upgrade`'s).
template <class T>
const T& some_of(const std::optional<T>& o) { return *o; }
template <class T>
T& some_of(std::optional<T>& o) { return *o; }
template <class T>
const Rc<T>& some_of(const Rc<T>& p) { return p; }

/// `Ok(())`: the success of a `Done` or of a `~unit`.
struct OkUnit {
    operator Done() const { return {}; }
    template <class E>
    operator Expected<Unit, E>() const { return Unit{}; }
};
inline OkUnit ok() { return {}; }

/// `Iterator::for_each`.
template <class R, class F>
void for_each(const R& xs, F f) {
    for (const auto& x : xs) f(x);
}

/// `Option::expect`.
template <class T>
T expect(std::optional<T> o, const char* what) {
    if (!o) panic(what);
    return std::move(*o);
}

/// `Map::get(k).cloned()`, `Option<&T>::cloned()`: the value at a pointer, if any.
template <class T>
std::optional<T> cloned(const T* p) {
    if (p) return *p;
    return std::nullopt;
}
template <class T>
std::optional<T> cloned(const std::optional<T>& o) { return o; }

/// `Result::ok()` whose value nobody reads: the result let go.
template <class T>
void ignore(T&&) {}

/// `Option::ok_or(e)`.
template <class T, class E>
Expected<T, E> ok_or(const std::optional<T>& o, E e) {
    if (o) return *o;
    return unexpected(std::move(e));
}

/// `Option::map`, `Event::map`, `Dynamic::map`.
template <class T, class F>
auto fmap(const std::optional<T>& o, F f) -> std::optional<std::decay_t<std::invoke_result_t<F, const T&>>> {
    if (o) return f(*o);
    return std::nullopt;
}
template <class T, class F>
auto fmap(const Event<T>& e, F f) { return e.map(std::move(f)); }
/// `iter().map(f).collect()` over an array, and over a map's entries (each a pair, `f`
/// giving a pair back).
template <class T, class F>
auto fmap(const Array<T>& xs, F f) {
    using U = std::decay_t<std::invoke_result_t<F, const T&>>;
    std::vector<U> out;
    for (const T& x : xs) out.push_back(f(x));
    return Array<U>(std::move(out));
}
template <class K, class V, class F>
auto fmap(const Map<K, V>& m, F f) {
    using R = std::decay_t<std::invoke_result_t<F, const std::pair<const K, V>&>>;
    using K2 = std::decay_t<std::tuple_element_t<0, R>>;
    using V2 = std::decay_t<std::tuple_element_t<1, R>>;
    // keys that come out in ascending order (a map of a map's entries by the same keys, as
    // a `for`'s gathered results are) build the map at once; any other order inserts
    std::vector<std::pair<K2, V2>> es;
    bool sorted = true;
    for (const auto& e : m) {
        R r = f(e);
        if (!es.empty() && !(es.back().first < std::get<0>(r))) sorted = false;
        es.emplace_back(std::move(std::get<0>(r)), std::move(std::get<1>(r)));
    }
    if (sorted) return Map<K2, V2>::from_sorted(std::move(es));
    Map<K2, V2> out;
    for (auto& [k, v] : es) out.insert(std::move(k), std::move(v));
    return out;
}
template <class T, class F>
auto fmap(const Dynamic<T>& d, F f) { return d.map(std::move(f)); }

/// `iter().enumerate()`: each element with its index.
template <class T>
std::vector<std::pair<std::size_t, T>> enumerate(const Array<T>& xs) {
    std::vector<std::pair<std::size_t, T>> out;
    std::size_t i = 0;
    for (const T& x : xs) out.emplace_back(i++, x);
    return out;
}

/// `Vector::iter_mut()`: an array's elements to update in place, the array's own (its
/// shared nodes copied first), chunk by chunk: linear.
template <class T>
struct MutElements {
    using Elem = typename Array<T>::Elem;
    std::vector<std::pair<Elem*, std::size_t>> chunks;
    struct iterator {
        const std::vector<std::pair<Elem*, std::size_t>>* cs;
        std::size_t c, i;
        T& operator*() const { return (*cs)[c].first[i].v; }
        iterator& operator++() {
            if (++i == (*cs)[c].second) {
                ++c;
                i = 0;
            }
            return *this;
        }
        bool operator!=(const iterator& o) const { return c != o.c || i != o.i; }
    };
    iterator begin() const { return {&chunks, 0, 0}; }
    iterator end() const { return {&chunks, chunks.size(), 0}; }
};

/// `iter_mut().enumerate()`: each element, to update in place, with its index.
template <class T>
std::vector<std::pair<std::size_t, T&>> enumerate(MutElements<T> xs) {
    std::vector<std::pair<std::size_t, T&>> out;
    std::size_t i = 0;
    for (T& x : xs) out.emplace_back(i++, x);
    return out;
}

/// `Option::unwrap_or_default()`.
template <class T>
T unwrap_or_default(const std::optional<T>& o) { return o ? *o : T(); }

template <class T>
MutElements<T> iter_mut(Array<T>& xs) { return MutElements<T>{xs.chunks_mut()}; }

// ---- The runtime's constructors whose type the context gives --------------------------------

/// `Snapshot::new(what)`.
struct NewSnapshot {
    std::string what;
    template <class T>
    operator Snapshot<T>() const { return Snapshot<T>(what); }
};
inline NewSnapshot snapshot(std::string what) { return {std::move(what)}; }

/// `Event::never()`.
struct NeverEvent {
    template <class T>
    operator Event<T>() const { return Event<T>::never(); }
};
inline NeverEvent never_event() { return {}; }

template <class T>
Event<T> event_of(const Rc<Port<T>>& port) { return Event<T>::of(port); }

/// `Dynamic::of(read, updates)`: the type is what `read` gives.
template <class F>
auto dynamic_of(F read, const Rc<Port<Unit>>& updates) {
    using T = std::decay_t<std::invoke_result_t<F, const Cx&>>;
    return Dynamic<T>::of(std::move(read), updates);
}

/// `Host::new(at, rows, supplier, enclosing, deliver)`: the host's types are its rows' and
/// its owner's, whose method `deliver` is.
template <class E, class A, class P, class S, class D>
Rc<Host<E, A, P>> host_new(Path at, Rows<E, A> rows, S supplier, Rc<HostOf<E, A>> enclosing, D P::*deliver) {
    using H = Host<E, A, P>;
    return H::make(std::move(at), std::move(rows), Supplier<E, A>(std::move(supplier)), std::move(enclosing),
                   typename H::DeliverFn(deliver));
}
template <class E, class A, class P, class S, class D>
Rc<Host<E, A, P>> host_new(Path at, Rows<E, A> rows, S supplier, std::nullopt_t, D P::*deliver) {
    return host_new(std::move(at), std::move(rows), std::move(supplier), Rc<HostOf<E, A>>(), deliver);
}
template <class E, class A, class P, class S, class D, class H>
Rc<Host<E, A, P>> host_new(Path at, Rows<E, A> rows, S supplier, std::optional<H> enclosing, D P::*deliver) {
    return host_new(std::move(at), std::move(rows), std::move(supplier),
                    enclosing ? Rc<HostOf<E, A>>(*enclosing) : nullptr, deliver);
}

/// Whether two maps have the same keys, in order: a builder whose rows came or went.
template <class K, class V, class W>
bool same_keys(const Map<K, V>& a, const Map<K, W>& b) {
    if (a.size() != b.size()) return false;
    auto j = b.begin();
    for (const auto& [k, v] : a) {
        if (!(k == (*j).first)) return false;
        ++j;
    }
    return true;
}

/// `Entry::new(what)`: a place's entry in its host's document, of the element type the
/// context gives.
struct NewEntry {
    std::string what;
    template <class E>
    operator Rc<Entry<E>>() const { return Entry<E>::make(what); }
};
inline NewEntry entry_new(std::string what) { return {std::move(what)}; }

/// `Turn::More(job)`: the next turn is the job's.
template <class J>
auto more(Rc<J> job) {
    using B = typename decltype(job->cycle())::Value;
    return Turn<B>::more(std::move(job));
}

/// `Turn::Finished(v)`: a finished turn of the job type the context gives.
template <class V>
struct Finishing {
    using Value = V;
    V value;
    template <class B>
    operator Turn<B>() && { return Turn<B>::finished(B(std::move(value))); }
    template <class B>
    operator Turn<B>() const& { return Turn<B>::finished(B(value)); }
};
template <class V>
Finishing<std::decay_t<V>> finished(V&& v) { return {std::forward<V>(v)}; }

// ---- Function values ---------------------------------------------------------------------

/// `Rc<dyn Fn(&A..) -> R>`: a shared function value.
template <class Sig>
class Fn;

template <class R, class... A>
class Fn<R(A...)> {
public:
    using prism_value = void;
    Fn() = default;
    template <class F>
        requires(!std::is_same_v<std::remove_cvref_t<F>, Fn> && std::is_invocable_v<const F&, A...>)
    Fn(F f) : f_(make_rc<const std::function<R(A...)>>(std::move(f))) {}
    template <class... B>
    R operator()(B&&... b) const { return (*f_)(std::forward<B>(b)...); }

private:
    Rc<const std::function<R(A...)>> f_;
};

// ---- Text --------------------------------------------------------------------------------

/// A value as `format!("{}")` spells it into a message or a `Text`.
inline void display(std::string& out, const Text& t) { out += t.view(); }
inline void display(std::string& out, const std::string& s) { out += s; }
inline void display(std::string& out, const char* s) { out += s; }
inline void display(std::string& out, std::string_view s) { out += s; }
/// A number's digits written straight into `out` (no `std::string` of their own), the
/// arbitrary-precision path aside.
inline void display(std::string& out, const Int& n) {
    if (auto v = n.to_i64()) {
        char digits[24];
        auto end = std::to_chars(digits, digits + sizeof digits, *v).ptr;
        out.append(digits, end);
    } else {
        out += n.to_string();
    }
}
inline void display(std::string& out, const Nat& n) { display(out, n.as_int()); }
inline void display(std::string& out, char32_t c) { utf8_push(out, c); }
inline void display(std::string& out, std::uint8_t b) { out += std::to_string(unsigned(b)); }
inline void display(std::string& out, std::uint32_t n) { out += std::to_string(n); }
inline void display(std::string& out, std::size_t n) { out += std::to_string(n); }
inline void display(std::string& out, Float f) { out += float_digits(f.v); }
inline void display(std::string& out, bool b) { out += b ? "true" : "false"; }
inline void display(std::string& out, const Path& p) { out += p.to_string(); }
inline void display(std::string& out, const Pod& p) { out += p.to_string(); }

/// `format!(..)`: the pieces one after the other, as a message.
template <class... X>
std::string str(const X&... xs) {
    std::string out;
    (display(out, xs), ...);
    return out;
}

/// `Text::from(format!(..))`.
template <class... X>
Text cat(const X&... xs) { return Text::of_utf8(str(xs...)); }

}  // namespace prism_rtl
