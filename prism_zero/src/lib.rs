//! `prism_zero`, compiler zero: a dynamic interpreter of Prism in Rust over `prism_rt`.
//!
//! *Written by Claude, 31 October 2026, on the `claude-self-hosting` branch. Scaffolding
//! (docs/architecture.md): it runs `prismc` once, to compile `prismc`, and is deleted
//! when stage 3 lands. Subject to change; Jimmy rewrites it by hand. The specification is
//! the books in `prism-notes/claude/prism/`; where this crate and a book disagree, the
//! book wins and the disagreement is a bug here, unless `README.md` beside this crate
//! lists it as a pick.*
//!
//! decisions.md D2 and D3 make it an interpreter with no type checker. What a checker
//! would decide statically is decided here at each application, from the runtime values
//! and the sigils written on names and signatures:
//!
//! - **Lifting** (Type System ch. 4, the lifting table): a value knows whether it is
//!   plain, `$` (a [`prism_rt::Value::Dynamic`]), `!` (a [`prism_rt::Value::Event`]) or `@`
//!   (a [`prism_rt::Value::Widget`]). At each application the interpreter compares each
//!   argument with the parameter's declared layer and builds the lift node the table
//!   names: a map over dynamics, a sample at one event, a refusal for two events.
//! - **Trait dispatch** (Type System ch. 3) on the runtime constructor of the receiver:
//!   an `impl` registered for the type, else the structural answer `derive` promises.
//! - **Rows** are the families of the `emit` lines, checked when an element is emitted
//!   and, for the root, against the World's families before tick 0 (Semantics ch. 6).
//!
//! The modules, from the bottom up:
//!
//! - [`load`]: a program as a set of files, one module each; the prelude and the bundled
//!   libraries; `use` resolution; bare-name collisions; the global tables (constructors,
//!   tags, traits, impls).
//! - [`resolve`]: name resolution without running anything, for `prism_zero check`.
//! - [`ops`]: arithmetic, comparison, equality in declaration order, display and `Show`.
//! - [`pattern`]: pattern matching.
//! - [`interp`]: the interpreter's core: code objects, closures and blueprints over
//!   `prism_rt`, scopes, application with the lifting table, unbounded functions.
//! - [`eval`]: expressions.
//! - [`block`]: blocks: widget blocks as binding groups, expression blocks, `given:`.
//! - [`update`]: `&` blocks, by Semantics ch. 5's meaning.
//! - [`builtins`]: the prelude functions written in Rust.
//! - [`world`]: the CLI World's `Prelude.Unwrap` policy, the Test World driver, the
//!   root row check.
//! - [`driver`]: the three commands, `run`, `test`, `check`.

#![allow(clippy::collapsible_if)]
#![allow(clippy::too_many_arguments)]

pub mod block;
pub mod builtins;
pub mod driver;
pub mod eval;
pub mod interp;
pub mod load;
pub mod ops;
pub mod pattern;
pub mod resolve;
pub mod update;
pub mod world;

pub use driver::{check_files, run_files, test_files, Outcome};
