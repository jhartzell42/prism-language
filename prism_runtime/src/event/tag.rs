use std::sync::{Arc, Mutex, Weak};

use crate::{
    behavior::Behavior,
    event::{
        Event, EventCallback, EventImpl,
        subscriber_list::{SubscriptionEvent, SubscriptionManager},
    },
    runtime::Runtime,
};

impl<A: 'static> Event<A> {
    /// This creates a new event that is fired whenever `self` is fired.
    /// It augments the value of `self` with the value of a [`Dynamic`] or
    /// [`Behavior`].
    ///
    /// It is **not** *prompt*, that is, it doesn't reflect updates to the
    /// tagged value that happened during this occurrence. Promptness is
    /// basically never what you want anyway.
    pub fn tag<B: 'static>(&self, behavior: impl Into<Behavior<B>>) -> Event<(Arc<A>, Arc<B>)> {
        self.tag_map(|a, b| Some(Arc::new((a, b))), behavior.into())
    }

    /// [`Event::tag()`], but with a custom function to combine the event and the behavior,
    /// which can also control whether or not the event fires.
    ///
    /// Also not prompt.
    pub fn tag_map<B: 'static, O: 'static>(
        &self,
        function: impl Fn(Arc<A>, Arc<B>) -> Option<Arc<O>> + 'static,
        behavior: impl Into<Behavior<B>>,
    ) -> Event<O> {
        Event(Arc::new_cyclic(|weak| Tag {
            event: self.clone(),
            behavior: behavior.into(),
            subscriber_list: SubscriptionManager::new(()),
            function,
            height: Mutex::new(None),
            weak_self: weak.clone(),
        }))
    }
}

struct Tag<A: 'static, B: 'static, O: 'static, F: 'static + Fn(Arc<A>, Arc<B>) -> Option<Arc<O>>> {
    subscriber_list: SubscriptionManager<O, Self>,
    event: Event<A>,
    behavior: Behavior<B>,
    function: F,
    height: Mutex<Option<usize>>,
    weak_self: Weak<Self>,
}

impl<A: 'static, B, O: 'static, F: Fn(Arc<A>, Arc<B>) -> Option<Arc<O>>> SubscriptionEvent<O>
    for Tag<A, B, O, F>
{
    type Inner = A;
    type Tag = ();

    fn invalidate_height(&self) {
        *self.height.lock().unwrap() = None;
    }

    fn handle_main_subscription(&self, runtime: &Runtime, value: Arc<Self::Inner>, _: ()) {
        self.subscriber_list
            .notify(self.weak_self.clone(), Some(&self.event), runtime, || {
                let behavior_value = self.behavior.0.query_for_tag();
                (self.function)(value, behavior_value)
            });
    }
}

impl<A: 'static, B, O: 'static, F: Fn(Arc<A>, Arc<B>) -> Option<Arc<O>>> EventImpl<O>
    for Tag<A, B, O, F>
{
    fn subscribe(&self, cb: Weak<dyn EventCallback<O>>) {
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
