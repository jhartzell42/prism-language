//! The runtime library's own rules, each through the API emitted code calls.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use prism_rtl::world::scene::find_clickables;
use prism_rtl::world::test_world::{XorShift, strip_comment_header};
use prism_rtl::*;

fn ready() -> ReadyMap {
    ReadyMap::default()
}

// ---- A probe node: what its methods saw, tick by tick ------------------------------------

#[derive(Default)]
struct Probe {
    hold: RefCell<Option<Hold<Int>>>,
    cache: Cache<Int>,
    computed: RefCell<u32>,
    seen: RefCell<Vec<String>>,
    defer: Defer<Text>,
}

impl Probe {
    fn hold(&self, cx: &Cx) -> Int {
        self.hold.borrow().as_ref().unwrap().get(cx)
    }
    fn log(&self, s: String) {
        self.seen.borrow_mut().push(s);
    }
}

impl Node for Probe {
    fn answer(self: Rc<Self>, _: &[Segment], v: &Pod, cx: &mut Cx) -> Done {
        let n = Int::from_pod(v)?;
        self.hold.borrow().as_ref().unwrap().set(n, cx);
        self.cache.mark(cx);
        cx.wake(3, self.clone());
        cx.wake(3, self.clone()); // once per height
        cx.dirty(self);
        Ok(())
    }
    fn wake(self: Rc<Self>, cx: &mut Cx) -> Done {
        let cached = self.cache.get(cx, || {
            *self.computed.borrow_mut() += 1;
            self.hold(cx)
        });
        self.log(format!("wake {}: hold {} cache {}", cx.tick(), self.hold(cx), cached));
        if self.seen.borrow().len() < 4 {
            cx.wake(1, self.clone()); // below the running height: clamped to the next one
        }
        // A read after the tick during propagation (`!Updated`'s value): the read point
        // moves past the tick for the closure and comes back.
        let (hold, cached) = cx.after(|cx| {
            let cached = self.cache.get(cx, || {
                *self.computed.borrow_mut() += 1;
                self.hold(cx)
            });
            (self.hold(cx), cached)
        });
        self.log(format!("after {}: hold {} cache {}", cx.tick(), hold, cached));
        Ok(())
    }
    fn deferred(self: Rc<Self>, cx: &mut Cx) -> Done {
        self.log(format!("deferred {}: {:?}", cx.tick(), self.defer.fired(cx)));
        Ok(())
    }
    fn started(self: Rc<Self>, cx: &mut Cx) -> Done {
        self.log(format!("started {}", cx.tick()));
        self.defer.arm("again".into(), self.clone(), cx);
        Ok(())
    }
    fn patch(self: Rc<Self>, cx: &mut Cx) -> Done {
        let cached = self.cache.get(cx, || {
            *self.computed.borrow_mut() += 1;
            self.hold(cx)
        });
        self.log(format!("patch {}: hold {} cache {}", cx.tick(), self.hold(cx), cached));
        cx.changed(Path::root().at(0), Pod::element("Test.Probe", vec![("n", self.hold(cx).to_pod())]));
        Ok(())
    }
}

fn probe() -> (Runtime, Rc<Probe>, TickOut) {
    let mut rt = Runtime::new();
    let r = ready();
    let mut cx = rt.build(&r);
    let p = Rc::new(Probe::default());
    *p.hold.borrow_mut() = Some(Hold::new(Int::from(7), &cx));
    cx.added(Path::root().at(0), Pod::element("Test.Probe", vec![("n", Pod::int(7))]));
    cx.on_started(p.clone());
    let out = rt.close(cx).unwrap();
    (rt, p, out)
}

#[test]
fn a_hold_reads_old_values_during_its_tick_and_new_ones_after() {
    let (mut rt, p, out) = probe();
    assert!(out.pending, "a newborn that binds @started owes a tick");
    let root: Rc<dyn Node> = p.clone();
    let r = ready();
    let t1 = rt.tick(&root, &TickIn::none(), &r).unwrap();
    assert!(t1.pending, "an armed @Defer sets pending");
    let t2 = rt.tick(&root, &TickIn::none().answer(Path::root().at(0), Pod::int(8)), &r).unwrap();
    assert!(!t2.pending);
    assert_eq!(t2.patch.changed, vec![(Path::root().at(0), Pod::element("Test.Probe", vec![("n", Pod::int(8))]))]);
    assert_eq!(
        *p.seen.borrow(),
        vec![
            "started 1",
            "deferred 2: Some(\"again\")",
            "wake 2: hold 7 cache 7",
            "after 2: hold 8 cache 8",
            "wake 2: hold 7 cache 7",
            "after 2: hold 8 cache 8",
            "patch 2: hold 8 cache 8",
        ]
    );
    assert_eq!(
        *p.computed.borrow(),
        3,
        "the reader in the tick of the write reads the clean cache; the read after the tick recomputes once and is cached; the reader back at the tick's point is not served it, and computes without caching"
    );
}

