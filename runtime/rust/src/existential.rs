//! An existential whose bounds are more than `^Show` (`any ^Eq`, `any ^Ord ^Show`): the
//! value behind `Rc<dyn Any>` and the dictionary its packing found (Type System ch. 5,
//! "Existentials are dictionaries"), one entry per bound. The checker unpacks an
//! existential into a rigid type at every parameter, so two packed values a program
//! compares were packed from one value of one type; a comparison across two types, which
//! the checker refuses, panics here rather than answer.
//!
//! Written by Claude on the claude-bugs branch; subject to change.

use std::any::Any;
use std::cmp::Ordering;
use std::rc::Rc;

use crate::print::{Printer, Show};

type ShowFn = fn(&dyn Any, &mut Printer);
type EqFn = fn(&dyn Any, &dyn Any) -> bool;
type CmpFn = fn(&dyn Any, &dyn Any) -> Ordering;

/// `any ^b ..`: a value and the impls of its bounds at its type.
#[derive(Clone)]
pub struct Packed {
    value: Rc<dyn Any>,
    show: Option<ShowFn>,
    eq: Option<EqFn>,
    cmp: Option<CmpFn>,
}

/// A packing under way: the value's own type is still known, so each bound's impl can be
/// taken (`Packed::of(v).with_eq().with_show().packed()`).
pub struct Packing<T: 'static> {
    value: T,
    show: Option<ShowFn>,
    eq: Option<EqFn>,
    cmp: Option<CmpFn>,
}

impl Packed {
    pub fn of<T: 'static>(value: T) -> Packing<T> {
        Packing { value, show: None, eq: None, cmp: None }
    }
}

fn same<T: 'static>(x: &dyn Any) -> &T {
    x.downcast_ref::<T>().expect("two existentials of different types met (the checker unpacks each into a type of its own)")
}

impl<T: 'static> Packing<T> {
    pub fn with_show(mut self) -> Self
    where
        T: Show,
    {
        self.show = Some(|x, p| same::<T>(x).show(p));
        self
    }
    pub fn with_eq(mut self) -> Self
    where
        T: PartialEq,
    {
        self.eq = Some(|x, y| same::<T>(x) == same::<T>(y));
        self
    }
    pub fn with_ord(mut self) -> Self
    where
        T: Ord,
    {
        self.eq = Some(|x, y| same::<T>(x) == same::<T>(y));
        self.cmp = Some(|x, y| same::<T>(x).cmp(same::<T>(y)));
        self
    }
    pub fn packed(self) -> Packed {
        Packed { value: Rc::new(self.value), show: self.show, eq: self.eq, cmp: self.cmp }
    }
}

impl Show for Packed {
    fn show(&self, p: &mut Printer) {
        (self.show.expect("an existential without ^Show shown"))(&*self.value, p)
    }
}

impl PartialEq for Packed {
    fn eq(&self, other: &Packed) -> bool {
        (self.eq.expect("an existential without ^Eq compared"))(&*self.value, &*other.value)
    }
}

impl Eq for Packed {}

impl PartialOrd for Packed {
    fn partial_cmp(&self, other: &Packed) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Packed {
    fn cmp(&self, other: &Packed) -> Ordering {
        (self.cmp.expect("an existential without ^Ord ordered"))(&*self.value, &*other.value)
    }
}
