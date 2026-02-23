use std::sync::{Arc, Mutex, Weak};

use crate::{
    event::{
        Event, EventImpl,
        subscriber_list::{SubscriptionEvent, SubscriptionManager},
    },
    runtime::Runtime,
    value::{Value, ValueType},
};

impl<T: ValueType> Event<T> {
    /// Creates a new event based on the existing one and the function `f`.
    ///
    /// If the existing event is fired, run `f` on the value.
    ///
    /// If `f` returns `Some(value)`, the new event is fired with
    /// the value `value`. If `f` returns `None`, the new event is not fired.
    pub fn filter_map<O: ValueType>(
        &self,
        f: impl Send + Sync + Fn(Value<T>) -> Option<Value<O>> + 'static,
    ) -> Event<O> {
        Event(Arc::new_cyclic(|weak| FilterMapEvent {
            subscriber_list: SubscriptionManager::new(()),
            inner: self.clone(),
            weak_self: weak.clone(),
            f,
            height: Mutex::new(None),
        }))
    }

    /// Make a new event that fires every time the existing one fires,
    /// but with a value transformed by `f`.
    pub fn map<O: ValueType>(
        &self,
        f: impl Send + Sync + Fn(Value<T>) -> Value<O> + 'static,
    ) -> Event<O> {
        self.filter_map(move |x| Some(f(x)))
    }
}

struct FilterMapEvent<
    O: ValueType,
    T: ValueType,
    F: 'static + Send + Sync + Fn(Value<T>) -> Option<Value<O>>,
> {
    subscriber_list: SubscriptionManager<O, Self>,
    inner: Event<T>,
    weak_self: Weak<Self>,
    f: F,
    height: Mutex<Option<usize>>,
}

impl<O: ValueType, T: ValueType, F: 'static + Send + Sync + Fn(Value<T>) -> Option<Value<O>>>
    SubscriptionEvent<O> for FilterMapEvent<O, T, F>
{
    type Inner = T;
    type Tag = ();

    fn invalidate_height(&self) {
        *self.height.lock().unwrap() = None;
    }

    fn handle_main_subscription(&self, runtime: &Runtime, value: Value<Self::Inner>, _: ()) {
        self.subscriber_list
            .notify(self.weak_self.clone(), Some(&self.inner), runtime, || {
                (self.f)(value)
            });
    }
}
impl<O: ValueType, T: ValueType, F: 'static + Send + Sync + Fn(Value<T>) -> Option<Value<O>>>
    EventImpl<O> for FilterMapEvent<O, T, F>
{
    fn subscribe(&self, cb: std::sync::Weak<dyn super::EventCallback<O>>) {
        self.subscriber_list
            .add_subscriber(self.weak_self.clone(), Some(&self.inner), cb);
    }

    fn height(&self) -> usize {
        let mut height = self.height.lock().unwrap();
        if let Some(height) = *height {
            return height;
        }
        let new_height = self.inner.0.height() + 1;
        *height = Some(new_height);
        new_height
    }
}
