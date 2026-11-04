//! `Nat` and `Int`: arbitrary precision with a small-value fast path (round 47 BI;
//! Compiler ch. 5, "Numbers"). The sized types are their Rust twins (`U8` is `u8`).
//!
//! A value that fits in an `i64` is held inline, and only an operation that overflows
//! allocates a [`BigInt`]. The representation is canonical (a value that fits is always
//! small), so equality and hashing are derived. Division follows the prelude: `/` floors
//! and answers `nil` for a zero divisor; `mod` floors, and `x mod 0` is `x`.

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};

/// A Prism `Int`.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Int(Repr);

#[derive(Clone, PartialEq, Eq, Hash)]
enum Repr {
    Small(i64),
    Big(Rc<BigInt>),
}

/// Floored division and remainder, the slow path.
fn floored(a: &BigInt, b: &BigInt) -> (BigInt, BigInt) {
    let (q, r) = (a / b, a % b);
    if !r.is_zero() && r.is_negative() != b.is_negative() { (q - 1, r + b) } else { (q, r) }
}

impl Int {
    pub fn to_big(&self) -> BigInt {
        match &self.0 {
            Repr::Small(n) => BigInt::from(*n),
            Repr::Big(b) => (**b).clone(),
        }
    }
    /// The value as an `i64`, when it fits.
    pub fn to_i64(&self) -> Option<i64> {
        if let Repr::Small(n) = self.0 { Some(n) } else { None }
    }
    pub fn is_negative(&self) -> bool {
        match &self.0 {
            Repr::Small(n) => *n < 0,
            Repr::Big(b) => b.is_negative(),
        }
    }
    /// `/`: floored, `nil` on a zero divisor.
    pub fn div(&self, d: &Int) -> Option<Int> {
        match (&self.0, &d.0) {
            (_, Repr::Small(0)) => None,
            (Repr::Small(a), Repr::Small(b)) if *b != -1 => Some(Int::from(a.div_euclid(*b) - i64::from(*b < 0 && a % b != 0))),
            _ => Some(Int::from(floored(&self.to_big(), &d.to_big()).0)),
        }
    }
    /// Decimal digits with an optional leading `-`, as an emitted literal too big for
    /// `i64` gives them; `None` on anything else.
    pub fn parse(digits: &str) -> Option<Int> {
        digits.parse::<BigInt>().ok().map(Int::from)
    }
    /// `mod`: floored, with `x mod 0 = x`.
    pub fn modulo(&self, d: &Int) -> Int {
        if d.0 == Repr::Small(0) { self.clone() } else { Int::from(floored(&self.to_big(), &d.to_big()).1) }
    }
}

impl From<i64> for Int {
    fn from(n: i64) -> Int {
        Int(Repr::Small(n))
    }
}

impl From<BigInt> for Int {
    fn from(b: BigInt) -> Int {
        Int(b.to_i64().map_or_else(|| Repr::Big(Rc::new(b)), Repr::Small))
    }
}

impl From<Nat> for Int {
    fn from(n: Nat) -> Int {
        n.0
    }
}

impl Ord for Int {
    fn cmp(&self, other: &Int) -> Ordering {
        match (&self.0, &other.0) {
            (Repr::Small(a), Repr::Small(b)) => a.cmp(b),
            _ => self.to_big().cmp(&other.to_big()),
        }
    }
}

impl PartialOrd for Int {
    fn partial_cmp(&self, other: &Int) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// An operator on `&Int`: the checked `i64` fast path, else `BigInt`; and by value.
macro_rules! int_op {
    ($trait:ident, $method:ident, $checked:ident) => {
        impl $trait<&Int> for &Int {
            type Output = Int;
            fn $method(self, rhs: &Int) -> Int {
                match (&self.0, &rhs.0) {
                    (Repr::Small(a), Repr::Small(b)) if a.$checked(*b).is_some() => Int(Repr::Small(a.$checked(*b).unwrap())),
                    _ => Int::from(self.to_big().$method(rhs.to_big())),
                }
            }
        }
        impl $trait for Int {
            type Output = Int;
            fn $method(self, rhs: Int) -> Int {
                (&self).$method(&rhs)
            }
        }
    };
}

int_op!(Add, add, checked_add);
int_op!(Sub, sub, checked_sub);
int_op!(Mul, mul, checked_mul);

impl Neg for &Int {
    type Output = Int;
    fn neg(self) -> Int {
        &Int::from(0) - self
    }
}

impl fmt::Display for Int {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Repr::Small(n) => write!(f, "{n}"),
            Repr::Big(b) => write!(f, "{b}"),
        }
    }
}

