use std::sync::{Arc, Mutex};

use crate::{
    backends::test_backend::{
        Path, TestAction, TestScript, TestStep, crickets, delay_trigger, event, script, trigger,
    },
    event::{Event, EventCallback},
    runtime::Runtime,
    widget::{ExternalEventInfo, Widget},
};

pub struct TestLogger<T> {
    pub log: Mutex<Vec<Arc<T>>>,
}

impl<T: 'static + Send + Sync> EventCallback<T> for TestLogger<T> {
    fn event_fired(&self, _: &Runtime, value: std::sync::Arc<T>) {
        let mut log = self.log.lock().unwrap();
        log.push(value);
    }

    fn invalidate_height(&self) {}
}

#[test]
fn event_filter_test() {
    struct TestWidget;

    impl Widget for TestWidget {
        type Output = ();

        fn build(&self, builder: &mut crate::widget::WidgetBuilder) -> Self::Output {
            let ExternalEventInfo { event, .. } =
                builder.add_external_event::<u32>("unfiltered".to_string());
            let filter_zero_event = event.filter_map(|a| {
                if *a == 0 {
                    None
                } else {
                    Some(Arc::new(*a - 1))
                }
            });
            builder.add_event("filtered".to_string(), filter_zero_event);
        }
    }

    let result = script([
        trigger([], "unfiltered", 2u32),
        event([], "filtered", 1u32),
        trigger([], "unfiltered", 0u32),
        trigger([], "unfiltered", 0u32),
        trigger([], "unfiltered", 2u32),
        event([], "filtered", 1u32),
        trigger([], "unfiltered", 0u32),
    ])
    .run(&TestWidget);

    if let Err(err) = result {
        panic!("{err}");
    }
}

#[test]
fn switch_hold_test2() {
    struct TestWidget;

    impl Widget for TestWidget {
        type Output = ();

        fn build(&self, builder: &mut crate::widget::WidgetBuilder) -> Self::Output {
            let ExternalEventInfo { event: outer, .. } =
                builder.add_external_event::<u32>("outer".to_string());
            let ExternalEventInfo { event: inner, .. } =
                builder.add_external_event::<u32>("inner".to_string());

            let outer_event = outer.filter_map(move |outer| {
                Some(Arc::new(
                    inner.filter_map(move |inner| Some(Arc::new(*outer + *inner))),
                ))
            });
            let event = outer_event.switch_hold();
            builder.add_event("output".into(), event);
        }
    }

    let result = script([
        // Outer event has not fired
        trigger([], "inner", 0u32),
        trigger([], "inner", 0u32),
        trigger([], "inner", 0u32),
        crickets(),
        // Now fire outer event, simultaneously to new inner
        delay_trigger([], "outer", 0u32),
        trigger([], "inner", 0u32),
        crickets(),
        // Now fire inner
        trigger([], "inner", 0u32),
        event([], "output", 0u32),
        trigger([], "inner", 1u32),
        event([], "output", 1u32),
        // Update outer event, twice
        trigger([], "outer", 3u32),
        crickets(),
        trigger([], "outer", 4u32),
        crickets(),
        // Fire inner
        trigger([], "inner", 3u32),
        event([], "output", 7u32),
    ])
    .run(&TestWidget);

    if let Err(err) = result {
        panic!("{err}");
    }
}

#[test]
fn leftmost_test() {
    struct TestWidget;

    impl Widget for TestWidget {
        type Output = ();

        fn build(&self, builder: &mut crate::widget::WidgetBuilder) -> Self::Output {
            let ExternalEventInfo { event: left, .. } =
                builder.add_external_event::<u32>("left".to_string());
            let ExternalEventInfo { event: right, .. } =
                builder.add_external_event::<u32>("right".to_string());

            let output = Event::leftmost(vec![left, right]);
            builder.add_event("output".into(), output);
        }
    }

    let result = script([
        crickets(),
        // Just left
        trigger([], "left", 0u32),
        event([], "output", 0u32),
        crickets(),
        // Just right
        trigger([], "right", 3u32),
        event([], "output", 3u32),
        crickets(),
        // Both
        delay_trigger([], "right", 3u32),
        trigger([], "left", 4u32),
        event([], "output", 4u32),
        crickets(),
        // Both reversed
        delay_trigger([], "left", 5u32),
        trigger([], "right", 6u32),
        event([], "output", 5u32),
        crickets(),
    ])
    .run(&TestWidget);

    if let Err(err) = result {
        panic!("{err}");
    }
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
