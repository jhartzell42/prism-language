//! The test language: `test` and `check` declarations (Conformance chapter 1 and
//! Appendix A).
//!
//! The lexer already did the test frame's lexing: `=>` is one token inside `test` and
//! `check` blocks, a path after `at` is one token, and a more-indented line beginning
//! with `=>` opened the step's checks (A.1, A.2). What is left here is position: "Position
//! decides a step word, never a reserved list: the first word of a step line, the word
//! after a delivery's comma, `at` after a delivery word, `seed` after the test's
//! blueprint." And the two kinds of comma (A.3): a comma followed by a delivery word
//! begins a delivery; the first comma after a `send` target separates the value; any
//! other comma at depth zero is an error asking for parentheses. Inside a step,
//! [`Parser::step_mode`] keeps every comma away from paren-less calls.

use super::*;

const DELIVERY_WORDS: [&str; 3] = ["send", "click", "answer"];

impl Parser {
    fn step_word(&self) -> Option<String> {
        match self.kind() {
            TokenKind::Name(n) if n.sigils.is_empty() && n.path.len() == 1 && n.case == NameCase::Lower => Some(n.path[0].clone()),
            _ => None,
        }
    }

    fn at_word(&self, w: &str) -> bool {
        self.step_word().as_deref() == Some(w)
    }

    /// `check name:` and lines `expr => pattern` (or a bare expression, short for
    /// `=> True`, doubt CK).
    pub(crate) fn check_decl(&mut self) -> PResult<CheckDecl> {
        let start = self.advance().span;
        let name = self.ident()?;
        self.expect_colon()?;
        if !self.at_layout(Layout::Open) {
            return self.error_here("a `check` block's lines stand in an indented block");
        }
        self.advance();
        let mut lines = Vec::new();
        self.layout_items(|p| {
            let lstart = p.span();
            let expr = p.rhs_expr()?;
            let pattern = if p.eat_op(Op::FatArrow) { Some(p.pattern(Ctx::TOP)?) } else { None };
            lines.push(CheckLine { expr, pattern, span: lstart.to(p.prev_span()) });
            Ok(())
        });
        Ok(CheckDecl { name, lines, span: start.to(self.prev_span()) })
    }

    /// `test name, @blueprint (, seed n)?:` and its steps (A.4, `testdecl`).
    pub(crate) fn test_decl(&mut self) -> PResult<TestDecl> {
        let start = self.advance().span;
        let name = self.ident()?;
        if !self.at_comma() {
            return self.error_here("a test names its blueprint: `test name, @blueprint:`");
        }
        self.advance();
        let blueprint = self.expr_ctx(Ctx::ARG)?;
        let mut seed = None;
        if self.at_comma() {
            self.advance();
            if !self.at_word("seed") {
                return self.error_here("after the blueprint, a test takes only `seed n`");
            }
            self.advance();
            match self.kind().clone() {
                TokenKind::Int { num, sigils } if sigils.is_empty() => {
                    self.advance();
                    seed = Some(parse_nat(&num.text));
                }
                _ => return self.error_here("`seed` takes a number"),
            }
        }
        self.expect_colon()?;
        if !self.at_layout(Layout::Open) {
            return self.error_here("a test's steps stand in an indented block");
        }
        self.advance();
        let mut steps = Vec::new();
        let saved = self.step_mode;
        self.layout_items(|p| {
            p.step_mode = true;
            let r = p.step();
            p.step_mode = saved;
            steps.push(r?);
            Ok(())
        });
        self.step_mode = saved;
        Ok(TestDecl { name, blueprint, seed, steps, span: start.to(self.prev_span()) })
    }

    /// `step ::= line (OPEN check (SEP check)* CLOSE)?`.
    fn step(&mut self) -> PResult<Step> {
        let start = self.span();
        let mut withs = Vec::new();
        while self.at_word("with") {
            self.advance();
            withs.push(self.entry()?);
            if self.at_comma() {
                self.advance();
            } else {
                break;
            }
        }
        let action = match self.step_word().as_deref() {
            Some("build") => {
                self.advance();
                StepAction::Build
            }
            Some("tick") => {
                self.advance();
                StepAction::Tick
            }
            Some("compute") => {
                self.advance();
                // "`until idle` after `compute`".
                let until_idle = if self.at_word("until") {
                    self.advance();
                    if !self.at_word("idle") {
                        return self.error_here("`compute until` is followed by `idle`");
                    }
                    self.advance();
                    true
                } else {
                    false
                };
                StepAction::Compute { until_idle }
            }
            Some("push") => {
                self.advance();
                StepAction::Push(self.entry()?)
            }
            Some(w) if DELIVERY_WORDS.contains(&w) => {
                let mut ds = vec![self.delivery()?];
                while self.at_comma() {
                    if matches!(self.kind_at(1), TokenKind::Name(n) if n.path.len() == 1 && DELIVERY_WORDS.contains(&n.path[0].as_str())) {
                        self.advance();
                        ds.push(self.delivery()?);
                    } else {
                        let span = self.span();
                        return self.error_at(span, "a comma in a step begins a delivery (`click p, click q`); a value with commas in it is written in parentheses: `send p, (Min a, b)`");
                    }
                }
                StepAction::Deliver(ds)
            }
            _ if !withs.is_empty() && (self.at_layout(Layout::Open) || self.at_stmt_boundary()) => StepAction::WithOnly,
            _ => return self.error_here("expected a step: `build`, `tick`, `compute`, `send`, `click`, `answer`, `with` or `push`"),
        };
        let mut checks = Vec::new();
        if self.at_layout(Layout::Open) {
            self.advance();
            self.layout_items(|p| {
                checks.push(p.test_check()?);
                Ok(())
            });
        } else if self.at_comma() {
            let span = self.span();
            return self.error_at(span, "a comma in a step begins a delivery; a value with commas in it is written in parentheses");
        }
        Ok(Step { withs, action, checks, span: start.to(self.prev_span()) })
    }

