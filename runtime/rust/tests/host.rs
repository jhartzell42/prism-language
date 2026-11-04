//! A host's lifetime: a host torn down with its generation is freed, its rows with it (D5:
//! no leaks, ever). The node below is what prismc emits for a generation of a `rebuild`
//! that holds a host line (`ignore <- @HostDocument @child, supplier`): the host in a field,
//! its rows in another, the host linked back to the generation weakly.

use std::rc::{Rc, Weak};

use prism_rtl::*;

#[derive(Clone)]
struct Answer;

impl AnswerOf<Pod> for Answer {
    fn decode(_: &Pod, _: &Pod) -> Result<Answer, Fault> {
        Ok(Answer)
    }
}

/// One generation: the host at place 0, and its rows.
struct Generation {
    host: Rc<Host<Pod, Answer, Generation>>,
    rows: Rows<Pod, Answer>,
}

impl Generation {
    fn build(path: Path, cx: &mut Cx) -> Result<Rc<Generation>, Fault> {
        let rows: Rows<Pod, Answer> = Rc::new(std::cell::RefCell::new(Map::new()));
        let supplier: Supplier<Pod, Answer> = Rc::new(|_: &Path, _: &Live<Pod>| plain_blueprint(Answer));
        let host = Host::new(path.at(0), rows.clone(), supplier, None, Generation::document, Generation::deliver);
        // The child's one place asks its first value at its build: the host builds its row.
        host.first_value(&Path::root().at(0), &Pod::int(1), cx)?;
        let node = Rc::new(Generation { host: host.clone(), rows });
        host.link(&node);
        Ok(node)
    }

    fn document(_: &Generation, _: &Cx) -> Map<Path, Pod> {
        let mut doc = Map::new();
        doc.insert(Path::root().at(0), Pod::int(1));
        doc
    }

    fn deliver(_: &Rc<Generation>, _: &Path, _: &Answer, _: &mut Cx) -> Done {
        Ok(())
    }
}

impl Node for Generation {
    /// The generation is replaced: its host's rows leave with it, at the end of the tick.
    fn leave(self: Rc<Self>, cx: &mut Cx) -> Done {
        for row in self.rows.borrow().values() {
            row.leave_row(cx);
            cx.leave_later(row.clone());
        }
        Ok(())
    }
}

#[test]
fn a_host_torn_down_with_its_generation_is_freed_with_its_rows() {
    let ready = ReadyMap::default();
    let mut rt = Runtime::new();
    let mut cx = rt.build(&ready);
    let generation = Generation::build(Path::root().at(0).child(Segment::Gen(0)), &mut cx).unwrap();
    rt.close(cx).unwrap();
    let host: Weak<Host<Pod, Answer, Generation>> = Rc::downgrade(&generation.host);
    let row: Weak<HostRow<Pod, Answer>> = Rc::downgrade(generation.rows.borrow().values().next().unwrap());
    let gen_weak = Rc::downgrade(&generation);

    // The `rebuild` swaps in its next generation: the old one leaves and is dropped.
    let mut cx = rt.build(&ready);
    generation.clone().leave(&mut cx).unwrap();
    drop(generation);
    rt.close(cx).unwrap();
    rt.compact();
    assert!(gen_weak.upgrade().is_none(), "the generation is freed");
    assert!(host.upgrade().is_none(), "the host is freed with its generation (D5)");
    assert!(row.upgrade().is_none(), "the host's row is freed with its host (D5)");
}

#[test]
fn a_host_dropped_with_its_rows_standing_is_freed() {
    let ready = ReadyMap::default();
    let mut rt = Runtime::new();
    let mut cx = rt.build(&ready);
    let generation = Generation::build(Path::root().at(0), &mut cx).unwrap();
    rt.close(cx).unwrap();
    let host = Rc::downgrade(&generation.host);
    let row = Rc::downgrade(generation.rows.borrow().values().next().unwrap());
    drop(generation);
    rt.compact();
    assert!(host.upgrade().is_none() && row.upgrade().is_none(), "no leaks, ever (D5)");
}
