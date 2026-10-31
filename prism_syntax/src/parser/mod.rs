//! The parser: tokens to the tree of [`crate::ast`].
//!
//! Recursive descent, with a Pratt loop over the precedence table of Semantics ch. 2 /
//! Appendix A.5 for expressions (`expr.rs`), and separate descents for types
//! (`types.rs`), patterns (`patterns.rs`), declarations (`decls.rs`) and the test
//! language (`tests.rs`).
//!
//! Two rules of chapter 2 are the parser's rather than the lexer's:
//!
//! - **The pairing rule.** "The parser matches brackets one at a time by kind. Outermost
//!   first, it looks at each matched pair. If the open is followed at once by a second
//!   open of the same kind, and that second open's close comes at once before the first
//!   one's close, the two pairs are one double bracket, and pairing continues inside the
//!   inner one." [`pair_brackets`] does this once per token stream, before parsing.
//! - **Statement position.** `<=` sets at the head of a statement and compares elsewhere,
//!   which only the parser knows.
//!
//! Errors are recorded where they are found, and the parser recovers at statement
//! boundaries: on an error it skips to the next `sep` or `close` of the block it is in
//! (layout makes every statement boundary a token). Speculative parses (is this line a
//! bind, `pattern <- e`?) roll back their diagnostics with their position.

mod decls;
mod expr;
mod patterns;
mod tests;
mod types;

use crate::ast::*;
use crate::diag::Diagnostic;
use crate::lexer::lex;
use crate::span::Span;
use crate::token::*;

/// What a parse produced: the tree, and every diagnostic in source order. A diagnostic
/// with [`crate::Stage::OutOfScope`] means the tree is fine and the feature is out of this
/// compiler.
#[derive(Clone, Debug)]
pub struct Parsed<T> {
    pub tree: T,
    pub diagnostics: Vec<Diagnostic>,
}

