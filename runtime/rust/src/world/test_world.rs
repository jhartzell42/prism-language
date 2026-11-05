//! The Test World: runs a script in the test language against any [`Program`] and prints
//! its transcript to the byte (Conformance ch. 1 and Appendix A).
//!
//! - **One line is one tick,** headed `tick n` from 0, except `compute until idle`, which
//!   ticks while the program says `pending` and prints `compute until idle: k ticks, n to
//!   m`, then each of its ticks that has a section (decisions.md D6). With none to run it
//!   prints `0 ticks`, and with one, `1 tick, n to n`.
//! - **Nothing arrives that the test did not write.** It never opens a tick on its own,
//!   never answers a request unless the test writes `answer`, and answers first-value
//!   queries from the `with` and `push` entries, then by Pitch ch. 6's policy for
//!   pictures: `Gui.Window` with `WindowId title`, `Gui.Metrics` from a font nine units a
//!   character and fourteen high.
//! - **Sections**, each omitted when empty: `in:`, `added:`, `changed:`, `torn down:`,
//!   `requested:`, `result:`; `aborted` in place of everything after `in:` for a failed
//!   tick, with `| ...` lines of this implementation's own wording.
//! - **Checks** (`=> expect ..`) are reported as failures beside the transcript.
//!
//! Steps: `build`, `tick`, `compute`, `compute until idle`, `with E = v`, `push E = v`,
//! `send p, v`, `click p`, `send at path, v`, `click at path`, `click element path`,
//! `answer p` (the seeded deck), several deliveries joined by commas. Values and
//! patterns are read in the transcript's spelling ([`crate::read`]). A `check` block runs
//! the lines a program registered for it with [`TestWorld::check`]; with none, each line's
//! left side is read as a value.
//!
//! A tick that aborts leaves no state to go on from, since this implementation keeps no
//! checkpoints: the test's later steps fail (doubt AB in claude/backend.md).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::doc::Document;
use crate::pod::{F64, Fault, Path, Pod};
use crate::read::{self, Pattern};
use crate::world::scene::find_clickables;
use crate::world::{Outcome, Program, ReadyState, TickIn, TickOut};

/// The most ticks `compute until idle` opens: the Test World's patience.
pub const COMPUTE_PATIENCE: usize = 100_000;

type Builder = Box<dyn Fn(&[Pod], &dyn ReadyState) -> Result<(Box<dyn Program>, TickOut), Fault>>;
type Lines = Box<dyn Fn() -> Vec<Result<Pod, Fault>>>;

/// The programs a `test name, @program:` line can name, and the compiled `check` blocks.
#[derive(Default)]
pub struct TestWorld {
    programs: BTreeMap<String, Builder>,
    checks: BTreeMap<String, Lines>,
}

/// A script's transcript, its failures, and the exit code: 0 when every test passes, 1
/// when a check fails, 2 when the script does not parse.
#[derive(Debug, Clone)]
pub struct Report {
    pub transcript: String,
    pub failures: Vec<String>,
    pub exit_code: u8,
}

/// An `.expected` file without its leading `# ` comment lines.
pub fn strip_comment_header(s: &str) -> String {
    let mut lines = s.split_inclusive('\n').peekable();
    while lines.peek().is_some_and(|l| l.starts_with("# ") || l.trim_end() == "#") {
        lines.next();
    }
    lines.collect()
}

impl TestWorld {
    pub fn new() -> TestWorld {
        TestWorld::default()
    }

