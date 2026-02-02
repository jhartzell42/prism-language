use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, OnceLock},
};

use serde_json::Value;

use crate::{
    runtime::Runtime,
    widget::{
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
        for child in children {
            child.set_delegate(
                runtime,
                self.delegate.get().unwrap().new_child_created(ctxt, &child),
            );
        }

        // XXX: Subscribe to all the events somehow
    }
}

pub(crate) struct Slots<T> {
    pub(crate) values: Vec<T>,
    pub(crate) names: BTreeMap<String, usize>,
}

impl<T> Default for Slots<T> {
    fn default() -> Self {
        Self {
            values: Default::default(),
            names: Default::default(),
        }
    }
}

impl<T: Clone> Slots<T> {
    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }

    pub(crate) fn add(&mut self, name: String, value: T) -> usize {
        if self.names.contains_key(&name) {
            panic!("Already have slot with name {name}");
        }
        let ix = self.values.len();
        self.values.push(value);
        self.names.insert(name, ix);
        ix
    }

    pub(crate) fn get_index(&self, ix: usize) -> T {
        self.values[ix].clone()
    }

    pub(crate) fn get_name(&self, name: &str) -> T {
        self.get_index(*self.names.get(name).expect("name not found"))
    }

    pub(crate) fn update_name(&mut self, name: &str, val: T) {
        self.update_index(*self.names.get(name).expect("name not found"), val);
    }

    pub(crate) fn update_index(&mut self, ix: usize, val: T) {
        self.values[ix] = val;
    }
}
