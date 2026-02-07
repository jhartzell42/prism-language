use std::{
    marker::PhantomData,
    sync::{Arc, Mutex, Weak},
};

use crate::{
    dynamic::Dynamic,
    event::{Event, EventCallback, EventImpl},
    runtime::Action,
    widget::{
        Widget,
        builder::{ExternalEventInfo, WidgetBuilder},
        delegate::WidgetDelegate,
    },
};

impl<T: Widget + 'static> Dynamic<T> {
    /// Create a dynamic widget that starts out by building current value of `self`,
    /// and then replaces it with whatever widget `self` updates to.
    pub fn dynamic_widget(&self) -> impl Widget<Output = Event<T::Output>> {
        DynamicWidget {
            dynamic: self.clone(),
            phantom: PhantomData,
        }
    }
}

struct DynamicWidget<T: 'static + Widget<Output = O>, O> {
    dynamic: Dynamic<T>,
    phantom: PhantomData<O>,
}

struct DynamicWidgetEvent<T: 'static + Widget<Output = O>, O> {
    widget: DynamicWidget<T, O>,
    weak_self: Weak<Self>,
    delegate: Arc<dyn WidgetDelegate>,
}

impl<T: 'static + Widget<Output = O>, O> Clone for DynamicWidget<T, O> {
    fn clone(&self) -> Self {
        Self {
            dynamic: self.dynamic.clone(),
            phantom: self.phantom.clone(),
        }
    }
}

impl<T: 'static + Widget<Output = O>, O: 'static> Widget for DynamicWidget<T, O> {
    type Output = Event<O>;
    fn build(&self, builder: &mut WidgetBuilder) -> Event<O> {
        let initial_widget = self.dynamic.behavior().0.query_for_tag();

        // `builder.done_event()` will only fire once, so we can use this hack
        let output = Mutex::new(Some(builder.bind(&*initial_widget)));
        let first_time_event = builder.done_event().filter_map(move |_| {
            let mut output = output.lock().unwrap();
            let output = output.take().unwrap();
            Some(Arc::new(output))
        });

        let next_event = Arc::new_cyclic(|weak_self| DynamicWidgetEvent {
            widget: self.clone(),
            weak_self: weak_self.clone(),
            delegate: todo!("where the hell do we get this from?"),
        });
        self.dynamic.event().0.subscribe(Arc::downgrade(
            &(next_event.clone() as Arc<dyn EventCallback<T>>),
        ));
        let next_event = Event(next_event);
        // Keep this alive even if no one's using the output event.
        builder.add_event("rebuilt".to_string(), next_event.clone());

        let event = Event::leftmost(vec![first_time_event, next_event]);
        event
    }
}

impl<T: 'static + Widget> EventImpl<T::Output> for DynamicWidgetEvent<T, T::Output> {
    fn subscribe(&self, cb: std::sync::Weak<dyn EventCallback<T::Output>>) {
        todo!("register subscribers, fire in action")
    }

    fn height(&self) -> usize {
        self.widget.dynamic.event().0.height() + 1
    }
}

impl<T: 'static + Widget> EventCallback<T> for DynamicWidgetEvent<T, T::Output> {
    fn event_fired(&self, runtime: &crate::runtime::Runtime, value: Arc<T>) {
        let Some(this) = self.weak_self.upgrade() else {
            // The outer widget's been destroyed lol, guess we can be done.
            return;
        };
        runtime.schedule(self.height(), DWAction { event: this, value });
    }

    fn invalidate_height(&self) {
        todo!()
    }
}

struct DWAction<T: 'static + Widget> {
    event: Arc<DynamicWidgetEvent<T, T::Output>>,
    value: Arc<T>,
}

impl<T: 'static + Widget> Action for DWAction<T> {
    fn act(self: Box<Self>, runtime: &crate::runtime::Runtime) {
        let output = WidgetBuilder::build_root(runtime, &*self.value, todo!(), self.event.height());
        todo!("fire our subscribers with `output`")
    }
}
