use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use crate::{
    dynamic::Dynamic,
    event::{Event, EventCallback},
    runtime::Runtime,
    value::Value,
};

#[test]
fn runtime_keeps_working_after_a_closure_panics() {
    let mut runtime = Runtime::new();
    let (event, trigger) = Event::<u32>::external();
    let held = Dynamic::hold(
        0u32.into(),
        event.map(|x| if *x == 1 { panic!("closure panicked") } else { x }),
    );

    runtime.schedule_trigger(&trigger, 1u32.into()).unwrap();
    let _ = catch_unwind(AssertUnwindSafe(|| runtime.propagate()));
    runtime.schedule_trigger(&trigger, 2u32.into()).unwrap();
    runtime.propagate();

    assert_eq!(*held.behavior().0.query_for_tag(), 2);
}

struct PanicOnFire;

impl EventCallback<u32> for PanicOnFire {
    fn event_fired(&self, _: &Runtime, _: Value<u32>) {
        panic!("subscriber panicked")
    }

    fn invalidate_height(&self) {}
}

#[test]
fn a_trigger_dropped_by_a_panic_accepts_values_again() {
    let mut runtime = Runtime::new();
    let (panicking, panicking_trigger) = Event::<u32>::external();
    let (other, other_trigger) = Event::<u32>::external();
    let held_other = Dynamic::hold(0u32.into(), other);
    let callback: Arc<dyn EventCallback<u32>> = Arc::new(PanicOnFire);
    panicking.0.subscribe(Arc::downgrade(&callback));

    runtime.schedule_trigger(&panicking_trigger, 1u32.into()).unwrap();
    runtime.schedule_trigger(&other_trigger, 1u32.into()).unwrap();
    let _ = catch_unwind(AssertUnwindSafe(|| runtime.propagate()));
    runtime.schedule_trigger(&other_trigger, 2u32.into()).unwrap();
    runtime.propagate();

    assert_eq!(*held_other.behavior().0.query_for_tag(), 2);
}
