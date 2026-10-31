//! Expressions, blocks and statements.
//!
//! The precedence table (Semantics ch. 2, Precedence; Appendix A.5), loosest first:
//!
//! | Level | Operators | Here |
//! |---|---|---|
//! | 1 | `<-`, `=`, `<=` `+=` ... | statements: [`Parser::stmt`] |
//! | 2 | `then` | the hold, in [`Parser::stmt`]; does not associate |
//! | 3 | application, `\` lambdas, free sigils | [`Parser::primary`]: each takes the rest |
//! | 4 to 11 | `<\|`, `on`, `or`, `and`, `not`, comparison, additive, multiplicative | the Pratt loop, [`Parser::binary`] |
//! | 12 | `as` | in the Pratt loop, a type on its right |
//! | 13 | prefix `-` `+` | [`Parser::prefix`] |
//! | 14 | `.f`, `[i]`, `.Method`, `<<T>>`, marks | [`Parser::postfix`] |
//!
//! Level 3 is not an operator level a Pratt loop can climb to: a paren-less call, a lambda
//! and a free sigil each "take the rest" of their expression, so they are parsed where an
//! operand is expected and swallow every operator after them. That gives `F a <| G b` as
//! `F (a <| G b)` and `Size $k + 1` as `Size ($k + 1)` (ch. 2's precedence examples),
//! while `$count + 1 on !click` stays `($count + 1) on !click`.

use super::*;

impl Parser {
    // ----- entry points --------------------------------------------------------------------

    /// A whole expression with commas allowed to the outermost call.
    pub(crate) fn expr(&mut self) -> PResult<Expr> {
        self.expr_ctx(Ctx::TOP)
    }

    pub(crate) fn expr_ctx(&mut self, cx: Ctx) -> PResult<Expr> {
        let cx = if self.step_mode { Ctx::ARG } else { cx };
        self.binary(4, cx)
    }

    /// An expression on the right of a statement's `=`, `<-` or `then`, with layout rule
    /// 9: "A `.Method` that begins a continuation line closes the call above it and takes
    /// it as its receiver" (Appendix A.4: "everything above it, back to the statement's
    /// `=`, `<-` or `then`").
    pub(crate) fn rhs_expr(&mut self) -> PResult<Expr> {
        let mut e = self.expr()?;
        while self.at_line_method() {
            let start = e.span;
            self.advance(); // the `.`
            let method = self.sname()?;
            let args = if self.arg_starts_here() { self.call_args(Ctx::TOP)? } else { Vec::new() };
            let span = start.to(self.prev_span());
            e = Expr { kind: ExprKind::Method { recv: Box::new(e), method, args }, span };
        }
        Ok(e)
    }

    /// A `.Method` at the start of a continuation line (rule 9).
    pub(crate) fn at_line_method(&self) -> bool {
        matches!(self.kind(), TokenKind::Dot { prefix: true })
            && self.peek().line_start
            && matches!(self.kind_at(1), TokenKind::Name(n) if n.case == NameCase::Upper && n.sigils.is_empty())
    }

    // ----- the Pratt loop ------------------------------------------------------------------

    /// The binary operator at the cursor, if any, with its level.
    fn peek_binop(&self) -> Option<(BinOp, Option<Vec<String>>)> {
        let op = match self.kind() {
            TokenKind::Op { op, prefix } => {
                // A sign the spacing rule made prefix is not binary; `-?` is binary by
                // longest match whatever its spacing (doubt MQ).
                if *prefix && *op != Op::SubQ {
                    return None;
                }
                *op
            }
            TokenKind::SuffixedOp { op, ty } => {
                return op_to_binop(*op).map(|b| (b, Some(ty.clone())));
            }
            TokenKind::Kw(Kw::On, s) if s.is_empty() => return Some((BinOp::On, None)),
            TokenKind::Kw(Kw::Or, s) if s.is_empty() => return Some((BinOp::Or, None)),
            TokenKind::Kw(Kw::And, s) if s.is_empty() => return Some((BinOp::And, None)),
            TokenKind::Kw(Kw::Mod, s) if s.is_empty() => return Some((BinOp::Mod, None)),
            _ => return None,
        };
        op_to_binop(op).map(|b| (b, None))
    }

