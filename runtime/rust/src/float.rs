//! `Float`: an IEEE double whose comparisons are its bits (Type System round 6, ruling FB).
//!
//! `==` is bit identity: two floats are equal exactly when their bits are, so a NaN equals
//! itself (the same NaN's bits) and `0.0` and `-0.0` differ. `Ord` is IEEE 754's totalOrder,
//! Rust's `f64::total_cmp`, which is equal exactly when the bits are; so the type is `Eq`,
//! `Ord` and `Hash` like any other, and a type holding one derives them all. Arithmetic is
//! IEEE's, `f64`'s own operators.
//!
//! `==` compares `to_bits` rather than asking `total_cmp` for `Equal`: the two agree, and
//! the bits are one integer comparison where `total_cmp` first flips the magnitude bits of
//! each side.

use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::ops::{Add, Div, Mul, Neg, Sub};

/// A Prism `Float`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Float(pub f64);

impl PartialEq for Float {
    #[inline]
    fn eq(&self, other: &Float) -> bool {
        self.0.to_bits() == other.0.to_bits()
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
        self.0.total_cmp(&other.0)
    }
}

impl Hash for Float {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.0.to_bits().hash(h)
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
    fn floats_compare_by_their_bits() {
        let (nan, zero, neg_zero, one) = (Float(f64::NAN), Float(0.0), Float(-0.0), Float(1.0));
        assert!(nan == nan && nan.cmp(&nan).is_eq());
        assert!(zero != neg_zero && neg_zero < zero);
        assert!(Float(-f64::NAN) < Float(f64::NEG_INFINITY) && Float(f64::INFINITY) < nan);
        assert!(one < Float(2.0) && Float(-2.0) < neg_zero);
        assert!(zero + one == one && zero - zero == zero && zero * Float(-1.0) == neg_zero);
        assert!(Float(0.1) + Float(0.2) != Float(0.3), "arithmetic stays IEEE");
    }
}
