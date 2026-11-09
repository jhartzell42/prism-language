//! The CLI World against programs written by hand in the shape claude/backend.md gives the
//! templates (as runtime/rust/tests/counter.rs is), driven by `prism_rtl::drive`.

use std::rc::Rc;
use std::time::{Duration, Instant};

use prism_rtl::*;
use prism_world_cli::{CliWorld, MemFs, RealFs};

// ---- A program around one root node -----------------------------------------------------

trait Root: Node + Sized {
    fn construct(path: Path, cx: &mut Cx) -> Result<Rc<Self>, Fault>;
}

struct Prog<N: Root> {
    rt: Runtime,
    root: Rc<N>,
}

impl<N: Root> Program for Prog<N> {
    fn build(_args: &[Pod], ready: &dyn ReadyState) -> Result<(Self, TickOut), Fault> {
        let mut rt = Runtime::new();
        let mut cx = rt.build(ready);
        let root = N::construct(Path::root(), &mut cx)?;
        let out = rt.close(cx)?;
        Ok((Prog { rt, root }, out))
    }
    fn tick(&mut self, input: &TickIn, ready: &dyn ReadyState) -> Result<TickOut, Fault> {
        let root: Rc<dyn Node> = self.root.clone();
        self.rt.tick(&root, input, ready)
    }
    fn compact(&mut self) {
        self.rt.compact();
    }
}

fn no_place(at: &[Segment]) -> Done {
    Err(Fault::answer(format!("no answering place at {}", Path(at.to_vec()))))
}

// ---- copy.prism -------------------------------------------------------------------------
//
// Upper: fn Contents -> Text
//
// @main:
//     $args <- Process.Args                                               # 0
//     !started <- @started
//     !~read <- Fs.File(path=$args[0], requests=(Read on !started))       # 1
//     !read <- Prelude.Unwrap(value=!~read)                               # 2
//     $text <- "" then Upper !read
//     $state <- Fs.Output(path=$args[1], text=$text)                      # 3
//     Process.Exit(code=(42 on !Gate (Len $text > 0), !Written $state))   # 4

struct Copy {
    path: Path,
    to: Text,
    text: Hold<Text>,
}

impl Copy {
    fn output(&self, cx: &Cx) -> Pod {
        Pod::element("Fs.Output", vec![("path", self.to.to_pod()), ("text", self.text.get(cx).to_pod())])
    }
    /// The output place's `Level` keys its pushes, and its text is live: listen again
    /// whenever it is sent.
    fn listen_output(self: &Rc<Self>, level: Pod, cx: &mut Cx) {
        let me: Rc<dyn Node> = self.clone();
        cx.listen(level, &me);
    }
}

impl Root for Copy {
    fn construct(path: Path, cx: &mut Cx) -> Result<Rc<Copy>, Fault> {
        let args_el = Pod::element("Process.Args", vec![]);
        let args = Array::<Text>::from_pod(&cx.first_value(&args_el).ok_or(Fault::answer("Process.Args unanswered"))?)?;
        let (from, to) = (args[0].clone(), args[1].clone());
        let node = Rc::new(Copy { path, to, text: Hold::new(Text::from(""), cx) });
        cx.added(node.path.at(0), args_el);
        cx.added(node.path.at(1), Pod::element("Fs.File", vec![("path", from.to_pod())]));
        cx.added(node.path.at(2), Pod::element("Prelude.Unwrap", vec![]));
        let output = node.output(cx);
        let first = cx.first_value(&output);
        assert_eq!(first, Some(Pod::name("Writing")));
        cx.added(node.path.at(3), output.clone());
        node.listen_output(output, cx);
        cx.added(node.path.at(4), Pod::element("Process.Exit", vec![]));
        cx.on_started(node.clone());
        Ok(node)
    }
}

impl Node for Copy {
    fn started(self: Rc<Self>, cx: &mut Cx) -> Done {
        cx.request(self.path.at(1), None, Pod::name("Read"));
        Ok(())
    }
    fn answer(self: Rc<Self>, at: &[Segment], v: &Pod, cx: &mut Cx) -> Done {
        match at {
            // `!~read` feeds the Unwrap's request field.
            [Segment::Pos(1)] => cx.request(self.path.at(2), None, Result::<Pod, Error>::from_pod(v)?.to_pod()),
            [Segment::Pos(2)] => {
                let text = v.field("text").and_then(Pod::as_text).ok_or(Fault::answer("expected Contents"))?;
                self.text.set(text.to_string().to_uppercase().as_str().into(), cx);
                cx.dirty(self);
            }
            _ => return no_place(at),
        }
        Ok(())
    }
    fn push(self: Rc<Self>, _: &Pod, v: &Pod, cx: &mut Cx) -> Done {
        if *v == Pod::name("Written") && !self.text.get(cx).is_empty() {
            cx.request(self.path.at(4), None, 42u8.to_pod());
        }
        Ok(())
    }
    fn patch(self: Rc<Self>, cx: &mut Cx) -> Done {
        let level = self.output(cx);
        cx.changed(self.path.at(3), level.clone());
        self.listen_output(level, cx);
        Ok(())
    }
}

