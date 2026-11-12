//! `Upper` and `Lower` (`tests/text/case.txt`, its format and the statement it pins in
//! `tests/text/README.md`) against the Rust target. The prelude emitted beside every Rust
//! program (prismc/emit/rust/prelude.prism) defines them as `Text::from(t.as_str()
//! .to_uppercase())` and `.to_lowercase()`; this test runs those bodies over the runtime's
//! `Text`.
//!
//! Written by Claude on the claude-self-hosting branch; subject to change.

use prism_rtl::Text;

/// A field of the file, decoded: `\u{hex}` a code point, `\\` a backslash.
fn decode(field: &str) -> String {
    let mut out = String::new();
    let mut rest = field;
    while let Some(at) = rest.find('\\') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        if let Some(after) = after.strip_prefix('\\') {
            out.push('\\');
            rest = after;
        } else if let Some(after) = after.strip_prefix("u{") {
            let close = after.find('}').expect("an unclosed \\u{ in case.txt");
            let c = u32::from_str_radix(&after[..close], 16).ok().and_then(char::from_u32);
            out.push(c.expect("a bad \\u{hex} in case.txt"));
            rest = &after[close + 1..];
        } else {
            panic!("an unknown escape in case.txt: \\{after}");
        }
    }
    out.push_str(rest);
    out
}

/// The prelude's `Upper` and `Lower`, word for word.
fn upper(t: &Text) -> Text {
    Text::from(t.as_str().to_uppercase())
}
fn lower(t: &Text) -> Text {
    Text::from(t.as_str().to_lowercase())
}

#[test]
fn case_file() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/text/case.txt");
    let file = std::fs::read_to_string(path).expect("tests/text/case.txt");
    let mut failures = Vec::new();
    let mut cases = 0;
    for (n, line) in file.lines().enumerate() {
        if line.starts_with('#') || line.trim().is_empty() && !line.contains('\t') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        assert_eq!(f.len(), 3, "case.txt line {}: three fields", n + 1);
        let (input, up, low) = (Text::from(decode(f[0]).as_str()), decode(f[1]), decode(f[2]));
        cases += 1;
        for (what, got, want) in [("Upper", upper(&input), up), ("Lower", lower(&input), low)] {
            if got.as_str() != want {
                failures.push(format!("line {}: {what} {:?} is {:?}, expected {:?}", n + 1, input.as_str(), got.as_str(), want));
            }
        }
    }
    assert!(cases > 0, "case.txt has no cases");
    assert!(failures.is_empty(), "{} of {cases} cases fail:\n{}", failures.len(), failures.join("\n"));
}
