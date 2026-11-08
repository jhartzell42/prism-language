// Plain data at the World's edge: `Pod`, paths, and the `^Pod` traits.
//
// Inside a program every value is typed C++. A World sees none of them: elements,
// answers, pushes and requests cross as plain data (Semantics ch. 9, "External Worlds
// see plain data"), and a `Pod` is that data. A program turns its values into `Pod`s
// through `ToPod` and back through `FromPod`, specialized per type by the templates; the
// library specializes both for its own types. A `Pod` prints in the transcript's spelling
// (print.hpp), reads back (read.hpp), and is totally ordered, so pushes and the ready
// state key by element value.

#pragma once

#include "prism_rtl/float.hpp"
#include "prism_rtl/rc.hpp"
#include <compare>
#include <cstdint>
#include <memory>
#include <optional>
#include <ostream>
#include <span>
#include <string>
#include <string_view>
#include <tuple>
#include <utility>
#include <variant>
#include <vector>

#include "prism_rtl/collections.hpp"
#include "prism_rtl/num.hpp"
#include "prism_rtl/result.hpp"

namespace prism_rtl {

class Pod;

/// Nanoseconds.
using Nanos = __int128;

// ---- Paths -------------------------------------------------------------------------------

/// One step of a path: an element statement's position, a `rebuild`'s generation `#g`,
/// or a `for`'s or host document's key `[k]` (Semantics ch. 8). Ordered Pos, then Gen,
/// then Key, each by its value.
class Segment {
public:
    enum class Kind : std::uint8_t { Pos, Gen, Key };
    static Segment pos(std::uint32_t n) { return Segment(Kind::Pos, n, nullptr); }
    static Segment gen(std::uint32_t g) { return Segment(Kind::Gen, g, nullptr); }
    static Segment key(Pod k);

    Kind kind() const { return kind_; }
    /// A position's or generation's number.
    std::uint32_t number() const { return n_; }
    /// A key's value.
    const Pod& key_value() const { return *key_; }
    bool is_pos(std::uint32_t n) const { return kind_ == Kind::Pos && n_ == n; }

    friend bool operator==(const Segment& a, const Segment& b);
    friend std::strong_ordering operator<=>(const Segment& a, const Segment& b);

private:
    Segment(Kind k, std::uint32_t n, Rc<const Pod> key) : kind_(k), n_(n), key_(std::move(key)) {}
    Kind kind_;
    std::uint32_t n_;
    Rc<const Pod> key_;
};

/// A place's path. The order is document order: a path before every path it prefixes,
/// and the book's order within one depth.
class Path {
public:
    Path() = default;
    explicit Path(std::vector<Segment> segs) : segs_(std::move(segs)) {}
    static Path root() { return Path(); }
    Path child(Segment seg) const;
    /// The child at an element statement's position.
    Path at(std::uint32_t pos) const { return child(Segment::pos(pos)); }
    /// The path `rest` under this one: a hosted child's place, relative to the child,
    /// under the child's root.
    Path join(const Path& rest) const;
    std::span<const Segment> segments() const { return segs_; }
    std::size_t len() const { return segs_.size(); }
    bool is_empty() const { return segs_.empty(); }
    /// The notation of Semantics ch. 8: `1/#0/[k]/0`.
    std::string to_string() const;

