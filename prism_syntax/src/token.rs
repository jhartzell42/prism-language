//! Tokens: what the lexer hands the parser.
//!
//! The token forms follow Semantics chapter 2 and Appendix A.2:
//!
//! - a name is one token with its attached sigils, outermost first (ch. 2, "The attachment
//!   rule"; "A stack is a run of attached sigils and a name, one token");
//! - a qualified name `Ui.Button` is one token (ch. 2, "Qualified names and families");
//! - every bracket is its own one-character token, marked with whether it touches the
//!   token before (ch. 2, "The bracket pairing rule": "It never emits `((` or `))`");
//! - a text literal is one token holding its plain segments and each placeholder's token
//!   stream (ch. 2, Text);
//! - a template is one token holding its text runs, placeholders, brace blocks and calls
//!   (ch. 2, "The template frame"; Templates ch. 2, Lexing);
//! - layout is three tokens, `open`, `sep` and `close` (ch. 2, Layout).

use crate::span::Span;
use std::fmt;

/// The six sigils (Semantics ch. 2, "The attachment rule"; round 48 LN).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sigil {
    /// `$` a dynamic: always there, changing.
    Dollar,
    /// `!` an event: sometimes happens.
    Bang,
    /// `@` a widget: built later.
    At,
    /// `?` an optional: maybe.
    Question,
    /// `~` a fault: worked, or an `Error`.
    Tilde,
    /// `&` a reference, in an `update` block.
    Amp,
}

impl Sigil {
    pub fn from_char(c: char) -> Option<Sigil> {
        Some(match c {
            '$' => Sigil::Dollar,
            '!' => Sigil::Bang,
            '@' => Sigil::At,
            '?' => Sigil::Question,
            '~' => Sigil::Tilde,
            '&' => Sigil::Amp,
            _ => return None,
        })
    }

    pub fn as_char(self) -> char {
        match self {
            Sigil::Dollar => '$',
            Sigil::Bang => '!',
            Sigil::At => '@',
            Sigil::Question => '?',
            Sigil::Tilde => '~',
            Sigil::Amp => '&',
        }
    }
}

/// Renders a sigil stack, outermost first: `@$?`.
pub fn sigils_str(sigils: &[Sigil]) -> String {
    sigils.iter().map(|s| s.as_char()).collect()
}

/// The case mark of a name (Semantics ch. 2, "Case is a mark").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NameCase {
    /// Begins with a lowercase letter or `_` and a letter: a value.
    Lower,
    /// Begins with an uppercase letter: a function, constructor, type or trait.
    Upper,
    /// `_` alone: the topic.
    Topic,
    /// `_` and digits: a hole, legal only in an `impl` header.
    Hole,
    /// `templatetext` or `templateplaceholder`: lowercase names the prelude reserves as
    /// constructors (Jimmy, round 42; round 56 FK-CS). The lexer reads them as lowercase;
    /// this mark lets the parser treat them as constructors.
    ReservedCtor,
}

/// A name token: attached sigils, then one or more `.`-joined segments.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NameTok {
    pub sigils: Vec<Sigil>,
    /// `["Ui", "Button"]` for `Ui.Button`; one segment for a plain name.
    pub path: Vec<String>,
    /// The case of the last segment.
    pub case: NameCase,
}

impl NameTok {
    pub fn text(&self) -> String {
        format!("{}{}", sigils_str(&self.sigils), self.path.join("."))
    }
}

/// Keywords, reserved everywhere (Semantics ch. 2, Keywords; Appendix A.2, which adds
/// `unbounded`). No word is a keyword in one position and a name in another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kw {
    // declarations
    Module, Emit, Use, Type, Record, Choice, Row, Tag, Of, Derive, Trait, Impl, Where, Any,
    Let, Fn, Test, Check,
    // blocks
    Widget, Rebuild, For, Given, Monad, Update, While, Match, If, Else, Otherwise,
    // inside blocks
    Require, Forbid, Return, Ignore,
    // operators
    Then, On, Not, And, Or, Mod, As,
    // templates
    Output, InlineText, Raw,
    // colors
    Unbounded,
    // values
    Unit, Nil,
    // bound names
    Key, Ix, Jx, Kx,
}

