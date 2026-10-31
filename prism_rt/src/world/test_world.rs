//! The Test World: runs a script in the test language against a program and prints its
//! transcript, to the byte (Conformance ch. 1 and Appendix A).
//!
//! What it keeps from the book:
//!
//! - **One line is one tick** (`tick n` headers, numbered from 0), except `compute until
//!   idle`, which opens as many no-input ticks as the program's `pending` asks for and
//!   prints their count (decisions.md D6).
//! - **Nothing arrives that the test did not write.** It never opens a tick on its own
//!   (round 47 OT), never answers a request, and answers first-value queries only from
//!   the ready state the test's `with` and `push` entries write.
//! - **The transcript's sections**, each omitted when empty, in this order: `in:`,
//!   `added:`, `changed:`, `torn down:`, `requested:`, `result:`; `aborted` in place of
//!   everything after `in:` for a tick that failed, followed by `| ...` lines that are
//!   this implementation's own wording. Every section lists its lines in path order.
//! - **Checks** (`=> expect ...`) are reported as failures beside the transcript; a step
//!   whose tick aborted with no `=> aborted` under it fails the test.
//!
//! The steps: `build`, `tick`, `compute`, `compute until idle`, `with E = v`, `push E =
//! v`, `send p, v`, `click p`, `send at path, v`, `click at path`, several deliveries
//! joined by commas. `answer` (the seeded deck's policy) is not supported here. Since
//! there is no Prism parser yet, delivered values, `with` entries and patterns are read in
//! the transcript's own spelling (`pod`), and a `check` block's lines are `value =>
//! pattern` in that spelling unless the World is given an evaluator.
//!
//! `click at path` on a path where no place is, but where a window's scene carries
//! `Draw.Clickable(rect, path)`, delivers a `Press` at the centre of the rectangle to the
//! window (decisions.md D9).
//!
//! Exit codes, as the CLI's: 0 when every test passes, 1 when a check fails or a tick
//! aborts unexpected, 2 when the script does not parse.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::rc::Rc;

use im_rc::OrdMap;

use crate::document::{elements_within, find_clickable, ReadyMap};
use crate::gc;
use crate::pod::{self, Pattern};
use crate::tick::{tick, Inputs, Outputs, Program, ResultReport, State};
use crate::value::{Fault, Path, Value};

/// The most ticks `compute until idle` opens before it gives up: the Test World's
/// patience.
pub const COMPUTE_PATIENCE: usize = 100_000;

/// A function that evaluates the Prism expression of a `check` line. Without one, a check
/// line's left side is read as a plain value in the transcript's spelling.
pub type Evaluator = Box<dyn Fn(&str) -> Result<Value, String>>;

/// The Test World: programs by name, and an optional evaluator for `check` lines.
#[derive(Default)]
pub struct TestWorld {
    programs: BTreeMap<String, Program>,
    evaluator: Option<Evaluator>,
}

/// What running a script gives: the transcript, the failures, and the exit code.
#[derive(Debug, Clone)]
pub struct Report {
    pub transcript: String,
    pub failures: Vec<String>,
    pub exit_code: u8,
}

impl TestWorld {
    pub fn new() -> TestWorld {
        TestWorld::default()
    }

    /// Register a program a `test name, @program:` line can name.
    pub fn program(mut self, program: Program) -> TestWorld {
        self.programs.insert(program.name.clone(), program);
        self
    }

    pub fn evaluator(mut self, e: Evaluator) -> TestWorld {
        self.evaluator = Some(e);
        self
    }

    /// Run a script and print its transcript.
    pub fn run(&self, script: &str) -> Report {
        let blocks = match parse_script(script) {
            Ok(b) => b,
            Err(e) => {
                return Report { transcript: String::new(), failures: vec![e], exit_code: 2 };
            }
        };
        let mut transcript = String::new();
        let mut failures = Vec::new();
        for (i, block) in blocks.iter().enumerate() {
            if i > 0 {
                transcript.push('\n');
            }
            match block {
                Block::Test(t) => self.run_test(t, &mut transcript, &mut failures),
                Block::Check(c) => self.run_check(c, &mut transcript, &mut failures),
            }
        }
        let exit_code = if failures.is_empty() { 0 } else { 1 };
        Report { transcript, failures, exit_code }
    }

