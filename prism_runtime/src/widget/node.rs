use std::any::Any;
use std::fmt::Debug;
use std::sync::{Arc, Mutex, OnceLock};

use crate::value::Value;
use serde_json::Value as JsonValue;

use crate::event::EventCallback;
use crate::value::AnyValue;
use crate::{
    runtime::Runtime,
    widget::{
        Slots,
        any::{AnyDynamic, AnyEvent, AnyEventTrigger},
        delegate::{WidgetDelegate, WidgetDelegateContext},
    },
};

/// A [`Widget`] is a built, live widget: what one bind of a
/// [`Blueprint`](super::Blueprint) produces. A tree of [`Widget`]s is the
/// state of a running app, and all ownership of FRP state in the app comes
/// from that tree. Inside the runtime a widget is also called a node, since
/// it is the node of that tree.
pub struct Widget {
    /// Tracks current children of the node.
    pub children: Mutex<Vec<Arc<Widget>>>,
    /// Exposed dynamics for backends to read and subscribe to.
    pub public_dynamics: Slots<AnyDynamic>,
    /// Exposed events for backends to subscribe to.
    pub public_events: Slots<AnyEvent>,
    /// Public event callbacks. These are just here to retain the relationship between
    /// the event and the delegate.
    pub _public_event_callbacks: OnceLock<Vec<Arc<DelegateCallback>>>,
    /// Cyclic events, rooted here in the widget.
    pub cyclic_events: Slots<Arc<dyn Any + Send + Sync>>,
    /// Cyclic dynamics, rooted here in the widget.
    pub cyclic_dynamics: Slots<Arc<dyn Any + Send + Sync>>,
    /// Exposed triggers for backends to provide data from the outside world
    pub triggers: Slots<AnyEventTrigger>,
    /// Backend data so the backend knows what (if anything) to do with this widget node.
    pub backend_data: JsonValue,
    /// The delegate is inserted by the backend and handles callbacks.
    /// It's kept alive here.
    pub delegate: OnceLock<Arc<dyn WidgetDelegate>>,
}

impl Widget {
    pub(crate) fn new() -> Widget {
        Widget {
            children: Mutex::new(vec![]),
            public_dynamics: Slots::default(),
            public_events: Slots::default(),
            _public_event_callbacks: OnceLock::new(),
            cyclic_events: Slots::default(),
            cyclic_dynamics: Slots::default(),
            triggers: Slots::default(),
            backend_data: JsonValue::Null,
            delegate: OnceLock::new(),
        }
    }

    pub(crate) fn set_delegate(&self, runtime: &Runtime, delegate: Arc<dyn WidgetDelegate>) {
        let Ok(_) = self.delegate.set(delegate.clone()) else {
            panic!("set delegate multiple times");
        };
        let cbs = {
            let mut cbs = vec![];
            for (name, &ix) in &self.public_events.names {
                let cb = Arc::new(DelegateCallback {
                    delegate: delegate.clone(),
                    name: name.into(),
                });
                self.public_events.values[ix]
                    .as_any_event()
                    .0
                    .subscribe(runtime, Arc::downgrade(
                        &(cb.clone() as Arc<dyn EventCallback<AnyValue>>),
                    ));
                cbs.push(cb);
            }
            cbs
        };
        let Ok(_) = self._public_event_callbacks.set(cbs) else {
            panic!("set public_event_callbacks multiple times");
        };
        let children = self.children.lock().unwrap().clone();
        let ctxt = WidgetDelegateContext {
            runtime,
            node: self,
        };

        for (ix, child) in children.into_iter().enumerate() {
            child.set_delegate(runtime, delegate.new_child_created(ctxt, ix, &child));
        }
    }

    pub(crate) fn prepare_destruction(&self, runtime: &Runtime) {
        let ctxt = WidgetDelegateContext {
            runtime,
            node: self,
        };
        if let Some(delegate) = self.delegate.get() {
            delegate.will_be_destroyed(ctxt);
        }
    }
}

impl Debug for Widget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Widget")
            .field("children", &self.children)
            .field("dynamics", &self.public_dynamics)
            .field("events", &self.public_events)
            .field("triggers", &self.triggers)
            .field("backend_data", &self.backend_data)
            .finish()
    }
}

#[allow(missing_docs)]
pub struct DelegateCallback {
    delegate: Arc<dyn WidgetDelegate>,
    name: String,
}

impl EventCallback<AnyValue> for DelegateCallback {
    fn event_fired(&self, _: &Runtime, value: Value<AnyValue>) {
        self.delegate.event_fired(&self.name, value.into());
    }

    fn already_fired(&self, _: &Runtime, value: Value<AnyValue>) {
        self.delegate.event_fired(&self.name, value.into());
    }

    fn invalidate_height(&self) {
        // We don't care.
    }
}
