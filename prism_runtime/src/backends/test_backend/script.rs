use std::{
    fmt::{Debug, Display, Formatter},
    sync::Arc,
};

use crate::{
    backends::test_backend::{ScriptError, TestValue, backend::TestBackend},
    widget::Widget,
};

/// Create a new script from an array of steps
pub fn script<const N: usize>(steps: [TestStep; N]) -> TestScript {
    TestScript {
        steps: steps.into(),
    }
}

/// Create a new event trigger, propagate immediately
pub fn trigger<T: TestValue, const N: usize>(
    node_path: [usize; N],
    name: &str,
    test_value: T,
) -> TestStep {
    TestStep {
        path: Path {
            indexes: node_path.into(),
            name: name.into(),
        },
        action: TestAction::TriggerEvent,
        value: Arc::new(test_value),
    }
}

/// Create a new event trigger, don't propagate yet
pub fn delay_trigger<T: TestValue, const N: usize>(
    node_path: [usize; N],
    name: &str,
    test_value: T,
) -> TestStep {
    TestStep {
        path: Path {
            indexes: node_path.into(),
            name: name.into(),
        },
        action: TestAction::DelayTriggerEvent,
        value: Arc::new(test_value),
    }
}

/// Indicate that no event will fire at all
pub fn crickets() -> TestStep {
    TestStep {
        path: Path {
            indexes: vec![],
            name: "".to_string(),
        },
        action: TestAction::Crickets,
        value: Arc::new(()),
    }
}

/// Indicate that an event will fire
pub fn event<T: TestValue, const N: usize>(
    node_path: [usize; N],
    name: &str,
    test_value: T,
) -> TestStep {
    TestStep {
        path: Path {
            indexes: node_path.into(),
            name: name.into(),
        },
        action: TestAction::ExpectEvent,
        value: Arc::new(test_value),
    }
}

/// Indicate that a dynamic has an expected value
pub fn dynamic<T: TestValue, const N: usize>(
    node_path: [usize; N],
    name: &str,
    test_value: T,
) -> TestStep {
    TestStep {
        path: Path {
            indexes: node_path.into(),
            name: name.into(),
        },
        action: TestAction::ExpectDynamic,
        value: Arc::new(test_value),
    }
}

/// This is the script to run to test the widget.
pub struct TestScript {
    /// These are the individual steps that we're testing.
    pub steps: Vec<TestStep>,
}

/// This is an individual step in testing the widget.
#[derive(Debug)]
pub struct TestStep {
    /// Where to find the node that we're testing in this step
    pub path: Path,
    /// What action to perform, either trigger an event, verify an event that's fired, or validate a dynamic.
    pub action: TestAction,
    /// What value to use as an argument to that action.
    pub value: Arc<dyn TestValue>,
}

/// What action to take.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TestAction {
    /// Trigger a new event, which will propagate immediately
    TriggerEvent,
    /// Trigger a new event, which will propagate at the next full trigger
    DelayTriggerEvent,
    /// Verify the next event off the queue.
    ExpectEvent,
    /// Query a dynamic to make sure it has the value we expect
    ExpectDynamic,
    /// No events should be present on the queue
    Crickets,
}

/// Where the actual event/dynamic/trigger is in the [`WidgetNode`] structure.
#[derive(Clone, PartialEq)]
pub struct Path {
    /// How to navigate from the root node.
    pub indexes: Vec<usize>,
    /// Which value to zoom in on once we're there.
    /// This might be an event name, a dynamic name, or a trigger name.
    pub name: String,
}

impl Debug for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Self { indexes, name } = self;
        write!(f, "{indexes:?}.{name}")
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Self { indexes, name } = self;
        write!(f, "{indexes:?}.{name}")
    }
}

impl TestScript {
    /// Run this test script on the following widget.
    pub fn run<W: Widget>(self, widget: &W) -> Result<W::Output, ScriptError> {
        let (output, mut backend) = TestBackend::build_tree(widget);
        backend.main_loop(self)?;
        Ok(output)
    }
}
