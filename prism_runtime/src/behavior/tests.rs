use std::sync::Arc;

use crate::{behavior::Behavior, event::Event, runtime::Runtime, value::Value};

#[test]
fn a_behavior_that_never_changes_does_not_collect_dependents() {
    let mut runtime = Runtime::new();
    let (unchanging, _unchanging_trigger) = Event::<u32>::external();
    let (changing, changing_trigger) = Event::<u32>::external();
    let sum = Behavior::map2(
        |a: Value<u32>, b: Value<u32>| Value::from(*a + *b),
        Behavior::hold(&runtime, 0u32.into(), unchanging),
        Behavior::hold(&runtime, 0u32.into(), changing),
    );

    for value in 0..1000u32 {
        runtime.schedule_trigger(&changing_trigger, value.into()).unwrap();
        runtime.propagate();
        sum.0.query_for_tag();
    }

    assert!(Arc::weak_count(&sum.0) < 10);
}

#[test]
fn joined_behavior_starts_at_and_follows_the_initial_inner() {
    let mut runtime = Runtime::new();
    let (first_changes, first_trigger) = Event::<u32>::external();
    let (outer_changes, _outer_trigger) = Event::<Arc<Behavior<u32>>>::external();
    let first = Behavior::hold(&runtime, 0u32.into(), first_changes);
    let joined = Behavior::hold(&runtime, Value::from(Arc::new(first)), outer_changes).join();
    let plus_one = joined.map(|x| Value::from(*x + 1));

    assert_eq!(*plus_one.0.query_for_tag(), 1);
    runtime.schedule_trigger(&first_trigger, 5u32.into()).unwrap();
    runtime.propagate();

    assert_eq!(*joined.0.query_for_tag(), 5);
    assert_eq!(*plus_one.0.query_for_tag(), 6);
}

#[test]
fn joined_behavior_follows_a_new_inner_and_ignores_the_old_one() {
    let mut runtime = Runtime::new();
    let (first_changes, first_trigger) = Event::<u32>::external();
    let (second_changes, second_trigger) = Event::<u32>::external();
    let (outer_changes, outer_trigger) = Event::<Arc<Behavior<u32>>>::external();
    let first = Behavior::hold(&runtime, 0u32.into(), first_changes);
    let second = Behavior::hold(&runtime, 100u32.into(), second_changes);
    let joined = Behavior::hold(&runtime, Value::from(Arc::new(first.clone())), outer_changes).join();

    runtime.schedule_trigger(&outer_trigger, Value::from(Arc::new(second))).unwrap();
    runtime.propagate();
    assert_eq!(*joined.0.query_for_tag(), 100);

    runtime.schedule_trigger(&first_trigger, 7u32.into()).unwrap();
    runtime.propagate();
    assert_eq!(*joined.0.query_for_tag(), 100);

    runtime.schedule_trigger(&second_trigger, 200u32.into()).unwrap();
    runtime.propagate();
    assert_eq!(*joined.0.query_for_tag(), 200);

    runtime.schedule_trigger(&outer_trigger, Value::from(Arc::new(first))).unwrap();
    runtime.propagate();
    assert_eq!(*joined.0.query_for_tag(), 7);
}

#[test]
fn joined_behavior_takes_the_new_inner_value_when_it_changes_in_the_switch_occurrence() {
    let mut runtime = Runtime::new();
    let (first_changes, _first_trigger) = Event::<u32>::external();
    let (second_changes, second_trigger) = Event::<u32>::external();
    let (outer_changes, outer_trigger) = Event::<Arc<Behavior<u32>>>::external();
    let (probe, probe_trigger) = Event::<()>::external();
    let first = Behavior::hold(&runtime, 0u32.into(), first_changes);
    let second = Behavior::hold(&runtime, 100u32.into(), second_changes);
    let joined = Behavior::hold(&runtime, Value::from(Arc::new(first)), outer_changes).join();
    let sampled_during = Behavior::hold(&runtime, 
        9999u32.into(),
        probe.tag_map(|_, value: Value<u32>| Some(value), joined.clone()),
    );

    runtime.schedule_trigger(&outer_trigger, Value::from(Arc::new(second))).unwrap();
    runtime.schedule_trigger(&second_trigger, 300u32.into()).unwrap();
    runtime.schedule_trigger(&probe_trigger, ().into()).unwrap();
    runtime.propagate();

    assert_eq!(*sampled_during.0.query_for_tag(), 0);
    assert_eq!(*joined.0.query_for_tag(), 300);
}

