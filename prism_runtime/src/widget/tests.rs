use std::sync::Arc;
use test_log::test;

use crate::{
    backends::test_backend::{dynamic, script, trigger},
    dynamic::Dynamic,
    widget::{Widget, WidgetBuilder},
};

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
