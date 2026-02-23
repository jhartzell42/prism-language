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
mod leftmost;
mod never;
pub(crate) mod subscriber_list;
mod switch_hold;
mod tag;
mod trace;

#[cfg(test)]
mod tests;

pub use combine::OneOrBoth;
pub use external::EventTrigger;

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
    fn subscribe(&self, cb: Weak<dyn EventCallback<T>>);
    fn height(&self) -> usize;
}
