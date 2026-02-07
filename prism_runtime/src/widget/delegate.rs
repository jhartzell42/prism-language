use crate::{runtime::Runtime, widget::WidgetNode};
use std::sync::Arc;

/// This is an event handler that the backend installs on a widget node.  It
/// must not hold a strong `Arc` pointer to the node, because the node owns the
/// delegate with a strong `Arc` pointer.
pub trait WidgetDelegate {
    /// A new subnode has been created of the node we're the delegate for.
    ///
    /// Return a delegate for the new subnode.  You can alias yourself, or you
    /// can create a new delegate.
    ///
    /// This is called when the node is originally created, and when subnodes
    /// change for those widgets that support that.
    ///
    /// The delegate must manage subscribing to its own events if that's something
    /// it would like to do in this case.
    fn new_child_created(
        &self,
        ctxt: WidgetDelegateContext,
        child: &WidgetNode,
    ) -> Arc<dyn WidgetDelegate>;
    /// The node we're a delegate for is about to be destroyed by a node reconstruction
    /// or else the termination of the application. If you're singly owned, you will
    /// soon be dropped.
    fn will_be_destroyed(&self, ctxt: WidgetDelegateContext);
}

/// Common parameters to pass to the widget delegate.
#[derive(Clone, Copy)]
pub struct WidgetDelegateContext<'a> {
    pub runtime: &'a Runtime,
    pub node: &'a WidgetNode,
}

pub struct TrivialDelegate;

impl WidgetDelegate for TrivialDelegate {
    fn new_child_created(
        &self,
        _: WidgetDelegateContext,
        _: &WidgetNode,
    ) -> Arc<dyn WidgetDelegate> {
        Arc::new(TrivialDelegate)
    }

    fn will_be_destroyed(&self, _: WidgetDelegateContext) {}
}
