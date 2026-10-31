//! The collector (decisions.md D5): the zombie loop of Graph ch. 4 ("The one loop") is
//! built, torn down in one tick, and swept to zero leaked cells.

// Programs here pass a dynamic as `&[d.clone()]`, as an interpreter would.
#![allow(clippy::cloned_ref_to_slice_refs)]

mod common;

use common::*;
use prism_rt::gc::{self, debug::live_functions};
use prism_rt::{tick, Blueprint, Closure, Inputs, Program, ResultKind, State, Value};

/// ```prism
/// @loop:
///     !set <- Ui.Button(label=$"Set")                     # 0
///     !kill <- Ui.Button(label=$"Kill")                   # 1
///     $gen <- 0 then $gen + 1 on !kill
///     ignore <- rebuild $gen:                             # 2/#g
///         $peers <- unit then $rows on !set
///         $rows <- for {1: unit, 2: unit}:                # 2/#g/0/[k]
///             $h <- unit then (\|| $peers) on !set        # a closure over the peers
///             @ $h
/// ```
///
/// After Set, row 1's hold holds a closure that captured `$peers`, whose value names row
/// 2's hold, whose closure captured `$peers` again: two rows whose holds capture closures
/// over each other's holds. Kill tears both rows (and their generation) down in one tick;
/// their holds become constants that only each other keep.
fn zombie_loop() -> Program {
    Program::new(
        "loop",
        Blueprint::pure("@loop", |b, _| {
            let set = button(b, "Set")?;
            let kill = button(b, "Kill")?;
            let generation = b.hold(Value::int(0));
            let next = b.sample(&kill, &[generation.clone()], ints("+1", |a| a[1] + 1))?;
            b.hold_on(&generation, &next)?;
            let block = Blueprint::new("generation", vec![set], |b, cap, _| {
                let set = cap[0].clone();
                let peers = b.hold(Value::Unit);
                let row = Blueprint::new("row", vec![set.clone(), peers.clone()], |b, cap, _| {
                    let make = Closure::new("capture peers", vec![cap[1].clone()], |c, _| {
                        Ok(Value::Closure(Closure::new("peer", vec![c[0].clone()], |c, _| Ok(c[0].clone()))))
                    });
                    let h = b.hold(Value::Unit);
                    let captured = b.map(&cap[0], make)?;
                    b.hold_on(&h, &captured)?;
                    Ok(h)
                });
                let keys = b.constant(Value::map(vec![(Value::int(1), Value::Unit), (Value::int(2), Value::Unit)]));
                let rows = b.for_each(&keys, ResultKind::Plain, &row)?;
                let taken = b.tag(&set, &rows)?;
                b.hold_on(&peers, &taken)?;
                Ok(Value::Unit)
            });
            b.rebuild(&generation, ResultKind::Unit, &block)?;
            Ok(Value::Unit)
        }),
    )
}

fn click(path: &str) -> Inputs {
    Inputs::none().answer(prism_rt::pod::read_path(path).unwrap(), Value::click())
}

fn step(program: &Program, state: State, inputs: Inputs) -> State {
    let t = tick(program, Some(state), &inputs);
    t.outputs.expect("the tick completes");
    t.state.unwrap()
}

#[test]
fn the_zombie_loop_is_swept_to_zero_leaked_cells() {
    let program = zombie_loop();
    let t = tick(&program, None, &Inputs::none());
    let mut st = t.state.unwrap();
    drop(t.outputs);
    gc::collect(&mut st);
    let baseline_cells = st.live_cells();
    let baseline_functions = live_functions();

    // Set: each row's hold now holds a closure over the peers, which name both holds.
    st = step(&program, st, click("0"));
    gc::collect(&mut st);
    assert_eq!(st.live_cells(), baseline_cells, "Set makes values, not cells");
    assert_eq!(live_functions(), baseline_functions + 2, "two closures in the two holds");

    // Kill: generation #0 and both rows leave in one tick; #1 is built in their place.
    st = step(&program, st, click("1"));
    let before_sweep = st.live_cells();
    assert!(before_sweep > baseline_cells, "the dead generation is still in memory: {before_sweep}");
    let freed = gc::collect(&mut st);
    assert!(freed.nodes > 0 && freed.widgets == 3, "{freed:?}");
    assert_eq!(st.live_cells(), baseline_cells, "zero live cells left of the loop");
    assert_eq!(live_functions(), baseline_functions, "every closure the dead held is freed");

    // A second collection finds nothing.
    assert_eq!(gc::collect(&mut st), gc::Collected::default());
}

#[test]
fn without_the_sweep_the_loop_stays() {
    let program = zombie_loop();
    let mut st = tick(&program, None, &Inputs::none()).state.unwrap();
    st = step(&program, st, click("0"));
    let with_loop = st.live_cells();
    st = step(&program, st, click("1"));
    // Refcounts alone would never free these: the cells of #0 are still there.
    assert!(st.live_cells() > with_loop);
}

#[test]
fn many_generations_do_not_grow_memory() {
    let program = zombie_loop();
    let mut st = tick(&program, None, &Inputs::none()).state.unwrap();
    gc::collect(&mut st);
    let cells = st.live_cells();
    for _ in 0..50 {
        st = step(&program, st, click("0"));
        st = step(&program, st, click("1"));
        gc::collect(&mut st);
    }
    assert_eq!(st.live_cells(), cells);
}