    /// Run a script and compare its transcript with `expected`, byte for byte. Fails with
    /// a line-by-line difference, or with the checks that failed.
    pub fn verify(&self, script: &str, expected: &str) -> Result<Report, String> {
        let report = self.run(script);
        if report.exit_code == 2 {
            return Err(format!("the script did not parse: {}", report.failures.join("; ")));
        }
        if report.transcript != expected {
            return Err(format!("the transcript differs:\n{}", diff(expected, &report.transcript)));
        }
        if !report.failures.is_empty() {
            return Err(format!("checks failed:\n{}", report.failures.join("\n")));
        }
        Ok(report)
    }

    fn run_check(&self, c: &CheckBlock, out: &mut String, failures: &mut Vec<String>) {
        let _ = writeln!(out, "check {}", c.name);
        for (i, (expr, pat, line)) in c.lines.iter().enumerate() {
            let value = match &self.evaluator {
                Some(eval) => eval(expr),
                None => pod::read(expr),
            };
            let ok = match &value {
                Ok(v) => pat.matches(v),
                Err(_) => false,
            };
            if ok {
                let _ = writeln!(out, "{}: ok", i + 1);
            } else {
                let shown = match value {
                    Ok(v) => pod::print(&v),
                    Err(e) => format!("(error: {e})"),
                };
                let _ = writeln!(out, "{}: {}", i + 1, shown);
                failures.push(format!("check {} line {line}: {expr} was {shown}", c.name));
            }
        }
    }

    fn run_test(&self, t: &TestBlock, out: &mut String, failures: &mut Vec<String>) {
        let _ = writeln!(out, "test {}", t.name);
        let Some(program) = self.programs.get(&t.program) else {
            failures.push(format!("test {}: no program named @{}", t.name, t.program));
            return;
        };
        let mut run = Run { program, state: None, ready: OrdMap::new(), next_tick: 0, pending: false, last_result: None };
        for step in &t.steps {
            if let Err(e) = run.step(step, out, failures, &t.name) {
                failures.push(format!("test {} line {}: {e}", t.name, step.line));
                return;
            }
        }
    }
}

/// One test's run: the program's state between steps and the ready state the test wrote.
struct Run<'a> {
    program: &'a Program,
    state: Option<State>,
    ready: OrdMap<Value, Value>,
    next_tick: usize,
    pending: bool,
    last_result: Option<Value>,
}

/// What a step's last tick left, for its checks.
struct Outcome {
    outputs: Result<Outputs, Fault>,
    before: crate::document::Document,
}

