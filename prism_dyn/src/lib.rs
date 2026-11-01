//! `prism_dyn`: the dynamic layer of Prism's semantics over `prism_rt`, shared by
//! compiler zero (`prism_zero`, which interprets a program's tree) and by the Rust that
//! `prismc` emits (decisions.md D16: "Emitted Rust uses `prism_rt`'s dynamic `Value` and
//! the same apply-with-lifting layer the interpreter uses, so codegen needs no types").
//!
//! *Written by Claude, 31 October 2026, on the `claude-self-hosting` branch, extracted
//! from compiler zero so that the interpreter and compiled code run one implementation
//! of the semantics. Subject to change; Jimmy rewrites it by hand. The books in
//! `prism-notes/claude/prism/` are the specification.*
//!
//! What a checker would decide statically is decided here, at each application, from
//! the runtime values and the layers a function's meta declares:
//!
//! - [`func`]: function values and their metas, application with the lifting table,
//!   peeling `?` and `~`, `Nat` conversion.
//! - [`values`]: fields, indexing, constructors, elements, operators with their user
//!   `impl`s, trait dispatch, `Show` and interpolation.
//! - [`tables`]: the program's constructors, tags and `impl`s, per thread.
//! - [`ops`]: arithmetic, comparison, equality, the transcript's spelling.
//! - [`builtins`]: the prelude's functions written in Rust, the event combinators, the
//!   widget bindings (`@Run`, the host documents, ...).
//! - [`pat`]: one test per pattern form.
//! - [`lens`]: `&` blocks' lens paths.
//! - [`widget`]: what a widget block's statements do beyond `prism_rt`'s `Build`.
//! - [`unbounded`]: cycle points (decisions.md D6).
//! - [`world`]: the `Prelude.Unwrap` policy over the CLI World, and the drivers a
//!   compiled program's `main` calls.
//! - [`rt`]: everything compiled code uses, under short names.

#![allow(clippy::collapsible_if)]
#![allow(clippy::too_many_arguments)]

pub mod builtins;
pub mod func;
pub mod lens;
pub mod ops;
pub mod pat;
pub mod rt;
pub mod tables;
pub mod unbounded;
pub mod values;
pub mod widget;
pub mod world;

pub use func::{apply, apply_with, Bx, FnMeta, Layer};
pub use ops::R;