impl<T> Parsed<T> {
    /// No lexical or parse errors (out-of-scope diagnostics allowed).
    pub fn parsed_ok(&self) -> bool {
        !self.diagnostics.iter().any(|d| d.stage != crate::Stage::OutOfScope)
    }
    /// No diagnostics at all.
    pub fn clean(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Parses a file as a module (Semantics ch. 6: one file is one module).
pub fn parse_module(src: &str) -> Parsed<Module> {
    let lexed = lex(src);
    let mut p = Parser::new(lexed.tokens);
    let module = p.module(false);
    finish(lexed.diagnostics, p, module)
}

/// Parses a run of statements and declarations at column 0, as the books' snippets are
/// written (a widget block's lines with no widget around them). Declarations parse as
/// declarations; anything else becomes [`DeclKind::Stmt`].
pub fn parse_fragment(src: &str) -> Parsed<Module> {
    let lexed = lex(src);
    let mut p = Parser::new(lexed.tokens);
    let module = p.module(true);
    finish(lexed.diagnostics, p, module)
}

/// A file read for its tests: the module, and its `test` and `check` declarations in
/// order. Tests are declarations of their module (Semantics ch. 6, "`test` and `check` are
/// declarations"), so a test file is a module like any other.
#[derive(Clone, Debug)]
pub struct TestFile {
    pub module: Module,
    pub tests: Vec<TestDecl>,
    pub checks: Vec<CheckDecl>,
}

pub fn parse_test_file(src: &str) -> Parsed<TestFile> {
    let parsed = parse_module(src);
    let mut tests = Vec::new();
    let mut checks = Vec::new();
    for d in &parsed.tree.decls {
        match &d.kind {
            DeclKind::Test(t) => tests.push(t.clone()),
            DeclKind::Check(c) => checks.push(c.clone()),
            _ => {}
        }
    }
    Parsed { tree: TestFile { module: parsed.tree, tests, checks }, diagnostics: parsed.diagnostics }
}

fn finish<T>(mut lex_diags: Vec<Diagnostic>, p: Parser, tree: T) -> Parsed<T> {
    lex_diags.extend(p.diags);
    lex_diags.extend(p.out);
    lex_diags.sort_by_key(|d| (d.span.start, d.stage as u8));
    // An error found twice (a speculative parse that was kept, a lexer and a parser both
    // seeing one mistake) is reported once.
    lex_diags.dedup_by(|a, b| a.span.same_range(b.span) && a.message == b.message);
    Parsed { tree, diagnostics: lex_diags }
}

/// A parse step failed; its diagnostic is already recorded.
pub(crate) type PResult<T> = Result<T, ()>;

/// Where a paren-less call may take commas (Semantics ch. 3, "Calls, commas and the time
/// words": "A paren-less call takes the rest of its expression, commas and all, and a
/// paren-less call inside an argument takes no commas, so commas belong to the outermost
/// call").
#[derive(Clone, Copy, Debug)]
pub(crate) struct Ctx {
    pub commas: bool,
}

impl Ctx {
    pub const TOP: Ctx = Ctx { commas: true };
    pub const ARG: Ctx = Ctx { commas: false };
}

/// The pairing of one bracket token.
#[derive(Clone, Copy, Debug, Default)]
struct Pair {
    partner: Option<usize>,
    /// The outer open (or outer close) of a double bracket.
    double_outer: bool,
    /// The inner open (or inner close) of a double bracket.
    double_inner: bool,
}

pub(crate) struct Parser {
    toks: Vec<Token>,
    pairs: Vec<Pair>,
    pos: usize,
    pub(crate) diags: Vec<Diagnostic>,
    /// Out-of-scope diagnostics, kept apart so a rolled-back speculation drops its own.
    pub(crate) out: Vec<Diagnostic>,
    /// Inside a test step: no comma belongs to a paren-less call (Conformance A.3).
    pub(crate) step_mode: bool,
    /// Parsing a trait's or impl's items (operators may name signatures there).
    pub(crate) in_trait: bool,
}

/// Matches brackets by kind and finds the doubles, outermost first.
fn pair_brackets(toks: &[Token]) -> Vec<Pair> {
    let mut pairs = vec![Pair::default(); toks.len()];
    let mut stack: Vec<(usize, BracketKind)> = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        match t.kind {
            TokenKind::Open { kind, .. } => stack.push((i, kind)),
            TokenKind::Close { kind } => {
                if let Some(&(j, k)) = stack.last() {
                    if k == kind {
                        stack.pop();
                        pairs[i].partner = Some(j);
                        pairs[j].partner = Some(i);
                    }
                }
            }
            _ => {}
        }
    }
    // Scanning left to right meets the outermost open of a nest first, so the double goes
    // to the outer two of `(((` ("Outermost first gives the double to the outer two").
    let mut i = 0;
    while i < toks.len() {
        if let TokenKind::Open { kind, .. } = toks[i].kind {
            if !pairs[i].double_inner && kind != BracketKind::Brace {
                if let (Some(j), Some(TokenKind::Open { kind: k2, .. })) = (pairs[i].partner, toks.get(i + 1).map(|t| &t.kind)) {
                    let inner_close = pairs[i + 1].partner;
                    if *k2 == kind && toks[i + 1].touches_prev && inner_close == Some(j - 1) && toks[j].touches_prev {
                        pairs[i].double_outer = true;
                        pairs[j].double_outer = true;
                        pairs[i + 1].double_inner = true;
                        pairs[j - 1].double_inner = true;
                    }
                }
            }
        }
        i += 1;
    }
    pairs
}

impl Parser {
    pub(crate) fn new(mut toks: Vec<Token>) -> Parser {
        if !matches!(toks.last().map(|t| &t.kind), Some(TokenKind::Eof)) {
            let end = toks.last().map(|t| t.span.at_end()).unwrap_or_default();
            toks.push(Token { kind: TokenKind::Eof, span: end, touches_prev: false, line_start: false });
        }
        let pairs = pair_brackets(&toks);
        Parser { toks, pairs, pos: 0, diags: Vec::new(), out: Vec::new(), step_mode: false, in_trait: false }
    }

