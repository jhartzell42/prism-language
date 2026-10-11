use std::sync::{Arc, OnceLock, Weak};

use crate::{
    behavior::{Behavior, BehaviorComputation, BehaviorDependencyTracker},
    dynamic::Dynamic,
    event::{Event, forward::ForwardEvent},
    runtime::Runtime,
    value::{Value, ValueType},
    widget::{
        AnyDynamic, Blueprint, Widget,
        any::{AnyEvent, AnyEventTrigger},
        widget_ready::WidgetReadyEvent,
    },
};

/// Passed into widgets to allow them to create subwidgets, cyclic events and dynamics,
/// and publicly exposed material.
pub struct WidgetBuilder<'a> {
    node: Widget,
    children: Vec<Arc<Widget>>,
    pub(super) done_event: Arc<WidgetReadyEvent>,
    /// The runtime this widget is being built in: the occurrence a hold or
    /// switch made during the build is created in.
    pub(crate) runtime: &'a Runtime,
}

impl WidgetBuilder<'_> {
    /// Call from outside (e.g. a backend) to create a root widget.
    /// This is used at the top level to build the initial tree with `height = 0`.
    ///
    /// Call from a rebuilding primitive (like `rebuilding` or `dynamic_list_widget`).
    /// In that case, it'll be in an event processing context, and the height will
    /// be the height of the input event.
    ///
    /// This is used for dynamic reconstruction of widgets in response to an event,
    /// where `height` is the height of the event that triggered this reconstruction.
    /// All widget creation events fire towards the end of this method. They will
    /// fire at a height 1 higher than the height you pass.
    ///
    /// The caller is responsible for setting the delegate.
    pub fn build_root<W: Blueprint>(
        runtime: &Runtime,
        widget: &W,
        height: usize,
    ) -> (Arc<Widget>, W::Output) {
        let (done_event, trigger) = WidgetReadyEvent::new_with_trigger(height + 1);
        let mut this = WidgetBuilder {
            node: Widget::new(),
            children: vec![],
            done_event,
            runtime,
        };
        let res = widget.build(&mut this);
        let node = Arc::new(this.node);
        this.done_event.set_node(node.clone());
        {
            let mut node_children = node.children.lock().unwrap();
            *node_children = this.children;
        }
        runtime.schedule(height, trigger);
        (node, res)
    }

    /// Call from a widget to create a subwidget.
    pub fn bind<W: Blueprint>(&mut self, widget: &W) -> W::Output {
        let mut sub = WidgetBuilder {
            node: Widget::new(),
            children: vec![],
            done_event: WidgetReadyEvent::new(self.done_event.clone()),
            runtime: self.runtime,
        };
        let res = widget.build(&mut sub);
        let node = Arc::new(sub.node);
        sub.done_event.set_node(node.clone());
        {
            let mut node_children = node.children.lock().unwrap();
            *node_children = sub.children;
        }
        self.children.push(node);
        res
    }

    /// This event fires and provides the `Widget` when it's actually done being built.
    pub fn done_event(&self) -> Event<Arc<Widget>> {
        Event(self.done_event.clone())
    }

    /// Add cyclic dynamic
    pub fn add_cyclic_dynamic<T: ValueType>(&mut self, name: String) -> (usize, Dynamic<T>) {
        let forward = ForwardEvent::<T>::new();
        let slot = Arc::new((OnceLock::<Dynamic<T>>::new(), forward.clone()));
        let ix = self.node.cyclic_dynamics.add(name, slot.clone());

        struct Computation<T: ValueType> {
            slot: Weak<(OnceLock<Dynamic<T>>, Arc<ForwardEvent<T>>)>,
        }

        impl<T: ValueType> BehaviorComputation<T> for Computation<T> {
            fn compute(&self, dep: BehaviorDependencyTracker) -> Value<T> {
                let slot = self
                    .slot
                    .upgrade()
                    .expect("cyclic dynamic sampled after its widget was dropped");
                let d = slot.0.get().expect("cyclic dynamic never closed");
                d.behavior().query_for_computation(dep)
            }
        }

        let behavior = Behavior::computation_behavior(Computation {
            slot: Arc::downgrade(&slot),
        });
        (ix, unsafe { Dynamic::new(behavior, Event(forward)) })
    }

    /// Close a cyclic dynamic by providing the underlying dynamic.
    pub fn close_cyclic_dynamic_by_index<T: ValueType>(&mut self, ix: usize, dynamic: Dynamic<T>) {
        let (lock, forward) = self.node.cyclic_dynamics[ix]
            .downcast_ref::<(OnceLock<Dynamic<T>>, Arc<ForwardEvent<T>>)>()
            .expect("cyclic dynamic closed with the wrong type");
        forward.close(self.runtime, &dynamic.event());
        lock.set(dynamic).expect("set cyclic dynamic twice");
    }

    /// Close a cyclic dynamic by providing the underlying dynamic.
    pub fn close_cyclic_dynamic_by_name<T: ValueType>(&mut self, name: &str, dynamic: Dynamic<T>) {
        self.close_cyclic_dynamic_by_index(
            self.node.cyclic_dynamics.index_for_name(name).unwrap(),
            dynamic,
        );
    }

    /// Create a new cyclic event. You must close it, or else it will never fire.
    pub fn add_cyclic_event<T: ValueType>(&mut self, name: String) -> (usize, Event<T>) {
        let forward = ForwardEvent::<T>::new();
        let ix = self
            .node
            .cyclic_events
            .add(name, Arc::new((forward.clone(), OnceLock::<Event<T>>::new())));
        (ix, Event(forward))
    }

    /// Close a cyclic loop by providing the event back, given the event's name. When this event fires, the
    /// original event we got from [`add_cyclic_event()`] will also fire.
    ///
    /// [`add_cyclic_event()`]: Self::add_cyclic_event()
    pub fn close_cyclic_event_by_name<T: ValueType>(&mut self, name: &str, event: Event<T>) {
        self.close_cyclic_event_by_index(
            self.node.cyclic_events.index_for_name(name).expect(name),
            event,
        );
    }

    /// Close a cyclic loop by providing the event back, given the event's index. When this event fires, the
    /// original event we got from [`add_cyclic_event()`] will also fire.
    ///
    /// [`add_cyclic_event()`]: Self::add_cyclic_event()
    pub fn close_cyclic_event_by_index<T: ValueType>(&mut self, index: usize, event: Event<T>) {
        let (forward, target) = self.node.cyclic_events[index]
            .downcast_ref::<(Arc<ForwardEvent<T>>, OnceLock<Event<T>>)>()
            .expect("cyclic event closed with the wrong type");
        forward.close(self.runtime, &event);
        target.set(event).expect("cyclic event closed twice");
    }

    /// Add an event that the backend/delegate will have to trigger. This doesn't automatically register the
    /// event as externally accessible, just the trigger.
    pub fn add_external_event<T: ValueType>(&mut self, name: String) -> Event<T> {
        let (event, trigger) = Event::<T>::external();
        self.node
            .triggers
            .add(name.clone(), AnyEventTrigger::new(trigger.clone()));

        event
    }

    /// Add an externally accessible event, so that the backend can access it.
    pub fn add_public_event<T: ValueType>(&mut self, name: String, event: Event<T>) {
        self.node.public_events.add(name, AnyEvent::new(event));
    }

    /// Add an externally accessible dynamic, so that the backend can access it.
    pub fn add_public_dynamic<T: ValueType>(&mut self, name: String, dynamic: Dynamic<T>) {
        self.node
            .public_dynamics
            .add(name, AnyDynamic::new(dynamic));
    }
}
