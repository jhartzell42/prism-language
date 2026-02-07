use std::sync::Arc;

use crate::{
    event::{Event, EventTrigger},
    runtime::Runtime,
    widget::{
        Widget, WidgetNode,
        erased::{ErasedEvent, ErasedEventTrigger},
        widget_ready::WidgetReadyEvent,
    },
};

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
    pub fn done_event(&self) -> Event<WidgetNode> {
        Event(self.done_event.clone())
    }

    /// Create a new cyclic event. You must close it, or else it will never fire.
    pub fn add_cyclic_event<T: 'static + Send + Sync>(
        &mut self,
        name: String,
    ) -> (usize, Event<T>) {
        let ix = self.node.events.len();
        let event = self
            .done_event()
            .filter_map(move |node| Some(Arc::new(node.events[ix].get::<T>())))
            .switch_hold();
        self.node
            .events
            .add(name, ErasedEvent::new(Event::<T>::never()));
        (ix, event)
    }

    /// Close a cyclic loop by providing the event back. When this event fires, the
    /// original event we got from `new_cyclic_event` will also fire.
    pub fn close_cyclic_event_by_name<T: 'static + Send + Sync>(
        &mut self,
        name: &str,
        event: Event<T>,
    ) {
        assert!(self.node.events[name].matches_inner_type::<T>());
        self.node
            .events
            .update_name(name, ErasedEvent::new(event.clone()))
            .unwrap();
    }

    /// Close a cyclic loop by providing the event back. When this event fires, the
    /// original event we got from `new_cyclic_event` will also fire.
    pub fn close_cyclic_event_by_index<T: 'static + Send + Sync>(
        &mut self,
        index: usize,
        event: Event<T>,
    ) {
        assert!(self.node.events[index].matches_inner_type::<T>());
        self.node
            .events
            .update_index(index, ErasedEvent::new(event.clone()))
            .unwrap();
    }

    /// Add an event that the backend/delegate will have to trigger. This doesn't automatically register the
    /// event as externally accessible, just the trigger.
    pub fn add_external_event<T: 'static + Send + Sync>(
        &mut self,
        name: String,
    ) -> ExternalEventInfo<T> {
        let (event, trigger) = Event::<T>::external();
        let trigger_index = self
            .node
            .triggers
            .add(name.clone(), ErasedEventTrigger::new(trigger.clone()));

        ExternalEventInfo {
            trigger,
            trigger_index,
            event,
        }
    }

    /// Add an externally accessible event, so that the backend can access it.
    pub fn add_event<T: 'static + Send + Sync>(&mut self, name: String, event: Event<T>) {
        self.node.events.add(name, ErasedEvent::new(event));
    }
}

pub struct ExternalEventInfo<T: 'static + Send + Sync> {
    pub trigger: EventTrigger<T>,
    pub event: Event<T>,
    pub trigger_index: usize,
}
