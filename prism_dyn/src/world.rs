//! The Worlds as a program runs under them: `prism_rt`'s CLI World with the
//! `Prelude.Unwrap` policy on top, and the drivers compiled programs call from `main`.

use std::collections::BTreeSet;
use std::rc::Rc;

use prism_rt::document::ReadyState;
use prism_rt::world::World;
use prism_rt::{Fault, Inputs, Outputs, Path, Segment, Value};

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

/// The World's patience for `run`, in ticks, unless `PRISM_PATIENCE` gives another: the
/// bootstrap's stage 1 (prismc compiling itself under this chain) takes more than this.
pub const RUN_PATIENCE: usize = 10_000_000;

fn run_patience() -> usize {
    std::env::var("PRISM_PATIENCE").ok().and_then(|p| p.parse().ok()).unwrap_or(RUN_PATIENCE)
}

/// Run a program under the CLI World with the `Prelude.Unwrap` policy, in the current
/// directory: the exit code.
pub fn run_cli(program: &prism_rt::Program, args: Vec<String>) -> u8 {
    let cwd = std::env::current_dir().unwrap_or_default();
    let mut world = UnwrapWorld::new(prism_rt::world::cli::CliWorld::new(prism_rt::world::cli::RealFs { root: cwd }, args));
    prism_rt::world::drive(&mut world, program, run_patience())
}

/// An `.expected` file without its leading `# ` comment lines.
pub fn strip_comment_header(s: &str) -> String {
    let mut lines = s.split_inclusive('\n').peekable();
    while lines.peek().is_some_and(|l| l.starts_with("# ") || l.trim_end() == "#") {
        lines.next();
    }
    lines.collect()
}

/// A line-by-line difference.
pub fn diff(expected: &str, got: &str) -> String {
    let (e, g): (Vec<&str>, Vec<&str>) = (expected.lines().collect(), got.lines().collect());
    let mut out = String::new();
    for i in 0..e.len().max(g.len()) {
        match (e.get(i), g.get(i)) {
            (Some(a), Some(b)) if a == b => out.push_str(&format!("  {a}\n")),
            (a, b) => {
                if let Some(a) = a {
                    out.push_str(&format!("- {a}\n"));
                }
                if let Some(b) = b {
                    out.push_str(&format!("+ {b}\n"));
                }
            }
        }
    }
    out
}

/// Run test scripts under the Test World, as `prism_zero test` does: each script's
/// transcript to standard output, its `.expected` beside it compared byte for byte. The
/// programs are the blueprints a `test name, @program:` line can name, and `eval` the
/// compiled `check` expressions by their source text. The exit code: 0 pass, 1 a check
/// failed or a transcript differs, 2 a script did not load.
pub fn run_tests(programs: &[(&str, prism_rt::Blueprint)], eval: Rc<dyn Fn(&str) -> Option<Result<Value, Fault>>>, scripts: &[String]) -> u8 {
    let mut code = 0;
    let mut first = true;
    for path in scripts {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{path}: {e}");
                code = 2;
                continue;
            }
        };
        let mut world = prism_rt::world::test_world::TestWorld::new();
        for (name, bp) in programs {
            world = world.program(prism_rt::Program::new(name, bp.clone()));
        }
        let ev = eval.clone();
        world = world.evaluator(Box::new(move |text: &str| match ev(text.trim()) {
            None => Err(format!("no expression {text}")),
            Some(Ok(v)) if v.is_plain() => Ok(v),
            Some(Ok(_)) => Err("a `$`, `!` or `@` value on a check line: a check has no ticks (Conformance ch. 1)".into()),
            Some(Err(f)) => Err(f.message),
        }));
        let before = prism_rt::pod::set_show_hook(Some(crate::values::show_hook()));
        let report = world.run(&text);
        prism_rt::pod::set_show_hook(before);
        if !first {
            println!();
        }
        first = false;
        print!("{}", report.transcript);
        for f in &report.failures {
            eprintln!("{path}: {f}");
        }
        let mut c = report.exit_code;
        let expected = std::path::Path::new(path).with_extension("expected");
        if let Ok(want) = std::fs::read_to_string(&expected) {
            let want = strip_comment_header(&want);
            if want != report.transcript {
                eprintln!("{path}: the transcript differs from its .expected:\n{}", diff(&want, &report.transcript));
                c = c.max(1);
            }
        }
        code = code.max(c);
    }
    code
}
