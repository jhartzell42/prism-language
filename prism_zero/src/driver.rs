//! The three commands (docs/architecture.md, "The pipeline"):
//!
//! - **`prism_zero run <files...> [-- args...]`**: the program under the CLI World (`Fs`,
//!   `Process`, `Time`), its root the `@main` of the one file with no `module` line.
//!   Exit codes are the CLI World's policy (Semantics ch. 9): the code `Process.Exit`
//!   asked for; 0 when nothing can cause another tick; 1 for a poisoned root or a failed
//!   tick; 2 for a program that fails to load or check before tick 0.
//! - **`prism_zero test <files...> [script.prismtest...]`**: each script under the Test
//!   World, printing its transcript to standard output. With no script named, each
//!   `X.prism` given runs `X.prismtest` beside it if there is one, and its own `test` and
//!   `check` declarations otherwise. A script's tests are declarations of the module whose
//!   file it is named after, or of the root file (Semantics ch. 6, "`test` and `check` are
//!   declarations"). When `X.expected` sits beside a script, the transcript is compared
//!   with it byte for byte, comment lines beginning `# ` at its top ignored, and a
//!   difference fails the run. Exit codes are the Test World's: 0 every check passed, 1 a
//!   check failed (or the transcript differs), 2 the script or the program did not load.
//! - **`prism_zero check <files...>`**: parse and resolve only; 0, or 2 with diagnostics.

use std::collections::HashMap;
use std::rc::Rc;

use prism_rt::world::cli::{CliWorld, RealFs};
use prism_rt::world::test_world::TestWorld;
use prism_rt::world::{drive, EXIT_UNCHECKED};
use prism_rt::{Program, Value};
use prism_syntax::ast::*;

use crate::interp::{Interp, Loc, Scopes};
use crate::load::{load, Loaded, ModId, ModInfo};
use crate::world::{row_check, UnwrapWorld, CLI_FAMILIES};

/// What a command printed and the code it exits with.
#[derive(Debug, Clone, Default)]
pub struct Outcome {
    pub stdout: String,
    pub stderr: String,
    pub code: u8,
}

impl Outcome {
    fn failed(stderr: String, code: u8) -> Outcome {
        Outcome { stdout: String::new(), stderr, code }
    }
}

/// Read files from disk, as `(name, text)`.
pub fn read_files(paths: &[String]) -> Result<Vec<(String, String)>, String> {
    paths.iter().map(|p| std::fs::read_to_string(p).map(|t| (p.clone(), t)).map_err(|e| format!("{p}: {e}"))).collect()
}

/// The World's patience for `run`, in ticks.
pub const RUN_PATIENCE: usize = 10_000_000;

/// `prism_zero run`.
pub fn run_files(files: &[(String, String)], args: Vec<String>) -> Outcome {
    let loaded = match load(files) {
        Ok(l) => l,
        Err(es) => return Outcome::failed(es.join("\n") + "\n", EXIT_UNCHECKED),
    };
    let user_roots: Vec<&ModInfo> = loaded.modules.iter().filter(|m| m.is_root()).collect();
    let Some(root) = loaded.root else {
        let names: Vec<&str> = user_roots.iter().map(|m| m.file.as_str()).collect();
        return Outcome::failed(
            format!(
                "prism run needs exactly one file with no `module` line, and was given {}: {} (Semantics ch. 6)\n",
                names.len(),
                if names.is_empty() { "none".to_string() } else { names.join(", ") }
            ),
            EXIT_UNCHECKED,
        );
    };
    let main = ["@main", "@$main", "@!main"]
        .iter()
        .find(|n| loaded.modules[root].decls.contains_key(**n))
        .map(|n| n.to_string())
        .or_else(|| loaded.modules[root].decls.keys().find(|k| k.starts_with('@') && k.ends_with("main")).map(|k| k.to_string()));
    let Some(main) = main else {
        return Outcome::failed(format!("{}: the root file has no `@main` (Semantics ch. 6)\n", loaded.modules[root].file), EXIT_UNCHECKED);
    };
    if let Err(e) = row_check(&loaded, root, CLI_FAMILIES, "CLI") {
        return Outcome::failed(e + "\n", EXIT_UNCHECKED);
    }
    let interp = Interp::new(loaded);
    let loc = Loc { module: root, span: Default::default() };
    let g = interp.l.modules[root].decls[main.as_str()].clone();
    let bp = match interp.global(root, &main, &g, None, loc) {
        Ok(Value::Widget(bp)) => bp,
        Ok(_) => return Outcome::failed(format!("{main} is not a blueprint\n"), EXIT_UNCHECKED),
        Err(f) => return Outcome::failed(format!("{}\n", f.message), EXIT_UNCHECKED),
    };
    let program = Program::new("main", bp);
    let cwd = std::env::current_dir().unwrap_or_default();
    let mut world = UnwrapWorld::new(CliWorld::new(RealFs { root: cwd }, args));
    let code = drive(&mut world, &program, RUN_PATIENCE);
    Outcome { stdout: String::new(), stderr: String::new(), code }
}

