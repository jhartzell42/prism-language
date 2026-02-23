//! A [`Behavior`] represents the ability to query at any time for a lazy value that
//! changes over time.
//!
//! [`Behavior`] is the **pull** component of push-pull propagation. When you
//! query a value, it goes and finds out what the current value should be,
//! pulling the data. Behavior values in general are only computed if they're
//! actually queried.
//!
//! Behaviors can originate from:
//!
//! 1. Holding the last fired value of an event.
//! 2. A constant value.
//! 3. A pure function of other behaviors.
//! 4. An external state.

use std::{
    any::type_name,
    sync::{Arc, Weak},
};

mod compute;
mod constant;
mod hold;

pub use compute::*;

use std::fmt::Debug;

use crate::value::{Value, ValueType};

/// This represents a handle of a behavior, useful for querying the value and
/// constructing other behaviors.
pub struct Behavior<T: ValueType>(pub(crate) Arc<dyn BehaviorImpl<T>>);

impl<T: ValueType> Debug for Behavior<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_name_content = type_name::<T>();
        f.debug_tuple("Behavior").field(&type_name_content).finish()
    }
}

impl<T: ValueType> Clone for Behavior<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub(crate) trait BehaviorDependent: Send + Sync {
    fn invalidate(&self);
}

pub(crate) trait BehaviorImpl<T: ValueType>: Send + Sync {
    // What is the current value of the behavior?
    fn query_for_behavior(&self, dependent: Weak<dyn BehaviorDependent>) -> Value<T>;
    fn query_for_tag(&self) -> Value<T>;
}
