//! This is the value system for Prism. Prism is a type-safe language,
//! but the type won't always be accessible to Rust. The types
//! and traits in this module encapsulate that abstraction.

// This implementation is intended to be abstracted so that it can
// change over time in a way that the rest of the runtime will adapt.
// The runtime uses these types and traits, and if we make more
// efficient implementations, or more manually checked implementations,
// the entire runtime will run with those trade-offs.

mod inner_trait;
#[cfg(test)]
mod tests;

use std::{fmt::Debug, marker::PhantomData, sync::Arc};

use crate::value::inner_trait::{HeapValue, PrismValueInner};

/// Implemented on types that you can use with [`Value`].
pub trait PrismValue: PrismValueInner + Clone + Debug {}

/// A Prism value that you know the type of in Rust.
#[derive(Clone)]
pub struct Value<T: PrismValue> {
    value: AnyValue,
    phantom: PhantomData<T>,
}

impl<T: PrismValue> Value<T> {
    /// Get a reference to the inner value.
    pub fn get_ref(&self) -> &T::Ref {
        T::get_any(&self.value)
    }

    /// Convert to the inner value
    pub fn get(self) -> T {
        T::from_any(self.value)
    }
}

impl<T: PrismValue> From<T> for Value<T> {
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
#[derive(Clone, Debug)]
pub struct AnyValue(AnyValueInner);

#[derive(Clone, Debug)]
enum AnyValueInner {
    Arc(Arc<dyn HeapValue>),
    U32(u32),
    I32(i32),
    // TODO: Events/Dynamics/Behaviors. Covered under `HeapValue` for now.
}

impl<T: PrismValue> From<AnyValue> for Value<T> {
    fn from(value: AnyValue) -> Self {
        T::get_any(&value); // Panic on mismatch
        Value {
            value,
            phantom: PhantomData,
        }
    }
}

impl<T: PrismValue> From<Value<T>> for AnyValue {
    fn from(value: Value<T>) -> Self {
        value.value
    }
}
