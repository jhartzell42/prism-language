use std::sync::{Arc, Mutex, Weak};

use crate::{
    event::{Event, EventCallback, EventImpl, subscriber_list::SubscriberList},
    runtime::{Action, Runtime},
};

impl<T: 'static> Event<Event<T>> {
    /// Given an event of events, keep track of the most recent inner event
    /// to have shown up on the outer event. When that inner event triggers,
    /// trigger with that value. If the inner event and outer event trigger within
    /// the same occurrence, follow the old inner event until the next occurrence.
    pub fn switch_hold(&self) -> Event<T> {
        Event(Arc::new_cyclic(|weak| {
            let outer_sub = Arc::new(SwitchHoldOuterCallback { this: weak.clone() });
            let outer_sub2: Arc<dyn EventCallback<Event<T>>> = outer_sub.clone();
            self.0.subscribe(Arc::downgrade(&outer_sub2));
            SwitchHold::<T> {
                subscriber_list: SubscriberList::new(),
                inner_event: Mutex::new(None),
                inner_sub: Mutex::new(None),
                _outer_event: self.clone(),
                _outer_sub: outer_sub,
                weak_self: weak.clone(),
                height: Mutex::new(None),
            }
        }))
    }
}

struct SwitchHold<T> {
    subscriber_list: SubscriberList<T>,
    inner_event: Mutex<Option<Event<T>>>,
    inner_sub: Mutex<Option<Arc<SwitchHoldInnerCallback<T>>>>,
    _outer_event: Event<Event<T>>,
    _outer_sub: Arc<SwitchHoldOuterCallback<T>>,
    weak_self: Weak<Self>,
    height: Mutex<Option<usize>>,
}

impl<T: 'static> EventImpl<T> for SwitchHold<T> {
    fn subscribe(&self, cb: Weak<dyn super::EventCallback<T>>) {
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

        let new_height = self
            .inner_event
            .lock()
            .unwrap()
            .as_ref()
            .map(|e| e.0.height() + 1)
            .unwrap_or(0);
        *height = Some(new_height);
        new_height
    }
}

impl<T: 'static> SwitchHold<T> {
    fn inner_subscription(&self) -> Arc<SwitchHoldInnerCallback<T>> {
        let sub = Arc::new(SwitchHoldInnerCallback {
            this: self.weak_self.clone(),
        });
        let Some(inner_event) = self.inner_event.lock().unwrap().clone() else {
            return sub;
        };
        let sub2: Arc<dyn EventCallback<T>> = sub.clone();
        inner_event.0.subscribe(Arc::downgrade(&sub2));
        sub
    }
}

struct SwitchHoldInnerCallback<T> {
    this: Weak<SwitchHold<T>>,
}

impl<T: 'static> EventCallback<T> for SwitchHoldInnerCallback<T> {
    fn event_fired(&self, runtime: &Runtime, value: Arc<T>) {
        let Some(this) = self.this.upgrade() else {
            return;
        };
        runtime.schedule(this.height(), SwitchHoldInnerAction { this, value });
    }

    fn invalidate_height(&self) {
        let Some(this) = self.this.upgrade() else {
            return;
        };
        *this.height.lock().unwrap() = None;
    }
}

struct SwitchHoldInnerAction<T> {
    this: Arc<SwitchHold<T>>,
    value: Arc<T>,
}

impl<T> Action for SwitchHoldInnerAction<T> {
    fn act(self: Box<Self>, runtime: &Runtime) {
        let Self { this, value } = *self;
        if !this.subscriber_list.notify(runtime, value) {
            *this.inner_sub.lock().unwrap() = None;
        }
    }
}

struct SwitchHoldOuterCallback<T> {
    this: Weak<SwitchHold<T>>,
}

impl<T: 'static> EventCallback<Event<T>> for SwitchHoldOuterCallback<T> {
    fn event_fired(&self, runtime: &Runtime, value: Arc<Event<T>>) {
        let Some(this) = self.this.upgrade() else {
            return;
        };
        let event = (*value).clone();
        // Not "prompt," never "prompt." Doesn't take effect until after this occurrence.
        runtime.schedule(usize::MAX, SwitchHoldOuterAction { this, event });
    }

    fn invalidate_height(&self) {
        let Some(this) = self.this.upgrade() else {
            return;
        };
        *this.height.lock().unwrap() = None;
    }
}

struct SwitchHoldOuterAction<T> {
    this: Arc<SwitchHold<T>>,
    event: Event<T>,
}

impl<T: 'static> Action for SwitchHoldOuterAction<T> {
    fn act(self: Box<Self>, _: &Runtime) {
        let Self { this, event } = *self;
        *this.inner_event.lock().unwrap() = Some(event);

        // Yeah, we have no clue what our height is anymore.
        //
        // Note that this actually happens **after** propagation
        // from "infinite" height when behaviors are also updated.
        *this.height.lock().unwrap() = None;
        // Let everyone else know what they now don't know.
        let cbs = this.subscriber_list.consolidate();
        let dont_subscribe = cbs.is_empty();
        for cb in cbs {
            cb.invalidate_height();
        }

        let mut sub = this.inner_sub.lock().unwrap();
        if dont_subscribe {
            *sub = None;
        } else {
            *sub = Some(this.inner_subscription());
        }
    }
}
