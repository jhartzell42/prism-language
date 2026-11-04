// What the emitted C++ uses besides the library's types (prismc/README.md, "The C++
// target"): the spellings of the model's operations that C++ has no syntax for. A Rust
// `match` or labelled block is a GNU statement expression whose value is a `Slot`; a
// pattern tests with `is<C>` and takes a payload with `as<C>`; `?` is `TRY`; a shared
// function value is `Fn`; the model's names for the Rust runtime's constructors and
// methods (`Rc::new`, `Rc::downgrade`, `upgrade`, `Port::new`) are the functions below.
//
// Written by Claude, 3 November 2026, on the claude-self-hosting branch. Subject to change.

#pragma once

#include <functional>
#include <memory>
#include <optional>
#include <string>
#include <type_traits>
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

/// `?`: the value of a `Result`, or the function returns its failure.
#define TRY(...)                                                                         \
    ({                                                                                   \
        auto&& prism_q_ = (__VA_ARGS__);                                                 \
        if (!prism_q_.has_value()) return ::prism_rtl::unexpected(prism_q_.error());    \
        std::move(prism_q_).value();                                                     \
    })

/// `unreachable!()`, as an expression of any type.
[[noreturn]] inline void unreachable(const char* what = "entered unreachable code") { panic(what); }

/// Every type `C` is one of `Cs`: a choice's constructor takes one of its cases.
template <class C, class... Cs>
concept one_of = (std::is_same_v<std::remove_cvref_t<C>, Cs> || ...);

// ---- Patterns ----------------------------------------------------------------------------

template <class T>
bool is_some(const std::optional<T>& o) { return o.has_value(); }
template <class T>
bool is_some(const std::shared_ptr<T>& p) { return p != nullptr; }
template <class T>
bool is_none(const std::optional<T>& o) { return !o.has_value(); }
template <class T>
bool is_none(const std::shared_ptr<T>& p) { return p == nullptr; }
template <class T, class E>
bool is_ok(const Expected<T, E>& e) { return e.has_value(); }
template <class T, class E>
bool is_err(const Expected<T, E>& e) { return !e.has_value(); }

/// A case of a choice (`Shape::Circle`) against a value of the choice: the choice is a
/// handle on its `cases()`, a `std::variant`.
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

/// Whether a type is a Prism value, as opposed to a node or a cell: a value is shared by
/// copying it (a choice is a handle on its cases already), so `Rc<T>` of a value is `T`,
/// and `Rc::new` of one is the value itself. Emitted types say so with `prism_value`.
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

/// `Rc::new(x)`: a value (or a closure, which a `Fn` takes) as it is; a node or a runtime
/// cell on the heap.
template <class T>
auto rc(T x) {
    if constexpr (IsValue<T>::value || requires { &T::operator(); }) return x;
    else return std::make_shared<T>(std::move(x));
}

/// `Rc::new(T::new())` with `T` the type the context asks for.
struct MakeNew {
    template <class T>
    operator std::shared_ptr<T>() const { return std::make_shared<T>(); }
};
inline MakeNew make_new() { return {}; }

/// `self` as a shared pointer, and a weak one.
template <class T>
std::shared_ptr<T> share(T* self) { return std::static_pointer_cast<T>(self->shared_from_this()); }
template <class T>
std::weak_ptr<T> weaken(T* self) { return share(self); }
template <class T>
std::weak_ptr<T> weaken(const std::shared_ptr<T>& p) { return p; }
/// `Weak::upgrade`: the node, if it lives.
template <class T>
std::optional<std::shared_ptr<T>> upgrade(const std::weak_ptr<T>& w) {
    if (auto p = w.lock()) return p;
    return std::nullopt;
}

/// A runtime cell reached as Rust auto-derefs it: through a shared pointer, or as it is.
template <class T>
T& deref(T& x) { return x; }
template <class T>
T& deref(const std::shared_ptr<T>& p) { return *p; }

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
    Map<std::decay_t<std::tuple_element_t<0, R>>, std::decay_t<std::tuple_element_t<1, R>>> out;
    for (const auto& e : m) {
        R r = f(e);
        out.insert(std::get<0>(r), std::get<1>(r));
    }
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

