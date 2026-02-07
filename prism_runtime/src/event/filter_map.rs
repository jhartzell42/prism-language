use std::sync::{Arc, Mutex, Weak};

use crate::{
    event::{
        Event, EventImpl,
        subscriber_list::{SubscriptionEvent, SubscriptionManager},
    },
    runtime::Runtime,
};

impl<T: 'static + ?Sized + Send + Sync> Event<T> {
    /// Creates a new event based on the existing one and the function `f`.
    ///
    /// If the existing event is fired, run `f` on the value.
    ///
    /// If `f` returns `Some(value)`, the new event is fired with
    /// the value `value`. If `f` returns `None`, the new event is not fired.
    pub fn filter_map<O: 'static + ?Sized + Send + Sync>(
        &self,
        f: impl Send + Sync + Fn(Arc<T>) -> Option<Arc<O>> + 'static,
    ) -> Event<O> {
        Event(Arc::new_cyclic(|weak| FilterMapEvent {
            subscriber_list: SubscriptionManager::new(()),
            inner: self.clone(),
            weak_self: weak.clone(),
            f,
            height: Mutex::new(None),
        }))
    }
}

struct FilterMapEvent<
    O: 'static + ?Sized + Send + Sync,
    T: 'static + ?Sized + Send + Sync,
    F: 'static + Send + Sync + Fn(Arc<T>) -> Option<Arc<O>>,
> {
    subscriber_list: SubscriptionManager<O, Self>,
    inner: Event<T>,
    weak_self: Weak<Self>,
    f: F,
    height: Mutex<Option<usize>>,
}

impl<
    O: 'static + ?Sized + Send + Sync,
    T: 'static + ?Sized + Send + Sync,
    F: 'static + Fn(Arc<T>) -> Option<Arc<O>> + Send + Sync,
> SubscriptionEvent<O> for FilterMapEvent<O, T, F>
{
    type Inner = T;
    type Tag = ();

    fn invalidate_height(&self) {
        *self.height.lock().unwrap() = None;
    }

    fn handle_main_subscription(&self, runtime: &Runtime, value: Arc<Self::Inner>, _: ()) {
        self.subscriber_list
            .notify(self.weak_self.clone(), Some(&self.inner), runtime, || {
                (self.f)(value)
            });
    }
}
impl<
    O: 'static + ?Sized + Send + Sync,
    T: 'static + ?Sized + Send + Sync,
    F: Fn(Arc<T>) -> Option<Arc<O>> + 'static + Send + Sync,
> EventImpl<O> for FilterMapEvent<O, T, F>
{
    fn subscribe(&self, cb: std::sync::Weak<dyn super::EventCallback<O>>) {
        self.subscriber_list
            .add_subscriber(self.weak_self.clone(), Some(&self.inner), cb);
    }

    fn height(&self) -> usize {
        let mut height = self.height.lock().unwrap();
        if let Some(height) = *height {
            return height;
        }
        let new_height = self.inner.0.height() + 1;
        *height = Some(new_height);
        new_height
    }
}