    /// Register a program under its root's name, as a test line writes it after `@`.
    pub fn program<P: Program + 'static>(mut self, name: &str) -> TestWorld {
        let build: Builder = Box::new(|args, ready| P::build(args, ready).map(|(p, o)| (Box::new(p) as Box<dyn Program>, o)));
        self.programs.insert(name.to_string(), build);
        self
    }

    /// Register a compiled `check` block: each line's value, in order.
    pub fn check(mut self, name: &str, lines: impl Fn() -> Vec<Result<Pod, Fault>> + 'static) -> TestWorld {
        self.checks.insert(name.to_string(), Box::new(lines));
        self
    }

    /// Run a script and print its transcript.
    pub fn run(&self, script: &str) -> Report {
        let blocks = match parse_script(script) {
            Ok(b) => b,
            Err(e) => return Report { transcript: String::new(), failures: vec![e], exit_code: 2 },
        };
        let (mut out, mut failures) = (String::new(), Vec::new());
        for (i, block) in blocks.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            match block {
                Block::Test(t) => self.run_test(t, &mut out, &mut failures),
                Block::Check(c) => self.run_check(c, &mut out, &mut failures),
            }
        }
        let exit_code = if failures.is_empty() { 0 } else { 1 };
        Report { transcript: out, failures, exit_code }
    }

    /// Run a script and compare its transcript with an `.expected` file's, byte for byte.
    pub fn verify(&self, script: &str, expected: &str) -> Result<Report, String> {
        let report = self.run(script);
        let expected = strip_comment_header(expected);
        if report.exit_code == 2 {
            Err(format!("the script did not parse: {}", report.failures.join("; ")))
        } else if report.transcript != expected {
            Err(format!("the transcript differs:\n{}", diff(&expected, &report.transcript)))
        } else if !report.failures.is_empty() {
            Err(format!("checks failed:\n{}", report.failures.join("\n")))
        } else {
            Ok(report)
        }
    }

    fn run_check(&self, c: &CheckBlock, out: &mut String, failures: &mut Vec<String>) {
        let _ = writeln!(out, "check {}", c.name);
        let compiled = self.checks.get(&c.name).map(|f| f());
        for (i, (expr, pat, line)) in c.lines.iter().enumerate() {
            let value = match &compiled {
                Some(vs) => vs.get(i).cloned().unwrap_or_else(|| Err(Fault::new("Test.Check", "no compiled line"))).map_err(|f| f.to_string()),
                None => read::value(expr),
            };
            match value {
                Ok(v) if pat.matches(&v) => {
                    let _ = writeln!(out, "{}: ok", i + 1);
                }
                v => {
                    let shown = v.map_or_else(|e| format!("(error: {e})"), |v| v.to_string());
                    let _ = writeln!(out, "{}: {shown}", i + 1);
                    failures.push(format!("check {} line {line}: {expr} was {shown}", c.name));
                }
            }
        }
    }

    fn run_test(&self, t: &TestBlock, out: &mut String, failures: &mut Vec<String>) {
        let _ = writeln!(out, "test {}", t.name);
        let Some(builder) = self.programs.get(&t.program) else {
            failures.push(format!("test {}: no program named @{}", t.name, t.program));
            return;
        };
        let mut run = Run {
            builder,
            args: t.args.clone(),
            program: None,
            doc: Document::new(),
            ready: BTreeMap::new(),
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

/// One test's run.
struct Run<'a> {
    builder: &'a Builder,
    args: Vec<Pod>,
    program: Option<Box<dyn Program>>,
    /// The document as the World has it, from the patches.
    doc: Document,
    ready: BTreeMap<Pod, Pod>,
    next_tick: usize,
    pending: bool,
    last_result: Option<Pod>,
    /// Each `Gui.Window` place's id, as its first value answered it.
    windows: BTreeMap<Path, Pod>,
    rng: Option<XorShift>,
    /// Each `Cards.Deck` place's shuffle, the next card, and its unanswered requests.
    decks: BTreeMap<Path, (Vec<Pod>, usize, usize)>,
}

/// xorshift64*, the generator every implementation shares (Conformance ch. 1, "The
/// seeded deck"): the state is the seed, or `0x9E3779B97F4A7C15` for 0.
pub struct XorShift(u64);

impl XorShift {
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
    /// Clubs, Diamonds, Hearts, Spades, each Two to Ace, shuffled by Fisher-Yates from
    /// the top: for `i` from 51 down to 1, `j` is the next output modulo `i + 1`.
    pub fn shuffle(&mut self) -> Vec<Pod> {
        const SUITS: [&str; 4] = ["Clubs", "Diamonds", "Hearts", "Spades"];
        const RANKS: [&str; 13] = ["Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Ten", "Jack", "Queen", "King", "Ace"];
        let mut deck: Vec<Pod> = SUITS
            .iter()
            .flat_map(|s| RANKS.iter().map(move |r| Pod::record("Card", vec![("rank", Pod::name(r)), ("suit", Pod::name(s))])))
            .collect();
        for i in (1..deck.len()).rev() {
            let j = (self.next_output() % (i as u64 + 1)) as usize;
            deck.swap(i, j);
        }
        deck
    }
}

/// The Test World's ready state: the test's entries, then the policy for pictures.
struct TestReady<'a>(&'a BTreeMap<Pod, Pod>);