impl Run<'_> {
    fn inputs(&self) -> Inputs {
        Inputs::with_ready(Rc::new(ReadyMap(self.ready.clone())))
    }

    fn step(&mut self, step: &Step, out: &mut String, failures: &mut Vec<String>, test: &str) -> Result<(), String> {
        for (e, v) in &step.withs {
            self.ready.insert(e.clone(), v.clone());
        }
        let outcome = match &step.kind {
            StepKind::Build => {
                if self.state.is_some() {
                    return Err("`build` after the program was built".into());
                }
                let inputs = self.inputs();
                self.one_tick(inputs, Vec::new(), out)
            }
            StepKind::Tick | StepKind::Compute => {
                self.need_built()?;
                let inputs = self.inputs();
                self.one_tick(inputs, Vec::new(), out)
            }
            StepKind::ComputeUntilIdle => {
                self.need_built()?;
                let mut ticks = String::new();
                let mut count = 0;
                let mut last = None;
                while self.pending {
                    if count == COMPUTE_PATIENCE {
                        return Err(format!("`compute until idle` ran out of patience after {count} ticks"));
                    }
                    let inputs = self.inputs();
                    let o = self.one_tick(inputs, Vec::new(), &mut ticks);
                    count += 1;
                    let aborted = o.outputs.is_err();
                    last = Some(o);
                    if aborted {
                        break;
                    }
                }
                let _ = writeln!(out, "\ncompute until idle: {count}");
                out.push_str(&ticks);
                match last {
                    Some(o) => o,
                    None => Outcome {
                        outputs: Ok(Outputs::default()),
                        before: self.state.as_ref().map(|s| s.document().clone()).unwrap_or_default(),
                    },
                }
            }
            StepKind::Push(e, v) => {
                self.need_built()?;
                self.ready.insert(e.clone(), v.clone());
                let inputs = self.inputs().push(e.clone(), v.clone());
                let shown = vec![format!("{} = {}", pod::print(e), pod::print(v))];
                self.one_tick(inputs, shown, out)
            }
            StepKind::Deliver(ds) => {
                self.need_built()?;
                let doc = self.state.as_ref().map(|s| s.document().clone()).unwrap_or_default();
                let mut inputs = self.inputs();
                let mut seen: Vec<Path> = Vec::new();
                for d in ds {
                    let (path, value) = resolve(&doc, d)?;
                    if seen.contains(&path) {
                        return Err(format!("two deliveries to one place, {path}"));
                    }
                    seen.push(path.clone());
                    inputs = inputs.answer(path, value);
                }
                inputs.answers.sort_by(|a, b| a.0.cmp(&b.0));
                let shown = inputs.answers.iter().map(|(p, v)| format!("{}: {}", p, pod::print(v))).collect();
                self.one_tick(inputs, shown, out)
            }
        };
        self.check(step, &outcome, failures, test);
        Ok(())
    }

    fn need_built(&self) -> Result<(), String> {
        if self.state.is_none() { Err("the first step of a test is `build`".into()) } else { Ok(()) }
    }

    /// Run one tick, print it, and collect garbage.
    fn one_tick(&mut self, inputs: Inputs, shown_inputs: Vec<String>, out: &mut String) -> Outcome {
        let before = self.state.as_ref().map(|s| s.document().clone()).unwrap_or_default();
        let t = tick(self.program, self.state.take(), &inputs);
        self.state = t.state;
        self.pending = t.pending;
        let n = self.next_tick;
        self.next_tick += 1;
        print_tick(out, n, &shown_inputs, &t.outputs);
        if let Ok(o) = &t.outputs {
            if let Some(ResultReport::Value(v)) = &o.result {
                self.last_result = Some(v.clone());
            }
        }
        if let Some(st) = self.state.as_mut() {
            gc::collect(st);
        }
        Outcome { outputs: t.outputs, before }
    }

    fn check(&self, step: &Step, o: &Outcome, failures: &mut Vec<String>, test: &str) {
        let doc = self.state.as_ref().map(|s| s.document().clone()).unwrap_or_default();
        let mut all = Vec::new();
        for level in doc.values() {
            elements_within(level, &mut all);
        }
        let aborted = o.outputs.is_err();
        let mut expected_abort = false;
        for (check, line) in &step.checks {
            let fail = |msg: String| format!("test {test} line {line}: {msg}");
            let ok = match check {
                Check::Aborted => {
                    expected_abort = true;
                    aborted
                }
                Check::Settles => !aborted,
                _ if aborted => false,
                Check::Expect(p) => all.iter().any(|e| p.matches(e)),
                Check::ExpectNone(p) => !all.iter().any(|e| p.matches(e)),
                Check::ExpectCount(p, n) => all.iter().filter(|e| p.matches(e)).count() == *n,
                Check::ExpectTop(p) => doc.iter().any(|(path, e)| path.len() == 1 && p.matches(e)),
                Check::ExpectTopNone(p) => !doc.iter().any(|(path, e)| path.len() == 1 && p.matches(e)),
                Check::Requested(p, v) | Check::Unrequested(p, v) => {
                    let outputs = o.outputs.as_ref().unwrap();
                    let hit = outputs.requests.iter().any(|r| {
                        let place = doc.get(&r.path).or_else(|| o.before.get(&r.path));
                        place.is_some_and(|e| p.matches(e)) && v.matches(&r.value)
                    });
                    if matches!(check, Check::Requested(..)) { hit } else { !hit }
                }
                Check::Returns(p) => self.last_result.as_ref().is_some_and(|v| p.matches(v)),
                Check::Fired(p) => {
                    matches!(&o.outputs.as_ref().unwrap().result, Some(ResultReport::Fired(v)) if p.matches(v))
                }
                Check::Quiet => !matches!(&o.outputs.as_ref().unwrap().result, Some(ResultReport::Fired(_))),
                Check::Snapshot(f) => {
                    failures.push(fail(format!("snapshot \"{f}\": this World draws no SVG")));
                    continue;
                }
            };
            if !ok {
                failures.push(fail(format!("{} did not hold", check_text(check))));
            }
        }
        if aborted && !expected_abort {
            let why = o.outputs.as_ref().err().map(|f| f.to_string()).unwrap_or_default();
            failures.push(format!("test {test} line {}: the tick aborted: {why}", step.line));
        }
    }
}

