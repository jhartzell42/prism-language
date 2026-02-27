use std::any::type_name;
use std::fmt::Debug;
use std::sync::Arc;

use crate::value::{AnyValue, TypeMismatch, ValueType};
use crate::{runtime::Runtime, widget::AnyEventTrigger};

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
        trigger: &AnyEventTrigger,
    ) -> Result<(), Mismatch>;

    #[allow(missing_docs)]
    fn assert_eq(self: Arc<Self>, other: AnyValue) -> Result<(), Mismatch>;
}

impl<T: ValueType + PartialEq> TestValue for T {
    fn trigger(
        self: Arc<Self>,
        runtime: &Runtime,
        trigger: &AnyEventTrigger,
    ) -> Result<(), Mismatch> {
        let Some(trigger) = trigger.try_get::<T>() else {
            return Err(Mismatch::Type(TypeMismatch {
                expected: type_name::<T>().to_string(),
                actual: trigger.type_name().to_string(),
            }));
        };
        runtime.schedule_trigger(&trigger, Arc::unwrap_or_clone(self).into());
        Ok(())
    }

    fn assert_eq(self: Arc<Self>, other: AnyValue) -> Result<(), Mismatch> {
        let other = other.try_downcast::<T>()?;
        let other = other.get();
        if *self != other {
            return Err(Mismatch::Value {
                expected: self,
                actual: Arc::new(other.clone()),
            });
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Mismatch {
    #[error("expected={expected:?} actual={actual:?}")]
    Value {
        expected: Arc<dyn TestValue>,
        actual: Arc<dyn TestValue>,
    },
    #[error("{0}")]
    Type(#[from] TypeMismatch),
}
