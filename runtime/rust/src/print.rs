//! The transcript's spelling of plain values, as the printer API derived `^Show` calls
//! (Type System ch. 2, "Plain values, and how they print"; Conformance ch. 1).
//!
//! ```
//! use prism_rtl::print::{Printer, Show, show};
//! enum Mark { Dot, Line(i64, i64) }
//! impl Show for Mark {                       // as `derive ^Show` emits it
//!     fn show(&self, p: &mut Printer) {
//!         match self {
//!             Mark::Dot => p.choice("Dot", &[]),
//!             Mark::Line(a, b) => p.choice("Line", &[a, b]),
//!         }
//!     }
//! }
//! assert_eq!(show(&Some(Mark::Line(1, 2))), "? (Line 1, 2)");
//! ```
//!
//! The printer, not the emitted code, knows where a choice needs parentheses: bare as the
//! whole value and as a field's value with one argument (`window=WindowId "Chess"`),
//! parenthesized everywhere else, so every spelling reads back. A type with its own
//! `impl ^Show` calls [`Printer::raw`] with the impl's text. The rest: text quoted with
//! `\"`, `\\`, `\n`, `\t` escaped and other control characters as `\u{hex}`; a float as
//! the shortest decimal that reads back, always with a point, and `nan`, `inf`, `-inf`; a
//! duration in its largest whole unit; `((a, b))`, `[[a, b]]`, `{k: v}` in key order;
//! `unit`, `True`, `nil`, `? x`, `~ x`; a path as `1/#0/[k]/0`.

use crate::collections::{Array, Map, Text};
use crate::num::{Int, Nat};
use crate::pod::{Error, Path, Pod, Segment};

/// A value that prints in the transcript's spelling: `^Show`.
pub trait Show {
    fn show(&self, p: &mut Printer);
}

/// Where a value stands, which decides a choice's parentheses.
#[derive(Clone, Copy, PartialEq)]
enum At {
    Whole,
    Field,
    Inner,
}

/// What emitted `show` methods write to.
pub struct Printer {
    out: String,
    at: At,
}

/// `Show x`: the spelling of a whole value.
pub fn show(x: &dyn Show) -> String {
    let mut p = Printer { out: String::new(), at: At::Whole };
    x.show(&mut p);
    p.out
}

/// `Show x` as a Prism `Text`.
pub fn show_text(x: &dyn Show) -> Text {
    Text::new(&show(x))
}

/// A path in the notation of Semantics ch. 8.
pub fn path(p: &Path) -> String {
    show(p)
}

impl Printer {
    /// Text as it is: a number, a name, an `impl ^Show`'s result.
    pub fn raw(&mut self, s: &str) {
        self.out.push_str(s);
    }

    /// A `Text`, quoted and escaped.
    pub fn text(&mut self, s: &str) {
        self.quoted(s.chars(), '"');
    }

    fn quoted(&mut self, chars: impl Iterator<Item = char>, quote: char) {
        self.out.push(quote);
        for c in chars {
            match c {
                '\\' => self.out.push_str("\\\\"),
                '\n' => self.out.push_str("\\n"),
                '\t' => self.out.push_str("\\t"),
                c if c == quote => self.out.extend(['\\', c]),
                c if c.is_control() => self.out.push_str(&format!("\\u{{{:x}}}", c as u32)),
                c => self.out.push(c),
            }
        }
        self.out.push(quote);
    }

    fn inner(&mut self, x: &dyn Show, at: At) {
        let outer = std::mem::replace(&mut self.at, at);
        x.show(self);
        self.at = outer;
    }

