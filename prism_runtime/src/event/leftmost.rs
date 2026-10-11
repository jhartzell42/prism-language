use std::sync::{Arc, Mutex, Weak};

use crate::{
    event::{
        Event, EventImpl,
        subscriber_list::{SubscriptionEvent, SubscriptionManager},
    },
    runtime::{Action, Runtime},
    value::{Value, ValueType},
};

impl<T: ValueType> Event<T> {
    /// Creates a new event based on the existing one and the function `f`.
    ///
    /// If the existing event is fired, run `f` on the value.
    ///
    /// If `f` returns `Some(value)`, the new event is fired with
    /// the value `value`. If `f` returns `None`, the new event is not fired.
    pub fn leftmost(events: Vec<Event<T>>) -> Event<T> {
        let subscription_managers = (0..events.len())
            .map(|i| SubscriptionManager::new(i))
            .collect();
        Event(Arc::new_cyclic(|weak| LeftmostEvent {
            events,
            subscription_managers,
            weak_self: weak.clone(),
            height: Mutex::new(None),
            winning_value: Mutex::new(None),
        }))
    }
}

struct LeftmostEvent<T: ValueType> {
    events: Vec<Event<T>>,
    subscription_managers: Vec<SubscriptionManager<T, Self>>,
    weak_self: Weak<Self>,
    height: Mutex<Option<usize>>,
    winning_value: Mutex<Option<(Value<T>, usize)>>,
}

impl<T: ValueType> SubscriptionEvent<T> for LeftmostEvent<T> {
    type Inner = T;
    type Tag = usize;

    fn invalidate_height(&self) {
        if self.height.lock().unwrap().take().is_none() {
            return;
        }
        for manager in &self.subscription_managers {
            for subscriber in manager.consolidate() {
                subscriber.invalidate_height();
            }
        }
    }

    fn handle_main_subscription(&self, runtime: &Runtime, _: Value<Self::Inner>, _: usize) {
        self.fire(runtime);
    }

    fn handle_early_subscription(&self, _: &Runtime, value: Value<Self::Inner>, tag: Self::Tag) {
        self.compete(value, tag);
    }

    fn handle_already_fired(&self, _: &Runtime, value: Value<Self::Inner>, tag: Self::Tag) {
        self.compete(value, tag);
    }

    fn finish_already_fired(&self, runtime: &Runtime) {
        let Some(current) = runtime.current_height() else {
            return;
        };
        if self.winning_value.lock().unwrap().is_none() {
            return;
        }
        let height = self.height();
        if height > current {
            if let Some(this) = self.weak_self.upgrade() {
                runtime.schedule(height, FireLeftmost(this));
            }
            return;
        }
        let Some((winner, _)) = self.winning_value.lock().unwrap().take() else {
            return;
        };
        if let Some(value) = self.subscription_managers[0].record_once(runtime, || Some(winner)) {
            self.subscription_managers[0].deliver_already_fired(runtime, value);
        }
    }
}

impl<T: ValueType> LeftmostEvent<T> {
    fn fire(&self, runtime: &Runtime) {
        let Some((winner, _)) = self.winning_value.lock().unwrap().take() else {
            return;
        };
        self.subscription_managers[0].notify(
            self.weak_self.clone(),
            Some(&self.events[0]),
            runtime,
            || Some(winner),
        );
    }

    fn compete(&self, value: Value<T>, tag: usize) {
        let mut winner = self.winning_value.lock().unwrap();
        if let Some((_, current_tag)) = *winner
            && current_tag < tag
        {
            return;
        }
        *winner = Some((value, tag));
    }
}

struct FireLeftmost<T: ValueType>(Arc<LeftmostEvent<T>>);

impl<T: ValueType> Action for FireLeftmost<T> {
    fn act(self: Box<Self>, runtime: &Runtime) {
        self.0.fire(runtime);
    }
}

impl<T: ValueType> EventImpl<T> for LeftmostEvent<T> {
    fn subscribe(&self, runtime: &Runtime, cb: Weak<dyn super::EventCallback<T>>) -> Option<Value<T>> {
        for (ix, man) in self.subscription_managers.iter().enumerate() {
            man.connect(runtime, self.weak_self.clone(), Some(&self.events[ix]));
        }
        self.finish_already_fired(runtime);
        for man in &self.subscription_managers {
            man.push_subscriber(cb.clone());
        }
        self.subscription_managers.first()?.current()
    }

    fn height(&self) -> usize {
        let height = self.events.iter().map(|e| e.0.height()).max().unwrap_or(0) + 1;
        *self.height.lock().unwrap() = Some(height);
        height
    }
}
