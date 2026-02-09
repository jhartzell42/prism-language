use std::{
    marker::PhantomData,
    sync::{Arc, Mutex, Weak},
};

use crate::behavior::{Behavior, BehaviorDependent, BehaviorImpl};

impl<O: ?Sized + 'static + Send + Sync> Behavior<O> {
    /// This returns a behavior whose value will always be `f(a)` where
    /// `a` is a behavior of appropriate type. `f` is assumed to be
    /// a pure function.
    pub fn map<T: ?Sized + 'static + Send + Sync>(
        &self,
        f: impl Send + Sync + Fn(Arc<O>) -> Arc<T> + 'static,
    ) -> Behavior<T> {
        struct OneBehaviorFunction<
            A: ?Sized + Send + Sync,
            F: Send + Sync + Fn(Arc<A>) -> Arc<O>,
            O: ?Sized,
        > {
            a: Behavior<A>,
            f: F,
            phantom: PhantomData<O>,
        }

        impl<
            A: ?Sized + 'static + Send + Sync,
            F: Send + Sync + Fn(Arc<A>) -> Arc<O>,
            O: ?Sized + Send + Sync,
        > BehaviorComputation<O> for OneBehaviorFunction<A, F, O>
        {
            fn compute(&self, dependent: BehaviorDependencyTracker) -> Arc<O> {
                let a = self.a.query_for_computation(dependent);
                (self.f)(a)
            }
        }

        Behavior::computation_behavior(OneBehaviorFunction {
            a: self.clone(),
            f,
            phantom: PhantomData,
        })
    }

    /// This returns a behavior whose value will always be `f(a,b)` where
    /// `a` and `b` are behaviors of appropriate type. `f` is assumed to be
    /// a pure function.
    pub fn map2<A: 'static + Send + Sync, B: 'static + Send + Sync>(
        f: impl Send + Sync + Fn(Arc<A>, Arc<B>) -> Arc<O> + 'static,
        a: Behavior<A>,
        b: Behavior<B>,
    ) -> Self {
        struct TwoBehaviorFunction<
            A: ?Sized + Send + Sync,
            B: ?Sized + Send + Sync,
            F: Fn(Arc<A>, Arc<B>) -> Arc<O>,
            O: ?Sized,
        > {
            a: Behavior<A>,
            b: Behavior<B>,
            f: F,
            phantom: PhantomData<O>,
        }

        impl<
            A: ?Sized + 'static + Send + Sync,
            B: ?Sized + 'static + Send + Sync,
            F: Send + Sync + Fn(Arc<A>, Arc<B>) -> Arc<O>,
            O: ?Sized + Send + Sync,
        > BehaviorComputation<O> for TwoBehaviorFunction<A, B, F, O>
        {
            fn compute(&self, dependent: BehaviorDependencyTracker) -> Arc<O> {
                let a = self.a.query_for_computation(dependent.clone());
                let b = self.b.query_for_computation(dependent);
                (self.f)(a, b)
            }
        }

        Self::computation_behavior(TwoBehaviorFunction {
            a,
            b,
            f,
            phantom: PhantomData,
        })
    }

    /// This should be used inside an implementation of `BehaviorComputation`
    /// so that a `DependentBehavior` can track its dependency tree.
    pub fn query_for_computation(&self, dep: BehaviorDependencyTracker) -> Arc<O> {
        self.0.query_for_behavior(dep.0)
    }

    /// Construct a new dependent behavior from an implementor of [`BehaviorComputation`].
    /// This low level function can be used to build a variety custom computational primitives.
    pub fn computation_behavior(computation: impl BehaviorComputation<O> + 'static) -> Behavior<O> {
        Behavior(Arc::new_cyclic(|weak| DependentBehavior {
            computation,
            weak_self: weak.clone(),
            cache: Mutex::new(None),
            phantom: PhantomData,
        }))
    }
}

impl<T: ?Sized + 'static + Send + Sync> Behavior<Behavior<T>> {
    /// Given a nested behavior inside a behavior, create a behavior that always has the
    /// current value of the current inner behavior.
    pub fn join(&self) -> Behavior<T> {
        struct JoinComputation<T: ?Sized + 'static + Send + Sync> {
            outer: Behavior<Behavior<T>>,
        }

        impl<T: 'static + ?Sized + Send + Sync> BehaviorComputation<T> for JoinComputation<T> {
            fn compute(&self, dep: BehaviorDependencyTracker) -> Arc<T> {
                let inner = Arc::unwrap_or_clone(self.outer.query_for_computation(dep.clone()));
                inner.query_for_computation(dep)
            }
        }

        Behavior::computation_behavior(JoinComputation {
            outer: self.clone(),
        })
    }
}

#[derive(Clone)]
pub struct BehaviorDependencyTracker(Weak<dyn BehaviorDependent>);

/// Trait for a value that represents how to compute a derived behavior.
/// The value should own other behaviors.
pub trait BehaviorComputation<O: ?Sized>: Send + Sync {
    /// This function should query the behaviors this value owns with
    /// [`Behavior::query_for_computation()`].
    fn compute(&self, dependent: BehaviorDependencyTracker) -> Arc<O>;
}

type Dependents = Vec<Weak<dyn BehaviorDependent>>;
struct DependentBehavior<O: ?Sized + Send + Sync, C: BehaviorComputation<O>> {
    computation: C,
    weak_self: Weak<Self>,
    cache: Mutex<Option<(Arc<O>, Dependents)>>,
    phantom: PhantomData<O>,
}

impl<O: ?Sized + 'static + Send + Sync, C: 'static + BehaviorComputation<O>> BehaviorImpl<O>
    for DependentBehavior<O, C>
{
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
        let value = self.computation.compute(BehaviorDependencyTracker(
            self.weak_self.clone() as Weak<dyn BehaviorDependent>
        ));
        *cache = Some((value.clone(), vec![]));
        value
    }
}

impl<O: ?Sized + 'static + Send + Sync, C: 'static + BehaviorComputation<O>> BehaviorDependent
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
