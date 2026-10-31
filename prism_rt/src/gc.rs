//! The collector: refcounted immutable values, plus a tracing mark-sweep over the graph's
//! arena rooted at the widget tree, run between ticks (decisions.md D5, Jimmy: "no
//! leaks, ever").
//!
//! Values are `Rc`s and immutable, so they cannot form a cycle of their own; they reach
//! the graph only through handles ([`NodeId`]s inside [`Value::Event`] and
//! [`Value::Dynamic`]). The one loop memory can hold is a loop through the arena: a dead
//! widget's hold, now a constant, holding a closure over another dead widget's hold, and
//! that one's over the first (Graph ch. 4, "The one loop"). Nothing fires in it and no
//! function runs, so it is garbage, not cruft; reference counts would never free it, and
//! this sweep does.
//!
//! - **Roots:** every widget on the tree (live, reached from the root through its
//!   children), every node such a widget made, the started list, the flattens that
//!   adopted, the armed `@Defer`s, and the root's result.
//! - **Edges followed:** a node's inputs (prompt, read and sampled), the nodes its data
//!   names (a builder's result, a host's document), and the handles inside every value
//!   it holds: its cells' two variables, its caches, its constants, and its closures'
//!   captured values.
//! - **The sweep** lets each unreached node go of the reached nodes it subscribed to or
//!   read, so their demand falls as if the dead consumer had unsubscribed, then frees
//!   its cell; unreached widgets are freed too.
//!
//! Graph's own book refuses a tracing collector on cost grounds (Compiler ch. 5,
//! "Research"); this is the self-hosting branch's runtime, and D5 records the refusal as
//! a doubt there.

use std::collections::BTreeSet;

use crate::tick::State;
use crate::value::{NodeId, Value, WidgetId};

/// What a collection freed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Collected {
    pub nodes: usize,
    pub widgets: usize,
}

/// Collect a state's garbage. Run it between ticks; a World decides how often.
pub fn collect(st: &mut State) -> Collected {
    let mut nodes: BTreeSet<NodeId> = BTreeSet::new();
    let mut widgets: BTreeSet<WidgetId> = BTreeSet::new();
    let mut work: Vec<NodeId> = Vec::new();

    // The tree.
    let mut stack: Vec<WidgetId> = st.root.into_iter().collect();
    while let Some(w) = stack.pop() {
        if !st.widgets.is_live(w) || !widgets.insert(w) {
            continue;
        }
        let widget = st.widgets.get(w);
        work.extend(widget.nodes.iter().copied());
        stack.extend(widget.children.iter().copied());
    }
    work.extend(st.started.iter().copied());
    work.extend(st.adoptions.iter().copied());
    work.extend(st.armed.iter().copied());
    trace_value(&st.result, &mut work);

    // Mark.
    while let Some(id) = work.pop() {
        if !st.graph.exists(id) || !nodes.insert(id) {
            continue;
        }
        let kind = &st.graph.kind(id);
        work.extend(kind.references());
        for v in kind.values() {
            trace_value(v, &mut work);
        }
    }

    // Sweep nodes: first let go of the reached, so their demand falls, then free.
    let dead: Vec<NodeId> = st.graph.ids().filter(|id| !nodes.contains(id)).collect();
    for id in &dead {
        let kind = st.graph.kind(*id).clone();
        if st.graph.node(*id).active {
            for p in kind.prompt_inputs() {
                if nodes.contains(&p) {
                    st.graph.unsubscribe(p, *id);
                }
            }
        }
        for r in kind.read_inputs() {
            if nodes.contains(&r) {
                st.graph.remove_reader(r, *id);
            }
        }
    }
    for id in &dead {
        st.graph.free(*id);
    }

    // Sweep widgets.
    let dead_widgets: Vec<WidgetId> = st.widgets.ids().filter(|w| !widgets.contains(w)).collect();
    for w in &dead_widgets {
        st.widgets.free(*w);
    }

    Collected { nodes: dead.len(), widgets: dead_widgets.len() }
}

/// Push every node handle inside a value.
fn trace_value(v: &Value, out: &mut Vec<NodeId>) {
    match v {
        Value::Event(id) | Value::Dynamic(id) => out.push(*id),
        Value::Record(r) => r.fields.iter().for_each(|(_, f)| trace_value(f, out)),
        Value::Tuple(t) => t.iter().for_each(|f| trace_value(f, out)),
        Value::Choice(c) => c.payload.iter().for_each(|f| trace_value(f, out)),
        Value::Map(m) => m.iter().for_each(|(k, f)| {
            trace_value(k, out);
            trace_value(f, out);
        }),
        Value::Array(a) => a.iter().for_each(|f| trace_value(f, out)),
        Value::Opt(Some(x)) | Value::Fallible(Ok(x)) | Value::Fallible(Err(x)) => trace_value(x, out),
        Value::Element(e) => e.fields.iter().for_each(|(_, f)| trace_value(f, out)),
        Value::Closure(c) => c.captured().iter().for_each(|f| trace_value(f, out)),
        Value::Widget(b) => b.captured().iter().for_each(|f| trace_value(f, out)),
        Value::Ref(r) => trace_value(&r.get(), out),
        _ => {}
    }
}

/// The debug allocator's counters: live function cells (closures and blueprints), per
/// thread. A test reads them before and after to see that a collection let go of every
/// closure the dead held.
pub mod debug {
    use std::cell::Cell;

    thread_local! {
        static LIVE_FUNCTIONS: Cell<i64> = const { Cell::new(0) };
    }

    pub(crate) fn closure_born() {
        LIVE_FUNCTIONS.with(|c| c.set(c.get() + 1));
    }

    pub(crate) fn closure_died() {
        LIVE_FUNCTIONS.with(|c| c.set(c.get() - 1));
    }

    /// Closures and blueprints alive on this thread.
    pub fn live_functions() -> i64 {
        LIVE_FUNCTIONS.with(|c| c.get())
    }
}
