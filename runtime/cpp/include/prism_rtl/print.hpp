// The transcript's spelling of plain values, as the printer API derived `^Show` calls
// (Type System ch. 2, "Plain values, and how they print"; Conformance ch. 1).
//
// ```
// struct Mark { ... };                        // choice: Dot | Line Int, Int
// template <> struct Show<Mark> {             // as `derive ^Show` emits it
//     static void show(const Mark& m, Printer& p) {
//         if (m.dot()) p.choice("Dot", {});
//         else p.choice("Line", {m.a, m.b});
//     }
// };
// show(std::optional<Mark>(line(1, 2)))       // "? (Line 1, 2)"
// ```
//
// The printer, not the emitted code, knows where a choice needs parentheses: bare as the
// whole value and as a field's value with one argument (`window=WindowId "Chess"`),
// parenthesized everywhere else, so every spelling reads back. A type with its own
// `impl ^Show` calls `Printer::raw` with the impl's text. The rest: text quoted with
// `\"`, `\\`, `\n`, `\t` escaped and other control characters as `\u{hex}`; a float as
// the shortest decimal that reads back, never with an exponent, always with a point, and
// `nan`, `inf`, `-inf`; a duration in its largest whole unit; `((a, b))`, `[[a, b]]`,
// `{k: v}` in key order; `unit`, `True`, `nil`, `? x`, `~ x`; a path as `1/#0/[k]/0`.

#pragma once

#include "prism_rtl/float.hpp"
#include "prism_rtl/rc.hpp"
#include <initializer_list>
#include <memory>
#include <optional>
#include <span>
#include <string>
#include <string_view>
#include <compare>
#include <tuple>
#include <typeinfo>
#include <type_traits>
#include <utility>
#include <vector>

#include "prism_rtl/collections.hpp"
#include "prism_rtl/num.hpp"
#include "prism_rtl/pod.hpp"

namespace prism_rtl {

class Printer;

/// A value that prints in the transcript's spelling: `^Show`. The primary template calls a
/// member `void show(Printer&) const`; the templates specialize it or emit the member.
template <class T>
struct Show {
    static void show(const T& x, Printer& p) { x.show(p); }
};

/// A borrowed value of any type with a `Show`: what `&dyn Show` is in Rust. It lives no
/// longer than the value it refers to (a call's arguments).
class ShowRef {
public:
    template <class T>
        requires(!std::is_same_v<std::remove_cvref_t<T>, ShowRef>)
    ShowRef(const T& x)
        : x_(&x), f_([](const void* v, Printer& p) { Show<T>::show(*static_cast<const T*>(v), p); }) {}
    void show(Printer& p) const { f_(x_, p); }

private:
    const void* x_;
    void (*f_)(const void*, Printer&);
};

/// An existential `any ^Show`, held as `Rc<const DynShow>`.
class DynShow {
public:
    virtual ~DynShow() = default;
    virtual void show(Printer& p) const = 0;
};

/// What emitted `show` functions write to.
class Printer {
public:
    /// Where a value stands, which decides a choice's parentheses.
    enum class At { Whole, Field, Inner };

