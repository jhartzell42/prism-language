use std::sync::Arc;

use crate::{
    value::ValueType,
    widget::{Blueprint, WidgetBuilder},
};

/// `dyn`-compatible version of [`Blueprint`] trait.
pub trait DynBlueprint<O: ValueType>: Sync + Send {
    /// Equivalent to [`Blueprint::build`]
    fn build(&self, builder: &mut WidgetBuilder) -> O;
}

/// Newtype adapter to get widgets to implement `dyn`-compatible version.
pub struct DynBlueprintAdapter<O: ValueType, W: Blueprint<Output = O>>(W);

impl<O: ValueType, W: Blueprint<Output = O>> DynBlueprint<O> for DynBlueprintAdapter<O, W> {
    fn build(&self, builder: &mut WidgetBuilder) -> O {
        Blueprint::build(&self.0, builder)
    }
}

impl<O: ValueType> Blueprint for Arc<dyn DynBlueprint<O>> {
    type Output = O;

    fn build(&self, builder: &mut WidgetBuilder) -> Self::Output {
        DynBlueprint::build(&**self, builder)
    }
}
