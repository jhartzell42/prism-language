use std::any::{type_name, type_name_of_val};
use std::fmt::Debug;
use std::{any::Any, sync::Arc};

use crate::{runtime::Runtime, widget::ErasedEventTrigger};

/// Almost all values implement this trait. Requires `'static`, [`Eq`], [`Debug`], [`Sync`], and [`Send`]
/// and it's automatically implemented. Required for using with [`test_backend`]. You don't
/// need to implement this yourself, nor call the methods.
///
/// [`test_backend`]: crate::backends::test_backend
pub trait TestValue: Debug + Send + Sync + 'static {
    #[allow(missing_docs)]
    fn trigger(
        self: Arc<Self>,
        runtime: &Runtime,
        trigger: &ErasedEventTrigger,
    ) -> Result<(), Mismatch>;

    #[allow(missing_docs)]
    fn assert_eq(self: Arc<Self>, other: Arc<dyn Any + Send + Sync>) -> Result<(), Mismatch>;
}

impl<T: Debug + Eq + 'static + Send + Sync> TestValue for T {
    fn trigger(
        self: Arc<Self>,
        runtime: &Runtime,
        trigger: &ErasedEventTrigger,
    ) -> Result<(), Mismatch> {
        let Some(trigger) = trigger.try_get::<T>() else {
            return Err(Mismatch::Type {
                expected: type_name::<T>().to_string(),
                actual: trigger.type_name().to_string(),
            });
        };
        runtime.schedule_trigger(&trigger, self);
        Ok(())
    }

    fn assert_eq(self: Arc<Self>, other: Arc<dyn Any + Send + Sync>) -> Result<(), Mismatch> {
        let other = match other.downcast::<T>() {
            Err(actual) => {
                return Err(Mismatch::Type {
                    expected: type_name::<T>().to_string(),
                    actual: type_name_of_val(&actual).to_string(),
                });
            }
            Ok(other) => other,
        };
        if self != other {
            return Err(Mismatch::Value {
                expected: self,
                actual: other,
            });
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Mismatch {
    #[error("value expected={expected:?} actual={actual:?}")]
    Value {
        expected: Arc<dyn TestValue>,
        actual: Arc<dyn TestValue>,
    },
    #[error("type expected={expected:?} actual={actual:?}")]
    Type { expected: String, actual: String },
}