    /// Text as it is: a number, a name, an `impl ^Show`'s result.
    void raw(std::string_view s) { out_ += s; }
    /// A `Text` (UTF-8), quoted and escaped.
    void text(std::string_view s);
    /// Characters between `quote`s, escaped.
    void quoted(std::u32string_view chars, char32_t quote);
    /// A choice's constructor applied to its arguments: `Lam (Var 0)`, `click`.
    void choice(std::string_view ctor, std::span<const ShowRef> args);
    void choice(std::string_view ctor, std::initializer_list<ShowRef> args) {
        choice(ctor, std::span<const ShowRef>(args.begin(), args.size()));
    }
    /// A record or an element, fields named: `Card(rank=Four, suit=Spades)`. An element
    /// with no fields prints bare (`Test.Log`); a record with none, `Name()`.
    void record(std::string_view name, std::span<const std::pair<std::string_view, ShowRef>> fields);
    void record(std::string_view name, std::initializer_list<std::pair<std::string_view, ShowRef>> fields) {
        record(name, std::span<const std::pair<std::string_view, ShowRef>>(fields.begin(), fields.size()));
    }
    void tuple(std::span<const ShowRef> items) { list("((", items, "))"); }
    void tuple(std::initializer_list<ShowRef> items) { tuple(std::span<const ShowRef>(items.begin(), items.size())); }
    void array(std::span<const ShowRef> items) { list("[[", items, "]]"); }
    /// Any range of showable elements as an array.
    template <class R>
    void array_of(const R& xs) {
        std::vector<ShowRef> items;
        for (const auto& x : xs) items.emplace_back(x);
        array(items);
    }
    void map(std::span<const std::pair<ShowRef, ShowRef>> entries);
    /// Any range of showable key-value pairs as a map, in the range's order.
    template <class M>
    void map_of(const M& m) {
        std::vector<std::pair<ShowRef, ShowRef>> es;
        for (const auto& [k, v] : m) es.emplace_back(ShowRef(k), ShowRef(v));
        map(es);
    }
    /// `nil` or `? x`.
    void option(const ShowRef* x);
    /// `~ x`, and `? x` for `Printer::option`.
    void prefixed(std::string_view prefix, ShowRef x);
    /// A Float: the shortest decimal that reads back.
    void float64(double f);
    /// A duration in its largest whole unit.
    void duration(Nanos ns);
    /// A value at a given standing (a path's key prints as a whole value).
    void inner(ShowRef x, At at);

    std::string take() { return std::move(out_); }

private:
    void list(std::string_view open, std::span<const ShowRef> items, std::string_view close);
    std::string out_;
    At at_ = At::Whole;
};

/// `Show x`: the spelling of a whole value.
std::string show_ref(ShowRef x);
template <class T>
std::string show(const T& x) {
    return show_ref(ShowRef(x));
}
/// `Show x` as a Prism `Text`.
template <class T>
Text show_text(const T& x) {
    return Text(show(x));
}
/// A path in the notation of Semantics ch. 8.
std::string print_path(const Path& p);
/// Rust's `Display` for `f64`: the shortest round trip, never an exponent (`1e21` is
/// `1000000000000000000000`, `-0.0` is `-0`); `nan`, `inf`, `-inf`.
std::string float_digits(double f);

// ---- The library's own types ---------------------------------------------------------------

#define PRISM_SHOW_WITH(T, BODY)                     \
    template <>                                      \
    struct Show<T> {                                 \
        static void show(const T& s, Printer& p) { BODY } \
    };

PRISM_SHOW_WITH(Int, p.raw(s.to_string());)
PRISM_SHOW_WITH(Nat, p.raw(s.to_string());)
PRISM_SHOW_WITH(std::uint8_t, p.raw(std::to_string(unsigned(s)));)
PRISM_SHOW_WITH(std::int64_t, p.raw(std::to_string(s));)
PRISM_SHOW_WITH(Unit, (void)s; p.raw("unit");)
PRISM_SHOW_WITH(bool, p.raw(s ? "True" : "False");)
PRISM_SHOW_WITH(Float, p.float64(s.v);)
PRISM_SHOW_WITH(char32_t, p.quoted(std::u32string_view(&s, 1), U'\'');)
PRISM_SHOW_WITH(Text, p.text(s.view());)
PRISM_SHOW_WITH(Cursor, p.record("Cursor", {{"at", ShowRef(Nat(static_cast<std::uint32_t>(s.at())))}});)
PRISM_SHOW_WITH(std::string, p.text(s);)

#undef PRISM_SHOW_WITH

template <>
struct Show<Error> {
    static void show(const Error& e, Printer& p) {
        Pod kind = Pod::name(e.kind_p);
        p.record("Error", {{"kind", kind}, {"message", e.message_p}});
    }
};

template <>
struct Show<Path> {
    static void show(const Path& s, Printer& p);
};

template <>
struct Show<Pod> {
    static void show(const Pod& s, Printer& p);
};

template <class T>
struct Show<Rc<const T>> {
    static void show(const Rc<const T>& x, Printer& p) { Show<T>::show(*x, p); }
};

template <class T>
struct Show<std::optional<T>> {
    static void show(const std::optional<T>& x, Printer& p) {
        if (!x) return p.option(nullptr);
        ShowRef r(*x);
        p.option(&r);
    }
};

template <class T>
struct Show<Fallible<T>> {
    static void show(const Fallible<T>& x, Printer& p) {
        if (x) p.prefixed("~ ", *x);
        else Show<Error>::show(x.error(), p);
    }
};

template <class T>
struct Show<Array<T>> {
    static void show(const Array<T>& xs, Printer& p) {
        std::vector<ShowRef> items;
        for (const T& x : xs) items.emplace_back(x);
        p.array(items);
    }
};

template <class K, class V>
struct Show<Map<K, V>> {
    static void show(const Map<K, V>& m, Printer& p) { p.map_of(m); }
};

/// A tuple prints as the printer's tuple, any arity.
template <class... Ts>
struct Show<std::tuple<Ts...>> {
    static void show(const std::tuple<Ts...>& t, Printer& p) {
        std::apply([&](const Ts&... x) { p.tuple({ShowRef(x)...}); }, t);
    }
};

/// An existential whose bounds are more than `^Show` (`any ^Eq`): the value and the
/// dictionary its packing found, one entry per bound (Type System ch. 5, "Existentials are
/// dictionaries"), as the Rust runtime's `Packed`. The checker unpacks every existential
/// into a type of its own at a parameter, so a comparison meets two values of one type; one
/// across two types panics rather than answer.
class Packed {
public:
    template <class T>
    class Packing {
    public:
        explicit Packing(T v) : value_(std::make_shared<const T>(std::move(v))) {}
        Packing&& with_show() && {
            show_ = [](const void* x, Printer& p) { Show<T>::show(*static_cast<const T*>(x), p); };
            return std::move(*this);
        }
        Packing&& with_eq() && {
            eq_ = [](const void* x, const void* y) { return *static_cast<const T*>(x) == *static_cast<const T*>(y); };
            return std::move(*this);
        }
        Packing&& with_ord() && {
            eq_ = [](const void* x, const void* y) { return *static_cast<const T*>(x) == *static_cast<const T*>(y); };
            cmp_ = [](const void* x, const void* y) -> std::strong_ordering { return *static_cast<const T*>(x) <=> *static_cast<const T*>(y); };
            return std::move(*this);
        }
        Packed packed() && { return Packed(std::move(value_), &typeid(T), show_, eq_, cmp_); }