    /// `element = value`, for `with` and `push`.
    fn entry(&mut self) -> PResult<Entry> {
        let start = self.span();
        let element = self.expr_ctx(Ctx::ARG)?;
        self.expect_op(Op::Eq)?;
        let value = self.expr_ctx(Ctx::ARG)?;
        Ok(Entry { element, value, span: start.to(self.prev_span()) })
    }

    /// `send target, v`, `click target`, `answer target`.
    fn delivery(&mut self) -> PResult<Delivery> {
        let start = self.span();
        let word = self.step_word().unwrap_or_default();
        self.advance();
        let target = self.target()?;
        let kind = match word.as_str() {
            "send" => {
                if !self.at_comma() {
                    return self.error_here("`send` takes a target and a value: `send p, v`");
                }
                self.advance();
                DeliveryKind::Send(self.expr_ctx(Ctx::ARG)?)
            }
            "click" => DeliveryKind::Click,
            _ => DeliveryKind::Answer,
        };
        Ok(Delivery { kind, target, span: start.to(self.prev_span()) })
    }

    fn target(&mut self) -> PResult<Target> {
        let element = self.at_word("element") && matches!(self.kind_at(1), TokenKind::Path(_));
        if self.at_word("at") || element {
            self.advance();
            if let TokenKind::Path(segs) = self.kind().clone() {
                let span = self.advance().span;
                let mut segments = Vec::new();
                for s in segs {
                    segments.push(match s {
                        PathSegTok::Num(n) => PathSeg::Num(n),
                        PathSegTok::Gen(n) => PathSeg::Gen(n),
                        PathSegTok::Key(toks) => PathSeg::Key(self.nested_expr(toks, span)?),
                    });
                }
                let path = Path { segments, span };
                return Ok(if element { Target::Element(path) } else { Target::Path(path) });
            }
            return self.error_here("`at` takes a path, `1/#0/0`");
        }
        Ok(Target::Pattern(self.pattern(Ctx::ARG)?))
    }

    /// One `=> check` line (A.4, `check`).
    fn test_check(&mut self) -> PResult<TestCheck> {
        let start = self.span();
        self.expect_op(Op::FatArrow)?;
        let Some(word) = self.step_word() else {
            return self.error_here("expected a check after `=>`: `expect`, `returns`, `fired`, `quiet`, ...");
        };
        self.advance();
        let kind = match word.as_str() {
            "expect" => TestCheckKind::Expect(self.pattern(Ctx::ARG)?),
            "expect_none" => TestCheckKind::ExpectNone(self.pattern(Ctx::ARG)?),
            "expect_top" => TestCheckKind::ExpectTop(self.pattern(Ctx::ARG)?),
            "expect_top_none" => TestCheckKind::ExpectTopNone(self.pattern(Ctx::ARG)?),
            "expect_count" => {
                let p = self.pattern(Ctx::ARG)?;
                if !self.at_comma() {
                    return self.error_here("`expect_count` takes a pattern and a count: `expect_count p, n`");
                }
                self.advance();
                match self.kind().clone() {
                    TokenKind::Int { num, sigils } if sigils.is_empty() => {
                        self.advance();
                        TestCheckKind::ExpectCount(p, parse_nat(&num.text))
                    }
                    _ => return self.error_here("`expect_count` takes a number"),
                }
            }
            "requested" => {
                let p = self.pattern(Ctx::ARG)?;
                if !self.at_comma() {
                    return self.error_here("`requested` takes a place and a value: `requested p, v`");
                }
                self.advance();
                TestCheckKind::Requested(p, self.pattern(Ctx::ARG)?)
            }
            "unrequested" => TestCheckKind::Unrequested(self.pattern(Ctx::ARG)?),
            "returns" => TestCheckKind::Returns(self.pattern(Ctx::ARG)?),
            "fired" => TestCheckKind::Fired(self.pattern(Ctx::ARG)?),
            "quiet" => TestCheckKind::Quiet,
            "settles" => TestCheckKind::Settles,
            "aborted" => TestCheckKind::Aborted,
            "snapshot" => match self.kind().clone() {
                TokenKind::Text { text, sigils } if sigils.is_empty() && text.segments.iter().all(|s| matches!(s, TextSegTok::Plain(_))) => {
                    self.advance();
                    let s: String = text.segments.iter().map(|s| if let TextSegTok::Plain(p) = s { p.as_str() } else { "" }).collect();
                    TestCheckKind::Snapshot(s)
                }
                _ => return self.error_here("`snapshot` takes a file name, a text literal"),
            },
            other => {
                return self.error_at(self.prev_span(), format!("`{other}` is not a check: the checks are expect, expect_none, expect_count, expect_top, expect_top_none, requested, unrequested, returns, fired, quiet, settles, aborted, snapshot"));
            }
        };
        Ok(TestCheck { kind, span: start.to(self.prev_span()) })
    }
}

fn parse_nat(text: &str) -> u64 {
    let t: String = text.chars().filter(|c| *c != '_').collect();
    if let Some(h) = t.strip_prefix("0x") {
        u64::from_str_radix(h, 16).unwrap_or(0)
    } else if let Some(o) = t.strip_prefix("0o") {
        u64::from_str_radix(o, 8).unwrap_or(0)
    } else if let Some(b) = t.strip_prefix("0b") {
        u64::from_str_radix(b, 2).unwrap_or(0)
    } else {
        t.parse().unwrap_or(0)
    }
}