/// Resolve a delivery to a path and a value, against the document as the tick opens.
fn resolve(doc: &crate::document::Document, d: &Delivery) -> Result<(Path, Value), String> {
    let (target, value) = match d {
        Delivery::Send(t, v) => (t, v.clone()),
        Delivery::Click(t) => (t, Value::click()),
        Delivery::Answer => return Err("`answer` needs a policy this World does not have".into()),
    };
    match target {
        Target::At(text) => {
            if let Some(path) = doc.keys().find(|p| pod::print_path(p) == *text) {
                return Ok((path.clone(), value));
            }
            if matches!(d, Delivery::Click(_)) {
                if let Some((window, x, y)) = find_clickable(doc, text) {
                    return Ok((window, crate::families::gui::press(x, y)));
                }
            }
            Err(format!("no place at {text}"))
        }
        Target::Pattern(p) => {
            let found: Vec<&Path> = doc.iter().filter(|(_, e)| p.matches(e)).map(|(path, _)| path).collect();
            match found.as_slice() {
                [one] => Ok(((*one).clone(), value)),
                [] => Err(format!("no place matches {p:?}")),
                many => Err(format!(
                    "{} places match: {}",
                    many.len(),
                    many.iter().map(|p| pod::print_path(p)).collect::<Vec<_>>().join(", ")
                )),
            }
        }
    }
}

/// Print one tick in the transcript's format.
pub fn print_tick(out: &mut String, n: usize, inputs: &[String], outputs: &Result<Outputs, Fault>) {
    let _ = writeln!(out, "\ntick {n}");
    if !inputs.is_empty() {
        out.push_str("  in:\n");
        for i in inputs {
            let _ = writeln!(out, "    {i}");
        }
    }
    let o = match outputs {
        Ok(o) => o,
        Err(f) => {
            out.push_str("  aborted\n");
            let _ = writeln!(out, "  | {f}");
            return;
        }
    };
    if !o.patch.added.is_empty() {
        out.push_str("  added:\n");
        for (p, e) in &o.patch.added {
            let _ = writeln!(out, "    {}: {}", p, pod::print(e));
        }
    }
    if !o.patch.changed.is_empty() {
        out.push_str("  changed:\n");
        for (p, _, e) in &o.patch.changed {
            let _ = writeln!(out, "    {}: {}", p, pod::print(e));
        }
    }
    if !o.patch.removed.is_empty() {
        out.push_str("  torn down:\n");
        for (p, e) in &o.patch.removed {
            let _ = writeln!(out, "    {}: {}", p, pod::print(e));
        }
    }
    if !o.requests.is_empty() {
        out.push_str("  requested:\n");
        for r in &o.requests {
            let _ = writeln!(out, "    {r}");
        }
    }
    match &o.result {
        Some(ResultReport::Value(v)) => {
            let _ = writeln!(out, "  result: {}", pod::print(v));
        }
        Some(ResultReport::Fired(v)) => {
            let _ = writeln!(out, "  result: fired: {}", pod::print(v));
        }
        None => {}
    }
}

/// A line-by-line difference, for a failed `verify`.
fn diff(expected: &str, got: &str) -> String {
    let (e, g): (Vec<&str>, Vec<&str>) = (expected.lines().collect(), got.lines().collect());
    let mut out = String::new();
    for i in 0..e.len().max(g.len()) {
        match (e.get(i), g.get(i)) {
            (Some(a), Some(b)) if a == b => {
                let _ = writeln!(out, "  {a}");
            }
            (a, b) => {
                if let Some(a) = a {
                    let _ = writeln!(out, "- {a}");
                }
                if let Some(b) = b {
                    let _ = writeln!(out, "+ {b}");
                }
            }
        }
    }
    if expected.ends_with('\n') != got.ends_with('\n') {
        out.push_str("(the final newline differs)\n");
    }
    out
}

// ---------------------------------------------------------------------------------------
// The script: Conformance Appendix A
// ---------------------------------------------------------------------------------------

enum Block {
    Test(TestBlock),
    Check(CheckBlock),
}

struct TestBlock {
    name: String,
    program: String,
    steps: Vec<Step>,
}

struct CheckBlock {
    name: String,
    lines: Vec<(String, Pattern, usize)>,
}

