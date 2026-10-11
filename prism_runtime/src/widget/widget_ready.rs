use std::sync::{Arc, Mutex, OnceLock, Weak};

use crate::event::subscriber_list::{SubscriptionEvent, SubscriptionManager};
use crate::event::{EventCallback, EventImpl};
use crate::runtime::{Action, Runtime};
use crate::value::Value;
use crate::widget::Widget;

pub(super) struct WidgetReadyEvent {
    pub(super) node: OnceLock<Arc<Widget>>,
    trigger: Arc<WidgetReadyTrigger>,
    subscribers: SubscriptionManager<Arc<Widget>, Self>,
    weak_self: Weak<Self>,
}

pub(super) struct WidgetReadyTrigger {
    events: Mutex<Vec<Arc<WidgetReadyEvent>>>,
    height: usize,
}

impl Action for Arc<WidgetReadyTrigger> {
    fn act(self: Box<Self>, runtime: &Runtime) {
        let events = {
            let mut events = self.events.lock().unwrap();
            std::mem::take(&mut *events)
        };
        for event in events {
            runtime.schedule(self.height, event);
        }
    }
}

impl Action for Arc<WidgetReadyEvent> {
    fn act(self: Box<Self>, runtime: &Runtime) {
        let value = self.node.get().unwrap().clone();
        self.subscribers.fire(runtime, value.into());
    }
}

impl WidgetReadyEvent {
    pub(super) fn new_with_trigger(height: usize) -> (Arc<Self>, Arc<WidgetReadyTrigger>) {
        let trigger = Arc::new(WidgetReadyTrigger {
            events: Mutex::new(vec![]),
            height,
        });
        (Self::new_from_trigger(trigger.clone()), trigger.clone())
    }

    pub(super) fn new(event: Arc<Self>) -> Arc<Self> {
        Self::new_from_trigger(event.trigger.clone())
    }

    pub(super) fn new_from_trigger(trigger: Arc<WidgetReadyTrigger>) -> Arc<Self> {
        let res = Arc::new_cyclic(|weak| Self {
            node: OnceLock::new(),
            trigger: trigger.clone(),
            subscribers: SubscriptionManager::new(()),
            weak_self: weak.clone(),
        });
        let mut events = trigger.events.lock().unwrap();
        events.push(res.clone());
        res
    }

    pub(super) fn set_node(&self, node: Arc<Widget>) {
        let Ok(_) = self.node.set(node) else {
            panic!("tried to set node for `WidgetReadyEvent` twice")
        };
    }
}

impl EventImpl<Arc<Widget>> for WidgetReadyEvent {
    fn subscribe(
        &self,
        runtime: &Runtime,
        cb: Weak<dyn EventCallback<Arc<Widget>>>,
    ) -> Option<Value<Arc<Widget>>> {
        self.subscribers
            .add_subscriber(runtime, self.weak_self.clone(), None, cb)
    }

    fn height(&self) -> usize {
        self.trigger.height
    }
}

impl SubscriptionEvent<Arc<Widget>> for WidgetReadyEvent {
    type Inner = ();
    type Tag = ();

    fn invalidate_height(&self) {
        unreachable!("WidgetReadyEvents have no outgoing subscription logic")
    }

    fn handle_main_subscription(
        &self,
        _: &crate::runtime::Runtime,
        _: Value<Self::Inner>,
        _: Self::Tag,
    ) {
        unreachable!("WidgetReadyEvents have no outgoing subscription logic")
    }

    fn handle_already_fired(&self, _: &Runtime, _: Value<Self::Inner>, _: Self::Tag) {
        unreachable!("WidgetReadyEvents have no outgoing subscription logic")
    }
}