impl Kw {
    pub fn from_word(w: &str) -> Option<Kw> {
        use Kw::*;
        Some(match w {
            "module" => Module, "emit" => Emit, "use" => Use, "type" => Type,
            "record" => Record, "choice" => Choice, "row" => Row, "tag" => Tag, "of" => Of,
            "derive" => Derive, "trait" => Trait, "impl" => Impl, "where" => Where,
            "any" => Any, "let" => Let, "fn" => Fn, "test" => Test, "check" => Check,
            "widget" => Widget, "rebuild" => Rebuild, "for" => For, "given" => Given,
            "monad" => Monad, "update" => Update, "while" => While, "match" => Match,
            "if" => If, "else" => Else, "otherwise" => Otherwise,
            "require" => Require, "forbid" => Forbid, "return" => Return, "ignore" => Ignore,
            "then" => Then, "on" => On, "not" => Not, "and" => And, "or" => Or,
            "mod" => Mod, "as" => As,
            "output" => Output, "inline_text" => InlineText, "raw" => Raw,
            "unbounded" => Unbounded,
            "unit" => Unit, "nil" => Nil,
            "key" => Key, "ix" => Ix, "jx" => Jx, "kx" => Kx,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        use Kw::*;
        match self {
            Module => "module", Emit => "emit", Use => "use", Type => "type",
            Record => "record", Choice => "choice", Row => "row", Tag => "tag", Of => "of",
            Derive => "derive", Trait => "trait", Impl => "impl", Where => "where",
            Any => "any", Let => "let", Fn => "fn", Test => "test", Check => "check",
            Widget => "widget", Rebuild => "rebuild", For => "for", Given => "given",
            Monad => "monad", Update => "update", While => "while", Match => "match",
            If => "if", Else => "else", Otherwise => "otherwise",
            Require => "require", Forbid => "forbid", Return => "return", Ignore => "ignore",
            Then => "then", On => "on", Not => "not", And => "and", Or => "or",
            Mod => "mod", As => "as",
            Output => "output", InlineText => "inline_text", Raw => "raw",
            Unbounded => "unbounded",
            Unit => "unit", Nil => "nil",
            Key => "key", Ix => "ix", Jx => "jx", Kx => "kx",
        }
    }
}

/// Symbol operators and arrows, each one token by longest match (Semantics ch. 2,
/// "Operators and punctuation").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Op {
    /// `<-` bind, emit, build or hold.
    BindArrow,
    /// `=` for `let`, `fn` and named arguments.
    Eq,
    /// `<=`: sets at the head of an `update` statement, compares elsewhere.
    Le,
    PlusEq, MinusEq, StarEq, SlashEq,
    /// `<|` the leftmost merge (round 46 MO).
    Merge,
    EqEq, NotEq, Lt, Gt, Ge,
    Add, Sub, Mul, Div,
    /// The checked operators `+?`, `-?`, `*?` (rounds 47 NS, OV).
    AddQ, SubQ, MulQ,
    /// `->` in function types.
    Arrow,
    /// `==>` the map type.
    MapArrow,
    /// `=>` exists only in `test` and `check` blocks (Conformance Appendix A.2).
    FatArrow,
}

impl Op {
    pub fn as_str(self) -> &'static str {
        use Op::*;
        match self {
            BindArrow => "<-", Eq => "=", Le => "<=", PlusEq => "+=", MinusEq => "-=",
            StarEq => "*=", SlashEq => "/=", Merge => "<|", EqEq => "==", NotEq => "!=",
            Lt => "<", Gt => ">", Ge => ">=", Add => "+", Sub => "-", Mul => "*",
            Div => "/", AddQ => "+?", SubQ => "-?", MulQ => "*?", Arrow => "->",
            MapArrow => "==>", FatArrow => "=>",
        }
    }

    /// Operators that take a type suffix when an uppercase name touches them
    /// (ch. 2, "Suffixed operators": "a symbol operator followed, with no space, by an
    /// uppercase name"). Arrows and statement operators are not value operators.
    pub fn takes_suffix(self) -> bool {
        use Op::*;
        matches!(self, Le | EqEq | NotEq | Lt | Gt | Ge | Add | Sub | Mul | Div | AddQ | SubQ | MulQ)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BracketKind {
    Paren,
    Square,
    Brace,
}

impl BracketKind {
    pub fn open_char(self) -> char {
        match self {
            BracketKind::Paren => '(',
            BracketKind::Square => '[',
            BracketKind::Brace => '{',
        }
    }
    pub fn close_char(self) -> char {
        match self {
            BracketKind::Paren => ')',
            BracketKind::Square => ']',
            BracketKind::Brace => '}',
        }
    }
}

/// The layout tokens (Semantics ch. 2, Layout; Appendix A.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Layout {
    Open,
    Sep,
    Close,
}

/// A text literal: plain segments and placeholders (Semantics ch. 2, Text).
#[derive(Clone, Debug, PartialEq)]
pub struct TextTok {
    pub segments: Vec<TextSegTok>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TextSegTok {
    /// Plain text, escapes already decoded.
    Plain(String),
    /// A placeholder's tokens, read with layout suspended up to the matching brace.
    Placeholder(Vec<Token>, Span),
}

/// The keyword that reaches a template (Templates ch. 1, "From Prism into a document").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TemplateKw {
    Output,
    InlineText,
    Raw,
    /// The tag was the file's first token: the file is a template (Templates ch. 1).
    File,
}

/// Whether a language's delimiters are single (`markdown`, `html`) or doubled
/// (every other language), by Semantics ch. 2's closed list (Q-CT1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Delims {
    Single,
    Double,
}