#[test]
fn the_patch_diff_sends_only_what_moved() {
    let mut doc = Document::new();
    let (a, b) = (Path::root().at(0), Path::root().at(1));
    let label = |t: &str| Pod::element("Ui.Label", vec![("text", Pod::text(t))]);
    doc.diff(vec![Edit::Added(b.clone(), label("b")), Edit::Added(a.clone(), label("a"))]);
    let p = doc.diff(vec![Edit::Changed(a.clone(), label("a")), Edit::Changed(b.clone(), label("B"))]);
    assert_eq!((p.added.len(), p.changed, p.removed.len()), (0, vec![(b.clone(), label("B"))], 0));
    let p = doc.diff(vec![Edit::Removed(a.clone()), Edit::Added(a.clone(), label("new"))]);
    assert_eq!(p.removed, vec![(a.clone(), label("a"))], "torn down with its element as it stood");
    assert_eq!(p.added, vec![(a.clone(), label("new"))], "two widgets at one path, never Changed");
    let mut world_side = Document::new();
    world_side.apply(&Patch { added: vec![(a.clone(), label("new")), (b.clone(), label("B"))], ..Patch::default() });
    assert_eq!(world_side, doc);
}

#[test]
fn ports_subscribe_and_unsubscribe_in_constant_time() {
    let port: Rc<Port<Int>> = Rc::new(Port::new());
    assert_eq!(port.demand(), Demand::Idle);
    port.lien();
    assert_eq!(port.demand(), Demand::Liened);
    let heard = Rc::new(RefCell::new(Vec::new()));
    let (h1, h2) = (heard.clone(), heard.clone());
    let a = port.subscribe(Rc::new(move |v: &Int, _: &mut Cx| -> Done {
        h1.borrow_mut().push(format!("a {v}"));
        Ok(())
    }));
    // A subscriber that unsubscribes itself while the port fires.
    let me: Rc<RefCell<Option<Handle>>> = Rc::new(RefCell::new(None));
    let (port2, me2) = (Rc::downgrade(&port), me.clone());
    let b = port.subscribe(Rc::new(move |v: &Int, _: &mut Cx| -> Done {
        h2.borrow_mut().push(format!("b {v}"));
        if let (Some(p), Some(h)) = (Weak::upgrade(&port2), *me2.borrow()) {
            p.unsubscribe(h);
        }
        Ok(())
    }));
    *me.borrow_mut() = Some(b);
    assert_eq!(port.demand(), Demand::Subscribed);
    let r = ready();
    let mut rt = Runtime::new();
    let mut cx = rt.build(&r);
    port.fire(Int::from(1), &mut cx).unwrap();
    port.fire(Int::from(2), &mut cx).unwrap();
    assert_eq!(*heard.borrow(), vec!["a 1", "b 1", "a 2"]);
    assert_eq!(port.kept(&cx), Some(Int::from(2)), "a subscriber arriving later this tick catches up");
    port.unsubscribe(a);
    port.release();
    assert_eq!(port.demand(), Demand::Idle);
    let c = port.subscribe(Rc::new(|_: &Int, _: &mut Cx| -> Done { Ok(()) }));
    assert_eq!(c, a, "a freed handle is reused");
    rt.close(cx).unwrap();
}

#[test]
fn a_constant_and_never() {
    let r = ready();
    let cx = Runtime::new().build(&r);
    assert_eq!(Dynamic::constant(Text::from("x")).sample(&cx), Text::from("x"));
    assert_eq!(Event::<Int>::never().port.demand(), Demand::Idle);
}

// ---- Pushes, first values, requests -----------------------------------------------------

struct Place {
    element: Pod,
    heard: RefCell<Vec<Pod>>,
}

impl Node for Place {
    fn push(self: Rc<Self>, element: &Pod, v: &Pod, cx: &mut Cx) -> Done {
        assert_eq!(element, &self.element);
        self.heard.borrow_mut().push(v.clone());
        cx.request(Path::root().at(1), Some("seen"), v.clone());
        cx.request(Path::root().at(0), None, Pod::Unit);
        Ok(())
    }
}

