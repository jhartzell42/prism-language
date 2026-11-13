//! Reading the transcript's spelling back: values, paths, and the patterns the test
//! language matches with (Conformance ch. 1, "Checks"; Appendix A).
//!
//! Reading is printing's inverse, without types: `Name(f=v)` is a record, `Fam.Tag(f=v)`
//! an element, `Name` or `Name a, b` a choice, a bare qualified name a fault kind, a
//! number followed by `/` a path, a number with a point a `Float`, an `Error(..)` record a
//! failed `~T`. A pattern is a value with three wildcards in it: `ignore` for anything,
//! `..` for the remaining fields, and `Fam.*` for any element of a family.

use num_bigint::BigInt;

use crate::num::Int;
use crate::float::Float;
use crate::pod::{Path, Pod, Segment};

/// A pattern of the test language: a value with wildcards in it.
#[derive(Clone, Debug, PartialEq)]
pub struct Pattern(pub Pod);

/// The wildcards, as the reader writes them into a pattern's value.
const ANY: &str = "ignore";
const REST: &str = "..";

impl Pattern {
    /// Matches anything.
    pub fn any() -> Pattern {
        Pattern(Pod::name(ANY))
    }
    /// Whether `v` matches.
    pub fn matches(&self, v: &Pod) -> bool {
        matches(&self.0, v)
    }
}

fn matches(p: &Pod, v: &Pod) -> bool {
    let all = |ps: &[Pod], vs: &[Pod]| ps.len() == vs.len() && ps.iter().zip(vs).all(|(p, v)| matches(p, v));
    let fields = |ps: &[(std::rc::Rc<str>, Pod)], fs: &[(std::rc::Rc<str>, Pod)]| {
        let rest = ps.iter().any(|(n, _)| &**n == REST);
        (rest || ps.len() == fs.len())
            && ps.iter().filter(|(n, _)| &**n != REST).all(|(n, p)| fs.iter().any(|(m, v)| m == n && matches(p, v)))
    };
    match (p, v.structure()) {
        (Pod::Choice(c, a), _) if &**c == ANY && a.is_empty() => true,
        (Pod::Path(a), Pod::Path(b)) => crate::print::path(a) == crate::print::path(b),
        (Pod::Element(t, _), Pod::Element(u, _)) if t.ends_with(".*") => u.rsplit_once('.').is_some_and(|(f, _)| *f == t[..t.len() - 2]),
        (Pod::Record(n, ps), Pod::Record(m, fs)) | (Pod::Element(n, ps), Pod::Element(m, fs)) => n == m && fields(ps, fs),
        // A bare qualified name is a fault kind, or an element with no fields.
        (Pod::Choice(c, a), Pod::Element(t, fs)) => a.is_empty() && fs.is_empty() && **c == **t,
        (Pod::Choice(c, ps), Pod::Choice(d, vs)) => (c == d || names_case(c, d)) && all(ps, vs),
        (Pod::Tuple(ps), Pod::Tuple(vs)) | (Pod::Array(ps), Pod::Array(vs)) => all(ps, vs),
        (Pod::Map(ps), Pod::Map(es)) => ps.len() == es.len() && ps.iter().zip(es).all(|((pk, pv), (k, v))| matches(pk, k) && matches(pv, v)),
        (Pod::Some(p), Pod::Some(v)) | (Pod::Ok(p), Pod::Ok(v)) | (Pod::Failed(p), Pod::Failed(v)) => matches(p, v),
        (p, v) => p == v,
    }
}

/// Whether a pattern's qualified case name (`Sizes.Large`) names the value's case, which
/// prints bare (`Large`): its last segment is the case. The module path is the checker's
/// to resolve; the Test World holds no types to check it against.
fn names_case(c: &str, d: &str) -> bool {
    !d.contains('.') && c.rsplit_once('.').is_some_and(|(_, last)| last == d)
}

/// A qualified name whose last segment is a case's (`Sizes.Large`), not a family's `.*`.
fn qualified_case(n: &str) -> bool {
    n.rsplit_once('.').is_some_and(|(_, last)| last.chars().next().is_some_and(char::is_uppercase))
}

