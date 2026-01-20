use std::{
    marker::PhantomData,
    sync::{Arc, Mutex, Weak},
};

use crate::{
    event::{Event, EventCallback, EventImpl},
    runtime::{Action, Runtime},
};

pub(super) struct SubscriptionManager<T, E: SubscriptionEvent<T>> {
    subscribers: Mutex<Vec<Weak<dyn EventCallback<T>>>>,
    subscription: Mutex<Option<Arc<MainSubscription<T, E>>>>,
}

pub(super) trait SubscriptionEvent<T>: EventImpl<T> + 'static {
    type Inner: 'static;
    fn invalidate_height(&self);
    fn handle_main_subscription(&self, runtime: &Runtime, value: Arc<Self::Inner>);
}

impl<T: 'static, E: SubscriptionEvent<T>> SubscriptionManager<T, E> {
    pub(super) fn new() -> Self {
        Self {
            subscribers: Mutex::new(vec![]),
            subscription: Mutex::new(None),
        }
    }

    pub(super) fn add_subscriber(
        &self,
        this: Weak<E>,
        event: Option<&Event<E::Inner>>,
        cb: Weak<dyn EventCallback<T>>,
    ) {
        let len = {
            let mut list = self.subscribers.lock().unwrap();
            let len = list.len();
            list.push(cb);
            len
        };
        if len == 0 {
            self.populate_subscriber(this, event, false);
        } else if len % 16 == 0 {
            // Every once in a while, we should check to see if these weak
            // pointers are all still pointing somewhere.
            //
            // TODO: Make a real `WeakBag` implementation.
            self.consolidate();
        }
    }

    pub(crate) fn clear_subscriber(&self) {
        *self.subscription.lock().unwrap() = None;
    }

    pub(crate) fn populate_subscriber(
        &self,
        this: Weak<E>,
        event: Option<&Event<E::Inner>>,
        // If true, reconstruct even if there already is one. If false, recycle existing.
        refresh: bool,
    ) {
        if let Some(event) = event {
            let mut sub = self.subscription.lock().unwrap();
            if sub.is_none() || refresh {
                let new_sub = Arc::new(MainSubscription {
                    this,
                    phantom: PhantomData,
                });
                let new_sub2: Arc<dyn EventCallback<E::Inner>> = new_sub.clone();
                event.0.subscribe(Arc::downgrade(&new_sub2));
                *sub = Some(new_sub);
            }
        }
    }

    pub(super) fn consolidate(&self) -> Vec<Arc<dyn EventCallback<T>>> {
        let mut subscribers = self.subscribers.lock().unwrap();
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

    // Notify can be lazy. Maybe we only even compute **whether** this thing triggers
    // if there's anyone actually listening.
    pub(super) fn notify(
        &self,
        // Bookkeeping our other subscription
        this: Weak<E>,
        event: Option<&Event<E::Inner>>,
        // Propagation
        runtime: &Runtime,
        // Thunk to compute value
        value: impl FnOnce() -> Option<Arc<T>>,
    ) -> bool {
        let subscribers = self.consolidate();

        if subscribers.is_empty() {
            self.clear_subscriber();
            return false;
        }
        if let Some(value) = value() {
            for subscriber in subscribers {
                subscriber.event_fired(runtime, value.clone())
            }
        }

        self.populate_subscriber(this, event, false);

        true
    }
}

pub(super) struct MainSubscription<T, E: SubscriptionEvent<T>> {
    this: Weak<E>,
    phantom: PhantomData<T>,
}

impl<T: 'static, E: SubscriptionEvent<T>> EventCallback<E::Inner> for MainSubscription<T, E> {
    fn event_fired(&self, runtime: &Runtime, value: Arc<E::Inner>) {
        let Some(this) = self.this.upgrade() else {
            return;
        };
        let height = this.height();
        runtime.schedule(height, MainAction { this, value })
    }

    fn invalidate_height(&self) {
        let Some(this) = self.this.upgrade() else {
            return;
        };
        this.invalidate_height();
    }
}

pub(super) struct MainAction<T, E: SubscriptionEvent<T>> {
    this: Arc<E>,
    value: Arc<E::Inner>,
}

impl<T, E: SubscriptionEvent<T>> Action for MainAction<T, E> {
    fn act(self: Box<Self>, runtime: &Runtime) {
        let Self { this, value } = *self;
        this.handle_main_subscription(runtime, value);
    }
}
