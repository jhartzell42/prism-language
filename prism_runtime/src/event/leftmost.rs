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
        *self.height.lock().unwrap() = None;
    }

    fn handle_main_subscription(&self, runtime: &Runtime, _: Value<Self::Inner>, _: usize) {
        let Some((winner, tag)) = self.winning_value.lock().unwrap().take() else {
            return;
        };
        self.subscription_managers[tag].notify(
            self.weak_self.clone(),
            Some(&self.events[tag]),
            runtime,
            || Some(winner),
        );
    }

    fn handle_early_subscription(&self, _: &Runtime, value: Value<Self::Inner>, tag: Self::Tag) {
        let mut winner = self.winning_value.lock().unwrap();
        if let Some((_, current_tag)) = *winner
            && current_tag < tag
        {
            return;
        }
        *winner = Some((value, tag));
    }
}

impl<T: ValueType> EventImpl<T> for LeftmostEvent<T> {
    fn subscribe(&self, cb: Weak<dyn super::EventCallback<T>>) {
        for (ix, man) in self.subscription_managers.iter().enumerate() {
            man.add_subscriber(self.weak_self.clone(), Some(&self.events[ix]), cb.clone());
        }
    }

    fn height(&self) -> usize {
        let height = self.events.iter().map(|e| e.0.height()).max().unwrap_or(0) + 1;
        *self.height.lock().unwrap() = Some(height);
        height
    }
}
