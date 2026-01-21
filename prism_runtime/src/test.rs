use std::sync::{Arc, Mutex};

use crate::{
    behavior::Behavior,
    event::{Event, EventCallback},
    runtime::Runtime,
};

struct TestLogger<T> {
    log: Mutex<Vec<Arc<T>>>,
}

impl<T> EventCallback<T> for TestLogger<T> {
    fn event_fired(&self, _: &Runtime, value: std::sync::Arc<T>) {
        let mut log = self.log.lock().unwrap();
        log.push(value);
    }

    fn invalidate_height(&self) {}
}

#[test]
fn event_filter_test() {
    let mut runtime = Runtime::new();
    let (event, trigger) = Event::<u32>::external();
    let filter_zero_event = event.filter_map(|a| {
        if *a == 0 {
            None
        } else {
            Some(Arc::new(*a - 1))
        }
    });

    let filter_zero_logger = Arc::new(TestLogger::<u32> {
        log: Mutex::new(vec![]),
    });
    let filter_zero_logger_2: Arc<dyn EventCallback<u32>> = filter_zero_logger.clone();
    filter_zero_event
        .0
        .subscribe(Arc::downgrade(&filter_zero_logger_2));

    runtime.schedule_trigger(&trigger, Arc::new(2));
    runtime.propagate();
    runtime.schedule_trigger(&trigger, Arc::new(0));
    runtime.propagate();
    runtime.schedule_trigger(&trigger, Arc::new(2));
    runtime.propagate();
    let log = filter_zero_logger.log.lock().unwrap();
    let log = log.clone();
    assert_eq!(log, vec![Arc::new(1), Arc::new(1)]);
}

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
fn switch_hold_test() {
    let mut runtime = Runtime::new();
    let (outer_event, outer_trigger) = Event::<Event<u32>>::external();
    let last_event = outer_event.switch_hold();

    let last_event_logger = Arc::new(TestLogger::<u32> {
        log: Mutex::new(vec![]),
    });
    let last_event_logger2: Arc<dyn EventCallback<u32>> = last_event_logger.clone();
    last_event.0.subscribe(Arc::downgrade(&last_event_logger2));

    let (inner_event, inner_trigger) = Event::<u32>::external();
    let inner_event_plus_1 = inner_event.filter_map(|x| Some(Arc::new(*x + 1)));
    let inner_event_plus_2 = inner_event_plus_1.filter_map(|x| Some(Arc::new(*x + 1)));
    let inner_event_plus_3 = inner_event_plus_2.filter_map(|x| Some(Arc::new(*x + 1)));
    let inner_event_plus_4 = inner_event_plus_3.filter_map(|x| Some(Arc::new(*x + 1)));
    let inner_event_plus_5 = inner_event_plus_4.filter_map(|x| Some(Arc::new(*x + 1)));

    runtime.schedule_trigger(&inner_trigger, Arc::new(0));
    runtime.propagate();
    runtime.schedule_trigger(&outer_trigger, Arc::new(inner_event.clone()));
    runtime.schedule_trigger(&inner_trigger, Arc::new(0));
    runtime.propagate();
    runtime.schedule_trigger(&inner_trigger, Arc::new(0));
    runtime.propagate(); // Now 0
    runtime.schedule_trigger(&outer_trigger, Arc::new(inner_event_plus_5.clone()));
    runtime.schedule_trigger(&inner_trigger, Arc::new(0));
    runtime.propagate(); // Also 0
    runtime.schedule_trigger(&inner_trigger, Arc::new(0));
    runtime.propagate(); // Now it's 5
    runtime.schedule_trigger(&outer_trigger, Arc::new(inner_event_plus_3.clone()));
    runtime.schedule_trigger(&inner_trigger, Arc::new(0));
    runtime.propagate(); // Also 5
    runtime.schedule_trigger(&inner_trigger, Arc::new(0));
    runtime.propagate(); // Now it's 3

    let log = last_event_logger.log.lock().unwrap();
    let log = log.clone();

    assert_eq!(
        log,
        vec![
            Arc::new(0),
            Arc::new(0),
            Arc::new(5),
            Arc::new(5),
            Arc::new(3)
        ]
    );
}

#[test]
fn leftmost_test() {
    let mut runtime = Runtime::new();
    let (left_event, left_trigger) = Event::<u32>::external();
    let (right_event, right_trigger) = Event::<u32>::external();
    let last_event = Event::leftmost(vec![left_event, right_event]);

    let last_event_logger = Arc::new(TestLogger::<u32> {
        log: Mutex::new(vec![]),
    });
    let last_event_logger2: Arc<dyn EventCallback<u32>> = last_event_logger.clone();
    last_event.0.subscribe(Arc::downgrade(&last_event_logger2));

    runtime.propagate(); // Nothing
    runtime.schedule_trigger(&left_trigger, Arc::new(1));
    runtime.propagate(); // 1
    runtime.schedule_trigger(&right_trigger, Arc::new(2));
    runtime.propagate(); // 2
    runtime.schedule_trigger(&right_trigger, Arc::new(2));
    runtime.schedule_trigger(&left_trigger, Arc::new(1));
    runtime.propagate(); // 1

    let log = last_event_logger.log.lock().unwrap();
    let log = log.clone();

    assert_eq!(log, vec![Arc::new(1), Arc::new(2), Arc::new(1),]);
}

#[test]
fn combine_test() {
    use crate::event::OneOrBoth;

    let mut runtime = Runtime::new();
    let (a_event, a_trigger) = Event::<u32>::external();
    let (b_event, b_trigger) = Event::<String>::external();

    // Combine into a string describing what fired
    let combined = Event::combine(
        |oob| match oob {
            OneOrBoth::A(a) => Some(Arc::new(format!("A({})", a))),
            OneOrBoth::B(b) => Some(Arc::new(format!("B({})", b))),
            OneOrBoth::Both(a, b) => Some(Arc::new(format!("Both({}, {})", a, b))),
        },
        a_event,
        b_event,
    );

    let logger = Arc::new(TestLogger::<String> {
        log: Mutex::new(vec![]),
    });
    let logger2: Arc<dyn EventCallback<String>> = logger.clone();
    combined.0.subscribe(Arc::downgrade(&logger2));

    // Fire A only
    runtime.schedule_trigger(&a_trigger, Arc::new(1));
    runtime.propagate();

    // Fire B only
    runtime.schedule_trigger(&b_trigger, Arc::new("hello".to_string()));
    runtime.propagate();

    // Fire both simultaneously
    runtime.schedule_trigger(&a_trigger, Arc::new(42));
    runtime.schedule_trigger(&b_trigger, Arc::new("world".to_string()));
    runtime.propagate();

    let log = logger.log.lock().unwrap();
    let log = log.clone();

    assert_eq!(
        log,
        vec![
            Arc::new("A(1)".to_string()),
            Arc::new("B(hello)".to_string()),
            Arc::new("Both(42, world)".to_string()),
        ]
    );
}