impl ReadyState for TestReady<'_> {
    fn first_value(&self, element: &Pod) -> Option<Pod> {
        if let Some(v) = self.0.get(element) {
            return Some(v.clone());
        }
        let text = |f| element.field(f).and_then(Pod::as_text).map(|t| t.to_string());
        match element.tag()? {
            "Gui.Window" => Some(Pod::Choice("WindowId".into(), vec![Pod::text(text("title")?)])),
            "Gui.Metrics" => {
                let w = 9.0 * text("text")?.chars().count() as f64;
                Some(Pod::record("Extent", vec![("w", Pod::Float(F64(w))), ("h", Pod::Float(F64(14.0)))]))
            }
            _ => None,
        }
    }
}

/// What a step's last tick left, for its checks.
struct Ticked {
    out: Result<TickOut, Fault>,
    before: Document,
}

impl Run<'_> {
    fn step(&mut self, step: &Step, out: &mut String, failures: &mut Vec<String>, test: &str) -> Result<(), String> {
        for (e, v) in &step.withs {
            self.ready.insert(e.clone(), v.clone());
        }
        if !matches!(step.kind, StepKind::Build) && self.program.is_none() {
            return Err(if self.next_tick == 0 { "the first step of a test is `build`" } else { "an earlier tick aborted" }.into());
        }
        let ticked = match &step.kind {
            StepKind::Build if self.next_tick > 0 => return Err("`build` after the program was built".into()),
            StepKind::Build | StepKind::Tick => self.one_tick(TickIn::none(), &[], out),
            StepKind::ComputeUntilIdle => self.until_idle(out)?,
            StepKind::Push(e, v) => {
                self.ready.insert(e.clone(), v.clone());
                self.one_tick(TickIn::none().push(e.clone(), v.clone()), &[format!("{e} = {v}")], out)
            }
            StepKind::Deliver(ds) => {
                let mut input = TickIn::none();
                for d in ds {
                    let (path, value) = match d {
                        Delivery::Answer(p) => self.deal(p)?,
                        Delivery::Element(target) => self.click_element(target)?,
                        Delivery::Send(t, v) => (self.place(t)?, v.clone()),
                        Delivery::Click(t) => (self.place(t)?, Pod::name("click")),
                    };
                    if input.answers.iter().any(|(p, _)| *p == path) {
                        return Err(format!("two deliveries to one place, {path}"));
                    }
                    input.answers.push((path, value));
                }
                input.answers.sort_by(|a, b| a.0.cmp(&b.0));
                let shown: Vec<String> = input.answers.iter().map(|(p, v)| format!("{p}: {v}")).collect();
                self.one_tick(input, &shown, out)
            }
        };
        self.check(step, &ticked, failures, test);
        Ok(())
    }

    fn until_idle(&mut self, out: &mut String) -> Result<Ticked, String> {
        let (first, mut count, mut ticks) = (self.next_tick, 0, String::new());
        let mut last = Ticked { out: Ok(TickOut::default()), before: self.doc.clone() };
        while self.pending {
            if count == COMPUTE_PATIENCE {
                return Err(format!("`compute until idle` ran out of patience after {count} ticks"));
            }
            let mut one = String::new();
            last = self.one_tick(TickIn::none(), &[], &mut one);
            // A tick with every section empty prints only in the count.
            if one.lines().count() > 2 {
                ticks.push_str(&one);
            }
            count += 1;
            if last.out.is_err() {
                break;
            }
        }
        let _ = match count {
            0 => writeln!(out, "\ncompute until idle: 0 ticks"),
            1 => writeln!(out, "\ncompute until idle: 1 tick, {first} to {first}"),
            k => writeln!(out, "\ncompute until idle: {k} ticks, {first} to {}", first + k - 1),
        };
        out.push_str(&ticks);
        Ok(last)
    }

    /// The one place a `send` or `click` names, in the document as the tick opens.
    fn place(&self, t: &Target) -> Result<Path, String> {
        match t {
            Target::At(text) => self.doc.iter().find(|(p, _)| p.to_string() == *text).map(|(p, _)| p.clone()).ok_or(format!("no place at {text}")),
            Target::Pattern(pat) => {
                let found: Vec<&Path> = self.doc.iter().filter(|(_, e)| pat.matches(e)).map(|(p, _)| p).collect();
                match found.as_slice() {
                    [one] => Ok((*one).clone()),
                    [] => Err(format!("no place matches {pat:?}")),
                    many => Err(format!("{} places match: {}", many.len(), many.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", "))),
                }
            }
        }
    }

    /// `answer p`: the next card of the matching deck place's shuffle.
    fn deal(&mut self, p: &Pattern) -> Result<(Path, Pod), String> {
        let path = self.place(&Target::Pattern(p.clone()))?;
        if self.rng.is_none() {
            return Err("`answer` deals from the test's seed, and this test names none".into());
        }
        let (cards, next, pending) = self.decks.get_mut(&path).ok_or(format!("{path} is not a Cards.Deck place"))?;
        if *pending == 0 {
            return Err(format!("the deck at {path} has no pending request to answer"));
        }
        let card = cards.get(*next).cloned().ok_or(format!("the deck at {path} has dealt all 52 cards"))?;
        *next += 1;
        *pending -= 1;
        Ok((path, card))
    }

    /// `click element path`: a `Press` at the centre of the one `Draw.Clickable` naming
    /// `path` in the windows' last scenes, to the window the scene draws.
    fn click_element(&self, target: &str) -> Result<(Path, Pod), String> {
        let searched = self.windows.iter().map(|(p, id)| format!("{p} ({id})")).collect::<Vec<_>>().join(", ");
        let found = find_clickables(&self.doc, target);
        let c = match found.as_slice() {
            [one] => one,
            [] => return Err(format!("no Draw.Clickable names {target}; searched {searched}")),
            many => return Err(format!("{} Draw.Clickables name {target}; searched {searched}", many.len())),
        };
        let windows: Vec<&Path> = self.windows.iter().filter(|(_, id)| **id == c.window).map(|(p, _)| p).collect();
        let [w] = windows.as_slice() else {
            return Err(format!("the scene at {} draws for {}, and {} windows have that id", c.scene, c.window, windows.len()));
        };
        let point = Pod::record("Point", vec![("x", Pod::Float(F64(c.x))), ("y", Pod::Float(F64(c.y)))]);
        Ok(((*w).clone(), Pod::Choice("Press".into(), vec![point])))
    }

    /// Run one tick, print it, and lend the program its compact round.
    fn one_tick(&mut self, input: TickIn, shown: &[String], out: &mut String) -> Ticked {
        let before = self.doc.clone();
        let ready = TestReady(&self.ready);
        let result = match self.program.as_mut() {
            Some(p) => p.tick(&input, &ready),
            None => (self.builder)(&self.args, &ready).map(|(p, o)| {
                self.program = Some(p);
                o
            }),
        };
        print_tick(out, self.next_tick, shown, &result);
        self.next_tick += 1;
        match &result {
            Ok(o) => {
                self.doc.apply(&o.patch);
                self.pending = o.pending;
                if let Some(Outcome::Value(v)) = &o.result {
                    self.last_result = Some(v.clone());
                }
                self.observe(o);
                if let Some(p) = self.program.as_mut() {
                    p.compact();
                }
            }
            Err(_) => self.program = None,
        }
        Ticked { out: result, before }
    }

    /// What the policies learn from a tick: windows' ids, decks' shuffles and requests.
    fn observe(&mut self, o: &TickOut) {
        for (p, _) in &o.patch.removed {
            self.windows.remove(p);
            self.decks.remove(p);
        }
        for (p, e) in &o.patch.added {
            match e.tag() {
                Some("Gui.Window") => {
                    if let Some(id) = TestReady(&self.ready).first_value(e) {
                        self.windows.insert(p.clone(), id);
                    }
                }
                Some("Cards.Deck") => {
                    if let Some(rng) = self.rng.as_mut() {
                        self.decks.insert(p.clone(), (rng.shuffle(), 0, 0));
                    }
                }
                _ => {}
            }
        }
        for r in &o.requests {
            if let Some(d) = self.decks.get_mut(&r.path) {
                d.2 += 1;
            }
        }
    }

    fn check(&self, step: &Step, t: &Ticked, failures: &mut Vec<String>, test: &str) {
        let all = self.doc.elements();
        let top = |p: &Pattern| self.doc.iter().any(|(path, e)| path.len() == 1 && p.matches(e));
        let mut expected_abort = false;
        for (check, line) in &step.checks {
            let Ok(o) = &t.out else {
                expected_abort |= matches!(check, Check::Aborted);
                if !matches!(check, Check::Aborted) {
                    failures.push(format!("test {test} line {line}: {check:?} after an aborted tick"));
                }
                continue;
            };
            let fired = |p: &Pattern| matches!(&o.result, Some(Outcome::Fired(v)) if p.matches(v));
            let requested = |p: &Pattern, v: &Pattern| {
                o.requests.iter().any(|r| {
                    let place = self.doc.get(&r.path).or_else(|| t.before.get(&r.path));
                    place.is_some_and(|e| p.matches(e)) && v.matches(&r.value)
                })
            };
            let ok = match check {
                Check::Aborted => false,
                Check::Settles => true,
                Check::Expect(p) => all.iter().any(|e| p.matches(e)),
                Check::ExpectNone(p) => !all.iter().any(|e| p.matches(e)),
                Check::ExpectCount(p, n) => all.iter().filter(|e| p.matches(e)).count() == *n,
                Check::ExpectTop(p) => top(p),
                Check::ExpectTopNone(p) => !top(p),
                Check::Requested(p, v) => requested(p, v),
                Check::Unrequested(p) => !requested(p, &Pattern::any()),
                Check::Returns(p) => self.last_result.as_ref().is_some_and(|v| p.matches(v)),
                Check::Fired(p) => fired(p),
                Check::Quiet => !fired(&Pattern::any()),
                Check::Snapshot(f) => {
                    failures.push(format!("test {test} line {line}: snapshot \"{f}\": this World draws no SVG"));
                    continue;
                }
            };
            if !ok {
                failures.push(format!("test {test} line {line}: {check:?} did not hold"));
            }
        }
        if let (Err(f), false) = (&t.out, expected_abort) {
            failures.push(format!("test {test} line {}: the tick aborted: {f}", step.line));
        }
    }
}

