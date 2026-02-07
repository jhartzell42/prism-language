use std::sync::{Arc, Mutex, Weak};

use crate::{
    event::{
        Event, EventCallback, EventImpl,
        subscriber_list::{SubscriptionEvent, SubscriptionManager},
    },
    runtime::Runtime,
};

/// Represents one or both of two values, used by [`Event::combine`].
pub enum OneOrBoth<A, B> {
    /// Only the first event fired.
    A(Arc<A>),
    /// Only the second event fired.
    B(Arc<B>),
    /// Both events fired simultaneously.
    Both(Arc<A>, Arc<B>),
}

impl<A, B> Clone for OneOrBoth<A, B> {
    fn clone(&self) -> Self {
        match self {
            Self::A(a) => Self::A(a.clone()),
            Self::B(b) => Self::B(b.clone()),
            Self::Both(a, b) => Self::Both(a.clone(), b.clone()),
        }
    }
}

impl<T: 'static> Event<T> {
    /// Creates a new event based on the existing one and the function `f`.
    ///
    /// If the existing event is fired, run `f` on the value.
    ///
    /// If `f` returns `Some(value)`, the new event is fired with
    /// the value `value`. If `f` returns `None`, the new event is not fired.
    pub fn combine<A: 'static, B: 'static>(
        function: impl Fn(OneOrBoth<A, B>) -> Option<Arc<T>> + 'static,
        a: Event<A>,
        b: Event<B>,
    ) -> Event<T> {
        Event(Arc::new_cyclic(|weak| CombineEvent {
            a: a.filter_map(|a| Some(Arc::new(OneOrBoth::A(a)))),
            b: b.filter_map(|b| Some(Arc::new(OneOrBoth::B(b)))),
            manager_a: SubscriptionManager::new(()),
            manager_b: SubscriptionManager::new(()),
            function,
            weak_self: weak.clone(),
            height: Mutex::new(None),
            one_or_both: Mutex::new(None),
        }))
    }
}

struct CombineEvent<
    A: 'static,
    B: 'static,
    F: 'static + Fn(OneOrBoth<A, B>) -> Option<Arc<O>>,
    O: 'static,
> {
    a: Event<OneOrBoth<A, B>>,
    b: Event<OneOrBoth<A, B>>,
    manager_a: SubscriptionManager<O, Self>,
    manager_b: SubscriptionManager<O, Self>,
    function: F,
    weak_self: Weak<Self>,
    height: Mutex<Option<usize>>,
    one_or_both: Mutex<Option<OneOrBoth<A, B>>>,
}

impl<A: 'static, B: 'static, F: 'static + Fn(OneOrBoth<A, B>) -> Option<Arc<O>>, O: 'static>
    SubscriptionEvent<O> for CombineEvent<A, B, F, O>
{
    type Inner = OneOrBoth<A, B>;
    type Tag = ();

    fn invalidate_height(&self) {
        *self.height.lock().unwrap() = None;
    }

    fn handle_main_subscription(&self, runtime: &Runtime, _: Arc<Self::Inner>, _: ()) {
        let Some(one_or_both) = self.one_or_both.lock().unwrap().take() else {
            return;
        };
        self.manager_a
            .notify(self.weak_self.clone(), Some(&self.a), runtime, || {
                (self.function)(one_or_both)
            });
        self.manager_b.consolidate();
    }

    fn handle_early_subscription(&self, _: &Runtime, value: Arc<Self::Inner>, _: ()) {
        let mut acc = self.one_or_both.lock().unwrap();
        let old = acc.take();
        *acc = match ((*value).clone(), old) {
            (OneOrBoth::A(a), Some(OneOrBoth::B(b))) => Some(OneOrBoth::Both(a, b)),
            (OneOrBoth::B(b), Some(OneOrBoth::A(a))) => Some(OneOrBoth::Both(a, b)),
            (value, None) => Some(value),
            (_, old) => old.clone(),
        };
    }
}

impl<A: 'static, B: 'static, F: Fn(OneOrBoth<A, B>) -> Option<Arc<O>>, O: 'static> EventImpl<O>
    for CombineEvent<A, B, F, O>
{
    fn subscribe(&self, cb: Weak<dyn EventCallback<O>>) {
        self.manager_a
            .add_subscriber(self.weak_self.clone(), Some(&self.a), cb.clone());
        self.manager_b
            .add_subscriber(self.weak_self.clone(), Some(&self.b), cb.clone());
    }

    fn height(&self) -> usize {
        let height = std::cmp::max(self.a.0.height(), self.b.0.height()) + 1;
        *self.height.lock().unwrap() = Some(height);
        height
    }
}
