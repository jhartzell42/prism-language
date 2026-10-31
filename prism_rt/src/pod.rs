//! The `Pod` text format for plain values: printing and reading back the transcript's
//! spelling, and the patterns the test language matches with.
//!
//! The spelling is Type System ch. 2's ("Plain values, and how they print", round 47 SP),
//! cited by Conformance ch. 1 ("The transcript"):
//!
//! - text in double quotes with `\"`, `\\`, `\n`, `\t` escaped and other control
//!   characters as `\u{hex}`;
//! - integers in decimal;
//! - a float as the shortest decimal that reads back, always with a point, never an
//!   exponent; `nan`, `inf`, `-inf`;
//! - a duration in its largest whole unit (`1300us` for 1.3ms);
//! - a map as `{k: v}` in ascending key order;
//! - a record with its fields named, in declaration order; a choice as its constructor
//!   applied; a tag qualified by its family;
//! - `nil`, `? x`, `~ x`.
//!
//! The book leaves some spellings open, and this module chooses them (each listed in the
//! crate report): a tuple prints `((a, b))` and an array `[[a, b]]`, as their literals
//! are written; a `Char` prints `'c'`; `unit`, `True`, `False`; a choice with a payload
//! is parenthesized wherever it is not the whole printed value, so `? (Letter "a")` and
//! `Write "end"` both read back, except a choice of one argument as a field's value,
//! which reads to the next comma as a named argument does in Prism
//! (`Gui.Scene(window=WindowId "Chess", ..)`, `Fs.File(path=PageFile _, ..)`); a record with no fields prints `Name()`; a path value
//! prints in the path notation, `1/#0/0`.
//!
//! **A type with its own `^Show` prints by it in transcripts** (Type System ch. 2), a
//! key in a path included (`0/[e2]/0` for a `Spot` whose impl prints `e2`). The runtime
//! cannot run a program's impl, so an interpreter registers a [`ShowHook`] with
//! [`set_show_hook`]: given a record or choice value, it gives the impl's text, or `None`
//! for a type with no impl of its own (a derived `^Show` is the structural spelling
//! here). Printing with no hook registered is the structural spelling throughout, which is
//! also the saved form (`Print` is not `^Show`). A value an impl is printing is not handed
//! to the hook again while it prints, so an impl that falls back on the structural
//! spelling of its own value ends.
//!
//! Reading is the inverse of printing, without type information: `Name(f=v)` is a record,
//! `Family.Tag(f=v)` an element, `Name` or `Name a, b` a choice, a lowercase name other
//! than `nil`, `unit` and `ignore` a unit type's value, and a number followed by `/` a
//! path, a number with a point a `Float` (as `nan`, `inf` and `-inf` are). Non-negative
//! integers read as `Nat` and negative ones as `Int`; the value order
//! compares integers numerically across types, so either matches a program's integer.

use std::cell::RefCell;
use std::rc::Rc;

use num_bigint::BigInt;

use crate::value::{float_spelling, Choice, Element, Path, Record, Segment, Value};

// ---------------------------------------------------------------------------------------
// A program's `^Show`
// ---------------------------------------------------------------------------------------

/// A program's `impl ^Show`, as an interpreter registers it: the text the impl gives for
/// a record or choice value, or `None` when its type has no impl of its own.
pub type ShowHook = Rc<dyn Fn(&Value) -> Option<String>>;

thread_local! {
    static SHOW_HOOK: RefCell<Option<ShowHook>> = const { RefCell::new(None) };
    static SHOWING: RefCell<Vec<Value>> = const { RefCell::new(Vec::new()) };
}

/// Register the program's `^Show` for transcripts on this thread, or `None` to go back to
/// the structural spelling. Returns the hook it replaces.
pub fn set_show_hook(hook: Option<ShowHook>) -> Option<ShowHook> {
    SHOW_HOOK.with(|h| std::mem::replace(&mut *h.borrow_mut(), hook))
}

/// The registered hook's text for `v`, unless `v` is being shown already.
fn shown_by_impl(v: &Value) -> Option<String> {
    let hook = SHOW_HOOK.with(|h| h.borrow().clone())?;
    if SHOWING.with(|s| s.borrow().contains(v)) {
        return None;
    }
    SHOWING.with(|s| s.borrow_mut().push(v.clone()));
    let r = hook(v);
    SHOWING.with(|s| s.borrow_mut().pop());
    r
}

