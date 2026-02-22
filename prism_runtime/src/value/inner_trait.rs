use std::{any::Any, fmt::Debug, sync::Arc};

use crate::value::{AnyValue, AnyValueInner, PrismValue};

pub trait PrismValueInner: Debug + Clone {
    type Ref: ?Sized;
    fn get_any(value: &AnyValue) -> &Self::Ref;
    fn from_any(value: AnyValue) -> Self;
    fn to_any(self) -> AnyValue;
}

impl<T: PrismValueInner> PrismValue for T {}

impl PrismValueInner for i32 {
    type Ref = i32;
    fn get_any(value: &AnyValue) -> &Self {
        let AnyValue(AnyValueInner::I32(value)) = value else {
            panic!();
        };
        value
    }

    fn from_any(value: AnyValue) -> Self {
        let AnyValue(AnyValueInner::I32(value)) = value else {
            panic!();
        };
        value
    }

    fn to_any(self) -> AnyValue {
        AnyValue(AnyValueInner::I32(self))
    }
}

impl PrismValueInner for u32 {
    type Ref = u32;
    fn get_any(value: &AnyValue) -> &Self {
        let AnyValue(AnyValueInner::U32(value)) = value else {
            panic!();
        };
        value
    }

    fn from_any(value: AnyValue) -> Self {
        let AnyValue(AnyValueInner::U32(value)) = value else {
            panic!();
        };
        value
    }

    fn to_any(self) -> AnyValue {
        AnyValue(AnyValueInner::U32(self))
    }
}

/// This trait represents anything that can be stored in a
/// [`Value`] through an [`Arc`].
pub trait HeapValue: Any + Sync + Send + Debug + 'static {}

impl<T: Any + Sync + Send + Debug + PartialEq + 'static> HeapValue for T {}

impl<T> PrismValueInner for Arc<T>
where
    T: HeapValue,
{
    type Ref = T;
    fn get_any(value: &AnyValue) -> &Self::Ref {
        let AnyValue(AnyValueInner::Arc(value)) = value else {
            panic!("value wasn't an Arc");
        };
        let value = &**value as &dyn Any;
        value.downcast_ref().expect("type mismatch")
    }

    fn from_any(value: AnyValue) -> Self {
        let AnyValue(AnyValueInner::Arc(value)) = value else {
            panic!("value wasn't an Arc");
        };
        Arc::downcast(value).expect("type mismatch")
    }

    fn to_any(self) -> AnyValue {
        AnyValue(AnyValueInner::Arc(self.clone()))
    }
}
