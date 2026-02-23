use std::fmt::Debug;
use std::sync::{Arc, Mutex, Weak};

use crate::value::{PrismArc, Value};
use crate::{
    behavior::Behavior,
    dynamic::Dynamic,
    event::{
        Event, EventCallback, EventImpl,
        subscriber_list::{SubscriptionEvent, SubscriptionManager},
    },
    runtime::{Action, Runtime},
    widget::{Widget, WidgetDelegateContext, WidgetNode, builder::WidgetBuilder},
};

impl<T: Widget + 'static + Send + Sync + Debug> Dynamic<Arc<T>> {
    /// Create a dynamic widget that starts out by building current value of `self`,
    /// and then replaces it with whatever widget `self` updates to.
    pub fn dynamic_widget(&self) -> impl Widget<Output = Arc<Event<Arc<T::Output>>>> {
        DynamicWidget {
            dynamic: self.clone(),
        }
    }
}

struct DynamicWidget<T: 'static + Widget + Send + Sync + Debug> {
    dynamic: Dynamic<Arc<T>>,
}

struct DynamicWidgetEvent<T: 'static + Widget + Send + Sync + Debug> {
    widget: DynamicWidget<T>,
    weak_self: Weak<Self>,
    node: Behavior<Arc<Option<Arc<WidgetNode>>>>,
    subscribers: SubscriptionManager<Arc<T::Output>, Self>,
}

impl<T: 'static + Widget + Debug + Send + Sync> Clone for DynamicWidget<T> {
    fn clone(&self) -> Self {
        Self {
            dynamic: self.dynamic.clone(),
        }
    }
}

impl<T: 'static + Widget + Debug + Send + Sync> Widget for DynamicWidget<T> {
    type Output = Arc<Event<Arc<T::Output>>>;
    fn build(&self, builder: &mut WidgetBuilder) -> Self::Output {
        let initial_widget = self.dynamic.behavior().0.query_for_tag();

        // `builder.done_event()` will only fire once, so we can use this hack
        let done_event = builder.done_event();
        let output = Mutex::new(Some(builder.bind(&*initial_widget)));
        let first_time_event = builder.done_event().filter_map(move |_| {
            let mut output = output.lock().unwrap();
            let output = output.take().unwrap();
            Some(PrismArc::new(output))
        });
        let next_event = Arc::new_cyclic(|weak_self| DynamicWidgetEvent {
            widget: self.clone(),
            weak_self: weak_self.clone(),
            node: Behavior::hold(
                PrismArc::new(None),
                done_event.filter_map(|x| {
                    let x = x.get();
                    Some(PrismArc::new(Some(x)))
                }),
            ),
            subscribers: SubscriptionManager::new(()),
        });
        self.dynamic.event().0.subscribe(Arc::downgrade(
            &(next_event.clone() as Arc<dyn EventCallback<Arc<T>>>),
        ));
        let next_event = Event(next_event);
        // Keep this alive even if no one's using the output event.
        // This fires before the delegate is attached. So, from the delegate's POV, it never fires.
        builder.add_public_event("__rebuilt__".to_string(), next_event.clone());

        let event = Event::leftmost(vec![first_time_event, next_event]);
        Arc::new(event)
    }
}

impl<T: 'static + Widget + Debug> EventImpl<Arc<T::Output>> for DynamicWidgetEvent<T> {
    fn subscribe(&self, cb: std::sync::Weak<dyn EventCallback<Arc<T::Output>>>) {
        self.subscribers
            .add_subscriber(self.weak_self.clone(), None, cb);
    }

    fn height(&self) -> usize {
        self.widget.dynamic.event().0.height() + 1
    }
}

impl<T: 'static + Widget + Debug> EventCallback<Arc<T>> for DynamicWidgetEvent<T> {
    fn event_fired(&self, runtime: &crate::runtime::Runtime, value: PrismArc<T>) {
        let Some(this) = self.weak_self.upgrade() else {
            // The outer widget's been destroyed lol, guess we can be done.
            return;
        };
        runtime.schedule(
            self.height(),
            DWAction {
                event: this,
                value: value.get(),
            },
        );
    }

    fn invalidate_height(&self) {
        // We never cache the height
    }
}

struct DWAction<T: 'static + Widget + Debug> {
    event: Arc<DynamicWidgetEvent<T>>,
    value: Arc<T>,
}

impl<T: 'static + Widget + Debug> Action for DWAction<T> {
    fn act(self: Box<Self>, runtime: &Runtime) {
        let Some(parent_node) = self.event.node.0.query_for_tag().extract() else {
            unreachable!("parent_node not set");
        };
        let Some(delegate) = parent_node.delegate.get() else {
            unreachable!("parent node has no delegate?");
        };
        let (child_node, output) =
            WidgetBuilder::build_root(runtime, &*self.value, self.event.height() + 1);
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
                Some(PrismArc::new(output))
            });
        let old_node = {
            let mut children = parent_node.children.lock().unwrap();
            std::mem::replace(&mut children[0], child_node)
        };
        old_node.prepare_destruction(runtime);
    }
}

impl<T: 'static + Widget + Debug> SubscriptionEvent<Arc<T::Output>> for DynamicWidgetEvent<T> {
    type Inner = ();

    type Tag = ();

    fn invalidate_height(&self) {
        unreachable!()
    }

    fn handle_main_subscription(
        &self,
        _: &crate::runtime::Runtime,
        _: Value<Self::Inner>,
        _: Self::Tag,
    ) {
        unreachable!()
    }
}
