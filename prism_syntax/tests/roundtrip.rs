//! Round trip: for every book example that parses, every program in `tests/programs/`,
//! and every `.prism` and `.prismtest` file under the workspace's `examples/` (chess, the
//! counter, the half counter, the todo list), printing the tree and parsing the print
//! gives an equal tree (spans aside), and the print is a fixed point of printing. The
//! examples must also parse with no diagnostic at all.

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
    out.extend(examples());
    out
}

/// The workspace's `examples/`, read where they live so the corpus cannot drift from
/// them: every `.prism` and `.prismtest` file, recursively.
fn examples() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "prism" || x == "prismtest") {
                out.push(p);
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples");
    let mut paths = Vec::new();
    walk(&root, &mut paths);
    paths.sort();
    paths
        .into_iter()
        .map(|p| (format!("examples/{}", p.strip_prefix(&root).unwrap().to_string_lossy()), fs::read_to_string(&p).unwrap()))
        .collect()
}

#[test]
fn every_example_parses_clean() {
    let ex = examples();
    assert!(ex.iter().any(|(n, _)| n == "examples/chess/rules.prism"), "the chess example is missing: {:?}", ex.iter().map(|e| &e.0).collect::<Vec<_>>());
    let mut failures = Vec::new();
    for (name, src) in &ex {
        let p = parse_module(src);
        if !p.clean() {
            failures.push(format!("{name}:\n{}", render_all(src, &p.diagnostics)));
        }
    }
    println!("{} example files parsed", ex.len());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
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