/// Read a value.
pub fn value(src: &str) -> Result<Pod, String> {
    let mut p = Parser { toks: lex(src)?, at: 0, split_close: false };
    let v = p.value(true)?;
    p.done(src)?;
    Ok(v)
}

/// Read a pattern.
pub fn pattern(src: &str) -> Result<Pattern, String> {
    value(src).map(Pattern)
}

/// Read a path (`1/#0/[2]/0`).
pub fn path(src: &str) -> Result<Path, String> {
    let mut p = Parser { toks: lex(src)?, at: 0, split_close: false };
    let path = p.path()?;
    p.done(src)?;
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

const UNITS: [(&str, i128); 6] =
    [("h", 3_600_000_000_000), ("min", 60_000_000_000), ("s", 1_000_000_000), ("ms", 1_000_000), ("us", 1_000), ("ns", 1)];

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let cs: Vec<char> = src.chars().collect();
    let (mut i, mut out) = (0, Vec::new());
    let word = |j: usize| cs.get(j).is_some_and(|d| d.is_alphanumeric() || *d == '_');
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '"' || c == '\'' {
            let (quote, mut s) = (c, String::new());
            i += 1;
            loop {
                let c = *cs.get(i).ok_or("unterminated quote")?;
                i += 1;
                if c == quote {
                    break;
                }
                if c != '\\' {
                    s.push(c);
                    continue;
                }
                let e = *cs.get(i).ok_or("dangling escape")?;
                i += 1;
                match e {
                    'n' => s.push('\n'),
                    't' => s.push('\t'),
                    'u' => {
                        let end = (i..cs.len()).find(|&j| cs[j] == '}').ok_or("unterminated \\u{")?;
                        let hex: String = cs[i + 1..end].iter().collect();
                        let code = u32::from_str_radix(&hex, 16).map_err(|e| e.to_string())?;
                        s.push(char::from_u32(code).ok_or("bad code point")?);
                        i = end + 1;
                    }
                    e => s.push(e),
                }
            }
            let mut one = s.chars();
            out.push(match (quote, one.next(), one.next()) {
                ('"', ..) => Tok::Str(s),
                (_, Some(c), None) => Tok::Char(c),
                _ => return Err(format!("a character literal holds one character: '{s}'")),
            });
        } else if c == '-' && cs[i + 1..].starts_with(&['i', 'n', 'f']) && !word(i + 4) {
            out.push(Tok::Float(f64::NEG_INFINITY));
            i += 4;
        } else if c.is_ascii_digit() || (c == '-' && cs.get(i + 1).is_some_and(char::is_ascii_digit)) {
            let start = i;
            i += 1;
            while cs.get(i).is_some_and(char::is_ascii_digit) {
                i += 1;
            }
            if cs.get(i) == Some(&'.') && cs.get(i + 1).is_some_and(char::is_ascii_digit) {
                i += 1;
                while cs.get(i).is_some_and(char::is_ascii_digit) {
                    i += 1;
                }
                let t: String = cs[start..i].iter().collect();
                out.push(Tok::Float(t.parse().map_err(|_| format!("bad number {t}"))?));
                continue;
            }
            let digits: String = cs[start..i].iter().collect();
            let n: BigInt = digits.parse().map_err(|_| format!("bad number {digits}"))?;
            let unit_start = i;
            while cs.get(i).is_some_and(char::is_ascii_alphabetic) {
                i += 1;
            }
            let unit: String = cs[unit_start..i].iter().collect();
            if unit.is_empty() {
                out.push(Tok::Num(n));
            } else {
                let (_, scale) = UNITS.iter().find(|(u, _)| *u == unit).ok_or(format!("unknown duration unit {unit}"))?;
                let n: i128 = n.try_into().map_err(|_| "duration out of range".to_string())?;
                out.push(Tok::Dur(n * scale));
            }
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < cs.len() {
                if word(i) {
                    i += 1;
                } else if cs[i] == '.' && cs.get(i + 1).is_some_and(|e| e.is_alphabetic() || *e == '*') {
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
        } else {
            let two: String = cs[i..(i + 2).min(cs.len())].iter().collect();
            if let Some(p) = ["((", "))", "[[", "]]", "{{", "}}", ".."].into_iter().find(|p| *p == two) {
                out.push(Tok::P(p));
                i += 2;
                continue;
            }
            // `-` alone is a sparse tuple's absent part (`(("hello", -))`); a number's
            // sign and `-inf` were read above
            let p = ["(", ")", "[", "]", "{", "}", ",", ":", "=", "?", "~", "/", "#", "-"]
                .into_iter()
                .find(|p| p.starts_with(c))
                .ok_or(format!("unexpected character {c:?}"))?;
            // A `(` that touches a name opens fields: `Card(rank=Four)`, where
            // `Node (Leaf 1)` applies a constructor.
            let touches = p == "(" && i > 0 && word(i - 1);
            out.push(Tok::P(if touches { "call(" } else { p }));
            i += 1;
        }
    }
    Ok(out)
}

struct Parser {
    toks: Vec<Tok>,
    at: usize,
    /// Half of a `))` has been consumed as a `)`.
    split_close: bool,
}

impl Parser {
    fn done(&self, src: &str) -> Result<(), String> {
        match self.toks.get(self.at) {
            Some(t) if !self.split_close => Err(format!("unexpected {t:?} in `{src}`")),
            _ => Ok(()),
        }
    }
    fn peek(&self) -> Option<&Tok> {
        if self.split_close { Some(&Tok::P(")")) } else { self.toks.get(self.at) }
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.peek().cloned();
        self.split_close = false;
        self.at += 1;
        t
    }
    fn is(&self, p: &str) -> bool {
        matches!(self.peek(), Some(Tok::P(q)) if *q == p)
    }
    fn eat(&mut self, p: &str) -> bool {
        let yes = self.is(p);
        if yes {
            self.next();
        }
        yes
    }
    fn expect(&mut self, p: &str) -> Result<(), String> {
        if self.eat(p) { Ok(()) } else { Err(format!("expected {p}, found {:?}", self.peek())) }
    }
    /// A `)`, splitting a `))` if that is what comes.
    fn close_paren(&mut self) -> Result<(), String> {
        if self.is("))") && !self.split_close {
            self.split_close = true;
            return Ok(());
        }
        self.expect(")")
    }
    fn starts_atom(&self) -> bool {
        match self.peek() {
            Some(Tok::P(p)) => matches!(*p, "(" | "((" | "[[" | "{{"),
            Some(_) => true,
            None => false,
        }
    }
    /// An uppercase constructor that is not a record, an element or a Bool: bare
    /// (`Large`), or qualified by its module (`Sizes.Large`, whose arguments only a case
    /// takes).
    fn bare_ctor(&self) -> Option<String> {
        match self.peek() {
            Some(Tok::Name(n)) if !self.split_close => {
                let upper = n.chars().next().is_some_and(char::is_uppercase);
                let call = matches!(self.toks.get(self.at + 1), Some(Tok::P("call(")));
                (upper && (!n.contains('.') || qualified_case(n)) && !call && n != "True" && n != "False").then(|| n.clone())
            }
            _ => None,
        }
    }
    /// A value; `app` lets a choice take several arguments with no parentheses, which the
    /// printer writes only as a whole value.
    fn value(&mut self, app: bool) -> Result<Pod, String> {
        if self.eat("?") {
            return Ok(Pod::Some(Box::new(self.value(false)?)));
        }
        if self.eat("~") {
            return Ok(Pod::Ok(Box::new(self.value(false)?)));
        }
        if let (true, Some(ctor)) = (app, self.bare_ctor()) {
            self.next();
            let mut payload = Vec::new();
            if self.starts_atom() {
                payload.push(self.value(false)?);
                while self.eat(",") {
                    payload.push(self.value(false)?);
                }
            }
            return Ok(Pod::Choice(ctor.into(), payload));
        }
        self.atom()
    }
    /// A field's value: a constructor applied to one argument needs no parentheses.
    fn field_value(&mut self) -> Result<Pod, String> {
        if let Some(ctor) = self.bare_ctor() {
            self.next();
            let payload = if self.starts_atom() { vec![self.value(false)?] } else { vec![] };
            return Ok(Pod::Choice(ctor.into(), payload));
        }
        self.value(false)
    }
    fn items(&mut self, close: &str) -> Result<Vec<Pod>, String> {
        let mut items = Vec::new();
        if !self.is(close) {
            items.push(self.value(false)?);
            while self.eat(",") {
                items.push(self.value(false)?);
            }
        }
        self.expect(close)?;
        Ok(items)
    }
    fn atom(&mut self) -> Result<Pod, String> {
        Ok(match self.next() {
            Some(Tok::Str(s)) => Pod::text(s.as_str()),
            Some(Tok::Char(c)) => Pod::Char(c),
            Some(Tok::Dur(d)) => Pod::Duration(d),
            Some(Tok::Float(f)) => Pod::Float(Float(f)),
            Some(Tok::Num(_)) if self.is("/") => {
                self.at -= 1;
                Pod::Path(self.path()?)
            }
            Some(Tok::Num(n)) => Pod::Int(Int::from(n)),
            Some(Tok::Name(n)) => self.named(n)?,
            Some(Tok::P("(")) => {
                let v = self.value(true)?;
                self.close_paren()?;
                v
            }
            Some(Tok::P("((")) => Pod::Tuple(self.items("))")?),
            Some(Tok::P("-")) => Pod::Absent,
            Some(Tok::P("[[")) => Pod::Array(self.items("]]")?),
            // a map is `{{k: v}}` (Jimmy's ruling DB); a single brace is no value
            Some(Tok::P("{")) => return Err("a map is written `{{k: v}}`, with double braces".into()),
            Some(Tok::P("{{")) => {
                let mut es = Vec::new();
                while !self.is("}}") {
                    let k = self.value(false)?;
                    self.expect(":")?;
                    es.push((k, self.value(false)?));
                    if !self.eat(",") {
                        break;
                    }
                }
                self.expect("}}")?;
                es.sort_by(|a, b| a.0.cmp(&b.0));
                Pod::Map(es)
            }
            other => return Err(format!("expected a value, found {other:?}")),
        })
    }
    fn named(&mut self, n: String) -> Result<Pod, String> {
        match n.as_str() {
            "nil" => return Ok(Pod::Nil),
            "unit" => return Ok(Pod::Unit),
            "True" | "False" => return Ok(Pod::Bool(n == "True")),
            "nan" => return Ok(Pod::Float(Float(f64::NAN))),
            "inf" => return Ok(Pod::Float(Float(f64::INFINITY))),
            _ if n.ends_with(".*") => return Ok(Pod::Element(n.into(), vec![])),
            _ => {}
        }
        if !self.eat("call(") {
            return Ok(Pod::name(&n));
        }
        let mut fields = Vec::new();
        while !self.is(")") && !self.is("))") {
            if self.eat(REST) {
                fields.push((REST.into(), Pod::Unit));
                break;
            }
            let Some(Tok::Name(f)) = self.next() else { return Err(format!("expected a field name in {n}(..)")) };
            self.expect("=")?;
            fields.push((f.into(), self.field_value()?));
            if !self.eat(",") {
                break;
            }
        }
        self.close_paren()?;
        Ok(match n.as_str() {
            "Error" => Pod::Failed(Box::new(Pod::Record(n.into(), fields))),
            _ if n.contains('.') => Pod::Element(n.into(), fields),
            _ => Pod::Record(n.into(), fields),
        })
    }
    fn path(&mut self) -> Result<Path, String> {
        let mut segs = Vec::new();
        loop {
            let n32 = |n: BigInt| u32::try_from(n).map_err(|_| "a path segment out of range".to_string());
            match self.next() {
                Some(Tok::Num(n)) => segs.push(Segment::Pos(n32(n)?)),
                Some(Tok::P("#")) => match self.next() {
                    Some(Tok::Num(n)) => segs.push(Segment::Gen(n32(n)?)),
                    other => return Err(format!("expected a generation after #, found {other:?}")),
                },
                Some(Tok::P("[")) => {
                    // A key that is a path prints as its notation, so read one back.
                    let is_path = matches!(self.peek(), Some(Tok::Num(_))) && matches!(self.toks.get(self.at + 1), Some(Tok::P("/")));
                    let key = if is_path { Pod::Path(self.path()?) } else { self.value(true)? };
                    self.expect("]")?;
                    segs.push(Segment::Key(key));
                }
                other => return Err(format!("expected a path segment, found {other:?}")),
            }
            if !self.eat("/") {
                return Ok(Path(segs));
            }
        }
    }
}
