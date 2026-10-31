//! The layout corpus's notation (Semantics ch. 2, "The layout corpus, sampled"):
//!
//! > Attached sigils are part of their tokens; `free@` is a free sigil; `TEXT(...)` a
//! > text literal with a placeholder's tokens in braces; `-pre` a prefix sign; `absent`
//! > and `nonempty` the two marks; each bracket one token, and `·` stands between two
//! > brackets that touch.
//!
//! The layout tokens print as `open`, `sep` and `close`, each beginning a new line, as
//! the book's sample breaks its lines at `sep`. Tokens are separated by one space, except
//! that `·` replaces the space between two brackets that touch.
//!
//! Two extensions, for tokens the book's sample does not show: a prefix `.` (the topic's
//! field, by the spacing rule) prints as `.pre`, and a `+` sign as `+pre`; a test path
//! prints as `PATH(1/#0/0)` and a template as `TEMPLATE<lang>(...)` with its parts.

use crate::token::*;
#[allow(unused_imports)]
use crate::token::TemplateKw;

/// Prints a token stream in the corpus notation.
pub fn render(tokens: &[Token]) -> String {
    let mut out = String::new();
    let mut prev: Option<&Token> = None;
    for t in tokens {
        if matches!(t.kind, TokenKind::Eof) {
            break;
        }
        let text = token_text(&t.kind);
        match prev {
            None => {}
            Some(p) => {
                if t.is_layout() {
                    out.push('\n');
                } else if is_bracket(&p.kind) && is_bracket(&t.kind) && t.touches_prev {
                    out.push('·');
                } else if p.is_layout() && matches!(p.kind, TokenKind::Layout(_)) {
                    out.push(' ');
                } else {
                    out.push(' ');
                }
            }
        }
        out.push_str(&text);
        prev = Some(t);
    }
    out.push('\n');
    out
}

fn is_bracket(k: &TokenKind) -> bool {
    matches!(k, TokenKind::Open { .. } | TokenKind::Close { .. })
}

fn render_inner(tokens: &[Token]) -> String {
    let r = render(tokens);
    r.trim_end().replace('\n', " ")
}

fn lit_text(sigils: &[Sigil], s: &str) -> String {
    format!("{}{}", sigils_str(sigils), s)
}

/// One token's text in the corpus notation.
pub fn token_text(k: &TokenKind) -> String {
    match k {
        TokenKind::Name(n) => n.text(),
        TokenKind::FamilyStar(p) => format!("{}.*", p.join(".")),
        TokenKind::Trait(p) => format!("^{}", p.join(".")),
        TokenKind::Kw(kw, s) => lit_text(s, kw.as_str()),
        TokenKind::FreeSigil(s) => format!("free{}", s.as_char()),
        TokenKind::LensDot => "&.".to_string(),
        TokenKind::Int { sigils, num } | TokenKind::Float { sigils, num } => lit_text(sigils, &num.text),
        TokenKind::Duration { sigils, num, unit } => lit_text(sigils, &format!("{}{}", num.text, unit)),
        TokenKind::Char { sigils, ch } => lit_text(sigils, &format!("{:?}", ch)),
        TokenKind::Text { sigils, text } => {
            let mut s = String::new();
            for seg in &text.segments {
                match seg {
                    TextSegTok::Plain(p) => s.push_str(&p.escape_debug().to_string().replace('{', "\\{").replace('}', "\\}")),
                    TextSegTok::Placeholder(toks, _) => {
                        s.push('{');
                        s.push_str(&render_inner(toks));
                        s.push('}');
                    }
                }
            }
            format!("{}TEXT({})", sigils_str(sigils), s)
        }
        TokenKind::Template(t) => {
            // The keyword before a template is its own token.
            let mut s = format!("TEMPLATE<{}>(", t.lang);
            render_parts(&t.parts, &mut s);
            s.push(')');
            s
        }
        TokenKind::Op { op, prefix } => {
            if *prefix { format!("{}pre", op.as_str()) } else { op.as_str().to_string() }
        }
        TokenKind::SuffixedOp { op, ty } => format!("{}{}", op.as_str(), ty.join(".")),
        TokenKind::Dot { prefix } => if *prefix { ".pre".into() } else { ".".into() },
        TokenKind::DotDot => "..".into(),
        TokenKind::Comma => ",".into(),
        TokenKind::Colon => ":".into(),
        TokenKind::Semi => ";".into(),
        TokenKind::Backslash => "\\".into(),
        TokenKind::LambdaBar { .. } => "|".into(),
        TokenKind::Bar => "|".into(),
        TokenKind::LtLt => "<<".into(),
        TokenKind::GtGt => ">>".into(),
        // The notation marks prefix signs only; a `[` that pairs into a double bracket is
        // a literal whatever its spacing, so its prefix flag is not printed.
        TokenKind::Open { kind, .. } => kind.open_char().to_string(),
        TokenKind::Close { kind } => kind.close_char().to_string(),
        TokenKind::Absent => "absent".into(),
        TokenKind::NonEmpty => "nonempty".into(),
        TokenKind::Path(segs) => {
            let v: Vec<String> = segs
                .iter()
                .map(|s| match s {
                    PathSegTok::Num(n) => n.to_string(),
                    PathSegTok::Gen(n) => format!("#{n}"),
                    PathSegTok::Key(t) => format!("[{}]", render_inner(t)),
                })
                .collect();
            format!("PATH({})", v.join("/"))
        }
        TokenKind::Layout(Layout::Open) => "open".into(),
        TokenKind::Layout(Layout::Sep) => "sep".into(),
        TokenKind::Layout(Layout::Close) => "close".into(),
        TokenKind::Eof => "eof".into(),
    }
}

fn render_parts(parts: &[TemplatePartTok], s: &mut String) {
    for p in parts {
        match p {
            TemplatePartTok::Text(t) => s.push_str(&format!("{:?}", t)),
            TemplatePartTok::Placeholder { tokens, column, .. } => {
                s.push_str("{{");
                s.push_str(&render_inner(tokens));
                s.push_str("}}");
                if let Some(c) = column {
                    s.push_str(&format!("@col{c}"));
                }
            }
            TemplatePartTok::Block { tokens, .. } => {
                s.push_str("BLOCK{ ");
                s.push_str(&render_inner(tokens));
                s.push_str(" }");
            }
            TemplatePartTok::Call { name, attrs, body, .. } => {
                s.push_str(&format!("<{}", name.text()));
                for a in attrs {
                    s.push_str(&format!(" {}={{{}}}", a.name, render_inner(&a.tokens)));
                }
                match body {
                    None => s.push_str("/>"),
                    Some(b) => {
                        s.push('>');
                        render_parts(b, s);
                        s.push_str(&format!("</{}>", name.text()));
                    }
                }
            }
        }
    }
}
