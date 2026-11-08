//! `Float`: an IEEE double whose comparisons are its bits (Type System round 6, ruling FB).
//!
//! `==` is bit identity, except that every NaN is one value (ruling NO): two floats are
//! equal when their bits are or both are NaNs, so `0.0` and `-0.0` differ and a NaN equals
//! every NaN, whatever sign or payload the hardware or the compiler's folding gave it. `Ord`
//! is IEEE 754's totalOrder, Rust's `f64::total_cmp`, over the same key: a NaN is read as
//! the positive quiet NaN, so every NaN sorts after +infinity and equal to every other.
//! The type is `Eq`, `Ord` and `Hash` like any other, and a type holding one derives them
//! all. Arithmetic is IEEE's, `f64`'s own operators; only the comparisons read the key.

use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::ops::{Add, Div, Mul, Neg, Sub};

/// The bits a comparison reads: a float's own, or the positive quiet NaN's for every NaN.
#[inline]
fn key(x: f64) -> u64 {
    if x.is_nan() { 0x7ff8_0000_0000_0000 } else { x.to_bits() }
}

/// A Prism `Float`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Float(pub f64);

impl PartialEq for Float {
    #[inline]
    fn eq(&self, other: &Float) -> bool {
        key(self.0) == key(other.0)
    }
}

impl Eq for Float {}

impl PartialOrd for Float {
    #[inline]
    fn partial_cmp(&self, other: &Float) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Float {
    #[inline]
    fn cmp(&self, other: &Float) -> Ordering {
        f64::from_bits(key(self.0)).total_cmp(&f64::from_bits(key(other.0)))
    }
}

impl Hash for Float {
    fn hash<H: Hasher>(&self, h: &mut H) {
        key(self.0).hash(h)
    }
}

impl From<i8> for Float {
    fn from(n: i8) -> Float {
        Float(f64::from(n))
    }
}

macro_rules! arith {
    ($($tr:ident $m:ident $op:tt;)*) => {$(
        impl $tr for Float {
            type Output = Float;
            #[inline]
            fn $m(self, o: Float) -> Float {
                Float(self.0 $op o.0)
            }
        }
        impl $tr for &Float {
            type Output = Float;
            #[inline]
            fn $m(self, o: &Float) -> Float {
                Float(self.0 $op o.0)
            }
        }
    )*};
}

arith! {
    Add add +;
    Sub sub -;
    Mul mul *;
    Div div /;
}

impl Neg for Float {
    type Output = Float;
    #[inline]
    fn neg(self) -> Float {
        Float(-self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Float;

    #[test]
    fn floats_compare_by_their_bits_every_nan_one() {
        let (nan, zero, neg_zero, one) = (Float(f64::NAN), Float(0.0), Float(-0.0), Float(1.0));
        assert!(nan == nan && nan.cmp(&nan).is_eq());
        assert!(zero != neg_zero && neg_zero < zero);
        // Every NaN one value (ruling NO): any sign or payload, after +infinity.
        let other = Float(f64::from_bits(0xfff8_0000_0000_0001));
        assert!(Float(-f64::NAN) == nan && other == nan && other.cmp(&nan).is_eq());
        assert!(Float(f64::NEG_INFINITY) < Float(-f64::NAN) && Float(f64::INFINITY) < other);
        assert!(one < Float(2.0) && Float(-2.0) < neg_zero);
        assert!(zero + one == one && zero - zero == zero && zero * Float(-1.0) == neg_zero);
        assert!(Float(0.1) + Float(0.2) != Float(0.3), "arithmetic stays IEEE");
    }
}
