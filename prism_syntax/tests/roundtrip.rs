//! Round trip: for every book example that parses, and every program in `tests/programs/`,
//! printing the tree and parsing the print gives an equal tree (spans aside), and the
//! print is a fixed point of printing.

use prism_syntax::{Parsed, ast::Module, diag::render_all, parse_fragment, parse_module, pretty, sexp};
use std::fs;
use std::path::Path;

fn sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut out = Vec::new();
    for dir in ["book", "programs"] {
        let Ok(rd) = fs::read_dir(root.join(dir)) else { continue };
        let mut names: Vec<_> = rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "prism")).collect();
        names.sort();
        for p in names {
            out.push((format!("{dir}/{}", p.file_name().unwrap().to_string_lossy()), fs::read_to_string(&p).unwrap()));
        }
    }
    out
}

fn parse_either(src: &str) -> Option<(Parsed<Module>, bool)> {
    let m = parse_module(src);
    if m.parsed_ok() {
        return Some((m, false));
    }
    let f = parse_fragment(src);
    if f.parsed_ok() {
        return Some((f, true));
    }
    None
}

#[test]
fn print_then_parse_gives_the_same_tree() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for (name, src) in sources() {
        let Some((first, fragment)) = parse_either(&src) else { continue };
        checked += 1;
        let printed = pretty::module(&first.tree);
        let second = if fragment { parse_fragment(&printed) } else { parse_module(&printed) };
        if !second.parsed_ok() {
            failures.push(format!("{name}: the print does not parse:\n{}\n--- print ---\n{printed}", render_all(&printed, &second.diagnostics)));
            continue;
        }
        if second.tree != first.tree {
            failures.push(format!(
                "{name}: the trees differ\n--- first ---\n{}\n--- second ---\n{}\n--- print ---\n{printed}",
                sexp::module(&first.tree),
                sexp::module(&second.tree)
            ));
            continue;
        }
        let again = pretty::module(&second.tree);
        if again != printed {
            failures.push(format!("{name}: printing is not a fixed point\n--- once ---\n{printed}\n--- twice ---\n{again}"));
        }
    }
    println!("{checked} files round-tripped");
    assert!(failures.is_empty(), "{} failures:\n\n{}", failures.len(), failures.join("\n\n"));
}
