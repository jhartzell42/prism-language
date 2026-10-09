use std::{
    marker::PhantomData,
    sync::{Arc, Mutex, Weak},
};

use crate::{
    event::{Event, EventCallback, EventImpl},
    runtime::{Action, CLEAR_AT_END, CURRENT_HEIGHT, Runtime},
    value::{Value, ValueType},
};

pub struct SubscriptionManager<T: ValueType, E: SubscriptionEvent<T>> {
    subscribers: Mutex<Vec<Weak<dyn EventCallback<T>>>>,
    subscription: Mutex<Option<Arc<MainSubscription<T, E>>>>,
    tag: E::Tag,
    occurrence: Arc<Mutex<Option<Option<Value<T>>>>>,
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

    fn handle_already_fired(&self, value: Value<Self::Inner>, tag: Self::Tag);

    fn finish_already_fired(&self) {}
}

impl<T: ValueType, E: SubscriptionEvent<T>> SubscriptionManager<T, E> {
    pub fn new(tag: E::Tag) -> Self {
        Self {
            subscribers: Mutex::new(vec![]),
            subscription: Mutex::new(None),
            tag,
            occurrence: Arc::new(Mutex::new(None)),
        }
    }

    pub fn add_subscriber(
        &self,
        this: Weak<E>,
        event: Option<&Event<E::Inner>>,
        cb: Weak<dyn EventCallback<T>>,
    ) -> Option<Value<T>> {
        self.connect(this, event);
        self.push_subscriber(cb);
        self.current()
    }

    pub(crate) fn push_subscriber(&self, cb: Weak<dyn EventCallback<T>>) {
        let len = {
            let mut list = self.subscribers.lock().unwrap();
            let len = list.len();
            list.push(cb);
            len
        };
        if len > 0 && len % 16 == 0 {
            // Every once in a while, we should check to see if these weak
            // pointers are all still pointing somewhere.
            //
            // TODO: Make a real `WeakBag` implementation.
            self.consolidate();
        }
    }

    pub(crate) fn connect(&self, this: Weak<E>, event: Option<&Event<E::Inner>>) {
        let Some(event) = event else {
            return;
        };
        if self.subscription.lock().unwrap().is_some() {
            return;
        }
        if let Some(value) = self.resubscribe(this.clone(), event)
            && let Some(this) = this.upgrade()
        {
            this.handle_already_fired(value, self.tag.clone());
        }
    }

    pub(crate) fn resubscribe(
        &self,
        this: Weak<E>,
        event: &Event<E::Inner>,
    ) -> Option<Value<E::Inner>> {
        let new_sub = Arc::new(MainSubscription {
            this,
            phantom: PhantomData,
            tag: self.tag.clone(),
        });
        *self.subscription.lock().unwrap() = Some(new_sub.clone());
        let new_sub: Arc<dyn EventCallback<E::Inner>> = new_sub;
        event.0.subscribe(Arc::downgrade(&new_sub))
    }

    pub(crate) fn clear_subscriber(&self) {
        *self.subscription.lock().unwrap() = None;
    }

    pub fn consolidate(&self) -> Vec<Arc<dyn EventCallback<T>>> {
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

    pub(crate) fn current(&self) -> Option<Value<T>> {
        self.occurrence.lock().unwrap().clone().flatten()
    }

    pub(crate) fn record_once(
        &self,
        compute: impl FnOnce() -> Option<Value<T>>,
    ) -> Option<Value<T>> {
        if self.occurrence.lock().unwrap().is_some() || CURRENT_HEIGHT.get().is_none() {
            return None;
        }
        let value = compute();
        let mut occurrence = self.occurrence.lock().unwrap();
        if occurrence.is_some() {
            return None;
        }
        *occurrence = Some(value.clone());
        let cleared = self.occurrence.clone();
        CLEAR_AT_END.with_borrow_mut(|clears| {
            clears.push(Box::new(move || *cleared.lock().unwrap() = None))
        });
        value
    }

    pub(crate) fn fire(&self, runtime: &Runtime, value: Value<T>) {
        if let Some(value) = self.record_once(|| Some(value)) {
            for subscriber in self.consolidate() {
                subscriber.event_fired(runtime, value.clone())
            }
        }
    }

    pub(crate) fn deliver_already_fired(&self, value: Value<T>) {
        for subscriber in self.consolidate() {
            subscriber.already_fired(value.clone())
        }
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
        let subscribers = self.consolidate();

        if subscribers.is_empty() {
            self.clear_subscriber();
            return false;
        }
        if let Some(value) = self.record_once(value) {
            for subscriber in subscribers {
                subscriber.event_fired(runtime, value.clone())
            }
        }

        self.connect(this, event);

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
        if CURRENT_HEIGHT.get().is_some_and(|current| height <= current) {
            this.handle_already_fired(value, tag);
            this.finish_already_fired();
            return;
        }
        this.handle_early_subscription(runtime, value.clone(), tag.clone());
        runtime.schedule(height, MainAction { this, value, tag })
    }

    fn already_fired(&self, value: Value<E::Inner>) {
        let Some(this) = self.this.upgrade() else {
            return;
        };
        this.handle_already_fired(value, self.tag.clone());
        this.finish_already_fired();
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
