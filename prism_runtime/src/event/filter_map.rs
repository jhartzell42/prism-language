use std::sync::{Arc, Mutex, Weak};

use crate::{
    event::{Event, EventCallback, EventImpl, subscriber_list::SubscriberList},
    runtime::{Action, Runtime},
};

struct FilterMapEvent<O, T, F: Fn(Arc<T>) -> Option<Arc<O>>> {
    subscriber_list: SubscriberList<O>,
    inner: Event<T>,
    inner_sub: Mutex<Option<Arc<FilterMapCallback<O, T, F>>>>,
    weak_self: Weak<Self>,
    f: F,
    height: Mutex<Option<usize>>,
}

struct FilterMapCallback<O, T, F: Fn(Arc<T>) -> Option<Arc<O>>> {
    filter_map: Weak<FilterMapEvent<O, T, F>>,
}

struct FilterMapAction<O, T, F: Fn(Arc<T>) -> Option<Arc<O>>> {
    filter_map: Arc<FilterMapEvent<O, T, F>>,
    value: Arc<T>,
}

impl<O: 'static, T: 'static, F: 'static + Fn(Arc<T>) -> Option<Arc<O>>> EventCallback<T>
    for FilterMapCallback<O, T, F>
{
    fn event_fired(&self, runtime: &Runtime, value: Arc<T>) {
        let Some(this) = self.filter_map.upgrade() else {
            return;
        };
        let height = this.height();
        runtime.schedule(
            height,
            FilterMapAction {
                filter_map: this,
                value,
            },
        );
    }
}

impl<O, T, F: Fn(Arc<T>) -> Option<Arc<O>>> Action for FilterMapAction<O, T, F> {
    fn act(self: Box<Self>, runtime: &Runtime) {
        let this = self.filter_map;
        let value = self.value;
        let subs = this.subscriber_list.consolidate();

        if subs.is_empty() {
            let mut inner_sub = this.inner_sub.lock().unwrap();
            *inner_sub = None;
            return;
        }

        if let Some(value) = (this.f)(value) {
            for sub in subs {
                sub.event_fired(runtime, value.clone());
            }
        }
    }
}

impl<O: 'static, T: 'static, F: Fn(Arc<T>) -> Option<Arc<O>> + 'static> FilterMapEvent<O, T, F> {
    fn inner_subscription(&self) -> Arc<FilterMapCallback<O, T, F>> {
        let sub = Arc::new(FilterMapCallback {
            filter_map: self.weak_self.clone(),
        });
        let sub_dyn: Arc<dyn EventCallback<T>> = sub.clone();
        self.inner.0.subscribe(Arc::downgrade(&sub_dyn));
        sub
    }
}

impl<O: 'static, T: 'static, F: Fn(Arc<T>) -> Option<Arc<O>> + 'static> EventImpl<O>
    for FilterMapEvent<O, T, F>
{
    fn subscribe(&self, cb: std::sync::Weak<dyn super::EventCallback<O>>) {
        self.subscriber_list.add(cb);
        let mut inner_sub = self.inner_sub.lock().unwrap();
        if inner_sub.is_none() {
            *inner_sub = Some(self.inner_subscription());
        }
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

impl<T: 'static> Event<T> {
    pub fn filter_map<O: 'static>(
        &self,
        f: impl Fn(Arc<T>) -> Option<Arc<O>> + 'static,
    ) -> Event<O> {
        Event(Arc::new_cyclic(|weak| FilterMapEvent {
            subscriber_list: SubscriberList::new(),
            inner: self.clone(),
            inner_sub: Mutex::new(None),
            weak_self: weak.clone(),
            f,
            height: Mutex::new(None),
        }))
    }
}
