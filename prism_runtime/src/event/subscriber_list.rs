use std::sync::{Arc, Mutex, Weak};

use crate::{event::EventCallback, runtime::Runtime};

pub(super) struct SubscriberList<T>(Mutex<Vec<Weak<dyn EventCallback<T>>>>);

impl<T> SubscriberList<T> {
    pub(super) fn new() -> Self {
        Self(Mutex::new(vec![]))
    }

    pub(super) fn add(&self, cb: Weak<dyn EventCallback<T>>) {
        let mut this = self.0.lock().unwrap();
        this.push(cb);
    }

    pub(super) fn notify(&self, runtime: &Runtime, value: Arc<T>) -> bool {
        let subscribers = self.consolidate();
        let notified_anyone = !subscribers.is_empty();
        for subscriber in subscribers {
            subscriber.event_fired(runtime, value.clone())
        }
        notified_anyone
    }

    pub(super) fn consolidate(&self) -> Vec<Arc<dyn EventCallback<T>>> {
        let mut subscribers = self.0.lock().unwrap();
        let mut new_subscribers = vec![];
        let mut to_notify = vec![];
        for subscriber in std::mem::take(&mut *subscribers) {
            if let Some(s) = subscriber.upgrade() {
                new_subscribers.push(subscriber);
                to_notify.push(s);
            }
        }
        *subscribers = new_subscribers;
        to_notify
    }
}
