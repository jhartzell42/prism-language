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
    pub fn build_root<T: 'static>(
        runtime: &Runtime,
        widget: impl Widget<T>,
        delegate: Arc<dyn WidgetDelegate>,
    ) -> (Arc<WidgetNode>, T) {
        let (mut done_event, trigger) = WidgetReadyEvent::new_with_trigger(0);
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
        runtime.schedule(0, trigger);
        node.set_delegate(runtime, delegate);
        (node, res)
    }

    /// Call from a widget to create a subwidget.
    pub fn bind<T: 'static>(&mut self, widget: impl Widget<T>) -> T {
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

    /// Call externally to a widget to build a widget node.
    /// This is used at the top level to build the initial tree with `height = 0`.
    /// This is used for dynamic reconstruction of widgets in response to an event,
    /// where `height` is the height of the event that triggered this reconstruction.
    /// All widget creation events fire towards the end of this method.
    pub fn from_scratch<T: 'static>(
        runtime: &Runtime,
        widget: impl Widget<T>,
        height: usize,
    ) -> (Arc<T>, Arc<WidgetNode>) {
        let (done_event, trigger) = WidgetReadyEvent::new_with_trigger(height + 1);
        let mut builder = WidgetBuilder {
            node: WidgetNode::new(),
            children: vec![],
            done_event: done_event.clone(),
            runtime,
        };
        let res = widget.build(&mut builder);
        let node = Arc::new(builder.node);
        done_event.set_node(node.clone());
        {
            let mut node_children = node.children.lock().unwrap();
            *node_children = builder.children;
        }
        trigger.trigger(runtime);
        (Arc::new(res), node)
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
}

struct ExternalEventInfo<T> {
    trigger: EventTrigger<T>,
    event: Event<T>,
    trigger_index: usize,
    event_index: usize,
}
