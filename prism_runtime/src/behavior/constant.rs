use std::sync::{Arc, Weak};

use crate::behavior::{Behavior, BehaviorDependent, BehaviorImpl};

impl<T: ?Sized + 'static + Send + Sync> Behavior<T> {
    /// This returns a behavior whose value is and always will be
    /// the value you pass in.
    pub fn constant(value: Arc<T>) -> Self {
        struct ConstantBehavior<T: ?Sized + Send + Sync> {
            pub(crate) value: Arc<T>,
        }

        impl<T: ?Sized + Send + Sync> BehaviorImpl<T> for ConstantBehavior<T> {
            fn query_for_behavior(&self, _: Weak<dyn BehaviorDependent>) -> Arc<T> {
                self.value.clone()
            }

            fn query_for_tag(&self) -> Arc<T> {
                self.value.clone()
            }
        }

        Behavior(Arc::new(ConstantBehavior { value }))
    }
}
