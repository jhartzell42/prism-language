//! The lexer: characters to tokens, with layout.
//!
//! Semantics chapter 2 is the specification, and every decision below cites the rule it
//! implements. Its constraint X3 says the lexer's whole state is "a stack of block
//! columns, a stack of open brackets and template frames, and whether the last character
//! was whitespace". Here:
//!
//! - the **stack of block columns** is [`Stream::layout`];
//! - the **stack of open brackets and template frames** is [`Stream::frames`] for the
//!   brackets of one stream, plus the Rust call stack for the frames that open a new
//!   stream (a text placeholder, a template, a template's placeholder or brace block, a
//!   test path's key): each of those calls [`Lexer::lex_stream`] recursively and returns
//!   its tokens as one token's contents. A recursion is a push, a return a pop;
//! - **whether the last character was whitespace** is [`Stream::last_ws`].
//!
//! Three more bits are named where the books add them: [`Stream::lambda_open`] ("the bit
//! only tracks whether a parameter list is open", ch. 2, "The merge, and the bar");
//! [`Stream::pending_open`], the `:` that ended the previous line (layout rule 2), which
//! is a fact about the previous line rather than state; and [`TestMode`], the frame a
//! `test` or `check` block puts around its lines (Conformance Appendix A.1, A.2).

use crate::diag::Diagnostic;
use crate::span::Span;
use crate::token::*;

/// Tokens and the diagnostics met on the way.
#[derive(Clone, Debug)]
pub struct LexOutput {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Lexes a whole file: a token stream ending in `Eof`, with the layout tokens in it.
pub fn lex(src: &str) -> LexOutput {
    let mut lx = Lexer { src, pos: 0, diags: Vec::new() };
    let tokens = lx.lex_file();
    LexOutput { tokens, diagnostics: lx.diags }
}

/// How a stream ends, and whether layout is on inside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// The file: layout on, ends at end of file.
    File,
    /// A template's brace block: layout on ("a brace block ... pushes a Prism frame with
    /// layout resumed, ending at the closing delimiter alone on a line").
    BraceBlock(Delims),
    /// A placeholder, in a text literal (`Single`) or a template: layout suspended,
    /// ending at the matching `}` or `}}` once the stream's own brackets are closed.
    Placeholder(Delims),
    /// The key of a test path segment, `[expr]`: ends at the matching `]`.
    PathKey,
}

impl Mode {
    fn layout_on(self) -> bool {
        matches!(self, Mode::File | Mode::BraceBlock(_))
    }
}

/// A bracket opened inside one stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Frame {
    Bracket(BracketKind),
    /// `<<` ... `>>` (layout rule 6 lists them among the brackets).
    TypeArgs,
}

/// The test language's frame (Conformance Appendix A).
#[derive(Clone, Copy, Debug, Default)]
struct TestMode {
    /// A `test` or `check` keyword began the current top-level statement; its block's
    /// `open` starts the mode.
    pending: Option<Kw>,
    /// The layout depth of the open `test` block's steps.
    test_depth: Option<usize>,
    /// The layout depth of the open `check` block's lines.
    check_depth: Option<usize>,
    /// The next token is a path: the last two tokens were a delivery word and `at`.
    path_next: bool,
}

/// One stream's state.
struct Stream {
    mode: Mode,
    tokens: Vec<Token>,
    /// The block columns (layout rule 1: "at first `[0]`").
    layout: Vec<u32>,
    frames: Vec<Frame>,
    /// Whether the character before the next token is whitespace (or the start of a line).
    last_ws: bool,
    lambda_open: bool,
    /// The previous line ended in a layout colon or a closing lambda bar.
    pending_open: bool,
    /// The span of the colon that set `pending_open`, for its error.
    pending_span: Span,
    /// Index in `tokens` of the first token of the current physical line.
    line_first: usize,
    /// The next token is the first of a continuation line.
    mark_line_start: bool,
    /// No line has been seen yet.
    first_line: bool,
    test: TestMode,
}

impl Stream {
    fn new(mode: Mode) -> Stream {
        Stream {
            mode,
            tokens: Vec::new(),
            layout: Vec::new(),
            frames: Vec::new(),
            last_ws: true,
            lambda_open: false,
            pending_open: false,
            pending_span: Span::default(),
            line_first: 0,
            mark_line_start: false,
            first_line: true,
            test: TestMode::default(),
        }
    }

    fn layout_active(&self) -> bool {
        self.mode.layout_on() && self.frames.is_empty()
    }

    fn fat_arrow_on(&self) -> bool {
        self.test.test_depth.is_some() || self.test.check_depth.is_some()
    }

    /// The spacing rule's left half (ch. 2, "The spacing rule"): "whitespace, an open
    /// bracket, a comma, an operator or the start of a line comes before it".
    fn prefix_position(&self) -> bool {
        if self.last_ws {
            return true;
        }
        match self.tokens.last() {
            None => true,
            Some(t) => !ends_operand(&t.kind),
        }
    }
}

