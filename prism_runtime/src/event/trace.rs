use std::{marker::PhantomData, sync::Arc};

use crate::{
    event::{Event, EventCallback},
    runtime::Runtime,
    value::{Value, ValueType},
};

impl<T: ValueType> Event<T> {
    /// This logs all triggerings of the event, at `log::trace!` level.
    ///
    /// It's strictly for debugging purposes as it leaks memory and violates
    /// many design principles.
    pub fn trace(&self, label: String) {
        struct Tracer<T: ValueType> {
            label: String,
            phantom: PhantomData<T>,
        }

        impl<T: ValueType> EventCallback<T> for Tracer<T> {
            fn event_fired(&self, _: &Runtime, value: Value<T>) {
                log::trace!("{}: {value:?}", self.label)
            }

            fn invalidate_height(&self) {}
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
