//! A [`Runtime`] represents a graph of behaviors, events, and dynamics.
//!
//! Its key method is `propagate()`. During propagation, every event
//! is either triggered or not, and if it's triggered it has a value.
//! Then, behaviors are updated or invalidated to reflect any
//! changes that occurred as a result of those events.
//!
//! Prior to propagation, external actors can explicitly trigger
//! events. Those events don't actually do anything until propagation
//! begins.

use std::{collections::BTreeMap, sync::Mutex};

pub struct Runtime(Mutex<RuntimeInner>);

struct RuntimeInner {
    actions: BTreeMap<usize, Vec<Box<dyn Action>>>,
    height: Option<usize>,
}

pub(crate) trait Action {
    fn act(self: Box<Self>, runtime: &Runtime);
}

impl Runtime {
    pub fn new() -> Self {
        Runtime(Mutex::new(RuntimeInner {
            actions: Default::default(),
            height: None,
        }))
    }

    pub(crate) fn schedule(&self, height: usize, action: impl Action + 'static) {
        let mut this = self.0.lock().unwrap();
        if let Some(current_height) = this.height
            && height <= current_height
        {
            panic!("Scheduling action at height={height} when already at height={current_height}");
        }
        this.actions
            .entry(height)
            .or_default()
            .push(Box::new(action));
    }

    pub fn propagate(&mut self) {
        {
            let mut this = self.0.lock().unwrap();
            if this.height.is_some() {
                panic!("Can't initiate propagation while propagation is already happening");
            }
            let Some((&height, _)) = this.actions.first_key_value() else {
                return;
            };
            this.height = Some(height);
        }
        loop {
            let actions = {
                let mut this = self.0.lock().unwrap();
                let Some((height, actions)) = this.actions.pop_first() else {
                    this.height = None;
                    return;
                };
                this.height = Some(height);
                actions
            };
            for action in actions {
                action.act(self);
            }
        }
    }
}
