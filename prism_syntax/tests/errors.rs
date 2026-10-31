//! Error positions and messages: each case is a source, the line and column of the first
//! diagnostic, and a piece of its message (the book's spelling where the book gives one).

use prism_syntax::{LineIndex, Stage, parse_module};

fn first_error(src: &str) -> (u32, u32, String) {
    let p = parse_module(src);
    let d = p.diagnostics.iter().find(|d| d.stage != Stage::OutOfScope).unwrap_or_else(|| panic!("no error for:\n{src}"));
    let lc = LineIndex::new(src).line_col(d.span.start);
    (lc.line, lc.col, d.message.clone())
}

fn check(src: &str, line: u32, col: u32, needle: &str) {
    let (l, c, msg) = first_error(src);
    assert!(msg.contains(needle), "message `{msg}` lacks `{needle}` for:\n{src}");
    assert_eq!((l, c), (line, col), "position of `{msg}` for:\n{src}");
}

#[test]
fn tab_in_indentation() {
    check("@w:\n\t!a <- Ui.Button(label=$\"A\")\n", 2, 1, "a tab in leading whitespace is an error");
}

#[test]
fn unindent_matches_no_block() {
    // Semantics ch. 2, the third layout case.
    check("@bad:\n    !x <- for $rows:\n        Ui.Button(label=$_)\n      <| !other\n", 4, 1, "unindent does not match any open block");
}

#[test]
fn colon_without_block() {
    check("@w:\nUi.Label(text=$\"x\")\n", 1, 3, "expected an indented block after `:`");
}

#[test]
fn shift_is_not_an_operator() {
    check("let a = x << y\n", 1, 11, "`<<` opens explicit type arguments and must touch a name");
}

#[test]
fn bare_bar_names_the_merge() {
    check("let a = !x | !y\n", 1, 12, "the merge is `<|`");
}

#[test]
fn decimal_needs_digits_on_both_sides() {
    check("let a = 1.\n", 1, 9, "digits on both sides of its point");
    check("let a = .5\n", 1, 9, "write `0.5`");
}

#[test]
fn no_literal_suffix() {
    check("let b = 255U8\n", 1, 9, "a number literal has no suffix");
}

#[test]
fn duration_whole_nanoseconds() {
    check("let d = 1.0000000001s\n", 1, 9, "whole number of nanoseconds");
}

#[test]
fn bad_escape_and_unterminated_text() {
    check("let t = \"a\\qb\"\n", 1, 11, "is not an escape");
    check("let t = \"abc\nlet u = 1\n", 1, 9, "a text literal ends on its line");
}

#[test]
fn names_that_stray() {
    check("let _x = 1\n", 1, 5, "`_` followed by a letter is not a name");
    check("let a = Open_door\n", 1, 9, "a capital promises UpperCamelCase");
}

#[test]
fn never_free_sigils() {
    check("let a = ! x\n", 1, 9, "`!` is never free");
    check("let a = ^ Ord\n", 1, 9, "no trait is a value");
}

#[test]
fn prefix_sign_after_a_value() {
    // X5: "the error asks 'did you mean `a - 1`?'"
    check("let a = b -1\n", 1, 11, "did you mean `a - 1`");
}

#[test]
fn three_opens_with_a_comma() {
    // "`(((a, b)))` an error, a tuple holding a group with a comma and no call at its head".
    check("let a = (((a, b)))\n", 1, 13, "a group holds one expression");
}

#[test]
fn nothing_at_top_level_is_built() {
    check("$count <- 0 then $count + 1 on !click\n", 1, 1, "nothing at top level is built");
}

#[test]
fn ignore_arm() {
    check("fn F x = match x:\n    ignore: 0\n", 2, 5, "the catch-all is `otherwise:`");
}

#[test]
fn step_comma_needs_parentheses() {
    // Conformance A.3: "Any other comma at depth zero is an error that asks for parentheses."
    check("test t, @w:\n    send Ui.Field(..), Min a, b\n", 2, 29, "written in parentheses");
}

#[test]
fn template_never_closes() {
    check("fn F x = inline_text <rust>\nabc\n", 1, 22, "a template never closes: expected `</rust>`");
}

#[test]
fn template_under_indented_line() {
    check("fn F x = inline_text <rust>\n    a {{x}}\n  {{x}}\n    </rust>\n", 3, 3, "indented less than its closing tag");
}

#[test]
fn then_does_not_associate() {
    check("@w:\n    $x <- 0 then 1 then 2\n", 2, 20, "`then` does not associate");
}

#[test]
fn a_value_is_not_applied() {
    check("let a = f(x)\n", 1, 10, "is a value and is not applied");
}

#[test]
fn checked_minus_needs_a_left_operand() {
    // Doubt MQ: "a soft error naming `-(?x)`".
    check("let a = F -?x\n", 1, 11, "`-(?x)`");
}

#[test]
fn spaced_method_on_one_line() {
    // Rule 9: "On one line a spaced `.Method` is an error asking for its own line."
    check("let a = xs .Map F\n", 1, 12, "begins its own line");
}

#[test]
fn recovery_reports_each_statement_and_parses_the_rest() {
    let src = "@w:\n    !a <- )\n    !b <- ]\n    Ui.Label(text=$\"ok\")\n\nlet fine = 1\n";
    let p = parse_module(src);
    let errs: Vec<_> = p.diagnostics.iter().filter(|d| d.stage != Stage::OutOfScope).collect();
    let index = LineIndex::new(src);
    let lines: Vec<u32> = errs.iter().map(|d| index.line_col(d.span.start).line).collect();
    assert!(lines.contains(&2) && lines.contains(&3), "errors on both lines: {lines:?}");
    assert_eq!(p.tree.decls.len(), 2, "the widget and the let both survive");
    match &p.tree.decls[0].kind {
        prism_syntax::ast::DeclKind::Widget(w) => assert_eq!(w.body.stmts.len(), 1, "the label survives"),
        k => panic!("{k:?}"),
    }
}

#[test]
fn out_of_scope_features_are_named() {
    let cases: &[(&str, &str)] = &[
        ("fn Both pa, pb = Parser monad:\n    a <- pa\n    return a\n", "`T monad:` blocks and `^Monad`"),
        ("let r = 0.75\n", "Float"),
        ("Size: fn U32 -> Nat\n", "sized ints other than `U8`"),
        ("let t = ((n, -))\n", "sparse tuples"),
        ("let x = !Align !a, !b\n", "`!Align`"),
        ("let xs = [[1]]+\n", "non-empty arrays"),
        ("let y = a >Int b\n", "suffixed operators"),
        ("&Home: fn &Fixture -> &Nat\n", "named lenses `&Home` and `Over`"),
        ("let d = @Collect x\n", "POD hosting (`@Collect`, `@View`, `@Edit`, `@Translate`)"),
        ("@w:\n    output <markdown>\n# hi\n</markdown>\n", "the template notation"),
        ("type E a = choice:\n    Lit: Int -> E Int\n", "indexed choices/GADTs for user types"),
        ("trait ^Functor f:\n    Map: fn f a, (fn a -> b) -> f b\n", "higher-kinded traits"),
        ("Now: fn unit -> Instant\n", "`Instant`"),
    ];
    for (src, feature) in cases {
        let p = parse_module(src);
        assert!(p.parsed_ok(), "{src}: {:?}", p.diagnostics);
        assert!(p.diagnostics.iter().any(|d| d.feature == Some(feature)), "{src}: wanted `{feature}`, got {:?}", p.diagnostics);
    }
}
