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

use std::sync::{Arc, Weak};

mod constant;
mod derived;
mod hold;

#[cfg(test)]
mod tests;

/// This represents a handle of a behavior, useful for querying the value and
/// constructing other behaviors.
pub struct Behavior<T: ?Sized + Send + Sync>(pub(crate) Arc<dyn BehaviorImpl<T>>);

impl<T: ?Sized + Send + Sync> Clone for Behavior<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub(crate) trait BehaviorDependent: Send + Sync {
    fn invalidate(&self);
}

pub(crate) trait BehaviorImpl<T: ?Sized>: Send + Sync {
    // What is the current value of the behavior?
    fn query_for_behavior(&self, dependent: Weak<dyn BehaviorDependent>) -> Arc<T>;
    fn query_for_tag(&self) -> Arc<T>;
}
