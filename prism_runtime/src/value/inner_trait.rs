use std::{
    any::{Any, type_name},
    convert::Infallible,
    fmt::Debug,
    sync::Arc,
};

use crate::value::{AnyValue, AnyValueInner, TypeMismatch, ValueType};

pub trait PrismValueInner: Debug + Clone + Sync + Send + 'static {
    type Ref: ?Sized;
    fn get_any(value: &AnyValue) -> &Self::Ref {
        Self::try_get_any(value).unwrap()
    }
    fn try_get_any(value: &AnyValue) -> Result<&Self::Ref, TypeMismatch>;
    fn from_any(value: AnyValue) -> Self;
    fn to_any(self) -> AnyValue;

    fn type_string() -> String;
    fn fail(value: &AnyValue) -> Result<Infallible, TypeMismatch> {
        Err(TypeMismatch {
            expected: Self::type_string(),
            actual: value.0.type_string(),
        })
    }
}

impl<T: PrismValueInner> ValueType for T {}

impl PrismValueInner for () {
    type Ref = ();

    fn type_string() -> String {
        "()".into()
    }

    fn try_get_any(value: &AnyValue) -> Result<&Self::Ref, TypeMismatch> {
        let AnyValue(AnyValueInner::Unit) = value else {
            match Self::fail(value)? {}
        };
        Ok(&())
    }

    fn from_any(value: AnyValue) -> Self {
        let AnyValue(AnyValueInner::Unit) = value else {
            panic!("expected unit")
        };
        ()
    }

    fn to_any(self) -> AnyValue {
        AnyValue(AnyValueInner::Unit)
    }
}

impl PrismValueInner for AnyValue {
    type Ref = AnyValue;

    fn type_string() -> String {
        "AnyValue".into()
    }

    fn try_get_any(value: &AnyValue) -> Result<&Self::Ref, TypeMismatch> {
        Ok(value)
    }

    fn from_any(value: AnyValue) -> Self {
        value
    }

    fn to_any(self) -> AnyValue {
        self
    }
}

impl PrismValueInner for i32 {
    type Ref = i32;

    fn type_string() -> String {
        "i32".into()
    }

    fn try_get_any(value: &AnyValue) -> Result<&Self, TypeMismatch> {
        let AnyValue(AnyValueInner::I32(value)) = value else {
            match Self::fail(value)? {}
        };
        Ok(value)
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

    fn type_string() -> String {
        "u32".into()
    }

    fn try_get_any(value: &AnyValue) -> Result<&Self, TypeMismatch> {
        let AnyValue(AnyValueInner::U32(value)) = value else {
            match Self::fail(value)? {}
        };
        Ok(value)
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
pub trait HeapValue: Any + Sync + Send + Debug + 'static {
    fn type_string(&self) -> String;
}

impl<T: Any + Sync + Send + Debug + 'static> HeapValue for T {
    fn type_string(&self) -> String {
        type_name::<T>().into()
    }
}

impl<T> PrismValueInner for Arc<T>
where
    T: HeapValue,
{
    type Ref = T;

    fn type_string() -> String {
        format!("PrismArc<{}>", type_name::<T>())
    }

    fn try_get_any(value: &AnyValue) -> Result<&Self::Ref, TypeMismatch> {
        let AnyValue(AnyValueInner::Arc(arc_value)) = value else {
            match Self::fail(value)? {}
        };
        let any_value = &**arc_value as &dyn Any;
        let Some(value) = any_value.downcast_ref() else {
            match Self::fail(value)? {}
        };
        Ok(value)
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