/// Print one tick in the transcript's format.
pub fn print_tick(out: &mut String, n: usize, inputs: &[String], result: &Result<TickOut, Fault>) {
    let _ = writeln!(out, "\ntick {n}");
    let section = |out: &mut String, name: &str, lines: Vec<String>| {
        if !lines.is_empty() {
            let _ = writeln!(out, "  {name}:");
            lines.iter().for_each(|l| {
                let _ = writeln!(out, "    {l}");
            });
        }
    };
    section(out, "in", inputs.to_vec());
    let o = match result {
        Ok(o) => o,
        Err(f) => {
            let _ = writeln!(out, "  aborted\n  | {f}");
            return;
        }
    };
    let places = |ps: &[(Path, Pod)]| ps.iter().map(|(p, e)| format!("{p}: {e}")).collect();
    section(out, "added", places(&o.patch.added));
    section(out, "changed", places(&o.patch.changed));
    section(out, "torn down", places(&o.patch.removed));
    section(out, "requested", o.requests.iter().map(|r| r.to_string()).collect());
    match &o.result {
        Some(Outcome::Value(v)) => _ = writeln!(out, "  result: {v}"),
        Some(Outcome::Fired(v)) => _ = writeln!(out, "  result: fired: {v}"),
        None => {}
    }
}

