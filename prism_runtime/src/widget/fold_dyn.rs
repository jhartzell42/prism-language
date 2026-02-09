use std::sync::Arc;

use crate::{dynamic::Dynamic, event::Event, widget::WidgetBuilder};

impl WidgetBuilder<'_> {
    /// Create a dynamic that updates upon receiving an event based on a function
    /// of the old value and the new event.
    pub fn fold_dyn<A: 'static + Send + Sync, B: 'static + Send + Sync>(
        &mut self,
        name: String,
        f: impl 'static + Sync + Send + Fn(Arc<A>, Arc<B>) -> Arc<A>,
        initial: Arc<A>,
        update: Event<B>,
    ) -> Dynamic<A> {
        let (cycle_handle, both_event_in) = self.add_cyclic_event(name);
        let new_value = both_event_in
            .filter_map(move |x: Arc<(Arc<A>, Arc<B>)>| Some(f(x.0.clone(), x.1.clone())));
        let dynamic = Dynamic::hold(initial, new_value);
        let both_event_out =
            update.tag_map(|update, old| Some(Arc::new((old, update))), dynamic.clone());
        self.close_cyclic_event_by_index(cycle_handle, both_event_out);
        dynamic
    }
}
