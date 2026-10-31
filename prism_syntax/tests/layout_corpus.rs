//! The layout corpus as goldens (Semantics ch. 2, "The layout corpus, sampled": "Both
//! implementations keep the corpus as golden tests").
//!
//! Each `tests/corpus/NAME.prism` lexes to the stream in `NAME.tokens`, printed in the
//! corpus's notation (`open`, `sep`, `close`, `free@`, `-pre`, `TEXT(...)`, `absent`,
//! `nonempty`, `·` between touching brackets; see `src/corpus.rs`), followed by the
//! lexer's diagnostics, one per line. `01-book-sample.tokens` is the book's own sample,
//! copied by hand; the rest were checked by hand against the rules they name.
//! `UPDATE_GOLDENS=1` rewrites the goldens.

use prism_syntax::{corpus, diag::render_all, lex};
use std::fs;
use std::path::Path;

#[test]
fn layout_corpus() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
    let mut names: Vec<_> = fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "prism")).collect();
    names.sort();
    let update = std::env::var("UPDATE_GOLDENS").is_ok();
    let mut failures = Vec::new();
    for p in &names {
        let src = fs::read_to_string(p).unwrap();
        let lexed = lex(&src);
        let mut got = corpus::render(&lexed.tokens);
        let diags = render_all(&src, &lexed.diagnostics);
        if !diags.is_empty() {
            got.push_str(&diags);
            got.push('\n');
        }
        let golden = p.with_extension("tokens");
        if update {
            fs::write(&golden, &got).unwrap();
            continue;
        }
        let want = fs::read_to_string(&golden).unwrap_or_default();
        if got != want {
            failures.push(format!("{}:\n--- want ---\n{want}--- got ---\n{got}", p.display()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
