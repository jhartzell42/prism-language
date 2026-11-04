//! The protocol every World shares with every program, and the loop the CLI and canvas
//! Worlds run (Semantics ch. 9, "The foreign World's rules"; Graph ch. 3).
//!
//! A program is a [`Program`]: `build` is tick 0, `tick` one tick, `compact` the free time
//! between. A tick is sync for the World: it hands in [`TickIn`] (answers by path, pushes
//! keyed by element value) and gets [`TickOut`] back (the patch, the requests in document
//! order, the result, `pending`). The one call into the World inside a tick is the
//! first-value query, [`ReadyState::first_value`], answered from what the World holds.
//! Everything that crosses is a [`Pod`]: the World holds no node and no typed value.
//!
//! - [`test_world`]: the Test World, which runs scripts in the test language and prints
//!   transcripts to the byte.
//! - [`scene`]: `Draw.Clickable` in a `Gui.Scene`, which the Test World clicks by path
//!   (decisions.md D9) and the canvas World ignores.
//!
//! The CLI and canvas Worlds are crates of their own over this protocol (`worlds/`).

pub mod scene;
pub mod test_world;

use std::collections::BTreeMap;
use std::rc::Rc;

use crate::cells::Tick;
use crate::doc::{Patch, Request};
use crate::pod::{Fault, Path, Pod};

/// A tick's inputs: answers by path, to places that existed as the tick opened, and
/// pushes keyed by element value, to every place with that element.
#[derive(Clone, Debug, Default)]
pub struct TickIn {
    pub answers: Vec<(Path, Pod)>,
    pub pushes: Vec<(Pod, Pod)>,
}

impl TickIn {
    pub fn none() -> TickIn {
        TickIn::default()
    }
    pub fn answer(mut self, path: Path, v: Pod) -> TickIn {
        self.answers.push((path, v));
        self
    }
    pub fn push(mut self, element: Pod, v: Pod) -> TickIn {
        self.pushes.push((element, v));
        self
    }
}

/// The root's result, when it is not `unit`.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// A plain result at the build, or a dynamic one at the build and when it changes.
    Value(Pod),
    /// An event result that fired this tick.
    Fired(Pod),
}

/// A tick's outputs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TickOut {
    pub tick: Tick,
    pub patch: Patch,
    /// In document order, torn-down places included.
    pub requests: Vec<Request>,
    pub result: Option<Outcome>,
    /// Set while a newborn owes a tick or a `@Defer` is armed: a World that sees it ticks
    /// again, with no inputs if it has none (decisions.md D6).
    pub pending: bool,
}

/// What a World holds as the tick opens, plus pure computation over it (round 47 RS).
pub trait ReadyState {
    /// The first value of a new place whose element's `Level` is `element`.
    fn first_value(&self, element: &Pod) -> Option<Pod>;
}

/// Entries keyed by element value, as the test language's `with` lines write them.
#[derive(Clone, Debug, Default)]
pub struct ReadyMap(pub BTreeMap<Pod, Pod>);

impl ReadyState for ReadyMap {
    fn first_value(&self, element: &Pod) -> Option<Pod> {
        self.0.get(element).cloned()
    }
}

/// A compiled program, as every World drives it. The templates emit one implementation
/// per program, around its root node and a [`crate::Runtime`].
pub trait Program {
    /// Tick 0, `tick` on no state: build the root from its arguments (as plain data, so
    /// a test line's `@Normal (App ..)` reaches it).
    fn build(args: &[Pod], ready: &dyn ReadyState) -> Result<(Self, TickOut), Fault>
    where
        Self: Sized;
    /// One tick. On a fault the tick never happened, and the World decides what next.
    fn tick(&mut self, input: &TickIn, ready: &dyn ReadyState) -> Result<TickOut, Fault>;
    /// The compact round, in the free time after a tick's outputs have left.
    fn compact(&mut self) {}
}

/// A foreign World, as [`drive`] sees it.
pub trait World {
    /// The ready state the next tick's first-value queries are answered from.
    fn ready(&self) -> Rc<dyn ReadyState>;
    /// Perform a tick's outputs: its requests in document order, torn-down places
    /// included, before acting on the patch.
    fn perform(&mut self, out: &TickOut);
    /// The next tick's inputs, or `None` when nothing the World holds can cause another.
    fn next_inputs(&mut self) -> Option<TickIn>;
    /// The code the program asked to exit with, once it has.
    fn exit_code(&self) -> Option<u8>;
    /// A failed tick, the World's own event. The CLI policy: report it and exit 1.
    fn failed(&mut self, fault: &Fault) -> u8 {
        eprintln!("prism: a tick failed: {fault}");
        EXIT_FAILED
    }
}

/// A poisoned root or a failed tick (Semantics ch. 9, "The CLI World's policy").
pub const EXIT_FAILED: u8 = 1;
/// A program that fails to check, before tick 0.
pub const EXIT_UNCHECKED: u8 = 2;

/// Run a program under a World until it exits: build, then tick while the World has
/// inputs or the program is `pending`, compacting after every tick. `patience` bounds the
/// ticks: a World's patience, not a semantics.
pub fn drive<P: Program>(world: &mut dyn World, args: &[Pod], patience: usize) -> u8 {
    let (mut program, mut out) = match P::build(args, &*world.ready()) {
        Ok(built) => built,
        Err(fault) => return world.failed(&fault),
    };
    for _ in 0..patience {
        world.perform(&out);
        if let Some(code) = world.exit_code() {
            return code;
        }
        program.compact();
        let input = match world.next_inputs() {
            Some(i) => i,
            None if out.pending => TickIn::none(),
            None => return 0,
        };
        out = match program.tick(&input, &*world.ready()) {
            Ok(o) => o,
            Err(fault) => return world.failed(&fault),
        };
    }
    eprintln!("prism: the World's patience ran out after {patience} ticks");
    EXIT_FAILED
}
