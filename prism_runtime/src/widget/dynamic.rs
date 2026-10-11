use std::fmt::Debug;
use std::sync::{Arc, Mutex, Weak};

use crate::value::{PrismArc, Value};
use crate::widget::widget_ready::WidgetReadyEvent;
use crate::{
    dynamic::Dynamic,
    event::{
        Event, EventCallback, EventImpl,
        subscriber_list::{SubscriptionEvent, SubscriptionManager},
    },
    runtime::{Action, Runtime},
    widget::{Blueprint, WidgetDelegateContext, Widget, builder::WidgetBuilder},
};

impl<T: Blueprint + 'static + Send + Sync + Debug> Dynamic<Arc<T>> {
    /// Create a dynamic widget that starts out by building current value of `self`,
    /// and then replaces it with whatever widget `self` updates to.
    pub fn dynamic_widget(&self) -> impl Blueprint<Output = Arc<Event<Arc<T::Output>>>> {
        DynamicBlueprint {
            dynamic: self.clone(),
        }
    }
}

struct DynamicBlueprint<T: 'static + Blueprint + Send + Sync + Debug> {
    dynamic: Dynamic<Arc<T>>,
}

struct DynamicWidgetEvent<T: 'static + Blueprint + Send + Sync + Debug> {
    widget: DynamicBlueprint<T>,
    weak_self: Weak<Self>,
    ready: Arc<WidgetReadyEvent>,
    subscribers: SubscriptionManager<Arc<T::Output>, Self>,
}

impl<T: 'static + Blueprint + Debug + Send + Sync> Clone for DynamicBlueprint<T> {
    fn clone(&self) -> Self {
        Self {
            dynamic: self.dynamic.clone(),
        }
    }
}

impl<T: 'static + Blueprint + Debug + Send + Sync> Blueprint for DynamicBlueprint<T> {
    type Output = Arc<Event<Arc<T::Output>>>;
    fn build(&self, builder: &mut WidgetBuilder) -> Self::Output {
        let next_event = Arc::new_cyclic(|weak_self| DynamicWidgetEvent {
            widget: self.clone(),
            weak_self: weak_self.clone(),
            ready: builder.done_event.clone(),
            subscribers: SubscriptionManager::new(()),
        });
        let initial_widget = self
            .dynamic
            .event()
            .0
            .subscribe(builder.runtime, Arc::downgrade(
                &(next_event.clone() as Arc<dyn EventCallback<Arc<T>>>),
            ))
            .unwrap_or_else(|| self.dynamic.behavior().0.query_for_tag());

        // `builder.done_event()` will only fire once, so we can use this hack
        let output = Mutex::new(Some(builder.bind(&*initial_widget)));
        let first_time_event = builder.done_event().filter_map(move |_| {
            let mut output = output.lock().unwrap();
            let output = output.take().unwrap();
            Some(PrismArc::new(output))
        });
        let next_event = Event(next_event);
        // Keep this alive even if no one's using the output event.
        // This fires before the delegate is attached. So, from the delegate's POV, it never fires.
        builder.add_public_event("__rebuilt__".to_string(), next_event.clone());

        let event = Event::leftmost(vec![next_event, first_time_event]);
        Arc::new(event)
    }
}

impl<T: 'static + Blueprint + Debug> EventImpl<Arc<T::Output>> for DynamicWidgetEvent<T> {
    fn subscribe(
        &self,
        runtime: &Runtime,
        cb: std::sync::Weak<dyn EventCallback<Arc<T::Output>>>,
    ) -> Option<Value<Arc<T::Output>>> {
        self.subscribers
            .add_subscriber(runtime, self.weak_self.clone(), None, cb)
    }

    fn height(&self) -> usize {
        self.widget.dynamic.event().0.height() + 1
    }
}

impl<T: 'static + Blueprint + Debug> EventCallback<Arc<T>> for DynamicWidgetEvent<T> {
    fn event_fired(&self, runtime: &crate::runtime::Runtime, value: PrismArc<T>) {
        let Some(this) = self.weak_self.upgrade() else {
            // The outer widget's been destroyed lol, guess we can be done.
            return;
        };
        let height = self.height();
        runtime.schedule(
            height,
            DWAction {
                event: this,
                value: value.get(),
                height,
            },
        );
    }

    fn already_fired(&self, runtime: &Runtime, value: PrismArc<T>) {
        let Some(this) = self.weak_self.upgrade() else {
            return;
        };
        let height = runtime
            .current_height()
            .map_or(self.height(), |current| self.height().max(current + 1));
        runtime.schedule(
            height,
            DWAction {
                event: this,
                value: value.get(),
                height,
            },
        );
    }

    fn invalidate_height(&self) {
        for subscriber in self.subscribers.consolidate() {
            subscriber.invalidate_height();
        }
    }
}

struct DWAction<T: 'static + Blueprint + Debug> {
    event: Arc<DynamicWidgetEvent<T>>,
    value: Arc<T>,
    height: usize,
}

impl<T: 'static + Blueprint + Debug> Action for DWAction<T> {
    fn act(self: Box<Self>, runtime: &Runtime) {
        let parent_node = self.event.ready.node.get().unwrap().clone();
        let (child_node, output) =
            WidgetBuilder::build_root(runtime, &*self.value, self.height + 1);
        if let Some(delegate) = parent_node.delegate.get() {
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
        }
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
        runtime.schedule(usize::MAX, KeepUntilOccurrenceEnds(old_node));
    }
}

struct KeepUntilOccurrenceEnds(Arc<Widget>);

impl Action for KeepUntilOccurrenceEnds {
    fn act(self: Box<Self>, _: &Runtime) {
        drop(self.0);
    }
}

impl<T: 'static + Blueprint + Debug> SubscriptionEvent<Arc<T::Output>> for DynamicWidgetEvent<T> {
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

    fn handle_already_fired(&self, _: &Runtime, _: Value<Self::Inner>, _: Self::Tag) {
        unreachable!()
    }
}