fn copy_world(files: &[(&str, &str)]) -> CliWorld<MemFs> {
    let mut fs = MemFs::default();
    files.iter().for_each(|(p, t)| _ = fs.files.insert(p.to_string(), t.to_string()));
    CliWorld::new(fs, vec!["in.txt".into(), "out/up.txt".into()])
}

#[test]
fn an_environment_variable_is_answered_plain_or_nil() {
    let world = copy_world(&[]);
    let ready = world.ready();
    let env = |n: &str| ready.first_value(&Pod::element("Process.Env", vec![("name", Pod::text(n))]));
    let path = std::env::var("PATH").expect("PATH is set where the tests run");
    assert_eq!(env("PATH"), Some(Pod::Some(Box::new(Pod::text(path.as_str())))));
    assert_eq!(env("PRISM_TEST_SURELY_UNSET_VARIABLE"), Some(Pod::Nil));
}

#[test]
fn a_program_reads_a_file_writes_one_and_exits_42() {
    let mut world = copy_world(&[("in.txt", "hello")]);
    assert_eq!(drive::<Prog<Copy>>(&mut world, &[], 100), 42);
    assert_eq!(world.fs.borrow().files.get("out/up.txt").map(String::as_str), Some("HELLO"));
}

#[test]
fn a_missing_file_poisons_the_root_and_exits_1() {
    let mut world = copy_world(&[]);
    assert_eq!(drive::<Prog<Copy>>(&mut world, &[], 100), 1);
    assert_eq!(world.fs.borrow().files.get("out/up.txt").map(String::as_str), Some(""), "only the build's write");
}