#[test]
fn pushes_reach_every_place_with_the_element_and_newborns_see_them() {
    let clock = Pod::element("Time.Clock", vec![]);
    let mut entries = ReadyMap::default();
    entries.0.insert(clock.clone(), Pod::int(1));
    let mut rt = Runtime::new();
    let mut cx = rt.build(&entries);
    assert_eq!(cx.first_value(&clock), Some(Pod::int(1)));
    let place = Rc::new(Place { element: clock.clone(), heard: RefCell::new(Vec::new()) });
    let node: Rc<dyn Node> = place.clone();
    cx.listen(clock.clone(), &node);
    rt.close(cx).unwrap();
    let out = rt.tick(&node, &TickIn::none().push(clock.clone(), Pod::int(2)), &entries).unwrap();
    assert_eq!(*place.heard.borrow(), vec![Pod::int(2)]);
    let lines: Vec<String> = out.requests.iter().map(|r| r.to_string()).collect();
    assert_eq!(lines, vec!["0: unit", "1: seen=2"], "requests in document order, named when a tag has several");
}

// ---- The zombie collector ---------------------------------------------------------------

/// A zombie row whose constant keeps another node (a value with its `$` inside).
#[derive(Default)]
struct Zombie {
    constant: RefCell<Option<Rc<dyn Node>>>,
}

impl Node for Zombie {
    fn edges(&self, visit: &mut dyn FnMut(&Rc<dyn Node>)) {
        if let Some(n) = self.constant.borrow().as_ref() {
            visit(n);
        }
    }
    fn release(&self) {
        self.constant.borrow_mut().take();
    }
}

#[test]
fn two_zombies_that_lien_each_other_are_freed_and_a_held_one_is_kept() {
    let r = ready();
    let mut rt = Runtime::new();
    let mut cx = rt.build(&r);
    let (a, b, c) = (Rc::new(Zombie::default()), Rc::new(Zombie::default()), Rc::new(Zombie::default()));
    *a.constant.borrow_mut() = Some(b.clone());
    *b.constant.borrow_mut() = Some(a.clone());
    *c.constant.borrow_mut() = Some(a.clone()); // c keeps a, and the test keeps c: all three live
    for z in [&a, &b, &c] {
        let z: Rc<dyn Node> = z.clone();
        cx.clean_up_later(z, true);
    }
    rt.close(cx).unwrap();
    let (wa, wb, wc) = (Rc::downgrade(&a), Rc::downgrade(&b), Rc::downgrade(&c));
    drop((a, b));
    assert_eq!(rt.compact(), 0, "c is held from outside, and it reaches the loop");
    drop(c); // its count frees it; the loop it kept is the collector's
    assert_eq!(rt.compact(), 2);
    assert!(wa.upgrade().is_none() && wb.upgrade().is_none() && wc.upgrade().is_none(), "no leaks, ever (D5)");
}

// ---- Spelling ---------------------------------------------------------------------------

#[test]
fn spellings_round_trip() {
    for s in [
        "unit",
        "True",
        "-1",
        "42",
        "\"a \\\"b\\\"\\n\\t\\u{1}\"",
        "'x'",
        "nil",
        "? 3",
        "~ Contents(text=\"hello\")",
        "? Card(rank=Four, suit=Spades)",
        "Write \"end of page 1\\n\"",
        "? (Letter \"a\")",
        "{1: \"a\", 2: \"b\"}",
        "[[1, 2, 3]]",
        "[[]]",
        "((1, \"x\"))",
        "Ui.Button(label=\"Increment\")",
        "Test.Log",
        "click",
        "Error(kind=Fs.NotFound, message=\"gone\")",
        "1300us",
        "0s",
        "Rect 1, 2",
        "280.0",
        "-0.5",
        "0.30000000000000004",
        "nan",
        "-inf",
        "Press Point(x=280.0, y=400.0)",
        "Gui.Scene(window=WindowId \"Chess\", at=nil)",
        "Box(paint=(Rgb 1.0, 0.5, 0.0), inner=Node (Leaf 1))",
        "? (Node (Leaf 1), (Leaf 2))",
        "Name()",
    ] {
        let v = read::value(s).unwrap_or_else(|e| panic!("{s}: {e}"));
        assert_eq!(v.to_string(), s);
    }
    for s in ["0", "1/#0/1/#0/2", "2/[\"a\"]/0", "3/[1/#0/0]/0"] {
        assert_eq!(read::path(s).unwrap().to_string(), s);
    }
}