/// Print a value in its structural spelling, ignoring any `^Show` impl: the saved form
/// of a POD document (Type System ch. 2, "`Print` is not `^Show`").
pub fn print_structural(v: &Value) -> String {
    let hook = set_show_hook(None);
    let s = print(v);
    set_show_hook(hook);
    s
}

// ---------------------------------------------------------------------------------------
// Printing
// ---------------------------------------------------------------------------------------

/// Print a value in the transcript's spelling, as the whole of a printed value.
pub fn print(v: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, v, true);
    out
}

/// Print a path in the notation of Semantics ch. 8: positions as numbers, generations as
/// `#g`, keys as `[k]`, joined by `/`. A key that is itself a path (a host document's
/// `[p]`) prints as its notation.
pub fn print_path(p: &Path) -> String {
    let mut out = String::new();
    for (i, seg) in p.0.iter().enumerate() {
        if i > 0 {
            out.push('/');
        }
        match seg {
            Segment::Pos(n) => out.push_str(&n.to_string()),
            Segment::Gen(g) => {
                out.push('#');
                out.push_str(&g.to_string());
            }
            Segment::Key(k) => {
                out.push('[');
                write_value(&mut out, k, true);
                out.push(']');
            }
        }
    }
    out
}

fn write_text(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn write_char(out: &mut String, c: char) {
    out.push('\'');
    match c {
        '\'' => out.push_str("\\'"),
        '\\' => out.push_str("\\\\"),
        '\n' => out.push_str("\\n"),
        '\t' => out.push_str("\\t"),
        c if c.is_control() => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
        c => out.push(c),
    }
    out.push('\'');
}

/// A duration in its largest whole unit (Type System ch. 2).
fn write_duration(out: &mut String, ns: i128) {
    const UNITS: [(&str, i128); 6] = [
        ("h", 3_600_000_000_000),
        ("m", 60_000_000_000),
        ("s", 1_000_000_000),
        ("ms", 1_000_000),
        ("us", 1_000),
        ("ns", 1),
    ];
    if ns == 0 {
        out.push_str("0s");
        return;
    }
    for (name, size) in UNITS {
        if ns % size == 0 {
            out.push_str(&format!("{}{}", ns / size, name));
            return;
        }
    }
}

fn write_list<'a>(out: &mut String, items: impl Iterator<Item = &'a Value>) {
    for (i, v) in items.enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        write_value(out, v, false);
    }
}

fn write_value(out: &mut String, v: &Value, whole: bool) {
    write_at(out, v, whole, false)
}

/// `whole`: the value is all that is printed; `field`: it is a field's value, `f=v`.
fn write_at(out: &mut String, v: &Value, whole: bool, field: bool) {
    if matches!(v, Value::Record(_) | Value::Choice(_)) {
        if let Some(text) = shown_by_impl(v) {
            out.push_str(&text);
            return;
        }
    }
    match v {
        Value::Unit => out.push_str("unit"),
        Value::Bool(true) => out.push_str("True"),
        Value::Bool(false) => out.push_str("False"),
        Value::Nat(n) | Value::Int(n) => out.push_str(&n.to_string()),
        Value::U8(n) => out.push_str(&n.to_string()),
        Value::Float(f) => out.push_str(&float_spelling(*f)),
        Value::Char(c) => write_char(out, *c),
        Value::Text(t) => write_text(out, &t.to_string()),
        Value::Duration(ns) => write_duration(out, *ns),
        Value::Record(r) => {
            out.push_str(&r.name);
            out.push('(');
            write_fields(out, &r.fields);
            out.push(')');
        }
        Value::Tuple(items) => {
            out.push_str("((");
            write_list(out, items.iter());
            out.push_str("))");
        }
        Value::Choice(c) => {
            if c.payload.is_empty() {
                out.push_str(&c.ctor);
            } else {
                let bare = whole || (field && c.payload.len() == 1);
                if !bare {
                    out.push('(');
                }
                out.push_str(&c.ctor);
                out.push(' ');
                write_list(out, c.payload.iter());
                if !bare {
                    out.push(')');
                }
            }
        }
        Value::Map(m) => {
            out.push('{');
            for (i, (k, v)) in m.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write_value(out, k, false);
                out.push_str(": ");
                write_value(out, v, false);
            }
            out.push('}');
        }
        Value::Array(a) => {
            out.push_str("[[");
            write_list(out, a.iter());
            out.push_str("]]");
        }
        Value::Opt(None) => out.push_str("nil"),
        Value::Opt(Some(x)) => {
            out.push_str("? ");
            write_value(out, x, false);
        }
        Value::Fallible(Ok(x)) => {
            out.push_str("~ ");
            write_value(out, x, false);
        }
        Value::Fallible(Err(e)) => write_at(out, e, whole, field),
        Value::Element(e) => {
            out.push_str(&e.tag);
            if !e.fields.is_empty() {
                out.push('(');
                write_fields(out, &e.fields);
                out.push(')');
            }
        }
        Value::Path(p) => out.push_str(&print_path(p)),
        Value::Closure(c) => out.push_str(&format!("<closure {}>", c.name())),
        Value::Event(n) => out.push_str(&format!("<event {}>", n.0)),
        Value::Dynamic(n) => out.push_str(&format!("<dynamic {}>", n.0)),
        Value::Widget(b) => out.push_str(&format!("<widget {}>", b.name())),
        Value::Ref(_) => out.push_str("<ref>"),
    }
}

