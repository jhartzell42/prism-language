//! An existential whose bounds are more than `^Show` (`any ^Eq`, `any ^Ord ^Show`): the
//! value behind `Rc<dyn Any>` and the dictionary its packing found (Type System ch. 5,
//! "Existentials are dictionaries"), one entry per bound. The checker unpacks an
//! existential into a rigid type at every parameter, so two packed values a program
//! compares were packed from one value of one type; a comparison across two types, which
//! the checker refuses, panics here rather than answer.
//!
//! A program's own trait is kept as a dictionary the compiler emits (prismc's `PackedDef`):
//! a struct of functions over packed values, made at the value's type by
//! [`Packing::with_dictionary`] and found again by its type ([`Packed::dictionary`]); the
//! emitted impl of the trait for `Packed` asks it, and each function reads the hidden
//! value at its type ([`Packed::held`]) or packs a result with the same dictionaries
//! ([`Packed::repacked`]).
//!
//! Written by Claude on the claude-bugs branch; subject to change.

use std::any::Any;
use std::cmp::Ordering;
use std::rc::Rc;

use crate::pod::{Fault, FromPod, Pod, ToPod};
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
    dictionaries: Vec<Rc<dyn Any>>,
}

/// A packing under way: the value's own type is still known, so each bound's impl can be
/// taken (`Packed::of(v).with_eq().with_show().packed()`).
pub struct Packing<T: 'static> {
    value: T,
    show: Option<ShowFn>,
    eq: Option<EqFn>,
    cmp: Option<CmpFn>,
    dictionaries: Vec<Rc<dyn Any>>,
}

impl Packed {
    pub fn of<T: 'static>(value: T) -> Packing<T> {
        Packing { value, show: None, eq: None, cmp: None, dictionaries: Vec::new() }
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
    /// `^Pod`: how the hidden value is told as plain data, kept as a dictionary.
    pub fn with_pod_of(self) -> Self
    where
        T: ToPod,
    {
        self.with_dictionary(|_: &T| PodPacked { to_pod: |x| same::<T>(x).to_pod() })
    }
    pub fn packed(self) -> Packed {
        Packed { value: Rc::new(self.value), show: self.show, eq: self.eq, cmp: self.cmp, dictionaries: self.dictionaries }
    }
    /// A program's trait: its dictionary at the value's type, which `make` gives from the
    /// value (the emitted `.._packed_of`).
    pub fn with_dictionary<D: 'static>(mut self, make: fn(&T) -> D) -> Self {
        let dictionary: Rc<dyn Any> = Rc::new(make(&self.value));
        self.dictionaries.push(dictionary);
        self
    }
}

impl Packed {
    /// The hidden value at its own type: a dictionary's function reads it at the type the
    /// dictionary was made at.
    pub fn held<T: 'static>(&self) -> &T {
        same::<T>(&*self.value)
    }
    /// The dictionary of a program's trait, by the dictionary's type.
    pub fn dictionary<D: 'static>(&self) -> &D {
        self.dictionaries.iter().find_map(|d| d.downcast_ref::<D>()).expect("an existential asked for a trait it was not packed with")
    }
    /// A value of the hidden type a method gave back, hidden again with this one's impls.
    pub fn repacked<T: 'static>(&self, value: T) -> Packed {
        Packed { value: Rc::new(value), show: self.show, eq: self.eq, cmp: self.cmp, dictionaries: self.dictionaries.clone() }
    }
}

/// `^Pod`'s dictionary of an existential: the hidden value as plain data.
struct PodPacked {
    to_pod: fn(&dyn Any) -> Pod,
}

/// An existential packed with `^Pod` is told as the plain data of the value it hides.
impl ToPod for Packed {
    fn to_pod(&self) -> Pod {
        (self.dictionary::<PodPacked>().to_pod)(&*self.value)
    }
}

/// Plain data names no type to hide (Type System ch. 5: an existential is made by `any`
/// alone), so reading one as an existential is a fault of what handed it over.
impl FromPod for Packed {
    fn from_pod(p: &Pod) -> Result<Packed, Fault> {
        crate::pod::mismatch("a value of a known type (an existential is made by `any`, never read from plain data)", p)
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

/// A type as a key (a generic function chooses by its type parameters what was made for
/// each set of types the program gives them: a widget's rows read as a value).
pub fn type_key<T: 'static>() -> std::any::TypeId {
    std::any::TypeId::of::<T>()
}

/// A value at the type `To`, which is its own type (a generic function's value where the
/// function's type parameters are known, [`type_key`]): the value itself.
pub fn recast<To: 'static>(value: impl std::any::Any) -> To {
    let mut held = Some(value);
    match (&mut held as &mut dyn std::any::Any).downcast_mut::<Option<To>>() {
        Some(same) => same.take().unwrap_or_else(|| unreachable!()),
        None => panic!("a prismc bug: a value read at a type that is not its own"),
    }
}
