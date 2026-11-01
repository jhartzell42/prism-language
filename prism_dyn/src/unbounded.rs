//! Unbounded functions (Semantics ch. 7, "An unbounded function is a bounded body
//! iterated"; decisions.md D6), per thread.
//!
//! A cycle point is every call of an unbounded function from an unbounded function, and
//! every turn of a `while` inside one. Continuations are not saved (compiler zero's pick
//! CP, which compiled code keeps so that both run one semantics): since the function is
//! pure, its run is the same whatever cycle it is replayed in, so `@Run` runs it once to
//! the end, counting cycle points, and reports `More` for as many cycles as the canonical
//! rule gives, then `Done` with the result. The transcript is the canonical rule's; only
//! the cost is front-loaded. A run past the cycle budget is stopped at the budget's cycle
//! point and resumed from its start, with twice the budget, in a later cycle, so a job
//! that never ends keeps `pending` set and every tick ends.

use std::cell::Cell;

use prism_rt::Fault;

use crate::ops::R;

#[derive(Default)]
pub struct Unbounded {
    pub depth: Cell<usize>,
    pub budget: Cell<Option<u64>>,
    pub points: Cell<u64>,
}

thread_local! {
    pub static UNBOUNDED: Unbounded = Unbounded::default();
}

/// The kind of the fault that unwinds a run past its cycle budget.
pub const CYCLE_BUDGET: &str = "prism.CycleBudget";

/// A cycle point, counted when an unbounded function is running: the fault unwinds the
/// run once it passes the budget.
pub fn cycle_point() -> R<()> {
    let over = UNBOUNDED.with(|u| {
        if u.depth.get() > 0 {
            let p = u.points.get() + 1;
            u.points.set(p);
            matches!(u.budget.get(), Some(budget) if p > budget)
        } else {
            false
        }
    });
    if over { Err(Fault::new(CYCLE_BUDGET, "the cycle budget ran out")) } else { Ok(()) }
}

/// Run the body of an unbounded function: its call is a cycle point when an unbounded
/// function called it, and the calls in its body are counted.
pub fn unbounded<T>(body: impl FnOnce() -> R<T>) -> R<T> {
    cycle_point()?;
    UNBOUNDED.with(|u| u.depth.set(u.depth.get() + 1));
    let r = body();
    UNBOUNDED.with(|u| u.depth.set(u.depth.get() - 1));
    r
}
