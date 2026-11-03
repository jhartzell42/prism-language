// `Nat` and `Int`: arbitrary precision with a small-value fast path (round 47 BI;
// Compiler ch. 5, "Numbers"). The sized types are their C++ twins (`U8` is
// `std::uint8_t`).
//
// A value that fits in an `int64_t` is held inline, and only an operation that overflows
// allocates a `BigInt`. The representation is canonical (a value that fits is always
// small), so equality compares representations. Division follows the prelude: `/` floors
// and answers `nil` for a zero divisor; `mod` floors, and `x mod 0` is `x`.
//
// `BigInt` is this library's own: sign and magnitude, 32-bit limbs, schoolbook
// multiplication and Knuth's algorithm D for division. It is what the slow path needs
// and no more.

#pragma once

#include <compare>
#include <cstdint>
#include <memory>
#include <optional>
#include <ostream>
#include <string>
#include <string_view>
#include <vector>

namespace prism_rtl {

/// An arbitrary-precision integer: a sign and a magnitude of 32-bit limbs, least
/// significant first, with no leading zero limb. Zero has no limbs and is not negative.
class BigInt {
public:
    BigInt() = default;
    BigInt(std::int64_t n);
    /// Decimal digits with an optional leading `-` (or `+`); `nullopt` on anything else.
    static std::optional<BigInt> parse(std::string_view digits);

    bool is_zero() const { return mag_.empty(); }
    bool is_negative() const { return neg_; }
    /// The value as an `int64_t`, when it fits.
    std::optional<std::int64_t> to_i64() const;
    std::string to_string() const;
    double to_double() const;

    friend BigInt operator+(const BigInt& a, const BigInt& b);
    friend BigInt operator-(const BigInt& a, const BigInt& b);
    friend BigInt operator*(const BigInt& a, const BigInt& b);
    BigInt operator-() const;
    /// Truncating division and its remainder (the remainder has the dividend's sign), as
    /// C++'s `/` and `%`. The divisor is not zero.
    static void divmod_trunc(const BigInt& a, const BigInt& b, BigInt& q, BigInt& r);
    /// Floored division and remainder (the remainder has the divisor's sign). The divisor
    /// is not zero.
    static void divmod_floor(const BigInt& a, const BigInt& b, BigInt& q, BigInt& r);

    friend bool operator==(const BigInt& a, const BigInt& b) = default;
    friend std::strong_ordering operator<=>(const BigInt& a, const BigInt& b);

private:
    bool neg_ = false;
    std::vector<std::uint32_t> mag_;
    void trim();
    friend struct BigIntMag;
};

/// A Prism `Int`.
class Int {
public:
    Int() : small_(0) {}
    Int(std::int64_t n) : small_(n) {}
    explicit Int(const BigInt& b);
    /// Decimal digits, as the reader and an emitted literal too big for `int64_t` give them.
    static std::optional<Int> parse(std::string_view digits);

    BigInt to_big() const;
    /// The value as an `int64_t`, when it fits.
    std::optional<std::int64_t> to_i64() const;
    bool is_negative() const;
    /// `/`: floored, `nil` on a zero divisor.
    std::optional<Int> div(const Int& d) const;
    /// `mod`: floored, with `x mod 0 = x`.
    Int modulo(const Int& d) const;
    std::string to_string() const;

    friend Int operator+(const Int& a, const Int& b);
    friend Int operator-(const Int& a, const Int& b);
    friend Int operator*(const Int& a, const Int& b);
    Int operator-() const;
    friend bool operator==(const Int& a, const Int& b);
    friend std::strong_ordering operator<=>(const Int& a, const Int& b);

private:
    std::int64_t small_;
    /// Set exactly when the value does not fit in `int64_t`.
    std::shared_ptr<const BigInt> big_;
};

/// A Prism `Nat`: an `Int` never negative. There is no `Nat - Nat`; `-?` is
/// `Nat::checked_sub` (round 47 NS).
class Nat {
public:
    Nat() = default;
    Nat(std::uint32_t n) : n_(static_cast<std::int64_t>(n)) {}
    /// `?Nat`: a non-negative `Int` as a `Nat`.
    static std::optional<Nat> from_int(const Int& n);
    /// `-?`: the difference, or `nil` below zero.
    std::optional<Nat> checked_sub(const Nat& rhs) const;
    std::optional<Nat> div(const Nat& d) const;
    Nat modulo(const Nat& d) const;
    const Int& as_int() const { return n_; }
    operator const Int&() const { return n_; }
    std::string to_string() const { return n_.to_string(); }

    friend Nat operator+(const Nat& a, const Nat& b) { return Nat(a.n_ + b.n_); }
    friend Nat operator*(const Nat& a, const Nat& b) { return Nat(a.n_ * b.n_); }
    friend bool operator==(const Nat& a, const Nat& b) = default;
    friend std::strong_ordering operator<=>(const Nat& a, const Nat& b) { return a.n_ <=> b.n_; }

private:
    explicit Nat(Int n) : n_(std::move(n)) {}
    Int n_;
};

inline std::ostream& operator<<(std::ostream& o, const BigInt& n) { return o << n.to_string(); }
inline std::ostream& operator<<(std::ostream& o, const Int& n) { return o << n.to_string(); }
inline std::ostream& operator<<(std::ostream& o, const Nat& n) { return o << n.to_string(); }

}  // namespace prism_rtl
