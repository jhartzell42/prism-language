use std::panic::catch_unwind;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use test_log::test;

use crate::{
    backends::test_backend::{crickets, dynamic, event, script, trigger},
    dynamic::Dynamic,
    event::Event,
    runtime::Runtime,
    value::{PrismArc, Value},
    widget::{Widget, WidgetBuilder},
};

#[test]
fn fold_dyn_cyclic_dyn() {
    struct TestWidget;

    impl Widget for TestWidget {
        type Output = ();

        fn build(&self, builder: &mut WidgetBuilder) -> Self::Output {
            let increment = builder.add_external_event::<i32>("increment".to_string());
            let accumulator = builder
                .add_cyclic_dynamic::<i32>("accumulator".to_string())
                .1;
            let new_accumulator = increment.tag_map(|a, b| Some(Value::from(*a + *b)), accumulator);
            let accumulator = Dynamic::hold(Value::from(0i32), new_accumulator);
            builder.close_cyclic_dynamic_by_name("accumulator", accumulator.clone());

            // This tests the behavior
            builder.add_public_dynamic("accumulator".to_string(), accumulator.clone());
            // This tests the event
            builder.add_public_event("new_value".to_string(), accumulator.event());
        }
    }

    script([
        // Check starting state
        dynamic([], "accumulator", 0),
        crickets(),
        // Add 3
        trigger([], "increment", 3),
        event([], "new_value", 3),
        crickets(),
        dynamic([], "accumulator", 3),
        // Subtract 4
        trigger([], "increment", -4),
        event([], "new_value", -1),
        crickets(),
        dynamic([], "accumulator", -1),
        // Add 1
        trigger([], "increment", 1),
        event([], "new_value", 0),
        crickets(),
        dynamic([], "accumulator", 0),
        // Add 0, refires event (hmpf)
        trigger([], "increment", 0),
        event([], "new_value", 0),
        crickets(),
        dynamic([], "accumulator", 0),
    ])
    .test(&TestWidget)
}

#[test]
fn fold_dyn() {
    struct TestWidget;

    impl Widget for TestWidget {
        type Output = ();

        fn build(&self, builder: &mut WidgetBuilder) -> Self::Output {
            let increment = builder.add_external_event::<i32>("increment".to_string());
            let dynamic = builder.fold_dyn(
                "accumulator".to_string(),
                |a, b| Value::from(*a + *b),
                Value::from(0),
                increment,
            );
            // This tests the behavior
            builder.add_public_dynamic("accumulator".to_string(), dynamic.clone());
            // This tests the event
            builder.add_public_event("new_value".to_string(), dynamic.event());
        }
    }

    script([
        // Check starting state
        dynamic([], "accumulator", 0),
        crickets(),
        // Add 3
        trigger([], "increment", 3),
        event([], "new_value", 3),
        crickets(),
        dynamic([], "accumulator", 3),
        // Subtract 4
        trigger([], "increment", -4),
        event([], "new_value", -1),
        crickets(),
        dynamic([], "accumulator", -1),
        // Add 1
        trigger([], "increment", 1),
        event([], "new_value", 0),
        crickets(),
        dynamic([], "accumulator", 0),
        // Add 0, refires event (hmpf)
        trigger([], "increment", 0),
        event([], "new_value", 0),
        crickets(),
        dynamic([], "accumulator", 0),
    ])
    .test(&TestWidget)
}

#[test]
fn dynamic_widget() {
    struct TestWidget;

    impl Widget for TestWidget {
        type Output = ();

        fn build(&self, builder: &mut WidgetBuilder) -> Self::Output {
            #[derive(Debug, Clone)]
            struct Inner(String);

            impl Widget for Inner {
                type Output = ();

                fn build(&self, builder: &mut WidgetBuilder) -> Self::Output {
                    builder
                        .add_public_dynamic(self.0.clone(), Dynamic::constant(PrismArc::new(())));
                }
            }

            let rebuild = builder.add_external_event::<Arc<String>>("rebuild".into());
            let d_widget = Dynamic::hold(
                PrismArc::new(Inner("initial".to_string())),
                rebuild.filter_map(|s| Some(PrismArc::new(Inner(s.extract())))),
            );
            builder.bind(&d_widget.dynamic_widget());
        }
    }

    script([
        dynamic([0, 0], "initial", Arc::new(())),
        trigger([], "rebuild", Arc::new("hello".to_string())),
        dynamic([0, 0], "hello", Arc::new(())),
        trigger([], "rebuild", Arc::new("hi".to_string())),
        dynamic([0, 0], "hi", Arc::new(())),
    ])
    .test(&TestWidget);
}

#[derive(Debug)]
struct Leaf(u32);

