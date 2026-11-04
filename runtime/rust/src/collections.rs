//! The collections Prism's types compile to, with Semantics Appendix B's cost promises:
//! `Text` a rope ([`Text`]: `Len` constant, `Take` and `Drop` logarithmic), `k ==> v` a
//! persistent ordered map ([`Map`]), `[[a]]` a persistent RRB vector ([`Array`]). Each
//! clones in constant time. They are generics rustc instantiates per element type, so
//! they cost the library nothing per program (Compiler ch. 5, "The heap"); a unique value
//! updates in place through `Rc::make_mut` on the emitted side (Compiler ch. 3).

use std::fmt;

/// `k ==> v`.
pub type Map<K, V> = im_rc::OrdMap<K, V>;
/// `[[a]]`.
pub type Array<T> = im_rc::Vector<T>;

/// `Text`: a rope of Unicode scalar values, ordered by its characters.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Text(ropey::Rope);

impl Text {
    pub fn new(s: &str) -> Text {
        Text(ropey::Rope::from_str(s))
    }
    /// `Len`: the number of characters.
    pub fn len(&self) -> usize {
        self.0.len_chars()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// `Take text, n`: the first `n` characters.
    pub fn take(&self, n: usize) -> Text {
        let mut r = self.0.clone();
        r.remove(n.min(r.len_chars())..);
        Text(r)
    }
    /// `Drop text, n`: everything after the first `n` characters.
    pub fn drop_front(&self, n: usize) -> Text {
        let mut r = self.0.clone();
        r.remove(..n.min(r.len_chars()));
        Text(r)
    }
    /// Concatenation, as interpolation does it: linear in the right operand only.
    pub fn concat(&self, other: &Text) -> Text {
        let mut r = self.0.clone();
        other.0.chunks().for_each(|c| r.insert(r.len_chars(), c));
        Text(r)
    }
    pub fn char_at(&self, i: usize) -> Option<char> {
        (i < self.len()).then(|| self.0.char(i))
    }
    pub fn chars(&self) -> impl Iterator<Item = char> + '_ {
        self.0.chars()
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Text {
        Text::new(s)
    }
}

impl From<String> for Text {
    fn from(s: String) -> Text {
        Text::new(&s)
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.chunks().try_for_each(|c| f.write_str(c))
    }
}

impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_takes_and_drops_by_characters() {
        let t = Text::new("héllo");
        assert_eq!((t.len(), t.take(2).to_string(), t.drop_front(2).to_string()), (5, "hé".into(), "llo".into()));
        assert_eq!(t.take(2).concat(&t.drop_front(2)), t);
        assert!(Text::new("ab") < Text::new("b"));
    }
}