    friend bool operator==(const Path& a, const Path& b) = default;
    friend std::strong_ordering operator<=>(const Path& a, const Path& b);

private:
    std::vector<Segment> segs_;
};

/// A path from a slice of segments, as `Node::answer` receives the rest of one.
inline Path path_of(std::span<const Segment> at) { return Path(std::vector<Segment>(at.begin(), at.end())); }

// ---- Pod ---------------------------------------------------------------------------------

/// Named fields in declaration order: a record's, or an element's.
using Fields = std::vector<std::pair<std::string, Pod>>;

/// A plain value as a World sees it. The kinds, in their order:
///
/// - `Unit`, `Bool`, `Int` (`Nat`, `Int` and the sized integers alike: a test's literal
///   `1` matches any), `Float`, `Char`, `Text`, `Duration` (nanoseconds);
/// - `Record`: `Name(f=v, ..)`, fields in declaration order;
/// - `Choice`: `Ctor a, b`; a unit type's value (`click`) and a fault kind
///   (`Fs.NotFound`) are choices with no arguments;
/// - `Element`: `Fam.Tag(f=v, ..)`, an element's `Level`, or an `Edge`'s value;
/// - `Tuple`, `Array`, `Map` (entries in ascending key order);
/// - `Nil`, `Some`, `Ok` (`~ x`), `Failed` (a failed `~T`: its `Error` record);
/// - `Path`;
/// - `Shown`: a value whose type has its own `impl ^Show`: it prints as the impl's text
///   and matches as its structure (transcripts print by `^Show`, Type System ch. 2). The
///   text takes no part in equality or order, which go by the structure (a chess `Spot`
///   orders by its rank and file, not by its name `e2`).
class Pod {
public:
    enum class Kind : std::uint8_t {
        Unit, Bool, Int, Float, Char, Text, Duration, Record, Choice, Element,
        Tuple, Array, Map, Nil, Some, Ok, Failed, Path, Shown,
    };
    struct Named {
        std::string name;
        Fields fields;
    };
    struct Choice {
        std::string ctor;
        std::vector<Pod> args;
    };
    struct Shown {
        std::string text;
        Rc<const Pod> structure;
    };
    using Entries = std::vector<std::pair<Pod, Pod>>;

    Pod() : v_(std::in_place_index<0>) {}

    static Pod unit() { return Pod(); }
    static Pod boolean(bool b) { return Pod(std::in_place_index<1>, b); }
    static Pod integer(Int n) { return Pod(std::in_place_index<2>, std::move(n)); }
    /// Rust's `Pod::int`.
    static Pod int64(std::int64_t n) { return integer(Int(n)); }
    static Pod floating(double f) { return Pod(std::in_place_index<3>, Float{f}); }
    static Pod character(char32_t c) { return Pod(std::in_place_index<4>, c); }
    static Pod text(Text t) { return Pod(std::in_place_index<5>, std::move(t)); }
    static Pod duration(Nanos ns) { return Pod(std::in_place_index<6>, ns); }
    static Pod record(std::string name, Fields fields) {
        return Pod(std::in_place_index<7>, Named{std::move(name), std::move(fields)});
    }
    static Pod choice(std::string ctor, std::vector<Pod> args) {
        return Pod(std::in_place_index<8>, Choice{std::move(ctor), std::move(args)});
    }
    /// A choice with no arguments: `click`, `Read`, `Fs.NotFound`.
    static Pod name(std::string ctor) { return choice(std::move(ctor), {}); }
    static Pod element(std::string tag, Fields fields) {
        return Pod(std::in_place_index<9>, Named{std::move(tag), std::move(fields)});
    }
    static Pod tuple(std::vector<Pod> xs) { return Pod(std::in_place_index<10>, std::move(xs)); }
    static Pod array(std::vector<Pod> xs) { return Pod(std::in_place_index<11>, std::move(xs)); }
    static Pod map(Entries es) { return Pod(std::in_place_index<12>, std::move(es)); }
    static Pod nil() { return Pod(std::in_place_index<13>, Unit{}); }
    static Pod some(Pod x) { return Pod(std::in_place_index<14>, make_rc<const Pod>(std::move(x))); }
    static Pod ok(Pod x) { return Pod(std::in_place_index<15>, make_rc<const Pod>(std::move(x))); }
    static Pod failed(Pod e) { return Pod(std::in_place_index<16>, make_rc<const Pod>(std::move(e))); }
    static Pod path(prism_rtl::Path p) { return Pod(std::in_place_index<17>, std::move(p)); }
    static Pod shown(std::string text, Pod structure) {
        return Pod(std::in_place_index<18>, Shown{std::move(text), make_rc<const Pod>(std::move(structure))});
    }

    Kind kind() const { return Kind(v_.index()); }
    bool is(Kind k) const { return kind() == k; }

