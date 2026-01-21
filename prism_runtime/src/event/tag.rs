use std::sync::{Arc, Mutex, Weak};

use crate::{
    behavior::Behavior,
    event::{
        Event, EventImpl,
        subscriber_list::{SubscriptionEvent, SubscriptionManager},
    },
    runtime::Runtime,
};

impl<A: 'static> Event<A> {
    /// This creates a new event that is triggered whenever `self` is triggered.
    /// It augments the value of `self` with the value of a [`Dynamic`] or
    /// [`Behavior`].
    ///
    /// It is **not** *prompt*, that is, it doesn't reflect updates to the
    /// tagged value that happened during this occurrence. Promptness is
    /// basically never what you want anyway.
    pub fn tag<B: 'static>(&self, behavior: impl Into<Behavior<B>>) -> Event<(Arc<A>, Arc<B>)> {
        Event(Arc::new_cyclic(|weak| Tag {
            event: self.clone(),
            behavior: behavior.into(),
            subscriber_list: SubscriptionManager::new(()),
            height: Mutex::new(None),
            weak_self: weak.clone(),
        }))
    }
}

struct Tag<A: 'static, B: 'static> {
    subscriber_list: SubscriptionManager<(Arc<A>, Arc<B>), Self>,
    event: Event<A>,
    behavior: Behavior<B>,
    height: Mutex<Option<usize>>,
    weak_self: Weak<Self>,
}

impl<A: 'static, B> SubscriptionEvent<(Arc<A>, Arc<B>)> for Tag<A, B> {
    type Inner = A;
    type Tag = ();

    fn invalidate_height(&self) {
        *self.height.lock().unwrap() = None;
    }

    fn handle_main_subscription(&self, runtime: &Runtime, value: Arc<Self::Inner>, _: ()) {
        self.subscriber_list
            .notify(self.weak_self.clone(), Some(&self.event), runtime, || {
                let behavior_value = self.behavior.0.query_for_tag();
                let combined_value = Arc::new((value, behavior_value));
                Some(combined_value)
            });
    }
}

impl<A: 'static, B: 'static> EventImpl<(Arc<A>, Arc<B>)> for Tag<A, B> {
    fn subscribe(&self, cb: Weak<dyn super::EventCallback<(Arc<A>, Arc<B>)>>) {
        self.subscriber_list
            .add_subscriber(self.weak_self.clone(), Some(&self.event), cb);
    }

    fn height(&self) -> usize {
        let mut height = self.height.lock().unwrap();
        if let Some(height) = *height {
            return height;
        }
        let new_height = self.event.0.height() + 1;
        *height = Some(new_height);
        new_height
    }
}
