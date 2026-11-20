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
//! An existential over kinded variables (`any (r: Access): ..`, `any (r: Role d): ..`;
//! ruling PK) keeps each hidden variable's variant number beside the value
//! ([`Packing::with_kind`], [`Packed::kind`]): the code that opens it matches on the
//! numbers and reads the value back at the type each variant gives.
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
    kinds: Vec<Rc<dyn Any>>,
}

/// A packing under way: the value's own type is still known, so each bound's impl can be
/// taken (`Packed::of(v).with_eq().with_show().packed()`).
pub struct Packing<T: 'static> {
    value: T,
    show: Option<ShowFn>,
    eq: Option<EqFn>,
    cmp: Option<CmpFn>,
    dictionaries: Vec<Rc<dyn Any>>,
    kinds: Vec<Rc<dyn Any>>,
}

impl Packed {
    pub fn of<T: 'static>(value: T) -> Packing<T> {
        Packing { value, show: None, eq: None, cmp: None, dictionaries: Vec::new(), kinds: Vec::new() }
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
        Packed { value: Rc::new(self.value), show: self.show, eq: self.eq, cmp: self.cmp, dictionaries: self.dictionaries, kinds: self.kinds }
    }
    /// The variant numbers of the existential's kinded variables, in the order it names
    /// them, each the program's own number type.
    pub fn with_kind<N: 'static>(mut self, number: N) -> Self {
        self.kinds.push(Rc::new(number));
        self
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
        Packed { value: Rc::new(value), show: self.show, eq: self.eq, cmp: self.cmp, dictionaries: self.dictionaries.clone(), kinds: self.kinds.clone() }
    }
    /// The variant number of the existential's kinded variable `i`, as it was packed.
    pub fn kind<N: Clone + 'static>(&self, i: usize) -> N {
        let held = self.kinds.get(i).expect("an existential opened by a kinded variable it was not packed with");
        same::<N>(&**held).clone()
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

/// A packed value read back at its own type, owned: a quantified function value's result
/// (Jimmy's ruling FA) at the type its call gives it.
pub fn unpacked<T: Clone + 'static>(p: &Packed) -> T {
    p.held::<T>().clone()
}

/// One part of the value an existential over kinded variables hides (the parts of its
/// tuple, or the one value): the part at the type it was packed at and, when that type is
/// plain data, how to tell it as plain data. The code that packs a value and the code
/// that opens it may hold one part at two types, plain data where an associated type's
/// index is a role only the value knows (a function generic in a document) and the type
/// itself where the role or the document is known: a part read at another type than it
/// was packed at goes through its plain data.
#[derive(Clone)]
pub struct Part {
    value: Rc<dyn Any>,
    pod: Option<Rc<dyn Fn() -> Pod>>,
}

impl Part {
    /// A part whose type is plain data.
    pub fn data<T: ToPod + Clone + 'static>(value: T) -> Part {
        let held = value.clone();
        Part { value: Rc::new(value), pod: Some(Rc::new(move || held.to_pod())) }
    }
    /// A part that is no plain data (a live element, a blueprint) or whose type the
    /// packing code cannot tell as plain data (a type parameter's): read back only at
    /// its own type.
    pub fn held<T: 'static>(value: T) -> Part {
        Part { value: Rc::new(value), pod: None }
    }
}

fn part(p: &Packed, i: usize) -> &Part {
    p.held::<Vec<Part>>().get(i).expect("a prismc bug: an existential opened for a part it was not packed with")
}

/// Part `i` of a packed value at a type that is plain data: the part itself where it was
/// packed at this type, else its plain data read at this type.
pub fn part_data<T: FromPod + Clone + 'static>(p: &Packed, i: usize) -> T {
    let part = part(p, i);
    if let Some(same) = part.value.downcast_ref::<T>() {
        return same.clone();
    }
    let pod = (part.pod.as_ref().expect("a prismc bug: a part of an existential packed at another type and as no plain data"))();
    T::from_pod(&pod).unwrap_or_else(|fault| panic!("a prismc bug: a part of an existential did not read at its type: {fault:?}"))
}

/// Part `i` of a packed value at a type that is no plain data: the part itself.
pub fn part_held<T: Clone + 'static>(p: &Packed, i: usize) -> T {
    same::<T>(&*part(p, i).value).clone()
}

/// The variant number of an existential's kinded variable `i` (the program's number type),
/// which the code that opens it matches on before it reads the value back.
pub fn kind_of<N: Clone + 'static>(p: &Packed, i: usize) -> N {
    p.kind::<N>(i)
}
