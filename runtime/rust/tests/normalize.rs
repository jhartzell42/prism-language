//! `@Normal` and `Normalize` through `@Run`, written by hand as docs/backend.md says the
//! templates emit them, against Conformance ch. 1's "A job, computed": `compute until
//! idle: 3 ticks, 1 to 3`, byte for byte.

use std::rc::Rc;

use prism_rtl::pod::mismatch;
use prism_rtl::*;

// ---- type Term = choice: Var Nat | Lam Term | App Term, Term; derive ^Eq, ^Show ---------

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Term {
    Var(Nat),
    Lam(Rc<Term>),
    App(Rc<Term>, Rc<Term>),
}

/// Derived `^Show`: the printer decides the parentheses.
impl Show for Term {
    fn show(&self, p: &mut Printer) {
        match self {
            Term::Var(n) => p.choice("Var", &[n]),
            Term::Lam(b) => p.choice("Lam", &[b]),
            Term::App(f, x) => p.choice("App", &[f, x]),
        }
    }
}

/// Derived `^Pod`, both ways: a root argument arrives as plain data.
impl ToPod for Term {
    fn to_pod(&self) -> Pod {
        match self {
            Term::Var(n) => Pod::Choice("Var".into(), vec![n.to_pod()]),
            Term::Lam(b) => Pod::Choice("Lam".into(), vec![b.to_pod()]),
            Term::App(f, x) => Pod::Choice("App".into(), vec![f.to_pod(), x.to_pod()]),
        }
    }
}

impl FromPod for Term {
    fn from_pod(p: &Pod) -> Result<Term, Fault> {
        match p.structure() {
            Pod::Choice(c, a) if &**c == "Var" && a.len() == 1 => Ok(Term::Var(Nat::from_pod(&a[0])?)),
            Pod::Choice(c, a) if &**c == "Lam" && a.len() == 1 => Ok(Term::Lam(Rc::from_pod(&a[0])?)),
            Pod::Choice(c, a) if &**c == "App" && a.len() == 2 => Ok(Term::App(Rc::from_pod(&a[0])?, Rc::from_pod(&a[1])?)),
            other => mismatch("a Term", other),
        }
    }
}

// ---- The plain functions ----------------------------------------------------------------

/// `Shift: fn Int, Nat, Term -> Term`.
fn shift(d: &Int, cut: &Nat, t: &Term) -> Term {
    match t {
        Term::Var(n) if n < cut => Term::Var(n.clone()),
        Term::Var(n) => Term::Var(Nat::from_int(n.as_int() + d).unwrap_or(Nat::from(0u32))),
        Term::Lam(b) => Term::Lam(Rc::new(shift(d, &(cut + &Nat::from(1u32)), b))),
        Term::App(f, x) => Term::App(Rc::new(shift(d, cut, f)), Rc::new(shift(d, cut, x))),
    }
}

/// `Subst: fn Nat, Term, Term -> Term`.
fn subst(j: &Nat, s: &Term, t: &Term) -> Term {
    match t {
        Term::Var(n) if n == j => s.clone(),
        Term::Var(n) => Term::Var(n.clone()),
        Term::Lam(b) => Term::Lam(Rc::new(subst(&(j + &Nat::from(1u32)), &shift(&Int::from(1), &Nat::from(0u32), s), b))),
        Term::App(f, x) => Term::App(Rc::new(subst(j, s, f)), Rc::new(subst(j, s, x))),
    }
}

/// `?Reduce: fn Term -> ?Term`: one leftmost-outermost reduction.
fn reduce(t: &Term) -> Option<Term> {
    let zero = Nat::from(0u32);
    match t {
        Term::Var(_) => None,
        Term::Lam(b) => reduce(b).map(|b2| Term::Lam(Rc::new(b2))),
        Term::App(f, x) => match &**f {
            Term::Lam(b) => Some(shift(&Int::from(-1), &zero, &subst(&zero, &shift(&Int::from(1), &zero, x), b))),
            _ => match reduce(f) {
                Some(f2) => Some(Term::App(Rc::new(f2), x.clone())),
                None => reduce(x).map(|x2| Term::App(f.clone(), Rc::new(x2))),
            },
        },
    }
}

/// `Describe: fn Term -> Text`, `Show t`.
fn describe(t: &Term) -> Text {
    show_text(t)
}

// ---- Normalize: unbounded fn Term -> Term, as a step machine ----------------------------
//
// fn Normalize t = match ?Reduce t:
//     ? t2: Normalize t2          # a cycle point: an unbounded call from an unbounded fn
//     nil: t

/// The job stopped at a cycle point: `Normalize` about to be entered with `t`. The call
/// is in tail position, so its continuation is "return what it returns", and the job
/// needs no stack.
struct Normalize {
    t: Term,
}

impl Job<Term> for Normalize {
    fn cycle(&self) -> Turn<Term> {
        match reduce(&self.t) {
            Some(t2) => Turn::More(Rc::new(Normalize { t: t2 })),
            None => Turn::Finished(self.t.clone()),
        }
    }
}

/// `Start Normalize, x`: what the function value `Normalize` is when passed to `@Run`.
fn start_normalize(t: Term) -> Rc<dyn Job<Term>> {
    Rc::new(Normalize { t })
}