struct Step {
    line: usize,
    withs: Vec<(Value, Value)>,
    kind: StepKind,
    checks: Vec<(Check, usize)>,
}

enum StepKind {
    Build,
    Tick,
    Compute,
    ComputeUntilIdle,
    Push(Value, Value),
    Deliver(Vec<Delivery>),
}

enum Delivery {
    Send(Target, Value),
    Click(Target),
    /// `answer p`: a policy's answer, which this World does not have.
    Answer,
}

enum Target {
    Pattern(Pattern),
    /// A path as written, compared with the printed paths of the document.
    At(String),
}

enum Check {
    Expect(Pattern),
    ExpectNone(Pattern),
    ExpectCount(Pattern, usize),
    ExpectTop(Pattern),
    ExpectTopNone(Pattern),
    Requested(Pattern, Pattern),
    Unrequested(Pattern, Pattern),
    Returns(Pattern),
    Fired(Pattern),
    Quiet,
    Settles,
    Aborted,
    Snapshot(String),
}

fn check_text(c: &Check) -> String {
    match c {
        Check::Expect(p) => format!("expect {p:?}"),
        Check::ExpectNone(p) => format!("expect_none {p:?}"),
        Check::ExpectCount(p, n) => format!("expect_count {p:?}, {n}"),
        Check::ExpectTop(p) => format!("expect_top {p:?}"),
        Check::ExpectTopNone(p) => format!("expect_top_none {p:?}"),
        Check::Requested(p, v) => format!("requested {p:?}, {v:?}"),
        Check::Unrequested(p, _) => format!("unrequested {p:?}"),
        Check::Returns(p) => format!("returns {p:?}"),
        Check::Fired(p) => format!("fired {p:?}"),
        Check::Quiet => "quiet".into(),
        Check::Settles => "settles".into(),
        Check::Aborted => "aborted".into(),
        Check::Snapshot(f) => format!("snapshot {f:?}"),
    }
}

/// Strip a comment: a `#` at the start of a line or after a space, outside a string
/// (Appendix A.2: inside a path, `#` followed by a digit is a segment).
fn strip_comment(line: &str) -> &str {
    let mut in_str = false;
    let mut prev = ' ';
    for (i, c) in line.char_indices() {
        match c {
            '"' if prev != '\\' => in_str = !in_str,
            '#' if !in_str && prev.is_whitespace() => return &line[..i],
            _ => {}
        }
        prev = c;
    }
    line
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn parse_script(src: &str) -> Result<Vec<Block>, String> {
    let lines: Vec<(usize, &str)> = src
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, strip_comment(l).trim_end()))
        .filter(|(_, l)| !l.trim().is_empty())
        .collect();
    let mut blocks = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let (no, head) = lines[i];
        if indent(head) != 0 {
            return Err(format!("line {no}: expected `test` or `check` at the left edge"));
        }
        let body_end = (i + 1..lines.len()).find(|&j| indent(lines[j].1) == 0).unwrap_or(lines.len());
        let body = &lines[i + 1..body_end];
        if let Some(rest) = head.strip_prefix("test ") {
            blocks.push(Block::Test(parse_test(no, rest, body)?));
        } else if let Some(rest) = head.strip_prefix("check ") {
            blocks.push(Block::Check(parse_check(no, rest, body)?));
        } else {
            return Err(format!("line {no}: expected `test` or `check`"));
        }
        i = body_end;
    }
    Ok(blocks)
}

fn parse_check(no: usize, head: &str, body: &[(usize, &str)]) -> Result<CheckBlock, String> {
    let name = head.strip_suffix(':').ok_or(format!("line {no}: a check line ends with :"))?.trim().to_string();
    let mut lines = Vec::new();
    for (n, l) in body {
        let l = l.trim();
        let (expr, pat) = match l.split_once("=>") {
            Some((e, p)) => (e.trim().to_string(), pod::read_pattern(p.trim()).map_err(|e| format!("line {n}: {e}"))?),
            // A bare line is short for `=> True` (Appendix A, doubt CK).
            None => (l.to_string(), Pattern::Lit(Value::Bool(true))),
        };
        lines.push((expr, pat, *n));
    }
    Ok(CheckBlock { name, lines })
}

