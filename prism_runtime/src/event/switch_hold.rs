use std::sync::{Arc, Mutex, Weak};

use crate::{
    event::{
        Event, EventCallback, EventImpl,
        subscriber_list::{SubscriptionEvent, SubscriptionManager},
    },
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
                subscriber_list: SubscriptionManager::new(()),
                inner_event: Mutex::new(None),
                _outer_event: self.clone(),
                _outer_sub: outer_sub,
                weak_self: weak.clone(),
                height: Mutex::new(None),
            }
        }))
    }
}

struct SwitchHold<T: 'static> {
    subscriber_list: SubscriptionManager<T, Self>,
    inner_event: Mutex<Option<Event<T>>>,
    _outer_event: Event<Event<T>>,
    _outer_sub: Arc<SwitchHoldOuterCallback<T>>,
    weak_self: Weak<Self>,
    height: Mutex<Option<usize>>,
}

impl<T: 'static> EventImpl<T> for SwitchHold<T> {
    fn subscribe(&self, cb: Weak<dyn super::EventCallback<T>>) {
        let event = self.inner_event.lock().unwrap().clone();
        self.subscriber_list
            .add_subscriber(self.weak_self.clone(), event.as_ref(), cb);
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

impl<T: 'static> SubscriptionEvent<T> for SwitchHold<T> {
    type Inner = T;
    type Tag = ();

    fn invalidate_height(&self) {
        *self.height.lock().unwrap() = None;
    }

    fn handle_main_subscription(&self, runtime: &Runtime, value: Arc<Self::Inner>, _: ()) {
        let event = self.inner_event.lock().unwrap().clone();
        self.subscriber_list
            .notify(self.weak_self.clone(), event.as_ref(), runtime, || {
                Some(value)
            });
    }
}

struct SwitchHoldOuterCallback<T: 'static> {
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

struct SwitchHoldOuterAction<T: 'static> {
    this: Arc<SwitchHold<T>>,
    event: Event<T>,
}

impl<T: 'static> Action for SwitchHoldOuterAction<T> {
    fn act(self: Box<Self>, _: &Runtime) {
        let Self { this, event } = *self;
        *this.inner_event.lock().unwrap() = Some(event.clone());

        // Yeah, we have no clue what our height is anymore.
        //
        // Note that this actually happens **after** propagation
        // from "infinite" height when behaviors are also updated.
        *this.height.lock().unwrap() = None;
        // Let everyone else know what they now don't know.
        let cbs = this.subscriber_list.consolidate();

        if cbs.is_empty() {
            this.subscriber_list.clear_subscriber();
        } else {
            this.subscriber_list
                .populate_subscriber(this.weak_self.clone(), Some(&event), true);
        }

        for cb in cbs {
            cb.invalidate_height();
        }
    }
}
