//! A widget block's binding group (Semantics ch. 4, "Binding groups and cycles"; ch. 7,
//! "There are no cycles"): lines read names bound below them, positions follow the page,
//! and a `for` gathers its rows by what their last line answers.

use prism_zero::driver::test_files;

fn run(program: &str, script: &str) -> prism_zero::Outcome {
    test_files(&[("p.prism".to_string(), program.to_string())], &[("p.prismtest".to_string(), script.to_string())])
}

const SHOWN: &str = "@Shown: fn $Int -> @$Text\nfn @Shown $n:\n    Ui.Label(text=$\"{$n}\")\n    @ $\"n={$n}\"\n\n";

#[test]
fn an_event_let_read_above_the_lines_it_needs_is_a_forward_event() {
    // Line 0 reads `!said`, which samples `$text`, bound by line 2: the `let` is
    // evaluated once the lines have run, and line 0 holds a forward event until then.
    let p = format!(
        "emit Ui.* | Test.*\n\n{SHOWN}@demo:\n    Test.Log(requests=!said)\n    !press <- Ui.Button(label=$\"Go\")\n    $n <- 0 then $n + 1 on !press\n    $text <- @Shown $n\n    let !said = $text on !press\n    @ $text\n"
    );
    let s = "test said, @demo:\n    build\n        => returns \"n=0\"\n    click Ui.Button(label=\"Go\")\n        => returns \"n=1\"\n";
    let out = run(&p, s);
    assert_eq!(out.code, 0, "{}{}", out.stdout, out.stderr);
    // The sample reads `$text` as the tick opened.
    assert!(out.stdout.contains("  requested:\n    0: \"n=0\"\n"), "{}", out.stdout);
    assert!(out.stdout.contains("    2: Ui.Label(text=\"0\")\n"), "{}", out.stdout);
}

#[test]
fn a_dynamic_read_above_a_bind_that_takes_no_position_runs_that_bind_first() {
    let p = "emit Ui.*\n\n@Doubled: fn $Int -> @$Text\nfn @Doubled $n:\n    @ $\"{$n * 2}\"\n\n@demo:\n    Ui.Label(text=$doubled)\n    !press <- Ui.Button(label=$\"Go\")\n    $n <- 1 then $n + 1 on !press\n    $doubled <- @Doubled $n\n";
    let s = "test doubled, @demo:\n    build\n        => expect Ui.Label(text=\"2\")\n    click Ui.Button(label=\"Go\")\n        => expect Ui.Label(text=\"4\")\n";
    let out = run(p, s);
    assert_eq!(out.code, 0, "{}{}", out.stdout, out.stderr);
    // Positions still follow the page.
    assert!(out.stdout.contains("    0: Ui.Label(text=\"2\")\n    1: Ui.Button(label=\"Go\")\n"), "{}", out.stdout);
}

#[test]
fn a_dynamic_read_above_a_bind_that_takes_a_position_is_refused() {
    // prism_rt has no forward dynamic (README.md, "What fails").
    let p = format!("emit Ui.*\n\n{SHOWN}@demo:\n    Ui.Label(text=$text)\n    $text <- @Shown $\"3\"\n");
    let out = run(&p, "test refused, @demo:\n    build\n");
    assert_eq!(out.code, 1, "{}{}", out.stdout, out.stderr);
    assert!(out.stdout.contains("$text is bound at p.prism:10:5, below this line"), "{}", out.stdout);
    assert!(out.stdout.contains("needs a forward dynamic"), "{}", out.stdout);
}

#[test]
fn a_for_of_elements_answering_dynamics_gathers_them() {
    // `Gui.Metrics` answers `$Extent` (prelude/gui.prism; prism_rt's families), so the rows
    // end on dynamics and the `for` gives `$(K ==> Extent)`.
    let p = "emit Gui.*\n\n@demo:\n    $sizes <- for w <- [[\"ab\", \"cde\"]]:\n        Gui.Metrics(text=w)\n    @ $sizes\n";
    let s = "test metrics, @demo:\n    build\n        => expect_count Gui.Metrics(..), 2\n        => returns {0: Extent(w=18.0, h=14.0), 1: Extent(w=27.0, h=14.0)}\n";
    let out = run(p, s);
    assert_eq!(out.code, 0, "{}{}", out.stdout, out.stderr);
}
