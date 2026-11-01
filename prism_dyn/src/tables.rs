//! The program's tables, which a dynamic application reads at run time: constructors
//! (for named arguments, `Nat` fields, `^Ord` by declaration order and the type an
//! `impl` is found by), tags (for elements), and `impl`s (trait dispatch and the
//! operators' user meanings).
//!
//! One set per thread. Compiler zero fills them from its loaded program when it starts;
//! a compiled program fills them in its `main`, from tables `prismc` emits.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use prism_rt::{TagDecl, Value};

use crate::func::Layer;

/// A record's or a choice's constructor (Type System ch. 2, Records and Choices).
pub struct CtorMeta {
    pub name: Rc<str>,
    pub type_name: Rc<str>,
    pub record: bool,
    /// Field names; a choice's payload is numbered `0`, `1`, ...
    pub fields: Vec<Rc<str>>,
    /// Which fields are declared `Nat`: an `Int` given there is converted.
    pub nat: Vec<bool>,
    /// What each field takes: a type-parameter field takes its argument whole.
    pub layers: Vec<Layer>,
    /// The alternative's place in its choice, for `^Ord` by declaration order.
    pub index: usize,
}

/// A tag (Semantics ch. 8): its declaration, or a fault kind.
pub struct TagMeta {
    pub decl: Rc<TagDecl>,
    pub fault: bool,
}

#[derive(Default)]
struct Tables {
    ctors: HashMap<Rc<str>, Rc<CtorMeta>>,
    tags: HashMap<Rc<str>, Rc<TagMeta>>,
    impls: HashMap<(Rc<str>, Rc<str>), HashMap<Rc<str>, Value>>,
    traits_with_impls: HashSet<Rc<str>>,
}

thread_local! {
    static TABLES: RefCell<Tables> = RefCell::new(Tables::default());
}

/// Empty every table: a new program on this thread.
pub fn reset() {
    TABLES.with(|t| *t.borrow_mut() = Tables::default());
}

pub fn add_ctor(c: CtorMeta) -> Rc<CtorMeta> {
    let c = Rc::new(c);
    TABLES.with(|t| t.borrow_mut().ctors.insert(c.name.clone(), c.clone()));
    c
}

pub fn ctor(name: &str) -> Option<Rc<CtorMeta>> {
    TABLES.with(|t| t.borrow().ctors.get(name).cloned())
}

pub fn add_tag(decl: Rc<TagDecl>, fault: bool) {
    TABLES.with(|t| t.borrow_mut().tags.insert(decl.name.clone(), Rc::new(TagMeta { decl, fault })));
}

pub fn tag(name: &str) -> Option<Rc<TagMeta>> {
    TABLES.with(|t| t.borrow().tags.get(name).cloned())
}

/// `impl ^Trait Type:` exists, with or without methods of its own.
pub fn add_impl(trait_name: &str, ty: &str) {
    TABLES.with(|t| {
        let mut t = t.borrow_mut();
        t.traits_with_impls.insert(Rc::from(trait_name));
        t.impls.entry((Rc::from(trait_name), Rc::from(ty))).or_default();
    });
}

/// One method of an `impl`, as a function value.
pub fn add_impl_method(trait_name: &str, ty: &str, method: &str, f: Value) {
    TABLES.with(|t| {
        let mut t = t.borrow_mut();
        t.traits_with_impls.insert(Rc::from(trait_name));
        t.impls.entry((Rc::from(trait_name), Rc::from(ty))).or_default().insert(Rc::from(method), f);
    });
}

pub fn has_impl(trait_name: &str, ty: &str) -> bool {
    TABLES.with(|t| {
        let t = t.borrow();
        t.traits_with_impls.contains(trait_name) && t.impls.contains_key(&(Rc::from(trait_name), Rc::from(ty)))
    })
}

/// Whether any type has an `impl` of the trait: the operators skip the lookup otherwise.
pub fn trait_has_impls(trait_name: &str) -> bool {
    TABLES.with(|t| t.borrow().traits_with_impls.contains(trait_name))
}

pub fn impl_method(trait_name: &str, ty: &str, method: &str) -> Option<Value> {
    TABLES.with(|t| {
        let t = t.borrow();
        if !t.traits_with_impls.contains(trait_name) {
            return None;
        }
        t.impls.get(&(Rc::from(trait_name), Rc::from(ty))).and_then(|m| m.get(method).cloned())
    })
}
