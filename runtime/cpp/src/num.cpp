// `BigInt`, `Int` and `Nat` (num.hpp).

#include "prism_rtl/num.hpp"

#include "prism_rtl/result.hpp"

#include <algorithm>
#include <cmath>
#include <limits>

namespace prism_rtl {

using Limb = std::uint32_t;
using Wide = std::uint64_t;
using Mag = std::vector<Limb>;

/// The magnitude arithmetic, on limb vectors with no leading zero limb.
struct BigIntMag {
    static int cmp(const Mag& a, const Mag& b) {
        if (a.size() != b.size()) return a.size() < b.size() ? -1 : 1;
        for (std::size_t i = a.size(); i-- > 0;)
            if (a[i] != b[i]) return a[i] < b[i] ? -1 : 1;
        return 0;
    }
    static void trim(Mag& m) {
        while (!m.empty() && m.back() == 0) m.pop_back();
    }
    static Mag add(const Mag& a, const Mag& b) {
        const Mag& l = a.size() >= b.size() ? a : b;
        const Mag& s = a.size() >= b.size() ? b : a;
        Mag out(l.size() + 1);
        Wide carry = 0;
        for (std::size_t i = 0; i < l.size(); ++i) {
            Wide t = Wide(l[i]) + (i < s.size() ? s[i] : 0) + carry;
            out[i] = Limb(t);
            carry = t >> 32;
        }
        out[l.size()] = Limb(carry);
        trim(out);
        return out;
    }
    /// `a - b` with `a >= b`.
    static Mag sub(const Mag& a, const Mag& b) {
        Mag out(a.size());
        std::int64_t borrow = 0;
        for (std::size_t i = 0; i < a.size(); ++i) {
            std::int64_t t = std::int64_t(a[i]) - (i < b.size() ? std::int64_t(b[i]) : 0) - borrow;
            borrow = t < 0;
            out[i] = Limb(t + (borrow << 32));
        }
        trim(out);
        return out;
    }
    static Mag mul(const Mag& a, const Mag& b) {
        if (a.empty() || b.empty()) return {};
        Mag out(a.size() + b.size());
        for (std::size_t i = 0; i < a.size(); ++i) {
            Wide carry = 0;
            for (std::size_t j = 0; j < b.size(); ++j) {
                Wide t = Wide(a[i]) * b[j] + out[i + j] + carry;
                out[i + j] = Limb(t);
                carry = t >> 32;
            }
            out[i + b.size()] = Limb(carry);
        }
        trim(out);
        return out;
    }
    /// Divide by one limb in place; the remainder.
    static Limb divmod_limb(Mag& a, Limb d) {
        Wide r = 0;
        for (std::size_t i = a.size(); i-- > 0;) {
            Wide t = (r << 32) | a[i];
            a[i] = Limb(t / d);
            r = t % d;
        }
        trim(a);
        return Limb(r);
    }
    /// Knuth's algorithm D (TAOCP vol. 2, 4.3.1): `a = q * b + r`, `0 <= r < b`.
    static void divmod(const Mag& a, const Mag& b, Mag& q, Mag& r) {
        if (cmp(a, b) < 0) {
            q.clear();
            r = a;
            return;
        }
        if (b.size() == 1) {
            q = a;
            Limb rem = divmod_limb(q, b[0]);
            r.clear();
            if (rem) r.push_back(rem);
            return;
        }
        // D1: normalize so the divisor's top limb has its high bit set.
        int s = __builtin_clz(b.back());
        Mag v(b.size()), u(a.size() + 1);
        for (std::size_t i = b.size(); i-- > 0;)
            v[i] = (b[i] << s) | (s && i > 0 ? Limb(Wide(b[i - 1]) >> (32 - s)) : 0);
        u[a.size()] = s ? Limb(Wide(a.back()) >> (32 - s)) : 0;
        for (std::size_t i = a.size(); i-- > 0;)
            u[i] = (a[i] << s) | (s && i > 0 ? Limb(Wide(a[i - 1]) >> (32 - s)) : 0);
        const std::size_t n = v.size(), m = a.size() - n;
        q.assign(m + 1, 0);
        const Wide base = Wide(1) << 32;
        for (std::size_t j = m + 1; j-- > 0;) {
            // D3: estimate the quotient limb from the top two limbs, and correct it.
            Wide num = (Wide(u[j + n]) << 32) | u[j + n - 1];
            Wide qhat = num / v[n - 1], rhat = num % v[n - 1];
            while (qhat >= base || qhat * v[n - 2] > ((rhat << 32) | u[j + n - 2])) {
                --qhat;
                rhat += v[n - 1];
                if (rhat >= base) break;
            }
            // D4: multiply and subtract.
            std::int64_t borrow = 0;
            Wide carry = 0;
            for (std::size_t i = 0; i < n; ++i) {
                Wide p = qhat * v[i] + carry;
                carry = p >> 32;
                std::int64_t t = std::int64_t(u[i + j]) - std::int64_t(Limb(p)) - borrow;
                borrow = t < 0;
                u[i + j] = Limb(t + (borrow << 32));
            }
            std::int64_t t = std::int64_t(u[j + n]) - std::int64_t(carry) - borrow;
            borrow = t < 0;
            u[j + n] = Limb(t + (borrow << 32));
            // D6: the estimate was one too large; add the divisor back.
            if (borrow) {
                --qhat;
                Wide c = 0;
                for (std::size_t i = 0; i < n; ++i) {
                    Wide sum = Wide(u[i + j]) + v[i] + c;
                    u[i + j] = Limb(sum);
                    c = sum >> 32;
                }
                u[j + n] = Limb(Wide(u[j + n]) + c);
            }
            q[j] = Limb(qhat);
        }
        trim(q);
        // D8: unnormalize the remainder.
        r.assign(n, 0);
        for (std::size_t i = 0; i < n; ++i) r[i] = (u[i] >> s) | (s ? Limb(Wide(u[i + 1]) << (32 - s)) : 0);
        trim(r);
    }
};

// ---- BigInt -------------------------------------------------------------------------------

BigInt::BigInt(std::int64_t n) {
    neg_ = n < 0;
    // The magnitude of INT64_MIN does not fit in int64_t; unsigned negation does it.
    std::uint64_t m = neg_ ? ~std::uint64_t(n) + 1 : std::uint64_t(n);
    while (m) {
        mag_.push_back(Limb(m));
        m >>= 32;
    }
}

void BigInt::trim() {
    BigIntMag::trim(mag_);
    if (mag_.empty()) neg_ = false;
}

std::optional<BigInt> BigInt::parse(std::string_view s) {
    bool neg = false;
    if (!s.empty() && (s[0] == '-' || s[0] == '+')) {
        neg = s[0] == '-';
        s.remove_prefix(1);
    }
    if (s.empty()) return std::nullopt;
    BigInt out;
    // Nine digits at a time: multiply by 10^k and add.
    for (std::size_t i = 0; i < s.size();) {
        std::size_t k = std::min<std::size_t>(9, s.size() - i);
        Limb chunk = 0, scale = 1;
        for (std::size_t j = 0; j < k; ++j) {
            char c = s[i + j];
            if (c < '0' || c > '9') return std::nullopt;
            chunk = chunk * 10 + Limb(c - '0');
            scale *= 10;
        }
        Mag scaled = BigIntMag::mul(out.mag_, Mag{scale});
        out.mag_ = BigIntMag::add(scaled, chunk ? Mag{chunk} : Mag{});
        i += k;
    }
    out.neg_ = neg;
    out.trim();
    return out;
}

std::optional<std::int64_t> BigInt::to_i64() const {
    if (mag_.size() > 2) return std::nullopt;
    std::uint64_t m = 0;
    for (std::size_t i = mag_.size(); i-- > 0;) m = (m << 32) | mag_[i];
    if (!neg_) {
        if (m > std::uint64_t(std::numeric_limits<std::int64_t>::max())) return std::nullopt;
        return std::int64_t(m);
    }
    if (m > std::uint64_t(std::numeric_limits<std::int64_t>::max()) + 1) return std::nullopt;
    return std::int64_t(~m + 1);
}

std::string BigInt::to_string() const {
    if (mag_.empty()) return "0";
    Mag m = mag_;
    std::vector<Limb> chunks;  // base 10^9, least significant first
    while (!m.empty()) chunks.push_back(BigIntMag::divmod_limb(m, 1000000000u));
    std::string out = neg_ ? "-" : "";
    out += std::to_string(chunks.back());
    for (std::size_t i = chunks.size() - 1; i-- > 0;) {
        std::string c = std::to_string(chunks[i]);
        out += std::string(9 - c.size(), '0') + c;
    }
    return out;
}

double BigInt::to_double() const {
    double d = 0;
    for (std::size_t i = mag_.size(); i-- > 0;) d = d * 4294967296.0 + mag_[i];
    return neg_ ? -d : d;
}

BigInt operator+(const BigInt& a, const BigInt& b) {
    BigInt out;
    if (a.neg_ == b.neg_) {
        out.mag_ = BigIntMag::add(a.mag_, b.mag_);
        out.neg_ = a.neg_;
    } else if (BigIntMag::cmp(a.mag_, b.mag_) >= 0) {
        out.mag_ = BigIntMag::sub(a.mag_, b.mag_);
        out.neg_ = a.neg_;
    } else {
        out.mag_ = BigIntMag::sub(b.mag_, a.mag_);
        out.neg_ = b.neg_;
    }
    out.trim();
    return out;
}

BigInt BigInt::operator-() const {
    BigInt out = *this;
    if (!out.mag_.empty()) out.neg_ = !out.neg_;
    return out;
}

BigInt operator-(const BigInt& a, const BigInt& b) { return a + (-b); }

BigInt operator*(const BigInt& a, const BigInt& b) {
    BigInt out;
    out.mag_ = BigIntMag::mul(a.mag_, b.mag_);
    out.neg_ = a.neg_ != b.neg_;
    out.trim();
    return out;
}

void BigInt::divmod_trunc(const BigInt& a, const BigInt& b, BigInt& q, BigInt& r) {
    if (b.is_zero()) panic("BigInt division by zero");
    BigInt qq, rr;
    BigIntMag::divmod(a.mag_, b.mag_, qq.mag_, rr.mag_);
    qq.neg_ = a.neg_ != b.neg_;
    rr.neg_ = a.neg_;
    qq.trim();
    rr.trim();
    q = std::move(qq);
    r = std::move(rr);
}

void BigInt::divmod_floor(const BigInt& a, const BigInt& b, BigInt& q, BigInt& r) {
    divmod_trunc(a, b, q, r);
    if (!r.is_zero() && r.is_negative() != b.is_negative()) {
        q = q - BigInt(1);
        r = r + b;
    }
}

std::strong_ordering operator<=>(const BigInt& a, const BigInt& b) {
    if (a.neg_ != b.neg_) return a.neg_ ? std::strong_ordering::less : std::strong_ordering::greater;
    int c = BigIntMag::cmp(a.mag_, b.mag_);
    if (a.neg_) c = -c;
    return c < 0 ? std::strong_ordering::less : c > 0 ? std::strong_ordering::greater : std::strong_ordering::equal;
}

// ---- Int ----------------------------------------------------------------------------------

Int::Int(const BigInt& b) : small_(0) {
    if (auto n = b.to_i64()) small_ = *n;
    else big_ = make_rc<const BigInt>(b);
}

std::optional<Int> Int::parse(std::string_view digits) {
    auto b = BigInt::parse(digits);
    if (!b) return std::nullopt;
    return Int(*b);
}

BigInt Int::to_big() const { return big_ ? *big_ : BigInt(small_); }

std::optional<std::int64_t> Int::to_i64() const {
    if (big_) return std::nullopt;
    return small_;
}

bool Int::is_negative() const { return big_ ? big_->is_negative() : small_ < 0; }

std::optional<Int> Int::div(const Int& d) const {
    if (!d.big_ && d.small_ == 0) return std::nullopt;
    if (!big_ && !d.big_ && d.small_ != -1) {
        std::int64_t q = small_ / d.small_, r = small_ % d.small_;
        if (r != 0 && ((r < 0) != (d.small_ < 0))) --q;
        return Int(q);
    }
    BigInt q, r;
    BigInt::divmod_floor(to_big(), d.to_big(), q, r);
    return Int(q);
}

Int Int::modulo(const Int& d) const {
    if (!d.big_ && d.small_ == 0) return *this;
    if (!big_ && !d.big_ && d.small_ != -1) {
        std::int64_t r = small_ % d.small_;
        if (r != 0 && ((r < 0) != (d.small_ < 0))) r += d.small_;
        return Int(r);
    }
    BigInt q, r;
    BigInt::divmod_floor(to_big(), d.to_big(), q, r);
    return Int(r);
}

std::string Int::to_string() const { return big_ ? big_->to_string() : std::to_string(small_); }

Int operator+(const Int& a, const Int& b) {
    std::int64_t out;
    if (!a.big_ && !b.big_ && !__builtin_add_overflow(a.small_, b.small_, &out)) return Int(out);
    return Int(a.to_big() + b.to_big());
}

Int operator-(const Int& a, const Int& b) {
    std::int64_t out;
    if (!a.big_ && !b.big_ && !__builtin_sub_overflow(a.small_, b.small_, &out)) return Int(out);
    return Int(a.to_big() - b.to_big());
}

Int operator*(const Int& a, const Int& b) {
    std::int64_t out;
    if (!a.big_ && !b.big_ && !__builtin_mul_overflow(a.small_, b.small_, &out)) return Int(out);
    return Int(a.to_big() * b.to_big());
}

Int Int::operator-() const { return Int(0) - *this; }

bool operator==(const Int& a, const Int& b) {
    if (!a.big_ && !b.big_) return a.small_ == b.small_;
    if (a.big_ && b.big_) return *a.big_ == *b.big_;
    return false;  // canonical: a big value never fits
}

std::strong_ordering operator<=>(const Int& a, const Int& b) {
    if (!a.big_ && !b.big_) return a.small_ <=> b.small_;
    return a.to_big() <=> b.to_big();
}

// ---- Nat ----------------------------------------------------------------------------------

std::optional<Nat> Nat::from_int(const Int& n) {
    if (n.is_negative()) return std::nullopt;
    return Nat(n);
}

std::optional<Nat> Nat::checked_sub(const Nat& rhs) const { return from_int(n_ - rhs.n_); }

std::optional<Nat> Nat::div(const Nat& d) const {
    auto q = n_.div(d.n_);
    if (!q) return std::nullopt;
    return Nat(*q);
}

Nat Nat::modulo(const Nat& d) const { return Nat(n_.modulo(d.n_)); }

}  // namespace prism_rtl