#[test]
fn joined_behavior_ignores_the_old_inner_changing_in_the_switch_occurrence() {
    let mut runtime = Runtime::new();
    let (first_changes, first_trigger) = Event::<u32>::external();
    let (second_changes, second_trigger) = Event::<u32>::external();
    let (outer_changes, outer_trigger) = Event::<Arc<Behavior<u32>>>::external();
    let first = Behavior::hold(&runtime, 0u32.into(), first_changes);
    let second = Behavior::hold(&runtime, 100u32.into(), second_changes);
    let joined = Behavior::hold(&runtime, Value::from(Arc::new(first)), outer_changes).join();

    runtime.schedule_trigger(&outer_trigger, Value::from(Arc::new(second))).unwrap();
    runtime.schedule_trigger(&first_trigger, 42u32.into()).unwrap();
    runtime.propagate();
    assert_eq!(*joined.0.query_for_tag(), 100);

    runtime.schedule_trigger(&second_trigger, 101u32.into()).unwrap();
    runtime.propagate();
    assert_eq!(*joined.0.query_for_tag(), 101);
}

#[test]
fn joined_behavior_sampled_mid_occurrence_shows_the_previous_value() {
    let mut runtime = Runtime::new();
    let (first_changes, first_trigger) = Event::<u32>::external();
    let (outer_changes, _outer_trigger) = Event::<Arc<Behavior<u32>>>::external();
    let (probe, probe_trigger) = Event::<()>::external();
    let first = Behavior::hold(&runtime, 0u32.into(), first_changes);
    let joined = Behavior::hold(&runtime, Value::from(Arc::new(first)), outer_changes).join();
    let sampled_during = Behavior::hold(&runtime, 
        9999u32.into(),
        probe.tag_map(|_, value: Value<u32>| Some(value), joined),
    );

    runtime.schedule_trigger(&first_trigger, 3u32.into()).unwrap();
    runtime.schedule_trigger(&probe_trigger, ().into()).unwrap();
    runtime.propagate();
    assert_eq!(*sampled_during.0.query_for_tag(), 0);

    runtime.schedule_trigger(&probe_trigger, ().into()).unwrap();
    runtime.propagate();
    assert_eq!(*sampled_during.0.query_for_tag(), 3);
}

#[test]
fn joined_behavior_follows_a_computed_inner() {
    let mut runtime = Runtime::new();
    let (first_changes, first_trigger) = Event::<u32>::external();
    let (outer_changes, outer_trigger) = Event::<Arc<Behavior<u32>>>::external();
    let first = Behavior::hold(&runtime, 0u32.into(), first_changes);
    let times_ten = first.map(|x| Value::from(*x * 10));
    let joined = Behavior::hold(&runtime, Value::from(Arc::new(first)), outer_changes).join();

    runtime.schedule_trigger(&outer_trigger, Value::from(Arc::new(times_ten))).unwrap();
    runtime.propagate();
    runtime.schedule_trigger(&first_trigger, 4u32.into()).unwrap();
    runtime.propagate();

    assert_eq!(*joined.0.query_for_tag(), 40);
}

#[test]
fn joined_behavior_dependents_stay_bounded_across_many_switches() {
    let mut runtime = Runtime::new();
    let (first_changes, first_trigger) = Event::<u32>::external();
    let (second_changes, _second_trigger) = Event::<u32>::external();
    let (outer_changes, outer_trigger) = Event::<Arc<Behavior<u32>>>::external();
    let first = Behavior::hold(&runtime, 0u32.into(), first_changes);
    let second = Behavior::hold(&runtime, 100u32.into(), second_changes);
    let joined = Behavior::hold(&runtime, Value::from(Arc::new(first.clone())), outer_changes).join();
    let plus_one = joined.map(|x| Value::from(*x + 1));

    for step in 0..500u32 {
        let inner = if step % 2 == 0 { second.clone() } else { first.clone() };
        runtime.schedule_trigger(&outer_trigger, Value::from(Arc::new(inner))).unwrap();
        runtime.schedule_trigger(&first_trigger, step.into()).unwrap();
        runtime.propagate();
        let expected = if step % 2 == 0 { 100 } else { step };
        assert_eq!(*plus_one.0.query_for_tag(), expected + 1);
    }

    assert!(Arc::weak_count(&joined.0) < 10);
}
