//! This is a backend useful for running automated tests.
//!
//! Given a widget, it builds a widget node. You then specify a script of
//! ways to trigger it and give it expectations of what events will fire.
//! It verifies that the test follows the script.

mod backend;
mod delegate;
mod test_value;
pub use test_value::*;
mod script;
pub use script::*;

use crate::backends::test_backend::backend::EventRecord;

#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum StepError {
    #[error("missing dynamic name: {0:?}")]
    MissingDynamicName(String),
    #[error("missing event name: {0:?}")]
    MissingEventName(String),
    #[error("missing event trigger name: {0:?}")]
    MissingTriggerName(String),
    #[error("mismatched event trigger at {path}: {mismatch}")]
    BadEventTrigger { path: Path, mismatch: Mismatch },
    #[error("mismatched event path expected={expected} actual={actual}")]
    BadEventPath { expected: Path, actual: Path },
    #[error("mismatched event at path={path}: {mismatch}")]
    BadEventValue { path: Path, mismatch: Mismatch },
    #[error("mismatched dynamic at path={path}: {mismatch}")]
    BadDynamic { path: Path, mismatch: Mismatch },
    #[error("couldn't find node at path={path}")]
    NoDelegateAtPath { path: Path },
    #[error("couldn't find item at path={path}")]
    NoItemAtPath { path: Path },
    #[error("no event actually took place")]
    EventsDone,
    #[error("unexpected event: {0:?}")]
    ExtraEvents(EventRecord),
}

#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
#[error("step={index}: {error}")]
pub struct ScriptError {
    pub index: usize,
    pub error: StepError,
}
