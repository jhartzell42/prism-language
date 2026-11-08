//! `prism_rtl`: the runtime library compiled Prism programs link (Compiler ch. 5).
//!
//! Emitted code is typed Rust: one Rust type per Prism type, typed closures, lifting
//! resolved at compile time, and each widget's graph (its nodes, heights, propagation,
//! holds, states and transitions) emitted from code templates. This crate is what is
//! left: the part of a compiled program that is not generated per block, written once
//! and shared by every program. `claude/backend.md` is the contract between the two.
//!
//! The modules, from the bottom up:
//!
//! - [`num`]: `Nat` and `Int`, arbitrary precision with a small-value fast path.
//! - [`collections`]: `Text` as a view of a shared UTF-8 buffer and its `Cursor`,
//!   persistent `Map` and `Array`.
//! - [`pod`]: [`Pod`], the plain data a World sees; paths; the `^Pod` traits; `Error`
//!   (a `~T`'s failure) and [`Fault`] (a failed tick).
//! - [`mod@print`]: the transcript's spelling, as the printer API derived `^Show` calls.
//! - [`read`]: reading that spelling back, and the test language's patterns.
//! - [`mod@regex`]: regular expressions over `Text` (the dialect checked in
//!   `regex_dialect`), on `regex-automata`.
//! - [`cells`]: the node primitives: `Hold`, `Stamped`, `Cache`, `Port`, `Event`,
//!   `Dynamic`, `Defer`, and the `Job` an unbounded function's step machine implements.
//! - [`cx`]: the [`Node`] interface, the tick context [`Cx`], and the [`Runtime`] that
//!   walks the lists in Graph's order.
//! - [`doc`]: the document and the patch diff; requests.
//! - [`gc`]: the zombie collector, trial deletion in the compact round.
//! - [`host`]: hosting: the rows of a host's `for` over its child's document, the
//!   blueprint forms a supplier answers with, and the typed way back into the child.
//! - [`world`]: the protocol ([`Program`], [`TickIn`], [`TickOut`], [`ReadyState`],
//!   [`World`], [`drive`]), the Test World, the CLI World, and the scene's clickables.

/// The allocator an emitted program declares its global allocator (README.md, pick AL):
/// jemalloc, as the C++ target's, through `tikv-jemallocator`, named here so that the
/// emitted crate needs no dependency of its own.
pub use tikv_jemallocator::Jemalloc;

pub mod cells;
pub mod collections;
pub mod cx;
pub mod doc;
pub mod gc;
pub mod host;
pub mod num;
pub mod pod;
pub mod print;
pub mod read;
pub mod regex;
mod regex_dialect;
pub mod world;

pub use cells::{Cache, Defer, Demand, Dynamic, Event, Handle, Hold, Job, Live, Port, Snapshot, Stamped, Tick, Turn};
pub use collections::{Array, Cursor, Map, Text};

/// Whether two maps have the same keys, in order: a builder whose rows came or went.
pub fn same_keys<K: Ord + Clone + PartialEq, V: Clone, W: Clone>(a: &Map<K, V>, b: &Map<K, W>) -> bool {
    a.same_keys(b)
}
pub use cx::{Cx, Done, Later, Node, Runtime};
pub use doc::{Document, Edit, Patch, Request};
pub use host::{AnswerOf, Blueprint, Entry, Follow, Host, HostDoc, HostOf, HostRow, Level, Link, PerKey, Rows, Scope, Sink, Supplied, Supplier, Watch, dynamic_blueprint, dynamics_pod, dynamics_ports, element_blueprint, event_blueprint, forward_on_none, never_blueprint, plain_blueprint};
pub use num::{Int, Nat};
pub use pod::{Error, Fault, FromPod, Path, Pod, Segment, ToPod};
pub use print::{Printer, Show, show, show_text};
pub use regex::{Group, Match, Regex, RegexError, RegexPart, RegexReplacement};
pub use world::test_world::TestWorld;
pub use world::{Outcome, Program, ReadyMap, ReadyState, TickIn, TickOut, World, drive};