impl Delims {
    /// Q-CT1 (a): a closed list of single-brace languages; an unknown language is assumed
    /// to own its braces.
    pub fn for_language(lang: &str) -> Delims {
        match lang {
            "markdown" | "html" => Delims::Single,
            _ => Delims::Double,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TemplateTok {
    pub keyword: TemplateKw,
    pub lang: String,
    pub delims: Delims,
    pub parts: Vec<TemplatePartTok>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TemplatePartTok {
    /// A run of text, after the whitespace rules of Templates ch. 2 have been applied.
    Text(String),
    /// `{{expr}}` (or `{expr}`): the expression's tokens; `column` is present when the
    /// placeholder stands alone on its line (Templates ch. 2, "Whitespace and escapes").
    Placeholder { tokens: Vec<Token>, column: Option<u32>, span: Span },
    /// `{{` alone on a line through `}}` alone on a line: statements, layout resumed.
    Block { tokens: Vec<Token>, span: Span },
    /// `<@Name attr=value ...>` with a body, or `<@Name .../>` without.
    Call { name: NameTok, attrs: Vec<TemplateAttrTok>, body: Option<Vec<TemplatePartTok>>, span: Span },
}

#[derive(Clone, Debug, PartialEq)]
pub struct TemplateAttrTok {
    pub name: String,
    /// The value's tokens: a placeholder's contents, or a single text-literal token.
    pub tokens: Vec<Token>,
    pub span: Span,
}

/// A segment of a test path, `1/#0/[k]` (Conformance Appendix A.2).
#[derive(Clone, Debug, PartialEq)]
pub enum PathSegTok {
    Num(u64),
    Gen(u64),
    Key(Vec<Token>),
}

/// A number literal's spelling. The text is kept as written (underscores included) so a
/// printer gives back what the program said; `value` has the underscores removed and no
/// radix prefix.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NumTok {
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    /// A name with its attached sigils: `x`, `$count`, `@$?draft`, `Ui.Button`, `_`, `$_`.
    Name(NameTok),
    /// `Ui.*`: every tag of a family; the only place `*` touches a `.`.
    FamilyStar(Vec<String>),
    /// `^Ord`: a trait, `^` being a mark that attaches by the sigil rule.
    Trait(Vec<String>),
    /// A keyword, with any sigils attached to it (`@widget`, `@unit`, `!unit`).
    Kw(Kw, Vec<Sigil>),
    /// A sigil with whitespace or a bracket after it: a prefix over the rest of its
    /// expression (ch. 2, "A free sigil takes the rest of its expression").
    FreeSigil(Sigil),
    /// `&` touching `.`: the start of a lens value, `&.score.home`.
    LensDot,
    /// Number literals. The sigils are marks on the literal (ch. 2, "Marks on literals").
    Int { sigils: Vec<Sigil>, num: NumTok },
    Float { sigils: Vec<Sigil>, num: NumTok },
    /// A duration: digits, at most one point, and a unit, one token.
    Duration { sigils: Vec<Sigil>, num: NumTok, unit: String },
    Char { sigils: Vec<Sigil>, ch: char },
    Text { sigils: Vec<Sigil>, text: TextTok },
    Template(Box<TemplateTok>),
    /// A symbol operator. `prefix` is the spacing rule's answer for `+` and `-` (and for
    /// `-?`, which doubt MQ's soft error needs), and false for every other operator.
    Op { op: Op, prefix: bool },
    /// An operator at a type: `>Int`, `+Float`, `-?U8` (out of this compiler).
    SuffixedOp { op: Op, ty: Vec<String> },
    /// `.`: `prefix` is the spacing rule's answer (a prefix `.f` applies to the topic).
    Dot { prefix: bool },
    DotDot,
    Comma,
    Colon,
    Semi,
    Backslash,
    /// A lambda's bar: `open` right after `\`, closing at the next `|` on the line.
    LambdaBar { open: bool },
    /// `|` between the families of a row, in a type or an `emit` line.
    Bar,
    /// `<<` touching a name: explicit type arguments; `>>` closes them.
    LtLt,
    GtGt,
    /// One bracket. `prefix` is the spacing rule's answer for a single `[`.
    Open { kind: BracketKind, prefix: bool },
    Close { kind: BracketKind },
    /// `-` with a comma or a close bracket directly after it: an absent component.
    Absent,
    /// `+` touching a closing double bracket or brace: the non-empty mark.
    NonEmpty,
    /// A path after `at` in a test step.
    Path(Vec<PathSegTok>),
    Layout(Layout),
    Eof,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    /// No whitespace between this token and the one before it on the same line.
    pub touches_prev: bool,
    /// The first token of a continuation line: the line is indented more than its block,
    /// so layout emitted nothing before it (rule 4). Rule 9's `.Method` needs this, and an
    /// inline block ends at the end of its line (rule 5).
    pub line_start: bool,
}

impl Token {
    pub fn is_layout(&self) -> bool {
        matches!(self.kind, TokenKind::Layout(_))
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&crate::corpus::token_text(self))
    }
}
