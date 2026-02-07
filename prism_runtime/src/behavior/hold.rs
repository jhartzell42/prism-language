use std::sync::{Arc, Mutex, Weak};

use crate::{
    behavior::{Behavior, BehaviorDependent, BehaviorImpl},
    event::{Event, EventCallback},
    runtime::{Action, Runtime},
};

// TODO: I think Reflex requires a widget context for this operation.
// Do we want to require that? Why does it require that? How's it doing that sort
// of thing anyway?

impl<T: 'static> Behavior<T> {
    /// Creates a behavior that has the value from this event
    /// from the last occurrence in which the event was fired.
    /// If it has never fired, it will have the `initial` value.
    pub fn hold(initial: Arc<T>, event: Event<T>) -> Self {
        let hold = Arc::new_cyclic(|weak| HoldBehavior {
            value: Mutex::new(initial),
            _event: event.clone(),
            weak_self: weak.clone(),
            dependents: Mutex::new(vec![]),
        });
        let hold2: Arc<dyn EventCallback<T>> = hold.clone();
        event.0.subscribe(Arc::downgrade(&hold2));

        Behavior(hold)
    }
}

struct HoldBehavior<T> {
    value: Mutex<Arc<T>>, // Not a cache, we have no way of recomputing it in case of problem
    _event: Event<T>,     // We need to keep the event alive
    weak_self: Weak<Self>,
    dependents: Mutex<Vec<Weak<dyn BehaviorDependent>>>,
}

impl<T> BehaviorImpl<T> for HoldBehavior<T> {
    fn query_for_behavior(&self, dep: Weak<dyn BehaviorDependent>) -> Arc<T> {
        let mut deps = self.dependents.lock().unwrap();
        deps.push(dep);
        self.value.lock().unwrap().clone()
    }

    fn query_for_tag(&self) -> Arc<T> {
        self.value.lock().unwrap().clone()
    }
}

impl<T: 'static> EventCallback<T> for HoldBehavior<T> {
    fn event_fired(&self, runtime: &Runtime, value: Arc<T>) {
        let Some(this) = self.weak_self.upgrade() else {
            return;
        };
        // Behavior updates happen after all event propagation.
        runtime.schedule(usize::MAX, HoldAction { hold: this, value })
    }

    // Heights of a behavior are always `usize::MAX`.
    fn invalidate_height(&self) {}
}

struct HoldAction<T> {
    hold: Arc<HoldBehavior<T>>,
    value: Arc<T>,
}

impl<T> Action for HoldAction<T> {
    fn act(self: Box<Self>, _: &Runtime) {
        let this = self.hold;
        let value = self.value;
        *this.value.lock().unwrap() = value;
        let deps = std::mem::take(&mut *this.dependents.lock().unwrap());
        for dep in deps {
            if let Some(dep) = dep.upgrade() {
                dep.invalidate();
            }
        }
    }
}