    // The payload of each kind, or null. These look at this value, not its structure.
    const bool* if_bool() const { return std::get_if<1>(&v_); }
    const Int* if_int() const { return std::get_if<2>(&v_); }
    const Float* if_float() const { return std::get_if<3>(&v_); }
    const char32_t* if_char() const { return std::get_if<4>(&v_); }
    const Text* if_text() const { return std::get_if<5>(&v_); }
    const Nanos* if_duration() const { return std::get_if<6>(&v_); }
    const Named* if_record() const { return std::get_if<7>(&v_); }
    const Choice* if_choice() const { return std::get_if<8>(&v_); }
    const Named* if_element() const { return std::get_if<9>(&v_); }
    const std::vector<Pod>* if_tuple() const { return std::get_if<10>(&v_); }
    const std::vector<Pod>* if_array() const { return std::get_if<11>(&v_); }
    const Entries* if_map() const { return std::get_if<12>(&v_); }
    /// `Some`'s, `Ok`'s or `Failed`'s inner value, by kind.
    const Pod* if_some() const { return boxed<14>(); }
    const Pod* if_ok() const { return boxed<15>(); }
    const Pod* if_failed() const { return boxed<16>(); }
    const prism_rtl::Path* if_path() const { return std::get_if<17>(&v_); }
    const Shown* if_shown() const { return std::get_if<18>(&v_); }

    /// The value a `Shown` stands for; any other value is itself.
    const Pod& structure() const;
    /// A record's or element's field.
    const Pod* field(std::string_view name) const;
    /// An element's qualified tag.
    const std::string* tag() const;
    const Text* as_text() const { return structure().if_text(); }
    std::optional<double> as_f64() const;
    /// Every element inside this value, itself included: a check matches at any depth, by
    /// the GUI library's convention that a container holds its children in a field.
    void elements(std::vector<const Pod*>& out) const;
    /// The transcript's spelling (print.hpp).
    std::string to_string() const;