/// `iter_mut().enumerate()`: each element, to update in place, with its index.
template <class T>
std::vector<std::pair<std::size_t, T&>> enumerate(std::vector<T>& xs) {
    std::vector<std::pair<std::size_t, T&>> out;
    std::size_t i = 0;
    for (T& x : xs) out.emplace_back(i++, x);
    return out;
}

/// `Option::unwrap_or_default()`.
template <class T>
T unwrap_or_default(const std::optional<T>& o) { return o ? *o : T(); }

/// `Vector::iter_mut()`: an array's elements to update in place.
template <class T>
std::vector<T>& iter_mut(Array<T>& xs) { return xs.items_mut(); }

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
Event<T> event_of(const std::shared_ptr<Port<T>>& port) { return Event<T>::of(port); }

/// `Dynamic::of(read, updates)`: the type is what `read` gives.
template <class F>
auto dynamic_of(F read, const std::shared_ptr<Port<Unit>>& updates) {
    using T = std::decay_t<std::invoke_result_t<F, const Cx&>>;
    return Dynamic<T>::of(std::move(read), updates);
}

/// `Host::new(at, rows, supplier, enclosing, document, deliver)`: the host's types are its
/// rows' and its owner's, whose methods `document` and `deliver` are.
template <class E, class A, class P, class S, class D1, class D2>
std::shared_ptr<Host<E, A, P>> host_new(Path at, Rows<E, A> rows, S supplier, std::shared_ptr<HostOf<E, A>> enclosing,
                                        D1 P::*document, D2 P::*deliver) {
    using H = Host<E, A, P>;
    return H::make(std::move(at), std::move(rows), Supplier<E, A>(std::move(supplier)), std::move(enclosing),
                   typename H::DocumentFn(document), typename H::DeliverFn(deliver));
}
template <class E, class A, class P, class S, class D1, class D2>
std::shared_ptr<Host<E, A, P>> host_new(Path at, Rows<E, A> rows, S supplier, std::nullopt_t, D1 P::*document,
                                        D2 P::*deliver) {
    return host_new(std::move(at), std::move(rows), std::move(supplier), std::shared_ptr<HostOf<E, A>>(), document, deliver);
}
template <class E, class A, class P, class S, class D1, class D2, class H>
std::shared_ptr<Host<E, A, P>> host_new(Path at, Rows<E, A> rows, S supplier, std::optional<H> enclosing,
                                        D1 P::*document, D2 P::*deliver) {
    return host_new(std::move(at), std::move(rows), std::move(supplier),
                    enclosing ? std::shared_ptr<HostOf<E, A>>(*enclosing) : nullptr, document, deliver);
}

/// `Turn::More(job)`: the next turn is the job's.
template <class J>
auto more(std::shared_ptr<J> job) {
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
    Fn(F f) : f_(std::make_shared<const std::function<R(A...)>>(std::move(f))) {}
    template <class... B>
    R operator()(B&&... b) const { return (*f_)(std::forward<B>(b)...); }

private:
    std::shared_ptr<const std::function<R(A...)>> f_;
};

// ---- Text --------------------------------------------------------------------------------

/// A value as `format!("{}")` spells it into a message or a `Text`.
inline void display(std::string& out, const Text& t) { out += t.to_string(); }
inline void display(std::string& out, const std::string& s) { out += s; }
inline void display(std::string& out, const char* s) { out += s; }
inline void display(std::string& out, const Int& n) { out += n.to_string(); }
inline void display(std::string& out, const Nat& n) { out += n.to_string(); }
inline void display(std::string& out, char32_t c) { utf8_push(out, c); }
inline void display(std::string& out, std::uint8_t b) { out += std::to_string(unsigned(b)); }
inline void display(std::string& out, std::uint32_t n) { out += std::to_string(n); }
inline void display(std::string& out, std::size_t n) { out += std::to_string(n); }
inline void display(std::string& out, double f) { out += float_digits(f); }
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
Text cat(const X&... xs) { return Text(str(xs...)); }

}  // namespace prism_rtl
