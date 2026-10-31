//! Every ```prism example in the Semantics, Type System, Templates and Conformance draft
//! books, extracted into `tests/book/` by `tests/extract_book_examples.py`.
//!
//! Each example must do what `tests/book/EXPECTED` says:
//!
//! - `module`: parses as a file with no diagnostics;
//! - `fragment`: parses as a run of statements (a widget block's lines shown without
//!   their widget) with no diagnostics;
//! - `out FEATURE`: parses, and is rejected with an out-of-scope diagnostic naming
//!   FEATURE (decisions.md's list);
//! - `lex`: a list of tokens or expressions shown one per line, not a program: it must
//!   lex with no diagnostic;
//! - `error`: the book shows the example to illustrate an error, or as something other
//!   than a program; the expectation records the first diagnostic.
//!
//! A `#` line in `EXPECTED` above an entry says why an example is not a clean module.
//!
//! `cargo test -p prism_syntax --test book_examples -- --nocapture` prints the table.

use prism_syntax::diag::render_all;
use prism_syntax::{Stage, parse_fragment, parse_module};
use std::fs;
use std::path::Path;

#[derive(Debug, PartialEq)]
enum Outcome {
    Module,
    Fragment,
    Out(Vec<String>),
    Error(String),
}

fn classify(src: &str) -> Outcome {
    let m = parse_module(src);
    if m.clean() {
        return Outcome::Module;
    }
    if m.parsed_ok() {
        let mut fs: Vec<String> = m.diagnostics.iter().filter_map(|d| d.feature.map(|f| f.to_string())).collect();
        fs.dedup();
        return Outcome::Out(fs);
    }
    let f = parse_fragment(src);
    if f.clean() {
        return Outcome::Fragment;
    }
    if f.parsed_ok() {
        let mut fs: Vec<String> = f.diagnostics.iter().filter_map(|d| d.feature.map(|f| f.to_string())).collect();
        fs.dedup();
        return Outcome::Out(fs);
    }
    let first = m.diagnostics.iter().find(|d| d.stage != Stage::OutOfScope).unwrap();
    Outcome::Error(render_all(src, std::slice::from_ref(first)))
}

fn expected() -> Vec<(String, String)> {
    let text = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/book/EXPECTED")).unwrap_or_default();
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (name, rest) = l.split_once(' ').unwrap_or((l, ""));
            (name.to_string(), rest.trim().to_string())
        })
        .collect()
}

fn render(o: &Outcome) -> String {
    match o {
        Outcome::Module => "module".into(),
        Outcome::Fragment => "fragment".into(),
        Outcome::Out(fs) => format!("out {}", fs.join(" + ")),
        Outcome::Error(e) => format!("error {}", e),
    }
}

#[test]
fn every_book_example() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/book");
    let mut names: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".prism"))
        .collect();
    names.sort();
    let exp = expected();
    let update = std::env::var("UPDATE_EXPECTED").is_ok();
    let mut lines = Vec::new();
    let mut failures = Vec::new();
    let (mut module, mut fragment, mut out, mut error) = (0, 0, 0, 0);
    for n in &names {
        let src = fs::read_to_string(dir.join(n)).unwrap();
        let o = classify(&src);
        match o {
            Outcome::Module => module += 1,
            Outcome::Fragment => fragment += 1,
            Outcome::Out(_) => out += 1,
            Outcome::Error(_) => error += 1,
        }
        let mut got = render(&o);
        // `lex`: the example is a list of tokens or expressions the book shows one per
        // line, not a program; it must lex with no diagnostic.
        let want_lex = exp.iter().any(|(e, w)| e == n && w == "lex");
        if want_lex && matches!(o, Outcome::Error(_)) && prism_syntax::lex(&src).diagnostics.is_empty() {
            got = "lex".into();
        }
        println!("{n:32} {got}");
        lines.push(format!("{n} {got}"));
        match exp.iter().find(|(e, _)| e == n) {
            Some((_, want)) if *want == got => {}
            Some((_, want)) => failures.push(format!("{n}: expected `{want}`, got `{got}`")),
            None => failures.push(format!("{n}: no expectation (got `{got}`)")),
        }
    }
    println!("\n{} examples: {module} modules, {fragment} fragments, {out} out of scope, {error} errors", names.len());
    if update {
        let header = "# Expected outcome of each book example; see tests/book_examples.rs.\n";
        fs::write(dir.join("EXPECTED"), format!("{header}{}\n", lines.join("\n"))).unwrap();
        return;
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