#[test]
fn the_real_file_system_writes_beside_and_renames() {
    let dir = temp_dir("copy");
    std::fs::write(dir.join("in.txt"), "real").unwrap();
    let mut world = CliWorld::new(RealFs { root: dir.clone() }, vec!["in.txt".into(), "out/up.txt".into()]);
    assert_eq!(drive::<Prog<Copy>>(&mut world, &[], 100), 42);
    assert_eq!(std::fs::read_to_string(dir.join("out/up.txt")).unwrap(), "REAL");
    assert!(!dir.join("out/up.txt.prism-tmp").exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_file_that_is_not_utf8_reads_as_fs_not_text() {
    use prism_world_cli::FileSystem;
    let dir = temp_dir("not-text");
    std::fs::write(dir.join("bad.txt"), b"ab\xFFc").unwrap();
    let err = RealFs { root: dir.clone() }.read("bad.txt").unwrap_err();
    assert_eq!((&*err.kind_p, err.message_p.to_string()), ("Fs.NotText", "bad.txt: not UTF-8: byte 2 is malformed".to_string()));
    std::fs::remove_dir_all(dir).unwrap();
}

// ---- Fs.List ----------------------------------------------------------------------------

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("prism-world-cli-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_listing_is_recursive_sorted_and_only_prism_files() {
    let dir = temp_dir("list");
    for f in ["tests/b.prismtest", "tests/b.expected", "a.prism", "tests/deep/c.prism", "notes.md", ".hidden/x.prism", "z.prism"] {
        let p = dir.join(f);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, "").unwrap();
    }
    let world = CliWorld::new(RealFs { root: dir.clone() }, vec![]);
    let list = |d: &str| world.ready().first_value(&Pod::element("Fs.List", vec![("dir", Pod::text(d))]));
    let texts = |xs: &[&str]| Some(Pod::Array(xs.iter().map(|x| Pod::text(*x)).collect()));
    assert_eq!(list("."), texts(&["a.prism", "tests/b.prismtest", "tests/deep/c.prism", "z.prism"]));
    assert_eq!(list("tests"), texts(&["tests/b.prismtest", "tests/deep/c.prism"]));
    assert_eq!(list("tests/"), texts(&["tests/b.prismtest", "tests/deep/c.prism"]));
    assert_eq!(list("missing"), texts(&[]));
    std::fs::remove_dir_all(dir).unwrap();
}

// lister.prism: a listing heard live, and a write under it.
//
// @main:
//     $files <- Fs.List(dir="src")                                       # 0
//     !started <- @started
//     !~wrote <- Fs.File(path="src/new.prism", requests=(Write "" on !started))   # 1
//     Process.Exit(code=(Len $files on !Changed $files))                 # 2

struct Lister {
    path: Path,
    files: Hold<Pod>,
}

impl Root for Lister {
    fn construct(path: Path, cx: &mut Cx) -> Result<Rc<Lister>, Fault> {
        let el = Pod::element("Fs.List", vec![("dir", Pod::text("src"))]);
        let files = cx.first_value(&el).ok_or(Fault::answer("Fs.List unanswered"))?;
        let node = Rc::new(Lister { path, files: Hold::new(files, cx) });
        cx.added(node.path.at(0), el.clone());
        cx.added(node.path.at(1), Pod::element("Fs.File", vec![("path", Pod::text("src/new.prism"))]));
        cx.added(node.path.at(2), Pod::element("Process.Exit", vec![]));
        let me: Rc<dyn Node> = node.clone();
        cx.listen(el, &me);
        cx.on_started(node.clone());
        Ok(node)
    }
}

impl Node for Lister {
    fn started(self: Rc<Self>, cx: &mut Cx) -> Done {
        cx.request(self.path.at(1), None, Pod::Choice("Write".into(), vec![Pod::text("")]));
        Ok(())
    }
    fn answer(self: Rc<Self>, at: &[Segment], _: &Pod, _: &mut Cx) -> Done {
        if at == [Segment::Pos(1)] { Ok(()) } else { no_place(at) }
    }
    fn push(self: Rc<Self>, _: &Pod, v: &Pod, cx: &mut Cx) -> Done {
        self.files.set(v.clone(), cx);
        let Pod::Array(xs) = v else { return Err(Fault::answer("expected [Text]")) };
        cx.request(self.path.at(2), None, (xs.len() as u8).to_pod());
        Ok(())
    }
}

#[test]
fn a_write_under_a_listed_directory_pushes_the_new_listing() {
    let mut fs = MemFs::default();
    fs.files.insert("src/a.prism".into(), String::new());
    fs.files.insert("src/b.txt".into(), String::new());
    let mut world = CliWorld::new(fs, vec![]);
    assert_eq!(drive::<Prog<Lister>>(&mut world, &[], 100), 2);
}

// ---- @Later -----------------------------------------------------------------------------
//
// @main:
//     !started <- @started
//     !woke <- Time.After(wakes=(Wake(after=DELAY, context="0") on !started))   # 0
//     Process.Exit(code=(TickNumber on !woke))                                  # 1
//
// `@Later` is the zero-delay case (Semantics Appendix B); the exit code is the tick the
// wake arrived in, so the test sees "one tick later".

struct Later<const MS: u64> {
    path: Path,
}

impl<const MS: u64> Root for Later<MS> {
    fn construct(path: Path, cx: &mut Cx) -> Result<Rc<Self>, Fault> {
        let node = Rc::new(Later { path });
        cx.added(node.path.at(0), Pod::element("Time.After", vec![]));
        cx.added(node.path.at(1), Pod::element("Process.Exit", vec![]));
        cx.on_started(node.clone());
        Ok(node)
    }
}

impl<const MS: u64> Node for Later<MS> {
    fn started(self: Rc<Self>, cx: &mut Cx) -> Done {
        let wake = Pod::record("Wake", vec![("after", Pod::Duration(i128::from(MS) * 1_000_000)), ("context", Pod::text("0"))]);
        cx.request(self.path.at(0), None, wake);
        Ok(())
    }
    fn answer(self: Rc<Self>, at: &[Segment], v: &Pod, cx: &mut Cx) -> Done {
        match at {
            [Segment::Pos(0)] if v.field("context") == Some(&Pod::text("0")) => {
                cx.request(self.path.at(1), None, (cx.tick() as u8).to_pod());
                Ok(())
            }
            _ => no_place(at),
        }
    }
}

#[test]
fn at_later_is_answered_one_tick_later() {
    let mut world = CliWorld::new(MemFs::default(), vec![]);
    // Tick 1 is the started tick, which asks; tick 2 hears the wake.
    assert_eq!(drive::<Prog<Later<0>>>(&mut world, &[], 100), 2);
}

#[test]
fn a_wake_with_a_delay_waits_for_it() {
    let mut world = CliWorld::new(MemFs::default(), vec![]);
    let t = Instant::now();
    assert_eq!(drive::<Prog<Later<30>>>(&mut world, &[], 100), 2);
    assert!(t.elapsed() >= Duration::from_millis(30), "{:?}", t.elapsed());
}

// ---- Patience ---------------------------------------------------------------------------

/// A node that keeps a `@Defer` armed for ever: every tick owes the next.
struct Forever {
    defer: Defer<()>,
}

impl Root for Forever {
    fn construct(_: Path, cx: &mut Cx) -> Result<Rc<Self>, Fault> {
        let node = Rc::new(Forever { defer: Defer::new() });
        node.defer.arm((), node.clone(), cx);
        Ok(node)
    }
}

impl Node for Forever {
    fn deferred(self: Rc<Self>, cx: &mut Cx) -> Done {
        self.defer.arm((), self.clone(), cx);
        Ok(())
    }
}

#[test]
fn a_run_that_exhausts_patience_exits_1() {
    let mut world = CliWorld::new(MemFs::default(), vec![]);
    assert_eq!(drive::<Prog<Forever>>(&mut world, &[], 5), 1);
}

#[test]
fn a_program_with_nothing_left_to_do_exits_0() {
    struct Quiet;
    impl Node for Quiet {}
    impl Root for Quiet {
        fn construct(_: Path, _: &mut Cx) -> Result<Rc<Self>, Fault> {
            Ok(Rc::new(Quiet))
        }
    }
    let mut world = CliWorld::new(MemFs::default(), vec![]);
    assert_eq!(drive::<Prog<Quiet>>(&mut world, &[], 5), 0);
}