    fn list<'a>(&mut self, open: &str, items: impl Iterator<Item = &'a dyn Show>, close: &str) {
        self.out.push_str(open);
        for (i, x) in items.enumerate() {
            self.out.push_str(if i > 0 { ", " } else { "" });
            self.inner(x, At::Inner);
        }
        self.out.push_str(close);
    }

    /// A choice's constructor applied to its arguments: `Lam (Var 0)`, `click`.
    pub fn choice(&mut self, ctor: &str, args: &[&dyn Show]) {
        let bare = args.is_empty() || self.at == At::Whole || (self.at == At::Field && args.len() == 1);
        self.out.push_str(if bare { "" } else { "(" });
        self.out.push_str(ctor);
        if !args.is_empty() {
            self.list(" ", args.iter().copied(), "");
        }
        self.out.push_str(if bare { "" } else { ")" });
    }

    /// A record or an element, fields named: `Card(rank=Four, suit=Spades)`. An element
    /// with no fields prints bare (`Test.Log`); a record with none, `Name()`.
    pub fn record(&mut self, name: &str, fields: &[(&str, &dyn Show)]) {
        self.out.push_str(name);
        if fields.is_empty() && name.contains('.') {
            return;
        }
        self.out.push('(');
        for (i, (n, v)) in fields.iter().enumerate() {
            self.out.push_str(if i > 0 { ", " } else { "" });
            self.out.push_str(n);
            self.out.push('=');
            self.inner(*v, At::Field);
        }
        self.out.push(')');
    }

    pub fn tuple(&mut self, items: &[&dyn Show]) {
        self.list("((", items.iter().copied(), "))");
    }

    pub fn array<'a>(&mut self, items: impl Iterator<Item = &'a dyn Show>) {
        self.list("[[", items, "]]");
    }

    pub fn map<'a>(&mut self, entries: impl Iterator<Item = (&'a dyn Show, &'a dyn Show)>) {
        self.out.push('{');
        for (i, (k, v)) in entries.enumerate() {
            self.out.push_str(if i > 0 { ", " } else { "" });
            self.inner(k, At::Inner);
            self.out.push_str(": ");
            self.inner(v, At::Inner);
        }
        self.out.push('}');
    }

    /// `nil` or `? x`.
    pub fn option(&mut self, x: Option<&dyn Show>) {
        match x {
            None => self.out.push_str("nil"),
            Some(x) => self.prefixed("? ", x),
        }
    }

    /// `~ x`, and `? x` for [`Printer::option`].
    pub fn prefixed(&mut self, prefix: &str, x: &dyn Show) {
        self.out.push_str(prefix);
        self.inner(x, At::Inner);
    }

    pub fn float(&mut self, f: f64) {
        // Rust's `Display` for `f64` is the shortest round trip and never an exponent.
        let s = match f {
            f if f.is_nan() => "nan".to_string(),
            f if f.is_infinite() => (if f > 0.0 { "inf" } else { "-inf" }).to_string(),
            f => format!("{f}"),
        };
        let point = s.contains('.') || !f.is_finite();
        self.out.push_str(&s);
        self.out.push_str(if point { "" } else { ".0" });
    }

    /// A duration in its largest whole unit.
    pub fn duration(&mut self, ns: i128) {
        const UNITS: [(&str, i128); 6] =
            [("h", 3_600_000_000_000), ("m", 60_000_000_000), ("s", 1_000_000_000), ("ms", 1_000_000), ("us", 1_000), ("ns", 1)];
        let (name, size) = if ns == 0 { ("s", 1) } else { *UNITS.iter().find(|(_, s)| ns % s == 0).unwrap() };
        self.out.push_str(&format!("{}{name}", ns / size));
    }
}

// ---- The library's own types ------------------------------------------------------------

macro_rules! show_with {
    ($($t:ty => |$s:ident, $p:ident| $body:expr;)*) => {$(
        impl Show for $t {
            fn show(&self, $p: &mut Printer) {
                let $s = self;
                $body
            }
        }
    )*};
}

