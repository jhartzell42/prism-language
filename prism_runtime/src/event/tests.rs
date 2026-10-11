use std::sync::{Arc, mpsc};
use std::time::Duration;
use test_log::test;

use crate::{
    backends::test_backend::{crickets, delay_trigger, event, script, trigger},
    dynamic::Dynamic,
    event::{Event, EventCallback},
    runtime::Runtime,
    value::{PrismArc, Value},
    widget::Widget,
};

#[test]
fn event_filter_test() {
    struct TestWidget;

    impl Widget for TestWidget {
        type Output = ();

        fn build(&self, builder: &mut crate::widget::WidgetBuilder) -> Self::Output {
            let event = builder.add_external_event::<u32>("unfiltered".to_string());
            let filter_zero_event = event.filter_map(|a| {
                let a = a.get();
                if a == 0 {
                    None
                } else {
                    Some(Value::from(a - 1))
                }
            });
            builder.add_public_event("filtered".to_string(), filter_zero_event);
        }
    }

    script([
        trigger([], "unfiltered", 2u32),
        event([], "filtered", 1u32),
        trigger([], "unfiltered", 0u32),
        trigger([], "unfiltered", 0u32),
        trigger([], "unfiltered", 2u32),
        event([], "filtered", 1u32),
        trigger([], "unfiltered", 0u32),
    ])
    .test(&TestWidget);
}

#[test]
fn switch_hold_test2() {
    struct TestWidget;

    impl Widget for TestWidget {
        type Output = ();

        fn build(&self, builder: &mut crate::widget::WidgetBuilder) -> Self::Output {
            let outer = builder.add_external_event::<u32>("outer".to_string());
            let inner = builder.add_external_event::<u32>("inner".to_string());

            let outer_event = outer.filter_map(move |outer| {
                Some(
                    inner
                        .filter_map(move |inner| Some(Value::from(*outer + *inner)))
                        .into(),
                )
            });
            let event = outer_event.switch_hold(builder.runtime);
            builder.add_public_event("output".into(), event);
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
            let left = builder.add_external_event::<u32>("left".to_string());
            let right = builder.add_external_event::<u32>("right".to_string());

            let output = Event::leftmost(vec![left, right]);
            builder.add_public_event("output".into(), output);
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
    struct TestWidget;

    impl Widget for TestWidget {
        type Output = ();

        fn build(&self, builder: &mut crate::widget::WidgetBuilder) -> Self::Output {
            let a = builder.add_external_event::<u32>("a".to_string());
            let b = builder.add_external_event::<Arc<String>>("b".to_string());

            let output = Event::combine(
                |oob| {
                    Some(PrismArc::new(match oob {
                        crate::event::OneOrBoth::A(a) => {
                            let a = a.get();
                            format!("a={a}")
                        }
                        crate::event::OneOrBoth::B(b) => {
                            let b = b.extract();
                            format!("b={b}")
                        }
                        crate::event::OneOrBoth::Both(a, b) => {
                            let a = a.get();
                            let b = b.extract();
                            format!("a={a} b={b}")
                        }
                    }))
                },
                a,
                b,
            );
            builder.add_public_event("output".into(), output);
        }
    }

    let result = script([
        crickets(),
        // Just a
        trigger([], "a", 0u32),
        event([], "output", Arc::new("a=0".to_string())),
        crickets(),
        // Just right
        trigger([], "b", Arc::new("hello".to_string())),
        event([], "output", Arc::new("b=hello".to_string())),
        crickets(),
        // Both
        delay_trigger([], "b", Arc::new("hey".to_string())),
        trigger([], "a", 4u32),
        event([], "output", Arc::new("a=4 b=hey".to_string())),
        crickets(),
        // Both reversed
        delay_trigger([], "a", 5u32),
        trigger([], "b", Arc::new("wassup".to_string())),
        event([], "output", Arc::new("a=5 b=wassup".to_string())),
        crickets(),
    ])
    .run(&TestWidget);

    if let Err(err) = result {
        panic!("{err}");
    }
}

#[test]
fn switching_to_a_deeper_event_keeps_downstream_heights_valid() {
    let mut runtime = Runtime::new();
    let (outer, outer_trigger) = Event::<Event<u32>>::external();
    let (shallow, shallow_trigger) = Event::<u32>::external();
    let (deep_source, deep_trigger) = Event::<u32>::external();
    let deep = deep_source.map(|x| x).map(|x| x).map(|x| x).map(|x| x);
    let held = Dynamic::hold(&runtime, 0u32.into(), outer.switch_hold(&runtime).map(|x| x).map(|x| x));

    runtime.schedule_trigger(&outer_trigger, shallow.into()).unwrap();
    runtime.propagate();
    runtime.schedule_trigger(&shallow_trigger, 1u32.into()).unwrap();
    runtime.propagate();
    runtime.schedule_trigger(&outer_trigger, deep.into()).unwrap();
    runtime.propagate();
    runtime.schedule_trigger(&deep_trigger, 2u32.into()).unwrap();
    runtime.propagate();

    assert_eq!(*held.behavior().0.query_for_tag(), 2);
}

struct CallOnFire<F>(F);

impl<F: Fn(&Runtime) + Send + Sync> EventCallback<u32> for CallOnFire<F> {
    fn event_fired(&self, runtime: &Runtime, _: Value<u32>) {
        (self.0)(runtime)
    }

    fn already_fired(&self, _: &Runtime, _: Value<u32>) {}

    fn invalidate_height(&self) {}
}

#[test]
fn scheduling_a_trigger_during_propagation_is_refused_without_a_panic() {
    let mut runtime = Runtime::new();
    let (first, first_trigger) = Event::<u32>::external();
    let (second, second_trigger) = Event::<u32>::external();
    let held_second = Dynamic::hold(&runtime, 0u32.into(), second);
    let callback: Arc<dyn EventCallback<u32>> = Arc::new(CallOnFire(move |runtime: &Runtime| {
        let _ = runtime.schedule_trigger(&second_trigger, 7u32.into());
    }));
    first.0.subscribe(&runtime, Arc::downgrade(&callback));

    runtime.schedule_trigger(&first_trigger, 1u32.into()).unwrap();
    runtime.propagate();
    runtime.propagate();

    assert_eq!(*held_second.behavior().0.query_for_tag(), 0);
}

#[test]
fn a_subscriber_rescheduling_its_own_trigger_does_not_hang() {
    let (finished_sender, finished_receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut runtime = Runtime::new();
        let (event, trigger) = Event::<u32>::external();
        let same_trigger = trigger.clone();
        let callback: Arc<dyn EventCallback<u32>> = Arc::new(CallOnFire(move |runtime: &Runtime| {
            let _ = runtime.schedule_trigger(&same_trigger, 2u32.into());
        }));
        event.0.subscribe(&runtime, Arc::downgrade(&callback));
        runtime.schedule_trigger(&trigger, 1u32.into()).unwrap();
        runtime.propagate();
        finished_sender.send(()).unwrap();
    });
    assert_eq!(finished_receiver.recv_timeout(Duration::from_secs(5)), Ok(()));
}

#[test]
fn a_second_value_on_one_trigger_before_propagating_is_refused() {
    let mut runtime = Runtime::new();
    let (event, trigger) = Event::<u32>::external();
    let held = Dynamic::hold(&runtime, 0u32.into(), event);

    let _ = runtime.schedule_trigger(&trigger, 1u32.into());
    let _ = runtime.schedule_trigger(&trigger, 2u32.into());
    runtime.propagate();

    assert_eq!(*held.behavior().0.query_for_tag(), 1);
}
