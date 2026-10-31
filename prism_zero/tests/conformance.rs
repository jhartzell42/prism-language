//! The conformance transcripts: every `tests/conformance/<name>.prismtest` runs under the
//! Test World against `<name>.prism` (and every `.prism` in `<name>/`, for a program of
//! several files), and its transcript is compared byte for byte with `<name>.expected`,
//! the comment lines at its top aside. Where the book prints a transcript, the
//! `.expected` file is the book's and says so on its first line; where it prints none,
//! the file is prism_zero's output, checked by hand against the book's rules.
//!
//! `UPDATE_EXPECTED=1 cargo test -p prism_zero` writes a missing `.expected` from the
//! run; it never rewrites one that exists.

use std::path::{Path, PathBuf};

use prism_zero::driver::{read_files, test_files};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn files_of(base: &Path) -> Vec<String> {
    let mut files = Vec::new();
    let single = base.with_extension("prism");
    if single.exists() {
        files.push(single.to_string_lossy().to_string());
    }
    if base.is_dir() {
        let mut more: Vec<String> = std::fs::read_dir(base)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "prism"))
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        more.sort();
        files.extend(more);
    }
    files
}

#[test]
fn every_conformance_transcript_matches() {
    let dir = root().join("tests/conformance");
    let mut scripts: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "prismtest"))
        .collect();
    scripts.sort();
    assert!(scripts.len() >= 15, "the conformance directory holds {} scripts", scripts.len());
    let mut failures = Vec::new();
    for script in &scripts {
        let base = script.with_extension("");
        let files = read_files(&files_of(&base)).unwrap();
        let s = vec![(script.to_string_lossy().to_string(), std::fs::read_to_string(script).unwrap())];
        let expected = script.with_extension("expected");
        let out = test_files(&files, &s);
        if !expected.exists() {
            if std::env::var("UPDATE_EXPECTED").is_ok() && out.code == 0 {
                let header = "# prism_zero's transcript; the book prints none for this program, and it was checked by hand against the book's rules.\n";
                std::fs::write(&expected, format!("{header}{}", out.stdout)).unwrap();
            } else {
                failures.push(format!("{}: no .expected (UPDATE_EXPECTED=1 writes it)", script.display()));
                continue;
            }
        }
        if out.code != 0 {
            failures.push(format!("{}: exit {}\n{}{}", script.display(), out.code, out.stdout, out.stderr));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// The examples committed beside this crate (examples/README.md): the counter, the half
/// counter, the todo list, and the chess rules module's checks.
#[test]
fn the_examples_pass_their_scripts() {
    let ex = root().join("examples");
    for (files, script) in [
        (vec!["counter.prism"], "counter.prismtest"),
        (vec!["half-counter.prism"], "half-counter.prismtest"),
        (vec!["todos.prism"], "todos.prismtest"),
        (vec!["chess/rules.prism"], "chess/rules.prismtest"),
    ] {
        let paths: Vec<String> = files.iter().map(|f| ex.join(f).to_string_lossy().to_string()).collect();
        let files = read_files(&paths).unwrap();
        let sp = ex.join(script);
        let s = vec![(sp.to_string_lossy().to_string(), std::fs::read_to_string(&sp).unwrap())];
        let out = test_files(&files, &s);
        assert_eq!(out.code, 0, "{script}:\n{}{}", out.stdout, out.stderr);
    }
}

/// The chess board's script, through the window library under `@$main`: the binding
/// group of `@Window` (a supplier lambda at line 0 reading `!route`, an event `let` that
/// depends on lines below it) and its `for` of `Gui.Metrics` rows, gathered as dynamics
/// because the tag answers `$Extent`. The transcript is compared with `board.expected`.
#[test]
fn the_chess_board_passes_its_script() {
    // The window library's layout recurses deeper than a test thread's 2 MB stack in a
    // debug build (it needs about 3 MB); the binary's main thread has 8 MB.
    let run = std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        let ex = root().join("examples/chess");
        let paths: Vec<String> =
            ["rules.prism", "window.prism", "board.prism"].iter().map(|f| ex.join(f).to_string_lossy().to_string()).collect();
        let files = read_files(&paths).unwrap();
        let sp = ex.join("board.prismtest");
        let s = vec![(sp.to_string_lossy().to_string(), std::fs::read_to_string(&sp).unwrap())];
        test_files(&files, &s)
    });
    let out = run.unwrap().join().unwrap();
    assert_eq!(out.code, 0, "{}{}", out.stdout, out.stderr);
}
