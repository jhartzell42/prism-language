//! `prism_rt`: the runtime library of the Prism language on the self-hosting branch.
//!
//! Every program links it: compiled programs and compiler zero's interpreter alike
//! (docs/architecture.md). The books in `prism-notes` (`claude/prism/`) are its
//! specification, and each module cites the chapter and rule it implements. Where the
//! books left a choice open, the choice is named in the module's documentation.
//!
//! The modules, from the bottom up:
//!
//! - [`value`]: the dynamic value representation, with bignum integers, rope text,
//!   persistent maps and arrays, closures, blueprints, elements, paths and handles.
//! - [`pod`]: the transcript's spelling of plain values, printing and reading back, and
//!   the test language's patterns.
//! - `graph`: nodes, heights, demand, double-buffered holds, marks, propagation by
//!   height (internal; reached through [`widget::Build`] and [`tick::tick`]).
//! - [`document`]: places, the patch, requests, rows, the ready state.
//! - [`widget`]: construction through [`widget::Build`], `rebuild`, keyed `for`, host
//!   documents, switches and flattens, `@Defer`, `@Machine`, `@Run`, leaving the tree.
//! - [`mod@tick`]: the tick function, `tick: ?State, inputs -> State, outputs, pending`.
//! - [`gc`]: the collector between ticks, and the debug counters.
//! - [`families`]: tag declarations of the element languages the Worlds answer.
//! - [`world`]: the `World` trait and its driver, the Test World running transcript
//!   scripts, and the `Fs` and `Process` Worlds of the command line.
//!
//! `API.md` beside this crate lists the surface an interpreter calls.

// Nested `if let`s are kept over let-chains: they read plainly to someone rewriting the
// crate by hand.
#![allow(clippy::collapsible_if)]

pub mod document;
pub mod families;
pub mod gc;
mod graph;
pub mod pod;
pub mod tick;
pub mod value;
pub mod widget;
pub mod world;

pub use document::{Document, Patch, ReadyMap, ReadyState, Request, Row};
pub use tick::{tick, Inputs, Outputs, Program, ResultReport, State, Ticked};
pub use value::{
    Blueprint, Choice, Closure, Element, Fault, FieldKind, NodeId, OutputKind, Path, Record, Segment, TagDecl,
    Text, Tick, Value,
};
pub use widget::{Build, ResultKind, Scope};
