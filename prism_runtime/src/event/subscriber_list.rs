use std::{
    marker::PhantomData,
    sync::{Arc, Mutex, Weak},
};

use crate::{
    event::{Event, EventCallback, EventImpl},
    runtime::{Action, CURRENT_OCCURRENCE, Runtime},
    value::{Value, ValueType},
};

pub struct SubscriptionManager<T: ValueType, E: SubscriptionEvent<T>> {
    subscribers: Mutex<Vec<(Weak<dyn EventCallback<T>>, Option<u64>)>>,
    subscription: Mutex<Option<Arc<MainSubscription<T, E>>>>,
    tag: E::Tag,
    created_in: Option<u64>,
}

pub trait SubscriptionEvent<T: ValueType>: EventImpl<T> + 'static {
    type Inner: ValueType;
    type Tag: Clone + Send + Sync;

    /// Clears this event's cached height and passes the call on to its
    /// subscribers, stopping if the cache was already empty. See
    /// [`EventImpl::height`] for why stopping there is safe.
    fn invalidate_height(&self);
    fn handle_main_subscription(
        &self,
        runtime: &Runtime,
        value: Value<Self::Inner>,
        tag: Self::Tag,
    );

    fn handle_early_subscription(&self, _: &Runtime, _: Value<Self::Inner>, _: Self::Tag) {
        // Do nothing by default.
    }
}

impl<T: ValueType, E: SubscriptionEvent<T>> SubscriptionManager<T, E> {
    pub fn new(tag: E::Tag) -> Self {
        Self {
            subscribers: Mutex::new(vec![]),
            subscription: Mutex::new(None),
            tag,
            created_in: CURRENT_OCCURRENCE.get(),
        }
    }

    pub fn add_subscriber(
        &self,
        this: Weak<E>,
        event: Option<&Event<E::Inner>>,
        cb: Weak<dyn EventCallback<T>>,
    ) {
        let len = {
            let mut list = self.subscribers.lock().unwrap();
            let len = list.len();
            list.push((cb, CURRENT_OCCURRENCE.get()));
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
                    tag: self.tag.clone(),
                });
                let new_sub2: Arc<dyn EventCallback<E::Inner>> = new_sub.clone();
                event.0.subscribe(Arc::downgrade(&new_sub2));
                *sub = Some(new_sub);
            }
        }
    }

    pub fn consolidate(&self) -> Vec<Arc<dyn EventCallback<T>>> {
        let mut subscribers = self.subscribers.lock().unwrap();
        let mut new_subscribers = vec![];
        let mut to_notify = vec![];
        for (subscriber, added_in) in std::mem::take(&mut *subscribers) {
            if let Some(s) = subscriber.upgrade() {
                new_subscribers.push((subscriber, added_in));
                to_notify.push(s);
            }
        }
        *subscribers = new_subscribers;
        to_notify
    }

    // Notify can be lazy. Maybe we only even compute **whether** this thing triggers
    // if there's anyone actually listening.
    pub fn notify(
        &self,
        // Bookkeeping our other subscription
        this: Weak<E>,
        event: Option<&Event<E::Inner>>,
        // Propagation
        runtime: &Runtime,
        // Thunk to compute value
        value: impl FnOnce() -> Option<Value<T>>,
    ) -> bool {
        let current = CURRENT_OCCURRENCE.get();
        let (alive, ready) = {
            let mut list = self.subscribers.lock().unwrap();
            list.retain(|(subscriber, _)| subscriber.strong_count() > 0);
            let ready: Vec<_> = list
                .iter()
                .filter(|(_, added_in)| {
                    current.is_none() || *added_in != current || self.created_in == current
                })
                .filter_map(|(subscriber, _)| subscriber.upgrade())
                .collect();
            (list.len(), ready)
        };

        if alive == 0 {
            self.clear_subscriber();
            return false;
        }
        if !ready.is_empty()
            && let Some(value) = value()
        {
            for subscriber in ready {
                subscriber.event_fired(runtime, value.clone())
            }
        }

        self.populate_subscriber(this, event, false);

        true
    }
}

pub struct MainSubscription<T: ValueType, E: SubscriptionEvent<T>> {
    this: Weak<E>,
    phantom: PhantomData<T>,
    tag: E::Tag,
}

impl<T: ValueType, E: SubscriptionEvent<T>> EventCallback<E::Inner> for MainSubscription<T, E> {
    fn event_fired(&self, runtime: &Runtime, value: Value<E::Inner>) {
        let Some(this) = self.this.upgrade() else {
            return;
        };
        let height = this.height();
        let tag = self.tag.clone();
        this.handle_early_subscription(runtime, value.clone(), tag.clone());
        runtime.schedule(height, MainAction { this, value, tag })
    }

    fn invalidate_height(&self) {
        let Some(this) = self.this.upgrade() else {
            return;
        };
        this.invalidate_height();
    }
}

pub struct MainAction<T: ValueType, E: SubscriptionEvent<T>> {
    this: Arc<E>,
    value: Value<E::Inner>,
    tag: E::Tag,
}

impl<T: ValueType, E: SubscriptionEvent<T>> Action for MainAction<T, E> {
    fn act(self: Box<Self>, runtime: &Runtime) {
        let Self { this, value, tag } = *self;
        this.handle_main_subscription(runtime, value, tag);
    }
}
