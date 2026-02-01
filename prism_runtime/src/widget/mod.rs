//! Widgets are how we organize FRP components. They also represent
//! the interface between FRP components and the outside world.

mod builder;
pub mod erased;
mod node;
pub mod widget_ready;

use builder::WidgetBuilder;
pub use node::WidgetNode;

pub trait Widget<T> {
    fn build(&self, builder: &mut WidgetBuilder) -> T;
}
