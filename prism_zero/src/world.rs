//! The Worlds as compiler zero uses them: `prism_rt`'s CLI World with the
//! `Prelude.Unwrap` policy on top (`prism_dyn::world`, which compiled programs share),
//! and the root's row check before tick 0.

use std::collections::{BTreeSet, HashSet};

use prism_rt::Row;

use crate::load::{Loaded, ModId};

/// The families the CLI World answers (prism_rt/src/world/cli.rs), and the prelude's,
/// which this World answers itself.
pub const CLI_FAMILIES: &[&str] = &["Fs", "Process", "Time", "Prelude"];

pub use prism_dyn::world::UnwrapWorld;

/// The families a module's expressions name by a tag: the families its widgets may emit.
pub fn families_named(l: &Loaded, module: ModId) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let src = l.modules[module].src;
    for (name, t) in &l.tags {
        if t.fault || t.module != module && !src.contains(name.as_str()) {
            continue;
        }
        if src.contains(name.as_str()) {
            out.insert(t.decl.family().to_string());
        }
    }
    out
}

/// The root's row against the World's families, before tick 0 (Semantics ch. 6, "The
/// root, and what a set of files means"; ch. 10, "The row check"). The row is the root
/// file's `emit` line, or, for the implied `emit Element`, the families every module the
/// root reaches by `use` names by a tag. Host documents subtracting what they answer is
/// the checker's and is not done here.
pub fn row_check(l: &Loaded, root: ModId, world: &[&str], world_name: &str) -> Result<(), String> {
    let m = &l.modules[root];
    let fams: BTreeSet<String> = match &m.row {
        Row::Families(f) => f.iter().cloned().collect(),
        Row::Any => {
            let mut seen = HashSet::new();
            let mut stack = vec![root];
            let mut out = BTreeSet::new();
            while let Some(id) = stack.pop() {
                if !seen.insert(id) {
                    continue;
                }
                out.extend(families_named(l, id));
                for q in &l.modules[id].qualified {
                    if *q != l.prelude {
                        stack.push(*q);
                    }
                }
            }
            out
        }
    };
    for f in &fams {
        if !world.contains(&f.as_str()) {
            let emit = m.ast.emit.as_ref().map(|e| format!("\n  {}       emit {}", m.at(e.span), m.text(e.span))).unwrap_or_default();
            return Err(format!(
                "error: @main may emit the family {f}, and this World does not answer it{emit}\n  the {world_name} World answers: {}\n  host @main in a document that answers {f}.*, or run it under a World that does",
                world.join(", ")
            ));
        }
    }
    Ok(())
}
