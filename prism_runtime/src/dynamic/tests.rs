use std::sync::Arc;
use test_log::test;

use crate::{
    backends::test_backend::{crickets, delay_trigger, dynamic, event, script, trigger},
    dynamic::Dynamic,
    widget::Widget,
};

#[test]
fn hold_map2_test() {
    struct TestWidget;

    impl Widget for TestWidget {
        type Output = ();

        fn build(&self, builder: &mut crate::widget::WidgetBuilder) -> Self::Output {
            let update = builder.add_external_event::<u32>("update".to_string());
            let tag = builder.add_external_event::<()>("query".to_string());
            let d_hold = Dynamic::hold(Arc::new(0), update);
            let d_const = Dynamic::constant(Arc::new(10));
            let d_combo = Dynamic::map2(d_hold.clone(), d_const, |a, b| Arc::new(*a + *b));
            let output = tag.tag(d_combo).filter_map(|x| Some(Arc::new(*x.1)));
            builder.add_public_event("output".into(), output);
            builder.add_public_dynamic("output".into(), d_hold);
        }
    }

    // `output` event is offset by 10, `output` dynamic is not
    script([
        // Query
        trigger([], "query".into(), ()),
        event([], "output".into(), 10u32),
        // Update
        trigger([], "update".into(), 4u32),
        crickets(),
        dynamic([], "output".into(), 4u32),
        // Update more
        trigger([], "update".into(), 5u32),
        trigger([], "update".into(), 6u32),
        crickets(),
        dynamic([], "output".into(), 6u32),
        // Query
        trigger([], "query".into(), ()),
        event([], "output".into(), 16u32),
        // Simultaneous, don't be prompt
        delay_trigger([], "update".into(), 7u32),
        trigger([], "query".into(), ()),
        dynamic([], "output".into(), 7u32),
        event([], "output".into(), 16u32),
        // Now it's updated
        trigger([], "query".into(), ()),
        event([], "output".into(), 17u32),
    ])
    .test(&TestWidget);
}
