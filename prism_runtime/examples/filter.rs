use std::sync::Arc;

use prism_runtime::{event::Event, runtime::Runtime};

fn main() {
    env_logger::init();
    let mut runtime = Runtime::new();
    let (event, trigger) = Event::<u32>::external();
    event.trace("trigger".into());
    let filter_zero_event = event.filter_map(|a| {
        if *a == 0 {
            None
        } else {
            Some(Arc::new(*a - 1))
        }
    });
    filter_zero_event.trace("filter_zero".into());

    runtime.schedule_trigger(&trigger, Arc::new(2));
    runtime.propagate();
    runtime.schedule_trigger(&trigger, Arc::new(0));
    runtime.propagate();
    runtime.schedule_trigger(&trigger, Arc::new(2));
    runtime.propagate();
}
