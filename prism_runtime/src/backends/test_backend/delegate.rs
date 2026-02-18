use std::{
    any::Any,
    sync::{Arc, Mutex, mpsc},
};

use crate::{
    backends::test_backend::{Path, backend::EventRecord},
    widget::{ErasedDynamic, ErasedEventTrigger, Slots, WidgetDelegate, WidgetNode},
};

pub(crate) struct TestDelegate {
    pub(crate) path: Vec<usize>,
    // These must be kept alive by the recipient.
    pub(crate) dynamics: Slots<ErasedDynamic>,
    pub(crate) triggers: Slots<ErasedEventTrigger>,
    pub(crate) event_tx: mpsc::Sender<EventRecord>,
    pub(crate) children: Mutex<Vec<Arc<TestDelegate>>>,
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
        Arc::new(TestDelegate {
            path,
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

    fn event_fired(&self, s: &str, value: Arc<dyn Any + Send + Sync + 'static>) {
        let _ = self.event_tx.send(EventRecord {
            path: Path {
                indexes: self.path.clone(),
                name: s.into(),
            },
            value,
        });
    }
}
