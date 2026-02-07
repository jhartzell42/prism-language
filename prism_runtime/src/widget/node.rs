use std::fmt::Debug;
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::Value;

use crate::{
    runtime::Runtime,
    widget::{
        Slots,
        delegate::{WidgetDelegate, WidgetDelegateContext},
        erased::{ErasedDynamic, ErasedEvent, ErasedEventTrigger},
    },
};

/// A [`WidgetNode`] is a widget in a form that it's already present
/// in an app. A tree of [`WidgetNode`]s represents the state of
/// a running app. All ownership of FRP state in the app comes from
/// a tree of widget nodes.
pub struct WidgetNode {
    /// Tracks current children of the node.
    pub children: Mutex<Vec<Arc<WidgetNode>>>,
    /// Exposed dynamics for backends to read and subscribe to.
    pub dynamics: Slots<ErasedDynamic>,
    /// Exposed events for backends to subscribe to.
    pub events: Slots<ErasedEvent>,
    /// Exposed triggers for backends to provide data from the outside world
    pub triggers: Slots<ErasedEventTrigger>,
    /// Backend data so the backend knows what (if anything) to do with this widget node.
    pub backend_data: Value,
    /// The delegate is inserted by the backend and handles callbacks.
    /// It's kept alive here.
    pub delegate: OnceLock<Arc<dyn WidgetDelegate>>,
}

impl WidgetNode {
    pub(crate) fn new() -> WidgetNode {
        WidgetNode {
            children: Mutex::new(vec![]),
            dynamics: Slots::default(),
            events: Slots::default(),
            triggers: Slots::default(),
            backend_data: Value::Null,
            delegate: OnceLock::new(),
        }
    }

    pub(crate) fn set_delegate(&self, runtime: &Runtime, delegate: Arc<dyn WidgetDelegate>) {
        let Ok(_) = self.delegate.set(delegate) else {
            panic!("set delegate multiple times");
        };
        let children = self.children.lock().unwrap().clone();
        let ctxt = WidgetDelegateContext {
            runtime,
            node: self,
        };

        for (ix, child) in children.into_iter().enumerate() {
            child.set_delegate(
                runtime,
                self.delegate
                    .get()
                    .unwrap()
                    .new_child_created(ctxt, ix, &child),
            );
        }
    }

    pub(crate) fn prepare_destruction(&self, runtime: &Runtime) {
        let ctxt = WidgetDelegateContext {
            runtime,
            node: self,
        };
        self.delegate.get().unwrap().will_be_destroyed(ctxt);
    }
}

impl Debug for WidgetNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetNode")
            .field("children", &self.children)
            .field("dynamics", &self.dynamics)
            .field("events", &self.events)
            .field("triggers", &self.triggers)
            .field("backend_data", &self.backend_data)
            .finish()
    }
}
