//! Worlds: what owns time, the loop, every effect and policy (Semantics ch. 9, "The
//! foreign World's rules").
//!
//! A World calls [`tick`] with the tick's inputs and gets its outputs back as one step;
//! nothing the tick does is visible until all of it is done. It performs the requests in
//! document order, sets up what the patch added, answers in a later tick, and keeps
//! ticking while the program says `pending` (decisions.md D6; Semantics ch. 9, "`tick`
//! returns `pending`, and a World promises to keep ticking while it is set").
//!
//! - [`World`] and [`drive`]: the loop every World but the Test World shares.
//! - [`test_world`]: the Test World, which runs a script in the test language
//!   (Conformance ch. 1 and Appendix A) and prints the transcript to the byte. It never
//!   opens a tick on its own (round 47 OT).
//! - [`cli`]: the command line's World, answering the `Fs`, `Process` and `Time`
//!   languages, with the CLI World's exit codes (Semantics ch. 9, "The CLI World's
//!   policy").

pub mod cli;
pub mod test_world;

use std::rc::Rc;

use crate::document::ReadyState;
use crate::tick::{tick, Inputs, Outputs, Program, State};
use crate::value::Fault;

/// A foreign World, as the driver loop sees it.
pub trait World {
    /// The ready state the next tick's first-value queries are answered from.
    fn ready(&self) -> Rc<dyn ReadyState>;
    /// Perform a tick's outputs: its requests in document order, torn-down places
    /// included, before acting on the patch (Semantics ch. 9, "A request fired by a place
    /// torn down in the same tick is still performed").
    fn perform(&mut self, outputs: &Outputs);
    /// The next tick's answers and pushes, or `None` when the World has nothing waiting
    /// and nothing it holds can cause another tick.
    fn next_inputs(&mut self) -> Option<Inputs>;
    /// The code the program asked to exit with, once it has.
    fn exit_code(&self) -> Option<u8>;
    /// A failed tick, the World's own event: it never happened. The CLI policy is to
    /// report it and exit 1 (doubt PW); return `None` to go on from the state before.
    fn failed(&mut self, fault: &Fault) -> Option<u8> {
        eprintln!("prism: a tick failed: {fault}");
        Some(EXIT_FAILED)
    }
}

/// A poisoned root or an aborted tick: CLI World policy (Semantics ch. 9; Platform ch. 1).
pub const EXIT_FAILED: u8 = 1;
/// A program that fails to check, before tick 0. The checker is `prismc`'s; the code is
/// here so every World spells it alike.
pub const EXIT_UNCHECKED: u8 = 2;

/// Run a program under a World until it exits: build, then tick while the World has
/// inputs or the program is `pending`. Garbage is collected after every tick. `patience`
/// bounds the ticks, a World's patience rather than a semantics (Semantics ch. 9, "Every
/// tick ends, so a compute limit is no part of the semantics").
pub fn drive(world: &mut dyn World, program: &Program, patience: usize) -> u8 {
    let mut state: Option<State> = None;
    let mut inputs = Inputs::with_ready(world.ready());
    for _ in 0..patience {
        let t = tick(program, state.take(), &inputs);
        state = t.state;
        match &t.outputs {
            Ok(out) => world.perform(out),
            Err(fault) => {
                if let Some(code) = world.failed(fault) {
                    return code;
                }
            }
        }
        let Some(st) = state.as_mut() else { return EXIT_FAILED };
        if let Some(code) = world.exit_code() {
            return code;
        }
        crate::gc::collect(st);
        inputs = match world.next_inputs() {
            Some(i) => i,
            None if t.pending => Inputs::with_ready(world.ready()),
            None => return 0,
        };
    }
    eprintln!("prism: the World's patience ran out after {patience} ticks");
    EXIT_FAILED
}
