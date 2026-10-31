//! `run` under the CLI World and `check`, and the diagnostics a program meets before
//! tick 0 or when evaluation reaches an out-of-scope feature.

use prism_zero::driver::{check_files, run_files, test_files};

fn src(name: &str, text: &str) -> Vec<(String, String)> {
    vec![(name.to_string(), text.to_string())]
}

#[test]
fn process_exit_ends_the_program_with_its_code() {
    let p = "emit Process.*\n\n@main:\n    !s <- @started\n    Process.Exit(code=(3 on !s))\n";
    assert_eq!(run_files(&src("exit.prism", p), vec![]).code, 3);
}

#[test]
fn a_program_with_nothing_to_wait_for_exits_zero() {
    let p = "emit Process.*\n\n@main:\n    let x = 1\n    @ x\n";
    assert_eq!(run_files(&src("quiet.prism", p), vec![]).code, 0);
}

#[test]
fn fs_output_writes_its_file() {
    let dir = std::env::temp_dir().join(format!("prism_zero_out_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let target = dir.join("hello.txt");
    let p = format!(
        "emit Fs.*\n\n@main:\n    ignore <- Fs.Output(path=\"{}\", text=$\"hello\\n\")\n",
        target.to_string_lossy()
    );
    let out = run_files(&src("write.prism", &p), vec![]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "hello\n");
}

#[test]
fn an_unwrapped_fault_at_the_root_exits_one() {
    // Semantics ch. 10: with no supervisor the root is poisoned; CLI policy, exit 1.
    let p = "emit Fs.* | Prelude.*\nuse Fs.*\n\n@main:\n    !started <- @started\n    !~read <- Fs.File(path=\"/no/such/file/anywhere\", requests=(Read on !started))\n    ignore <- Prelude.Unwrap(value=!~read)\n";
    assert_eq!(run_files(&src("unwrap.prism", p), vec![]).code, 1);
}

#[test]
fn the_root_row_is_checked_against_the_world() {
    // Semantics ch. 10, "The row check": Ui under the CLI World never starts.
    let p = "emit Ui.*\n\n@main:\n    Ui.Label(text=$\"hi\")\n";
    let out = run_files(&src("ui.prism", p), vec![]);
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("@main may emit the family Ui"), "{}", out.stderr);
}

#[test]
fn run_needs_exactly_one_root() {
    let a = ("a.prism".to_string(), "@main:\n    @ 1\n".to_string());
    let b = ("b.prism".to_string(), "@main:\n    @ 2\n".to_string());
    let out = run_files(&[a, b], vec![]);
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("exactly one file with no `module` line"), "{}", out.stderr);
}

#[test]
fn colliding_bare_names_are_an_error_at_the_use() {
    let a = ("a.prism".to_string(), "module A\nemit r\n\nF: fn Nat -> Nat\nfn F n = n\n".to_string());
    let b = ("b.prism".to_string(), "module B\nemit r\n\nF: fn Nat -> Nat\nfn F n = n\n".to_string());
    let root = ("main.prism".to_string(), "use A.*\nuse B.*\n\n@main:\n    @ F 1\n".to_string());
    let out = check_files(&[a, b, root]);
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("F is brought in by both A and B"), "{}", out.stderr);
}

#[test]
fn a_use_of_a_missing_module_is_an_error() {
    let out = check_files(&src("m.prism", "use Nowhere\n\n@main:\n    @ 1\n"));
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("Nowhere, a module not among the files"), "{}", out.stderr);
}

#[test]
fn check_reports_unknown_names() {
    let out = check_files(&src("m.prism", "@main:\n    @ Missing 1\n"));
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("unknown name Missing"), "{}", out.stderr);
}

#[test]
fn an_out_of_scope_feature_stops_where_evaluation_reaches_it() {
    let p = "Twice: fn Nat -> Nat\nfn Twice n = n +Nat n\n\ncheck twice:\n    Twice 2 => 4\n";
    let out = test_files(&src("oos.prism", p), &[]);
    assert_eq!(out.code, 1);
    assert!(out.stdout.contains("out of scope (suffixed operators)"), "{}", out.stdout);
}

#[test]
fn two_events_in_one_operation_are_refused() {
    let p = "emit Ui.*\n\n@adder:\n    !a <- Ui.Button(label=$\"Add 1\")\n    !b <- Ui.Button(label=$\"Add 10\")\n    let !total = (1 on !a) + (10 on !b)\n    $sum <- 0 then !total\n\ntest adder, @adder:\n    build\n";
    let out = test_files(&src("adder.prism", p), &[]);
    assert_eq!(out.code, 1);
    assert!(out.stdout.contains("two events in one operation"), "{}", out.stdout);
}

#[test]
fn a_prompt_cycle_is_refused_naming_its_binding() {
    let p = "emit Ui.*\n\n@broken:\n    !x <- Ui.Button(label=$\"x\")\n    let !a = 1 + !b\n    let !b = 2 + !a\n    $h <- 0 then !a\n\ntest broken, @broken:\n    build\n";
    let out = test_files(&src("broken.prism", p), &[]);
    assert_eq!(out.code, 1);
    assert!(out.stdout.contains("in terms of itself"), "{}", out.stdout);
}

#[test]
fn a_dynamic_in_an_identity_field_names_rebuild() {
    let p = "emit Ui.*\n\n@pic:\n    !b <- Ui.Button(label=$\"x\")\n    $src <- \"a.png\" then \"b.png\" on !b\n    Ui.Image(src=$src)\n\ntest pic, @pic:\n    build\n";
    let out = test_files(&src("pic.prism", p), &[]);
    assert_eq!(out.code, 1);
    assert!(out.stdout.contains("identity field `src`") && out.stdout.contains("rebuild"), "{}", out.stdout);
}