/// Whether a token ends an operand, so that a touching `+`, `-`, `.` or `[` after it is
/// binary.
fn ends_operand(k: &TokenKind) -> bool {
    match k {
        TokenKind::Name(_)
        | TokenKind::FamilyStar(_)
        | TokenKind::Int { .. }
        | TokenKind::Float { .. }
        | TokenKind::Duration { .. }
        | TokenKind::Char { .. }
        | TokenKind::Text { .. }
        | TokenKind::Template(_)
        | TokenKind::Close { .. }
        | TokenKind::GtGt
        | TokenKind::NonEmpty
        | TokenKind::Absent => true,
        TokenKind::Kw(kw, _) => matches!(kw, Kw::Unit | Kw::Nil | Kw::Key | Kw::Ix | Kw::Jx | Kw::Kx),
        _ => false,
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The two lowercase constructors the prelude reserves (ch. 2, "Case is a mark").
pub fn is_reserved_ctor(w: &str) -> bool {
    w == "templatetext" || w == "templateplaceholder"
}

const DURATION_UNITS: [(&str, u128); 6] =
    [("ns", 1), ("us", 1_000), ("ms", 1_000_000), ("min", 60_000_000_000), ("s", 1_000_000_000), ("h", 3_600_000_000_000)];

struct Lexer<'s> {
    src: &'s str,
    pos: usize,
    diags: Vec<Diagnostic>,
}

impl<'s> Lexer<'s> {
    // ----- characters -----------------------------------------------------------------

    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.src[self.pos..].chars().nth(n)
    }

    fn starts_with(&self, s: &str) -> bool {
        self.src[self.pos..].starts_with(s)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn err(&mut self, start: usize, end: usize, msg: impl Into<String>) {
        self.diags.push(Diagnostic::lex(Span::new(start, end.max(start)), msg));
    }

    /// The rest of the current line from `pos`, without the newline.
    fn rest_of_line(&self) -> &'s str {
        let rest = &self.src[self.pos..];
        let end = rest.find('\n').unwrap_or(rest.len());
        &rest[..end]
    }

    // ----- streams ----------------------------------------------------------------------

    fn lex_file(&mut self) -> Vec<Token> {
        let mut prefix = Vec::new();
        // A.7: "a `<` touching a lowercase name touching `>` ... as a file's first token,
        // which makes the file a template in that language".
        let save = self.pos;
        self.skip_blank_and_comment_lines();
        if self.peek() == Some('<') && self.template_tag_ahead().is_some() {
            let start = self.pos;
            if let Some(t) = self.lex_template(TemplateKw::File, start) {
                prefix.push(Token { kind: TokenKind::Template(Box::new(t)), span: Span::new(start, self.pos), touches_prev: false, line_start: false });
            }
        } else {
            self.pos = save;
        }
        let mut toks = self.lex_stream(Mode::File);
        if !prefix.is_empty() {
            prefix.append(&mut toks);
            return prefix;
        }
        toks
    }

    fn skip_blank_and_comment_lines(&mut self) {
        loop {
            let line = self.rest_of_line();
            let t = line.trim_start_matches([' ', '\r']);
            if (t.is_empty() || t.starts_with('#')) && self.pos + line.len() < self.src.len() {
                self.pos += line.len() + 1;
            } else {
                return;
            }
        }
    }

    /// Lexes one stream to its end, per its mode, and returns its tokens.
    fn lex_stream(&mut self, mode: Mode) -> Vec<Token> {
        let mut st = Stream::new(mode);
        if mode.layout_on() {
            st.layout.clear();
        }
        let mut at_line_start = mode.layout_on();
        loop {
            // Layout happens at the start of each physical line, outside brackets.
            if at_line_start && st.layout_active() {
                at_line_start = false;
                match self.begin_line(&mut st) {
                    LineStart::Tokens => {}
                    LineStart::Skip => {
                        at_line_start = true;
                        continue;
                    }
                    LineStart::End => break,
                }
            }
            let Some(c) = self.peek() else {
                if !matches!(mode, Mode::File) {
                    let what = match mode {
                        Mode::BraceBlock(_) => "a brace block never closes: expected its closing delimiter alone on a line",
                        Mode::Placeholder(Delims::Single) => "a placeholder never closes: expected `}`",
                        Mode::Placeholder(Delims::Double) => "a placeholder never closes: expected `}}`",
                        _ => "a path key never closes: expected `]`",
                    };
                    self.err(self.pos, self.pos, what);
                }
                break;
            };
            match c {
                '\n' => {
                    self.bump();
                    st.last_ws = true;
                    if st.lambda_open {
                        self.err(self.pos - 1, self.pos, "a lambda's parameter list closes with `|` on its own line");
                        st.lambda_open = false;
                    }
                    if st.layout_active() {
                        self.end_line(&mut st);
                        at_line_start = true;
                    }
                }
                ' ' | '\r' => {
                    self.bump();
                    st.last_ws = true;
                }
                '\t' => {
                    // A.1: a tab in leading whitespace is an error; elsewhere it is
                    // whitespace, though no rule needs one.
                    self.bump();
                    st.last_ws = true;
                }
                '#' => {
                    // A comment runs from `#` to the end of the line (ch. 2, "Source and
                    // comments"), and is whitespace for every purpose.
                    let line = self.rest_of_line();
                    self.pos += line.len();
                    st.last_ws = true;
                }
                _ => {
                    if self.stream_ends_here(&mut st) {
                        break;
                    }
                    self.lex_token(&mut st);
                }
            }
        }
        if mode.layout_on() {
            // Rule 10: "End of file closes every block."
            while st.layout.len() > 1 {
                st.layout.pop();
                self.push(&mut st, TokenKind::Layout(Layout::Close), self.pos, self.pos);
            }
        }
        if matches!(mode, Mode::File) {
            self.push(&mut st, TokenKind::Eof, self.pos, self.pos);
        }
        st.tokens
    }

    /// Whether the character at `pos` ends a nested stream; consumes the closer if so.
    fn stream_ends_here(&mut self, st: &mut Stream) -> bool {
        if !st.frames.is_empty() {
            return false;
        }
        match st.mode {
            Mode::Placeholder(Delims::Single) if self.peek() == Some('}') => {
                self.bump();
                true
            }
            Mode::Placeholder(Delims::Double) if self.starts_with("}}") => {
                self.pos += 2;
                true
            }
            Mode::PathKey if self.peek() == Some(']') => {
                self.bump();
                true
            }
            _ => false,
        }
    }

    /// Rule 2 and rule 3 look at the last token of the line that just ended.
    fn end_line(&mut self, st: &mut Stream) {
        if st.tokens.len() > st.line_first {
            let last = st.tokens.last().unwrap();
            match last.kind {
                // Rule 2: "A `:` that is the last token on its line, comments aside,
                // opens a layout block."
                TokenKind::Colon => {
                    st.pending_open = true;
                    st.pending_span = last.span;
                }
                // Rule 3: "A closing lambda bar that is the last token on its line opens
                // a layout block the same way."
                TokenKind::LambdaBar { open: false } => {
                    st.pending_open = true;
                    st.pending_span = last.span;
                }
                _ => {}
            }
        }
    }

    /// Measures a line's indentation and emits its layout tokens (rules 1 to 4 and 10;
    /// Conformance A.1 inside a `test` block).
    fn begin_line(&mut self, st: &mut Stream) -> LineStart {
        let line_begin = self.pos;
        let mut col = 0u32;
        loop {
            match self.peek() {
                Some(' ') => {
                    self.bump();
                    col += 1;
                }
                Some('\t') => {
                    let p = self.pos;
                    self.bump();
                    col += 1;
                    self.err(p, p + 1, "a tab in leading whitespace is an error: indent with spaces");
                }
                Some('\r') => {
                    self.bump();
                }
                _ => break,
            }
        }
        // Rule 10: "Blank and comment-only lines never open, close or continue anything."
        match self.peek() {
            None => return LineStart::End,
            Some('\n') => {
                self.bump();
                return LineStart::Skip;
            }
            Some('#') => {
                let line = self.rest_of_line();
                self.pos += line.len();
                if self.peek() == Some('\n') {
                    self.bump();
                    return LineStart::Skip;
                }
                return LineStart::End;
            }
            _ => {}
        }
        // A brace block ends "at the closing delimiter alone on a line".
        if let Mode::BraceBlock(d) = st.mode {
            let close = if d == Delims::Double { "}}" } else { "}" };
            let rest = self.rest_of_line();
            if rest.trim_end_matches([' ', '\r']) == close {
                self.pos += rest.len();
                if self.peek() == Some('\n') {
                    self.bump();
                }
                return LineStart::End;
            }
        }
        st.line_first = st.tokens.len();
        st.last_ws = true;
        let here = self.pos;
        if st.first_line {
            st.first_line = false;
            match st.mode {
                Mode::File => {
                    st.layout.push(0);
                    if col != 0 {
                        self.err(line_begin, here, "the first line of a file starts at column 0");
                        st.layout[0] = col;
                    }
                }
                // "A brace block in a template resumes layout, its first statement fixing
                // its column."
                _ => st.layout.push(col),
            }
            return LineStart::Tokens;
        }
        let top = *st.layout.last().unwrap();
        let starts_fat_arrow = self.starts_with("=>");
        if st.pending_open {
            st.pending_open = false;
            if col > top {
                st.layout.push(col);
                self.push(st, TokenKind::Layout(Layout::Open), here, here);
                if let Some(kw) = st.test.pending.take() {
                    match kw {
                        Kw::Test => st.test.test_depth = Some(st.layout.len()),
                        _ => st.test.check_depth = Some(st.layout.len()),
                    }
                }
                return LineStart::Tokens;
            }
            let sp = st.pending_span;
            self.diags.push(Diagnostic::lex(sp, "expected an indented block after `:`"));
        }
        // Conformance A.1 rule 1: "A more-indented line that begins with `=>` opens the
        // step's checks", with no `:` written.
        if col > top && starts_fat_arrow && st.test.test_depth == Some(st.layout.len()) {
            st.layout.push(col);
            self.push(st, TokenKind::Layout(Layout::Open), here, here);
            return LineStart::Tokens;
        }
        if col == top {
            // Rule 4: "A line at the block's column begins a statement, after a `sep`."
            self.push(st, TokenKind::Layout(Layout::Sep), here, here);
            if st.layout.len() == 1 {
                st.test.pending = None;
            }
        } else if col > top {
            // "A line indented more continues the statement and emits nothing."
            st.mark_line_start = true;
        } else {
            // "A line indented less emits `close`, pops, and compares again."
            while st.layout.len() > 1 && col < *st.layout.last().unwrap() {
                st.layout.pop();
                self.push(st, TokenKind::Layout(Layout::Close), here, here);
                let depth = st.layout.len();
                if st.test.test_depth.is_some_and(|d| depth < d) {
                    st.test.test_depth = None;
                }
                if st.test.check_depth.is_some_and(|d| depth < d) {
                    st.test.check_depth = None;
                }
            }
            let top = *st.layout.last().unwrap();
            if col != top {
                // "after the pops its column must equal the top of the stack, or the error
                // is 'unindent does not match any open block'".
                self.err(line_begin, here, "unindent does not match any open block");
                if col < top {
                    // Only a brace block can be under its first column; treat as the base.
                }
            }
            self.push(st, TokenKind::Layout(Layout::Sep), here, here);
            if st.layout.len() == 1 {
                st.test.pending = None;
            }
        }
        LineStart::Tokens
    }

    fn push(&mut self, st: &mut Stream, kind: TokenKind, start: usize, end: usize) {
        let touches_prev = !st.last_ws && !st.tokens.is_empty() && !kind_is_layout(&kind);
        let line_start = if kind_is_layout(&kind) {
            false
        } else {
            std::mem::take(&mut st.mark_line_start)
        };
        // The test language's frame: `test` or `check` beginning a top-level statement.
        if let TokenKind::Kw(kw @ (Kw::Test | Kw::Check), _) = kind {
            if st.mode == Mode::File && st.layout.len() == 1 && st.tokens[st.line_first..].iter().all(|t| t.is_layout()) {
                st.test.pending = Some(kw);
            }
        }
        st.tokens.push(Token { kind, span: Span::new(start, end), touches_prev, line_start });
        st.last_ws = false;
    }

    // ----- tokens -----------------------------------------------------------------------

    fn lex_token(&mut self, st: &mut Stream) {
        let start = self.pos;
        let c = self.peek().unwrap();

        // Conformance A.2: a path after `at`, read by the test lexer.
        if st.test.path_next {
            st.test.path_next = false;
            if c.is_ascii_digit() || c == '#' || c == '[' {
                let segs = self.lex_path();
                self.push(st, TokenKind::Path(segs), start, self.pos);
                return;
            }
        }

        match c {
            '"' => {
                let text = self.lex_text();
                self.push(st, TokenKind::Text { sigils: vec![], text }, start, self.pos);
            }
            '\'' => {
                let ch = self.lex_char();
                self.push(st, TokenKind::Char { sigils: vec![], ch }, start, self.pos);
            }
            '0'..='9' => {
                let kind = self.lex_number(vec![]);
                self.push(st, kind, start, self.pos);
            }
            c if is_ident_start(c) => {
                self.lex_word_token(st, vec![], start);
            }
            '$' | '@' | '?' | '~' | '&' | '^' => self.lex_sigils(st),
            '!' => {
                if self.peek_at(1) == Some('=') {
                    self.pos += 2;
                    self.push_op(st, Op::NotEq, start);
                } else {
                    self.lex_sigils(st);
                }
            }
            '(' | '[' | '{' => {
                let kind = match c {
                    '(' => BracketKind::Paren,
                    '[' => BracketKind::Square,
                    _ => BracketKind::Brace,
                };
                // The spacing rule applies to a single `[`; the pairing rule decides
                // doubles in the parser.
                let prefix = kind == BracketKind::Square && st.prefix_position() && !self.next_is_space(1);
                self.bump();
                st.frames.push(Frame::Bracket(kind));
                self.push(st, TokenKind::Open { kind, prefix }, start, self.pos);
            }
            ')' | ']' | '}' => {
                let kind = match c {
                    ')' => BracketKind::Paren,
                    ']' => BracketKind::Square,
                    _ => BracketKind::Brace,
                };
                self.bump();
                match st.frames.last() {
                    Some(Frame::Bracket(k)) if *k == kind => {
                        st.frames.pop();
                    }
                    Some(Frame::Bracket(k)) => {
                        let k = *k;
                        self.err(start, self.pos, format!("`{}` closes a `{}`", c, k.open_char()));
                        st.frames.pop();
                    }
                    Some(Frame::TypeArgs) => {
                        self.err(start, self.pos, format!("`{c}` inside `<<`: expected `>>`"));
                    }
                    None => {
                        self.err(start, self.pos, format!("`{c}` closes nothing"));
                    }
                }
                self.push(st, TokenKind::Close { kind }, start, self.pos);
            }
            ',' => {
                self.bump();
                self.push(st, TokenKind::Comma, start, self.pos);
            }
            ':' => {
                self.bump();
                self.push(st, TokenKind::Colon, start, self.pos);
            }
            ';' => {
                self.bump();
                self.push(st, TokenKind::Semi, start, self.pos);
            }
            '\\' => {
                self.bump();
                self.push(st, TokenKind::Backslash, start, self.pos);
                // "A `|` immediately after `\` opens a parameter list."
                if self.peek() == Some('|') {
                    let s = self.pos;
                    self.bump();
                    st.lambda_open = true;
                    self.push(st, TokenKind::LambdaBar { open: true }, s, self.pos);
                }
            }
            '|' => {
                self.bump();
                if st.lambda_open {
                    // "... which closes at the next `|` on the same line".
                    st.lambda_open = false;
                    self.push(st, TokenKind::LambdaBar { open: false }, start, self.pos);
                } else {
                    // A row separator. "A `|` anywhere else is an error that names `<|`",
                    // which the parser reports, since only it knows a type from a value.
                    self.push(st, TokenKind::Bar, start, self.pos);
                }
            }
            '.' => {
                if self.starts_with("..") {
                    self.pos += 2;
                    self.push(st, TokenKind::DotDot, start, self.pos);
                } else {
                    let prefix = st.prefix_position() && !self.next_is_space(1);
                    if prefix && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
                        self.err(start, start + 1, "a decimal literal has digits on both sides of its point: write `0.5`, not `.5`");
                    }
                    self.bump();
                    self.push(st, TokenKind::Dot { prefix }, start, self.pos);
                }
            }
            _ => {
                if !self.lex_operator(st) {
                    self.bump();
                    self.err(start, self.pos, format!("unexpected character `{c}`"));
                    st.last_ws = true;
                }
            }
        }
    }

    fn next_is_space(&self, n: usize) -> bool {
        match self.peek_at(n) {
            None => true,
            Some(c) => c == ' ' || c == '\n' || c == '\r' || c == '\t',
        }
    }

    fn push_op(&mut self, st: &mut Stream, op: Op, start: usize) {
        // Suffixed operators (ch. 2): "a symbol operator followed, with no space, by an
        // uppercase name is one token".
        if op.takes_suffix() && self.peek().is_some_and(|c| c.is_ascii_uppercase()) {
            let path = self.lex_qualified_path();
            self.push(st, TokenKind::SuffixedOp { op, ty: path }, start, self.pos);
            return;
        }
        self.push(st, TokenKind::Op { op, prefix: false }, start, self.pos);
    }

    /// Longest match over the operator table (ch. 2, "Operators and punctuation").
    fn lex_operator(&mut self, st: &mut Stream) -> bool {
        let start = self.pos;
        let c = self.peek().unwrap();
        // The two marks that a sign makes against a closing bracket (ch. 2, "The spacing
        // rule"): "A `-` with a comma or a close bracket directly after it is an absent
        // component", and "A `+` touching a closing double bracket or brace, with
        // whitespace, a comma, a close bracket or the end of the line after it, is the
        // non-empty mark". The lexer cannot know which brackets double, so any `]` or `}`
        // counts; `)` never does, since a tuple has no non-empty mark.
        if c == '-' && matches!(self.peek_at(1), Some(',' | ')' | ']' | '}')) {
            self.bump();
            self.push(st, TokenKind::Absent, start, self.pos);
            return true;
        }
        if c == '+' && !st.last_ws {
            if let Some(TokenKind::Close { kind: BracketKind::Square | BracketKind::Brace }) = st.tokens.last().map(|t| &t.kind) {
                let after = self.peek_at(1);
                let after2 = self.peek_at(2);
                let ends = |a: Option<char>| matches!(a, None | Some(' ' | '\n' | '\r' | ',' | ')' | ']' | '}'));
                if ends(after) || (after == Some('?') && ends(after2)) {
                    self.bump();
                    self.push(st, TokenKind::NonEmpty, start, self.pos);
                    if after == Some('?') {
                        // `as [Text]+?`: the non-empty mark, then the checked cast's `?`
                        // (doubt PQ).
                        let s = self.pos;
                        self.bump();
                        self.push(st, TokenKind::FreeSigil(Sigil::Question), s, self.pos);
                    }
                    return true;
                }
            }
        }
        let fat = st.fat_arrow_on();
        let table: &[(&str, Op)] = &[
            ("==>", Op::MapArrow),
            ("<-", Op::BindArrow),
            ("<=", Op::Le),
            ("<|", Op::Merge),
            ("->", Op::Arrow),
            ("-=", Op::MinusEq),
            ("-?", Op::SubQ),
            ("+=", Op::PlusEq),
            ("+?", Op::AddQ),
            ("*=", Op::StarEq),
            ("*?", Op::MulQ),
            ("/=", Op::SlashEq),
            ("==", Op::EqEq),
            (">=", Op::Ge),
            ("=>", Op::FatArrow),
            ("<", Op::Lt),
            (">", Op::Gt),
            ("-", Op::Sub),
            ("+", Op::Add),
            ("*", Op::Mul),
            ("/", Op::Div),
            ("=", Op::Eq),
        ];
        if self.starts_with("<<") {
            // "`<<` touching a name on its left opens explicit type arguments ... Anywhere
            // else, `<<` and `>>` are errors."
            self.pos += 2;
            let touching_name = !st.last_ws && matches!(st.tokens.last().map(|t| &t.kind), Some(TokenKind::Name(_)));
            if !touching_name {
                self.err(start, self.pos, "`<<` opens explicit type arguments and must touch a name on its left (Prism has no shift operators)");
            }
            st.frames.push(Frame::TypeArgs);
            self.push(st, TokenKind::LtLt, start, self.pos);
            return true;
        }
        if self.starts_with(">>") && st.frames.last() == Some(&Frame::TypeArgs) {
            self.pos += 2;
            st.frames.pop();
            self.push(st, TokenKind::GtGt, start, self.pos);
            return true;
        }
        if self.starts_with(">>") {
            self.pos += 2;
            self.err(start, self.pos, "`>>` closes explicit type arguments, and none is open (Prism has no shift operators)");
            self.push(st, TokenKind::GtGt, start, self.pos);
            return true;
        }
        for (s, op) in table {
            if *op == Op::FatArrow && !fat {
                continue;
            }
            if self.starts_with(s) {
                self.pos += s.len();
                if matches!(op, Op::Add | Op::Sub | Op::SubQ) && !self.peek().is_some_and(|c| c.is_ascii_uppercase()) {
                    // "a token that can be prefix or binary is prefix when whitespace, an
                    // open bracket, a comma, an operator or the start of a line comes
                    // before it and a non-whitespace character comes after it."
                    let prefix = st.prefix_position() && !self.next_is_space(0);
                    self.push(st, TokenKind::Op { op: *op, prefix }, start, self.pos);
                } else {
                    self.push_op(st, *op, start);
                }
                return true;
            }
        }
        false
    }

    /// A run of sigils and what follows it (ch. 2, "The attachment rule"): attached
    /// "when the character after it is a letter, a digit, `_`, `"`, `.` (for `&` only), or
    /// another sigil. Otherwise it is free."
    fn lex_sigils(&mut self, st: &mut Stream) {
        let start = self.pos;
        let mut sigils = Vec::new();
        let mut caret = false;
        while let Some(c) = self.peek() {
            if c == '^' {
                self.bump();
                caret = true;
                break;
            }
            if c == '!' && self.peek_at(1) == Some('=') {
                break;
            }
            match Sigil::from_char(c) {
                Some(s) => {
                    self.bump();
                    sigils.push(s);
                }
                None => break,
            }
        }
        let next = self.peek();
        if caret {
            // `^` is a mark that attaches by the same rule, to a trait name.
            if next.is_some_and(|c| c.is_ascii_uppercase()) && sigils.is_empty() {
                let path = self.lex_qualified_path();
                self.push(st, TokenKind::Trait(path), start, self.pos);
            } else {
                let w_start = self.pos;
                if next.is_some_and(is_ident_start) {
                    let _ = self.lex_qualified_path();
                }
                self.err(start, self.pos.max(w_start), "`^` marks a trait and attaches to its uppercase name: no trait is a value");
            }
            return;
        }
        let attached = match next {
            Some(c) => is_ident_start(c) || c.is_ascii_digit() || c == '"',
            None => false,
        };
        if attached {
            let c = next.unwrap();
            if c == '"' {
                let text = self.lex_text();
                self.push(st, TokenKind::Text { sigils, text }, start, self.pos);
            } else if c.is_ascii_digit() {
                let kind = self.lex_number(sigils);
                self.push(st, kind, start, self.pos);
            } else {
                self.lex_word_token(st, sigils, start);
            }
            return;
        }
        if next == Some('.') && sigils == [Sigil::Amp] && self.peek_at(1).is_some_and(|c| c.is_ascii_lowercase()) {
            // "`&` attaches to `.` as well as to a name, because a lens path written as a
            // value starts at no name". The token is `&.`; the first field follows it.
            self.bump();
            self.push(st, TokenKind::LensDot, start, self.pos);
            return;
        }
        // Free. A run of several sigils before a bracket is several free sigils, each the
        // prefix over what follows (`@!(Text ==> ?Nat)` in a type).
        let space_after = self.next_is_space(0);
        for (i, s) in sigils.iter().enumerate() {
            let s_start = start + i;
            if space_after && i == sigils.len() - 1 {
                match s {
                    Sigil::Bang => self.err(s_start, s_start + 1, "`!` is never free: no event fires a constant (write `!e` touching its name)"),
                    Sigil::Amp => self.err(s_start, s_start + 1, "`&` is never free: no reference exists outside the value an `update` block names"),
                    _ => {}
                }
            }
            self.push(st, TokenKind::FreeSigil(*s), s_start, s_start + 1);
        }
    }

    /// `Ui.Button`, `Rust.templatetext`: uppercase segments joined by `.` with nothing
    /// between, the last possibly one of the reserved lowercase constructors.
    fn lex_qualified_path(&mut self) -> Vec<String> {
        let mut path = vec![self.lex_ident()];
        loop {
            if self.peek() != Some('.') {
                break;
            }
            let first_upper = path.last().unwrap().starts_with(|c: char| c.is_ascii_uppercase());
            if !first_upper {
                break;
            }
            match self.peek_at(1) {
                Some(c) if c.is_ascii_uppercase() => {
                    self.bump();
                    path.push(self.lex_ident());
                }
                Some(c) if is_ident_start(c) => {
                    let save = self.pos;
                    self.bump();
                    let w = self.lex_ident();
                    if is_reserved_ctor(&w) {
                        path.push(w);
                        break;
                    }
                    self.pos = save;
                    break;
                }
                _ => break,
            }
        }
        path
    }

    fn lex_ident(&mut self) -> String {
        let s = self.pos;
        while self.peek().is_some_and(is_ident_char) {
            self.bump();
        }
        self.src[s..self.pos].to_string()
    }

    /// A word: a keyword, a name, a qualified name, a family pattern `Ui.*`, the topic or a
    /// hole; with `sigils` already read.
    fn lex_word_token(&mut self, st: &mut Stream, sigils: Vec<Sigil>, start: usize) {
        let w_start = self.pos;
        let path = self.lex_qualified_path();
        // A non-ASCII letter glued to a name: "Names are ASCII letters, digits and
        // underscores."
        if self.peek().is_some_and(|c| c.is_alphabetic() && !c.is_ascii()) {
            while self.peek().is_some_and(|c| c.is_alphanumeric() || c == '_') {
                self.bump();
            }
            self.err(w_start, self.pos, "names are ASCII letters, digits and underscores");
        }
        // `Ui.*` is one token, "and the only place `*` touches a `.`".
        if self.starts_with(".*") && path[0].starts_with(|c: char| c.is_ascii_uppercase()) {
            self.pos += 2;
            if !sigils.is_empty() {
                self.err(start, self.pos, "a family pattern takes no sigil");
            }
            self.push(st, TokenKind::FamilyStar(path), start, self.pos);
            return;
        }
        if path.len() == 1 {
            let w = &path[0];
            let bound_name = matches!(w.as_str(), "key" | "ix" | "jx" | "kx");
            if let Some(kw) = Kw::from_word(w).filter(|_| !(bound_name && !sigils.is_empty())) {
                if !sigils.is_empty() && !matches!(kw, Kw::Widget | Kw::Unit | Kw::Nil) {
                    self.err(start, self.pos, format!("`{}` is a keyword and takes no sigil", w));
                }
                if matches!(kw, Kw::Widget) && sigils != [Sigil::At] {
                    self.err(start, self.pos, "`widget` is a block only as `@widget:`");
                }
                self.push(st, TokenKind::Kw(kw, sigils), start, self.pos);
                if matches!(kw, Kw::Output | Kw::InlineText | Kw::Raw) {
                    self.maybe_template(st, kw);
                }
                return;
            }
        }
        let last = path.last().unwrap().clone();
        let case = if last == "_" {
            NameCase::Topic
        } else if last.starts_with('_') && last[1..].chars().all(|c| c.is_ascii_digit()) {
            NameCase::Hole
        } else if last.starts_with('_') {
            self.err(w_start, self.pos, "`_` followed by a letter is not a name: `_` alone is the topic, `_1` a hole");
            NameCase::Lower
        } else if is_reserved_ctor(&last) {
            NameCase::ReservedCtor
        } else if last.starts_with(|c: char| c.is_ascii_uppercase()) {
            if path.iter().any(|seg| seg.contains('_')) {
                self.err(w_start, self.pos, format!("`{}`: a capital promises UpperCamelCase", path.join(".")));
            }
            NameCase::Upper
        } else {
            NameCase::Lower
        };
        // Conformance A.2: "A path after `at` or `click element` is read by the test
        // lexer": `at` after a delivery word, `element` after `click`.
        if st.test.test_depth.is_some() && sigils.is_empty() && path.len() == 1 && (path[0] == "at" || path[0] == "element") {
            let words: &[&str] = if path[0] == "at" { &["send", "click", "answer"] } else { &["click"] };
            let prev_is_delivery = matches!(
                st.tokens.last().map(|t| &t.kind),
                Some(TokenKind::Name(n)) if n.sigils.is_empty() && n.path.len() == 1 && words.contains(&n.path[0].as_str())
            );
            if prev_is_delivery {
                st.test.path_next = true;
            }
        }
        self.push(st, TokenKind::Name(NameTok { sigils, path, case }), start, self.pos);
    }

    // ----- literals ---------------------------------------------------------------------

    /// Numbers and durations (ch. 2, Literals: Numbers, Durations).
    fn lex_number(&mut self, sigils: Vec<Sigil>) -> TokenKind {
        let start = self.pos;
        // Radix prefixes: 0x, 0o, 0b.
        if self.peek() == Some('0') && matches!(self.peek_at(1), Some('x' | 'o' | 'b')) {
            let radix = self.peek_at(1).unwrap();
            self.pos += 2;
            let ok = |c: char| match radix {
                'x' => c.is_ascii_hexdigit(),
                'o' => ('0'..='7').contains(&c),
                _ => c == '0' || c == '1',
            };
            let ds = self.pos;
            while self.peek().is_some_and(|c| ok(c) || c == '_') {
                self.bump();
            }
            if ds == self.pos {
                self.err(start, self.pos, "a radix prefix needs digits after it");
            }
            self.check_no_suffix(start);
            return TokenKind::Int { sigils, num: NumTok { text: self.src[start..self.pos].to_string() } };
        }
        while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '_') {
            self.bump();
        }
        let mut decimal = false;
        if self.peek() == Some('.') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
            while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '_') {
                self.bump();
            }
            decimal = true;
        } else if self.peek() == Some('.') && !matches!(self.peek_at(1), Some('.')) && self.peek_at(1).is_some_and(|c| !is_ident_start(c)) {
            // `1.` is an error: "A decimal-spelled literal has digits on both sides of its
            // one `.`". A `.` before a name is field access, which the parser refuses on
            // a literal; `1..` is a range.
            self.bump();
            self.err(start, self.pos, format!("`{}` is not a number: a decimal literal has digits on both sides of its point", &self.src[start..self.pos]));
            return TokenKind::Float { sigils, num: NumTok { text: self.src[start..self.pos - 1].to_string() } };
        }
        // A duration: "a digit string, with at most one point, followed with nothing
        // between by one of `ns`, `us`, `ms`, `s`, `min` and `h`".
        for (unit, scale) in DURATION_UNITS {
            if self.starts_with(unit) && !self.src[self.pos + unit.len()..].starts_with(is_ident_char) {
                let num_text = self.src[start..self.pos].to_string();
                self.pos += unit.len();
                if !whole_nanoseconds(&num_text, scale) {
                    self.err(start, self.pos, format!("`{}{}` does not name a whole number of nanoseconds", num_text, unit));
                }
                return TokenKind::Duration { sigils, num: NumTok { text: num_text }, unit: unit.to_string() };
            }
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            let save = self.pos;
            self.bump();
            if matches!(self.peek(), Some('+' | '-')) {
                self.bump();
            }
            if self.peek().is_some_and(|c| c.is_ascii_digit()) {
                while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '_') {
                    self.bump();
                }
                decimal = true;
            } else {
                self.pos = save;
            }
        }
        self.check_no_suffix(start);
        let num = NumTok { text: self.src[start..self.pos].to_string() };
        if decimal { TokenKind::Float { sigils, num } } else { TokenKind::Int { sigils, num } }
    }

    /// "A literal has no type suffix; `255U8` is not a token" (round 47 BI; doubt LS).
    fn check_no_suffix(&mut self, start: usize) {
        if self.peek().is_some_and(is_ident_char) {
            while self.peek().is_some_and(is_ident_char) {
                self.bump();
            }
            self.err(start, self.pos, format!("`{}` is not a token: a number literal has no suffix (write a type annotation)", &self.src[start..self.pos]));
        }
    }

    /// Decodes one escape after `\` (ch. 2, Text: "The escapes are `\\`, `\"`, `\n`,
    /// `\t`, `\{`, `\}` and `\u{hex}`; any other `\` is an error").
    fn lex_escape(&mut self, out: &mut String) {
        let start = self.pos;
        self.bump(); // the backslash
        match self.bump() {
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('\'') => out.push('\''),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('{') => out.push('{'),
            Some('}') => out.push('}'),
            Some('u') => {
                if self.peek() != Some('{') {
                    self.err(start, self.pos, "`\\u` is written `\\u{hex}`");
                    return;
                }
                self.bump();
                let hs = self.pos;
                while self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
                    self.bump();
                }
                let hex = &self.src[hs..self.pos];
                if self.peek() != Some('}') {
                    self.err(start, self.pos, "`\\u{` needs its closing `}`");
                    return;
                }
                self.bump();
                match u32::from_str_radix(hex, 16).ok().and_then(char::from_u32) {
                    Some(c) => out.push(c),
                    None => self.err(start, self.pos, format!("`\\u{{{hex}}}` is not a code point")),
                }
            }
            Some(c) => self.err(start, self.pos, format!("`\\{c}` is not an escape: the escapes are \\\\ \\\" \\n \\t \\{{ \\}} \\u{{hex}}")),
            None => self.err(start, self.pos, "a text literal ends inside an escape"),
        }
    }

    /// `"..."` on one line, with placeholders read as token streams.
    fn lex_text(&mut self) -> TextTok {
        let start = self.pos;
        self.bump(); // the quote
        let mut segments = Vec::new();
        let mut cur = String::new();
        loop {
            match self.peek() {
                None | Some('\n') => {
                    self.err(start, self.pos, "a text literal ends on its line: close it with `\"`, or write a multiline text as `raw <markdown>...</markdown>`");
                    break;
                }
                Some('"') => {
                    self.bump();
                    break;
                }
                Some('\\') => self.lex_escape(&mut cur),
                Some('{') => {
                    if !cur.is_empty() {
                        segments.push(TextSegTok::Plain(std::mem::take(&mut cur)));
                    }
                    let ps = self.pos;
                    self.bump();
                    // "A placeholder is read as ordinary tokens, nested literals and
                    // brackets included, until the matching brace, with layout suspended."
                    let toks = self.lex_stream(Mode::Placeholder(Delims::Single));
                    segments.push(TextSegTok::Placeholder(toks, Span::new(ps, self.pos)));
                }
                Some('}') => {
                    let s = self.pos;
                    self.bump();
                    self.err(s, self.pos, "a `}` in text is written `\\}`");
                    cur.push('}');
                }
                Some(c) => {
                    self.bump();
                    cur.push(c);
                }
            }
        }
        if !cur.is_empty() {
            segments.push(TextSegTok::Plain(cur));
        }
        TextTok { segments }
    }

    /// `'c'`, one code point.
    fn lex_char(&mut self) -> char {
        let start = self.pos;
        self.bump();
        let mut s = String::new();
        match self.peek() {
            Some('\\') => self.lex_escape(&mut s),
            Some('\'') | Some('\n') | None => {}
            Some(c) => {
                self.bump();
                s.push(c);
            }
        }
        if self.peek() == Some('\'') {
            self.bump();
        } else {
            while self.peek().is_some_and(|c| c != '\'' && c != '\n') {
                self.bump();
            }
            if self.peek() == Some('\'') {
                self.bump();
            }
            self.err(start, self.pos, "a character literal holds one code point: `'c'`");
        }
        if s.chars().count() != 1 {
            if s.is_empty() {
                self.err(start, self.pos, "a character literal holds one code point: `'c'`");
            }
            return s.chars().next().unwrap_or('\0');
        }
        s.chars().next().unwrap()
    }

    // ----- the test language's path -----------------------------------------------------

    /// `1/#0/[k]` (Conformance A.2): "a segment is a number, `#` and a number, or `[` a
    /// Prism expression `]`; segments are joined by `/`, with no spaces".
    fn lex_path(&mut self) -> Vec<PathSegTok> {
        let mut segs = Vec::new();
        loop {
            let s = self.pos;
            match self.peek() {
                Some('#') => {
                    self.bump();
                    let ds = self.pos;
                    while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                        self.bump();
                    }
                    if ds == self.pos {
                        self.err(s, self.pos, "a generation segment is `#` and a number");
                    }
                    segs.push(PathSegTok::Gen(self.src[ds..self.pos].parse().unwrap_or(0)));
                }
                Some(c) if c.is_ascii_digit() => {
                    while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                        self.bump();
                    }
                    segs.push(PathSegTok::Num(self.src[s..self.pos].parse().unwrap_or(0)));
                }
                Some('[') => {
                    self.bump();
                    let toks = self.lex_stream(Mode::PathKey);
                    segs.push(PathSegTok::Key(toks));
                }
                _ => {
                    self.err(s, s, "expected a path segment: a number, `#` and a number, or `[key]`");
                    break;
                }
            }
            if self.peek() == Some('/') {
                self.bump();
            } else {
                break;
            }
        }
        segs
    }

    // ----- templates --------------------------------------------------------------------

    /// After `output`, `inline_text` or `raw`, a language tag begins a template
    /// ("The language tag is a token form").
    fn maybe_template(&mut self, st: &mut Stream, kw: Kw) {
        let save = self.pos;
        while self.peek() == Some(' ') {
            self.bump();
        }
        if self.peek() == Some('<') && self.template_tag_ahead().is_some() {
            let start = self.pos;
            let tkw = match kw {
                Kw::Output => TemplateKw::Output,
                Kw::InlineText => TemplateKw::InlineText,
                _ => TemplateKw::Raw,
            };
            if let Some(t) = self.lex_template(tkw, start) {
                st.last_ws = true;
                self.push(st, TokenKind::Template(Box::new(t)), start, self.pos);
            }
        } else {
            self.pos = save;
            self.err(save, save, format!("`{}` is followed by a language tag, `<rust>`", kw.as_str()));
        }
    }

    /// `<` touching a lowercase name touching `>`: the name, if one is here.
    fn template_tag_ahead(&self) -> Option<String> {
        let rest = &self.src[self.pos..];
        let rest = rest.strip_prefix('<')?;
        let end = rest.find(|c: char| !is_ident_char(c)).unwrap_or(rest.len());
        let name = &rest[..end];
        if name.is_empty() || !name.starts_with(|c: char| c.is_ascii_lowercase()) {
            return None;
        }
        if rest[end..].starts_with('>') { Some(name.to_string()) } else { None }
    }

    fn lex_template(&mut self, keyword: TemplateKw, start: usize) -> Option<TemplateTok> {
        let lang = self.template_tag_ahead()?;
        self.pos += lang.len() + 2;
        let delims = Delims::for_language(&lang);
        let raw = keyword == TemplateKw::Raw;
        // "When the language tag ends its line, the newline after it is not text."
        let tag_ends_line = matches!(self.peek(), Some('\n')) || self.starts_with("\r\n");
        if tag_ends_line {
            if self.peek() == Some('\r') {
                self.bump();
            }
            self.bump();
        }
        let close = format!("</{lang}>");
        let parts = self.lex_template_body(&close, delims, raw, !tag_ends_line, start);
        Some(TemplateTok { keyword, lang, delims, parts })
    }

    /// Reads a template's text up to `close` and applies Templates ch. 2's whitespace
    /// rules. `first_continues` is true when the first line of text shares the opening
    /// tag's line.
    fn lex_template_body(&mut self, close: &str, delims: Delims, raw: bool, first_continues: bool, open_start: usize) -> Vec<TemplatePartTok> {
        let open = if delims == Delims::Double { "{{" } else { "{" };
        let mut lines: Vec<RawLine> = Vec::new();
        let mut cur = RawLine::default();
        let mut at_line_start = !first_continues;
        let mut closing_col: Option<u32> = None;
        let mut closed = false;
        loop {
            if at_line_start && !raw {
                // A brace block: the opener alone on its line.
                let rest = self.rest_of_line();
                if rest.trim_matches([' ', '\r']) == open {
                    let bs = self.pos;
                    self.pos += rest.len();
                    if self.peek() == Some('\n') {
                        self.bump();
                    }
                    let toks = self.lex_stream(Mode::BraceBlock(delims));
                    lines.push(RawLine { items: vec![RawItem::Block(toks, Span::new(bs, self.pos))], newline: false, block: true });
                    continue;
                }
            }
            at_line_start = false;
            if self.starts_with(close) {
                // "The frame ends at the first matching closing tag in text."
                if cur.only_whitespace() && (!lines.is_empty() || !first_continues) {
                    // The closing tag begins its line: its column is stripped from every
                    // line, and the newline before it is text.
                    closing_col = Some(cur.text_len() as u32);
                    cur.items.clear();
                }
                self.pos += close.len();
                closed = true;
                break;
            }
            let Some(c) = self.peek() else { break };
            if c == '\n' {
                self.bump();
                cur.newline = true;
                lines.push(std::mem::take(&mut cur));
                at_line_start = true;
                continue;
            }
            if !raw {
                if delims == Delims::Double && self.starts_with("\\{{") {
                    // "`\{{` writes `{{`. Every other backslash is text."
                    self.pos += 3;
                    cur.push_text("{{");
                    continue;
                }
                if delims == Delims::Single && self.starts_with("\\{") {
                    self.pos += 2;
                    cur.push_text("{");
                    continue;
                }
                if self.starts_with(open) {
                    let ps = self.pos;
                    self.pos += open.len();
                    let toks = self.lex_stream(Mode::Placeholder(delims));
                    cur.items.push(RawItem::Placeholder(toks, Span::new(ps, self.pos)));
                    continue;
                }
                if self.starts_with("<@") && self.peek_at(2).is_some_and(|c| c.is_ascii_uppercase()) {
                    let call = self.lex_template_call(delims);
                    cur.items.push(call);
                    continue;
                }
            }
            self.bump();
            cur.push_char(c);
        }
        if !closed {
            self.err(open_start, self.pos, format!("a template never closes: expected `{close}`"));
        }
        lines.push(cur);
        finish_template_lines(self, lines, closing_col.unwrap_or(0), first_continues)
    }

    /// `<@Name attr={{v}} attr="t"/>` or `<@Name ...>body</@Name>` (Templates ch. 2,
    /// "`<@` is the only call form").
    fn lex_template_call(&mut self, delims: Delims) -> RawItem {
        let start = self.pos;
        self.pos += 1; // `<`
        let mut sigils = Vec::new();
        while let Some(s) = self.peek().and_then(Sigil::from_char) {
            self.bump();
            sigils.push(s);
        }
        let path = self.lex_qualified_path();
        let case = NameCase::Upper;
        let name = NameTok { sigils, path, case };
        let mut attrs = Vec::new();
        let open = if delims == Delims::Double { "{{" } else { "{" };
        let body;
        loop {
            while matches!(self.peek(), Some(' ' | '\n' | '\r')) {
                self.bump();
            }
            if self.starts_with("/>") {
                self.pos += 2;
                body = None;
                break;
            }
            if self.peek() == Some('>') {
                self.bump();
                let close = format!("</{}>", name.text());
                let first_continues = self.peek() != Some('\n');
                if !first_continues {
                    self.bump();
                }
                body = Some(self.lex_template_body(&close, delims, false, first_continues, start));
                break;
            }
            let as_ = self.pos;
            if !self.peek().is_some_and(is_ident_start) {
                self.err(as_, as_ + 1, "expected an attribute `name=value`, `/>` or `>` in a template call");
                body = None;
                break;
            }
            let aname = self.lex_ident();
            if self.peek() != Some('=') {
                self.err(as_, self.pos, "an attribute is `name=value`");
                continue;
            }
            self.bump();
            let tokens = if self.starts_with(open) {
                self.pos += open.len();
                self.lex_stream(Mode::Placeholder(delims))
            } else if self.peek() == Some('"') {
                let ts = self.pos;
                let text = self.lex_text();
                vec![Token { kind: TokenKind::Text { sigils: vec![], text }, span: Span::new(ts, self.pos), touches_prev: false, line_start: false }]
            } else {
                self.err(self.pos, self.pos, format!("an attribute's value is a placeholder `{open}...` or a text literal"));
                Vec::new()
            };
            attrs.push(TemplateAttrTok { name: aname, tokens, span: Span::new(as_, self.pos) });
        }
        RawItem::Call(name, attrs, body, Span::new(start, self.pos))
    }
}