impl Widget for Leaf {
    type Output = u32;

    fn build(&self, _: &mut WidgetBuilder) -> u32 {
        self.0
    }
}

#[test]
fn dyn_rebuilds_without_a_backend_delegate() {
    let mut runtime = Runtime::new();
    let (input, trigger) = Event::<u32>::external();
    let widgets = Dynamic::hold(
        PrismArc::new(Leaf(0)),
        input.map(|x| PrismArc::new(Leaf(*x))),
    );
    let (_node, built) = WidgetBuilder::build_root(&runtime, &widgets.dynamic_widget(), 0);
    let latest = Dynamic::hold(PrismArc::new(0u32), (*built).clone());
    runtime.propagate();

    runtime.schedule_trigger(&trigger, 5u32.into()).unwrap();
    runtime.propagate();

    assert_eq!(*latest.behavior().0.query_for_tag(), 5);
}

#[test]
fn dyn_rebuilds_when_its_input_arrives_in_occurrence_zero() {
    let mut runtime = Runtime::new();
    let (input, trigger) = Event::<u32>::external();
    let widgets = Dynamic::hold(
        PrismArc::new(Leaf(0)),
        input.map(|x| PrismArc::new(Leaf(*x))),
    );
    let (_node, built) = WidgetBuilder::build_root(&runtime, &widgets.dynamic_widget(), 0);
    let latest = Dynamic::hold(PrismArc::new(0u32), (*built).clone());

    runtime.schedule_trigger(&trigger, 5u32.into()).unwrap();
    runtime.propagate();

    assert_eq!(*latest.behavior().0.query_for_tag(), 5);
}

#[test]
fn nested_dyn_rebuilds_in_the_same_occurrence_as_its_parent() {
    #[derive(Debug)]
    struct Middle(Event<u32>);

    impl Widget for Middle {
        type Output = Arc<Event<Arc<u32>>>;

        fn build(&self, builder: &mut WidgetBuilder) -> Self::Output {
            let leaves = Dynamic::hold(
                PrismArc::new(Leaf(0)),
                self.0.map(|x| PrismArc::new(Leaf(*x))),
            );
            builder.bind(&leaves.dynamic_widget())
        }
    }

    let mut runtime = Runtime::new();
    let (input, trigger) = Event::<u32>::external();
    let deep = input.map(|x| x).map(|x| x).map(|x| x).map(|x| x);
    let _deep_is_observed = Dynamic::hold(0u32.into(), deep.clone());
    let initial_middle = PrismArc::new(Middle(deep.clone()));
    let middles = Dynamic::hold(
        initial_middle,
        input.map(move |_| PrismArc::new(Middle(deep.clone()))),
    );
    let (_node, _built) = WidgetBuilder::build_root(&runtime, &middles.dynamic_widget(), 0);
    runtime.propagate();

    runtime.schedule_trigger(&trigger, 7u32.into()).unwrap();
    runtime.propagate();
}

#[test]
fn cyclic_dynamic_event_fires_when_its_value_changes_in_the_build_occurrence() {
    struct StartedAtSeven;

    impl Widget for StartedAtSeven {
        type Output = Dynamic<u32>;

        fn build(&self, builder: &mut WidgetBuilder) -> Dynamic<u32> {
            let (index, cyclic) = builder.add_cyclic_dynamic::<u32>("value".into());
            let started = builder.done_event().map(|_| Value::from(7u32));
            builder.close_cyclic_dynamic_by_index(index, Dynamic::hold(0u32.into(), started));
            cyclic
        }
    }

    let mut runtime = Runtime::new();
    let (_node, cyclic) = WidgetBuilder::build_root(&runtime, &StartedAtSeven, 0);
    let last_event_value = Dynamic::hold(0u32.into(), cyclic.event());
    runtime.propagate();

    assert_eq!(*cyclic.behavior().0.query_for_tag(), 7);
    assert_eq!(*last_event_value.behavior().0.query_for_tag(), 7);
}

#[test]
fn fold_dyn_counts_an_event_from_the_build_occurrence() {
    struct CountStarted;

    impl Widget for CountStarted {
        type Output = Dynamic<u32>;

        fn build(&self, builder: &mut WidgetBuilder) -> Dynamic<u32> {
            let started = builder.done_event().map(|_| Value::from(1u32));
            builder.fold_dyn(
                "count".into(),
                |count: Value<u32>, step: Value<u32>| Value::from(*count + *step),
                0u32.into(),
                started,
            )
        }
    }

    let mut runtime = Runtime::new();
    let (_node, count) = WidgetBuilder::build_root(&runtime, &CountStarted, 0);
    runtime.propagate();

    assert_eq!(*count.behavior().0.query_for_tag(), 1);
}

