use std::sync::Arc;

use crate::{behavior::Behavior, event::Event, runtime::Runtime, value::Value};

#[test]
fn a_behavior_that_never_changes_does_not_collect_dependents() {
    let mut runtime = Runtime::new();
    let (unchanging, _unchanging_trigger) = Event::<u32>::external();
    let (changing, changing_trigger) = Event::<u32>::external();
    let sum = Behavior::map2(
        |a: Value<u32>, b: Value<u32>| Value::from(*a + *b),
        Behavior::hold(0u32.into(), unchanging),
        Behavior::hold(0u32.into(), changing),
    );

    for value in 0..1000u32 {
        runtime.schedule_trigger(&changing_trigger, value.into());
        runtime.propagate();
        sum.0.query_for_tag();
    }

    assert!(Arc::weak_count(&sum.0) < 10);
}
