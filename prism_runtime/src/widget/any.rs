#![allow(missing_docs)]

use std::any::type_name;
use std::fmt::{Debug, Formatter};
use std::{any::Any, sync::Arc};

use crate::value::{AnyValue, ValueType};
use crate::{dynamic::Dynamic, event::Event, event::EventTrigger};

#[derive(Clone)]
pub struct AnyEvent {
    inner: Arc<dyn Any + Send + Sync>,
    converted: Event<AnyValue>,
}

impl AnyEvent {
    pub fn new<T: ValueType>(event: Event<T>) -> AnyEvent {
        Self {
            inner: Arc::new(event.clone()),
            converted: event.filter_map(|x| Some(AnyValue::from(x).into())),
        }
    }

    pub fn try_get<T: ValueType>(&self) -> Option<Event<T>> {
        self.inner.downcast_ref::<Event<T>>().map(|e| e.clone())
    }

    pub fn get<T: ValueType>(&self) -> Event<T> {
        self.try_get().expect("wrong event type")
    }

    pub fn matches_inner_type<T: ValueType>(&self) -> bool {
        self.inner.is::<Event<T>>()
    }

    pub fn as_any_event(&self) -> Event<AnyValue> {
        self.converted.clone()
    }
}

#[derive(Clone)]
pub struct AnyDynamic {
    inner: Arc<dyn Any + Send + Sync>,
    converted: Dynamic<AnyValue>,
}

impl AnyDynamic {
    pub fn new<T: ValueType>(dynamic: Dynamic<T>) -> AnyDynamic {
        Self {
            inner: Arc::new(dynamic.clone()),
            converted: dynamic.map(|x| AnyValue::from(x).into()),
        }
    }

    pub fn try_get<T: ValueType>(&self) -> Option<Dynamic<T>> {
        self.inner.downcast_ref::<Dynamic<T>>().map(|e| e.clone())
    }

    pub fn get<T: ValueType>(&self) -> Dynamic<T> {
        self.try_get().expect("wrong event type")
    }

    pub fn as_any_dynamic(&self) -> Dynamic<AnyValue> {
        self.converted.clone()
    }
}

#[derive(Clone)]
pub struct AnyEventTrigger {
    inner: Arc<dyn Any + Send + Sync>,
    debug_type: String,
}

impl AnyEventTrigger {
    pub fn new<T: ValueType>(event: EventTrigger<T>) -> AnyEventTrigger {
        Self {
            inner: Arc::new(event),
            debug_type: type_name::<T>().into(),
        }
    }

    pub fn type_name(&self) -> &str {
        &self.debug_type
    }

    pub fn try_get<T: ValueType>(&self) -> Option<EventTrigger<T>> {
        self.inner
            .downcast_ref::<EventTrigger<T>>()
            .map(|e| e.clone())
    }

    pub fn get<T: ValueType>(&self) -> EventTrigger<T> {
        self.try_get().expect("wrong trigger type")
    }
}

impl Debug for AnyEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ErasedEvent").finish()
    }
}

impl Debug for AnyDynamic {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ErasedEvent").finish()
    }
}

impl Debug for AnyEventTrigger {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ErasedEvent").finish()
    }
}
