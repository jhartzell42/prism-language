use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::{dynamic::Dynamic, event::Event, runtime::Runtime};

#[test]
fn runtime_keeps_working_after_a_closure_panics() {
    let mut runtime = Runtime::new();
    let (event, trigger) = Event::<u32>::external();
    let held = Dynamic::hold(
        0u32.into(),
        event.map(|x| if *x == 1 { panic!("closure panicked") } else { x }),
    );

    runtime.schedule_trigger(&trigger, 1u32.into());
    let _ = catch_unwind(AssertUnwindSafe(|| runtime.propagate()));
    runtime.schedule_trigger(&trigger, 2u32.into());
    runtime.propagate();

    assert_eq!(*held.behavior().0.query_for_tag(), 2);
}
