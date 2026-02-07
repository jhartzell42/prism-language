use std::sync::{Arc, Mutex};

use crate::{
    behavior::Behavior,
    dynamic::Dynamic,
    event::{Event, EventCallback, tests::TestLogger},
    runtime::Runtime,
};

#[test]
fn hold_test() {
    let mut runtime = Runtime::new();
    let (event1, trigger1) = Event::<u32>::external();
    let (event2, trigger2) = Event::<()>::external();

    let hold_behavior = Behavior::hold(Arc::new(0), event1);
    let const_behavior = Behavior::constant(Arc::new(1));
    let derived_behavior = Behavior::map2(|a, b| Arc::new(*a + *b), hold_behavior, const_behavior);
    let tag_event = event2.tag(derived_behavior);
    let last_event = tag_event.filter_map(|x| Some(Arc::new(*x.1)));

    let last_event_logger = Arc::new(TestLogger::<u32> {
        log: Mutex::new(vec![]),
    });
    let last_event_logger2: Arc<dyn EventCallback<u32>> = last_event_logger.clone();
    last_event.0.subscribe(Arc::downgrade(&last_event_logger2));

    runtime.schedule_trigger(&trigger2, Arc::new(()));
    runtime.propagate(); // 0
    runtime.schedule_trigger(&trigger1, Arc::new(2));
    runtime.propagate();
    runtime.schedule_trigger(&trigger1, Arc::new(4));
    runtime.propagate();
    // behavior has 4
    runtime.schedule_trigger(&trigger2, Arc::new(()));
    runtime.propagate(); // 4
    runtime.schedule_trigger(&trigger2, Arc::new(()));
    runtime.propagate(); // 4
    runtime.schedule_trigger(&trigger1, Arc::new(3));
    runtime.schedule_trigger(&trigger2, Arc::new(()));
    runtime.propagate(); // still 4, not prompt
    runtime.schedule_trigger(&trigger2, Arc::new(()));
    runtime.propagate(); // 3

    let log = last_event_logger.log.lock().unwrap();
    let log = log.clone();

    // Everything's incremented by 1 b/c we added to the other behavior
    assert_eq!(
        log,
        vec![
            Arc::new(1),
            Arc::new(5),
            Arc::new(5),
            Arc::new(5),
            Arc::new(4)
        ]
    );
}

#[test]
fn dynamic_map2_test() {
    let mut runtime = Runtime::new();
    let (a_event, a_trigger) = Event::<u32>::external();
    let (b_event, b_trigger) = Event::<u32>::external();

    // Create dynamics with initial values
    let a_dyn = Dynamic::hold(Arc::new(1), a_event);
    let b_dyn = Dynamic::hold(Arc::new(10), b_event);

    // map2: sum them
    let sum_dyn = Dynamic::map2(a_dyn, b_dyn, |a, b| Arc::new(*a + *b));

    let logger = Arc::new(TestLogger::<u32> {
        log: Mutex::new(vec![]),
    });
    let logger2: Arc<dyn EventCallback<u32>> = logger.clone();
    sum_dyn.event().0.subscribe(Arc::downgrade(&logger2));

    // Fire A only: a=2, b=10 (from behavior) => 12
    runtime.schedule_trigger(&a_trigger, Arc::new(2));
    runtime.propagate();

    // Fire B only: a=2 (from behavior), b=20 => 22
    runtime.schedule_trigger(&b_trigger, Arc::new(20));
    runtime.propagate();

    // Fire both: a=3, b=30 => 33 (both from events, no stale behavior values)
    runtime.schedule_trigger(&a_trigger, Arc::new(3));
    runtime.schedule_trigger(&b_trigger, Arc::new(30));
    runtime.propagate();

    let log = logger.log.lock().unwrap();
    let log = log.clone();

    assert_eq!(log, vec![Arc::new(12), Arc::new(22), Arc::new(33)]);
}
