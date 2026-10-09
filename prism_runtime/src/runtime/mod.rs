//! A [`Runtime`] represents a graph of behaviors, events, and dynamics.
//!
//! Its key method is `propagate()`. During propagation of an *occurrence*, every event
//! is either fired or not, and if it's fired it has a value.
//! Each event may be fired at most once per occurrence.
//!
//! Then, behaviors are updated or invalidated to reflect any
//! changes that occurred as a result of those events.
//!
//! Prior to propagation, external actors can explicitly trigger
//! events. Those events don't actually do anything until propagation
//! begins.

use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    sync::Mutex,
};

#[cfg(test)]
mod tests;

// TODO: Should we have any guarantees that we don't mix runtimes?

thread_local! {
    pub(crate) static CURRENT_HEIGHT: Cell<Option<usize>> = const { Cell::new(None) };
    static DEFERRED: RefCell<Vec<(usize, Box<dyn Action>)>> = const { RefCell::new(Vec::new()) };
    pub(crate) static CLEAR_AT_END: RefCell<Vec<Box<dyn FnOnce()>>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn defer(height: usize, action: impl Action + 'static) {
    DEFERRED.with_borrow_mut(|deferred| deferred.push((height, Box::new(action))));
}

/// This represents an instance of a reactive graph.
pub struct Runtime(pub(crate) Mutex<RuntimeInner>);

pub(crate) struct RuntimeInner {
    actions: BTreeMap<usize, Vec<Box<dyn Action>>>,
    pub(crate) height: Option<usize>,
}

pub(crate) trait Action: Send {
    fn act(self: Box<Self>, runtime: &Runtime);
}

impl Action for Box<dyn Action> {
    fn act(self: Box<Self>, runtime: &Runtime) {
        (*self).act(runtime)
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    /// Construct a fresh runtime.
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

    /// Propagate scheduled events.
    pub fn propagate(&mut self) {
        let mut first_round = true;
        loop {
            let actions = {
                let mut this = self.0.lock().unwrap();

                if first_round && this.height.is_some() {
                    panic!("Can't initiate propagation while propagation is already happening");
                }
                first_round = false;

                let Some((height, actions)) = this.actions.pop_first() else {
                    // Done propagating
                    this.height = None;
                    drop(this);
                    CURRENT_HEIGHT.set(None);
                    for clear in CLEAR_AT_END.take() {
                        clear();
                    }
                    return;
                };
                if let Some(this_height) = this.height
                    && height <= this_height
                {
                    panic!(
                        "Runtime actions must always increase in height to prevent infinite regress"
                    );
                }
                this.height = Some(height);
                CURRENT_HEIGHT.set(Some(height));
                actions
            };
            for action in actions {
                let outcome = catch_unwind(AssertUnwindSafe(|| {
                    action.act(self);
                    for (height, deferred) in DEFERRED.take() {
                        self.schedule(height, deferred);
                    }
                }));
                if let Err(panic) = outcome {
                    let mut this = self.0.lock().unwrap();
                    this.actions.clear();
                    this.height = None;
                    drop(this);
                    DEFERRED.take();
                    CURRENT_HEIGHT.set(None);
                    for clear in CLEAR_AT_END.take() {
                        clear();
                    }
                    resume_unwind(panic);
                }
            }
        }
    }
}
