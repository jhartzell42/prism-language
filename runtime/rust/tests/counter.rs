//! The counter and the half counter, written by hand as docs/backend.md says the
//! templates emit them, run by the Test World against the books' transcripts byte for
//! byte (Conformance ch. 1, Semantics ch. 1, Compiler ch. 3).

use std::rc::Rc;

use prism_rtl::*;

// ---- The `Ui` family, as its tag declarations derive `^Pod` -----------------------------

/// `tag Ui.Button of Element !Click: label: $Text`: the `Level` the patch sends.
struct UiButton {
    label: Text,
}

impl ToPod for UiButton {
    fn to_pod(&self) -> Pod {
        Pod::element("Ui.Button", vec![("label", self.label.to_pod())])
    }
}

/// `tag Ui.Label of Element: text: $Text`.
struct UiLabel {
    text: Text,
}

impl ToPod for UiLabel {
    fn to_pod(&self) -> Pod {
        Pod::element("Ui.Label", vec![("text", self.text.to_pod())])
    }
}

/// `type Click = unit type click`: a button's answer.
#[derive(Clone)]
struct Click;

impl FromPod for Click {
    fn from_pod(p: &Pod) -> Result<Click, Fault> {
        match p {
            Pod::Choice(c, args) if &**c == "click" && args.is_empty() => Ok(Click),
            other => prism_rtl::pod::mismatch("click", other),
        }
    }
}

// ---- counter.prism ----------------------------------------------------------------------
//
// @counter:
//     !inc <- Ui.Button(label=$"Increment")                 # 0
//     !dec <- Ui.Button(label=$"Decrement")                 # 1
//     $count <- 0 then $count + (-1 on !dec <| +1 on !inc)
//     Ui.Label(text=$"Count: {$count}")                     # 2

/// `@counter`: a static widget, one node, one state. The merge inside it records its
/// inputs and decides in the node's one wake (Compiler ch. 3, option (c)).
pub struct Counter {
    path: Path,
    /// `$count`.
    count: Hold<Int>,
    /// `!inc` and `!dec` as the merge's stamped scratch, never cleared.
    inc: Stamped<Click>,
    dec: Stamped<Click>,
}

impl Counter {
    /// The merge's distance from the node's inputs.
    const WAKE: u32 = 1;

    fn build(path: Path, cx: &mut Cx) -> Rc<Counter> {
        let node = Rc::new(Counter { path, count: Hold::new(Int::from(0), cx), inc: Stamped::new(), dec: Stamped::new() });
        cx.added(node.path.at(0), UiButton { label: "Increment".into() }.to_pod());
        cx.added(node.path.at(1), UiButton { label: "Decrement".into() }.to_pod());
        cx.added(node.path.at(2), node.label(cx));
        node
    }

    /// `!inc`: an answer at place 0 records and asks for the wake.
    fn inc(self: &Rc<Self>, v: Click, cx: &mut Cx) {
        self.inc.set(v, cx);
        cx.wake(Self::WAKE, self.clone());
    }

    /// `!dec`: an answer at place 1.
    fn dec(self: &Rc<Self>, v: Click, cx: &mut Cx) {
        self.dec.set(v, cx);
        cx.wake(Self::WAKE, self.clone());
    }

    /// `!delta`, the merge's firing: `$count + !delta`, then the hold's write.
    fn delta(self: &Rc<Self>, d: Int, cx: &mut Cx) {
        self.set_count(&self.count.get(cx) + &d, cx);
    }

    fn set_count(self: &Rc<Self>, v: Int, cx: &mut Cx) {
        self.count.set(v, cx);
        cx.dirty(self.clone()); // the label reads `$count`
    }

    /// Place 2's `Level`, pulled after the tick.
    fn label(&self, cx: &Cx) -> Pod {
        UiLabel { text: format!("Count: {}", self.count.get(cx)).into() }.to_pod()
    }
}

impl Node for Counter {
    fn answer(self: Rc<Self>, at: &[Segment], v: &Pod, cx: &mut Cx) -> Done {
        match at {
            [Segment::Pos(0)] => self.inc(Click::from_pod(v)?, cx),
            [Segment::Pos(1)] => self.dec(Click::from_pod(v)?, cx),
            _ => return Err(Fault::answer(format!("no answering place at {}", Path(at.to_vec())))),
        }
        Ok(())
    }

    /// `-1 on !dec <| +1 on !inc`: the left wins.
    fn wake(self: Rc<Self>, cx: &mut Cx) -> Done {
        if self.dec.fired(cx) {
            self.delta(Int::from(-1), cx);
        } else if self.inc.fired(cx) {
            self.delta(Int::from(1), cx);
        }
        Ok(())
    }

    fn patch(self: Rc<Self>, cx: &mut Cx) -> Done {
        let label = self.label(cx);
        cx.changed(self.path.at(2), label);
        Ok(())
    }
}

/// The program: the root node and the runtime.
pub struct CounterProgram {
    rt: Runtime,
    root: Rc<Counter>,
}

impl Program for CounterProgram {
    fn build(_args: &[Pod], ready: &dyn ReadyState) -> Result<(Self, TickOut), Fault> {
        let mut rt = Runtime::new();
        let mut cx = rt.build(ready);
        let root = Counter::build(Path::root(), &mut cx);
        let out = rt.close(cx)?;
        Ok((CounterProgram { rt, root }, out))
    }
    fn tick(&mut self, input: &TickIn, ready: &dyn ReadyState) -> Result<TickOut, Fault> {
        let root: Rc<dyn Node> = self.root.clone();
        self.rt.tick(&root, input, ready)
    }
    fn compact(&mut self) {
        self.rt.compact();
    }
}

