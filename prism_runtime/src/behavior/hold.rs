use std::sync::{Arc, Mutex, Weak};

use crate::{
    behavior::{Behavior, BehaviorDependent, BehaviorImpl},
    event::{Event, EventCallback},
    runtime::{Action, Runtime},
};

// TODO: I think Reflex requires a widget for this operation.
// Do we want to require that?

impl<T: 'static> Event<T> {
    pub fn hold(&self, initial: Arc<T>) -> Behavior<T> {
        let hold = Arc::new_cyclic(|weak| HoldBehavior {
            value: Mutex::new(initial),
            _event: self.clone(),
            weak_self: weak.clone(),
            dependents: Mutex::new(vec![]),
        });
        let hold2: Arc<dyn EventCallback<T>> = hold.clone();
        self.0.subscribe(Arc::downgrade(&hold2));

        Behavior(hold)
    }
}

struct HoldBehavior<T> {
    value: Mutex<Arc<T>>,
    _event: Event<T>, // We need to keep the event alive
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
        runtime.schedule(usize::MAX, HoldAction { hold: this, value })
    }
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