    /// A parser over a nested token stream (a placeholder, a brace block), sharing this
    /// parser's modes. Its diagnostics are merged back by [`Parser::absorb`].
    pub(crate) fn sub(&self, toks: Vec<Token>, end: Span) -> Parser {
        let mut toks = toks;
        toks.push(Token { kind: TokenKind::Eof, span: end, touches_prev: false, line_start: false });
        let mut p = Parser::new(toks);
        p.step_mode = self.step_mode;
        p
    }

    pub(crate) fn absorb(&mut self, sub: Parser) {
        self.diags.extend(sub.diags);
        self.out.extend(sub.out);
    }

    // ----- the cursor -----------------------------------------------------------------------

    pub(crate) fn peek(&self) -> &Token {
        &self.toks[self.pos.min(self.toks.len() - 1)]
    }

    pub(crate) fn peek_at(&self, n: usize) -> &Token {
        &self.toks[(self.pos + n).min(self.toks.len() - 1)]
    }

    pub(crate) fn kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    pub(crate) fn kind_at(&self, n: usize) -> &TokenKind {
        &self.peek_at(n).kind
    }

    pub(crate) fn span(&self) -> Span {
        self.peek().span
    }

    /// The span of the last token consumed.
    pub(crate) fn prev_span(&self) -> Span {
        if self.pos == 0 { self.toks[0].span } else { self.toks[self.pos - 1].span }
    }

    pub(crate) fn advance(&mut self) -> Token {
        let t = self.peek().clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }

    pub(crate) fn at_eof(&self) -> bool {
        matches!(self.kind(), TokenKind::Eof)
    }

    pub(crate) fn at_kw(&self, kw: Kw) -> bool {
        matches!(self.kind(), TokenKind::Kw(k, s) if *k == kw && s.is_empty())
    }

