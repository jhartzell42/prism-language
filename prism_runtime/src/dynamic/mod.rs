//! A [`Dynamic`] is a value that changes over time, implemented with both the
//! push and pull nature of push-pull FRP. Generally, we use [`Dynamic`] over
//! [`Behavior`], and only [`Dynamic`] is exposed as a programming-language
//! concept.
//!
//! Internally, it contains an [`Event`] and a [`Behavior`]. The event
//! represents the "push" nature and the behavior represents the "pull" nature.
//! There is an invariant tying them together, so that the behavior should
//! always contain the last fired event.
//!
//! A `Dynamic` has similar semantics to a raw `Behavior`, but more
//! capabilities. There is little performance cost to these additional
//! capabilities---since both events and behaviors are lazy, using a `Dynamic`
//! only does work on the side where it's being used.  That is, if it only needs
//! to be computed on push, then only the event side is activated. If it only
//! needs to be computed on pull, then only the behavior side is activated.
//! Events won't even compute whether they're triggering unless someone is
//! subscribing to them.

use std::sync::Arc;

use crate::{behavior::Behavior, event::Event};

/// This represents an actual dynamic value.
///
/// It includes a read-only [`Event`] and [`Behavior`]. If you want a new
/// combination of events and behaviors, construct a new dynamic.
///
/// The event is guaranteed to fire whenever the behavior changes, and the
/// behavior is guaranteed to always contain the new value of an event.
/// This invariant is essential to maintaining everyone's sanity.
///
/// All safe ways of constructing a dynamic should uphold that guarantee. While
/// [`Dynamic::new_unchecked()`] is available to construct a dynamic raw, you
/// should only use it when you're willing to uphold this guarantee.
#[derive(Clone)]
pub struct Dynamic<T> {
    event: Event<T>,
    behavior: Behavior<T>,
}

impl<T: 'static> Dynamic<T> {
    /// Get the event that fires whenever the [`Dynamic`] changes.
    pub fn event(&self) -> Event<T> {
        self.event.clone()
    }

    /// Get the [`Behavior`] that represents the current value.
    pub fn behavior(&self) -> Behavior<T> {
        self.behavior.clone()
    }

    /// Create a [`Dynamic`] that always has the last value of `event`,
    /// starting with the value `initial`.
    pub fn hold(initial: Arc<T>, event: Event<T>) -> Dynamic<T> {
        Self {
            behavior: Behavior::hold(initial, event.clone()),
            event: event,
        }
    }

    /// Create a constant [`Dynamic`] that always has the value `val`.
    pub fn constant(val: Arc<T>) -> Dynamic<T> {
        Self {
            behavior: Behavior::constant(val),
            event: Event::<T>::never(),
        }
    }
}

impl<T: 'static> From<Dynamic<T>> for Behavior<T> {
    fn from(value: Dynamic<T>) -> Self {
        value.behavior()
    }
}