fn parse_test(no: usize, head: &str, body: &[(usize, &str)]) -> Result<TestBlock, String> {
    let head = head.strip_suffix(':').ok_or(format!("line {no}: a test line ends with :"))?;
    let parts = split_commas(head);
    if parts.len() < 2 {
        return Err(format!("line {no}: test name, @program:"));
    }
    let name = parts[0].trim().to_string();
    let program = parts[1].trim().strip_prefix('@').ok_or(format!("line {no}: the program is written @name"))?;
    let mut steps: Vec<Step> = Vec::new();
    let step_col = body.first().map(|(_, l)| indent(l)).unwrap_or(4);
    for (n, l) in body {
        let col = indent(l);
        let text = l.trim();
        if col == step_col {
            steps.push(parse_step(*n, text)?);
        } else if col > step_col {
            let Some(step) = steps.last_mut() else { return Err(format!("line {n}: a check before any step")) };
            if let Some(c) = text.strip_prefix("=>") {
                step.checks.push((parse_check_line(*n, c.trim())?, *n));
            } else if step.checks.is_empty() {
                return Err(format!("line {n}: write a long step on one line; continuations are not supported"));
            } else {
                return Err(format!("line {n}: a continuation after a check (Appendix A.1)"));
            }
        } else {
            return Err(format!("line {n}: indentation"));
        }
    }
    Ok(TestBlock { name, program: program.trim().to_string(), steps })
}

fn pattern(n: usize, s: &str) -> Result<Pattern, String> {
    pod::read_pattern(s.trim()).map_err(|e| format!("line {n}: {e}"))
}

fn value(n: usize, s: &str) -> Result<Value, String> {
    pod::read(s.trim()).map_err(|e| format!("line {n}: {e}"))
}

fn parse_check_line(n: usize, c: &str) -> Result<Check, String> {
    let (word, rest) = c.split_once(' ').unwrap_or((c, ""));
    let two = |rest: &str| -> Result<(Pattern, String), String> {
        let parts = split_commas(rest);
        if parts.len() != 2 {
            return Err(format!("line {n}: `{word}` takes two operands"));
        }
        Ok((pattern(n, &parts[0])?, parts[1].trim().to_string()))
    };
    Ok(match word {
        "expect" => Check::Expect(pattern(n, rest)?),
        "expect_none" => Check::ExpectNone(pattern(n, rest)?),
        "expect_count" => {
            let (p, k) = two(rest)?;
            Check::ExpectCount(p, k.parse().map_err(|_| format!("line {n}: a count"))?)
        }
        "expect_top" => Check::ExpectTop(pattern(n, rest)?),
        "expect_top_none" => Check::ExpectTopNone(pattern(n, rest)?),
        "requested" => {
            let (p, v) = two(rest)?;
            Check::Requested(p, pattern(n, &v)?)
        }
        "unrequested" => Check::Unrequested(pattern(n, rest)?, Pattern::Any),
        "returns" => Check::Returns(pattern(n, rest)?),
        "fired" => Check::Fired(pattern(n, rest)?),
        "quiet" => Check::Quiet,
        "settles" => Check::Settles,
        "aborted" => Check::Aborted,
        "snapshot" => Check::Snapshot(rest.trim().trim_matches('"').to_string()),
        other => return Err(format!("line {n}: unknown check `{other}`")),
    })
}

/// Split at commas at bracket depth zero, outside strings (Appendix A.3).
fn split_commas(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut prev = ' ';
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '"' if prev != '\\' => in_str = !in_str,
            '(' | '[' | '{' if !in_str => depth += 1,
            ')' | ']' | '}' if !in_str => depth -= 1,
            ',' if !in_str && depth == 0 => {
                out.push(std::mem::take(&mut cur));
                prev = c;
                continue;
            }
            _ => {}
        }
        cur.push(c);
        prev = c;
    }
    out.push(cur);
    out
}

/// Split `E = v` at the first `=` at depth zero that is not part of `==` or `=>`.
fn split_entry(n: usize, s: &str) -> Result<(Value, Value), String> {
    let cs: Vec<char> = s.chars().collect();
    let mut depth = 0i32;
    let mut in_str = false;
    for i in 0..cs.len() {
        match cs[i] {
            '"' if i == 0 || cs[i - 1] != '\\' => in_str = !in_str,
            '(' | '[' | '{' if !in_str => depth += 1,
            ')' | ']' | '}' if !in_str => depth -= 1,
            '=' if !in_str && depth == 0 && cs.get(i + 1) != Some(&'=') && cs.get(i + 1) != Some(&'>') => {
                let (e, v): (String, String) = (cs[..i].iter().collect(), cs[i + 1..].iter().collect());
                return Ok((value(n, &e)?, value(n, &v)?));
            }
            _ => {}
        }
    }
    Err(format!("line {n}: an entry is `element = value`"))
}

