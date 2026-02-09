use std::{
    any::Any,
    sync::{Arc, Mutex, mpsc},
};

use crate::{
    backends::test_backend::{Path, backend::EventRecord},
    event::EventCallback,
    runtime::Runtime,
    widget::{ErasedDynamic, ErasedEventTrigger, Slots, WidgetDelegate, WidgetNode},
};

pub(crate) struct TestDelegate {
    pub(crate) path: Vec<usize>,
    // These must be kept alive by the recipient.
    pub(crate) _event_callbacks: Vec<Arc<TestCallback>>,
    pub(crate) dynamics: Slots<ErasedDynamic>,
    pub(crate) triggers: Slots<ErasedEventTrigger>,
    pub(crate) event_tx: mpsc::Sender<EventRecord>,
    pub(crate) children: Mutex<Vec<Arc<TestDelegate>>>,
}

pub(crate) struct TestCallback {
    path: Path,
    event_tx: mpsc::Sender<EventRecord>,
}

impl TestDelegate {
    pub(super) fn for_root_node(
        node: &WidgetNode,
        event_tx: mpsc::Sender<EventRecord>,
    ) -> Arc<TestDelegate> {
        Self::for_node(vec![], node, event_tx)
    }

    pub(super) fn for_child(&self, ix: usize, child: &WidgetNode) -> Arc<TestDelegate> {
        let path = {
            let mut path = self.path.clone();
            path.push(ix);
            path
        };
        let delegate = Self::for_node(path, child, self.event_tx.clone());
        {
            let mut siblings = self.children.lock().unwrap();
            if siblings.len() == ix {
                siblings.push(delegate.clone());
            } else {
                siblings[ix] = delegate.clone();
            }
        }
        delegate
    }

    fn for_node(
        path: Vec<usize>,
        node: &WidgetNode,
        event_tx: mpsc::Sender<EventRecord>,
    ) -> Arc<TestDelegate> {
        let event_callbacks = {
            let mut cbs = vec![];
            for (name, &ix) in &node.public_events.names {
                let cb = Arc::new(TestCallback {
                    path: Path {
                        indexes: path.clone(),
                        name: name.clone(),
                    },
                    event_tx: event_tx.clone(),
                });
                node.public_events.values[ix]
                    .as_any_event()
                    .0
                    .subscribe(Arc::downgrade(
                        &(cb.clone() as Arc<dyn EventCallback<dyn Any + Send + Sync>>),
                    ));
                cbs.push(cb);
            }
            cbs
        };
        Arc::new(TestDelegate {
            path,
            _event_callbacks: event_callbacks,
            dynamics: node.public_dynamics.clone(),
            triggers: node.triggers.clone(),
            event_tx,
            children: Mutex::new(vec![]),
        })
    }
}

impl WidgetDelegate for TestDelegate {
    fn new_child_created(
        &self,
        _: crate::widget::WidgetDelegateContext,
        ix: usize,
        child: &WidgetNode,
    ) -> Arc<dyn WidgetDelegate> {
        self.for_child(ix, child)
    }

    fn will_be_destroyed(&self, _: crate::widget::WidgetDelegateContext) {
        // It's fine.
    }
}

impl EventCallback<dyn Any + Send + Sync> for TestCallback {
    fn event_fired(&self, _: &Runtime, value: Arc<dyn Any + Send + Sync>) {
        let _ = self.event_tx.send(EventRecord {
            path: self.path.clone(),
            value,
        });
    }

    fn invalidate_height(&self) {
        // Wow, we so don't care.
    }
}
