use std::any::type_name;
use std::fmt::{Debug, Formatter};
use std::{any::Any, sync::Arc};

use crate::{dynamic::Dynamic, event::Event, event::EventTrigger};

#[derive(Clone)]
pub struct ErasedEvent(Arc<dyn Any + Send + Sync>, Event<dyn Any + Send + Sync>);

impl ErasedEvent {
    pub fn new<T: 'static + Send + Sync>(event: Event<T>) -> ErasedEvent {
        Self(
            Arc::new(event.clone()),
            event.filter_map(|x| Some(x as Arc<dyn Any + Send + Sync>)),
        )
    }

    pub fn try_get<T: 'static + Send + Sync>(&self) -> Option<Event<T>> {
        self.0.downcast_ref::<Event<T>>().map(|e| e.clone())
    }

    pub fn get<T: 'static + Send + Sync>(&self) -> Event<T> {
        self.try_get().expect("wrong event type")
    }

    pub fn matches_inner_type<T: 'static + Send + Sync>(&self) -> bool {
        self.0.is::<Event<T>>()
    }

    pub fn as_any_event(&self) -> Event<dyn Any + Send + Sync> {
        self.1.clone()
    }
}

#[derive(Clone)]
pub struct ErasedDynamic(Arc<dyn Any + Send + Sync>, Dynamic<dyn Any + Send + Sync>);

impl ErasedDynamic {
    pub fn new<T: 'static + Send + Sync>(dynamic: Dynamic<T>) -> ErasedDynamic {
        Self(
            Arc::new(dynamic.clone()),
            dynamic.map(|x| x.clone() as Arc<dyn Any + Send + Sync>),
        )
    }

    pub fn try_get<T: 'static + Send + Sync>(&self) -> Option<Dynamic<T>> {
        self.0.downcast_ref::<Dynamic<T>>().map(|e| e.clone())
    }

    pub fn get<T: 'static + Send + Sync>(&self) -> Dynamic<T> {
        self.try_get().expect("wrong event type")
    }

    pub fn as_any_dynamic(&self) -> Dynamic<dyn Any + Send + Sync> {
        self.1.clone()
    }
}

#[derive(Clone)]
pub struct ErasedEventTrigger(Arc<dyn Any + Send + Sync>, String);

impl ErasedEventTrigger {
    pub fn new<T: 'static + Send + Sync>(event: EventTrigger<T>) -> ErasedEventTrigger {
        Self(Arc::new(event), type_name::<T>().into())
    }

    pub fn type_name(&self) -> &str {
        &self.1
    }

    pub fn try_get<T: 'static + Send + Sync>(&self) -> Option<EventTrigger<T>> {
        self.0.downcast_ref::<EventTrigger<T>>().map(|e| e.clone())
    }

    pub fn get<T: 'static + Send + Sync>(&self) -> EventTrigger<T> {
        self.try_get().expect("wrong trigger type")
    }
}

impl Debug for ErasedEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ErasedEvent").finish()
    }
}

impl Debug for ErasedDynamic {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ErasedEvent").finish()
    }
}

impl Debug for ErasedEventTrigger {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ErasedEvent").finish()
    }
}