const DELIVERY_WORDS: [&str; 4] = ["send", "click", "answer", "with"];

fn first_word(s: &str) -> &str {
    s.split_whitespace().next().unwrap_or("")
}

fn parse_target(n: usize, s: &str) -> Result<Target, String> {
    let s = s.trim();
    match s.strip_prefix("at ") {
        Some(path) => {
            let p = pod::read_path(path.trim()).map_err(|e| format!("line {n}: {e}"))?;
            Ok(Target::At(pod::print_path(&p)))
        }
        None => Ok(Target::Pattern(pattern(n, s)?)),
    }
}

fn parse_step(n: usize, text: &str) -> Result<Step, String> {
    let parts = split_commas(text);
    let mut withs = Vec::new();
    let mut deliveries = Vec::new();
    let mut kind = None;
    let mut i = 0;
    while i < parts.len() {
        let part = parts[i].trim();
        let word = first_word(part);
        let rest = part[word.len()..].trim();
        match word {
            "with" => withs.push(split_entry(n, rest)?),
            "build" if rest.is_empty() => kind = Some(StepKind::Build),
            "tick" if rest.is_empty() => kind = Some(StepKind::Tick),
            "compute" if rest.is_empty() => kind = Some(StepKind::Compute),
            "compute" if rest == "until idle" => kind = Some(StepKind::ComputeUntilIdle),
            "push" => {
                let (e, v) = split_entry(n, rest)?;
                kind = Some(StepKind::Push(e, v));
            }
            "click" => deliveries.push(Delivery::Click(parse_target(n, rest)?)),
            "answer" => {
                parse_target(n, rest)?;
                deliveries.push(Delivery::Answer);
            }
            "send" => {
                let Some(v) = parts.get(i + 1) else { return Err(format!("line {n}: send p, v")) };
                if DELIVERY_WORDS.contains(&first_word(v)) {
                    return Err(format!("line {n}: send p, v"));
                }
                deliveries.push(Delivery::Send(parse_target(n, rest)?, value(n, v)?));
                i += 1;
            }
            _ => return Err(format!("line {n}: `{part}`: a comma here needs parentheses (Appendix A.3)")),
        }
        i += 1;
    }
    let kind = match (kind, deliveries.is_empty()) {
        (Some(k), true) => k,
        (None, false) => StepKind::Deliver(deliveries),
        (None, true) => return Err(format!("line {n}: a step with no tick")),
        (Some(_), false) => return Err(format!("line {n}: deliveries beside build, tick or push")),
    };
    Ok(Step { line: n, withs, kind, checks: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commas_split_at_depth_zero() {
        assert_eq!(split_commas("click Ui.Button(label=\"a, b\"), click at 1"), vec![
            "click Ui.Button(label=\"a, b\")".to_string(),
            " click at 1".to_string()
        ]);
    }

    #[test]
    fn a_hash_in_a_path_is_not_a_comment() {
        assert_eq!(strip_comment("click at 1/#0/0  # the button"), "click at 1/#0/0  ");
    }

    #[test]
    fn steps_parse() {
        let s = parse_step(1, "with Fs.Contents(path=\"a\") = ~ Contents(text=\"x\"), click at 1/#0/0").unwrap();
        assert_eq!(s.withs.len(), 1);
        assert!(matches!(s.kind, StepKind::Deliver(ref d) if d.len() == 1));
        assert!(matches!(parse_step(2, "compute until idle").unwrap().kind, StepKind::ComputeUntilIdle));
        assert!(parse_step(3, "send Ui.Button(label=\"x\"), Rect 1, 2").is_err());
    }

    #[test]
    fn a_bad_script_exits_2() {
        let r = TestWorld::new().run("  build\n");
        assert_eq!(r.exit_code, 2);
    }

    #[test]
    fn check_blocks_print_ok_or_the_value() {
        let r = TestWorld::new().run("check sums:\n    3 => 3\n    \"a\" => \"b\"\n    True\n");
        assert_eq!(r.transcript, "check sums\n1: ok\n2: \"a\"\n3: ok\n");
        assert_eq!(r.exit_code, 1);
    }
}
