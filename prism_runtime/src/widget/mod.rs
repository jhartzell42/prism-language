//! Widgets are how we organize FRP components. They also represent
//! the interface between FRP components and the outside world.

mod builder;
mod delegate;
mod dynamic;
mod erased;
mod fold_dyn;
mod node;
mod slots;
mod widget_ready;

#[cfg(test)]
mod tests;

pub use builder::*;
pub use delegate::*;
pub use erased::*;
pub use node::*;
pub use slots::*;

/// Implementers of this trait represent a Prism component that
/// is implemented inside the context of the Prism runtime.
/// This is distinct from backend components, which are implemented
/// outside of the Prism runtime.
///
/// Implementers of this trait should only implement externally visible mutable
/// state through the graph of events, dynamics, and behaviors they build in the
/// form of a widget node. They construct the widget node through the builder.
/// They should not call methods like `subscribe()` on events, nor should they
/// trigger external events, nor should they query the current state of behaviors
/// or dynamics directly. They should instead build new events out of old ones,
/// and new behaviors out of old ones, and express their logic in that way.
///
/// `Self` constitutes the inputs to this component. The `T`
/// represents the output type. The implementation of the `build` function represents
/// the implementation of the component.
pub trait Widget: Send + Sync {
    // XXX: This might be too inflexible. Do we want this trait to be `dyn`-compatible??
    // I guess we'll find out whether that's necessary at some point.
    //
    // I think you could create a type-erased version of this through some sort of trickery with
    // an adapter and a dyn-compatible helper trait?
    /// What type does this widget output?
    type Output: 'static + Send + Sync;
    /// Use `builder` to build a widget node. Return the outputs.
    fn build(&self, builder: &mut WidgetBuilder) -> Self::Output;
}
