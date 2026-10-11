use std::sync::Arc;

use crate::{
    value::ValueType,
    widget::{Blueprint, WidgetBuilder},
};

/// `dyn`-compatible version of [`Blueprint`] trait.
pub trait PolymorphicBlueprint<O: ValueType>: Sync + Send {
    /// Equivalent to [`Blueprint::build`]
    fn build(&self, builder: &mut WidgetBuilder) -> O;
}

/// Newtype adapter to get widgets to implement `dyn`-compatible version.
pub struct PolymorphicWidgetAdapter<O: ValueType, W: Blueprint<Output = O>>(W);

impl<O: ValueType, W: Blueprint<Output = O>> PolymorphicBlueprint<O> for PolymorphicWidgetAdapter<O, W> {
    fn build(&self, builder: &mut WidgetBuilder) -> O {
        Blueprint::build(&self.0, builder)
    }
}

impl<O: ValueType> Blueprint for Arc<dyn PolymorphicBlueprint<O>> {
    type Output = O;

    fn build(&self, builder: &mut WidgetBuilder) -> Self::Output {
        PolymorphicBlueprint::build(&**self, builder)
    }
}
