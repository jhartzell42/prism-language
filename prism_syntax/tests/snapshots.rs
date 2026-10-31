//! AST snapshots of fourteen book examples, chosen to cover every part of the grammar:
//! a whole file with its header and a check (Semantics ch. 3), blocks (ch. 4), `&`
//! blocks (ch. 5), modules (ch. 6), compute ticks (ch. 7), hosting (ch. 9), faults
//! (ch. 10), sigils on definitions and traits (Type System chs. 1 and 3), code templates
//! (Templates ch. 2), and tests (Conformance ch. 1 and Appendix A).
//!
//! Each `tests/snapshots/NAME.sexp` is `sexp::module` of `tests/book/NAME.prism`, checked
//! by hand when written. `UPDATE_SNAPSHOTS=1` rewrites them.

use prism_syntax::{parse_module, sexp};
use std::fs;
use std::path::Path;

const SNAPSHOTS: &[&str] = &[
    "sem-ch03-06", "sem-ch04-01", "sem-ch04-02", "sem-ch05-04", "sem-ch05-07", "sem-ch06-02", "sem-ch07-10",
    "sem-ch09-01", "sem-ch10-05", "ts-ch01-08", "ts-ch03-09", "tpl-ch02-02", "conf-ch01-05", "conf-chA-01",
];

#[test]
fn ast_snapshots() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let update = std::env::var("UPDATE_SNAPSHOTS").is_ok();
    let mut failures = Vec::new();
    for name in SNAPSHOTS {
        let src = fs::read_to_string(root.join("book").join(format!("{name}.prism"))).unwrap();
        let parsed = parse_module(&src);
        assert!(parsed.parsed_ok(), "{name}: {:?}", parsed.diagnostics);
        let got = sexp::module(&parsed.tree);
        let file = root.join("snapshots").join(format!("{name}.sexp"));
        if update {
            fs::write(&file, &got).unwrap();
            continue;
        }
        let want = fs::read_to_string(&file).unwrap_or_default();
        if got != want {
            failures.push(format!("{name}:\n--- want ---\n{want}--- got ---\n{got}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