fn write_fields(out: &mut String, fields: &[(Rc<str>, Value)]) {
    for (i, (n, v)) in fields.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(n);
        out.push('=');
        write_at(out, v, false, true);
    }
}

// ---------------------------------------------------------------------------------------
// Patterns
// ---------------------------------------------------------------------------------------

/// A pattern of the test language (Conformance ch. 1, "Checks"): a Prism pattern over
/// plain values, with `ignore` for anything, `..` for the remaining fields, and `Fam.*`
/// for any element of a family.
#[derive(Clone, Debug, PartialEq)]
pub enum Pattern {
    /// `ignore`.
    Any,
    /// A literal leaf: matched by equality.
    Lit(Value),
    /// `Name(f=p, ..)`: a record.
    Record { name: String, fields: Vec<(String, Pattern)>, rest: bool },
    /// `Fam.Tag(f=p, ..)`: an element (or, with no parentheses, an element with no fields
    /// or a fault kind of that name).
    Element { tag: String, fields: Vec<(String, Pattern)>, rest: bool, bare: bool },
    /// `Fam.*`: any element of the family.
    Family(String),
    /// `Ctor p, q`.
    Choice { ctor: String, payload: Vec<Pattern> },
    Tuple(Vec<Pattern>),
    Array(Vec<Pattern>),
    Map(Vec<(Pattern, Pattern)>),
    /// `? p`.
    Some(Box<Pattern>),
    /// `~ p`.
    Ok(Box<Pattern>),
    /// A path literal.
    Path(Path),
}

impl Pattern {
    /// Whether `v` matches this pattern.
    pub fn matches(&self, v: &Value) -> bool {
        match (self, v) {
            (Pattern::Any, _) => true,
            (Pattern::Lit(l), v) => l == v,
            (Pattern::Path(p), Value::Path(q)) => print_path(p) == print_path(q),
            (Pattern::Record { name, fields, rest }, Value::Record(r)) => {
                &*r.name == name && fields_match(fields, *rest, &r.fields)
            }
            // `Error(kind=..)` matches a failed `~T`.
            (Pattern::Record { .. }, Value::Fallible(Err(e))) => self.matches(e),
            (Pattern::Element { tag, fields, rest, bare }, Value::Element(e)) => {
                &*e.tag == tag && (if *bare { e.fields.is_empty() } else { fields_match(fields, *rest, &e.fields) })
            }
            (Pattern::Element { tag, bare: true, .. }, Value::Choice(c)) => {
                &*c.ctor == tag && c.payload.is_empty()
            }
            (Pattern::Family(f), Value::Element(e)) => e.family() == f,
            (Pattern::Choice { ctor, payload }, Value::Choice(c)) => {
                &*c.ctor == ctor
                    && payload.len() == c.payload.len()
                    && payload.iter().zip(c.payload.iter()).all(|(p, v)| p.matches(v))
            }
            (Pattern::Tuple(ps), Value::Tuple(vs)) => {
                ps.len() == vs.len() && ps.iter().zip(vs.iter()).all(|(p, v)| p.matches(v))
            }
            (Pattern::Array(ps), Value::Array(vs)) => {
                ps.len() == vs.len() && ps.iter().zip(vs.iter()).all(|(p, v)| p.matches(v))
            }
            (Pattern::Map(ps), Value::Map(m)) => {
                ps.len() == m.len()
                    && ps.iter().zip(m.iter()).all(|((pk, pv), (k, v))| pk.matches(k) && pv.matches(v))
            }
            (Pattern::Some(p), Value::Opt(Some(v))) => p.matches(v),
            (Pattern::Ok(p), Value::Fallible(Ok(v))) => p.matches(v),
            _ => false,
        }
    }

