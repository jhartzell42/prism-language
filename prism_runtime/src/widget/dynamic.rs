use std::sync::{Arc, Mutex, Weak};

use crate::{
    behavior::Behavior,
    dynamic::Dynamic,
    event::{
        Event, EventCallback, EventImpl,
        subscriber_list::{SubscriptionEvent, SubscriptionManager},
    },
    runtime::Action,
    widget::{Widget, WidgetDelegateContext, WidgetNode, builder::WidgetBuilder},
};

impl<T: Widget + 'static + Send + Sync> Dynamic<T> {
    /// Create a dynamic widget that starts out by building current value of `self`,
    /// and then replaces it with whatever widget `self` updates to.
    pub fn dynamic_widget(&self) -> impl Widget<Output = Event<T::Output>> {
        DynamicWidget {
            dynamic: self.clone(),
        }
    }
}

struct DynamicWidget<T: 'static + Widget> {
    dynamic: Dynamic<T>,
}

struct DynamicWidgetEvent<T: 'static + Widget> {
    widget: DynamicWidget<T>,
    weak_self: Weak<Self>,
    node: Behavior<WidgetNode>,
    subscribers: SubscriptionManager<T::Output, Self>,
}

impl<T: 'static + Widget> Clone for DynamicWidget<T> {
    fn clone(&self) -> Self {
        Self {
            dynamic: self.dynamic.clone(),
        }
    }
}

impl<T: 'static + Widget> Widget for DynamicWidget<T> {
    type Output = Event<T::Output>;
    fn build(&self, builder: &mut WidgetBuilder) -> Self::Output {
        let initial_widget = self.dynamic.behavior().0.query_for_tag();

        // `builder.done_event()` will only fire once, so we can use this hack
        let done_event = builder.done_event();
        let output = Mutex::new(Some(builder.bind(&*initial_widget)));
        let first_time_event = builder.done_event().filter_map(move |_| {
            let mut output = output.lock().unwrap();
            let output = output.take().unwrap();
            Some(Arc::new(output))
        });

        let next_event = Arc::new_cyclic(|weak_self| DynamicWidgetEvent {
            widget: self.clone(),
            weak_self: weak_self.clone(),
            node: Behavior::hold(
                Arc::new(WidgetNode::new()),
                done_event.filter_map(|x| Some(x)),
            ),
            subscribers: SubscriptionManager::new(()),
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

impl<T: 'static + Widget> EventImpl<T::Output> for DynamicWidgetEvent<T> {
    fn subscribe(&self, cb: std::sync::Weak<dyn EventCallback<T::Output>>) {
        self.subscribers
            .add_subscriber(self.weak_self.clone(), None, cb);
    }

    fn height(&self) -> usize {
        self.widget.dynamic.event().0.height() + 1
    }
}

impl<T: 'static + Widget> EventCallback<T> for DynamicWidgetEvent<T> {
    fn event_fired(&self, runtime: &crate::runtime::Runtime, value: Arc<T>) {
        let Some(this) = self.weak_self.upgrade() else {
            // The outer widget's been destroyed lol, guess we can be done.
            return;
        };
        runtime.schedule(self.height(), DWAction { event: this, value });
    }

    fn invalidate_height(&self) {
        // We never cache the height
    }
}

struct DWAction<T: 'static + Widget> {
    event: Arc<DynamicWidgetEvent<T>>,
    value: Arc<T>,
}

impl<T: 'static + Widget> Action for DWAction<T> {
    fn act(self: Box<Self>, runtime: &crate::runtime::Runtime) {
        let parent_node = self.event.node.0.query_for_tag();
        let Some(delegate) = parent_node.delegate.get() else {
            unreachable!();
        };
        let (child_node, output) =
            WidgetBuilder::build_root(runtime, &*self.value, self.event.height());
        child_node.set_delegate(
            runtime,
            delegate.new_child_created(
                WidgetDelegateContext {
                    runtime,
                    node: &parent_node,
                },
                0,
                &child_node,
            ),
        );
        self.event
            .subscribers
            .notify(Arc::downgrade(&self.event), None, runtime, || {
                Some(Arc::new(output))
            });
        let old_node = {
            let mut children = parent_node.children.lock().unwrap();
            std::mem::replace(&mut children[0], child_node)
        };
        old_node.prepare_destruction(runtime);
    }
}

impl<T: 'static + Widget> SubscriptionEvent<T::Output> for DynamicWidgetEvent<T> {
    type Inner = ();

    type Tag = ();

    fn invalidate_height(&self) {
        unreachable!()
    }

    fn handle_main_subscription(
        &self,
        _: &crate::runtime::Runtime,
        _: Arc<Self::Inner>,
        _: Self::Tag,
    ) {
        unreachable!()
    }
}