#[test]
fn typed_values_print_and_cross_as_pods() {
    let m: Map<Text, Array<Option<Int>>> = Map::unit("k".into(), Array::from(vec![Some(Int::from(1)), None]));
    assert_eq!(show(&m), "{\"k\": [[? 1, nil]]}");
    assert_eq!(Map::<Text, Array<Option<Int>>>::from_pod(&m.to_pod()).unwrap(), m);
    let failed: Result<Text, Error> = Err(Error::new("Fs.NotFound", "gone"));
    assert_eq!(show(&failed), "Error(kind=Fs.NotFound, message=\"gone\")");
    assert_eq!(failed.to_pod().to_string(), show(&failed));
    assert_eq!(Result::<Text, Error>::from_pod(&read::value("Error(kind=Fs.NotFound, message=\"gone\")").unwrap()).unwrap(), failed);
    assert_eq!(show(&(Nat::from(3u32), 2.5)), "((3, 2.5))");
    let shown = Pod::Shown("e2".into(), Box::new(Pod::record("Spot", vec![("down", Pod::int(6))])));
    assert_eq!(Path::root().at(0).child(Segment::Key(shown.clone())).at(0).to_string(), "0/[e2]/0");
    assert!(read::pattern("Spot(down=6)").unwrap().matches(&shown));
    assert!(u8::from_pod(&Pod::int(256)).is_err());
}

#[test]
fn patterns_match() {
    let e = read::value("Ui.Button(label=\"OK\", enabled=True)").unwrap();
    let m = |p: &str| read::pattern(p).unwrap().matches(&e);
    assert!(m("Ui.Button(label=\"OK\", ..)") && m("Ui.*") && m("Ui.Button(label=ignore, enabled=True)"));
    assert!(!m("Ui.Button(label=\"OK\")") && !m("Fs.*"));
    let err = read::value("Error(kind=Fs.NotFound, message=\"gone\")").unwrap();
    assert!(read::pattern("Error(kind=Fs.NotFound, ..)").unwrap().matches(&err));
}

// ---- Worlds ----------------------------------------------------------------------------

/// `Process.Exit(code=(3 on !started))` at 0.
struct Exits;

impl Node for Exits {
    fn started(self: Rc<Self>, cx: &mut Cx) -> Done {
        cx.request(Path::root().at(0), None, 3u8.to_pod());
        Ok(())
    }
}

struct ExitsProgram {
    rt: Runtime,
    root: Rc<Exits>,
}

impl Program for ExitsProgram {
    fn build(_: &[Pod], ready: &dyn ReadyState) -> Result<(Self, TickOut), Fault> {
        let mut rt = Runtime::new();
        let mut cx = rt.build(ready);
        cx.added(Path::root().at(0), Pod::element("Process.Exit", vec![]));
        let root = Rc::new(Exits);
        cx.on_started(root.clone());
        let out = rt.close(cx)?;
        Ok((ExitsProgram { rt, root }, out))
    }
    fn tick(&mut self, input: &TickIn, ready: &dyn ReadyState) -> Result<TickOut, Fault> {
        let root: Rc<dyn Node> = self.root.clone();
        self.rt.tick(&root, input, ready)
    }
}

/// A World that answers nothing and exits when a `Process.Exit` place requests.
#[derive(Default)]
struct Quiet {
    doc: Document,
    exit: Option<u8>,
    ticks: usize,
}

impl World for Quiet {
    fn ready(&self) -> Rc<dyn ReadyState> {
        Rc::new(ReadyMap::default())
    }
    fn perform(&mut self, out: &TickOut) {
        self.ticks += 1;
        self.doc.apply(&out.patch);
        for r in &out.requests {
            if self.doc.get(&r.path).and_then(Pod::tag) == Some("Process.Exit") {
                self.exit = u8::from_pod(&r.value).ok();
            }
        }
    }
    fn next_inputs(&mut self) -> Option<TickIn> {
        None
    }
    fn exit_code(&self) -> Option<u8> {
        self.exit
    }
}

#[test]
fn drive_ticks_while_pending_and_exits_with_the_code_requested() {
    let mut world = Quiet::default();
    assert_eq!(drive::<ExitsProgram>(&mut world, &[], 10), 3);
    assert_eq!(world.ticks, 2, "the build, then the owed tick");
}

#[test]
fn seed_seven_deals_the_four_of_spades_first() {
    let deck = XorShift::new(7).shuffle();
    assert_eq!(deck[0].to_string(), "Card(rank=Four, suit=Spades)");
    let mut all: Vec<String> = deck.iter().map(Pod::to_string).collect();
    all.sort();
    all.dedup();
    assert_eq!(all.len(), 52);
}

