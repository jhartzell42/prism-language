use std::{any::Any, sync::Arc};

use crate::{dynamic::Dynamic, event::Event, event::EventTrigger};

#[derive(Clone)]
pub struct ErasedEvent(Arc<dyn Any>, Event<dyn Any>);

impl ErasedEvent {
    pub fn new<T: 'static>(event: Event<T>) -> ErasedEvent {
        Self(
            Arc::new(event.clone()),
            event.filter_map(|x| Some(x as Arc<dyn Any>)),
        )
    }

    pub fn try_get<T: 'static>(&self) -> Option<Event<T>> {
        self.0.downcast_ref::<Event<T>>().map(|e| e.clone())
    }

    pub fn get<T: 'static>(&self) -> Event<T> {
        self.try_get().expect("wrong event type")
    }

    pub fn matches_inner_type<T: 'static>(&self) -> bool {
        self.0.is::<Event<T>>()
    }

    pub fn as_any_event(&self) -> Event<dyn Any> {
        self.1.clone()
    }
}

#[derive(Clone)]
pub struct ErasedDynamic(Arc<dyn Any>);

impl ErasedDynamic {
    pub fn new<T: 'static>(event: Dynamic<T>) -> ErasedDynamic {
        Self(Arc::new(event))
    }

    pub fn try_get<T: 'static>(&self) -> Option<Dynamic<T>> {
        self.0.downcast_ref::<Dynamic<T>>().map(|e| e.clone())
    }

    pub fn get<T: 'static>(&self) -> Dynamic<T> {
        self.try_get().expect("wrong event type")
    }
}

#[derive(Clone)]
pub struct ErasedEventTrigger(Arc<dyn Any>);

impl ErasedEventTrigger {
    pub fn new<T: 'static>(event: EventTrigger<T>) -> ErasedEventTrigger {
        Self(Arc::new(event))
    }

    pub fn try_get<T: 'static>(&self) -> Option<EventTrigger<T>> {
        self.0.downcast_ref::<EventTrigger<T>>().map(|e| e.clone())
    }

    pub fn get<T: 'static>(&self) -> EventTrigger<T> {
        self.try_get().expect("wrong event type")
    }
}
