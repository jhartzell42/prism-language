//! The Worlds as compiler zero uses them: `prism_rt`'s CLI World with the
//! `Prelude.Unwrap` policy on top, the Test World's transcript in the Conformance book's
//! spelling, and the root's row check before tick 0.

use std::collections::{BTreeSet, HashSet};
use std::rc::Rc;

use prism_rt::document::ReadyState;
use prism_rt::world::World;
use prism_rt::{Fault, Inputs, Outputs, Path, Row, Segment, Value};

use crate::load::{Loaded, ModId};

/// The families the CLI World answers (prism_rt/src/world/cli.rs), and the prelude's,
/// which this World answers itself.
pub const CLI_FAMILIES: &[&str] = &["Fs", "Process", "Time", "Prelude"];

/// The CLI World with the prelude's `Prelude.Unwrap` answered (Semantics ch. 10,
/// "Exceptions: a widget giving up"): each `~ x` is answered with `x` in the next tick;
/// the first `Error` poisons the innermost subtree with a path of its own that holds the
/// element (a `rebuild` generation, a `for` row, or the whole program), which is answered
/// nothing and performs no requests from then on. The report goes, a tick later, to the
/// nearest `Prelude.Exceptions` place outside that subtree; with none, the root is
/// poisoned, and the CLI World's policy is to print the exception and exit 1
/// (Semantics ch. 9, "The CLI World's policy"; doubt UH).
pub struct UnwrapWorld<W: World> {
    pub inner: W,
    answers: Vec<(Path, Value)>,
    poisoned: Vec<Path>,
    exceptions: BTreeSet<Path>,
    exit: Option<u8>,
}

impl<W: World> UnwrapWorld<W> {
    pub fn new(inner: W) -> UnwrapWorld<W> {
        UnwrapWorld { inner, answers: vec![], poisoned: vec![], exceptions: BTreeSet::new(), exit: None }
    }

    fn under_poison(&self, p: &Path) -> bool {
        self.poisoned.iter().any(|q| p.0.starts_with(&q.0))
    }
}

/// The innermost subtree with a path of its own holding `p`: its longest prefix ending in
/// a generation or a key; the root when there is none.
fn subtree_of(p: &Path) -> Path {
    let mut end = 0;
    for (i, s) in p.0.iter().enumerate() {
        if matches!(s, Segment::Gen(_) | Segment::Key(_)) {
            end = i + 1;
        }
    }
    Path(p.0[..end].to_vec())
}

impl<W: World> World for UnwrapWorld<W> {
    fn ready(&self) -> Rc<dyn ReadyState> {
        self.inner.ready()
    }

    fn perform(&mut self, outputs: &Outputs) {
        for (p, e) in &outputs.patch.added {
            if e.as_element().is_some_and(|e| &*e.tag == "Prelude.Exceptions") {
                self.exceptions.insert(p.clone());
            }
        }
        for (p, _) in &outputs.patch.removed {
            self.exceptions.remove(p);
        }
        let mut kept = outputs.clone();
        kept.requests.retain(|r| !self.under_poison(&r.path));
        for r in &kept.requests {
            if &*r.tag != "Prelude.Unwrap" {
                continue;
            }
            match &r.value {
                Value::Fallible(Ok(x)) => self.answers.push((r.path.clone(), (**x).clone())),
                Value::Fallible(Err(e)) => {
                    let sub = subtree_of(&r.path);
                    let message = e.field("message").and_then(|m| m.as_text()).map(|t| t.to_string()).unwrap_or_default();
                    let kind = e.field("kind").cloned().unwrap_or(Value::Unit);
                    let exception = Value::record(
                        "Exception",
                        vec![("kind", kind), ("message", Value::text(&message)), ("spans", Value::array(vec![])), ("path", Value::some(Value::Path(sub.clone())))],
                    );
                    // The supervisor: a `Prelude.Exceptions` place outside the subtree whose
                    // parent holds it, the nearest first.
                    let supervisor = self
                        .exceptions
                        .iter()
                        .filter(|p| !p.0.starts_with(&sub.0) && sub.0.starts_with(&p.0[..p.0.len().saturating_sub(1)]))
                        .max_by_key(|p| p.0.len())
                        .cloned();
                    self.poisoned.push(sub.clone());
                    match supervisor {
                        Some(p) if !sub.is_empty() => self.answers.push((p, exception)),
                        _ => {
                            eprintln!("prism: the program gave up: {message}");
                            self.exit = Some(prism_rt::world::EXIT_FAILED);
                        }
                    }
                }
                other => self.answers.push((r.path.clone(), other.clone())),
            }
        }
        self.inner.perform(&kept);
    }

    fn next_inputs(&mut self) -> Option<Inputs> {
        let mine = std::mem::take(&mut self.answers);
        let inner = self.inner.next_inputs();
        let mut inputs = match (inner, mine.is_empty()) {
            (Some(i), _) => i,
            (None, true) => return None,
            (None, false) => Inputs::with_ready(self.inner.ready()),
        };
        inputs.answers.extend(mine);
        let poisoned = self.poisoned.clone();
        inputs.answers.retain(|(p, _)| !poisoned.iter().any(|q| !q.is_empty() && p.0.starts_with(&q.0)));
        Some(inputs)
    }

    fn exit_code(&self) -> Option<u8> {
        self.exit.or(self.inner.exit_code())
    }

    fn failed(&mut self, fault: &Fault) -> Option<u8> {
        self.inner.failed(fault)
    }
}

/// The Test World's transcript, with `compute until idle` in the Conformance book's
/// spelling (ch. 1, "The transcript"): `compute until idle: k ticks, n to m`, then each of
/// its ticks that has a section; a tick with every section empty prints only in the
/// count. `prism_rt`'s Test World prints `compute until idle: k` and every tick; README.md
/// lists the difference as prism_rt's.
pub fn book_transcript(t: &str) -> String {
    let lines: Vec<&str> = t.split('\n').collect();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    let mut next_tick: i64 = 0;
    while i < lines.len() {
        let l = lines[i];
        if let Some(n) = l.strip_prefix("tick ").and_then(|n| n.parse::<i64>().ok()) {
            next_tick = n + 1;
        }
        if let Some(k) = l.strip_prefix("compute until idle: ").and_then(|k| k.parse::<i64>().ok()) {
            let first = next_tick;
            let last = first + k - 1;
            out.push(match k {
                0 => "compute until idle: 0 ticks".to_string(),
                1 => format!("compute until idle: 1 tick, {first} to {last}"),
                _ => format!("compute until idle: {k} ticks, {first} to {last}"),
            });
            i += 1;
            // Its ticks: keep the ones with a section.
            let mut seen = 0;
            while seen < k && i < lines.len() {
                if lines[i].is_empty() && lines.get(i + 1).is_some_and(|x| x.starts_with("tick ")) {
                    let header = lines[i + 1];
                    let mut j = i + 2;
                    let mut body = Vec::new();
                    while j < lines.len() && lines[j].starts_with("  ") {
                        body.push(lines[j]);
                        j += 1;
                    }
                    if !body.is_empty() {
                        out.push(String::new());
                        out.push(header.to_string());
                        out.extend(body.iter().map(|s| s.to_string()));
                    }
                    seen += 1;
                    i = j;
                } else {
                    break;
                }
            }
            next_tick = last + 1;
            continue;
        }
        out.push(l.to_string());
        i += 1;
    }
    out.join("\n")
}

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