/// A line-by-line difference, for a failed `verify`.
pub fn diff(expected: &str, got: &str) -> String {
    let (e, g): (Vec<&str>, Vec<&str>) = (expected.lines().collect(), got.lines().collect());
    let mut out = String::new();
    for i in 0..e.len().max(g.len()) {
        match (e.get(i), g.get(i)) {
            (Some(a), Some(b)) if a == b => _ = writeln!(out, "  {a}"),
            (a, b) => {
                a.into_iter().for_each(|a| _ = writeln!(out, "- {a}"));
                b.into_iter().for_each(|b| _ = writeln!(out, "+ {b}"));
            }
        }
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
    args: Vec<Pod>,
    seed: Option<u64>,
    steps: Vec<Step>,
}

struct CheckBlock {
    name: String,
    lines: Vec<(String, Pattern, usize)>,
}

struct Step {
    line: usize,
    withs: Vec<(Pod, Pod)>,
    kind: StepKind,
    checks: Vec<(Check, usize)>,
}

enum StepKind {
    Build,
    /// `tick` and `compute`: a tick with no inputs; the word says why it is there.
    Tick,
    ComputeUntilIdle,
    Push(Pod, Pod),
    Deliver(Vec<Delivery>),
}

enum Delivery {
    Send(Target, Pod),
    Click(Target),
    /// `click element path`, the path as printed.
    Element(String),
    /// `answer p`.
    Answer(Pattern),
}

enum Target {
    Pattern(Pattern),
    /// `at path`, the path as printed.
    At(String),
}

#[derive(Debug)]
enum Check {
    Expect(Pattern),
    ExpectNone(Pattern),
    ExpectCount(Pattern, usize),
    ExpectTop(Pattern),
    ExpectTopNone(Pattern),
    Requested(Pattern, Pattern),
    Unrequested(Pattern),
    Returns(Pattern),
    Fired(Pattern),
    Quiet,
    Settles,
    Aborted,
    Snapshot(String),
}

/// Strip a comment: a `#` at the start of a line or after a space, outside a string (in
/// a path, `#` followed by a digit is a generation).
fn strip_comment(line: &str) -> &str {
    let (mut in_str, mut prev) = (false, ' ');
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

/// A declaration at the left edge of a test module (claude/modules-style-pass.md) beside its
/// tests: a helper type, function or value, its `derive` or signature line. The compiled
/// program holds it, so the script passes over it, body and all.
fn is_declaration(head: &str) -> bool {
    if ["type ", "derive ", "fn ", "let "].iter().any(|k| head.starts_with(k)) {
        return true;
    }
    match head.split_once(':') {
        Some((name, _)) => {
            !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || "_@$!?~&".contains(c)) && name.chars().any(|c| c.is_ascii_alphabetic())
        }
        None => false,
    }
}

fn parse_script(src: &str) -> Result<Vec<Block>, String> {
    let lines: Vec<(usize, &str)> =
        src.lines().enumerate().map(|(i, l)| (i + 1, strip_comment(l).trim_end())).filter(|(_, l)| !l.trim().is_empty()).collect();
    let (mut blocks, mut i) = (Vec::new(), 0);
    while i < lines.len() {
        let (no, head) = lines[i];
        let end = (i + 1..lines.len()).find(|&j| indent(lines[j].1) == 0).unwrap_or(lines.len());
        let body = &lines[i + 1..end];
        let head_err = || format!("line {no}: expected `test` or `check` at the left edge");
        if is_declaration(head) {
            i = end;
            continue;
        }
        let block = match head.strip_prefix("test ") {
            Some(rest) if indent(head) == 0 => Block::Test(parse_test(no, rest, body)?),
            _ => match head.strip_prefix("check ") {
                Some(rest) if indent(head) == 0 => Block::Check(parse_check(no, rest, body)?),
                _ => return Err(head_err()),
            },
        };
        blocks.push(block);
        i = end;
    }
    Ok(blocks)
}

fn parse_check(no: usize, head: &str, body: &[(usize, &str)]) -> Result<CheckBlock, String> {
    let name = head.strip_suffix(':').ok_or(format!("line {no}: a check line ends with :"))?.trim().to_string();
    let mut lines = Vec::new();
    for (n, l) in body {
        let l = l.trim();
        let (expr, pat) = match l.split_once("=>") {
            Some((e, p)) => (e.trim().to_string(), pattern(*n, p)?),
            // A bare line is short for `=> True`.
            None => (l.to_string(), Pattern(Pod::Bool(true))),
        };
        lines.push((expr, pat, *n));
    }
    Ok(CheckBlock { name, lines })
}

fn parse_test(no: usize, head: &str, body: &[(usize, &str)]) -> Result<TestBlock, String> {
    let head = head.strip_suffix(':').ok_or(format!("line {no}: a test line ends with :"))?;
    let parts = split_commas(head);
    let [name, program, extras @ ..] = parts.as_slice() else { return Err(format!("line {no}: test name, @program:")) };
    let program = program.trim().strip_prefix('@').ok_or(format!("line {no}: the program is written @name"))?;
    let (program, first) = program.split_once(' ').unwrap_or((program, ""));
    let mut args = Vec::new();
    if !first.trim().is_empty() {
        args.push(value(no, first)?);
    }
    let mut seed = None;
    for extra in extras {
        match extra.trim().strip_prefix("seed ") {
            Some(n) => seed = Some(n.trim().parse::<u64>().map_err(|_| format!("line {no}: a seed is a whole number"))?),
            None => args.push(value(no, extra)?),
        }
    }
    let mut steps: Vec<Step> = Vec::new();
    let step_col = body.first().map_or(4, |(_, l)| indent(l));
    for (n, l) in body {
        let text = l.trim();
        if indent(l) == step_col {
            steps.push(parse_step(*n, text)?);
            continue;
        }
        let (Some(step), Some(c)) = (steps.last_mut(), text.strip_prefix("=>")) else {
            return Err(format!("line {n}: a check is `=> ...` under a step; write a long step on one line"));
        };
        step.checks.push((parse_check_line(*n, c.trim())?, *n));
    }
    Ok(TestBlock { name: name.trim().to_string(), program: program.to_string(), args, seed, steps })
}

fn pattern(n: usize, s: &str) -> Result<Pattern, String> {
    read::pattern(s.trim()).map_err(|e| format!("line {n}: {e}"))
}

fn value(n: usize, s: &str) -> Result<Pod, String> {
    read::value(s.trim()).map_err(|e| format!("line {n}: {e}"))
}

fn printed_path(n: usize, s: &str) -> Result<String, String> {
    read::path(s.trim()).map(|p| p.to_string()).map_err(|e| format!("line {n}: {e}"))
}

fn parse_check_line(n: usize, c: &str) -> Result<Check, String> {
    let (word, rest) = c.split_once(' ').unwrap_or((c, ""));
    let two = || -> Result<(Pattern, String), String> {
        match split_commas(rest).as_slice() {
            [p, q] => Ok((pattern(n, p)?, q.trim().to_string())),
            _ => Err(format!("line {n}: `{word}` takes two operands")),
        }
    };
    Ok(match word {
        "expect" => Check::Expect(pattern(n, rest)?),
        "expect_none" => Check::ExpectNone(pattern(n, rest)?),
        "expect_count" => {
            let (p, k) = two()?;
            Check::ExpectCount(p, k.parse().map_err(|_| format!("line {n}: a count"))?)
        }
        "expect_top" => Check::ExpectTop(pattern(n, rest)?),
        "expect_top_none" => Check::ExpectTopNone(pattern(n, rest)?),
        "requested" => {
            let (p, v) = two()?;
            Check::Requested(p, pattern(n, &v)?)
        }
        "unrequested" => Check::Unrequested(pattern(n, rest)?),
        "returns" => Check::Returns(pattern(n, rest)?),
        "fired" => Check::Fired(pattern(n, rest)?),
        "quiet" => Check::Quiet,
        "settles" => Check::Settles,
        "aborted" => Check::Aborted,
        "snapshot" => Check::Snapshot(rest.trim().trim_matches('"').to_string()),
        other => return Err(format!("line {n}: unknown check `{other}`")),
    })
}

/// Split at commas at bracket depth zero, outside strings.
fn split_commas(s: &str) -> Vec<String> {
    let (mut out, mut cur, mut depth, mut in_str, mut prev) = (Vec::new(), String::new(), 0i32, false, ' ');
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
fn split_entry(n: usize, s: &str) -> Result<(Pod, Pod), String> {
    let cs: Vec<char> = s.chars().collect();
    let (mut depth, mut in_str) = (0i32, false);
    for i in 0..cs.len() {
        match cs[i] {
            '"' if i == 0 || cs[i - 1] != '\\' => in_str = !in_str,
            '(' | '[' | '{' if !in_str => depth += 1,
            ')' | ']' | '}' if !in_str => depth -= 1,
            '=' if !in_str && depth == 0 && !matches!(cs.get(i + 1), Some('=' | '>')) => {
                let (e, v): (String, String) = (cs[..i].iter().collect(), cs[i + 1..].iter().collect());
                return Ok((value(n, &e)?, value(n, &v)?));
            }
            _ => {}
        }
    }
    Err(format!("line {n}: an entry is `element = value`"))
}

fn parse_target(n: usize, s: &str) -> Result<Target, String> {
    match s.trim().strip_prefix("at ") {
        Some(path) => Ok(Target::At(printed_path(n, path)?)),
        None => Ok(Target::Pattern(pattern(n, s)?)),
    }
}

fn parse_step(n: usize, text: &str) -> Result<Step, String> {
    let parts = split_commas(text);
    let (mut withs, mut deliveries, mut kind, mut i) = (Vec::new(), Vec::new(), None, 0);
    while i < parts.len() {
        let part = parts[i].trim();
        let (word, rest) = part.split_once(' ').map_or((part, ""), |(w, r)| (w, r.trim()));
        match (word, rest) {
            ("with", _) => withs.push(split_entry(n, rest)?),
            ("build", "") => kind = Some(StepKind::Build),
            ("tick" | "compute", "") => kind = Some(StepKind::Tick),
            ("compute", "until idle") => kind = Some(StepKind::ComputeUntilIdle),
            ("push", _) => {
                let (e, v) = split_entry(n, rest)?;
                kind = Some(StepKind::Push(e, v));
            }
            ("click", _) => deliveries.push(match rest.strip_prefix("element ") {
                Some(path) => Delivery::Element(printed_path(n, path)?),
                None => Delivery::Click(parse_target(n, rest)?),
            }),
            ("answer", _) => deliveries.push(Delivery::Answer(pattern(n, rest)?)),
            ("send", _) => {
                let v = parts.get(i + 1).filter(|v| !["send", "click", "answer", "with"].contains(&v.split_whitespace().next().unwrap_or("")));
                let v = v.ok_or(format!("line {n}: send p, v"))?;
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
