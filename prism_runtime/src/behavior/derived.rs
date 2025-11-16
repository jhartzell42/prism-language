use std::{
    marker::PhantomData,
    sync::{Arc, Mutex, Weak},
};

use crate::behavior::{Behavior, BehaviorDependent, BehaviorImpl};

impl<O: 'static> Behavior<O> {
    /// This returns a behavior whose value will always be `f(a,b)` where
    /// `a` and `b` are behaviors of appropriate type. `f` is assumed to be
    /// a pure function.
    pub fn map2<A: 'static, B: 'static>(
        f: impl Fn(Arc<A>, Arc<B>) -> Arc<O> + 'static,
        a: Behavior<A>,
        b: Behavior<B>,
    ) -> Self {
        struct BinaryBehavior<A, B, O, F: Fn(Arc<A>, Arc<B>) -> Arc<O>> {
            a: Behavior<A>,
            b: Behavior<B>,
            f: F,
            weak_self: Weak<Self>,
            cache: Mutex<Option<(Arc<O>, Vec<Weak<dyn BehaviorDependent>>)>>,
            phantom: PhantomData<O>,
        }

        impl<A: 'static, B: 'static, O: 'static, F: 'static + Fn(Arc<A>, Arc<B>) -> Arc<O>>
            BehaviorImpl<O> for BinaryBehavior<A, B, O, F>
        {
            fn query_for_behavior(&self, dependent: Weak<dyn BehaviorDependent>) -> Arc<O> {
                let mut cache = self.cache.lock().unwrap();
                if let Some((value, dependents)) = &mut *cache {
                    dependents.push(dependent);
                    return value.clone();
                }
                let a = self.a.0.query_for_behavior(self.weak_self.clone());
                let b = self.b.0.query_for_behavior(self.weak_self.clone());
                let value = (self.f)(a, b);
                *cache = Some((value.clone(), vec![dependent]));
                value
            }

            fn query_for_tag(&self) -> Arc<O> {
                let mut cache = self.cache.lock().unwrap();
                if let Some((value, _)) = &mut *cache {
                    return value.clone();
                }
                let a = self.a.0.query_for_behavior(self.weak_self.clone());
                let b = self.b.0.query_for_behavior(self.weak_self.clone());
                let value = (self.f)(a, b);
                *cache = Some((value.clone(), vec![]));
                value
            }
        }

        impl<A, B, O, F: Fn(Arc<A>, Arc<B>) -> Arc<O>> BehaviorDependent for BinaryBehavior<A, B, O, F> {
            fn invalidate(&self) {
                let cache = self.cache.lock().unwrap().take();
                let Some((_, dependents)) = cache else { return };
                for dep in dependents {
                    if let Some(dep) = dep.upgrade() {
                        dep.invalidate()
                    }
                }
            }
        }

        Behavior(Arc::new_cyclic(|weak| BinaryBehavior {
            a,
            b,
            f,
            weak_self: weak.clone(),
            cache: Mutex::new(None),
            phantom: PhantomData,
        }))
    }
}
