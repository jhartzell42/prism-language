use std::sync::Arc;
use test_log::test;

use crate::{
    backends::test_backend::{crickets, dynamic, event, script, trigger},
    dynamic::Dynamic,
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
            let new_accumulator = increment.tag_map(|a, b| Some(Arc::new(*a + *b)), accumulator);
            let accumulator = Dynamic::hold(Arc::new(0), new_accumulator);
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
                |a, b| Arc::new(*a + *b),
                Arc::new(0),
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
            struct Inner(String);

            impl Widget for Inner {
                type Output = ();

                fn build(&self, builder: &mut WidgetBuilder) -> Self::Output {
                    builder.add_public_dynamic(self.0.clone(), Dynamic::constant(Arc::new(())));
                }
            }

            let rebuild = builder.add_external_event::<String>("rebuild".to_string());
            let d_widget = Dynamic::hold(
                Arc::new(Inner("initial".to_string())),
                rebuild.filter_map(|s| Some(Arc::new(Inner(Arc::unwrap_or_clone(s))))),
            );
            builder.bind(&d_widget.dynamic_widget());
        }
    }

    script([
        dynamic([0, 0], "initial", ()),
        trigger([], "rebuild", "hello".to_string()),
        dynamic([0, 0], "hello", ()),
        trigger([], "rebuild", "hi".to_string()),
        dynamic([0, 0], "hi", ()),
    ])
    .test(&TestWidget);
}
