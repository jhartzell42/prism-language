use std::sync::{Arc, Weak};

use crate::{
    behavior::{Behavior, BehaviorDependent, BehaviorImpl},
    value::{Value, ValueType},
};

impl<T: ValueType> Behavior<T> {
    /// This returns a behavior whose value is and always will be
    /// the value you pass in.
    pub fn constant(value: Value<T>) -> Self {
        struct ConstantBehavior<T: ValueType> {
            pub(crate) value: Value<T>,
        }

        impl<T: ValueType> BehaviorImpl<T> for ConstantBehavior<T> {
            fn query_for_behavior(&self, _: Weak<dyn BehaviorDependent>) -> Value<T> {
                self.value.clone()
            }

            fn query_for_tag(&self) -> Value<T> {
                self.value.clone()
            }
        }

        Behavior(Arc::new(ConstantBehavior { value }))
    }
}
