//! Diagnostics: lexical errors, parse errors, and the out-of-scope features.
//!
//! Every diagnostic has a span and a message in the books' spellings. A diagnostic for a
//! feature that `docs/decisions.md` ("Features in and out") puts out of this compiler
//! names the feature in [`Diagnostic::feature`], so a caller can tell "you wrote it wrong"
//! from "this compiler does not do that yet".

use crate::span::{LineIndex, Span};
use std::fmt;

/// Which stage produced a diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Lex,
    Parse,
    /// A construct that parses but is out of this compiler (decisions.md, Features in and out).
    OutOfScope,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub stage: Stage,
    pub span: Span,
    pub message: String,
    /// For [`Stage::OutOfScope`]: the feature's name as decisions.md's list spells it.
    pub feature: Option<&'static str>,
}

impl Diagnostic {
    pub fn lex(span: Span, message: impl Into<String>) -> Diagnostic {
        Diagnostic { stage: Stage::Lex, span, message: message.into(), feature: None }
    }

    pub fn parse(span: Span, message: impl Into<String>) -> Diagnostic {
        Diagnostic { stage: Stage::Parse, span, message: message.into(), feature: None }
    }

    pub fn out_of_scope(span: Span, feature: &'static str, message: impl Into<String>) -> Diagnostic {
        Diagnostic { stage: Stage::OutOfScope, span, message: message.into(), feature: Some(feature) }
    }

    /// `line:col: message`, one-based.
    pub fn render(&self, index: &LineIndex<'_>) -> String {
        let at = index.line_col(self.span.start);
        match self.feature {
            Some(f) => format!("{at}: out of scope ({f}): {}", self.message),
            None => format!("{at}: {}", self.message),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} at {:?}: {}", self.stage, self.span, self.message)
    }
}

/// Renders every diagnostic against its source, one per line.
pub fn render_all(src: &str, diags: &[Diagnostic]) -> String {
    let index = LineIndex::new(src);
    diags.iter().map(|d| d.render(&index)).collect::<Vec<_>>().join("\n")
}
