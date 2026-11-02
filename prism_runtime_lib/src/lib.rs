//! `prism_rtl`: the runtime library compiled Prism programs link (Compiler ch. 5).
//!
//! Emitted code is typed Rust: one Rust type per Prism type, typed closures, lifting
//! resolved at compile time, and each widget's graph (its nodes, heights, propagation,
//! holds, states and transitions) emitted from code templates. This crate is what is
//! left: the part of a compiled program that is not generated per block, written once
//! and shared by every program. `docs/backend.md` is the contract between the two.
//!
//! The modules, from the bottom up:
//!
//! - [`num`]: `Nat` and `Int`, arbitrary precision with a small-value fast path.
//! - [`collections`]: `Text` as a rope, persistent `Map` and `Array`.
//! - [`pod`]: [`Pod`], the plain data a World sees; paths; the `^Pod` traits; `Error`
//!   (a `~T`'s failure) and [`Fault`] (a failed tick).
//! - [`mod@print`]: the transcript's spelling, as the printer API derived `^Show` calls.
//! - [`read`]: reading that spelling back, and the test language's patterns.
//! - [`cells`]: the node primitives: `Hold`, `Stamped`, `Cache`, `Port`, `Event`,
//!   `Dynamic`, `Defer`, and the `Job` an unbounded function's step machine implements.
//! - [`cx`]: the [`Node`] interface, the tick context [`Cx`], and the [`Runtime`] that
//!   walks the lists in Graph's order.
//! - [`doc`]: the document and the patch diff; requests.
//! - [`gc`]: the zombie collector, trial deletion in the compact round.
//! - [`host`]: host nodes, their supplier rows and the child's document (hosting).
//! - [`world`]: the protocol ([`Program`], [`TickIn`], [`TickOut`], [`ReadyState`],
//!   [`World`], [`drive`]), the Test World, the CLI World, and the scene's clickables.

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
pub mod world;

pub use cells::{Cache, Defer, Demand, Dynamic, Event, Handle, Hold, Job, Live, Port, Stamped, Tick, Turn};
pub use collections::{Array, Map, Text};
pub use cx::{Cx, Done, Level, Node, Runtime};
pub use doc::{Document, Edit, Patch, Request};
pub use host::{Answering, Blueprint, Host, HostDoc, built, element_blueprint, pure_blueprint};
pub use num::{Int, Nat};
pub use pod::{Error, Fault, FromPod, Path, Pod, Segment, ToPod};
pub use print::{Printer, Show, show, show_text};
pub use world::test_world::TestWorld;
pub use world::{Outcome, Program, ReadyMap, ReadyState, TickIn, TickOut, World, drive};
