use std::sync::{Arc, Mutex, Weak};

use crate::{
    event::{Event, EventCallback, EventImpl, subscriber_list::SubscriberList},
    runtime::{Action, Runtime},
};

pub struct EventTrigger<T>(Weak<EventTriggerInner<T>>);

impl<T> Clone for EventTrigger<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

struct EventTriggerInner<T> {
    subscribers: SubscriberList<T>,
    scheduled: Mutex<bool>,
}

impl Runtime {
    /// Schedule an event to trigger in the next propagation stage of the runtime.
    pub fn schedule_trigger<T: 'static>(&self, trigger: &EventTrigger<T>, value: Arc<T>) {
        struct TriggerAction<T> {
            trigger: EventTrigger<T>,
            value: Arc<T>,
        }

        impl<T> Action for TriggerAction<T> {
            fn act(self: Box<Self>, runtime: &Runtime) {
                let Some(trigger) = self.trigger.0.upgrade() else {
                    return;
                };
                let mut scheduled = trigger.scheduled.lock().unwrap();
                *scheduled = false;
                trigger.subscribers.notify(runtime, self.value);
            }
        }

        {
            let Some(trigger) = trigger.0.upgrade() else {
                return;
            };

            let mut scheduled = trigger.scheduled.lock().unwrap();
            if *scheduled {
                panic!("can't schedule the same event twice before propagating");
            }
            *scheduled = true;
        }

        self.schedule(
            0,
            TriggerAction {
                trigger: trigger.clone(),
                value,
            },
        );
    }
}

impl<T> EventImpl<T> for EventTriggerInner<T> {
    fn subscribe(&self, cb: Weak<dyn EventCallback<T>>) {
        self.subscribers.add(cb);
    }

    fn height(&self) -> usize {
        0
    }
}

impl<T: 'static> Event<T> {
    /// Return a tuple of an event that represents an external event,
    /// and a way to trigger it in a given runtime.
    pub fn external() -> (Event<T>, EventTrigger<T>) {
        let trigger = Arc::new(EventTriggerInner {
            subscribers: SubscriberList::new(),
            scheduled: Mutex::new(false),
        });
        (
            Event(trigger.clone()),
            EventTrigger(Arc::downgrade(&trigger)),
        )
    }
}
