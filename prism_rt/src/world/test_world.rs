//! The Test World: runs a script in the test language against a program and prints its
//! transcript, to the byte (Conformance ch. 1 and Appendix A).
//!
//! What it keeps from the book:
//!
//! - **One line is one tick** (`tick n` headers, numbered from 0), except `compute until
//!   idle`, which opens as many no-input ticks as the program's `pending` asks for and
//!   prints `compute until idle: k ticks, n to m`, then each of its ticks that has a
//!   section; a tick with every section empty prints only in the count (decisions.md D6;
//!   Conformance ch. 1, "The transcript"). With none to run it prints `compute until
//!   idle: 0 ticks`, and with one, `1 tick, n to n` (spellings the book leaves open).
//! - **Nothing arrives that the test did not write.** It never opens a tick on its own
//!   (round 47 OT), never answers a request unless the test writes `answer`, and answers
//!   first-value queries from the ready state the test's `with` and `push` entries
//!   write, and by Pitch ch. 6's policy for pictures: `Gui.Window` with `WindowId title`
//!   (README doubt WI), `Gui.Metrics` from its bundled font, nine units a character and
//!   fourteen high (doubt TM), so a GUI test needs no `with` lines (doubt SW). A `with`
//!   entry for either wins over the policy.
//! - **The transcript's sections**, each omitted when empty, in this order: `in:`,
//!   `added:`, `changed:`, `torn down:`, `requested:`, `result:`; `aborted` in place of
//!   everything after `in:` for a tick that failed, followed by `| ...` lines that are
//!   this implementation's own wording. Every section lists its lines in path order.
//! - **Checks** (`=> expect ...`) are reported as failures beside the transcript; a step
//!   whose tick aborted with no `=> aborted` under it fails the test.
//!
//! The steps: `build`, `tick`, `compute`, `compute until idle`, `with E = v`, `push E =
//! v`, `send p, v`, `click p`, `send at path, v`, `click at path`, `click element path`,
//! `answer p`, several deliveries joined by commas. Since the runtime has no Prism
//! parser, delivered values, `with` entries and patterns are read in the transcript's own
//! spelling (`pod`), and a `check` block's lines are `value => pattern` in that spelling
//! unless the World is given an evaluator.
//!
//! - **`click element path`** (decisions.md D9): the one `Draw.Clickable` naming `path`
//!   in the windows' last scenes (the document as the tick opens) gives a `Press` at its
//!   rectangle's centre, delivered to the `Gui.Window` place whose id the scene names. A
//!   path is matched by its printed spelling, a key by its `^Show` (README doubt KY).
//!   None found, or two, fails the line, naming every window searched.
//! - **`answer p`** answers the pending `Cards.Deck` request of the one place matching
//!   `p` with the next card of its shuffle (Conformance ch. 1, "The seeded deck"): one
//!   xorshift64* generator per test, seeded by the test line's `seed n`; each new deck
//!   place takes 51 outputs from it as it is built, path order within a tick.
//!
//! Exit codes, as the CLI's: 0 when every test passes, 1 when a check fails or a tick
//! aborts unexpected, 2 when the script does not parse.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::rc::Rc;

use im_rc::OrdMap;

use crate::document::{elements_within, find_clickables, ReadyState};
use crate::families::{cards, gui};
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
        let mut run = Run {
            program,
            state: None,
            ready: OrdMap::new(),
            next_tick: 0,
            pending: false,
            last_result: None,
            windows: BTreeMap::new(),
            rng: t.seed.map(XorShift::new),
            decks: BTreeMap::new(),
        };
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
    /// Each `Gui.Window` place's id, as its first value answered it.
    windows: BTreeMap<Path, Value>,
    /// The test's generator, when its line names a seed.
    rng: Option<XorShift>,
    /// Each `Cards.Deck` place's shuffle and its requests not yet answered.
    decks: BTreeMap<Path, Deck>,
}

/// A deck place's shuffle, the next position to draw, and its pending requests.
struct Deck {
    cards: Vec<Value>,
    next: usize,
    pending: usize,
}

/// xorshift64*, the generator every implementation shares (Conformance ch. 1, "The
/// seeded deck"; round 47 SD).
pub struct XorShift(u64);