enum LineStart {
    Tokens,
    Skip,
    End,
}

fn kind_is_layout(k: &TokenKind) -> bool {
    matches!(k, TokenKind::Layout(_) | TokenKind::Eof)
}

/// Whether `digits[.frac]` times `scale` nanoseconds is whole.
fn whole_nanoseconds(text: &str, scale: u128) -> bool {
    let t: String = text.chars().filter(|c| *c != '_').collect();
    match t.split_once('.') {
        None => true,
        Some((_, frac)) => {
            let frac = frac.trim_end_matches('0');
            let k = frac.len() as u32;
            if k > 30 {
                return false;
            }
            let f: u128 = if frac.is_empty() { 0 } else { frac.parse().unwrap_or(1) };
            (f * scale) % 10u128.pow(k) == 0
        }
    }
}

#[derive(Default)]
struct RawLine {
    items: Vec<RawItem>,
    newline: bool,
    block: bool,
}

enum RawItem {
    Text(String),
    Placeholder(Vec<Token>, Span),
    Block(Vec<Token>, Span),
    Call(NameTok, Vec<TemplateAttrTok>, Option<Vec<TemplatePartTok>>, Span),
}

impl RawLine {
    fn push_char(&mut self, c: char) {
        if let Some(RawItem::Text(s)) = self.items.last_mut() {
            s.push(c);
        } else {
            self.items.push(RawItem::Text(c.to_string()));
        }
    }
    fn push_text(&mut self, t: &str) {
        if let Some(RawItem::Text(s)) = self.items.last_mut() {
            s.push_str(t);
        } else {
            self.items.push(RawItem::Text(t.to_string()));
        }
    }
    fn only_whitespace(&self) -> bool {
        self.items.iter().all(|i| matches!(i, RawItem::Text(s) if s.chars().all(|c| c == ' ' || c == '\r')))
    }
    fn text_len(&self) -> usize {
        self.items.iter().map(|i| if let RawItem::Text(s) = i { s.chars().count() } else { 0 }).sum()
    }
}