    /// The value this pattern spells, if it has no wildcard.
    pub fn to_value(&self) -> Result<Value, String> {
        Ok(match self {
            Pattern::Any => return Err("`ignore` is not a value".into()),
            Pattern::Family(f) => return Err(format!("`{f}.*` is not a value")),
            Pattern::Lit(v) => v.clone(),
            Pattern::Path(p) => Value::Path(p.clone()),
            Pattern::Record { name, fields, rest } => {
                if *rest {
                    return Err("`..` is not a value".into());
                }
                let mut fs = Vec::new();
                for (n, p) in fields {
                    fs.push((Rc::from(n.as_str()), p.to_value()?));
                }
                if name == "Error" {
                    // An `Error` record read back is a failed `~T`, as it printed.
                    Value::Fallible(Err(Rc::new(Value::Record(Rc::new(Record { name: name.as_str().into(), fields: fs })))))
                } else {
                    Value::Record(Rc::new(Record { name: name.as_str().into(), fields: fs }))
                }
            }
            Pattern::Element { tag, fields, rest, bare } => {
                if *rest {
                    return Err("`..` is not a value".into());
                }
                if *bare {
                    // A bare qualified name is a fault kind, as `Fs.NotFound` prints.
                    return Ok(Value::ctor(tag, vec![]));
                }
                let mut fs = Vec::new();
                for (n, p) in fields {
                    fs.push((Rc::from(n.as_str()), p.to_value()?));
                }
                Value::Element(Rc::new(Element { tag: tag.as_str().into(), fields: fs, decl: None }))
            }
            Pattern::Choice { ctor, payload } => {
                let mut ps = Vec::new();
                for p in payload {
                    ps.push(p.to_value()?);
                }
                Value::Choice(Rc::new(Choice { ctor: ctor.as_str().into(), payload: ps }))
            }
            Pattern::Tuple(ps) => Value::tuple(ps.iter().map(|p| p.to_value()).collect::<Result<_, _>>()?),
            Pattern::Array(ps) => Value::array(ps.iter().map(|p| p.to_value()).collect::<Result<_, _>>()?),
            Pattern::Map(ps) => {
                let mut es = Vec::new();
                for (k, v) in ps {
                    es.push((k.to_value()?, v.to_value()?));
                }
                Value::map(es)
            }
            Pattern::Some(p) => Value::some(p.to_value()?),
            Pattern::Ok(p) => Value::ok(p.to_value()?),
        })
    }
}

fn fields_match(ps: &[(String, Pattern)], rest: bool, fs: &[(Rc<str>, Value)]) -> bool {
    if !rest && ps.len() != fs.len() {
        return false;
    }
    ps.iter().all(|(n, p)| fs.iter().any(|(m, v)| &**m == n && p.matches(v)))
}

// ---------------------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------------------

/// Read a value printed by [`fn@print`].
pub fn read(src: &str) -> Result<Value, String> {
    read_pattern(src)?.to_value()
}

/// Read a pattern of the test language.
pub fn read_pattern(src: &str) -> Result<Pattern, String> {
    let toks = lex(src)?;
    let mut p = Parser { toks, at: 0, split_close: false };
    let pat = p.value(true)?;
    if p.at < p.toks.len() {
        return Err(format!("unexpected {:?} after a value in `{}`", p.toks[p.at], src));
    }
    Ok(pat)
}