// ---- half-counter.prism -----------------------------------------------------------------
//
// @halfcounter:
//     !increment <- Ui.Button(label=$"Increment")          # 0
//     let !delta = +1 on !increment
//     $count <- 0 then $count + !delta
//     Ui.Label(text=$"Count: {$count}")                    # 1

/// `@halfcounter`: no combinator, so no wake. `increment` calls `delta`, which calls the
/// setter (Compiler ch. 3, "The half counter").
pub struct HalfCounter {
    path: Path,
    count: Hold<Int>,
}

impl HalfCounter {
    fn build(path: Path, cx: &mut Cx) -> Rc<HalfCounter> {
        let node = Rc::new(HalfCounter { path, count: Hold::new(Int::from(0), cx) });
        cx.added(node.path.at(0), UiButton { label: "Increment".into() }.to_pod());
        cx.added(node.path.at(1), node.label(cx));
        node
    }
    fn increment(self: &Rc<Self>, _: Click, cx: &mut Cx) {
        self.delta(Int::from(1), cx);
    }
    fn delta(self: &Rc<Self>, d: Int, cx: &mut Cx) {
        self.count.set(&self.count.get(cx) + &d, cx);
        cx.dirty(self.clone());
    }
    fn label(&self, cx: &Cx) -> Pod {
        UiLabel { text: format!("Count: {}", self.count.get(cx)).into() }.to_pod()
    }
}

impl Node for HalfCounter {
    fn answer(self: Rc<Self>, at: &[Segment], v: &Pod, cx: &mut Cx) -> Done {
        match at {
            [Segment::Pos(0)] => self.increment(Click::from_pod(v)?, cx),
            _ => return Err(Fault::answer(format!("no answering place at {}", Path(at.to_vec())))),
        }
        Ok(())
    }
    fn patch(self: Rc<Self>, cx: &mut Cx) -> Done {
        let label = self.label(cx);
        cx.changed(self.path.at(1), label);
        Ok(())
    }
}

pub struct HalfCounterProgram {
    rt: Runtime,
    root: Rc<HalfCounter>,
}

impl Program for HalfCounterProgram {
    fn build(_args: &[Pod], ready: &dyn ReadyState) -> Result<(Self, TickOut), Fault> {
        let mut rt = Runtime::new();
        let mut cx = rt.build(ready);
        let root = HalfCounter::build(Path::root(), &mut cx);
        let out = rt.close(cx)?;
        Ok((HalfCounterProgram { rt, root }, out))
    }
    fn tick(&mut self, input: &TickIn, ready: &dyn ReadyState) -> Result<TickOut, Fault> {
        let root: Rc<dyn Node> = self.root.clone();
        self.rt.tick(&root, input, ready)
    }
}

// ---- The transcripts --------------------------------------------------------------------

fn world() -> TestWorld {
    TestWorld::new().program::<CounterProgram>("counter").program::<HalfCounterProgram>("halfcounter")
}

fn verify(script: &str, expected: &str) {
    if let Err(e) = world().verify(script, expected) {
        panic!("{e}");
    }
}

#[test]
fn two_clicks_in_one_tick_print_count_minus_one() {
    verify(
        include_str!("../../../tests/conformance/sem-ch01-counter/tests.prism"),
        include_str!("../../../tests/conformance/sem-ch01-counter/tests.expected"),
    );
}

#[test]
fn the_conformance_counter_counts() {
    verify(
        include_str!("../../../tests/conformance/conf-ch01-counter/tests.prism"),
        include_str!("../../../tests/conformance/conf-ch01-counter/tests.expected"),
    );
}

#[test]
fn the_example_counter_keeps_the_left() {
    verify(include_str!("../../../tests/conformance/counter/tests.prism"), include_str!("../../../tests/conformance/counter/tests.expected"));
}

#[test]
fn the_half_counter_calls_straight_through() {
    verify(
        include_str!("../../../tests/conformance/compiler-ch03-halfcounter/tests.prism"),
        include_str!("../../../tests/conformance/compiler-ch03-halfcounter/tests.expected"),
    );
}

#[test]
fn the_semantics_transcript_is_byte_for_byte() {
    let report = world().run(include_str!("../../../tests/conformance/sem-ch01-counter/tests.prism"));
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    assert!(report.transcript.ends_with("  changed:\n    2: Ui.Label(text=\"Count: -1\")\n"), "{}", report.transcript);
}

#[test]
fn a_failed_check_is_reported_beside_the_transcript() {
    let script = "test wrong, @counter:\n    build\n        => expect Ui.Label(text=\"Count: 9\")\n";
    let report = world().run(script);
    assert_eq!(report.exit_code, 1);
    assert!(report.failures[0].contains("line 3"), "{:?}", report.failures);
}

#[test]
fn an_answer_to_no_place_aborts_the_tick() {
    let script = "test nowhere, @counter:\n    build\n    click at 2\n        => aborted\n    tick\n";
    let report = world().run(script);
    assert!(report.transcript.contains("tick 1\n  in:\n    2: click\n  aborted\n  | Runtime.Answer"), "{}", report.transcript);
    assert_eq!(report.failures.len(), 1, "{:?}", report.failures);
    assert!(report.failures[0].contains("an earlier tick aborted"));
}

#[test]
fn a_static_widget_owes_no_tick() {
    let (_, out) = CounterProgram::build(&[], &ReadyMap::default()).unwrap();
    assert!(!out.pending);
    assert_eq!(out.patch.added.len(), 3);
}
