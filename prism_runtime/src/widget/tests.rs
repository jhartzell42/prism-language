use std::sync::Arc;
use test_log::test;

use crate::{
    backends::test_backend::{crickets, dynamic, event, script, trigger},
    dynamic::Dynamic,
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

/// Q-G7 experiment: a widget torn down by a rebuild fires a request (a public event) later
/// in the same tick. The short path fires it below the rebuild's height, the long path
/// above it. Each generation names its event, so the old and new inner are told apart.
fn torn_down_request(long_path: bool) -> Vec<String> {
    use crate::event::Event;
    #[derive(Debug, Clone)]
    struct Inner(Event<i32>, bool, u32);
    impl Widget for Inner {
        type Output = ();
        fn build(&self, builder: &mut WidgetBuilder) -> Self::Output {
            let mut e = self.0.clone();
            if self.1 {
                for _ in 0..5 {
                    e = e.filter_map(|x| Some(Value::from(*x)));
                }
            }
            builder.add_public_event(format!("sends{}", self.2), e);
        }
    }
    #[derive(Debug, Clone)]
    struct Outer(bool);
    impl Widget for Outer {
        type Output = ();
        fn build(&self, builder: &mut WidgetBuilder) -> Self::Output {
            let hang_up = builder.add_external_event::<i32>("hangUp".into());
            let long = self.0;
            let h2 = hang_up.clone();
            let d_widget = Dynamic::hold(
                PrismArc::new(Inner(hang_up.clone(), long, 0)),
                h2.filter_map(move |_| Some(PrismArc::new(Inner(hang_up.clone(), long, 1)))),
            );
            builder.bind(&d_widget.dynamic_widget());
        }
    }
    let (_, mut backend) = crate::backends::test_backend::backend::TestBackend::build_tree(&Outer(long_path));
    let _ = backend.drain_paths();
    backend
        .main_loop(crate::backends::test_backend::TestScript { steps: vec![trigger([], "hangUp", 7)] })
        .unwrap();
    backend.drain_paths()
}

#[test]
fn q_g7_report() {
    println!("short path: {:?}", torn_down_request(false));
    println!("long path: {:?}", torn_down_request(true));
}
