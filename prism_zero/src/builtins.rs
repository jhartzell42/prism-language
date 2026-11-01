//! The prelude functions written in Rust are `prism_dyn::builtins`, which compiled code
//! shares: each is registered by the name its signature has in `prelude/prelude.prism`
//! (a signature with no clauses), and lifted by that signature like any function. This
//! module places their faults in the program.

use prism_rt::Value;

pub use prism_dyn::builtins::{table_entries, table_get, table_set};

use crate::interp::{Bx, Interp, Loc};
use crate::ops::R;

/// Builtins reached by a qualified name, with no signature line (Semantics ch. 5's
/// block-local types).
pub fn is_qualified_builtin(name: &str) -> bool {
    matches!(name, "Buffer.Numbered" | "Buffer.Empty" | "Table.Empty")
}

/// Call the builtin `name`.
pub fn call(interp: &Interp, name: &str, args: &[Value], mut b: Bx, loc: Loc) -> R<Value> {
    prism_dyn::builtins::call(name, args, &mut b).map_err(|f| interp.wrap_fault(loc, f))
}

/// The structural answers of the derivable traits (Type System ch. 3, "Derive, out
/// loud").
pub fn structural(interp: &Interp, name: &str, args: &[Value], loc: Loc) -> R<Value> {
    prism_dyn::builtins::structural(name, args).map_err(|f| interp.wrap_fault(loc, f))
}
