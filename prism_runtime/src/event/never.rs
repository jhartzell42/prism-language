use std::sync::Arc;

use crate::event::{Event, EventImpl};

impl<T> Event<T> {
    /// Returns an event that is never firing.
    pub fn never() -> Self {
        struct Never;

        impl<T> EventImpl<T> for Never {
            fn subscribe(&self, _: std::sync::Weak<dyn super::EventCallback<T>>) {
                // Don't need this callback
            }

            fn height(&self) -> usize {
                0
            }
        }

        Self(Arc::new(Never))
    }
}
