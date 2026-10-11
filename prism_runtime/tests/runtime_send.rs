use prism_runtime::runtime::Runtime;

#[test]
fn runtime_can_move_to_another_thread() {
    let mut runtime = Runtime::new();
    std::thread::spawn(move || runtime.propagate()).join().unwrap();
}