/// A Prism `Nat`: an [`Int`] never negative. There is no `Nat - Nat`; `-?` is
/// [`Nat::checked_sub`] (round 47 NS).
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Nat(Int);

impl Nat {
    /// `?Nat`: a non-negative `Int` as a `Nat`.
    pub fn from_int(n: Int) -> Option<Nat> {
        (!n.is_negative()).then_some(Nat(n))
    }
    /// `-?`: the difference, or `nil` below zero.
    pub fn checked_sub(&self, rhs: &Nat) -> Option<Nat> {
        Nat::from_int(&self.0 - &rhs.0)
    }
    pub fn div(&self, d: &Nat) -> Option<Nat> {
        self.0.div(&d.0).map(Nat)
    }
    pub fn modulo(&self, d: &Nat) -> Nat {
        Nat(self.0.modulo(&d.0))
    }
    pub fn as_int(&self) -> &Int {
        &self.0
    }
}

impl From<u32> for Nat {
    fn from(n: u32) -> Nat {
        Nat(Int::from(i64::from(n)))
    }
}

macro_rules! nat_op {
    ($trait:ident, $method:ident) => {
        impl $trait<&Nat> for &Nat {
            type Output = Nat;
            fn $method(self, rhs: &Nat) -> Nat {
                Nat((&self.0).$method(&rhs.0))
            }
        }
        impl $trait for Nat {
            type Output = Nat;
            fn $method(self, rhs: Nat) -> Nat {
                Nat((&self.0).$method(&rhs.0))
            }
        }
    };
}

nat_op!(Add, add);
nat_op!(Mul, mul);

impl fmt::Display for Nat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Numbers debug-print as they print.
macro_rules! debug_as_display {
    ($($t:ty),*) => {$(
        impl fmt::Debug for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(self, f)
            }
        }
    )*};
}

debug_as_display!(Int, Nat);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_values_overflow_into_bignums_and_back() {
        let max = Int::from(i64::MAX);
        let big = &max + &Int::from(1);
        assert_eq!((big.to_i64(), big.to_string()), (None, "9223372036854775808".into()));
        assert_eq!((&big - &Int::from(1)).to_i64(), Some(i64::MAX));
        assert_eq!((-&big).to_i64(), Some(i64::MIN));
        assert!(big > max && -&(&big + &big) < Int::from(i64::MIN));
    }

    #[test]
    fn division_floors_and_zero_is_nil() {
        let q = |a: i64, b: i64| Int::from(a).div(&Int::from(b)).map(|n| n.to_string());
        let all: Vec<_> = [(7, 2), (-7, 2), (7, -2), (-7, -2), (6, -2), (i64::MIN, -1)].iter().map(|(a, b)| q(*a, *b)).collect();
        let want = ["3", "-4", "-4", "3", "-3", "9223372036854775808"];
        assert_eq!(all, want.iter().map(|s| Some(s.to_string())).collect::<Vec<_>>());
        assert_eq!(q(1, 0), None);
        assert_eq!(Int::from(-7).modulo(&Int::from(2)).to_string(), "1");
        assert_eq!(Int::from(5).modulo(&Int::from(0)).to_string(), "5");
    }

    #[test]
    fn nat_subtracts_only_with_a_question() {
        let (two, three) = (Nat::from(2u32), Nat::from(3u32));
        assert_eq!((three.checked_sub(&two), two.checked_sub(&three)), (Some(Nat::from(1u32)), None));
        assert_eq!((&two * &three).to_string(), "6");
    }
}