impl XorShift {
    /// The state is the seed, or `0x9E3779B97F4A7C15` for the seed 0, since the state
    /// must not be zero.
    pub fn new(seed: u64) -> XorShift {
        XorShift(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_output(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// A deck shuffled by Fisher-Yates from the top: for `i` from 51 down to 1, `j` is
    /// the next output modulo `i + 1`, and positions `i` and `j` swap.
    pub fn shuffle(&mut self) -> Vec<Value> {
        let mut deck = cards::unshuffled();
        for i in (1..deck.len()).rev() {
            let j = (self.next_output() % (i as u64 + 1)) as usize;
            deck.swap(i, j);
        }
        deck
    }
}

/// The Test World's ready state: the test's `with` entries, then Pitch ch. 6's policy for
/// pictures.
struct TestReady(OrdMap<Value, Value>);

impl ReadyState for TestReady {
    fn first_value(&self, element: &Value) -> Option<Value> {
        if let Some(v) = self.0.get(element) {
            return Some(v.clone());
        }
        let e = element.as_element()?;
        match &*e.tag {
            "Gui.Window" => Some(gui::window_id(&e.field("title").and_then(Value::as_text)?.to_string())),
            "Gui.Metrics" => Some(gui::test_font_extent(&e.field("text").and_then(Value::as_text)?.to_string())),
            _ => None,
        }
    }
}

/// What a step's last tick left, for its checks.
struct Outcome {
    outputs: Result<Outputs, Fault>,
    before: crate::document::Document,
}

impl Run<'_> {
    fn inputs(&self) -> Inputs {
        Inputs::with_ready(Rc::new(TestReady(self.ready.clone())))
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
                let first = self.next_tick;
                let mut ticks = String::new();
                let mut count = 0;
                let mut last = None;
                while self.pending {
                    if count == COMPUTE_PATIENCE {
                        return Err(format!("`compute until idle` ran out of patience after {count} ticks"));
                    }
                    let inputs = self.inputs();
                    let mut one = String::new();
                    let o = self.one_tick(inputs, Vec::new(), &mut one);
                    // A tick with every section empty prints only in the count.
                    if one.lines().count() > 2 {
                        ticks.push_str(&one);
                    }
                    count += 1;
                    let aborted = o.outputs.is_err();
                    last = Some(o);
                    if aborted {
                        break;
                    }
                }
                let _ = match count {
                    0 => writeln!(out, "\ncompute until idle: 0 ticks"),
                    1 => writeln!(out, "\ncompute until idle: 1 tick, {first} to {first}"),
                    k => writeln!(out, "\ncompute until idle: {k} ticks, {first} to {}", first + k - 1),
                };
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
                    let (path, value) = match d {
                        Delivery::Answer(p) => self.answer(&doc, p)?,
                        Delivery::Element(target) => self.click_element(&doc, target)?,
                        _ => resolve(&doc, d)?,
                    };
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

    /// `answer p`: the next card of the matching deck place's shuffle, for its pending
    /// request.
    fn answer(&mut self, doc: &crate::document::Document, p: &Pattern) -> Result<(Path, Value), String> {
        let path = one_place(doc, p)?;
        if self.rng.is_none() {
            return Err("`answer` deals from the test's seed, and this test names none (`test name, @w, seed n:`)".into());
        }
        let Some(deck) = self.decks.get_mut(&path) else {
            return Err(format!("{path} is not a Cards.Deck place: `answer` has a policy for decks only"));
        };
        if deck.pending == 0 {
            return Err(format!("the deck at {path} has no pending request to answer"));
        }
        let Some(card) = deck.cards.get(deck.next).cloned() else {
            return Err(format!("the deck at {path} has dealt all 52 cards"));
        };
        deck.next += 1;
        deck.pending -= 1;
        Ok((path, card))
    }

    /// `click element path`: a `Press` at the centre of the one `Draw.Clickable` naming
    /// `path` in the windows' last scenes, to the window the scene draws.
    fn click_element(&self, doc: &crate::document::Document, target: &str) -> Result<(Path, Value), String> {
        let searched = || {
            let ws: Vec<String> = self.windows.iter().map(|(p, id)| format!("{p} ({})", pod::print(id))).collect();
            if ws.is_empty() { "no window".to_string() } else { ws.join(", ") }
        };
        let found = find_clickables(doc, target);
        let c = match found.as_slice() {
            [one] => one,
            [] => return Err(format!("no Draw.Clickable names {target}; searched {}", searched())),
            many => {
                return Err(format!(
                    "{} Draw.Clickables name {target}, in the scenes at {}; searched {}",
                    many.len(),
                    many.iter().map(|c| c.scene.to_string()).collect::<Vec<_>>().join(", "),
                    searched()
                ));
            }
        };
        let windows: Vec<&Path> = self.windows.iter().filter(|(_, id)| **id == c.window).map(|(p, _)| p).collect();
        match windows.as_slice() {
            [w] => Ok(((*w).clone(), gui::press(c.x, c.y))),
            [] => Err(format!("the scene at {} draws for {}, and no Gui.Window has that id; searched {}", c.scene, pod::print(&c.window), searched())),
            _ => Err(format!("two windows have the id {}; searched {}", pod::print(&c.window), searched())),
        }
    }

    /// What a tick's outputs tell the Test World's policies: new windows' ids, new decks'
    /// shuffles, decks' requests, and places that left.
    fn observe(&mut self, o: &Outputs) {
        for (p, _) in &o.patch.removed {
            self.windows.remove(p);
            self.decks.remove(p);
        }
        let ready = TestReady(self.ready.clone());
        for (p, e) in &o.patch.added {
            let Some(el) = e.as_element() else { continue };
            match &*el.tag {
                "Gui.Window" => {
                    if let Some(id) = ready.first_value(e) {
                        self.windows.insert(p.clone(), id);
                    }
                }
                "Cards.Deck" => {
                    if let Some(rng) = self.rng.as_mut() {
                        self.decks.insert(p.clone(), Deck { cards: rng.shuffle(), next: 0, pending: 0 });
                    }
                }
                _ => {}
            }
        }
        for r in &o.requests {
            if let Some(d) = self.decks.get_mut(&r.path) {
                d.pending += 1;
            }
        }
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
            self.observe(o);
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

/// Resolve a `send` or `click` to a path and a value, against the document as the tick
/// opens.
fn resolve(doc: &crate::document::Document, d: &Delivery) -> Result<(Path, Value), String> {
    let (target, value) = match d {
        Delivery::Send(t, v) => (t, v.clone()),
        Delivery::Click(t) => (t, Value::click()),
        Delivery::Answer(_) | Delivery::Element(_) => unreachable!("resolved by the run"),
    };
    match target {
        Target::At(text) => match doc.keys().find(|p| pod::print_path(p) == *text) {
            Some(path) => Ok((path.clone(), value)),
            None => Err(format!("no place at {text}")),
        },
        Target::Pattern(p) => Ok((one_place(doc, p)?, value)),
    }
}

/// The one place of the document matching a pattern.
fn one_place(doc: &crate::document::Document, p: &Pattern) -> Result<Path, String> {
    let found: Vec<&Path> = doc.iter().filter(|(_, e)| p.matches(e)).map(|(path, _)| path).collect();
    match found.as_slice() {
        [one] => Ok((*one).clone()),
        [] => Err(format!("no place matches {p:?}")),
        many => Err(format!(
            "{} places match: {}",
            many.len(),
            many.iter().map(|p| pod::print_path(p)).collect::<Vec<_>>().join(", ")
        )),
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
    /// `seed n` on the test line: the seeded deck's generator.
    seed: Option<u64>,
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
    /// `click element path`: the path as written, compared with the printed paths of the
    /// scenes' `Draw.Clickable`s.
    Element(String),
    /// `answer p`: the seeded deck's answer to the matching place's pending request.
    Answer(Pattern),
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
    let mut seed = None;
    for extra in &parts[2..] {
        match extra.trim().strip_prefix("seed ") {
            Some(n) => seed = Some(n.trim().parse::<u64>().map_err(|_| format!("line {no}: a seed is a whole number"))?),
            None => return Err(format!("line {no}: after the program, a test line takes `seed n` only")),
        }
    }
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
    Ok(TestBlock { name, program: program.trim().to_string(), seed, steps })
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
            "click" => match rest.strip_prefix("element ") {
                Some(path) => {
                    let p = pod::read_path(path.trim()).map_err(|e| format!("line {n}: {e}"))?;
                    deliveries.push(Delivery::Element(pod::print_path(&p)));
                }
                None => deliveries.push(Delivery::Click(parse_target(n, rest)?)),
            },
            "answer" => deliveries.push(Delivery::Answer(pattern(n, rest)?)),
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
    fn seed_seven_deals_the_four_of_spades_first() {
        let deck = XorShift::new(7).shuffle();
        assert_eq!(pod::print(&deck[0]), "Card(rank=Four, suit=Spades)");
        let mut all: Vec<String> = deck.iter().map(pod::print).collect();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), 52);
    }

    #[test]
    fn test_lines_take_a_seed() {
        let b = parse_script("test t, @w, seed 7:\n    build\n").unwrap();
        assert!(matches!(&b[0], Block::Test(t) if t.seed == Some(7)));
        assert!(parse_script("test t, @w, sed 7:\n    build\n").is_err());
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