/// `prism_zero check`.
pub fn check_files(files: &[(String, String)]) -> Outcome {
    match load(files) {
        Err(es) => Outcome::failed(es.join("\n") + "\n", EXIT_UNCHECKED),
        Ok(l) => {
            let es = crate::resolve::check(&l);
            if es.is_empty() {
                Outcome::default()
            } else {
                Outcome::failed(es.join("\n") + "\n", EXIT_UNCHECKED)
            }
        }
    }
}

/// A script to run: its name, text, the module its tests belong to, and the expected
/// transcript beside it, if any.
struct Script {
    name: String,
    text: String,
    module: Option<String>,
    expected: Option<String>,
}

/// `prism_zero test`. `scripts` are `(name, text)` pairs; with none, each file's
/// `.prismtest` beside it or its own tests are run.
pub fn test_files(files: &[(String, String)], scripts: &[(String, String)]) -> Outcome {
    let mut loaded = match load(files) {
        Ok(l) => l,
        Err(es) => return Outcome::failed(es.join("\n") + "\n", EXIT_UNCHECKED),
    };
    let module_of_file = |l: &Loaded, file: &str| -> Option<String> {
        l.modules.iter().find(|m| m.file == file).map(|m| m.file.clone())
    };
    let mut todo: Vec<Script> = Vec::new();
    let expected_beside = |script: &str| -> Option<String> {
        let p = std::path::Path::new(script).with_extension("expected");
        std::fs::read_to_string(p).ok()
    };
    if scripts.is_empty() {
        for (file, _) in files {
            let beside = std::path::Path::new(file).with_extension("prismtest");
            if let Ok(text) = std::fs::read_to_string(&beside) {
                let name = beside.to_string_lossy().to_string();
                todo.push(Script { expected: expected_beside(&name), name, text, module: module_of_file(&loaded, file) });
            } else if let Some(m) = loaded.modules.iter().find(|m| &m.file == file) {
                let mut text = String::new();
                for d in &m.ast.decls {
                    if matches!(d.kind, DeclKind::Test(_) | DeclKind::Check(_)) {
                        text.push_str(m.text(d.span));
                        text.push('\n');
                    }
                }
                if !text.is_empty() {
                    todo.push(Script { name: file.clone(), text, module: Some(m.file.clone()), expected: None });
                }
            }
        }
    } else {
        for (name, text) in scripts {
            // `X.prismtest` belongs to `X.prism` if it is among the files.
            let owner = std::path::Path::new(name).with_extension("prism").to_string_lossy().to_string();
            let module = module_of_file(&loaded, &owner);
            todo.push(Script { name: name.clone(), text: text.clone(), module, expected: expected_beside(name) });
        }
    }
    if todo.is_empty() {
        return Outcome::failed("prism test: no tests among the files, and no script\n".into(), EXIT_UNCHECKED);
    }

    // Every expression a script names, parsed now, as a module that resolves its names
    // in the script's module.
    let mut exprs: Vec<HashMap<String, (ModId, &'static Expr)>> = Vec::new();
    let mut errors = Vec::new();
    for s in &todo {
        let scope = s
            .module
            .as_ref()
            .and_then(|f| loaded.modules.iter().position(|m| &m.file == f))
            .or(loaded.root)
            .unwrap_or(loaded.modules.len() - 1);
        let mut table = HashMap::new();
        for text in script_expressions(&s.text) {
            if table.contains_key(&text) {
                continue;
            }
            match fragment(&mut loaded, scope, &s.name, &text) {
                Ok(pair) => {
                    table.insert(text, pair);
                }
                Err(e) => errors.push(e),
            }
        }
        exprs.push(table);
    }
    if !errors.is_empty() {
        return Outcome::failed(errors.join("\n") + "\n", EXIT_UNCHECKED);
    }
    let interp = Interp::new(loaded);

    let mut out = Outcome::default();
    for (s, table) in todo.iter().zip(exprs) {
        let table = Rc::new(table);
        let mut world = TestWorld::new();
        for header in test_headers(&s.text) {
            let Some((m, e)) = table.get(&format!("@{header}")).copied() else { continue };
            let mut sc = Scopes::new(m);
            match interp.eval(e, &mut sc, None) {
                Ok(Value::Widget(bp)) => world = world.program(Program::new(&header, bp)),
                Ok(other) => out.stderr.push_str(&format!("{}: @{header} is {}, not a blueprint\n", s.name, crate::ops::show_plain(&other))),
                Err(f) => out.stderr.push_str(&format!("{}\n", f.message)),
            }
        }
        let i2 = interp.clone();
        let t2 = table.clone();
        world = world.evaluator(Box::new(move |text: &str| {
            let (m, e) = t2.get(text).copied().ok_or_else(|| format!("no expression {text}"))?;
            let mut sc = Scopes::new(m);
            match i2.eval(e, &mut sc, None) {
                Ok(v) if v.is_plain() => Ok(v),
                Ok(_) => Err("a `$`, `!` or `@` value on a check line: a check has no ticks (Conformance ch. 1)".into()),
                Err(f) => Err(f.message),
            }
        }));
        // A type with its own `^Show` prints by it in the transcript, a path's key
        // included (Type System ch. 2): prism_rt asks the interpreter through its hook.
        let i3 = interp.clone();
        let hook: prism_rt::pod::ShowHook = Rc::new(move |v: &Value| {
            let ty = crate::ops::type_name(&i3.l, v);
            if !i3.has_impl("Show", &ty) {
                return None;
            }
            i3.show(v, None, Loc { module: i3.l.prelude, span: Default::default() }).ok()
        });
        let before = prism_rt::pod::set_show_hook(Some(hook));
        let report = world.run(&s.text);
        prism_rt::pod::set_show_hook(before);
        let transcript = report.transcript.clone();
        if !out.stdout.is_empty() {
            out.stdout.push('\n');
        }
        out.stdout.push_str(&transcript);
        for f in &report.failures {
            out.stderr.push_str(&format!("{}: {f}\n", s.name));
        }
        let mut code = report.exit_code;
        if let Some(expected) = &s.expected {
            let want: String = strip_comment_header(expected);
            if want != transcript {
                out.stderr.push_str(&format!("{}: the transcript differs from its .expected:\n{}", s.name, diff(&want, &transcript)));
                code = code.max(1);
            }
        }
        out.code = out.code.max(code);
    }
    out
}

/// An `.expected` file without its leading `# ` comment lines.
pub fn strip_comment_header(s: &str) -> String {
    let mut lines = s.split_inclusive('\n').peekable();
    while lines.peek().is_some_and(|l| l.starts_with("# ") || l.trim_end() == "#") {
        lines.next();
    }
    lines.collect()
}

/// A line-by-line difference.
pub fn diff(expected: &str, got: &str) -> String {
    let (e, g): (Vec<&str>, Vec<&str>) = (expected.lines().collect(), got.lines().collect());
    let mut out = String::new();
    for i in 0..e.len().max(g.len()) {
        match (e.get(i), g.get(i)) {
            (Some(a), Some(b)) if a == b => out.push_str(&format!("  {a}\n")),
            (a, b) => {
                if let Some(a) = a {
                    out.push_str(&format!("- {a}\n"));
                }
                if let Some(b) = b {
                    out.push_str(&format!("+ {b}\n"));
                }
            }
        }
    }
    out
}

/// Parse an expression from a script as a module of its own whose names resolve in
/// `scope`.
fn fragment(l: &mut Loaded, scope: ModId, script: &str, text: &str) -> Result<(ModId, &'static Expr), String> {
    let src: &'static str = Box::leak(text.to_string().into_boxed_str());
    let p = prism_syntax::parse_fragment(src);
    if !p.parsed_ok() {
        return Err(format!("{script}: `{text}`: {}", prism_syntax::diag::render_all(src, &p.diagnostics)));
    }
    let ast: &'static Module = Box::leak(Box::new(p.tree));
    let expr = match ast.decls.as_slice() {
        [Decl { kind: DeclKind::Stmt(Stmt { kind: StmtKind::Expr(e), .. }), .. }] => e,
        _ => return Err(format!("{script}: `{text}` is not one expression")),
    };
    let id = l.modules.len();
    l.modules.push(ModInfo {
        name: Some(format!("<{script}>")),
        file: script.to_string(),
        src,
        ast,
        bundled: true,
        decls: HashMap::new(),
        scope: HashMap::new(),
        qualified: Default::default(),
        row: prism_rt::Row::Any,
        out_of_scope: vec![],
        alias: Some(scope),
    });
    Ok((id, expr))
}

/// Strip a test-language comment as the Test World does: a `#` after whitespace,
/// outside a string.
fn strip_comment(line: &str) -> &str {
    let mut in_str = false;
    let mut prev = ' ';
    for (i, c) in line.char_indices() {
        match c {
            '"' if prev != '\\' => in_str = !in_str,
            '#' if !in_str && prev.is_whitespace() => return &line[..i],
            _ => {}
        }
        prev = c;
    }
    line
}

/// The blueprints a script's `test name, @w:` lines name, as the Test World names
/// programs: the text after the `@`.
fn test_headers(script: &str) -> Vec<String> {
    let mut out = Vec::new();
    for l in script.lines() {
        let l = strip_comment(l).trim_end();
        let Some(rest) = l.strip_prefix("test ") else { continue };
        let Some(rest) = rest.strip_suffix(':') else { continue };
        let parts = split_commas(rest);
        if let Some(p) = parts.get(1) {
            if let Some(name) = p.trim().strip_prefix('@') {
                out.push(name.trim().to_string());
            }
        }
    }
    out
}

/// Every expression a script asks the interpreter to evaluate: each test's blueprint,
/// with its `@`, and each `check` line's left side.
fn script_expressions(script: &str) -> Vec<String> {
    let mut out: Vec<String> = test_headers(script).into_iter().map(|h| format!("@{h}")).collect();
    let mut in_check = false;
    for l in script.lines() {
        let l = strip_comment(l).trim_end();
        if l.trim().is_empty() {
            continue;
        }
        if !l.starts_with(' ') {
            in_check = l.starts_with("check ");
            continue;
        }
        if in_check {
            let t = l.trim();
            let e = match t.split_once("=>") {
                Some((e, _)) => e.trim(),
                None => t,
            };
            out.push(e.to_string());
        }
    }
    out
}

fn split_commas(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut prev = ' ';
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '"' if prev != '\\' => in_str = !in_str,
            '(' | '[' | '{' if !in_str => depth += 1,
            ')' | ']' | '}' if !in_str => depth -= 1,
            ',' if !in_str && depth == 0 => {
                out.push(std::mem::take(&mut cur));
                prev = c;
                continue;
            }
            _ => {}
        }
        cur.push(c);
        prev = c;
    }
    out.push(cur);
    out
}
