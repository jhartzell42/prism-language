use std::sync::{Arc, Mutex, Weak};

use crate::{
    behavior::Behavior,
    event::{
        Event, EventCallback, EventImpl,
        subscriber_list::{SubscriptionEvent, SubscriptionManager},
    },
    runtime::Runtime,
    value::{PrismArc, Value, ValueType},
};

impl<A: ValueType> Event<A> {
    /// This creates a new event that is fired whenever `self` is fired.
    /// It augments the value of `self` with the value of a [`Dynamic`] or
    /// [`Behavior`].
    ///
    /// It is **not** *prompt*, that is, it doesn't reflect updates to the
    /// tagged value that happened during this occurrence. Promptness is
    /// basically never what you want anyway.
    pub fn tag<B: ValueType>(
        &self,
        behavior: impl Into<Behavior<B>>,
    ) -> Event<Arc<(Value<A>, Value<B>)>> {
        self.tag_map(|a, b| Some(PrismArc::new((a, b)).into()), behavior.into())
    }

    /// [`Event::tag()`], but with a custom function to combine the event and the behavior,
    /// which can also control whether or not the event fires.
    ///
    /// Also not prompt.
    pub fn tag_map<B: ValueType, O: ValueType>(
        &self,
        function: impl Fn(Value<A>, Value<B>) -> Option<Value<O>> + 'static + Send + Sync,
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

struct Tag<
    A: ValueType,
    B: ValueType,
    O: ValueType,
    F: 'static + Fn(Value<A>, Value<B>) -> Option<Value<O>> + Send + Sync,
> {
    subscriber_list: SubscriptionManager<O, Self>,
    event: Event<A>,
    behavior: Behavior<B>,
    function: F,
    height: Mutex<Option<usize>>,
    weak_self: Weak<Self>,
}

impl<
    A: ValueType,
    B: ValueType,
    O: ValueType,
    F: 'static + Fn(Value<A>, Value<B>) -> Option<Value<O>> + Send + Sync,
> SubscriptionEvent<O> for Tag<A, B, O, F>
{
    type Inner = A;
    type Tag = ();

    fn invalidate_height(&self) {
        *self.height.lock().unwrap() = None;
    }

    fn handle_main_subscription(&self, runtime: &Runtime, value: Value<Self::Inner>, _: ()) {
        self.subscriber_list
            .notify(self.weak_self.clone(), Some(&self.event), runtime, || {
                let behavior_value = self.behavior.0.query_for_tag();
                (self.function)(value, behavior_value)
            });
    }
}

impl<
    A: ValueType,
    B: ValueType,
    O: ValueType,
    F: 'static + Fn(Value<A>, Value<B>) -> Option<Value<O>> + Send + Sync,
> EventImpl<O> for Tag<A, B, O, F>
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
