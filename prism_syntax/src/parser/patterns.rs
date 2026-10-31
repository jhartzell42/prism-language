//! Patterns: function parameters, lambda parameters, `match` arms, the left of `<-` and
//! `let`, `for` binders, and the test language's targets and checks.
//!
//! A pattern reads like the expression that would build the value: a constructor applied
//! by commas (`Node l, r`), named fields in touching parentheses (`Error(kind=Fs.NotFound,
//! ..)`), the free forms `? p` and `~ p` ("`? v`, the injection into an optional, which in
//! a pattern takes one apart", ch. 2), tuples and arrays by doubled brackets, `ignore`,
//! literals, `nil`, `unit`, and a family `Ui.*`. The comma rule is the expression rule: a
//! constructor's paren-less arguments take the commas unless the constructor is itself an
//! argument.

use super::*;

impl Parser {
    pub(crate) fn pattern(&mut self, cx: Ctx) -> PResult<Pattern> {
        let cx = if self.step_mode { Ctx::ARG } else { cx };
        let start = self.span();
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::FreeSigil(s @ (Sigil::Question | Sigil::Tilde)) => {
                self.advance();
                let inner = self.pattern(cx)?;
                let span = start.to(inner.span);
                Ok(Pattern { kind: PatternKind::Sigil { sigil: s, inner: Box::new(inner) }, span })
            }
            TokenKind::FreeSigil(s) => self.error_at(tok.span, format!("a free `{}` is not a pattern: only `? p` and `~ p` take a value apart", s.as_char())),
            TokenKind::Name(n) => {
                self.advance();
                let name = SName { sigils: n.sigils.clone(), path: n.path.clone(), span: tok.span };
                if name.is_callee() {
                    if self.at_single_open(BracketKind::Paren) && self.peek().touches_prev {
                        return self.record_pattern(name);
                    }
                    if self.pat_arg_starts_here() {
                        let mut args = vec![self.pattern(Ctx::ARG)?];
                        if cx.commas {
                            while self.at_comma() {
                                self.advance();
                                args.push(self.pattern(Ctx::ARG)?);
                            }
                        }
                        let span = start.to(self.prev_span());
                        return Ok(Pattern { kind: PatternKind::Ctor { name, args }, span });
                    }
                }
                Ok(Pattern { kind: PatternKind::Name(name), span: tok.span })
            }
            TokenKind::Kw(Kw::Ignore, _) => {
                self.advance();
                Ok(Pattern { kind: PatternKind::Ignore, span: tok.span })
            }
            TokenKind::Kw(Kw::Unit, _) => {
                self.advance();
                Ok(Pattern { kind: PatternKind::Unit, span: tok.span })
            }
            TokenKind::Kw(Kw::Nil, _) => {
                self.advance();
                Ok(Pattern { kind: PatternKind::Nil, span: tok.span })
            }
            TokenKind::Int { .. } | TokenKind::Float { .. } | TokenKind::Duration { .. } | TokenKind::Char { .. } | TokenKind::Text { .. } => {
                let lit = self.literal()?;
                Ok(Pattern { span: lit.span, kind: PatternKind::Lit(lit) })
            }
            TokenKind::Op { op: Op::Sub, prefix: true } => {
                // "In a pattern `-40` is a literal."
                self.advance();
                if !matches!(self.kind(), TokenKind::Int { .. } | TokenKind::Float { .. } | TokenKind::Duration { .. }) {
                    return self.error_here("a `-` in a pattern is the sign of a number literal");
                }
                let lit = self.literal()?;
                let span = start.to(lit.span);
                Ok(Pattern { kind: PatternKind::Neg(lit), span })
            }
            TokenKind::Open { kind: BracketKind::Paren, .. } => {
                if self.at_double_open(BracketKind::Paren) {
                    self.open_double();
                    let mut elems = vec![self.pattern(Ctx::ARG)?];
                    while self.at_comma() {
                        self.advance();
                        elems.push(self.pattern(Ctx::ARG)?);
                    }
                    self.close_double(BracketKind::Paren)?;
                    Ok(Pattern { kind: PatternKind::Tuple(elems), span: start.to(self.prev_span()) })
                } else {
                    self.advance();
                    let inner = self.pattern(Ctx::TOP)?;
                    self.expect_close(BracketKind::Paren)?;
                    Ok(Pattern { kind: PatternKind::Group(Box::new(inner)), span: start.to(self.prev_span()) })
                }
            }
            TokenKind::Open { kind: BracketKind::Square, .. } if self.at_double_open(BracketKind::Square) => {
                self.open_double();
                let mut elems = Vec::new();
                if !self.at_close(BracketKind::Square) {
                    elems.push(self.pattern(Ctx::ARG)?);
                    while self.at_comma() {
                        self.advance();
                        elems.push(self.pattern(Ctx::ARG)?);
                    }
                }
                self.close_double(BracketKind::Square)?;
                Ok(Pattern { kind: PatternKind::Array(elems), span: start.to(self.prev_span()) })
            }
            TokenKind::FamilyStar(path) => {
                self.advance();
                Ok(Pattern { kind: PatternKind::Family(QualName { path, span: tok.span }), span: tok.span })
            }
            TokenKind::Absent => {
                self.advance();
                self.out_of_scope(tok.span, "sparse tuples", "an absent component `-` is a sparse tuple's, out of this compiler");
                Ok(Pattern { kind: PatternKind::Ignore, span: tok.span })
            }
            _ => self.error_here("expected a pattern"),
        }
    }

    /// Whether the cursor can begin a constructor pattern's argument.
    fn pat_arg_starts_here(&self) -> bool {
        match self.kind() {
            TokenKind::Name(_)
            | TokenKind::Int { .. }
            | TokenKind::Float { .. }
            | TokenKind::Duration { .. }
            | TokenKind::Char { .. }
            | TokenKind::Text { .. }
            | TokenKind::FamilyStar(_)
            | TokenKind::FreeSigil(Sigil::Question | Sigil::Tilde) => true,
            TokenKind::Open { kind: BracketKind::Paren, .. } => !self.peek().touches_prev || self.at_double_open(BracketKind::Paren),
            TokenKind::Open { kind: BracketKind::Square, .. } => self.at_double_open(BracketKind::Square),
            TokenKind::Op { op: Op::Sub, prefix: true } => true,
            TokenKind::Kw(Kw::Ignore | Kw::Unit | Kw::Nil, _) => true,
            _ => false,
        }
    }

    /// `Name(field=p, p, ..)`.
    fn record_pattern(&mut self, name: SName) -> PResult<Pattern> {
        let start = name.span;
        self.advance(); // (
        let mut fields = Vec::new();
        let mut rest = false;
        if !self.at_close(BracketKind::Paren) {
            loop {
                if matches!(self.kind(), TokenKind::DotDot) {
                    self.advance();
                    rest = true;
                } else {
                    let fstart = self.span();
                    let fname = self.arg_name();
                    let pattern = self.pattern(Ctx::ARG)?;
                    let span = fstart.to(pattern.span);
                    fields.push(FieldPat { name: fname, pattern, span });
                }
                if self.at_comma() {
                    self.advance();
                    continue;
                }
                break;
            }
        }
        self.expect_close(BracketKind::Paren)?;
        Ok(Pattern { kind: PatternKind::Record { name, fields, rest }, span: start.to(self.prev_span()) })
    }
}