/// The whitespace rules of Templates ch. 2 ("Whitespace and escapes"):
///
/// - "The closing tag's column is stripped from every line, and an under-indented line is
///   an error (Q-CT3)."
/// - "A placeholder alone on its line stands for whole lines": it gets its column, and
///   its line's indentation and newline are its own, not text.
/// - "`{{` and `}}` alone on their lines leave no line."
fn finish_template_lines(lx: &mut Lexer<'_>, lines: Vec<RawLine>, strip: u32, first_continues: bool) -> Vec<TemplatePartTok> {
    let mut parts: Vec<TemplatePartTok> = Vec::new();
    let push_text = |parts: &mut Vec<TemplatePartTok>, t: &str| {
        if t.is_empty() {
            return;
        }
        if let Some(TemplatePartTok::Text(s)) = parts.last_mut() {
            s.push_str(t);
        } else {
            parts.push(TemplatePartTok::Text(t.to_string()));
        }
    };
    for (i, mut line) in lines.into_iter().enumerate() {
        if line.block {
            for item in line.items {
                if let RawItem::Block(tokens, span) = item {
                    parts.push(TemplatePartTok::Block { tokens, span });
                }
            }
            continue;
        }
        let strip_this = !(i == 0 && first_continues) && strip > 0;
        if strip_this {
            let is_blank = line.only_whitespace();
            let mut n = 0usize;
            if let Some(RawItem::Text(s)) = line.items.first_mut() {
                n = s.chars().take_while(|c| *c == ' ').count();
                let take = n.min(strip as usize);
                s.drain(..take);
            }
            if n < strip as usize && !is_blank {
                let span = line.items.iter().find_map(|it| match it {
                    RawItem::Placeholder(_, sp) | RawItem::Block(_, sp) | RawItem::Call(.., sp) => Some(*sp),
                    _ => None,
                });
                lx.diags.push(Diagnostic::lex(span.unwrap_or_default(), "a template line is indented less than its closing tag (Q-CT3)"));
            }
            if line.items.first().is_some_and(|it| matches!(it, RawItem::Text(s) if s.is_empty())) {
                line.items.remove(0);
            }
        }
        // A placeholder alone on its line.
        let can_be_alone = !(i == 0 && first_continues) && line.newline;
        let alone = can_be_alone && {
            let non_ws: Vec<&RawItem> = line.items.iter().filter(|it| !matches!(it, RawItem::Text(s) if s.trim_matches([' ', '\r']).is_empty())).collect();
            non_ws.len() == 1 && matches!(non_ws[0], RawItem::Placeholder(..))
        };
        if alone {
            let mut col = 0u32;
            for item in line.items {
                match item {
                    RawItem::Text(s) => {
                        if col == 0 {
                            col = s.chars().take_while(|c| *c == ' ').count() as u32;
                        }
                    }
                    RawItem::Placeholder(tokens, span) => {
                        parts.push(TemplatePartTok::Placeholder { tokens, column: Some(col), span });
                        col = u32::MAX;
                    }
                    _ => {}
                }
            }
            continue;
        }
        for item in line.items {
            match item {
                RawItem::Text(s) => push_text(&mut parts, &s),
                RawItem::Placeholder(tokens, span) => parts.push(TemplatePartTok::Placeholder { tokens, column: None, span }),
                RawItem::Block(tokens, span) => parts.push(TemplatePartTok::Block { tokens, span }),
                RawItem::Call(name, attrs, body, span) => parts.push(TemplatePartTok::Call { name, attrs, body, span }),
            }
        }
        if line.newline {
            push_text(&mut parts, "\n");
        }
    }
    parts
}
