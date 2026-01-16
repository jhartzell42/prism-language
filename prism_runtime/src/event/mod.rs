//! An [`Event`] represents a potential value at every edge, every instant in
//! time.  During an "occurrence," when the runtime processes a state change, an
//! [`Event`] can either be firing or not firing. If it is firing, it contains a
//! value for that occurrence.
//!
//! Events are the **push** component of the push-pull architecture. Events
//! trigger other events, but their values are ephemeral. Events only implement
//! triggering and compute their value if someone is subscribed to them.

use std::fmt::Debug;
use std::{
    marker::PhantomData,
    sync::{Arc, Weak},
};

use crate::runtime::Runtime;

mod external;
mod filter_map;
mod never;
mod subscriber_list;
mod switch_hold;
mod tag;
#[cfg(test)]
mod test;

/// This is a handle for an event that contains a value of type `T` in
/// any occurrence for which it is triggered.
pub struct Event<T>(pub(crate) Arc<dyn EventImpl<T>>);

impl<T> Clone for Event<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub(crate) trait EventCallback<T> {
    fn event_fired(&self, runtime: &Runtime, value: Arc<T>);
}

pub(crate) trait EventImpl<T> {
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

impl<T: Debug + 'static> Event<T> {
    /// This logs all triggerings of the event, at `log::debug!` level.
    pub fn trace(&self, label: String) {
        struct Tracer<T> {
            label: String,
            phantom: PhantomData<T>,
        }

        impl<T: Debug> EventCallback<T> for Tracer<T> {
            fn event_fired(&self, _: &Runtime, value: Arc<T>) {
                log::trace!("{}: {value:?}", self.label)
            }
        }

        let tracer: Arc<dyn EventCallback<T>> = Arc::new(Tracer {
            label,
            phantom: PhantomData,
        });
        self.0.subscribe(Arc::downgrade(&tracer));

        // Leak tracer. Continue subscribing for rest of the program.
        std::mem::forget(tracer);
    }
}