/// Read a path written in the notation of Semantics ch. 8 (`1/#0/[2]/0`).
pub fn read_path(src: &str) -> Result<Path, String> {
    let toks = lex(src)?;
    let mut p = Parser { toks, at: 0, split_close: false };
    let path = p.path()?;
    if p.at < p.toks.len() {
        return Err(format!("unexpected {:?} after a path in `{}`", p.toks[p.at], src));
    }
    Ok(path)
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Str(String),
    Char(char),
    Num(BigInt),
    Float(f64),
    Dur(i128),
    Name(String),
    P(&'static str),
}

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let cs: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '"' || c == '\'' {
            let quote = c;
            i += 1;
            let mut s = String::new();
            loop {
                let Some(&c) = cs.get(i) else { return Err("unterminated quote".into()) };
                i += 1;
                if c == quote {
                    break;
                }
                if c == '\\' {
                    let e = *cs.get(i).ok_or("dangling escape")?;
                    i += 1;
                    match e {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        '"' | '\\' | '\'' | '{' | '}' => s.push(e),
                        'u' => {
                            if cs.get(i) != Some(&'{') {
                                return Err("expected { after \\u".into());
                            }
                            let start = i + 1;
                            let end = (start..cs.len()).find(|&j| cs[j] == '}').ok_or("unterminated \\u{")?;
                            let hex: String = cs[start..end].iter().collect();
                            let code = u32::from_str_radix(&hex, 16).map_err(|e| e.to_string())?;
                            s.push(char::from_u32(code).ok_or("bad code point")?);
                            i = end + 1;
                        }
                        other => return Err(format!("unknown escape \\{other}")),
                    }
                } else {
                    s.push(c);
                }
            }
            if quote == '"' {
                out.push(Tok::Str(s));
            } else {
                let mut it = s.chars();
                match (it.next(), it.next()) {
                    (Some(c), None) => out.push(Tok::Char(c)),
                    _ => return Err(format!("a character literal holds one character: '{s}'")),
                }
            }
            continue;
        }
        if c == '-' && cs[i + 1..].starts_with(&['i', 'n', 'f']) && !cs.get(i + 4).is_some_and(|d| d.is_alphanumeric() || *d == '_') {
            out.push(Tok::Float(f64::NEG_INFINITY));
            i += 4;
            continue;
        }
        let neg_num = c == '-' && cs.get(i + 1).is_some_and(|d| d.is_ascii_digit());
        if c.is_ascii_digit() || neg_num {
            let start = i;
            i += 1;
            while i < cs.len() && cs[i].is_ascii_digit() {
                i += 1;
            }
            // A float: digits, a point, digits (Type System ch. 2 always prints the point
            // with a digit on each side, and never an exponent).
            if cs.get(i) == Some(&'.') && cs.get(i + 1).is_some_and(|d| d.is_ascii_digit()) {
                i += 1;
                while i < cs.len() && cs[i].is_ascii_digit() {
                    i += 1;
                }
                let text: String = cs[start..i].iter().collect();
                out.push(Tok::Float(text.parse().map_err(|_| format!("bad number {text}"))?));
                continue;
            }
            let digits: String = cs[start..i].iter().collect();
            let n: BigInt = digits.parse().map_err(|_| format!("bad number {digits}"))?;
            // A duration literal: digits followed directly by a unit.
            let unit_start = i;
            while i < cs.len() && cs[i].is_ascii_alphabetic() {
                i += 1;
            }
            let unit: String = cs[unit_start..i].iter().collect();
            if unit.is_empty() {
                out.push(Tok::Num(n));
            } else {
                let scale: i128 = match unit.as_str() {
                    "h" => 3_600_000_000_000,
                    "m" => 60_000_000_000,
                    "s" => 1_000_000_000,
                    "ms" => 1_000_000,
                    "us" => 1_000,
                    "ns" => 1,
                    _ => return Err(format!("unknown duration unit {unit}")),
                };
                let n: i128 = n.try_into().map_err(|_| "duration out of range".to_string())?;
                out.push(Tok::Dur(n * scale));
            }
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < cs.len() {
                let d = cs[i];
                if d.is_alphanumeric() || d == '_' {
                    i += 1;
                } else if d == '.' && cs.get(i + 1).is_some_and(|e| e.is_alphabetic() || *e == '*') {
                    i += 1;
                    if cs[i] == '*' {
                        i += 1;
                        break;
                    }
                } else {
                    break;
                }
            }
            out.push(Tok::Name(cs[start..i].iter().collect()));
            continue;
        }
        let two: String = cs[i..(i + 2).min(cs.len())].iter().collect();
        let p2 = ["((", "))", "[[", "]]", ".."].into_iter().find(|p| *p == two);
        if let Some(p) = p2 {
            out.push(Tok::P(p));
            i += 2;
            continue;
        }
        let p1 = ["(", ")", "[", "]", "{", "}", ",", ":", "=", "?", "~", "/", "#"]
            .into_iter()
            .find(|p| p.starts_with(c));
        match p1 {
            // A `(` that touches a name opens a record's or element's fields:
            // `Card(rank=Four)`, where `Node (Leaf 1)` applies a constructor.
            Some("(") if i > 0 && (cs[i - 1].is_alphanumeric() || cs[i - 1] == '_') => {
                out.push(Tok::P("call("));
                i += 1;
            }
            Some(p) => {
                out.push(Tok::P(p));
                i += 1;
            }
            None => return Err(format!("unexpected character {c:?}")),
        }
    }
    Ok(out)
}