#[test]
fn a_clickable_is_found_by_its_path_through_a_turn() {
    let rect = read::value("Rect(x=0.0, y=0.0, w=10.0, h=20.0)").unwrap();
    let flat = Pod::element("Draw.Clickable", vec![("rect", rect), ("path", Pod::Path(read::path("1/#0/0").unwrap()))]);
    let turn = read::value("Turn(x=0.0, y=0.0, angle=90.0, scale=2.0)").unwrap();
    let group = Pod::element("Draw.Group", vec![("turn", turn), ("draws", Pod::Array(vec![flat.clone()]))]);
    let scene = |d: Pod| Pod::element("Gui.Scene", vec![("window", read::value("WindowId \"w\"").unwrap()), ("draw", Pod::Array(vec![d]))]);
    let mut doc = Document::new();
    doc.diff(vec![Edit::Added(Path::root().at(3), scene(flat))]);
    let found = find_clickables(&doc, "1/#0/0");
    assert_eq!((found.len(), found[0].x, found[0].y), (1, 5.0, 10.0));
    assert!(find_clickables(&doc, "1/#0/1").is_empty());
    doc.diff(vec![Edit::Changed(Path::root().at(3), scene(group))]);
    let c = &find_clickables(&doc, "1/#0/0")[0];
    assert!((c.x - -20.0).abs() < 1e-9 && (c.y - 10.0).abs() < 1e-9, "{c:?}");
}

#[test]
fn scripts_parse_or_exit_two_and_checks_print_ok_or_the_value() {
    assert_eq!(TestWorld::new().run("  build\n").exit_code, 2);
    let r = TestWorld::new().run("check sums:\n    3 => 3\n    \"a\" => \"b\"\n    True\n");
    assert_eq!(r.transcript, "check sums\n1: ok\n2: \"a\"\n3: ok\n");
    assert_eq!(r.exit_code, 1);
    let compiled = TestWorld::new().check("sums", || vec![Ok(Pod::int(3)), Ok(Pod::text("b")), Ok(Pod::Bool(true))]);
    assert_eq!(compiled.run("check sums:\n    1 + 2 => 3\n    \"a\" + \"b\" => \"b\"\n    True\n").exit_code, 0);
    assert_eq!(strip_comment_header("# a note\n#\ntest t\n"), "test t\n");
}

// ---- A structural dynamic's equal snapshot keeps its value -------------------------------

/// A value whose equality ignores its `id`: the id tells which of two equal values a
/// snapshot holds, which no Prism program can (Semantics ch. 11, "sharing is invisible").
#[derive(Clone, Debug)]
struct Tagged {
    v: u32,
    id: u32,
}

impl PartialEq for Tagged {
    fn eq(&self, other: &Tagged) -> bool {
        self.v == other.v
    }
}

/// A node that takes the next of `takes` as its snapshot in each tick's construction step
/// and logs whether it moved, the structure's id after the take, and the dynamic's.
struct Taker {
    s: Snapshot<Tagged>,
    takes: RefCell<Vec<Tagged>>,
    seen: RefCell<Vec<String>>,
}

impl Node for Taker {
    fn answer(self: Rc<Self>, _: &[Segment], _: &Pod, cx: &mut Cx) -> Done {
        cx.build_later(self.clone());
        Ok(())
    }
    fn build(self: Rc<Self>, cx: &mut Cx) -> Done {
        let next = self.takes.borrow_mut().remove(0);
        let moved = self.s.take_compared(next, cx);
        let line = format!("moved {moved} current {} dynamic {}", self.s.current(cx).id, self.s.get(cx).id);
        self.seen.borrow_mut().push(line);
        Ok(())
    }
}

#[test]
fn an_equal_snapshot_keeps_the_value_it_had_so_its_sharing_survives() {
    let r = ready();
    let mut rt = Runtime::new();
    let cx = rt.build(&r);
    let t = Rc::new(Taker {
        s: Snapshot::new("$s"),
        takes: RefCell::new(vec![Tagged { v: 1, id: 2 }, Tagged { v: 2, id: 3 }, Tagged { v: 2, id: 4 }, Tagged { v: 2, id: 5 }]),
        seen: RefCell::new(Vec::new()),
    });
    assert!(!t.s.take_compared(Tagged { v: 1, id: 1 }, &cx));
    rt.close(cx).unwrap();
    let root: Rc<dyn Node> = t.clone();
    for _ in 0..4 {
        rt.tick(&root, &TickIn::none().answer(Path::root().at(0), Pod::Unit), &r).unwrap();
    }
    // A take equal to the snapshot before it keeps that snapshot (ids 1, then 3): the same
    // moves, and the dynamic's values equal to those of a take that kept the new value.
    assert_eq!(
        *t.seen.borrow(),
        vec!["moved false current 1 dynamic 1", "moved true current 3 dynamic 1", "moved false current 3 dynamic 3", "moved false current 3 dynamic 3"]
    );
}
