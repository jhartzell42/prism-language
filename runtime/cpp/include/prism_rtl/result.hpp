// The vocabulary every other header uses: the tick number, `unit`, a failed tick
// (`Fault`), a bug of the compiler (`Panic`), and `Expected<T, E>`, the small
// `std::expected` this library writes itself (clang 18 over libstdc++ 13 has none).
//
// A node method returns `Done`, a `Result<void>`: a `Fault` escaping it fails the tick,
// which then never happened (Semantics ch. 9, ch. 10). `TRY` is Rust's `?`.

#pragma once

#include <compare>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <type_traits>
#include <utility>
#include <variant>

namespace prism_rtl {

/// A tick number. Tick 0 is the build.
using Tick = std::uint64_t;

/// The one alternative of a choice with no case (`type Never = choice:`): nothing makes
/// it, so the choice has no value, as Rust's empty enum has none.
struct NoCase {
    NoCase() = delete;
    friend bool operator==(const NoCase&, const NoCase&) { return true; }
    friend auto operator<=>(const NoCase&, const NoCase&) { return std::strong_ordering::equal; }
};

/// Prism's `unit`, Rust's `()`.
struct Unit {
    bool operator==(const Unit&) const = default;
    auto operator<=>(const Unit&) const = default;
};

/// A bug of the compiler, not a value: what Rust's runtime panics with. Thrown, so a
/// test can see it; emitted code never catches it.
struct Panic : std::logic_error {
    using std::logic_error::logic_error;
};

[[noreturn]] inline void panic(const std::string& message) { throw Panic(message); }

/// A failure that escapes a tick, which then never happened (Semantics ch. 9, ch. 10).
/// A Prism `~` value is data instead: an `Expected<T, Error>` (pod.hpp).
struct Fault {
    std::string kind;
    std::string message;

    Fault(std::string kind, std::string message) : kind(std::move(kind)), message(std::move(message)) {}
    /// A World answered no place, or with data of the wrong type.
    static Fault answer(std::string message) { return Fault("Runtime.Answer", std::move(message)); }
    /// A row of a recursive widget deeper than the compiler inlined it was built.
    static Fault too_deep(std::string message) { return Fault("Runtime.TooDeep", std::move(message)); }
    /// `kind: message`, as the transcript's `| ...` line shows it.
    std::string to_string() const { return kind + ": " + message; }
    bool operator==(const Fault&) const = default;
};

/// The failure half of an `Expected`, as `unexpected(e)` makes it.
template <class E>
struct Unexpected {
    E error;
    bool operator==(const Unexpected&) const = default;
};

template <class E>
Unexpected<std::decay_t<E>> unexpected(E&& e) {
    return {std::forward<E>(e)};
}

template <class T>
inline constexpr bool is_unexpected_v = false;
template <class E>
inline constexpr bool is_unexpected_v<Unexpected<E>> = true;

/// A value or a failure: Rust's `Result<T, E>`.
template <class T, class E>
class [[nodiscard]] Expected {
    std::variant<T, Unexpected<E>> v_;

public:
    /// The value's type, as `std::expected` names it.
    using value_type = T;
    template <class U = T>
        requires(std::is_constructible_v<T, U &&> && !std::is_same_v<std::remove_cvref_t<U>, Expected> &&
                 !is_unexpected_v<std::remove_cvref_t<U>>)
    Expected(U&& x) : v_(std::in_place_index<0>, std::forward<U>(x)) {}
    template <class G>
    Expected(Unexpected<G> u) : v_(std::in_place_index<1>, Unexpected<E>{E(std::move(u.error))}) {}

    bool has_value() const { return v_.index() == 0; }
    explicit operator bool() const { return has_value(); }
    T& value() & {
        if (!has_value()) panic("Expected::value on a failure");
        return std::get<0>(v_);
    }
    const T& value() const& {
        if (!has_value()) panic("Expected::value on a failure");
        return std::get<0>(v_);
    }
    T&& value() && {
        if (!has_value()) panic("Expected::value on a failure");
        return std::get<0>(std::move(v_));
    }
    T& operator*() & { return value(); }
    const T& operator*() const& { return value(); }
    T* operator->() { return &value(); }
    const T* operator->() const { return &value(); }
    const E& error() const {
        if (has_value()) panic("Expected::error on a value");
        return std::get<1>(v_).error;
    }
    bool operator==(const Expected&) const = default;
};

/// `Expected<void, E>`: done, or a failure.
template <class E>
class [[nodiscard]] Expected<void, E> {
    std::optional<E> e_;

public:
    Expected() = default;
    template <class G>
    Expected(Unexpected<G> u) : e_(E(std::move(u.error))) {}
    bool has_value() const { return !e_.has_value(); }
    explicit operator bool() const { return has_value(); }
    void value() const {
        if (e_) panic("Expected::value on a failure");
    }
    const E& error() const {
        if (!e_) panic("Expected::error on a value");
        return *e_;
    }
    bool operator==(const Expected&) const = default;
};

/// What can fail a tick.
template <class T>
using Result = Expected<T, Fault>;

/// What a node method returns: a fault escaping it fails the tick.
using Done = Result<void>;

/// Rust's `?` on a `Done` or any `Expected`, as an expression: the function returns the
/// failure, or the expression is the value. The operand is moved from, so it must be a
/// temporary (lowering refuses `?` of a place). A GNU statement expression, which g++ and
/// clang++ both take.
#define TRY(...)                                                                         \
    ({                                                                                   \
        auto&& prism_try_ = (__VA_ARGS__);                                               \
        if (!prism_try_.has_value()) return ::prism_rtl::unexpected(prism_try_.error()); \
        ::std::move(prism_try_).value();                                                 \
    })

}  // namespace prism_rtl
