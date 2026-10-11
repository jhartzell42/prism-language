use std::sync::Arc;

use crate::{
    event::{Event, EventImpl},
    value::{Value, ValueType},
};

impl<T: ValueType> Event<T> {
    /// Returns an event that is never firing.
    pub fn never() -> Self {
        struct Never;

        impl<T: ValueType> EventImpl<T> for Never {
            fn subscribe(&self, _: &crate::runtime::Runtime, _: std::sync::Weak<dyn super::EventCallback<T>>) -> Option<Value<T>> {
                None
            }

            fn height(&self) -> usize {
                0
            }
        }

        Self(Arc::new(Never))
    }
}