    friend bool operator==(const Pod& a, const Pod& b) { return (a <=> b) == 0; }
    friend std::strong_ordering operator<=>(const Pod& a, const Pod& b);

private:
    using V = std::variant<Unit, bool, Int, Float, char32_t, Text, Nanos, Named, Choice, Named, std::vector<Pod>,
                           std::vector<Pod>, Entries, Unit, Rc<const Pod>, Rc<const Pod>,
                           Rc<const Pod>, prism_rtl::Path, Shown>;
    template <std::size_t I, class... A>
    Pod(std::in_place_index_t<I> i, A&&... a) : v_(i, std::forward<A>(a)...) {}
    template <std::size_t I>
    const Pod* boxed() const {
        auto p = std::get_if<I>(&v_);
        return p ? p->get() : nullptr;
    }
    V v_;
};

inline std::ostream& operator<<(std::ostream& o, const Pod& p) { return o << p.to_string(); }
inline std::ostream& operator<<(std::ostream& o, const Path& p) { return o << p.to_string(); }

/// Fields from names and values, as `Pod::record` and `Pod::element` take them.
inline Fields named(std::initializer_list<std::pair<std::string, Pod>> fs) { return Fields(fs); }

// ---- Error ---------------------------------------------------------------------------------

/// Prism's `Error`, a `~T`'s failure: a fault kind (`Fs.NotFound`) and a message.
struct Error {
    std::string kind;
    Text message;
    Error(std::string kind, Text message) : kind(std::move(kind)), message(std::move(message)) {}
    bool operator==(const Error&) const = default;
    auto operator<=>(const Error&) const = default;
};

/// A Prism `~a`: the value, or its `Error`.
template <class T>
using Fallible = Expected<T, Error>;

// ---- ^Pod ----------------------------------------------------------------------------------

/// A value as plain data. The primary template calls a member `Pod to_pod() const`; the
/// templates specialize it or emit the member per type.
template <class T>
struct ToPod {
    static Pod to_pod(const T& x) { return x.to_pod(); }
};

/// A value from plain data: an answer, a push, a first value, a root argument. Only a
/// World can hand a program data of the wrong type, so a mismatch is a `Fault::answer`.
/// The primary template calls a static member `Result<T> from_pod(const Pod&)`.
template <class T>
struct FromPod {
    static Result<T> from_pod(const Pod& p) { return T::from_pod(p); }
};

template <class T>
Pod to_pod(const T& x) {
    return ToPod<T>::to_pod(x);
}

template <class T>
Result<T> from_pod(const Pod& p) {
    return FromPod<T>::from_pod(p);
}

/// What every derived `from_pod` gives on a mismatch: `expected <want>, got <got>`.
Fault mismatch_fault(std::string_view want, const Pod& got);
template <class T>
Result<T> mismatch(std::string_view want, const Pod& got) {
    return unexpected(mismatch_fault(want, got));
}

/// `ToPod` and `FromPod` for a type that is one `Pod` kind.
#define PRISM_POD_LEAF(T, WANT, TO, FROM)                                          \
    template <>                                                                    \
    struct ToPod<T> {                                                              \
        static Pod to_pod(const T& s) { TO }                                       \
    };                                                                             \
    template <>                                                                    \
    struct FromPod<T> {                                                            \
        static Result<T> from_pod(const Pod& whole) {                              \
            const Pod& p = whole.structure();                                      \
            FROM return mismatch<T>(WANT, p);                                      \
        }                                                                          \
    };

PRISM_POD_LEAF(Unit, "unit", (void)s; return Pod::unit();, if (p.is(Pod::Kind::Unit)) return Unit{};)
PRISM_POD_LEAF(bool, "a Bool", return Pod::boolean(s);, if (auto b = p.if_bool()) return *b;)
PRISM_POD_LEAF(Int, "an Int", return Pod::integer(s);, if (auto n = p.if_int()) return *n;)
PRISM_POD_LEAF(Nat, "a Nat", return Pod::integer(s.as_int());,
               if (auto n = p.if_int(); n && !n->is_negative()) return *Nat::from_int(*n);)
PRISM_POD_LEAF(std::uint8_t, "a U8", return Pod::int64(s);,
               if (auto n = p.if_int()) if (auto i = n->to_i64(); i && *i >= 0 && *i < 256) return std::uint8_t(*i);)
PRISM_POD_LEAF(Float, "a Float", return Pod::floating(s.v);, if (auto f = p.if_float()) return *f;)
PRISM_POD_LEAF(char32_t, "a Char", return Pod::character(s);, if (auto c = p.if_char()) return *c;)
PRISM_POD_LEAF(Text, "a Text", return Pod::text(s);, if (auto t = p.if_text()) return *t;)
PRISM_POD_LEAF(Path, "a path", return Pod::path(s);, if (auto q = p.if_path()) return *q;)

#undef PRISM_POD_LEAF

/// A cursor is plain data only on its way out (a `check` line's value, a transcript):
/// `Cursor(at=N)`, as it shows. It is opaque, so nothing reads one back.
template <>
struct ToPod<Cursor> {
    static Pod to_pod(const Cursor& c) { return Pod::record("Cursor", {{"at", Pod::int64(std::int64_t(c.at()))}}); }
};

template <>
struct ToPod<Pod> {
    static Pod to_pod(const Pod& p) { return p; }
};
template <>
struct FromPod<Pod> {
    static Result<Pod> from_pod(const Pod& p) { return p; }
};

template <>
struct ToPod<Error> {
    static Pod to_pod(const Error& e) {
        return Pod::record("Error", {{"kind", Pod::name(e.kind)}, {"message", Pod::text(e.message)}});
    }
};
template <>
struct FromPod<Error> {
    static Result<Error> from_pod(const Pod& p) {
        const Pod* kind = p.field("kind");
        const Pod* message = p.field("message");
        const Pod::Choice* c = kind ? kind->structure().if_choice() : nullptr;
        const Text* m = message ? message->as_text() : nullptr;
        if (c && c->args.empty() && m) return Error(c->ctor, *m);
        return mismatch<Error>("an Error", p);
    }
};

/// `?a`.
template <class T>
struct ToPod<std::optional<T>> {
    static Pod to_pod(const std::optional<T>& x) { return x ? Pod::some(prism_rtl::to_pod(*x)) : Pod::nil(); }
};
template <class T>
struct FromPod<std::optional<T>> {
    static Result<std::optional<T>> from_pod(const Pod& whole) {
        const Pod& p = whole.structure();
        if (p.is(Pod::Kind::Nil)) return std::optional<T>();
        if (auto x = p.if_some()) {
            auto v = prism_rtl::from_pod<T>(*x);
            if (!v) return unexpected(v.error());
            return std::optional<T>(std::move(v).value());
        }
        return mismatch<std::optional<T>>("an option", p);
    }
};

/// `~a`.
template <class T>
struct ToPod<Fallible<T>> {
    static Pod to_pod(const Fallible<T>& x) {
        return x ? Pod::ok(prism_rtl::to_pod(*x)) : Pod::failed(prism_rtl::to_pod(x.error()));
    }
};
template <class T>
struct FromPod<Fallible<T>> {
    static Result<Fallible<T>> from_pod(const Pod& whole) {
        const Pod& p = whole.structure();
        if (auto x = p.if_ok()) {
            auto v = prism_rtl::from_pod<T>(*x);
            if (!v) return unexpected(v.error());
            return Fallible<T>(std::move(v).value());
        }
        if (auto e = p.if_failed()) {
            auto v = prism_rtl::from_pod<Error>(*e);
            if (!v) return unexpected(v.error());
            return Fallible<T>(unexpected(std::move(v).value()));
        }
        return mismatch<Fallible<T>>("a ~ value", p);
    }
};

/// A recursive payload, `Rc<T>` in Rust (`Rc<const T>` here): goes out as its value and
/// comes back as one made from it.
template <class T>
struct ToPod<Rc<const T>> {
    static Pod to_pod(const Rc<const T>& x) { return prism_rtl::to_pod(*x); }
};
template <class T>
struct FromPod<Rc<const T>> {
    static Result<Rc<const T>> from_pod(const Pod& p) {
        auto v = prism_rtl::from_pod<T>(p);
        if (!v) return unexpected(v.error());
        return Rc<const T>(make_rc<const T>(std::move(v).value()));
    }
};

template <class T>
struct ToPod<Array<T>> {
    static Pod to_pod(const Array<T>& xs) {
        std::vector<Pod> out;
        for (const T& x : xs) out.push_back(prism_rtl::to_pod(x));
        return Pod::array(std::move(out));
    }
};
template <class T>
struct FromPod<Array<T>> {
    static Result<Array<T>> from_pod(const Pod& p) {
        auto xs = p.structure().if_array();
        if (!xs) return mismatch<Array<T>>("an array", p);
        Array<T> out;
        for (const Pod& x : *xs) {
            auto v = prism_rtl::from_pod<T>(x);
            if (!v) return unexpected(v.error());
            out.push_back(std::move(v).value());
        }
        return out;
    }
};

template <class K, class V>
struct ToPod<Map<K, V>> {
    static Pod to_pod(const Map<K, V>& m) {
        Pod::Entries out;
        for (const auto& [k, v] : m) out.emplace_back(prism_rtl::to_pod(k), prism_rtl::to_pod(v));
        return Pod::map(std::move(out));
    }
};
template <class K, class V>
struct FromPod<Map<K, V>> {
    static Result<Map<K, V>> from_pod(const Pod& p) {
        auto es = p.structure().if_map();
        if (!es) return mismatch<Map<K, V>>("a map", p);
        Map<K, V> out;
        for (const auto& [k, v] : *es) {
            auto kk = prism_rtl::from_pod<K>(k);
            if (!kk) return unexpected(kk.error());
            auto vv = prism_rtl::from_pod<V>(v);
            if (!vv) return unexpected(vv.error());
            out.insert(std::move(kk).value(), std::move(vv).value());
        }
        return out;
    }
};

/// `((a, b, ..))`, any arity.
template <class... Ts>
struct ToPod<std::tuple<Ts...>> {
    static Pod to_pod(const std::tuple<Ts...>& t) {
        return std::apply([](const Ts&... x) { return Pod::tuple({prism_rtl::to_pod(x)...}); }, t);
    }
};
template <class... Ts>
struct FromPod<std::tuple<Ts...>> {
    static Result<std::tuple<Ts...>> from_pod(const Pod& p) {
        auto xs = p.structure().if_tuple();
        if (!xs || xs->size() != sizeof...(Ts)) return mismatch<std::tuple<Ts...>>("a tuple", p.structure());
        return each(*xs, std::index_sequence_for<Ts...>{});
    }

private:
    template <std::size_t... I>
    static Result<std::tuple<Ts...>> each(const std::vector<Pod>& xs, std::index_sequence<I...>) {
        std::tuple<Result<Ts>...> parts{prism_rtl::from_pod<Ts>(xs[I])...};
        std::optional<Fault> bad;
        ((bad = bad ? bad : (std::get<I>(parts) ? std::nullopt : std::optional<Fault>(std::get<I>(parts).error()))), ...);
        if (bad) return unexpected(*bad);
        return std::tuple<Ts...>(std::move(std::get<I>(parts)).value()...);
    }
};

}  // namespace prism_rtl