    pub(crate) fn eat_kw(&mut self, kw: Kw) -> bool {
        if self.at_kw(kw) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(crate) fn at_op(&self, op: Op) -> bool {
        matches!(self.kind(), TokenKind::Op { op: o, .. } if *o == op)
    }

    pub(crate) fn eat_op(&mut self, op: Op) -> bool {
        if self.at_op(op) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(crate) fn at_layout(&self, l: Layout) -> bool {
        matches!(self.kind(), TokenKind::Layout(x) if *x == l)
    }

    pub(crate) fn at_comma(&self) -> bool {
        matches!(self.kind(), TokenKind::Comma)
    }

    pub(crate) fn at_colon(&self) -> bool {
        matches!(self.kind(), TokenKind::Colon)
    }

    /// The pairing of the bracket at the cursor.
    fn pair(&self) -> Pair {
        self.pairs[self.pos.min(self.pairs.len() - 1)]
    }

    /// The cursor is at the outer open of a double bracket of this kind.
    pub(crate) fn at_double_open(&self, kind: BracketKind) -> bool {
        matches!(self.kind(), TokenKind::Open { kind: k, .. } if *k == kind) && self.pair().double_outer
    }

    /// The cursor is at a single (not doubled) open bracket of this kind.
    pub(crate) fn at_single_open(&self, kind: BracketKind) -> bool {
        matches!(self.kind(), TokenKind::Open { kind: k, .. } if *k == kind) && !self.pair().double_outer
    }

    pub(crate) fn at_close(&self, kind: BracketKind) -> bool {
        matches!(self.kind(), TokenKind::Close { kind: k } if *k == kind)
    }

    /// Consumes the two opens of a double bracket.
    pub(crate) fn open_double(&mut self) {
        self.advance();
        self.advance();
    }

    /// Expects the two closes of a double bracket.
    pub(crate) fn close_double(&mut self, kind: BracketKind) -> PResult<()> {
        let c = kind.close_char();
        if self.at_close(kind) && self.pair().double_inner {
            self.advance();
            if self.at_close(kind) && self.pair().double_outer {
                self.advance();
                return Ok(());
            }
        }
        self.error_here(format!("expected `{c}{c}` to close the double bracket"))
    }

    pub(crate) fn expect_close(&mut self, kind: BracketKind) -> PResult<()> {
        if self.at_close(kind) {
            self.advance();
            Ok(())
        } else {
            let c = kind.close_char();
            self.error_here(format!("expected `{c}`"))
        }
    }

    // ----- diagnostics ----------------------------------------------------------------------

    pub(crate) fn error_at<T>(&mut self, span: Span, msg: impl Into<String>) -> PResult<T> {
        self.diags.push(Diagnostic::parse(span, msg));
        Err(())
    }

    pub(crate) fn error_here<T>(&mut self, msg: impl Into<String>) -> PResult<T> {
        let span = self.span();
        let found = describe(self.kind());
        self.error_at(span, format!("{}, found {}", msg.into(), found))
    }

    /// Records an error without failing.
    pub(crate) fn note_error(&mut self, span: Span, msg: impl Into<String>) {
        self.diags.push(Diagnostic::parse(span, msg));
    }

    /// Records an out-of-scope feature (decisions.md, "Features in and out").
    pub(crate) fn out_of_scope(&mut self, span: Span, feature: &'static str, msg: impl Into<String>) {
        self.out.push(Diagnostic::out_of_scope(span, feature, msg));
    }

    pub(crate) fn expect_colon(&mut self) -> PResult<Span> {
        if self.at_colon() {
            Ok(self.advance().span)
        } else {
            self.error_here("expected `:`")
        }
    }

    pub(crate) fn expect_op(&mut self, op: Op) -> PResult<Span> {
        if self.at_op(op) {
            Ok(self.advance().span)
        } else {
            self.error_here(format!("expected `{}`", op.as_str()))
        }
    }

    pub(crate) fn expect_kw(&mut self, kw: Kw) -> PResult<Span> {
        if self.at_kw(kw) {
            Ok(self.advance().span)
        } else {
            self.error_here(format!("expected `{}`", kw.as_str()))
        }
    }

    /// Runs `f`; on failure restores the cursor and drops what `f` reported.
    pub(crate) fn speculate<T>(&mut self, f: impl FnOnce(&mut Parser) -> PResult<T>) -> Option<T> {
        let pos = self.pos;
        let nd = self.diags.len();
        let no = self.out.len();
        match f(self) {
            Ok(v) => Some(v),
            Err(()) => {
                self.pos = pos;
                self.diags.truncate(nd);
                self.out.truncate(no);
                None
            }
        }
    }

    /// Recovery: skips to the `sep` or `close` that ends the current statement, past any
    /// blocks the statement opened.
    pub(crate) fn recover_to_stmt_end(&mut self) {
        let mut depth = 0usize;
        loop {
            match self.kind() {
                TokenKind::Eof => return,
                TokenKind::Layout(Layout::Open) => depth += 1,
                TokenKind::Layout(Layout::Close) => {
                    if depth == 0 {
                        return;
                    }
                    depth -= 1;
                }
                TokenKind::Layout(Layout::Sep) if depth == 0 => return,
                _ => {}
            }
            self.advance();
        }
    }

    /// The statement or declaration ends here: the next token is `sep`, `close` or the end.
    pub(crate) fn at_stmt_boundary(&self) -> bool {
        matches!(self.kind(), TokenKind::Layout(Layout::Sep | Layout::Close) | TokenKind::Eof)
    }

    /// After a statement: expects its boundary, and recovers if something else follows.
    pub(crate) fn expect_stmt_end(&mut self, what: &str) {
        if !self.at_stmt_boundary() {
            let span = self.span();
            let found = describe(self.kind());
            self.note_error(span, format!("expected the end of the {what}, found {found}"));
            self.recover_to_stmt_end();
        }
    }
}

/// How a token is named in an error message.
pub(crate) fn describe(k: &TokenKind) -> String {
    match k {
        TokenKind::Layout(Layout::Open) => "an indented block".into(),
        TokenKind::Layout(Layout::Sep) => "the end of the line".into(),
        TokenKind::Layout(Layout::Close) => "the end of the block".into(),
        TokenKind::Eof => "the end of the input".into(),
        TokenKind::Text { .. } => "a text literal".into(),
        TokenKind::Template(_) => "a template".into(),
        other => format!("`{}`", crate::corpus::token_text(other)),
    }
}
