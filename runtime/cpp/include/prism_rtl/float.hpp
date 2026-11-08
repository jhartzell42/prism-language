// `Float`: an IEEE double whose comparisons are its bits (Type System round 6, ruling FB),
// as the Rust runtime's `prism_rtl::Float` is (runtime/rust/src/float.rs).
//
// `==` is bit identity: two floats are equal exactly when their bits are, so a NaN equals
// itself (the same NaN's bits) and `0.0` and `-0.0` differ. `<=>` is IEEE 754's totalOrder,
// Rust's `f64::total_cmp`, a `std::strong_ordering` that is equal exactly when the bits
// are; so a struct, a variant, a tuple or a container holding one compares by the defaulted
// operators like any other. Arithmetic is IEEE's, `double`'s own operators.
//
// A type of its own rather than `double` with comparison functions beside it: a defaulted
// `operator==` or `<=>`, `std::variant`'s and `std::tuple`'s, and the containers' call the
// element's operators, and a `double`'s built-in ones cannot be replaced, so only a
// distinct type makes every derived comparison bit identity without the compiler knowing
// which types hold a `Float`.
//
// Written by Claude on the claude-float-bits branch; subject to change.

#pragma once

#include <bit>
#include <compare>
#include <cstdint>

namespace prism_rtl {

/// A Prism `Float`.
struct Float {
    double v;

    /// The bits, compared as one integer.
    friend constexpr bool operator==(Float a, Float b) noexcept {
        return std::bit_cast<std::uint64_t>(a.v) == std::bit_cast<std::uint64_t>(b.v);
    }
    /// IEEE 754's totalOrder, as Rust's `f64::total_cmp`: flip the magnitude bits of a
    /// negative number, then compare as signed integers.
    friend constexpr std::strong_ordering operator<=>(Float a, Float b) noexcept {
        return key(a.v) <=> key(b.v);
    }

    friend constexpr Float operator+(Float a, Float b) noexcept { return Float{a.v + b.v}; }
    friend constexpr Float operator-(Float a, Float b) noexcept { return Float{a.v - b.v}; }
    friend constexpr Float operator*(Float a, Float b) noexcept { return Float{a.v * b.v}; }
    friend constexpr Float operator/(Float a, Float b) noexcept { return Float{a.v / b.v}; }
    constexpr Float operator-() const noexcept { return Float{-v}; }

private:
    static constexpr std::int64_t key(double d) noexcept {
        std::int64_t x = std::bit_cast<std::int64_t>(d);
        return x ^ std::int64_t(std::uint64_t(x >> 63) >> 1);
    }
};

}  // namespace prism_rtl
