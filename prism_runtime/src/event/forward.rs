use std::sync::{Arc, Mutex, OnceLock, Weak};

use crate::{
    event::{
        Event, EventCallback, EventImpl,
        subscriber_list::{SubscriptionEvent, SubscriptionManager},
    },
    runtime::Runtime,
    value::{Value, ValueType},
};

pub(crate) struct ForwardEvent<T: ValueType> {
    inner: OnceLock<Weak<dyn EventImpl<T>>>,
    subscriber_list: SubscriptionManager<T, Self>,
    weak_self: Weak<Self>,
    height: Mutex<Option<usize>>,
}

impl<T: ValueType> ForwardEvent<T> {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new_cyclic(|weak| ForwardEvent {
            inner: OnceLock::new(),
            subscriber_list: SubscriptionManager::new(()),
            weak_self: weak.clone(),
            height: Mutex::new(None),
        })
    }

    pub(crate) fn close(&self, runtime: &Runtime, event: &Event<T>) {
        if self.inner.set(Arc::downgrade(&event.0)).is_err() {
            panic!("cyclic event closed twice");
        }
        if !self.subscriber_list.consolidate().is_empty()
            && let Some(value) = self
                .subscriber_list
                .resubscribe(runtime, self.weak_self.clone(), event)
        {
            self.handle_already_fired(runtime, value, ());
        }
    }

    fn target(&self) -> Option<Event<T>> {
        self.inner.get().and_then(Weak::upgrade).map(Event)
    }
}

impl<T: ValueType> EventImpl<T> for ForwardEvent<T> {
    fn subscribe(&self, runtime: &Runtime, cb: Weak<dyn EventCallback<T>>) -> Option<Value<T>> {
        self.subscriber_list
            .add_subscriber(runtime, self.weak_self.clone(), self.target().as_ref(), cb)
    }

    fn height(&self) -> usize {
        let mut height = self
            .height
            .try_lock()
            .unwrap_or_else(|_| panic!("event cycle: an event depends on itself in one occurrence"));
        if let Some(height) = *height {
            return height;
        }
        let new_height = self.target().map(|e| e.0.height() + 1).unwrap_or(0);
        *height = Some(new_height);
        new_height
    }
}

impl<T: ValueType> SubscriptionEvent<T> for ForwardEvent<T> {
    type Inner = T;
    type Tag = ();

    fn invalidate_height(&self) {
        if self.height.lock().unwrap().take().is_none() {
            return;
        }
        for subscriber in self.subscriber_list.consolidate() {
            subscriber.invalidate_height();
        }
    }

    fn handle_main_subscription(&self, runtime: &Runtime, value: Value<T>, _: ()) {
        self.subscriber_list
            .notify(self.weak_self.clone(), self.target().as_ref(), runtime, || Some(value));
    }

    fn handle_already_fired(&self, runtime: &Runtime, value: Value<T>, _: ()) {
        if let Some(value) = self.subscriber_list.record_once(runtime, || Some(value)) {
            self.subscriber_list.deliver_already_fired(runtime, value);
        }
    }
}