struct Parser {
    toks: Vec<Tok>,
    at: usize,
    /// Half of a `))` token has been consumed as a `)`.
    split_close: bool,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        if self.split_close {
            return Some(&Tok::P(")"));
        }
        self.toks.get(self.at)
    }
    fn next(&mut self) -> Option<Tok> {
        if self.split_close {
            self.split_close = false;
            self.at += 1;
            return Some(Tok::P(")"));
        }
        let t = self.toks.get(self.at).cloned();
        self.at += 1;
        t
    }
    fn is(&self, p: &str) -> bool {
        matches!(self.peek(), Some(Tok::P(q)) if *q == p)
    }
    fn eat(&mut self, p: &str) -> bool {
        if self.is(p) {
            self.next();
            true
        } else {
            false
        }
    }
    /// Expect a `)`, splitting a `))` if that is what comes.
    fn close_paren(&mut self) -> Result<(), String> {
        if self.eat(")") {
            return Ok(());
        }
        if self.is("))") {
            self.split_close = true;
            return Ok(());
        }
        Err(format!("expected ), found {:?}", self.peek()))
    }
    fn expect(&mut self, p: &str) -> Result<(), String> {
        if self.eat(p) { Ok(()) } else { Err(format!("expected {p}, found {:?}", self.peek())) }
    }

    fn starts_atom(&self) -> bool {
        match self.peek() {
            Some(Tok::Str(_) | Tok::Char(_) | Tok::Num(_) | Tok::Float(_) | Tok::Dur(_) | Tok::Name(_)) => true,
            Some(Tok::P(p)) => matches!(*p, "(" | "((" | "[[" | "{"),
            None => false,
        }
    }

    /// A value; `app` allows a choice constructor applied to arguments without
    /// parentheses, which the printer writes only as a whole value.
    fn value(&mut self, app: bool) -> Result<Pattern, String> {
        if self.eat("?") {
            return Ok(Pattern::Some(Box::new(self.value(false)?)));
        }
        if self.eat("~") {
            return Ok(Pattern::Ok(Box::new(self.value(false)?)));
        }
        if let Some(Tok::Name(n)) = self.peek().cloned() {
            let upper = n.chars().next().is_some_and(|c| c.is_uppercase());
            let next_is_paren = matches!(self.toks.get(self.at + 1), Some(Tok::P("call(")));
            if app && upper && !n.contains('.') && !next_is_paren && n != "True" && n != "False" {
                self.next();
                let mut payload = Vec::new();
                if self.starts_atom() {
                    payload.push(self.value(false)?);
                    while self.eat(",") {
                        payload.push(self.value(false)?);
                    }
                }
                return Ok(Pattern::Choice { ctor: n, payload });
            }
        }
        self.atom()
    }

    /// A field's value: a value, or a constructor applied to one argument with no
    /// parentheses, which ends at the next comma.
    fn field_value(&mut self) -> Result<Pattern, String> {
        if let Some(Tok::Name(n)) = self.peek().cloned() {
            let upper = n.chars().next().is_some_and(|c| c.is_uppercase());
            let next_is_paren = matches!(self.toks.get(self.at + 1), Some(Tok::P("call(")));
            if upper && !n.contains('.') && !next_is_paren && n != "True" && n != "False" && !self.split_close {
                self.next();
                let mut payload = Vec::new();
                if self.starts_atom() {
                    payload.push(self.value(false)?);
                }
                return Ok(Pattern::Choice { ctor: n, payload });
            }
        }
        self.value(false)
    }

    fn atom(&mut self) -> Result<Pattern, String> {
        match self.next() {
            Some(Tok::Str(s)) => Ok(Pattern::Lit(Value::text(&s))),
            Some(Tok::Char(c)) => Ok(Pattern::Lit(Value::Char(c))),
            Some(Tok::Dur(d)) => Ok(Pattern::Lit(Value::Duration(d))),
            Some(Tok::Float(f)) => Ok(Pattern::Lit(Value::Float(f))),
            Some(Tok::Num(n)) => {
                if self.is("/") {
                    self.at -= 1;
                    return Ok(Pattern::Path(self.path()?));
                }
                if n < BigInt::from(0) { Ok(Pattern::Lit(Value::Int(n))) } else { Ok(Pattern::Lit(Value::Nat(n))) }
            }
            Some(Tok::Name(n)) => self.named(n),
            Some(Tok::P("(")) => {
                let v = self.value(true)?;
                self.close_paren()?;
                Ok(v)
            }
            Some(Tok::P("((")) => {
                let mut items = vec![self.value(false)?];
                while self.eat(",") {
                    items.push(self.value(false)?);
                }
                self.expect("))")?;
                Ok(Pattern::Tuple(items))
            }
            Some(Tok::P("[[")) => {
                let mut items = Vec::new();
                if !self.is("]]") {
                    items.push(self.value(false)?);
                    while self.eat(",") {
                        items.push(self.value(false)?);
                    }
                }
                self.expect("]]")?;
                Ok(Pattern::Array(items))
            }
            Some(Tok::P("{")) => {
                let mut entries = Vec::new();
                if !self.is("}") {
                    loop {
                        let k = self.value(false)?;
                        self.expect(":")?;
                        let v = self.value(false)?;
                        entries.push((k, v));
                        if !self.eat(",") {
                            break;
                        }
                    }
                }
                self.expect("}")?;
                Ok(Pattern::Map(entries))
            }
            other => Err(format!("expected a value, found {other:?}")),
        }
    }

    fn named(&mut self, n: String) -> Result<Pattern, String> {
        match n.as_str() {
            "nil" => return Ok(Pattern::Lit(Value::nil())),
            "unit" => return Ok(Pattern::Lit(Value::Unit)),
            "True" => return Ok(Pattern::Lit(Value::Bool(true))),
            "False" => return Ok(Pattern::Lit(Value::Bool(false))),
            "ignore" => return Ok(Pattern::Any),
            "nan" => return Ok(Pattern::Lit(Value::Float(f64::NAN))),
            "inf" => return Ok(Pattern::Lit(Value::Float(f64::INFINITY))),
            _ => {}
        }
        if let Some(fam) = n.strip_suffix(".*") {
            return Ok(Pattern::Family(fam.to_string()));
        }
        let qualified = n.contains('.');
        if self.eat("call(") {
            let mut fields = Vec::new();
            let mut rest = false;
            if !self.is(")") && !self.is("))") {
                loop {
                    if self.eat("..") {
                        rest = true;
                        break;
                    }
                    let Some(Tok::Name(f)) = self.next() else {
                        return Err(format!("expected a field name in {n}(..)"));
                    };
                    self.expect("=")?;
                    let v = self.field_value()?;
                    fields.push((f, v));
                    if !self.eat(",") {
                        break;
                    }
                }
            }
            self.close_paren()?;
            return Ok(if qualified {
                Pattern::Element { tag: n, fields, rest, bare: false }
            } else {
                Pattern::Record { name: n, fields, rest }
            });
        }
        if qualified {
            return Ok(Pattern::Element { tag: n, fields: vec![], rest: false, bare: true });
        }
        // An uppercase constructor with no arguments, or a unit type's lowercase value.
        Ok(Pattern::Choice { ctor: n, payload: vec![] })
    }

    fn path(&mut self) -> Result<Path, String> {
        let mut segs = Vec::new();
        loop {
            match self.next() {
                Some(Tok::Num(n)) => segs.push(Segment::Pos(n.try_into().map_err(|_| "position out of range")?)),
                Some(Tok::P("#")) => match self.next() {
                    Some(Tok::Num(n)) => segs.push(Segment::Gen(n.try_into().map_err(|_| "generation out of range")?)),
                    other => return Err(format!("expected a generation number after #, found {other:?}")),
                },
                Some(Tok::P("[")) => {
                    // A key that is a path prints as its notation, so read one back.
                    let key = if matches!(self.peek(), Some(Tok::Num(_)))
                        && matches!(self.toks.get(self.at + 1), Some(Tok::P("/")))
                    {
                        Value::Path(self.path()?)
                    } else {
                        self.value(true)?.to_value()?
                    };
                    self.expect("]")?;
                    segs.push(Segment::Key(key));
                }
                other => return Err(format!("expected a path segment, found {other:?}")),
            }
            if !self.eat("/") {
                break;
            }
        }
        Ok(Path(segs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(src: &str) {
        let v = read(src).unwrap_or_else(|e| panic!("{src}: {e}"));
        assert_eq!(print(&v), src);
    }

    #[test]
    fn spellings_round_trip() {
        for s in [
            "unit",
            "True",
            "-1",
            "42",
            "\"a \\\"b\\\"\\n\\t\\u{1}\"",
            "'x'",
            "nil",
            "? 3",
            "~ Contents(text=\"hello\")",
            "Card(rank=Four, suit=Spades)",
            "? Card(rank=Four, suit=Spades)",
            "Write \"end of page 1\\n\"",
            "? (Letter \"a\")",
            "{1: \"a\", 2: \"b\"}",
            "[[1, 2, 3]]",
            "[[]]",
            "((1, \"x\"))",
            "Ui.Button(label=\"Increment\")",
            "Test.Log",
            "click",
            "Error(kind=Fs.NotFound, message=\"gone\")",
            "1300us",
            "0s",
            "Rect 1, 2",
            "280.0",
            "-0.5",
            "0.30000000000000004",
            "nan",
            "inf",
            "-inf",
            "Press Point(x=280.0, y=400.0)",
            "Extent(w=512.0, h=560.0)",
            "Rgb 1.0, 0.5, -inf",
            "Gui.Scene(window=WindowId \"Chess\", at=nil)",
            "Box(paint=(Rgb 1.0, 0.5, 0.0), inner=Node (Leaf 1))",
            "Card(rank=Four, suit=Spades)",
        ] {
            round_trip(s);
        }
        assert_eq!(read("280.0").unwrap(), Value::float(280.0));
        assert_ne!(read("280.0").unwrap(), Value::nat(280));
    }

    #[test]
    fn a_show_impl_prints_values_and_path_keys() {
        let spot = |d: u64, a: u64| Value::record("Spot", vec![("down", Value::nat(d)), ("across", Value::nat(a))]);
        let hook: ShowHook = Rc::new(|v: &Value| match v {
            Value::Record(r) if &*r.name == "Spot" => {
                let n = |i: usize| r.fields[i].1.as_i128().unwrap();
                Some(format!("{}{}", (b'a' + n(1) as u8) as char, 8 - n(0)))
            }
            _ => None,
        });
        let path = Path(vec![Segment::Pos(0), Segment::Key(spot(6, 4)), Segment::Pos(0)]);
        assert_eq!(print_path(&path), "0/[Spot(down=6, across=4)]/0");
        let old = set_show_hook(Some(hook));
        assert_eq!(print_path(&path), "0/[e2]/0");
        assert_eq!(print(&Value::tuple(vec![spot(0, 0), Value::text("x")])), "((a8, \"x\"))");
        assert_eq!(print_structural(&spot(0, 0)), "Spot(down=0, across=0)");
        assert_eq!(print(&spot(0, 0)), "a8");
        set_show_hook(old);
        assert_eq!(print(&spot(0, 0)), "Spot(down=0, across=0)");
    }

    #[test]
    fn an_impl_that_prints_its_own_value_structurally_ends() {
        let hook: ShowHook = Rc::new(|v: &Value| Some(format!("<{}>", print(v))));
        let old = set_show_hook(Some(hook));
        assert_eq!(print(&Value::ctor("A", vec![])), "<A>");
        set_show_hook(old);
    }

    #[test]
    fn paths_round_trip() {
        for s in ["0", "1/#0/1/#0/2", "2/[\"a\"]/0", "3/[1/#0/0]/0"] {
            let p = read_path(s).unwrap();
            assert_eq!(print_path(&p), s);
        }
    }

    #[test]
    fn nested_parens_split() {
        round_trip("Node (Leaf 1), (Leaf 2)");
        let v = read("? (Node (Leaf 1), (Leaf 2))").unwrap();
        assert_eq!(print(&v), "? (Node (Leaf 1), (Leaf 2))");
    }

    #[test]
    fn patterns_match() {
        let e = read("Ui.Button(label=\"OK\", enabled=True)").unwrap();
        assert!(read_pattern("Ui.Button(label=\"OK\", ..)").unwrap().matches(&e));
        assert!(!read_pattern("Ui.Button(label=\"OK\")").unwrap().matches(&e));
        assert!(read_pattern("Ui.*").unwrap().matches(&e));
        assert!(read_pattern("Ui.Button(label=ignore, enabled=True)").unwrap().matches(&e));
        let err = Value::error("Fs.NotFound", "gone");
        assert!(read_pattern("Error(kind=Fs.NotFound, ..)").unwrap().matches(&err));
    }
}