    private:
        std::shared_ptr<const void> value_;
        void (*show_)(const void*, Printer&) = nullptr;
        bool (*eq_)(const void*, const void*) = nullptr;
        std::strong_ordering (*cmp_)(const void*, const void*) = nullptr;
    };

    template <class T>
    static Packing<T> of(T v) { return Packing<T>(std::move(v)); }

    void show(Printer& p) const {
        if (!show_) panic("an existential without ^Show shown");
        show_(value_.get(), p);
    }
    bool operator==(const Packed& o) const {
        if (!eq_) panic("an existential without ^Eq compared");
        same(o);
        return eq_(value_.get(), o.value_.get());
    }
    std::strong_ordering operator<=>(const Packed& o) const {
        if (!cmp_) panic("an existential without ^Ord ordered");
        same(o);
        return cmp_(value_.get(), o.value_.get());
    }

private:
    Packed(std::shared_ptr<const void> v, const std::type_info* type, void (*show)(const void*, Printer&),
           bool (*eq)(const void*, const void*), std::strong_ordering (*cmp)(const void*, const void*))
        : value_(std::move(v)), type_(type), show_(show), eq_(eq), cmp_(cmp) {}
    void same(const Packed& o) const {
        if (*type_ != *o.type_) panic("two existentials of different types met (the checker unpacks each into a type of its own)");
    }
    std::shared_ptr<const void> value_;
    const std::type_info* type_;
    void (*show_)(const void*, Printer&);
    bool (*eq_)(const void*, const void*);
    std::strong_ordering (*cmp_)(const void*, const void*);
};

}  // namespace prism_rtl
