use std::{
    marker::PhantomData,
    sync::{Arc, OnceLock},
};

use crate::{
    behavior::{Behavior, BehaviorComputation, BehaviorDependencyTracker},
    dynamic::Dynamic,
    event::Event,
    runtime::Runtime,
    value::{PrismArc, Value, ValueType},
    widget::{
        AnyDynamic, Widget, WidgetNode,
        any::{AnyEvent, AnyEventTrigger},
        widget_ready::WidgetReadyEvent,
    },
};

/// Passed into widgets to allow them to create subwidgets, cyclic events and dynamics,
/// and publicly exposed material.
pub struct WidgetBuilder<'a> {
    node: WidgetNode,
    children: Vec<Arc<WidgetNode>>,
    done_event: Arc<WidgetReadyEvent>,
    runtime: &'a Runtime,
}

impl WidgetBuilder<'_> {
    /// Call from outside (e.g. a backend) to create a root widget.
    /// This is used at the top level to build the initial tree with `height = 0`.
    ///
    /// Call from a rebuilding primitive (like `dynamic_widget` or `dynamic_list_widget`).
    /// In that case, it'll be in an event processing context, and the height will
    /// be the height of the input event.
    ///
    /// This is used for dynamic reconstruction of widgets in response to an event,
    /// where `height` is the height of the event that triggered this reconstruction.
    /// All widget creation events fire towards the end of this method. They will
    /// fire at a height 1 higher than the height you pass.
    ///
    /// The caller is responsible for setting the delegate.
    pub fn build_root<W: Widget>(
        runtime: &Runtime,
        widget: &W,
        height: usize,
    ) -> (Arc<WidgetNode>, W::Output) {
        let (done_event, trigger) = WidgetReadyEvent::new_with_trigger(height + 1);
        let mut this = WidgetBuilder {
            node: WidgetNode::new(),
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
    pub fn bind<W: Widget>(&mut self, widget: &W) -> W::Output {
        let mut sub = WidgetBuilder {
            node: WidgetNode::new(),
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

    /// This event fires and provides the `WidgetNode` when it's actually done being built.
    pub fn done_event(&self) -> Event<Arc<WidgetNode>> {
        Event(self.done_event.clone())
    }

    /// Add cyclic dynamic
    pub fn add_cyclic_dynamic<T: ValueType>(&mut self, name: String) -> (usize, Dynamic<T>) {
        let lock = Arc::new(OnceLock::new());
        let ix = self.node.cyclic_dynamics.add(name, lock.clone());

        struct Computation<T: 'static + Send + Sync> {
            lock: Arc<OnceLock<AnyDynamic>>,
            phantom: PhantomData<T>,
        }

        impl<T: ValueType> BehaviorComputation<T> for Computation<T> {
            fn compute(&self, dep: BehaviorDependencyTracker) -> Value<T> {
                let d = self.lock.get().expect("cyclic dynamic never closed");
                let d = d.get::<T>();
                d.behavior().query_for_computation(dep)
            }
        }

        let behavior = Behavior::computation_behavior(Computation {
            lock,
            phantom: PhantomData,
        });
        let event = self
            .done_event()
            .filter_map(move |node| {
                Some(
                    Arc::new(
                        node.get().cyclic_dynamics[ix]
                            .get()
                            .expect("cyclic dynamic never closed")
                            .get::<T>()
                            .event(),
                    )
                    .into(),
                )
            })
            .switch_hold();
        (ix, unsafe { Dynamic::new(behavior, event) })
    }

    /// Close a cyclic dynamic by providing the underlying dynamic.
    pub fn close_cyclic_dynamic_by_index<T: ValueType>(&mut self, ix: usize, dynamic: Dynamic<T>) {
        self.node.cyclic_dynamics[ix]
            .set(AnyDynamic::new(dynamic))
            .expect("set cyclic dynamic twice");
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
        let ix = self.node.cyclic_events.len();
        let event = self
            .done_event()
            .filter_map(move |node| Some(PrismArc::new(node.get().cyclic_events[ix].get::<T>())))
            .switch_hold();
        self.node
            .cyclic_events
            .add(name, AnyEvent::new(Event::<T>::never()));
        (ix, event)
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
        assert!(self.node.cyclic_events[index].matches_inner_type::<T>());
        self.node
            .cyclic_events
            .update_index(index, AnyEvent::new(event.clone()))
            .unwrap();
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
