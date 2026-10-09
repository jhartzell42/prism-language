use std::sync::Arc;

use crate::{
    value::ValueType,
    widget::{Widget, WidgetBuilder},
};

/// `dyn`-compatible version of [`Widget`] trait.
pub trait PolymorphicWidget<O: ValueType>: Sync + Send {
    /// Equivalent to [`Widget::build`]
    fn build(&self, builder: &mut WidgetBuilder) -> O;
}

/// Newtype adapter to get widgets to implement `dyn`-compatible version.
pub struct PolymorphicWidgetAdapter<O: ValueType, W: Widget<Output = O>>(W);

impl<O: ValueType, W: Widget<Output = O>> PolymorphicWidget<O> for PolymorphicWidgetAdapter<O, W> {
    fn build(&self, builder: &mut WidgetBuilder) -> O {
        Widget::build(&self.0, builder)
    }
}

impl<O: ValueType> Widget for Arc<dyn PolymorphicWidget<O>> {
    type Output = O;

    fn build(&self, builder: &mut WidgetBuilder) -> Self::Output {
        PolymorphicWidget::build(&**self, builder)
    }
}
