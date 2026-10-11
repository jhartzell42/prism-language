//! An [`Event`] represents a potential value at every edge, every instant in
//! time.  During an "occurrence," when the runtime processes a state change, an
//! [`Event`] can either be firing or not firing. If it is firing, it contains a
//! value for that occurrence.
//!
//! Events are the **push** component of the push-pull architecture. Events
//! trigger other events, but their values are ephemeral. Events only implement
//! triggering and compute their value if someone is subscribed to them.

use std::any::type_name;
use std::fmt::Debug;
use std::sync::{Arc, Weak};

use crate::runtime::Runtime;
use crate::value::{Value, ValueType};

mod combine;
mod external;
mod filter_map;
pub(crate) mod forward;
mod leftmost;
mod never;
pub(crate) mod subscriber_list;
mod switch_hold;
mod tag;
mod trace;

#[cfg(test)]
mod tests;

pub use combine::OneOrBoth;
pub use external::{EventTrigger, ScheduleError};

/// This is a handle for an event that contains a value of type `T` in
/// any occurrence for which it is fired.
pub struct Event<T: ValueType>(pub(crate) Arc<dyn EventImpl<T>>);

impl<T: ValueType> Debug for Event<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_name_content = type_name::<T>();
        f.debug_tuple("Event").field(&type_name_content).finish()
    }
}

impl<T: ValueType> Clone for Event<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub(crate) trait EventCallback<T: ValueType>: Send + Sync {
    fn event_fired(&self, runtime: &Runtime, value: Value<T>);
    /// Called at subscription time when the event already fired in the
    /// occurrence now propagating, with the value it fired with.
    fn already_fired(&self, runtime: &Runtime, value: Value<T>);
    /// Called when the height of the event this callback subscribes to may
    /// have changed. Implementations clear any height they cached and pass
    /// the call on to their own subscribers. See [`EventImpl::height`] for
    /// the rule that lets this stop early.
    fn invalidate_height(&self);
}

pub(crate) trait EventImpl<T: ValueType>: Send + Sync + 'static {
    // When you subscribe to an event, you are responsible for making sure
    // the event callback stays alive, or otherwise, your subscription will vanish.
    //
    // If no one cares if an event triggers, then we won't compute any values
    // associated with the event.
    //
    // Additionally, if you subscribe to an event, you're responsible for
    // keeping the event itself alive, or else your callback won't get called.
    //
    // If this seems complicated, this is exactly why callbacks are nasty.  The
    // ability to completely avoid this sort of thing in user code is a major
    // reason why FRP exists.
    //
    // The runtime is the occurrence the subscription is made in: if the
    // event already fired in it, `subscribe` returns that value so the
    // subscriber can catch up.
    fn subscribe(&self, runtime: &Runtime, cb: Weak<dyn EventCallback<T>>) -> Option<Value<T>>;
    /// One more than the height of every event this one is computed from.
    ///
    /// Heights may be cached. Invalidation of cached heights stops early at
    /// any node whose cache is already empty. Stopping early is safe only
    /// while every node that caches a height also caches whatever it
    /// computed that height from. Every new event type must follow this
    /// rule: if it caches its height, it must get that height by calling
    /// `height` on its upstream events, which caches theirs, and its
    /// `invalidate_height` must clear its cache and pass the call on to its
    /// subscribers.
    fn height(&self) -> usize;
}