#[test]
fn a_same_occurrence_cycle_through_a_cyclic_event_is_reported_instead_of_hanging() {
    struct Loop(Event<u32>);

    impl Widget for Loop {
        type Output = Dynamic<u32>;

        fn build(&self, builder: &mut WidgetBuilder) -> Dynamic<u32> {
            let (index, looped_back) = builder.add_cyclic_event::<u32>("loop".into());
            let next = Event::leftmost(vec![
                self.0.clone(),
                looped_back.map(|x| Value::from(*x + 1)),
            ]);
            builder.close_cyclic_event_by_index(index, next.clone());
            Dynamic::hold(0u32.into(), next)
        }
    }

    let (reported_sender, reported_receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let outcome = catch_unwind(|| {
            let mut runtime = Runtime::new();
            let (input, trigger) = Event::<u32>::external();
            let (_node, _held) = WidgetBuilder::build_root(&runtime, &Loop(input), 0);
            runtime.propagate();
            runtime.schedule_trigger(&trigger, 0u32.into()).unwrap();
            runtime.propagate();
        });
        reported_sender.send(outcome.is_err()).unwrap();
    });
    assert_eq!(reported_receiver.recv_timeout(Duration::from_secs(5)), Ok(true));
}

#[test]
fn a_dynamic_defined_as_a_function_of_itself_is_reported_instead_of_hanging() {
    struct SelfReferential;

    impl Widget for SelfReferential {
        type Output = Dynamic<u32>;

        fn build(&self, builder: &mut WidgetBuilder) -> Dynamic<u32> {
            let (index, previous) = builder.add_cyclic_dynamic::<u32>("value".into());
            let next = previous.map(|x| Value::from(*x + 1));
            builder.close_cyclic_dynamic_by_index(index, next.clone());
            next
        }
    }

    let (reported_sender, reported_receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let outcome = catch_unwind(|| {
            let mut runtime = Runtime::new();
            let (_node, next) = WidgetBuilder::build_root(&runtime, &SelfReferential, 0);
            runtime.propagate();
            next.behavior().0.query_for_tag();
        });
        reported_sender.send(outcome.is_err()).unwrap();
    });
    assert_eq!(reported_receiver.recv_timeout(Duration::from_secs(5)), Ok(true));
}

#[test]
fn a_widget_built_mid_occurrence_sees_the_same_events_at_any_depth() {
    #[derive(Debug)]
    struct Child {
        shallow: Event<u32>,
        deep: Event<u32>,
        built: Arc<Mutex<Vec<(Dynamic<u32>, Dynamic<u32>)>>>,
    }

    impl Widget for Child {
        type Output = ();

        fn build(&self, _: &mut WidgetBuilder) {
            self.built.lock().unwrap().push((
                Dynamic::hold(0u32.into(), self.shallow.clone()),
                Dynamic::hold(0u32.into(), self.deep.clone()),
            ));
        }
    }

    let mut runtime = Runtime::new();
    let (input, trigger) = Event::<u32>::external();
    let deep = input.map(|x| x).map(|x| x).map(|x| x).map(|x| x).map(|x| x);
    let _deep_is_observed = Dynamic::hold(0u32.into(), deep.clone());
    let built = Arc::new(Mutex::new(vec![]));
    let new_child = {
        let (input, deep, built) = (input.clone(), deep.clone(), built.clone());
        move || PrismArc::new(Child { shallow: input.clone(), deep: deep.clone(), built: built.clone() })
    };
    let children = Dynamic::hold(new_child(), input.map(move |_| new_child()));
    let (_node, _output) = WidgetBuilder::build_root(&runtime, &children.dynamic_widget(), 0);
    runtime.propagate();

    runtime.schedule_trigger(&trigger, 42u32.into()).unwrap();
    runtime.propagate();

    let (shallow_held, deep_held) = built.lock().unwrap().last().unwrap().clone();
    assert_eq!(
        *shallow_held.behavior().0.query_for_tag(),
        *deep_held.behavior().0.query_for_tag()
    );
}

#[test]
fn closing_a_cyclic_dynamic_with_the_wrong_type_fails_while_building() {
    struct WrongType;

    impl Widget for WrongType {
        type Output = ();

        fn build(&self, builder: &mut WidgetBuilder) {
            let (index, _value) = builder.add_cyclic_dynamic::<u32>("value".into());
            builder.close_cyclic_dynamic_by_index(index, Dynamic::constant(Value::from(-1i32)));
        }
    }

    let runtime = Runtime::new();
    let build = catch_unwind(|| WidgetBuilder::build_root(&runtime, &WrongType, 0));

    assert!(build.is_err());
}
