use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use serde_json::Value;

use crate::widget::erased::{ErasedDynamic, ErasedEvent, ErasedEventTrigger};

/// A [`WidgetNode`] is a widget in a form that it's already present
/// in an app. A tree of [`WidgetNode`]s represents the state of
/// a running app. All ownership of FRP state in the app comes from
/// a tree of widget nodes.
pub struct WidgetNode {
    /// Tracks current children of the node.
    pub(crate) children: Mutex<Vec<Arc<WidgetNode>>>,
    /// Exposed dynamics for backends to read and subscribe to.
    pub(crate) dynamics: Slots<ErasedDynamic>,
    /// Exposed events for backends to subscribe to.
    pub(crate) events: Slots<ErasedEvent>,
    /// Exposed triggers for backends to provide data from the outside world
    pub(crate) triggers: Slots<ErasedEventTrigger>,
    /// Backend data so the backend knows what (if anything) to do with this widget node.
    pub(crate) backend_data: Value,
}

impl WidgetNode {
    pub(crate) fn new() -> WidgetNode {
        WidgetNode {
            children: Mutex::new(vec![]),
            dynamics: Slots::default(),
            events: Slots::default(),
            triggers: Slots::default(),
            backend_data: Value::Null,
        }
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
    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

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
