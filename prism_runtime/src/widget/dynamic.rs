use std::marker::PhantomData;

use crate::{
    dynamic::Dynamic,
    event::Event,
    widget::{Widget, builder::WidgetBuilder},
};

impl<T: Widget + 'static> Dynamic<T> {
    /// Create a dynamic widget that starts out by building current value of `self`,
    /// and then replaces it with whatever widget `self` updates to.
    pub fn dynamic_widget(&self) -> impl Widget<Output = Event<T::Output>> {
        DynamicWidget {
            dynamic: self.clone(),
            phantom: PhantomData,
        }
    }
}

struct DynamicWidget<T: 'static + Widget<Output = O>, O> {
    dynamic: Dynamic<T>,
    phantom: PhantomData<O>,
}

impl<T: 'static + Widget<Output = O>, O: 'static> Widget for DynamicWidget<T, O> {
    type Output = Event<O>;
    fn build(&self, builder: &mut WidgetBuilder) -> Event<O> {
        todo!()
    }
}
