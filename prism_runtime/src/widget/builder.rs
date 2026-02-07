use std::sync::Arc;

use crate::{
    event::{Event, EventTrigger},
    runtime::Runtime,
    widget::{
        Widget, WidgetNode,
        delegate::WidgetDelegate,
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
    pub fn build_root<W: Widget>(
        runtime: &Runtime,
        widget: &W,
        delegate: Arc<dyn WidgetDelegate>,
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
        node.set_delegate(runtime, delegate);
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

    /// Get the `done_event`
    pub fn done_event(&self) -> Event<WidgetNode> {
        Event(self.done_event.clone())
    }

    /// Create a new cyclic event. You must close it, or else it will never fire.
    pub fn add_cyclic_event<T: 'static>(&mut self, name: String) -> (usize, Event<T>) {
        let ix = self.node.events.len();
        let event = self
            .done_event()
            .filter_map(move |node| Some(Arc::new(node.events.get_index(ix).get::<T>())))
            .switch_hold();
        self.node
            .events
            .add(name, ErasedEvent::new(Event::<T>::never()));
        (ix, event)
    }

    /// Close a cyclic loop by providing the event back. When this event fires, the
    /// original event we got from `new_cyclic_event` will also fire.
    pub fn close_cyclic_event_by_name<T: 'static>(&mut self, name: &str, event: Event<T>) {
        assert!(self.node.events.get_name(name).matches_inner_type::<T>());
        self.node
            .events
            .update_name(name, ErasedEvent::new(event.clone()));
    }

    /// Close a cyclic loop by providing the event back. When this event fires, the
    /// original event we got from `new_cyclic_event` will also fire.
    pub fn close_cyclic_event_by_index<T: 'static>(&mut self, index: usize, event: Event<T>) {
        assert!(self.node.events.get_index(index).matches_inner_type::<T>());
        self.node
            .events
            .update_index(index, ErasedEvent::new(event.clone()));
    }

    pub fn add_external_event<T: 'static>(&mut self, name: String) -> ExternalEventInfo<T> {
        let (event, trigger) = Event::<T>::external();
        let trigger_index = self
            .node
            .triggers
            .add(name.clone(), ErasedEventTrigger::new(trigger.clone()));
        let event_index = self.node.events.add(name, ErasedEvent::new(event.clone()));

        ExternalEventInfo {
            trigger,
            trigger_index,
            event,
            event_index,
        }
    }

    pub fn add_event<T: 'static>(&mut self, name: String, event: Event<T>) {
        self.node.events.add(name, ErasedEvent::new(event));
    }
}

pub struct ExternalEventInfo<T: 'static> {
    pub trigger: EventTrigger<T>,
    pub event: Event<T>,
    pub trigger_index: usize,
    pub event_index: usize,
}
