use std::sync::{Arc, Mutex, Weak};

use crate::{
    event::{
        Event, EventCallback, EventImpl,
        subscriber_list::{SubscriptionEvent, SubscriptionManager},
    },
    runtime::{Action, Runtime},
    value::{Value, ValueType},
};

/// This is a handle used to schedule the triggering of an event.
pub struct EventTrigger<T: ValueType>(Weak<EventTriggerInner<T>>);

impl<T: ValueType> Event<T> {
    /// Return a tuple of an event that represents an external event,
    /// and a way to trigger it in a given runtime.
    pub fn external() -> (Event<T>, EventTrigger<T>) {
        let trigger = Arc::new_cyclic(|weak| EventTriggerInner {
            subscribers: SubscriptionManager::new(()),
            scheduled: Mutex::new(false),
            weak_self: weak.clone(),
        });
        (
            Event(trigger.clone()),
            EventTrigger(Arc::downgrade(&trigger)),
        )
    }
}

impl Runtime {
    /// Schedule an event to trigger in the next propagation stage of the runtime.
    pub fn schedule_trigger<T: ValueType>(&self, trigger: &EventTrigger<T>, value: Value<T>) {
        struct TriggerAction<T: ValueType> {
            trigger: EventTrigger<T>,
            value: Value<T>,
        }

        impl<T: ValueType> Action for TriggerAction<T> {
            fn act(self: Box<Self>, runtime: &Runtime) {
                let Some(trigger) = self.trigger.0.upgrade() else {
                    return;
                };
                let mut scheduled = trigger.scheduled.lock().unwrap();
                *scheduled = false;
                trigger
                    .subscribers
                    .notify(self.trigger.0.clone(), None, runtime, || Some(self.value));
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

struct EventTriggerInner<T: ValueType> {
    subscribers: SubscriptionManager<T, Self>,
    scheduled: Mutex<bool>,
    weak_self: Weak<Self>,
}

impl<T: ValueType> EventImpl<T> for EventTriggerInner<T> {
    fn subscribe(&self, cb: Weak<dyn EventCallback<T>>) {
        self.subscribers
            .add_subscriber(self.weak_self.clone(), None, cb);
    }

    fn height(&self) -> usize {
        0
    }
}

impl<T: ValueType> Clone for EventTrigger<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: ValueType> SubscriptionEvent<T> for EventTriggerInner<T> {
    // Does Rust choke on "never" types?
    type Inner = ();
    type Tag = ();

    fn invalidate_height(&self) {
        unreachable!("external events have no outgoing subscription edge")
    }

    fn handle_main_subscription(&self, _: &Runtime, _: Value<Self::Inner>, _: ()) {
        unreachable!("external events have no outgoing subscription edge")
    }
}
