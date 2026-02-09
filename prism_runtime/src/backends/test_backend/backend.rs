use std::{
    any::Any,
    sync::{Arc, mpsc},
};

use crate::{
    backends::test_backend::{
        Path, ScriptError, StepError, TestAction, TestScript, TestStep, delegate::TestDelegate,
    },
    runtime::Runtime,
    widget::{Widget, WidgetBuilder, WidgetNode},
};

pub struct TestBackend {
    // Backend is responsible for keeping the application root node alive
    node: Arc<WidgetNode>,
    runtime: Runtime,
    delegate: Arc<TestDelegate>,
    event_rx: mpsc::Receiver<EventRecord>,
}

#[derive(Debug)]
pub struct EventRecord {
    pub(super) path: Path,
    pub(super) value: Arc<dyn Any + Send + Sync>,
}

impl TestBackend {
    pub fn build_tree<W: Widget>(widget: &W) -> (W::Output, TestBackend) {
        let runtime = Runtime::new();
        let (node, output) = WidgetBuilder::build_root(&runtime, widget, 0);
        let (event_tx, event_rx) = mpsc::channel();
        // TODO: This should live in `TestDelegate`, not here.
        let delegate = TestDelegate::for_root_node(&node, event_tx);
        node.set_delegate(&runtime, delegate.clone());
        let mut backend = TestBackend {
            node,
            runtime,
            delegate,
            event_rx,
        };

        // Must propagate once for the widget node's initial creation events to land.
        backend.runtime.propagate();
        (output, backend)
    }

    fn find_delegate(&self, path: &Path) -> Result<Arc<TestDelegate>, StepError> {
        let mut delegate = self.delegate.clone();
        for &step in &path.indexes {
            delegate = {
                let children = delegate.children.lock().unwrap();
                let Some(child) = children.get(step) else {
                    return Err(StepError::NoDelegateAtPath { path: path.clone() });
                };
                child.clone()
            };
        }
        Ok(delegate)
    }

    fn step(&self, runtime: &Runtime, step: TestStep) -> Result<(), StepError> {
        match step.action {
            TestAction::Crickets => {
                if let Ok(event) = self.event_rx.try_recv() {
                    return Err(StepError::ExtraEvents(event));
                }
            }
            TestAction::TriggerEvent | TestAction::DelayTriggerEvent => {
                let delegate = self.find_delegate(&step.path)?;
                let trigger = delegate
                    .triggers
                    .get_name(&step.path.name)
                    .ok_or_else(|| StepError::MissingTriggerName(step.path.name.clone()))?;
                if let Err(mismatch) = step.value.trigger(runtime, &trigger) {
                    return Err(StepError::BadEventTrigger {
                        path: step.path,
                        mismatch,
                    });
                }
            }
            TestAction::ExpectEvent => {
                // TODO: Add way to filter events
                let Ok(event) = self.event_rx.try_recv() else {
                    return Err(StepError::EventsDone);
                };
                let EventRecord { path, value } = event;
                if path != step.path {
                    return Err(StepError::BadEventPath {
                        expected: step.path,
                        actual: path,
                    });
                }
                if let Err(mismatch) = step.value.assert_eq(value.clone()) {
                    return Err(StepError::BadEventValue {
                        path: step.path,
                        mismatch,
                    });
                }
            }
            TestAction::ExpectDynamic => {
                let delegate = self.find_delegate(&step.path)?;
                let dynamic = delegate
                    .dynamics
                    .get_name(&step.path.name)
                    .ok_or_else(|| StepError::MissingDynamicName(step.path.name.clone()))?;
                let value = dynamic.as_any_dynamic().behavior().0.query_for_tag();
                if let Err(mismatch) = step.value.assert_eq(value.clone()) {
                    return Err(StepError::BadDynamic {
                        path: step.path,
                        mismatch,
                    });
                }
            }
        }
        Ok(())
    }

    pub fn main_loop(&mut self, script: TestScript) -> Result<(), ScriptError> {
        for (index, step) in script.steps.into_iter().enumerate() {
            log::debug!("{index}: {step:?}");
            let propagate = step.action == TestAction::TriggerEvent;
            self.step(&self.runtime, step)
                .map_err(|error| ScriptError { index, error })?;
            if propagate {
                self.runtime.propagate();
            }
        }
        self.node.prepare_destruction(&self.runtime);
        Ok(())
    }
}
