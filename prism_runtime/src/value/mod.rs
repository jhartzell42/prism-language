//! This is the value system for Prism. Prism is a type-safe language,
//! but the type won't always be accessible to Rust. The types
//! and traits in this module encapsulate that abstraction.
//!
//! For now, all downcasts are checked at runtime. In future, we might
//! consider a separate implementation in release where unchecked operations
//! are used instead, trusting the type checking of the programming language.
//! As a result, there are no public faculties for introspection or
//! fallible downcasting---failed downcasts panic and may be optionally
//! unsound in future.

// This implementation is intended to be abstracted so that it can
// change over time in a way that the rest of the runtime will adapt.
// The runtime uses these types and traits, and if we make more
// efficient implementations, or more manually checked implementations,
// the entire runtime will run with those trade-offs.

mod inner_trait;
#[cfg(test)]
mod tests;

use std::{fmt::Debug, marker::PhantomData, ops::Deref, sync::Arc};

use crate::value::inner_trait::{HeapValue, PrismValueInner};

/// Implemented on types that you can use with [`Value`].
pub trait ValueType: PrismValueInner + Clone + Debug + Sync + Send + 'static {}

/// A Prism value that you know the type of in Rust.
#[derive(Clone)]
pub struct Value<T: ValueType> {
    value: AnyValue,
    phantom: PhantomData<T>,
}

impl<T: ValueType> Debug for Value<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

/// A Prism value for a heap, boxed type.
pub type PrismArc<T> = Value<Arc<T>>;

impl<T> PrismArc<T>
where
    Arc<T>: ValueType,
{
    /// Create a new managed heap value.
    pub fn new(value: T) -> Self {
        Arc::new(value).into()
    }
}

impl<T> PrismArc<T>
where
    Arc<T>: ValueType,
    T: Clone,
{
    /// Unwrap or clone heap value.
    pub fn extract(self) -> T {
        Arc::unwrap_or_clone(self.get())
    }
}

impl<T: ValueType> Deref for Value<T> {
    type Target = T::Ref;

    fn deref(&self) -> &Self::Target {
        T::get_any(&self.value)
    }
}

impl<T: ValueType> Value<T> {
    /// Convert to the inner value
    pub fn get(self) -> T {
        T::from_any(self.value)
    }
}

impl<T: ValueType> From<T> for Value<T> {
    fn from(value: T) -> Self {
        Self {
            value: T::to_any(value),
            phantom: PhantomData,
        }
    }
}

/// Represents a prism value of a type not exposed to Rust.
///
/// You must track the type outside of Rust. Introspection is not
/// permitted. In the Prism programming language, you must statically
/// know the type.
#[derive(Clone)]
pub struct AnyValue(AnyValueInner);

impl AnyValue {
    pub(crate) fn try_downcast<T: ValueType>(self) -> Result<Value<T>, TypeMismatch> {
        T::try_get_any(&self)?;
        Ok(self.downcast())
    }

    /// Go from [`AnyValue`] to a specific [`Value`] type.
    /// Currently panics on type mismatch.
    pub fn downcast<T: ValueType>(self) -> Value<T> {
        T::get_any(&self); // Panic on mismatch
        Value {
            value: self,
            phantom: PhantomData,
        }
    }
}

impl<T: ValueType> From<Value<T>> for AnyValue {
    fn from(value: Value<T>) -> Self {
        value.value
    }
}

#[derive(Clone)]
enum AnyValueInner {
    Arc(Arc<dyn HeapValue>),
    U32(u32),
    I32(i32),
    Unit,
    // TODO: Events/Dynamics/Behaviors. Covered under `HeapValue` for now.
}

impl AnyValueInner {
    // Test/debug purposes only!
    fn type_string(&self) -> String {
        match self {
            AnyValueInner::Arc(heap_value) => heap_value.type_string(),
            AnyValueInner::U32(_) => "u32".into(),
            AnyValueInner::I32(_) => "i32".into(),
            AnyValueInner::Unit => "()".into(),
        }
    }
}

impl Debug for AnyValueInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Arc(arg0) => arg0.fmt(f),
            Self::U32(arg0) => arg0.fmt(f),
            Self::I32(arg0) => arg0.fmt(f),
            Self::Unit => ().fmt(f),
        }
    }
}

impl Debug for AnyValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// There's a type mismatch. This should only be exposed via the test backend.
#[derive(Debug, thiserror::Error)]
#[error("type mismatch expected={expected:?} actual={actual:?}")]
pub struct TypeMismatch {
    pub(crate) expected: String,
    pub(crate) actual: String,
}
