use std::sync::{Arc, Mutex, OnceLock, Weak};

use crate::event::subscriber_list::{SubscriptionEvent, SubscriptionManager};
use crate::event::{EventCallback, EventImpl};
use crate::runtime::{Action, Runtime};
use crate::widget::WidgetNode;

pub(super) struct WidgetReadyEvent {
    node: OnceLock<Arc<WidgetNode>>,
    trigger: Arc<WidgetReadyTrigger>,
    subscribers: SubscriptionManager<WidgetNode, Self>,
    weak_self: Weak<Self>,
}

pub(super) struct WidgetReadyTrigger {
    events: Mutex<Vec<Arc<WidgetReadyEvent>>>,
    height: usize,
}

impl WidgetReadyTrigger {
    pub(super) fn trigger(&self, runtime: &Runtime) {
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
        self.subscribers
            .notify(self.weak_self.clone(), None, runtime, || Some(value));
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

    pub(super) fn set_node(&self, node: Arc<WidgetNode>) {
        let Ok(_) = self.node.set(node) else {
            panic!("tried to set node for `WidgetReadyEvent` twice")
        };
    }
}

impl EventImpl<WidgetNode> for WidgetReadyEvent {
    fn subscribe(&self, cb: Weak<dyn EventCallback<WidgetNode>>) {
        self.subscribers
            .add_subscriber(self.weak_self.clone(), None, cb);
    }

    fn height(&self) -> usize {
        self.trigger.height
    }
}

impl SubscriptionEvent<WidgetNode> for WidgetReadyEvent {
    type Inner = ();
    type Tag = ();

    fn invalidate_height(&self) {
        unreachable!("WidgetReadyEvents have no outgoing subscription logic")
    }

    fn handle_main_subscription(
        &self,
        _: &crate::runtime::Runtime,
        _: Arc<Self::Inner>,
        _: Self::Tag,
    ) {
        unreachable!("WidgetReadyEvents have no outgoing subscription logic")
    }
}
