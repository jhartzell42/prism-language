//! `prism_syntax`: the lexer, parser, AST and printer of the Prism language.
//!
//! The specification is the Prism books (`prism-notes/claude/prism/`): Semantics chapter 2
//! (lexical structure), chapters 3 to 6 and Appendix A (syntax and grammar), Templates
//! chapter 2 (code templates), and Conformance chapter 1 and Appendix A (the test
//! language). Doc comments cite the rule each decision implements. Where the books are
//! ambiguous this crate picks, and `API.md` lists every pick.
//!
//! The entry points:
//!
//! - [`lex`]: a file to tokens, layout included;
//! - [`parse_module`]: a file to a [`ast::Module`] and its diagnostics;
//! - [`parse_test_file`]: the same, with the file's `test` and `check` declarations
//!   gathered;
//! - [`parse_fragment`]: a run of statements, as the books' snippets are written;
//! - [`pretty::module`]: a tree back to source that parses to an equal tree.

pub mod ast;
pub mod corpus;
pub mod diag;
pub mod features;
pub mod lexer;
pub mod parser;
pub mod pretty;
pub mod sexp;
pub mod span;
pub mod token;

pub use diag::{Diagnostic, Stage};
pub use lexer::{LexOutput, lex};
pub use parser::{Parsed, TestFile, parse_fragment, parse_module, parse_test_file};
pub use span::{LineCol, LineIndex, Span};
