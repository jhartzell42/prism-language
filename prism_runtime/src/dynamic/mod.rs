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

#[cfg(test)]
mod tests;

use std::sync::Arc;

use crate::{
    behavior::Behavior,
    event::{Event, OneOrBoth},
    value::{Value, ValueType},
};

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
#[derive(Debug)]
pub struct Dynamic<T: ValueType> {
    event: Event<T>,
    behavior: Behavior<T>,
}

impl<T: ValueType> Clone for Dynamic<T> {
    fn clone(&self) -> Self {
        Self {
            event: self.event.clone(),
            behavior: self.behavior.clone(),
        }
    }
}

impl<T: ValueType> Dynamic<T> {
    /// Construct a new dynamic from a behavior or an event.
    ///
    /// SAFETY: It must uphold the invariant.
    pub unsafe fn new(behavior: Behavior<T>, event: Event<T>) -> Dynamic<T> {
        Dynamic { event, behavior }
    }

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
    pub fn hold(initial: Value<T>, event: Event<T>) -> Dynamic<T> {
        Self {
            behavior: Behavior::hold(initial, event.clone()),
            event,
        }
    }

    /// Create a constant [`Dynamic`] that always has the value `val`.
    pub fn constant(val: Value<T>) -> Dynamic<T> {
        Self {
            behavior: Behavior::constant(val),
            event: Event::<T>::never(),
        }
    }

    /// Create a new [`Dynamic`] that always has the value `f(val)` where `val`
    /// is the current value of `self`.
    pub fn map<O: ValueType>(
        &self,
        f: impl Send + Sync + 'static + Clone + Fn(Value<T>) -> Value<O>,
    ) -> Dynamic<O> {
        Dynamic {
            behavior: self.behavior.map(f.clone()),
            event: self.event.filter_map(move |e| Some(f(e))),
        }
    }

    /// Combine two [`Dynamic`] values with a function. Output dynamic always
    /// equals `f(a,b)` for the current values of `a` and `b`, even if they
    /// update in the same occurrence.
    pub fn map2<A: ValueType, B: ValueType>(
        a: Dynamic<A>,
        b: Dynamic<B>,
        f: impl 'static + Clone + Send + Sync + Fn(Value<A>, Value<B>) -> Value<T>,
    ) -> Self {
        type Tagged<A, B> = OneOrBoth<Arc<(Value<A>, Value<B>)>, Arc<(Value<B>, Value<A>)>>;

        let behavior = Behavior::map2(f.clone(), a.behavior.clone(), b.behavior.clone());
        let a_event = a.event.tag(b.behavior);
        let b_event = b.event.tag(a.behavior);
        let combinator = move |one_or_both: Tagged<A, B>| match one_or_both {
            OneOrBoth::A(a) => {
                let a = a.extract();
                Some(f(a.0.clone(), a.1.clone()))
            }
            OneOrBoth::B(b) => {
                let b = b.extract();
                Some(f(b.1.clone(), b.0.clone()))
            }
            OneOrBoth::Both(a, b) => Some(f(a.extract().0, b.extract().0)),
        };
        let event = Event::combine(combinator, a_event, b_event);
        Self { behavior, event }
    }
}

impl<T: ValueType> From<Dynamic<T>> for Behavior<T> {
    fn from(value: Dynamic<T>) -> Self {
        value.behavior()
    }
}