show_with! {
    Int => |s, p| p.raw(&s.to_string());
    Nat => |s, p| p.raw(&s.to_string());
    u8 => |s, p| p.raw(&s.to_string());
    i64 => |s, p| p.raw(&s.to_string());
    () => |_s, p| p.raw("unit");
    bool => |s, p| p.raw(if *s { "True" } else { "False" });
    f64 => |s, p| p.float(*s);
    char => |s, p| p.quoted(std::iter::once(*s), '\'');
    Text => |s, p| p.quoted(s.chars(), '"');
    Error => |s, p| p.record("Error", &[("kind", &Pod::name(&s.kind)), ("message", &s.message)]);
    Path => |s, p| for (i, seg) in s.0.iter().enumerate() {
        p.raw(if i > 0 { "/" } else { "" });
        match seg {
            Segment::Pos(n) => p.raw(&n.to_string()),
            Segment::Gen(g) => p.raw(&format!("#{g}")),
            Segment::Key(k) => {
                p.raw("[");
                p.inner(k, At::Whole);
                p.raw("]");
            }
        }
    };
}

impl<T: Show + ?Sized> Show for std::rc::Rc<T> {
    fn show(&self, p: &mut Printer) {
        (**self).show(p);
    }
}

impl<T: Show> Show for Option<T> {
    fn show(&self, p: &mut Printer) {
        p.option(self.as_ref().map(|x| x as &dyn Show));
    }
}

impl<T: Show> Show for Result<T, Error> {
    fn show(&self, p: &mut Printer) {
        match self {
            Ok(x) => p.prefixed("~ ", x),
            Err(e) => e.show(p),
        }
    }
}

impl<T: Show + Clone> Show for Array<T> {
    fn show(&self, p: &mut Printer) {
        p.array(self.iter().map(|x| x as &dyn Show));
    }
}

impl<K: Show + Ord + Clone, V: Show + Clone> Show for Map<K, V> {
    fn show(&self, p: &mut Printer) {
        p.map(self.iter().map(|(k, v)| (k as &dyn Show, v as &dyn Show)));
    }
}

/// A tuple prints as the printer's tuple, for the arities a program writes (pods have
/// the same, `pod_tuple!`).
macro_rules! show_tuple {
    ($($t:ident $i:tt),+) => {
        impl<$($t: Show),+> Show for ($($t,)+) {
            fn show(&self, p: &mut Printer) {
                p.tuple(&[$(&self.$i),+]);
            }
        }
    };
}

show_tuple!(A 0, B 1);
show_tuple!(A 0, B 1, C 2);
show_tuple!(A 0, B 1, C 2, D 3);
show_tuple!(A 0, B 1, C 2, D 3, E 4);
show_tuple!(A 0, B 1, C 2, D 3, E 4, F 5);

fn fields(fs: &[(std::rc::Rc<str>, Pod)]) -> Vec<(&str, &dyn Show)> {
    fs.iter().map(|(n, v)| (&**n, v as &dyn Show)).collect()
}

fn items(xs: &[Pod]) -> Vec<&dyn Show> {
    xs.iter().map(|x| x as &dyn Show).collect()
}

impl Show for Pod {
    fn show(&self, p: &mut Printer) {
        match self {
            Pod::Unit => p.raw("unit"),
            Pod::Bool(b) => b.show(p),
            Pod::Int(n) => n.show(p),
            Pod::Float(f) => p.float(f.0),
            Pod::Char(c) => c.show(p),
            Pod::Text(t) => t.show(p),
            Pod::Duration(ns) => p.duration(*ns),
            Pod::Record(n, fs) | Pod::Element(n, fs) => p.record(n, &fields(fs)),
            Pod::Choice(c, args) => p.choice(c, &items(args)),
            Pod::Tuple(xs) => p.tuple(&items(xs)),
            Pod::Array(xs) => p.array(xs.iter().map(|x| x as &dyn Show)),
            Pod::Map(es) => p.map(es.iter().map(|(k, v)| (k as &dyn Show, v as &dyn Show))),
            Pod::Nil => p.option(None),
            Pod::Some(x) => p.option(Some(&**x)),
            Pod::Ok(x) => p.prefixed("~ ", &**x),
            Pod::Failed(e) => e.show(p),
            Pod::Path(path) => path.show(p),
            Pod::Shown(text, _) => p.raw(&text.0),
        }
    }
}
