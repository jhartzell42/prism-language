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
    collections::BTreeMap,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    sync::Mutex,
};

#[cfg(test)]
mod tests;

// TODO: Should we have any guarantees that we don't mix runtimes?


/// This represents an instance of a reactive graph.
pub struct Runtime(pub(crate) Mutex<RuntimeInner>);

pub(crate) struct RuntimeInner {
    actions: BTreeMap<usize, Vec<Box<dyn Action>>>,
    pub(crate) height: Option<usize>,
    /// Per-occurrence memos to reset once propagation ends, whether it
    /// finishes or aborts.
    clear_at_end: Vec<Box<dyn FnOnce() + Send>>,
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
            clear_at_end: Vec::new(),
        }))
    }

    /// The height of the current occurrence, or `None` if we're not in an occurrence.
    pub(crate) fn current_height(&self) -> Option<usize> {
        self.0.lock().unwrap().height
    }

    /// Schedule a reset for the end of the current occurrence.
    pub(crate) fn clear_at_end(&self, clear: impl FnOnce() + Send + 'static) {
        self.0.lock().unwrap().clear_at_end.push(Box::new(clear));
    }

    fn finish_occurrence(&self) {
        let clears = {
            let mut this = self.0.lock().unwrap();
            this.height = None;
            std::mem::take(&mut this.clear_at_end)
        };
        for clear in clears {
            clear();
        }
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
                    drop(this);
                    self.finish_occurrence();
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
                actions
            };
            for action in actions {
                let outcome = catch_unwind(AssertUnwindSafe(|| action.act(self)));
                if let Err(panic) = outcome {
                    self.0.lock().unwrap().actions.clear();
                    self.finish_occurrence();
                    resume_unwind(panic);
                }
            }
        }
    }
}
