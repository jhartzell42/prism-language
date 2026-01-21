use std::sync::{Arc, Mutex, Weak};

use crate::{
    event::{
        Event, EventImpl,
        subscriber_list::{SubscriptionEvent, SubscriptionManager},
    },
    runtime::Runtime,
};

impl<T: 'static> Event<T> {
    /// Creates a new event based on the existing one and the function `f`.
    ///
    /// If the existing event is triggered, run `f` on the value.
    ///
    /// If `f` returns `Some(value)`, the new event is triggered with
    /// the value `value`. If `f` returns `None`, the new event is not triggered.
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

struct LeftmostEvent<T: 'static> {
    events: Vec<Event<T>>,
    subscription_managers: Vec<SubscriptionManager<T, Self>>,
    weak_self: Weak<Self>,
    height: Mutex<Option<usize>>,
    winning_value: Mutex<Option<(Arc<T>, usize)>>,
}

impl<T: 'static> SubscriptionEvent<T> for LeftmostEvent<T> {
    type Inner = T;
    type Tag = usize;

    fn invalidate_height(&self) {
        *self.height.lock().unwrap() = None;
    }

    fn handle_main_subscription(&self, runtime: &Runtime, _: Arc<Self::Inner>, _: usize) {
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

    fn handle_early_subscription(&self, _: &Runtime, value: Arc<Self::Inner>, tag: Self::Tag) {
        let mut winner = self.winning_value.lock().unwrap();
        if let Some((_, current_tag)) = *winner
            && current_tag < tag
        {
            return;
        } else {
            *winner = Some((value, tag));
        }
    }
}

impl<T: 'static> EventImpl<T> for LeftmostEvent<T> {
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
