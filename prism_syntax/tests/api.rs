//! The snippets of `API.md`, run.

use prism_syntax::ast::{DeclKind, StepAction};
use prism_syntax::token::{Layout, TokenKind};
use prism_syntax::{Stage, corpus, lex, parse_fragment, parse_module, parse_test_file, pretty, sexp};

const COUNTER: &str = "emit Ui.*

@counter: @unit
@counter:
    !inc <- Ui.Button(label=$\"Increment\")
    !dec <- Ui.Button(label=$\"Decrement\")
    $count <- 0 then $count + (-1 on !dec <| +1 on !inc)
    Ui.Label(text=$\"Count: {$count}\")

test counter_counts, @counter:
    build
        => expect Ui.Label(text=\"Count: 0\")
    click Ui.Button(label=\"Increment\"), click Ui.Button(label=\"Decrement\")
        => expect Ui.Label(text=\"Count: 0\")
    compute until idle

check sums:
    1 + 1 => 2
    (1 + 1) == 2
";

#[test]
fn lexing() {
    let lexed = lex(COUNTER);
    assert!(lexed.diagnostics.is_empty());
    assert!(matches!(lexed.tokens.last().unwrap().kind, TokenKind::Eof));
    assert!(lexed.tokens.iter().any(|t| t.kind == TokenKind::Layout(Layout::Open)));
    assert!(corpus::render(&lexed.tokens).starts_with("emit Ui.*\nsep @counter : @unit\n"));
}

#[test]
fn parsing_a_module() {
    let parsed = parse_module(COUNTER);
    assert!(parsed.clean(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.tree.decls.len(), 4);
    assert!(matches!(parsed.tree.decls[1].kind, DeclKind::Widget(_)));
    let printed = pretty::module(&parsed.tree);
    assert_eq!(parse_module(&printed).tree, parsed.tree);
    assert!(sexp::module(&parsed.tree).contains("(hold $count 0 then (+ $count (group (<| (on (neg 1) !dec) (on (pos 1) !inc)))))"));
}

#[test]
fn parsing_a_test_file() {
    let parsed = parse_test_file(COUNTER);
    assert!(parsed.clean());
    let t = &parsed.tree.tests[0];
    assert_eq!(t.name.name, "counter_counts");
    assert_eq!(t.steps.len(), 3);
    assert!(matches!(&t.steps[1].action, StepAction::Deliver(ds) if ds.len() == 2));
    assert!(matches!(t.steps[2].action, StepAction::Compute { until_idle: true }));
    let c = &parsed.tree.checks[0];
    assert!(c.lines[0].pattern.is_some() && c.lines[1].pattern.is_none());
}

#[test]
fn fragments_and_out_of_scope() {
    let f = parse_fragment("$count <- 0 then $count + 1 on !click\n");
    assert!(f.clean());
    assert!(matches!(f.tree.decls[0].kind, DeclKind::Stmt(_)));
    let p = parse_module("let r = 0.75\n");
    assert!(p.parsed_ok() && !p.clean());
    assert_eq!(p.diagnostics[0].stage, Stage::OutOfScope);
    assert_eq!(p.diagnostics[0].feature, Some("Float"));
}
