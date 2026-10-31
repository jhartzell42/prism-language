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
    let p = parse_module("Now: fn unit -> Instant\n");
    assert!(p.parsed_ok() && !p.clean());
    assert_eq!(p.diagnostics[0].stage, Stage::OutOfScope);
    assert_eq!(p.diagnostics[0].feature, Some("`Instant`"));
}

#[test]
fn a_field_access_shows_the_fields_stack() {
    // Type System ch. 1: "the marks between the dot and the field name".
    let p = parse_fragment("let p = g.?passed == ? to\nlet v = input.!changed.$value\n");
    assert!(p.clean(), "{:?}", p.diagnostics);
    let dump = sexp::module(&p.tree);
    assert!(dump.contains("(. g ?passed)"), "{dump}");
    assert!(dump.contains("(. (. input !changed) $value)"), "{dump}");
    match &p.tree.decls[0].kind {
        DeclKind::Let(l) => match &l.value.kind {
            prism_syntax::ast::ExprKind::Binary { lhs, .. } => {
                assert!(matches!(&lhs.kind, prism_syntax::ast::ExprKind::Field { sigils, name, .. } if sigils == &[prism_syntax::token::Sigil::Question] && name.name == "passed"));
            }
            k => panic!("{k:?}"),
        },
        k => panic!("{k:?}"),
    }
    assert_eq!(pretty::module(&p.tree), "let p = g.?passed == ? to\n\nlet v = input.!changed.$value\n");
}

#[test]
fn a_guards_commas_belong_to_its_call() {
    // decisions.md D15: `require Clear g, from, corner` is `require (Clear g, from, corner)`.
    let p = parse_module("fn F g, from, corner:\n    require Clear g, from, corner\n    forbid (Attacked g, from)\n    g\n");
    assert!(p.clean(), "{:?}", p.diagnostics);
    let dump = sexp::module(&p.tree);
    assert!(dump.contains("(require (call Clear g from corner))"), "{dump}");
    assert!(dump.contains("(forbid (group (call Attacked g from)))"), "{dump}");
}

#[test]
fn a_delivered_value_may_hold_a_comma_in_brackets() {
    // Pitch ch. 8's spelling and the transcript's, both in one test (Conformance A.3: a step
    // splits only at bracket depth zero).
    let src = "test press, @board:\n    send Gui.Window(..), Press (Point 280.0, 400.0)\n        => fired Press (Point 280.0, 400.0)\n    send Gui.Window(..), Press Point(x=280.0, y=400.0)\n";
    let p = parse_test_file(src);
    assert!(p.clean(), "{:?}", p.diagnostics);
    let dump = sexp::module(&p.tree.module);
    assert!(dump.contains("(send (record Gui.Window ..) (call Press (group (call Point 280.0 400.0))))"), "{dump}");
    assert!(dump.contains("(=> fired (ctor Press (group (ctor Point 280.0 400.0))))"), "{dump}");
    assert!(dump.contains("(send (record Gui.Window ..) (call Press (call() Point x=280.0 y=400.0)))"), "{dump}");
    assert!(matches!(&p.tree.tests[0].steps[0].action, StepAction::Deliver(ds) if ds.len() == 1));
    // A bare comma at depth zero is still the step's, and an error asking for parentheses.
    let bad = parse_test_file("test t, @b:\n    send p, Min a, b\n");
    assert!(!bad.parsed_ok());
}