    /// Operators at `min` and tighter, left-associative (ch. 2: "operators at one level
    /// associate to the left, except `then`").
    pub(crate) fn binary(&mut self, min: u8, cx: Ctx) -> PResult<Expr> {
        let mut lhs = self.prefix(min, cx)?;
        loop {
            // `as`, level 12: "a cast, a type on its right".
            if self.at_kw(Kw::As) && min <= 12 {
                self.advance();
                let ty = self.ty_cast()?;
                // `ranks as [Text]+?`: the checked cast's `?` (doubt PQ).
                let mut checked = false;
                if matches!(self.kind(), TokenKind::FreeSigil(Sigil::Question)) && self.peek().touches_prev {
                    self.advance();
                    checked = true;
                }
                let span = lhs.span.to(self.prev_span());
                lhs = Expr { kind: ExprKind::Cast { expr: Box::new(lhs), ty, checked }, span };
                continue;
            }
            let Some((op, suffix)) = self.peek_binop() else {
                // A prefix sign after a value: the spacing rule's surprise (X5): "the error
                // asks 'did you mean `a - 1`?'"
                if let TokenKind::Op { op: o @ (Op::Sub | Op::Add), prefix: true } = self.kind() {
                    if !self.peek().line_start {
                        let o = o.as_str();
                        let span = self.span();
                        return self.error_at(span, format!("`{o}` here is a prefix sign by the spacing rule (a space before it and none after); did you mean `a {o} 1`, with spaces on both sides?"));
                    }
                }
                if matches!(self.kind(), TokenKind::Bar) {
                    let span = self.span();
                    return self.error_at(span, "a bare `|` is not an operator: the merge is `<|`, and `|` separates the families of a row");
                }
                if matches!(self.kind(), TokenKind::Dot { prefix: true }) && !self.at_line_method() && matches!(self.kind_at(1), TokenKind::Name(n) if n.case == NameCase::Upper) {
                    let span = self.span();
                    return self.error_at(span, "a spaced `.Method` begins its own line, below the call it takes as its receiver (layout rule 9)");
                }
                break;
            };
            if op.level() < min {
                break;
            }
            if let TokenKind::Op { op: Op::SubQ, prefix: true } = self.kind() {
                // Doubt MQ: "longest match wins, with a soft error naming `-(?x)`".
                let span = self.span();
                self.note_error(span, "`-?` is the checked subtraction by longest match; to subtract or negate an optional, write `-(?x)`");
            }
            let op_tok = self.advance();
            let rhs = self.binary(op.level() + 1, cx)?;
            let span = lhs.span.to(rhs.span);
            lhs = match suffix {
                None => Expr { kind: ExprKind::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) }, span },
                Some(ty) => {
                    self.out_of_scope(op_tok.span, "suffixed operators", format!("the suffixed operator `{}` is out of this compiler", crate::corpus::token_text(&op_tok.kind)));
                    Expr { kind: ExprKind::Suffixed { op, ty: QualName { path: ty, span: op_tok.span }, lhs: Box::new(lhs), rhs: Box::new(rhs) }, span }
                }
            };
        }
        Ok(lhs)
    }

    /// Prefix operators: `not` (level 8) and the signs (level 13).
    fn prefix(&mut self, min: u8, cx: Ctx) -> PResult<Expr> {
        let start = self.span();
        if self.at_kw(Kw::Not) {
            self.advance();
            // "`not a == b` is `not (a == b)`": the operand is a comparison.
            let operand = self.binary(9.max(min), cx)?;
            let span = start.to(operand.span);
            return Ok(Expr { kind: ExprKind::Unary { op: UnOp::Not, operand: Box::new(operand) }, span });
        }
        if let TokenKind::Op { op: op @ (Op::Sub | Op::Add), prefix: true } = *self.kind() {
            self.advance();
            let operand = self.postfix_expr(cx)?;
            let span = start.to(operand.span);
            let op = if op == Op::Sub { UnOp::Neg } else { UnOp::Pos };
            return Ok(Expr { kind: ExprKind::Unary { op, operand: Box::new(operand) }, span });
        }
        if let TokenKind::Op { op: Op::SubQ, .. } = self.kind() {
            // Doubt MQ: "longest match wins, with a soft error naming `-(?x)`".
            let span = self.span();
            return self.error_at(span, "`-?` is the checked subtraction and needs a left operand; to negate an optional write `-(?x)`");
        }
        self.postfix_expr(cx)
    }

    /// A primary and its postfix operators.
    pub(crate) fn postfix_expr(&mut self, cx: Ctx) -> PResult<Expr> {
        let (e, took_rest) = self.primary(cx)?;
        if took_rest {
            return Ok(e);
        }
        self.postfix(e, cx)
    }

    /// Level 14: `.f`, `[i]`, `.Method`, `<<T>>`, the non-empty mark.
    pub(crate) fn postfix(&mut self, mut e: Expr, cx: Ctx) -> PResult<Expr> {
        loop {
            match self.kind().clone() {
                TokenKind::Dot { prefix: false } => {
                    let dot = self.advance();
                    match self.kind().clone() {
                        TokenKind::Name(n) if n.sigils.is_empty() && n.path.len() == 1 && matches!(n.case, NameCase::Lower | NameCase::Topic) => {
                            let t = self.advance();
                            let span = e.span.to(t.span);
                            e = Expr { kind: ExprKind::Field { recv: Box::new(e), name: Ident { name: n.path[0].clone(), span: t.span } }, span };
                            // A field holding a function is applied like a callee: a field's
                            // name is lowercase, so `p.run input` is the only way to call one
                            // (Type System ch. 3's `Parser`; see API.md's picks).
                            if self.arg_starts_here() && !self.peek().line_start {
                                let args = self.call_args(cx)?;
                                let span = e.span.to(self.prev_span());
                                return Ok(Expr { kind: ExprKind::Call { callee: Box::new(e), args, parens: false, rest: false }, span });
                            }
                        }
                        TokenKind::Kw(kw @ (Kw::Key | Kw::Ix | Kw::Jx | Kw::Kx), s) if s.is_empty() => {
                            // "a record may still have a field named `key`".
                            let t = self.advance();
                            let span = e.span.to(t.span);
                            e = Expr { kind: ExprKind::Field { recv: Box::new(e), name: Ident { name: kw.as_str().into(), span: t.span } }, span };
                        }
                        TokenKind::Name(n) if n.sigils.is_empty() && (n.case == NameCase::Upper || n.case == NameCase::ReservedCtor) => {
                            let method = self.sname()?;
                            let args = if self.arg_starts_here() { self.call_args(cx)? } else { Vec::new() };
                            let span = e.span.to(self.prev_span());
                            return Ok(Expr { kind: ExprKind::Method { recv: Box::new(e), method, args }, span });
                        }
                        _ => return self.error_at(dot.span, "expected a field name or a `.Method` after `.`"),
                    }
                }
                TokenKind::Open { kind: BracketKind::Square, prefix: false } if self.peek().touches_prev && !self.at_double_open(BracketKind::Square) => {
                    self.advance();
                    let index = self.expr()?;
                    self.expect_close(BracketKind::Square)?;
                    let span = e.span.to(self.prev_span());
                    e = Expr { kind: ExprKind::Index { recv: Box::new(e), index: Box::new(index) }, span };
                }
                TokenKind::LtLt => {
                    let types = self.type_args()?;
                    let span = e.span.to(self.prev_span());
                    e = Expr { kind: ExprKind::TypeArgs { target: Box::new(e), types }, span };
                }
                TokenKind::NonEmpty => {
                    let t = self.advance();
                    self.out_of_scope(t.span, "non-empty arrays", "the non-empty mark `+` is out of this compiler");
                    let span = e.span.to(t.span);
                    e = Expr { kind: ExprKind::NonEmpty(Box::new(e)), span };
                }
                _ => return Ok(e),
            }
        }
    }

    /// `<<T, U>>` after a name.
    pub(crate) fn type_args(&mut self) -> PResult<Vec<Type>> {
        self.advance(); // <<
        let mut types = vec![self.ty()?];
        while self.at_comma() {
            self.advance();
            types.push(self.ty()?);
        }
        if matches!(self.kind(), TokenKind::GtGt) {
            self.advance();
            Ok(types)
        } else {
            self.error_here("expected `>>` to close the type arguments")
        }
    }

    // ----- calls ----------------------------------------------------------------------------

    /// Whether the token at the cursor can begin an argument of a paren-less call.
    pub(crate) fn arg_starts_here(&self) -> bool {
        let t = self.peek();
        match &t.kind {
            TokenKind::Name(_)
            | TokenKind::Int { .. }
            | TokenKind::Float { .. }
            | TokenKind::Duration { .. }
            | TokenKind::Char { .. }
            | TokenKind::Text { .. }
            | TokenKind::Template(_)
            | TokenKind::FreeSigil(_)
            | TokenKind::LensDot
            | TokenKind::Backslash
            | TokenKind::Absent => true,
            // A touching single `[` indexes the callee; a touching `(` was its argument list.
            TokenKind::Open { kind: BracketKind::Square, prefix } => *prefix || !t.touches_prev || self.at_double_open(BracketKind::Square),
            TokenKind::Open { .. } => true,
            TokenKind::Op { op: Op::Add | Op::Sub, prefix: true } => true,
            TokenKind::Dot { prefix: true } => !self.at_line_method(),
            TokenKind::Kw(kw, _) => matches!(
                kw,
                Kw::Unit | Kw::Nil | Kw::Key | Kw::Ix | Kw::Jx | Kw::Kx | Kw::Not | Kw::Any | Kw::Given | Kw::Update
                    | Kw::Match | Kw::Rebuild | Kw::Widget | Kw::Output | Kw::InlineText | Kw::Raw
            ),
            _ => false,
        }
    }

    /// Paren-less arguments: the first, then (when this call may take commas) the rest.
    pub(crate) fn call_args(&mut self, cx: Ctx) -> PResult<Vec<Arg>> {
        let mut args = vec![self.arg()?];
        if cx.commas && !self.step_mode {
            while self.at_comma() {
                self.advance();
                args.push(self.arg()?);
            }
        }
        Ok(args)
    }

    fn arg(&mut self) -> PResult<Arg> {
        let value = self.expr_ctx(Ctx::ARG)?;
        Ok(Arg { name: None, span: value.span, value })
    }

    /// `F(a, b)`, `Ui.Button(label=$"Go")`, `Error(kind=Fs.NotFound, ..)`: arguments in
    /// touching parentheses, each named or not. Returns the arguments and whether `..`
    /// ended them.
    fn paren_args(&mut self) -> PResult<(Vec<Arg>, bool)> {
        self.advance(); // (
        let mut args = Vec::new();
        let mut rest = false;
        if self.at_close(BracketKind::Paren) {
            self.advance();
            return Ok((args, rest));
        }
        loop {
            if matches!(self.kind(), TokenKind::DotDot) {
                self.advance();
                rest = true;
            } else {
                let start = self.span();
                let name = self.arg_name();
                let value = self.expr_ctx(Ctx::ARG)?;
                let span = start.to(value.span);
                args.push(Arg { name, value, span });
            }
            if self.at_comma() {
                self.advance();
                continue;
            }
            self.expect_close(BracketKind::Paren)?;
            return Ok((args, rest));
        }
    }

    /// `name=` before a named argument.
    pub(crate) fn arg_name(&mut self) -> Option<Ident> {
        if let TokenKind::Name(n) = self.kind() {
            if n.sigils.is_empty() && n.path.len() == 1 && n.case == NameCase::Lower && matches!(self.kind_at(1), TokenKind::Op { op: Op::Eq, .. }) {
                let name = n.path[0].clone();
                let span = self.advance().span;
                self.advance();
                return Some(Ident { name, span });
            }
        }
        None
    }

    pub(crate) fn sname(&mut self) -> PResult<SName> {
        match self.kind().clone() {
            TokenKind::Name(n) => {
                let span = self.advance().span;
                Ok(SName { sigils: n.sigils, path: n.path, span })
            }
            _ => self.error_here("expected a name"),
        }
    }

    // ----- primaries ------------------------------------------------------------------------

    /// An operand. The flag is true when the primary "took the rest" (a paren-less call, a
    /// lambda, a free sigil, a block form), so no postfix operator follows it.
    pub(crate) fn primary(&mut self, cx: Ctx) -> PResult<(Expr, bool)> {
        let start = self.span();
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::Name(n) => {
                self.advance();
                let name = SName { sigils: n.sigils.clone(), path: n.path.clone(), span: tok.span };
                self.check_name_scope(&name);
                let mut e = Expr { kind: ExprKind::Name(name.clone()), span: tok.span };
                if matches!(self.kind(), TokenKind::LtLt) {
                    let types = self.type_args()?;
                    let span = start.to(self.prev_span());
                    e = Expr { kind: ExprKind::TypeArgs { target: Box::new(e), types }, span };
                }
                if name.is_callee() {
                    // `F(a, b)`: parentheses touching the callee hold its arguments.
                    if self.at_single_open(BracketKind::Paren) && self.peek().touches_prev {
                        let (args, rest) = self.paren_args()?;
                        let span = start.to(self.prev_span());
                        let call = Expr { kind: ExprKind::Call { callee: Box::new(e), args, parens: true, rest }, span };
                        return Ok((call, false));
                    }
                    // `Parser monad:` (out of this compiler).
                    if self.at_kw(Kw::Monad) {
                        let kw = self.advance();
                        self.out_of_scope(kw.span, "`T monad:` blocks and `^Monad`", "monad blocks are out of this compiler");
                        let body = self.block_after_colon()?;
                        let span = start.to(body.span);
                        return Ok((Expr { kind: ExprKind::Monad { monad: Box::new(e), body }, span }, true));
                    }
                    if self.arg_starts_here() {
                        let args = self.call_args(cx)?;
                        let span = start.to(self.prev_span());
                        return Ok((Expr { kind: ExprKind::Call { callee: Box::new(e), args, parens: false, rest: false }, span }, true));
                    }
                } else if self.at_single_open(BracketKind::Paren) && self.peek().touches_prev && !name.is_topic() {
                    let span = self.span();
                    return self.error_at(span, format!("`{}` is a value and is not applied: a function's name is uppercase (case is a mark)", name.text()));
                }
                Ok((e, false))
            }
            TokenKind::Kw(kw, sigils) => self.keyword_primary(kw, sigils, cx),
            TokenKind::Int { .. } | TokenKind::Float { .. } | TokenKind::Duration { .. } | TokenKind::Char { .. } | TokenKind::Text { .. } => {
                let lit = self.literal()?;
                Ok((Expr { span: lit.span, kind: ExprKind::Lit(lit) }, false))
            }
            TokenKind::Template(t) => {
                self.advance();
                let t = self.template(*t, tok.span)?;
                Ok((Expr { kind: ExprKind::Template(Box::new(t)), span: tok.span }, false))
            }
            TokenKind::Open { kind: BracketKind::Paren, .. } => {
                if self.at_double_open(BracketKind::Paren) {
                    // `((a, b))`: a tuple; "A comma belongs to the nearest bracket around it".
                    self.open_double();
                    let mut elems = vec![self.expr_ctx(Ctx::ARG)?];
                    while self.at_comma() {
                        self.advance();
                        elems.push(self.expr_ctx(Ctx::ARG)?);
                    }
                    self.close_double(BracketKind::Paren)?;
                    let span = start.to(self.prev_span());
                    Ok((Expr { kind: ExprKind::Tuple(elems), span }, false))
                } else {
                    self.advance();
                    let inner = self.rhs_expr()?;
                    if self.at_comma() {
                        let span = self.span();
                        return self.error_at(span, "a group holds one expression: a comma here needs a call at the group's head, or `((` `))` for a tuple");
                    }
                    self.expect_close(BracketKind::Paren)?;
                    let span = start.to(self.prev_span());
                    Ok((Expr { kind: ExprKind::Group(Box::new(inner)), span }, false))
                }
            }
            TokenKind::Open { kind: BracketKind::Square, .. } => {
                if self.at_double_open(BracketKind::Square) {
                    self.open_double();
                    let mut elems = Vec::new();
                    if !self.at_close(BracketKind::Square) {
                        elems.push(self.expr_ctx(Ctx::ARG)?);
                        while self.at_comma() {
                            self.advance();
                            elems.push(self.expr_ctx(Ctx::ARG)?);
                        }
                    }
                    self.close_double(BracketKind::Square)?;
                    let span = start.to(self.prev_span());
                    Ok((Expr { kind: ExprKind::Array(elems), span }, false))
                } else {
                    // A prefix `[i]` applies to the topic (the spacing rule).
                    self.advance();
                    let index = self.expr()?;
                    self.expect_close(BracketKind::Square)?;
                    let span = start.to(self.prev_span());
                    Ok((Expr { kind: ExprKind::TopicIndex(Box::new(index)), span }, false))
                }
            }
            TokenKind::Open { kind: BracketKind::Brace, .. } => {
                // A map: `{"ada": 3}`, `{}`.
                self.advance();
                let mut entries = Vec::new();
                if !self.at_close(BracketKind::Brace) {
                    loop {
                        let k = self.expr_ctx(Ctx::ARG)?;
                        self.expect_colon()?;
                        let v = self.expr_ctx(Ctx::ARG)?;
                        entries.push((k, v));
                        if self.at_comma() {
                            self.advance();
                            continue;
                        }
                        break;
                    }
                }
                self.expect_close(BracketKind::Brace)?;
                let span = start.to(self.prev_span());
                Ok((Expr { kind: ExprKind::Map(entries), span }, false))
            }
            TokenKind::Dot { prefix: true } => {
                self.advance();
                match self.kind().clone() {
                    TokenKind::Name(n) if n.sigils.is_empty() && n.path.len() == 1 && n.case == NameCase::Lower => {
                        let t = self.advance();
                        Ok((Expr { kind: ExprKind::TopicField(Ident { name: n.path[0].clone(), span: t.span }), span: start.to(t.span) }, false))
                    }
                    TokenKind::Name(n) if n.case == NameCase::Upper => {
                        // Rule 9: "On one line a spaced `.Method` is an error asking for its
                        // own line."
                        self.error_at(start, format!("a spaced `.{}` begins its own line, below the call it takes as its receiver (layout rule 9)", n.path.join(".")))
                    }
                    _ => self.error_at(start, "a prefix `.` takes a field name, applied to the topic `_`"),
                }
            }
            TokenKind::LensDot => {
                // `&.score.home`, a lens value (Type System ch. 3).
                self.advance();
                let mut path = Vec::new();
                loop {
                    match self.kind().clone() {
                        TokenKind::Name(n) if n.sigils.is_empty() && n.case == NameCase::Lower && n.path.len() == 1 => {
                            let t = self.advance();
                            path.push(Ident { name: n.path[0].clone(), span: t.span });
                        }
                        _ => return self.error_here("expected a field name in a lens path"),
                    }
                    if matches!(self.kind(), TokenKind::Dot { prefix: false }) {
                        self.advance();
                        continue;
                    }
                    break;
                }
                let span = start.to(self.prev_span());
                Ok((Expr { kind: ExprKind::Lens(path), span }, false))
            }
            TokenKind::FreeSigil(s) => {
                self.advance();
                match s {
                    Sigil::Bang | Sigil::Amp => {
                        return self.error_at(tok.span, format!("`{}` is never free: {}", s.as_char(), if s == Sigil::Bang { "no event fires a constant" } else { "no reference exists outside the value an `update` block names" }));
                    }
                    _ => {}
                }
                // "a free sigil applies to everything to its right, to the end of the
                // expression it stands in, as a paren-less call does."
                let operand = self.expr_ctx(cx)?;
                let span = start.to(operand.span);
                Ok((Expr { kind: ExprKind::FreeSigil { sigil: s, operand: Box::new(operand) }, span }, true))
            }
            TokenKind::Backslash => {
                let e = self.lambda(cx)?;
                Ok((e, true))
            }
            TokenKind::Absent => {
                self.advance();
                self.out_of_scope(tok.span, "sparse tuples", "an absent component `-` makes a sparse tuple, which is out of this compiler");
                Ok((Expr { kind: ExprKind::Absent, span: tok.span }, false))
            }
            TokenKind::DotDot => {
                self.advance();
                Ok((Expr { kind: ExprKind::Rest, span: tok.span }, false))
            }
            TokenKind::Trait(_) => self.error_at(tok.span, "no trait is a value: a `^Trait` stands in a type, a bound, `derive`, `trait` or `impl`"),
            TokenKind::FamilyStar(p) => self.error_at(tok.span, format!("`{}.*` names every tag of a family: it stands in a pattern, an `emit` row or a `use`, not in an expression", p.join("."))),
            TokenKind::Bar => self.error_at(tok.span, "a bare `|` is not an operator: the merge is `<|`, and `|` separates the families of a row"),
            _ => self.error_here("expected an expression"),
        }
    }

    /// Primaries that begin with a keyword.
    fn keyword_primary(&mut self, kw: Kw, sigils: Vec<Sigil>, cx: Ctx) -> PResult<(Expr, bool)> {
        let start = self.span();
        let simple = |k: ExprKind, s: Span| Ok((Expr { kind: k, span: s }, false));
        match kw {
            Kw::Unit => {
                self.advance();
                simple(ExprKind::Unit(sigils), start)
            }
            Kw::Nil => {
                self.advance();
                simple(ExprKind::Nil(sigils), start)
            }
            Kw::Key | Kw::Ix | Kw::Jx | Kw::Kx => {
                self.advance();
                let b = match kw {
                    Kw::Key => BoundName::Key,
                    Kw::Ix => BoundName::Ix,
                    Kw::Jx => BoundName::Jx,
                    _ => BoundName::Kx,
                };
                simple(ExprKind::Bound(b), start)
            }
            Kw::If => {
                let e = self.if_expr()?;
                Ok((e, true))
            }
            Kw::Match => {
                let e = self.match_expr()?;
                Ok((e, true))
            }
            Kw::Rebuild => {
                self.advance();
                let scrutinee = self.expr()?;
                let body = self.block_after_colon()?;
                let span = start.to(body.span);
                Ok((Expr { kind: ExprKind::Rebuild { scrutinee: Box::new(scrutinee), body }, span }, true))
            }
            Kw::For => {
                let e = self.for_expr()?;
                Ok((e, true))
            }
            Kw::Given => {
                self.advance();
                let body = self.block_after_colon()?;
                let span = start.to(body.span);
                Ok((Expr { kind: ExprKind::Given(body), span }, true))
            }
            Kw::Update => {
                self.advance();
                let target = self.expr()?;
                let body = self.block_after_colon()?;
                let span = start.to(body.span);
                Ok((Expr { kind: ExprKind::Update { target: Box::new(target), body }, span }, true))
            }
            Kw::Widget => {
                // `@widget:` is one token (ch. 2, Keywords).
                self.advance();
                let body = self.block_after_colon()?;
                let span = start.to(body.span);
                Ok((Expr { kind: ExprKind::WidgetBlock(body), span }, true))
            }
            Kw::Any => {
                self.advance();
                let operand = self.expr_ctx(cx)?;
                let span = start.to(operand.span);
                Ok((Expr { kind: ExprKind::Any(Box::new(operand)), span }, true))
            }
            Kw::Not => {
                let e = self.binary(8, cx)?;
                Ok((e, true))
            }
            Kw::Output | Kw::InlineText | Kw::Raw => {
                // The lexer made the template the next token, or reported its absence.
                self.advance();
                match self.kind().clone() {
                    TokenKind::Template(t) => {
                        let tspan = self.advance().span;
                        let t = self.template(*t, tspan)?;
                        let span = start.to(tspan);
                        Ok((Expr { kind: ExprKind::Template(Box::new(t)), span }, false))
                    }
                    _ => Err(()),
                }
            }
            Kw::Then => self.error_at(start, "`then` belongs to a hold: `$x <- start then step`"),
            Kw::Else => self.error_at(start, "`else` without its `if`"),
            Kw::Ignore => self.error_at(start, "`ignore` stands only inside a pattern or as the left side of a bind"),
            Kw::Return => self.error_at(start, "`return` ends a monad block, as its last statement"),
            _ => self.error_here("expected an expression"),
        }
    }

    // ----- literals and templates ------------------------------------------------------------

    pub(crate) fn literal(&mut self) -> PResult<Literal> {
        let tok = self.advance();
        let span = tok.span;
        let (marks, kind) = match tok.kind {
            TokenKind::Int { sigils, num } => (sigils, LitKind::Int(num.text)),
            TokenKind::Float { sigils, num } => {
                self.out_of_scope(span, "Float", "`Float` is out of this compiler, and a decimal-spelled literal is a `Float`");
                (sigils, LitKind::Float(num.text))
            }
            TokenKind::Duration { sigils, num, unit } => (sigils, LitKind::Duration { digits: num.text, unit }),
            TokenKind::Char { sigils, ch } => (sigils, LitKind::Char(ch)),
            TokenKind::Text { sigils, text } => {
                let mut segs = Vec::new();
                for seg in text.segments {
                    match seg {
                        TextSegTok::Plain(s) => segs.push(TextSeg::Plain(s)),
                        TextSegTok::Placeholder(toks, pspan) => {
                            let e = self.nested_expr(toks, pspan)?;
                            segs.push(TextSeg::Placeholder(e));
                        }
                    }
                }
                (sigils, LitKind::Text(segs))
            }
            _ => return self.error_at(span, "expected a literal"),
        };
        Ok(Literal { marks, kind, span })
    }

    /// Parses a nested token stream (a placeholder's) as one expression.
    pub(crate) fn nested_expr(&mut self, toks: Vec<Token>, span: Span) -> PResult<Expr> {
        let mut sub = self.sub(toks, span.at_end());
        sub.step_mode = false;
        let r = sub.rhs_expr();
        if r.is_ok() && !sub.at_eof() {
            let sp = sub.span();
            let found = describe(sub.kind());
            sub.note_error(sp, format!("expected the end of the placeholder, found {found}"));
        }
        if r.is_err() && sub.diags.is_empty() {
            sub.note_error(span, "expected an expression in the placeholder");
        }
        self.absorb(sub);
        match r {
            Ok(e) => Ok(e),
            Err(()) => Ok(Expr { kind: ExprKind::Error, span }),
        }
    }

    /// A template token to its tree (Templates ch. 2).
    pub(crate) fn template(&mut self, t: TemplateTok, span: Span) -> PResult<Template> {
        if t.delims == Delims::Single && t.keyword != TemplateKw::Raw {
            self.out_of_scope(span, "the template notation", format!("a `<{}>` template is the template notation for an element language, out of this compiler (code templates only)", t.lang));
        }
        if t.keyword == TemplateKw::File {
            self.out_of_scope(span, "the template notation", "a file whose first token is a language tag is a template file, out of this compiler (code templates only)");
        }
        let parts = self.template_parts(t.parts)?;
        Ok(Template { keyword: t.keyword, lang: t.lang, delims: t.delims, parts, span })
    }

    fn template_parts(&mut self, parts: Vec<TemplatePartTok>) -> PResult<Vec<TemplatePart>> {
        let mut out = Vec::new();
        for p in parts {
            match p {
                TemplatePartTok::Text(s) => out.push(TemplatePart::Text(s)),
                TemplatePartTok::Placeholder { tokens, column, span } => {
                    let expr = self.nested_expr(tokens, span)?;
                    out.push(TemplatePart::Placeholder { expr, column });
                }
                TemplatePartTok::Block { tokens, span } => {
                    let mut sub = self.sub(tokens, span.at_end());
                    let block = sub.block_body_until_eof(span);
                    self.absorb(sub);
                    out.push(TemplatePart::Block(block));
                }
                TemplatePartTok::Call { name, attrs, body, span } => {
                    let sname = SName { sigils: name.sigils, path: name.path, span };
                    let mut args = Vec::new();
                    for a in attrs {
                        let value = self.nested_expr(a.tokens, a.span)?;
                        args.push(Arg { name: Some(Ident { name: a.name, span: a.span }), value, span: a.span });
                    }
                    let body = match body {
                        Some(b) => Some(self.template_parts(b)?),
                        None => None,
                    };
                    out.push(TemplatePart::Call { name: sname, attrs: args, body, span });
                }
            }
        }
        Ok(out)
    }

    // ----- lambdas and block forms ----------------------------------------------------------

    /// `\|a, b| body`, `\body` on the topic. "the lambda's body takes the rest".
    fn lambda(&mut self, cx: Ctx) -> PResult<Expr> {
        let start = self.advance().span; // `\`
        let mut params = None;
        if matches!(self.kind(), TokenKind::LambdaBar { open: true }) {
            self.advance();
            let mut ps = Vec::new();
            if !matches!(self.kind(), TokenKind::LambdaBar { open: false }) {
                ps.push(self.pattern(Ctx::ARG)?);
                while self.at_comma() {
                    self.advance();
                    ps.push(self.pattern(Ctx::ARG)?);
                }
            }
            if !matches!(self.kind(), TokenKind::LambdaBar { open: false }) {
                return self.error_here("expected `|` to close the lambda's parameters");
            }
            self.advance();
            params = Some(ps);
        }
        // Rule 3: a closing bar last on its line opens a block, "The lines before the last
        // are `let`s, and the last is the lambda's body."
        let body = if self.at_layout(Layout::Open) {
            LambdaBody::Block(self.indented_block()?)
        } else {
            let _ = cx;
            LambdaBody::Expr(self.expr_ctx(Ctx::TOP)?)
        };
        let end = match &body {
            LambdaBody::Expr(e) => e.span,
            LambdaBody::Block(b) => b.span,
        };
        Ok(Expr { kind: ExprKind::Lambda { params, body: Box::new(body) }, span: start.to(end) })
    }

    /// A condition, or `pattern <- e` with an optional guard.
    fn cond(&mut self, allow_guard: bool) -> PResult<Cond> {
        let bind = self.speculate(|p| {
            let pat = p.pattern(Ctx::TOP)?;
            if p.at_op(Op::BindArrow) { Ok(pat) } else { Err(()) }
        });
        match bind {
            Some(pattern) => {
                self.advance(); // <-
                let value = self.expr()?;
                let guard = if allow_guard && self.eat_kw(Kw::If) { Some(self.expr()?) } else { None };
                Ok(Cond::Bind { pattern, value, guard })
            }
            None => Ok(Cond::Expr(self.expr()?)),
        }
    }

    /// `if c: a else: b`, inline or in blocks; rule 8 attaches an `else` at the `if`'s
    /// column.
    fn if_expr(&mut self) -> PResult<Expr> {
        let start = self.advance().span;
        let cond = self.cond(false)?;
        let then = self.block_after_colon()?;
        let mut else_ = None;
        if self.at_layout(Layout::Sep) && matches!(self.kind_at(1), TokenKind::Kw(Kw::Else, _)) {
            self.advance();
        }
        if self.eat_kw(Kw::Else) {
            else_ = Some(self.block_after_colon()?);
        }
        let end = else_.as_ref().map(|b| b.span).unwrap_or(then.span);
        Ok(Expr { kind: ExprKind::If { cond: Box::new(cond), then, else_ }, span: start.to(end) })
    }

    /// `match e:` and its arms: "An arm is a pattern, a guard if it has one, and a colon,
    /// with its body on the line or in a block"; the catch-all is `otherwise:`.
    fn match_expr(&mut self) -> PResult<Expr> {
        let start = self.advance().span;
        let scrutinee = self.expr()?;
        self.expect_colon()?;
        if !self.at_layout(Layout::Open) {
            return self.error_here("a `match`'s arms stand in an indented block after its `:`");
        }
        self.advance();
        let mut arms = Vec::new();
        loop {
            let astart = self.span();
            let arm = (|| -> PResult<Arm> {
                if self.eat_kw(Kw::Otherwise) {
                    let body = self.block_after_colon()?;
                    return Ok(Arm { pattern: None, guard: None, span: astart.to(body.span), body });
                }
                if self.at_kw(Kw::Ignore) && matches!(self.kind_at(1), TokenKind::Colon) {
                    let span = self.span();
                    return self.error_at(span, "an arm beginning `ignore:` would read as \"ignore these cases\": the catch-all is `otherwise:`");
                }
                let pattern = self.pattern(Ctx::TOP)?;
                let guard = if self.eat_kw(Kw::If) { Some(self.expr()?) } else { None };
                let body = self.block_after_colon()?;
                Ok(Arm { pattern: Some(pattern), guard, span: astart.to(body.span), body })
            })();
            match arm {
                Ok(a) => arms.push(a),
                Err(()) => self.recover_to_stmt_end(),
            }
            if self.at_layout(Layout::Sep) {
                self.advance();
                continue;
            }
            if self.at_layout(Layout::Close) {
                self.advance();
                break;
            }
            if self.at_eof() {
                break;
            }
            let _ = self.error_here::<()>("expected the end of the arm");
            self.recover_to_stmt_end();
        }
        let span = start.to(self.prev_span());
        Ok(Expr { kind: ExprKind::Match { scrutinee: Box::new(scrutinee), arms }, span })
    }

    /// `for k, v <- e:`, `for x <- e:`, `for e:` (binding `key` and `_`).
    fn for_expr(&mut self) -> PResult<Expr> {
        let start = self.advance().span;
        let binders = self.speculate(|p| {
            let first = p.pattern(Ctx::ARG)?;
            let second = if p.at_comma() {
                p.advance();
                Some(p.pattern(Ctx::ARG)?)
            } else {
                None
            };
            if p.at_op(Op::BindArrow) { Ok(ForBinders { first, second }) } else { Err(()) }
        });
        if binders.is_some() {
            self.advance(); // <-
        }
        let source = self.expr()?;
        let body = self.block_after_colon()?;
        let span = start.to(body.span);
        Ok(Expr { kind: ExprKind::For { binders, source: Box::new(source), body }, span })
    }

    // ----- blocks ----------------------------------------------------------------------------

    /// `:` and then either an indented block (rule 2) or an inline one (rule 5).
    pub(crate) fn block_after_colon(&mut self) -> PResult<Block> {
        self.expect_colon()?;
        if self.at_layout(Layout::Open) {
            return self.indented_block();
        }
        // Rule 5: "one statement, or several separated by `;`, ending at an `else`, at the
        // end of the line, or at the close of an enclosing bracket."
        let start = self.span();
        if self.at_stmt_boundary() {
            return self.error_here("expected a statement after `:`");
        }
        let mut stmts = vec![self.stmt()?];
        while matches!(self.kind(), TokenKind::Semi) {
            self.advance();
            stmts.push(self.stmt()?);
        }
        let span = start.to(self.prev_span());
        Ok(Block { stmts, inline: true, span })
    }

    /// `open` statements separated by `sep`, then `close`.
    pub(crate) fn indented_block(&mut self) -> PResult<Block> {
        let start = self.advance().span; // open
        let mut stmts = Vec::new();
        loop {
            match self.stmt() {
                Ok(s) => stmts.push(s),
                Err(()) => self.recover_to_stmt_end(),
            }
            match self.kind() {
                TokenKind::Layout(Layout::Sep) => {
                    self.advance();
                }
                TokenKind::Layout(Layout::Close) => {
                    self.advance();
                    break;
                }
                TokenKind::Eof => break,
                _ => {
                    let _ = self.error_here::<()>("expected the end of the statement");
                    self.recover_to_stmt_end();
                }
            }
        }
        let span = start.to(self.prev_span());
        Ok(Block { stmts, inline: false, span })
    }

    /// A brace block's statements: a stream of its own with its own layout, ending at its
    /// end of input.
    pub(crate) fn block_body_until_eof(&mut self, span: Span) -> Block {
        let mut stmts = Vec::new();
        while !self.at_eof() {
            match self.stmt() {
                Ok(s) => stmts.push(s),
                Err(()) => self.recover_to_stmt_end(),
            }
            match self.kind() {
                TokenKind::Layout(Layout::Sep) | TokenKind::Layout(Layout::Close) => {
                    self.advance();
                }
                TokenKind::Eof => break,
                _ => {
                    let _ = self.error_here::<()>("expected the end of the statement");
                    self.recover_to_stmt_end();
                }
            }
        }
        Block { stmts, inline: false, span }
    }

    // ----- statements ------------------------------------------------------------------------

    /// One statement (Appendix A.6, `stmt` and `ustmt`).
    pub(crate) fn stmt(&mut self) -> PResult<Stmt> {
        let start = self.span();
        let kind = match self.kind().clone() {
            TokenKind::Kw(Kw::Emit, s) if s.is_empty() => {
                self.advance();
                StmtKind::Emit(self.row()?)
            }
            TokenKind::Kw(Kw::Let, s) if s.is_empty() => {
                self.advance();
                let pattern = self.pattern(Ctx::ARG)?;
                let ty = if self.at_colon() {
                    self.advance();
                    Some(self.ty()?)
                } else {
                    None
                };
                self.expect_op(Op::Eq)?;
                let value = self.rhs_expr()?;
                StmtKind::Let { pattern, ty, value }
            }
            TokenKind::Kw(Kw::Fn, s) if s.is_empty() => StmtKind::Fn(self.fn_clause()?),
            TokenKind::Kw(Kw::Ignore, s) if s.is_empty() && matches!(self.kind_at(1), TokenKind::Op { op: Op::BindArrow, .. }) => {
                self.advance();
                self.advance();
                StmtKind::Ignore(self.rhs_expr()?)
            }
            TokenKind::Kw(kw @ (Kw::Require | Kw::Forbid), s) if s.is_empty() => {
                self.advance();
                // The condition takes no commas, so a second argument can follow it.
                let c = self.expr_ctx(Ctx::ARG)?;
                let e = if self.at_comma() {
                    self.advance();
                    Some(self.expr()?)
                } else {
                    None
                };
                if kw == Kw::Require { StmtKind::Require(c, e) } else { StmtKind::Forbid(c, e) }
            }
            TokenKind::Kw(Kw::While, s) if s.is_empty() => {
                self.advance();
                let cond = self.cond(true)?;
                let body = self.block_after_colon()?;
                StmtKind::While { cond, body }
            }
            TokenKind::Kw(Kw::Return, s) if s.is_empty() => {
                let t = self.advance();
                self.out_of_scope(t.span, "`T monad:` blocks and `^Monad`", "`return` ends a monad block, which is out of this compiler");
                StmtKind::Return(self.rhs_expr()?)
            }
            TokenKind::Name(_) if matches!(self.kind_at(1), TokenKind::Colon) => {
                // A signature line, or `@name:` and a block.
                if matches!(self.kind_at(2), TokenKind::Layout(Layout::Open)) {
                    let name = self.sname()?;
                    let body = self.block_after_colon()?;
                    let span = start.to(body.span);
                    StmtKind::Widget(WidgetDecl { name, body, span })
                } else {
                    StmtKind::Signature(self.signature()?)
                }
            }
            _ => {
                if let Some(k) = self.speculate(|p| p.update_stmt()) {
                    k
                } else if let Some(pattern) = self.speculate(|p| {
                    let pat = p.pattern(Ctx::TOP)?;
                    if p.at_op(Op::BindArrow) { Ok(pat) } else { Err(()) }
                }) {
                    self.advance(); // <-
                    let value = self.rhs_expr()?;
                    let then = if self.eat_kw(Kw::Then) { Some(self.rhs_expr()?) } else { None };
                    if self.at_kw(Kw::Then) {
                        let span = self.span();
                        return self.error_at(span, "`then` does not associate: a hold has one start and one step");
                    }
                    StmtKind::Bind { pattern, value, then }
                } else {
                    let e = self.rhs_expr()?;
                    if self.at_op(Op::BindArrow) {
                        let span = self.span();
                        return self.error_at(span, "the left side of `<-` is not a pattern");
                    }
                    if self.at_kw(Kw::Then) {
                        let span = self.span();
                        return self.error_at(span, "`then` belongs to a hold: `$x <- start then step`");
                    }
                    StmtKind::Expr(e)
                }
            }
        };
        let span = start.to(self.prev_span());
        Ok(Stmt { kind, span })
    }

    /// `&r <= e` and `&r += e`: a reference at the head of a statement, then a setting or
    /// accumulating operator ("`<=` is one token with two meanings, decided by statement
    /// position").
    fn update_stmt(&mut self) -> PResult<StmtKind> {
        let target = self.reference()?;
        let op = match self.kind() {
            TokenKind::Op { op, .. } => *op,
            _ => return Err(()),
        };
        let acc = match op {
            Op::Le => None,
            Op::PlusEq => Some(AccumOp::Add),
            Op::MinusEq => Some(AccumOp::Sub),
            Op::StarEq => Some(AccumOp::Mul),
            Op::SlashEq => Some(AccumOp::Div),
            _ => return Err(()),
        };
        self.advance();
        let value = self.rhs_expr()?;
        Ok(match acc {
            None => StmtKind::Set { target, value },
            Some(op) => StmtKind::Accum { target, op, value },
        })
    }

    /// `ref ::= "&_" lens* | "&" lname lens* | "&" uname ref` (Appendix A.6).
    fn reference(&mut self) -> PResult<Expr> {
        let start = self.span();
        match self.kind().clone() {
            TokenKind::Name(n) if n.sigils.first() == Some(&Sigil::Amp) => {
                let name = self.sname()?;
                if name.is_upper() {
                    // A named lens applied, `&Acc &_` (out of this compiler).
                    self.out_of_scope(name.span, "named lenses `&Home` and `Over`", format!("the named lens `{}` is out of this compiler; write its path, `&_.score.home`", name.text()));
                    let inner = self.reference()?;
                    let span = start.to(inner.span);
                    let callee = Expr { kind: ExprKind::Name(name), span: start };
                    return Ok(Expr { kind: ExprKind::Call { callee: Box::new(callee), args: vec![Arg { name: None, span: inner.span, value: inner }], parens: false, rest: false }, span });
                }
                let mut e = Expr { span: name.span, kind: ExprKind::Name(name) };
                loop {
                    match self.kind() {
                        TokenKind::Dot { prefix: false } => {
                            self.advance();
                            match self.kind().clone() {
                                TokenKind::Name(f) if f.sigils.is_empty() && f.case == NameCase::Lower => {
                                    let t = self.advance();
                                    let span = e.span.to(t.span);
                                    e = Expr { kind: ExprKind::Field { recv: Box::new(e), name: Ident { name: f.path[0].clone(), span: t.span } }, span };
                                }
                                TokenKind::Kw(Kw::Key, _) => {
                                    let t = self.advance();
                                    let span = e.span.to(t.span);
                                    e = Expr { kind: ExprKind::Field { recv: Box::new(e), name: Ident { name: "key".into(), span: t.span } }, span };
                                }
                                _ => return Err(()),
                            }
                        }
                        TokenKind::Open { kind: BracketKind::Square, prefix: false } if self.peek().touches_prev => {
                            self.advance();
                            let index = self.expr()?;
                            self.expect_close(BracketKind::Square)?;
                            let span = e.span.to(self.prev_span());
                            e = Expr { kind: ExprKind::Index { recv: Box::new(e), index: Box::new(index) }, span };
                        }
                        _ => break,
                    }
                }
                Ok(e)
            }
            _ => Err(()),
        }
    }

    /// Names whose use is out of this compiler (decisions.md, "Features in and out").
    pub(crate) fn check_name_scope(&mut self, name: &SName) {
        let text = name.text();
        let feature = match text.as_str() {
            "!Align" => Some("`!Align`"),
            "@Collect" | "@View" | "@Edit" | "@Translate" => Some("POD hosting (`@Collect`, `@View`, `@Edit`, `@Translate`)"),
            "Over" => Some("named lenses `&Home` and `Over`"),
            _ => None,
        };
        if let Some(f) = feature {
            self.out_of_scope(name.span, f, format!("`{text}` is out of this compiler"));
        }
    }
}

fn op_to_binop(op: Op) -> Option<BinOp> {
    Some(match op {
        Op::Merge => BinOp::Merge,
        Op::EqEq => BinOp::Eq,
        Op::NotEq => BinOp::NotEq,
        Op::Lt => BinOp::Lt,
        Op::Le => BinOp::Le,
        Op::Gt => BinOp::Gt,
        Op::Ge => BinOp::Ge,
        Op::Add => BinOp::Add,
        Op::Sub => BinOp::Sub,
        Op::AddQ => BinOp::AddQ,
        Op::SubQ => BinOp::SubQ,
        Op::Mul => BinOp::Mul,
        Op::Div => BinOp::Div,
        Op::MulQ => BinOp::MulQ,
        _ => return None,
    })
}