// ---- @Normal ----------------------------------------------------------------------------
//
// fn @Normal term:
//     !done <- @Run Normalize, term                           # 0
//     $shown <- "working" then Describe !done
//     Ui.Label(text=$shown)                                   # 1
//
// with `@Run` (Semantics Appendix B) in a static position, so its fields are merged in:
//
// fn @Run F, x:
//     !started <- @started                                          # 0
//     !again <- @Defer (!Present (?More !turns))
//     !turns <- @Machine Cycle, (Start F, x), (!started <| !again)
//     @ !Filter (?Finished !turns)

pub struct Normal {
    path: Path,
    /// `@Run`'s `!started`, a stamped input of the merge.
    run_started: Stamped<()>,
    /// `@Run`'s `!again`.
    run_again: Defer<()>,
    /// `@Machine`'s state: the job.
    run_job: Hold<Rc<dyn Job<Term>>>,
    /// `$shown`.
    shown: Hold<Text>,
}

impl Normal {
    /// The merge `!started <| !again` decides one height above the inputs.
    const WAKE: u32 = 1;

    fn build(path: Path, term: Term, cx: &mut Cx) -> Rc<Normal> {
        let node = Rc::new(Normal {
            path,
            run_started: Stamped::new(),
            run_again: Defer::new(),
            run_job: Hold::new(start_normalize(term), cx),
            shown: Hold::new("working".into(), cx),
        });
        cx.on_started(node.clone()); // `@Run` binds `@started`: the next tick is owed
        cx.added(node.path.at(1), node.label(cx));
        node
    }

    /// `!done` fired: `Describe !done` writes `$shown`, which the label reads.
    fn done(self: &Rc<Self>, t: Term, cx: &mut Cx) {
        self.shown.set(describe(&t), cx);
        cx.dirty(self.clone());
    }

    fn label(&self, cx: &Cx) -> Pod {
        Pod::element("Ui.Label", vec![("text", self.shown.get(cx).to_pod())])
    }
}

impl Node for Normal {
    fn started(self: Rc<Self>, cx: &mut Cx) -> Done {
        self.run_started.set((), cx);
        cx.wake(Self::WAKE, self);
        Ok(())
    }

    fn deferred(self: Rc<Self>, cx: &mut Cx) -> Done {
        cx.wake(Self::WAKE, self);
        Ok(())
    }

    /// `!turns <- @Machine Cycle, job, (!started <| !again)`: one cycle per firing; a
    /// `More` arms `!again`, a `Finished` fires `!done`.
    fn wake(self: Rc<Self>, cx: &mut Cx) -> Done {
        if !self.run_started.fired(cx) && self.run_again.fired(cx).is_none() {
            return Ok(());
        }
        match self.run_job.get(cx).cycle() {
            Turn::More(next) => {
                self.run_job.set(next, cx);
                self.run_again.arm((), self.clone(), cx);
            }
            Turn::Finished(t) => self.done(t, cx),
        }
        Ok(())
    }

    fn patch(self: Rc<Self>, cx: &mut Cx) -> Done {
        let label = self.label(cx);
        cx.changed(self.path.at(1), label);
        Ok(())
    }
}

pub struct NormalProgram {
    rt: Runtime,
    root: Rc<Normal>,
}

impl Program for NormalProgram {
    fn build(args: &[Pod], ready: &dyn ReadyState) -> Result<(Self, TickOut), Fault> {
        let [term] = args else { return Err(Fault::answer("@Normal takes one Term")) };
        let term = Term::from_pod(term)?;
        let mut rt = Runtime::new();
        let mut cx = rt.build(ready);
        let root = Normal::build(Path::root(), term, &mut cx);
        let out = rt.close(cx)?;
        Ok((NormalProgram { rt, root }, out))
    }
    fn tick(&mut self, input: &TickIn, ready: &dyn ReadyState) -> Result<TickOut, Fault> {
        let root: Rc<dyn Node> = self.root.clone();
        self.rt.tick(&root, input, ready)
    }
}

fn world() -> TestWorld {
    TestWorld::new().program::<NormalProgram>("Normal")
}

#[test]
fn two_reductions_take_three_cycles() {
    let script = include_str!("../../../tests/conformance/sem-ch07-normalize/tests.prism");
    let expected = include_str!("../../../tests/conformance/sem-ch07-normalize/tests.expected");
    if let Err(e) = world().verify(script, expected) {
        panic!("{e}");
    }
    let report = world().run(script);
    assert!(report.transcript.contains("compute until idle: 3 ticks, 1 to 3\n"), "{}", report.transcript);
}

#[test]
fn compute_lines_watch_the_cycles_one_tick_each() {
    let script = "test cycles, @Normal (App (Lam (Var 0)), (Lam (Var 0))):\n    build\n    compute\n    compute\n        => expect Ui.Label(text=\"Lam (Var 0)\")\n    compute until idle\n";
    let report = world().run(script);
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    assert_eq!(
        report.transcript,
        "test cycles\n\ntick 0\n  added:\n    1: Ui.Label(text=\"working\")\n\ntick 1\n\ntick 2\n  changed:\n    1: Ui.Label(text=\"Lam (Var 0)\")\n\ncompute until idle: 0 ticks\n"
    );
}

#[test]
fn the_term_prints_as_the_book_spells_it() {
    let t = Term::App(Rc::new(Term::Lam(Rc::new(Term::Var(Nat::from(0u32))))), Rc::new(Term::Var(Nat::from(1u32))));
    assert_eq!(show(&t), "App (Lam (Var 0)), (Var 1)");
    assert_eq!(Term::from_pod(&prism_rtl::read::value(&show(&t)).unwrap()).unwrap(), t);
}
