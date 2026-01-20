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
        struct TwoBehaviorFunction<A, B, F: Fn(Arc<A>, Arc<B>) -> Arc<O>, O> {
            a: Behavior<A>,
            b: Behavior<B>,
            f: F,
            phantom: PhantomData<O>,
        }

        impl<A: 'static, B: 'static, F: Fn(Arc<A>, Arc<B>) -> Arc<O>, O> BehaviorComputation<O>
            for TwoBehaviorFunction<A, B, F, O>
        {
            fn compute(&self, dependent: BehaviorDependencyTracker) -> Arc<O> {
                let a = self.a.query_for_computation(dependent.clone());
                let b = self.b.query_for_computation(dependent);
                (self.f)(a, b)
            }
        }

        Behavior(Arc::new_cyclic(|weak| DependentBehavior {
            computation: TwoBehaviorFunction {
                a,
                b,
                f,
                phantom: PhantomData,
            },
            weak_self: weak.clone(),
            cache: Mutex::new(None),
            phantom: PhantomData,
        }))
    }

    /// This should be used inside an implementation of `BehaviorComputation`
    /// so that a `DependentBehavior` can track its dependency tree.
    pub fn query_for_computation(&self, dep: BehaviorDependencyTracker) -> Arc<O> {
        self.0.query_for_behavior(dep.0)
    }

    /// Construct a new dependent behavior from an implementor of [`BehaviorComputation`].
    /// This low level function can be used to build a variety custom computational primitives.
    pub fn dependent_behavior(computation: impl BehaviorComputation<O> + 'static) -> Behavior<O> {
        Behavior(Arc::new_cyclic(|weak| DependentBehavior {
            computation,
            weak_self: weak.clone(),
            cache: Mutex::new(None),
            phantom: PhantomData,
        }))
    }
}

#[derive(Clone)]
pub struct BehaviorDependencyTracker(Weak<dyn BehaviorDependent>);

/// Trait for a value that represents how to compute a derived behavior.
/// The value should own other behaviors.
pub trait BehaviorComputation<O> {
    /// This function should query the behaviors this value owns with
    /// [`Behavior::query_for_computation()`].
    fn compute(&self, dependent: BehaviorDependencyTracker) -> Arc<O>;
}

type Dependents = Vec<Weak<dyn BehaviorDependent>>;
struct DependentBehavior<O, C: BehaviorComputation<O>> {
    computation: C,
    weak_self: Weak<Self>,
    cache: Mutex<Option<(Arc<O>, Dependents)>>,
    phantom: PhantomData<O>,
}

impl<O: 'static, C: 'static + BehaviorComputation<O>> BehaviorImpl<O> for DependentBehavior<O, C> {
    fn query_for_behavior(&self, dependent: Weak<dyn BehaviorDependent>) -> Arc<O> {
        let mut cache = self.cache.lock().unwrap();
        if let Some((value, dependents)) = &mut *cache {
            dependents.push(dependent);
            return value.clone();
        }
        let value = self
            .computation
            .compute(BehaviorDependencyTracker(self.weak_self.clone()));
        *cache = Some((value.clone(), vec![dependent]));
        value
    }

    fn query_for_tag(&self) -> Arc<O> {
        let mut cache = self.cache.lock().unwrap();
        if let Some((value, _)) = &mut *cache {
            return value.clone();
        }
        let value = self
            .computation
            .compute(BehaviorDependencyTracker(self.weak_self.clone()));
        *cache = Some((value.clone(), vec![]));
        value
    }
}

impl<O: 'static, C: 'static + BehaviorComputation<O>> BehaviorDependent
    for DependentBehavior<O, C>
{
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
