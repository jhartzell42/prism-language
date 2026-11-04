//! The collections Prism's types compile to, with Semantics Appendix B's cost promises:
//! `Text` a view of a shared UTF-8 buffer ([`Text`]: `Len` in bytes, constant; a
//! [`Cursor`]'s `Before` and `After` constant views), `k ==> v` a persistent ordered map
//! ([`Map`]), `[[a]]` a persistent RRB vector ([`Array`]). Each clones in constant time.
//! They are generics rustc instantiates per element type, so they cost the library
//! nothing per program (Compiler ch. 5, "The heap"); a unique value updates in place
//! through `Rc::make_mut` on the emitted side (Compiler ch. 3).

use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// `k ==> v`.
pub type Map<K, V> = im_rc::OrdMap<K, V>;
/// `[[a]]`.
pub type Array<T> = im_rc::Vector<T>;

thread_local! {
    static EMPTY: Rc<str> = Rc::from("");
}

/// `Text` (Semantics Appendix B, Text; round 4 TX): UTF-8, a byte range of a shared
/// buffer, ordered bytewise (which for UTF-8 is code-point order). Its range always starts
/// and ends on a character boundary, so `as_str` never fails.
#[derive(Clone)]
pub struct Text {
    buf: Rc<str>,
    start: usize,
    end: usize,
}

impl Text {
    pub fn new(s: &str) -> Text {
        if s.is_empty() { Text::default() } else { Text { buf: Rc::from(s), start: 0, end: s.len() } }
    }
    /// The text's bytes as a `str`.
    pub fn as_str(&self) -> &str {
        &self.buf[self.start..self.end]
    }
    /// `Len`: the number of bytes.
    pub fn len(&self) -> usize {
        self.end - self.start
    }
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
    /// Concatenation, as interpolation does it: one copy of both.
    pub fn concat(&self, other: &Text) -> Text {
        if other.is_empty() {
            return self.clone();
        }
        if self.is_empty() {
            return other.clone();
        }
        let mut s = String::with_capacity(self.len() + other.len());
        s.push_str(self.as_str());
        s.push_str(other.as_str());
        Text::from(s)
    }
    pub fn chars(&self) -> std::str::Chars<'_> {
        self.as_str().chars()
    }
    /// The bytes `a..b` of this text, both on character boundaries: a view of the same
    /// buffer.
    fn view(&self, a: usize, b: usize) -> Text {
        Text { buf: self.buf.clone(), start: self.start + a, end: self.start + b }
    }
}

impl Default for Text {
    fn default() -> Text {
        Text { buf: EMPTY.with(Rc::clone), start: 0, end: 0 }
    }
}

impl PartialEq for Text {
    fn eq(&self, other: &Text) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for Text {}

impl PartialOrd for Text {
    fn partial_cmp(&self, other: &Text) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Text {
    fn cmp(&self, other: &Text) -> std::cmp::Ordering {
        self.as_str().as_bytes().cmp(other.as_str().as_bytes())
    }
}

impl Hash for Text {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.as_str().hash(h)
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Text {
        Text::new(s)
    }
}

impl From<String> for Text {
    fn from(s: String) -> Text {
        if s.is_empty() {
            return Text::default();
        }
        let end = s.len();
        Text { buf: Rc::from(s), start: 0, end }
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

/// `Cursor` (Semantics Appendix B, Text): a text and a byte position in it, always on a
/// character boundary. Opaque to programs; cursors compare by their position.
#[derive(Clone)]
pub struct Cursor {
    text: Text,
    at: usize,
}

impl Cursor {
    /// `Front`: the cursor at the start.
    pub fn front(t: &Text) -> Cursor {
        Cursor { text: t.clone(), at: 0 }
    }
    /// `Back`: the cursor at the end.
    pub fn back(t: &Text) -> Cursor {
        Cursor { text: t.clone(), at: t.len() }
    }
    /// `?CursorAt`: the cursor at byte `n`, `None` past the end or inside a character.
    pub fn at_byte(t: &Text, n: usize) -> Option<Cursor> {
        t.as_str().is_char_boundary(n).then(|| Cursor { text: t.clone(), at: n })
    }
    /// `At`: the byte index.
    pub fn at(&self) -> usize {
        self.at
    }
    /// `?Step`: the character at the cursor and the cursor past it, `None` at the end.
    pub fn step(&self) -> Option<(char, Cursor)> {
        let c = self.text.as_str()[self.at..].chars().next()?;
        Some((c, Cursor { text: self.text.clone(), at: self.at + c.len_utf8() }))
    }
    /// `?StepBack`: the character before the cursor and the cursor at its start, `None` at
    /// the start.
    pub fn step_back(&self) -> Option<(char, Cursor)> {
        let c = self.text.as_str()[..self.at].chars().next_back()?;
        Some((c, Cursor { text: self.text.clone(), at: self.at - c.len_utf8() }))
    }
    /// `Before`: the text before the cursor, a view of the same bytes.
    pub fn before(&self) -> Text {
        self.text.view(0, self.at)
    }
    /// `After`: the text from the cursor on, a view of the same bytes.
    pub fn after(&self) -> Text {
        self.text.view(self.at, self.text.len())
    }
}

impl PartialEq for Cursor {
    fn eq(&self, other: &Cursor) -> bool {
        self.at == other.at
    }
}

impl Eq for Cursor {}

impl PartialOrd for Cursor {
    fn partial_cmp(&self, other: &Cursor) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Cursor {
    fn cmp(&self, other: &Cursor) -> std::cmp::Ordering {
        self.at.cmp(&other.at)
    }
}

impl fmt::Debug for Cursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cursor(at={})", self.at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_bytes_and_orders_bytewise() {
        let t = Text::new("héllo");
        assert_eq!(t.len(), 6);
        assert!(Text::new("ab") < Text::new("b"));
        assert!(Text::new("z") < Text::new("é"));
        assert_eq!(Text::new("hé").concat(&Text::new("llo")), t);
        assert_eq!(Text::default(), Text::new(""));
    }

    #[test]
    fn cursors_walk_characters_and_cut_views() {
        let t = Text::new("h→世");
        let c0 = Cursor::front(&t);
        let (h, c1) = c0.step().unwrap();
        let (arrow, c2) = c1.step().unwrap();
        let (world, c3) = c2.step().unwrap();
        assert_eq!((h, arrow, world), ('h', '→', '世'));
        assert_eq!((c1.at(), c2.at(), c3.at()), (1, 4, 7));
        assert!(c3.step().is_none() && c0.step_back().is_none());
        assert_eq!(c3, Cursor::back(&t));
        let (back, c4) = Cursor::back(&t).step_back().unwrap();
        assert_eq!((back, c4.at()), ('世', 4));
        assert_eq!((c2.before().as_str(), c2.after().as_str()), ("h→", "世"));
        let (inner, rest) = Cursor::front(&c2.after()).step().unwrap();
        assert_eq!((inner, rest.before().as_str(), rest.after().as_str()), ('世', "世", ""));
        assert!(Cursor::at_byte(&t, 2).is_none() && Cursor::at_byte(&t, 8).is_none());
        assert_eq!(Cursor::at_byte(&t, 4).unwrap().after().as_str(), "世");
        assert!(c1 < c2);
    }
}
