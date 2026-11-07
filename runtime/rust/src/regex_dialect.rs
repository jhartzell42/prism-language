//! The dialect check (claude/regex-dialect.md): one left-to-right parse of a pattern, its
//! flags and a replacement, giving either the first diagnostic, at its byte offset, or
//! the pattern spelled for `regex-automata` with the meaning the books give it, the
//! number of capture groups and the names. The C++ runtime's `src/regex.cpp` is the same
//! procedure, line for line, spelling for RE2; prismc's checker will be a third.
//!
//! Written by Claude on the claude-self-hosting branch; subject to change.

use std::fmt;

/// Which text a diagnostic is about.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RegexPart {
    Flags,
    Pattern,
    Replacement,
}

impl fmt::Display for RegexPart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            RegexPart::Flags => "flags",
            RegexPart::Pattern => "pattern",
            RegexPart::Replacement => "replacement",
        })
    }
}

/// A diagnostic: the part, the byte offset in it, and one of the dialect's fixed messages.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RegexError {
    pub part: RegexPart,
    pub at: usize,
    pub message: &'static str,
}

impl fmt::Display for RegexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{} {}", self.part, self.at, self.message)
    }
}

// The messages, word for word as claude/regex-dialect.md has them.
const UNKNOWN_FLAG: &str = "an unknown flag: the flags are i, m and s";
const FLAG_TWICE: &str = "a flag written twice";
const STRAY_CLOSE_BRACKET: &str = "a ] outside a class: a literal one is \\]";
const STRAY_CLOSE_BRACE: &str = "a } outside a count: a literal one is \\}";
const TRAILING_BACKSLASH: &str = "a \\ at the end of the pattern";
const HEX_TWO: &str = "a \\x escape needs two hex digits or braces";
const HEX_BRACES: &str = "a \\x{...} escape needs one to six hex digits";
const HEX_NO_CHAR: &str = "a \\x escape names no character: a surrogate or past U+10FFFF";
const WORD_BRACES: &str = "\\b{...} is Rust's only: write \\b";
const ANGLE: &str = "\\< and \\> are Rust's only: write \\b";
const BACKREFERENCE: &str = "a backreference: refused so that every match is linear in its text";
const OCTAL: &str = "an octal escape is RE2's only: write \\x";
const QUOTE: &str = "\\Q...\\E is RE2's only: escape each character";
const ONE_BYTE: &str = "\\C, one byte, is RE2's only";
const UNKNOWN_ESCAPE: &str = "an unknown escape";
const PROPERTY_NAME: &str = "a \\p escape needs a property name";
const PROPERTY_OPEN: &str = "a \\p{ that is never closed";
const PROPERTY_NEGATED: &str = "a negated property is written \\P{...}";
const PROPERTY_REFUSED: &str = "the property is refused: RE2 and Rust disagree on C, Cn, Cs and LC";
const PROPERTY_UNKNOWN: &str = "an unknown property: general categories by short name, scripts and Any";
const CLASS_OPEN: &str = "a class opened here is never closed";
const CLASS_FIRST_BRACKET: &str = "a ] first in a class: write \\]";
const CLASS_NESTED: &str = "a [ inside a class: write \\[ (nested classes are Rust's only)";
const CLASS_ASCII_UNKNOWN: &str = "an unknown ASCII class";
const CLASS_SET_OPERATION: &str = "&&, ~~ and -- in a class are Rust's set operations: escape them";
const CLASS_DASH: &str = "a - in a class that is neither a range nor first or last: write \\-";
const CLASS_ORDER: &str = "a range whose ends are out of order";
const CLASS_RANGE_SET: &str = "a range end that is a class";
const CLASS_NEGATED_UNDER_I: &str = "a negated class inside a class under i: write it outside the class";
const CLASS_ESCAPE: &str = "an escape that a class cannot hold";
const LOOKAROUND: &str = "a lookaround: refused so that every match is linear in its text";
const UNKNOWN_GROUP: &str = "an unknown group: the groups are (re), (?:re), (?<name>re), (?P<name>re) and the flag groups";
const NAME_SHAPE: &str = "a group name is a lowercase value name, [a-z_][a-z0-9_]*";
const NAME_OPEN: &str = "a group name that is never closed";
const NAME_TWICE: &str = "a group name used twice";
const GROUP_OPEN: &str = "a group opened here is never closed";
const GROUP_CLOSE: &str = "a ) that closes no group";
const FLAGS_EMPTY: &str = "an empty flag group";
const FLAGS_SECOND_DASH: &str = "a second - in a flag group";
const FLAGS_DASH: &str = "a - that turns off no flag";
const BRACE: &str = "a brace that opens no count: a literal one is \\{";
const COUNT_ZERO: &str = "a count with a leading zero";
const COUNT_BOUND: &str = "a count above 1000";
const COUNT_ORDER: &str = "a count whose minimum exceeds its maximum";
const NOTHING_TO_REPEAT: &str = "a repetition with nothing to repeat";
const REPEAT_REPEAT: &str = "a repetition of a repetition: write (?:a*)* to say it";
const REPEAT_ASSERTION: &str = "a repetition of an assertion";
const PRODUCT: &str = "nested counts multiplying past 1000";
const NESTING: &str = "groups and repetitions nested more than 100 deep";
const SIZE: &str = "the pattern expands past 10,000 atoms";
const R_ZERO: &str = "a group number with a leading zero";
const R_BRACES: &str = "a ${...} that is neither a group number nor a group name";
const R_DOLLAR: &str = "a $ that is not $n, ${n}, ${name} or $$";
const R_NO_NUMBER: &str = "the pattern has no group of that number";
const R_NO_NAME: &str = "the pattern has no group of that name";
const R_UNICODE: &str = "a \\u{...} escape needs one to six hex digits naming a character";
const R_ESCAPE: &str = "an unknown escape in a replacement: the escapes are \\\\, \\n, \\t, \\u{hex} and \\/";

/// The bounds of Appendix B's dialect.
const MAX_COUNT: u32 = 1000;
const MAX_DEPTH: u32 = 100;
const MAX_SIZE: u64 = 10_000;

/// The general categories both libraries read alike (doubt PC refuses `C`, `Cn`, `Cs`, `LC`).
const CATEGORIES: [&str; 34] = [
    "Cc", "Cf", "Co", "L", "Ll", "Lm", "Lo", "Lt", "Lu", "M", "Mc", "Me", "Mn", "N", "Nd", "Nl",
    "No", "P", "Pc", "Pd", "Pe", "Pf", "Pi", "Po", "Ps", "S", "Sc", "Sk", "Sm", "So", "Z", "Zl",
    "Zp", "Zs",
];

/// Unicode 15.0's scripts, the names both libraries accept (doubt UN).
const SCRIPTS: [&str; 163] = [
    "Adlam", "Ahom", "Anatolian_Hieroglyphs", "Arabic", "Armenian", "Avestan", "Balinese", "Bamum",
    "Bassa_Vah", "Batak", "Bengali", "Bhaiksuki", "Bopomofo", "Brahmi", "Braille", "Buginese",
    "Buhid", "Canadian_Aboriginal", "Carian", "Caucasian_Albanian", "Chakma", "Cham", "Cherokee",
    "Chorasmian", "Common", "Coptic", "Cuneiform", "Cypriot", "Cypro_Minoan", "Cyrillic", "Deseret",
    "Devanagari", "Dives_Akuru", "Dogra", "Duployan", "Egyptian_Hieroglyphs", "Elbasan", "Elymaic",
    "Ethiopic", "Georgian", "Glagolitic", "Gothic", "Grantha", "Greek", "Gujarati", "Gunjala_Gondi",
    "Gurmukhi", "Han", "Hangul", "Hanifi_Rohingya", "Hanunoo", "Hatran", "Hebrew", "Hiragana",
    "Imperial_Aramaic", "Inherited", "Inscriptional_Pahlavi", "Inscriptional_Parthian", "Javanese",
    "Kaithi", "Kannada", "Katakana", "Kawi", "Kayah_Li", "Kharoshthi", "Khitan_Small_Script",
    "Khmer", "Khojki", "Khudawadi", "Lao", "Latin", "Lepcha", "Limbu", "Linear_A", "Linear_B",
    "Lisu", "Lycian", "Lydian", "Mahajani", "Makasar", "Malayalam", "Mandaic", "Manichaean",
    "Marchen", "Masaram_Gondi", "Medefaidrin", "Meetei_Mayek", "Mende_Kikakui", "Meroitic_Cursive",
    "Meroitic_Hieroglyphs", "Miao", "Modi", "Mongolian", "Mro", "Multani", "Myanmar", "Nabataean",
    "Nag_Mundari", "Nandinagari", "New_Tai_Lue", "Newa", "Nko", "Nushu", "Nyiakeng_Puachue_Hmong",
    "Ogham", "Ol_Chiki", "Old_Hungarian", "Old_Italic", "Old_North_Arabian", "Old_Permic",
    "Old_Persian", "Old_Sogdian", "Old_South_Arabian", "Old_Turkic", "Old_Uyghur", "Oriya", "Osage",
    "Osmanya", "Pahawh_Hmong", "Palmyrene", "Pau_Cin_Hau", "Phags_Pa", "Phoenician",
    "Psalter_Pahlavi", "Rejang", "Runic", "Samaritan", "Saurashtra", "Sharada", "Shavian",
    "Siddham", "SignWriting", "Sinhala", "Sogdian", "Sora_Sompeng", "Soyombo", "Sundanese",
    "Syloti_Nagri", "Syriac", "Tagalog", "Tagbanwa", "Tai_Le", "Tai_Tham", "Tai_Viet", "Takri",
    "Tamil", "Tangsa", "Tangut", "Telugu", "Thaana", "Thai", "Tibetan", "Tifinagh", "Tirhuta",
    "Toto", "Ugaritic", "Vai", "Vithkuqi", "Wancho", "Warang_Citi", "Yezidi", "Yi",
    "Zanabazar_Square",
];

/// The ASCII classes `[:name:]`, as ranges of code points.
fn ascii_class(name: &str) -> Option<&'static [(u32, u32)]> {
    Some(match name {
        "alnum" => &[(0x30, 0x39), (0x41, 0x5A), (0x61, 0x7A)],
        "alpha" => &[(0x41, 0x5A), (0x61, 0x7A)],
        "ascii" => &[(0x00, 0x7F)],
        "blank" => &[(0x09, 0x09), (0x20, 0x20)],
        "cntrl" => &[(0x00, 0x1F), (0x7F, 0x7F)],
        "digit" => &[(0x30, 0x39)],
        "graph" => &[(0x21, 0x7E)],
        "lower" => &[(0x61, 0x7A)],
        "print" => &[(0x20, 0x7E)],
        "punct" => &[(0x21, 0x2F), (0x3A, 0x40), (0x5B, 0x60), (0x7B, 0x7E)],
        "space" => &[(0x09, 0x0D), (0x20, 0x20)],
        "upper" => &[(0x41, 0x5A)],
        "word" => &[(0x30, 0x39), (0x41, 0x5A), (0x5F, 0x5F), (0x61, 0x7A)],
        "xdigit" => &[(0x30, 0x39), (0x41, 0x46), (0x61, 0x66)],
        _ => return None,
    })
}

/// `\d`, `\w`, `\s` by their letter.
fn perl_class(letter: u8) -> &'static [(u32, u32)] {
    match letter.to_ascii_lowercase() {
        b'd' => ascii_class("digit").unwrap(),
        b'w' => ascii_class("word").unwrap(),
        _ => ascii_class("space").unwrap(),
    }
}

/// The code points not in `ranges` (sorted, disjoint), as ranges.
fn complement(ranges: &[(u32, u32)]) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    let mut next = 0u32;
    for &(lo, hi) in ranges {
        if lo > next {
            out.push((next, lo - 1));
        }
        next = hi + 1;
    }
    if next <= 0x10FFFF {
        out.push((next, 0x10FFFF));
    }
    out
}

/// A code point as the libraries read it: ASCII letters and digits as themselves,
/// everything else `\x{...}`, so that no character of the pattern is syntax to them.
fn push_cp(out: &mut String, c: u32) {
    match char::from_u32(c) {
        Some(ch) if ch.is_ascii_alphanumeric() => out.push(ch),
        _ => out.push_str(&format!("\\x{{{c:x}}}")),
    }
}

fn push_ranges(out: &mut String, ranges: &[(u32, u32)]) {
    for &(lo, hi) in ranges {
        push_cp(out, lo);
        if hi != lo {
            out.push('-');
            push_cp(out, hi);
        }
    }
}

#[derive(Clone, Copy)]
struct Flags {
    i: bool,
    m: bool,
    s: bool,
}

/// What the last piece of a sequence was, for the repetition checks.
#[derive(Clone, Copy, PartialEq)]
enum Last {
    Nothing,
    FlagGroup,
    Assertion,
    Repetition,
    Atom,
}

/// The whole-pattern measures of a piece (claude/regex-dialect.md, the whole-pattern checks).
#[derive(Clone, Copy, Default)]
struct Measure {
    depth: u32,
    size: u64,
    product: u32,
}

impl Measure {
    const ATOM: Measure = Measure { depth: 0, size: 1, product: 1 };
    fn then(self, other: Measure) -> Measure {
        Measure { depth: self.depth.max(other.depth), size: self.size + other.size, product: self.product.max(other.product) }
    }
}

/// A class's item: code points, or a property (its name and whether negated).
enum Item {
    Ranges(Vec<(u32, u32)>),
    Property(bool, String),
}

/// One character read inside a class: a single code point, or a set.
enum ClassAtom {
    Char(u32),
    Set(Item),
}

/// The result of the check: the pattern as the library is to read it, the number of
/// capture groups (group 0 not counted), and each name with its group's number.
pub(crate) struct Checked {
    pub spelling: String,
    pub groups: usize,
    pub names: Vec<(String, usize)>,
}

/// The flags after a literal's last slash.
fn check_flags(flags: &str) -> Result<Flags, RegexError> {
    let mut f = Flags { i: false, m: false, s: false };
    for (at, c) in flags.char_indices() {
        let slot = match c {
            'i' => &mut f.i,
            'm' => &mut f.m,
            's' => &mut f.s,
            _ => return Err(RegexError { part: RegexPart::Flags, at, message: UNKNOWN_FLAG }),
        };
        if *slot {
            return Err(RegexError { part: RegexPart::Flags, at, message: FLAG_TWICE });
        }
        *slot = true;
    }
    Ok(f)
}

struct Parser<'a> {
    s: &'a str,
    b: &'a [u8],
    i: usize,
    open: u32,
    groups: usize,
    names: Vec<(String, usize)>,
    out: String,
}

fn err(at: usize, message: &'static str) -> RegexError {
    RegexError { part: RegexPart::Pattern, at, message }
}

fn hex_value(digits: &[u8]) -> u32 {
    digits.iter().fold(0u32, |v, &d| v * 16 + (d as char).to_digit(16).unwrap())
}

fn is_name(name: &[u8]) -> bool {
    match name.split_first() {
        Some((&first, rest)) => {
            (first.is_ascii_lowercase() || first == b'_') && rest.iter().all(|&c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
        }
        None => false,
    }
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }
    fn peek_at(&self, k: usize) -> Option<u8> {
        self.b.get(self.i + k).copied()
    }
    /// The code point at the cursor, and its length in bytes.
    fn char_here(&self) -> (u32, usize) {
        let c = self.s[self.i..].chars().next().unwrap();
        (c as u32, c.len_utf8())
    }

    /// Branches until the end or a `)`, which is left for the caller.
    fn alternation(&mut self, flags: Flags) -> Result<Measure, RegexError> {
        let mut flags = flags;
        let mut m = self.sequence(&mut flags)?;
        while self.peek() == Some(b'|') {
            self.i += 1;
            self.out.push('|');
            m = m.then(self.sequence(&mut flags)?);
        }
        Ok(m)
    }

    fn sequence(&mut self, flags: &mut Flags) -> Result<Measure, RegexError> {
        let mut total = Measure::default();
        // The last piece: its kind, its measure, and where its spelling starts in `out`.
        let mut last = Last::Nothing;
        let mut last_m = Measure::default();
        while let Some(c) = self.peek() {
            let at = self.i;
            match c {
                b'|' | b')' => break,
                b'*' | b'+' | b'?' | b'{' => {
                    let (lo, hi) = if c == b'{' {
                        self.count()?
                    } else {
                        self.i += 1;
                        match c {
                            b'*' => (0, None),
                            b'+' => (1, None),
                            _ => (0, Some(1)),
                        }
                    };
                    match last {
                        Last::Nothing | Last::FlagGroup => return Err(err(at, NOTHING_TO_REPEAT)),
                        Last::Repetition => return Err(err(at, REPEAT_REPEAT)),
                        Last::Assertion => return Err(err(at, REPEAT_ASSERTION)),
                        Last::Atom => {}
                    }
                    let counted = c == b'{';
                    let factor = if counted { hi.unwrap_or(lo).max(1) } else { 1 };
                    let product = factor * last_m.product.max(1);
                    if counted && product > MAX_COUNT {
                        return Err(err(at, PRODUCT));
                    }
                    match (c, hi) {
                        (b'*', _) => self.out.push('*'),
                        (b'+', _) => self.out.push('+'),
                        (b'?', _) => self.out.push('?'),
                        (_, None) => self.out.push_str(&format!("{{{lo},}}")),
                        (_, Some(h)) if h == lo => self.out.push_str(&format!("{{{lo}}}")),
                        (_, Some(h)) => self.out.push_str(&format!("{{{lo},{h}}}")),
                    }
                    if self.peek() == Some(b'?') {
                        self.i += 1;
                        self.out.push('?');
                    }
                    let m = Measure { depth: last_m.depth + 1, size: last_m.size * u64::from(factor), product };
                    // The repeated piece replaces the piece it repeats in the sequence's sums.
                    total = Measure { depth: total.depth.max(m.depth), size: total.size - last_m.size + m.size, product: total.product.max(m.product) };
                    last = Last::Repetition;
                    last_m = m;
                    continue;
                }
                _ => {}
            }
            let (kind, m) = self.piece(flags)?;
            if kind != Last::FlagGroup {
                total = total.then(m);
            }
            last = kind;
            last_m = m;
        }
        Ok(total)
    }

    /// A count after `{`: its shape, zeros, bound and order, as `(n, m)` (`None`: no bound).
    fn count(&mut self) -> Result<(u32, Option<u32>), RegexError> {
        let at = self.i;
        let mut j = at + 1;
        let digits = |j: &mut usize| -> &'a [u8] {
            let start = *j;
            while self.b.get(*j).is_some_and(u8::is_ascii_digit) {
                *j += 1;
            }
            &self.b[start..*j]
        };
        let first = digits(&mut j);
        if first.is_empty() {
            return Err(err(at, BRACE));
        }
        let second: Option<&[u8]> = match self.b.get(j) {
            Some(b'}') => None,
            Some(b',') => {
                j += 1;
                let d = digits(&mut j);
                if self.b.get(j) != Some(&b'}') {
                    return Err(err(at, BRACE));
                }
                Some(d)
            }
            _ => return Err(err(at, BRACE)),
        };
        self.i = j + 1;
        let zero = |d: &[u8]| d.len() > 1 && d[0] == b'0';
        if zero(first) || second.is_some_and(zero) {
            return Err(err(at, COUNT_ZERO));
        }
        let value = |d: &[u8]| d.iter().fold(0u32, |v, &c| (v * 10 + u32::from(c - b'0')).min(MAX_COUNT + 1));
        let lo = value(first);
        let hi = match second {
            None => Some(lo),
            Some(d) if d.is_empty() => None,
            Some(d) => Some(value(d)),
        };
        if lo > MAX_COUNT || hi.is_some_and(|h| h > MAX_COUNT) {
            return Err(err(at, COUNT_BOUND));
        }
        if hi.is_some_and(|h| lo > h) {
            return Err(err(at, COUNT_ORDER));
        }
        Ok((lo, hi))
    }

    /// One piece that is not a repetition operator.
    fn piece(&mut self, flags: &mut Flags) -> Result<(Last, Measure), RegexError> {
        let at = self.i;
        let c = self.peek().unwrap();
        match c {
            b'(' => self.group(flags),
            b'[' => {
                self.class(*flags)?;
                Ok((Last::Atom, Measure::ATOM))
            }
            b']' => Err(err(at, STRAY_CLOSE_BRACKET)),
            b'}' => Err(err(at, STRAY_CLOSE_BRACE)),
            b'.' => {
                self.i += 1;
                self.out.push_str(if flags.s { "(?s:.)" } else { "[^\\x{a}]" });
                Ok((Last::Atom, Measure::ATOM))
            }
            b'^' => {
                self.i += 1;
                self.out.push_str(if flags.m { "(?m:^)" } else { "\\A" });
                Ok((Last::Assertion, Measure::ATOM))
            }
            b'$' => {
                self.i += 1;
                self.out.push_str(if flags.m { "(?m:$)" } else { "\\z" });
                Ok((Last::Assertion, Measure::ATOM))
            }
            b'\\' => self.escape(*flags),
            _ => {
                let (cp, len) = self.char_here();
                self.i += len;
                self.literal(cp, *flags);
                Ok((Last::Atom, Measure::ATOM))
            }
        }
    }

    fn literal(&mut self, cp: u32, flags: Flags) {
        if flags.i {
            self.out.push_str("(?i:");
        }
        push_cp(&mut self.out, cp);
        if flags.i {
            self.out.push(')');
        }
    }

    /// `\x..` after the `\` at `at`, the cursor on the `x`.
    fn hex(&mut self, at: usize) -> Result<u32, RegexError> {
        self.i += 1;
        let v = if self.peek() == Some(b'{') {
            let start = self.i + 1;
            let mut j = start;
            while self.b.get(j).is_some_and(u8::is_ascii_hexdigit) {
                j += 1;
            }
            let n = j - start;
            if self.b.get(j) != Some(&b'}') || n == 0 || n > 6 {
                return Err(err(at, HEX_BRACES));
            }
            self.i = j + 1;
            hex_value(&self.b[start..j])
        } else {
            match (self.peek(), self.peek_at(1)) {
                (Some(a), Some(b)) if a.is_ascii_hexdigit() && b.is_ascii_hexdigit() => {
                    self.i += 2;
                    hex_value(&[a, b])
                }
                _ => return Err(err(at, HEX_TWO)),
            }
        };
        if (0xD800..=0xDFFF).contains(&v) || v > 0x10FFFF {
            return Err(err(at, HEX_NO_CHAR));
        }
        Ok(v)
    }

    /// `\p...` or `\P...` after the `\` at `at`, the cursor on the `p`: its name, checked.
    fn property(&mut self, at: usize) -> Result<String, RegexError> {
        self.i += 1;
        let name = match self.peek() {
            Some(b'{') => {
                let start = self.i + 1;
                let close = self.b[start..].iter().position(|&c| c == b'}').ok_or(err(at, PROPERTY_OPEN))?;
                let name = &self.s[start..start + close];
                self.i = start + close + 1;
                if name.starts_with('^') {
                    return Err(err(at, PROPERTY_NEGATED));
                }
                name
            }
            Some(c) if c.is_ascii_alphabetic() => {
                self.i += 1;
                &self.s[self.i - 1..self.i]
            }
            _ => return Err(err(at, PROPERTY_NAME)),
        };
        if matches!(name, "C" | "Cn" | "Cs" | "LC") {
            return Err(err(at, PROPERTY_REFUSED));
        }
        if name == "Any" || CATEGORIES.contains(&name) || SCRIPTS.contains(&name) {
            Ok(name.to_string())
        } else {
            Err(err(at, PROPERTY_UNKNOWN))
        }
    }

    /// An escape outside a class.
    fn escape(&mut self, flags: Flags) -> Result<(Last, Measure), RegexError> {
        let at = self.i;
        self.i += 1;
        let Some(c) = self.peek() else { return Err(err(at, TRAILING_BACKSLASH)) };
        let cp = match c {
            b'<' | b'>' => return Err(err(at, ANGLE)),
            c if c.is_ascii_punctuation() => {
                self.i += 1;
                u32::from(c)
            }
            b't' | b'n' | b'r' | b'f' | b'v' | b'a' => {
                self.i += 1;
                match c {
                    b't' => 0x09,
                    b'n' => 0x0A,
                    b'r' => 0x0D,
                    b'f' => 0x0C,
                    b'v' => 0x0B,
                    _ => 0x07,
                }
            }
            b'x' => self.hex(at)?,
            b'd' | b'w' | b's' | b'D' | b'W' | b'S' => {
                self.i += 1;
                let ranges = perl_class(c);
                if flags.i {
                    self.out.push_str("(?i:");
                }
                self.out.push('[');
                if c.is_ascii_uppercase() {
                    self.out.push('^');
                }
                push_ranges(&mut self.out, ranges);
                self.out.push(']');
                if flags.i {
                    self.out.push(')');
                }
                return Ok((Last::Atom, Measure::ATOM));
            }
            b'p' | b'P' => {
                let name = self.property(at)?;
                if flags.i {
                    self.out.push_str("(?i:");
                }
                self.out.push_str(&format!("\\{}{{{name}}}", c as char));
                if flags.i {
                    self.out.push(')');
                }
                return Ok((Last::Atom, Measure::ATOM));
            }
            b'A' | b'z' | b'b' | b'B' => {
                if c == b'b' && self.peek_at(1) == Some(b'{') {
                    return Err(err(at, WORD_BRACES));
                }
                self.i += 1;
                self.out.push_str(match c {
                    b'A' => "\\A",
                    b'z' => "\\z",
                    b'b' => "(?-u:\\b)",
                    _ => "(?-u:\\B)",
                });
                return Ok((Last::Assertion, Measure::ATOM));
            }
            b'1'..=b'9' => return Err(err(at, BACKREFERENCE)),
            b'0' => return Err(err(at, OCTAL)),
            b'Q' | b'E' => return Err(err(at, QUOTE)),
            b'C' => return Err(err(at, ONE_BYTE)),
            _ => return Err(err(at, UNKNOWN_ESCAPE)),
        };
        self.literal(cp, flags);
        Ok((Last::Atom, Measure::ATOM))
    }

    /// One character inside a class: a code point or an escape.
    fn class_atom(&mut self, flags: Flags) -> Result<ClassAtom, RegexError> {
        let at = self.i;
        if self.peek() != Some(b'\\') {
            let (cp, len) = self.char_here();
            self.i += len;
            return Ok(ClassAtom::Char(cp));
        }
        self.i += 1;
        let Some(c) = self.peek() else { return Err(err(at, TRAILING_BACKSLASH)) };
        Ok(match c {
            b'<' | b'>' => return Err(err(at, CLASS_ESCAPE)),
            c if c.is_ascii_punctuation() => {
                self.i += 1;
                ClassAtom::Char(u32::from(c))
            }
            b't' | b'n' | b'r' | b'f' | b'v' | b'a' => {
                self.i += 1;
                ClassAtom::Char(match c {
                    b't' => 0x09,
                    b'n' => 0x0A,
                    b'r' => 0x0D,
                    b'f' => 0x0C,
                    b'v' => 0x0B,
                    _ => 0x07,
                })
            }
            b'x' => ClassAtom::Char(self.hex(at)?),
            b'd' | b'w' | b's' | b'D' | b'W' | b'S' => {
                self.i += 1;
                let ranges = perl_class(c);
                if c.is_ascii_uppercase() {
                    if flags.i {
                        return Err(err(at, CLASS_NEGATED_UNDER_I));
                    }
                    ClassAtom::Set(Item::Ranges(complement(ranges)))
                } else {
                    ClassAtom::Set(Item::Ranges(ranges.to_vec()))
                }
            }
            b'p' | b'P' => {
                let name = self.property(at)?;
                if c == b'P' && flags.i {
                    return Err(err(at, CLASS_NEGATED_UNDER_I));
                }
                ClassAtom::Set(Item::Property(c == b'P', name))
            }
            _ => return Err(err(at, CLASS_ESCAPE)),
        })
    }

    /// An ASCII class `[:name:]` at the cursor (a `[` inside a class), or the nested-class
    /// diagnostic.
    fn ascii_item(&mut self, flags: Flags) -> Result<Item, RegexError> {
        let at = self.i;
        let mut j = at + 1;
        if self.b.get(j) == Some(&b':') {
            j += 1;
            let negated = self.b.get(j) == Some(&b'^');
            if negated {
                j += 1;
            }
            let start = j;
            while self.b.get(j).is_some_and(u8::is_ascii_alphabetic) {
                j += 1;
            }
            if j > start && self.b.get(j) == Some(&b':') && self.b.get(j + 1) == Some(&b']') {
                let ranges = ascii_class(&self.s[start..j]).ok_or(err(at, CLASS_ASCII_UNKNOWN))?;
                if negated && flags.i {
                    return Err(err(at, CLASS_NEGATED_UNDER_I));
                }
                self.i = j + 2;
                return Ok(Item::Ranges(if negated { complement(ranges) } else { ranges.to_vec() }));
            }
        }
        Err(err(at, CLASS_NESTED))
    }

    /// The `&&`, `~~` and `--` refusal, at an unescaped character of a class.
    fn set_operation(&self) -> Result<(), RegexError> {
        match self.peek() {
            Some(c @ (b'&' | b'~' | b'-')) if self.peek_at(1) == Some(c) => Err(err(self.i, CLASS_SET_OPERATION)),
            _ => Ok(()),
        }
    }

    fn class(&mut self, flags: Flags) -> Result<(), RegexError> {
        let open = self.i;
        self.i += 1;
        let negated = self.peek() == Some(b'^');
        if negated {
            self.i += 1;
        }
        let mut items: Vec<Item> = Vec::new();
        let mut first = true;
        loop {
            let at = self.i;
            let Some(c) = self.peek() else { return Err(err(open, CLASS_OPEN)) };
            if c == b']' {
                if first {
                    return Err(err(at, CLASS_FIRST_BRACKET));
                }
                self.i += 1;
                break;
            }
            self.set_operation()?;
            first = false;
            if c == b'[' {
                items.push(self.ascii_item(flags)?);
                continue;
            }
            if c == b'-' {
                if self.peek_at(1).is_none() {
                    return Err(err(open, CLASS_OPEN));
                }
                if at == open + 1 + usize::from(negated) || self.peek_at(1) == Some(b']') {
                    self.i += 1;
                    items.push(Item::Ranges(vec![(0x2D, 0x2D)]));
                    continue;
                }
                return Err(err(at, CLASS_DASH));
            }
            match self.class_atom(flags)? {
                ClassAtom::Set(item) => items.push(item),
                ClassAtom::Char(lo) => {
                    let ranged = self.peek() == Some(b'-') && self.peek_at(1) != Some(b']');
                    if !ranged {
                        items.push(Item::Ranges(vec![(lo, lo)]));
                        continue;
                    }
                    self.set_operation()?;
                    self.i += 1;
                    let right = self.i;
                    match self.peek() {
                        None => return Err(err(open, CLASS_OPEN)),
                        Some(b'[') => return Err(err(right, CLASS_NESTED)),
                        _ => {}
                    }
                    self.set_operation()?;
                    match self.class_atom(flags)? {
                        ClassAtom::Set(_) => return Err(err(right, CLASS_RANGE_SET)),
                        ClassAtom::Char(hi) if hi < lo => return Err(err(at, CLASS_ORDER)),
                        ClassAtom::Char(hi) => items.push(Item::Ranges(vec![(lo, hi)])),
                    }
                }
            }
        }
        if flags.i {
            self.out.push_str("(?i:");
        }
        self.out.push('[');
        if negated {
            self.out.push('^');
        }
        for item in &items {
            match item {
                Item::Ranges(r) => push_ranges(&mut self.out, r),
                Item::Property(neg, name) => self.out.push_str(&format!("\\{}{{{name}}}", if *neg { 'P' } else { 'p' })),
            }
        }
        self.out.push(']');
        if flags.i {
            self.out.push(')');
        }
        Ok(())
    }

    /// A group at its `(`; a bare flag group changes `flags` for the rest of the enclosing one.
    fn group(&mut self, flags: &mut Flags) -> Result<(Last, Measure), RegexError> {
        let open = self.i;
        self.i += 1;
        let mut inner = *flags;
        // The library's opening: a capture, or `(?:` for every other kind.
        let mut capture = false;
        let mut name: Option<String> = None;
        if self.peek() != Some(b'?') {
            capture = true;
        } else {
            self.i += 1;
            let Some(c) = self.peek() else { return Err(err(open, GROUP_OPEN)) };
            match c {
                b':' => self.i += 1,
                b'=' | b'!' => return Err(err(open, LOOKAROUND)),
                b'<' if matches!(self.peek_at(1), Some(b'=' | b'!')) => return Err(err(open, LOOKAROUND)),
                b'<' => {
                    self.i += 1;
                    name = Some(self.name(open)?);
                    capture = true;
                }
                b'P' => match self.peek_at(1) {
                    None => return Err(err(open, GROUP_OPEN)),
                    Some(b'<') => {
                        self.i += 2;
                        name = Some(self.name(open)?);
                        capture = true;
                    }
                    Some(b'=' | b'>') => return Err(err(open, BACKREFERENCE)),
                    Some(_) => return Err(err(open, UNKNOWN_GROUP)),
                },
                c if c.is_ascii_alphabetic() || c == b'-' || c == b')' => {
                    let bare = self.flag_group(open, &mut inner)?;
                    if bare {
                        *flags = inner;
                        return Ok((Last::FlagGroup, Measure::default()));
                    }
                }
                _ => return Err(err(open, UNKNOWN_GROUP)),
            }
        }
        if capture {
            self.groups += 1;
            if let Some(name) = name {
                self.names.push((name, self.groups));
            }
            self.out.push('(');
        } else {
            self.out.push_str("(?:");
        }
        if self.open == MAX_DEPTH {
            return Err(err(0, NESTING));
        }
        self.open += 1;
        let m = self.alternation(inner)?;
        if self.peek() != Some(b')') {
            return Err(err(open, GROUP_OPEN));
        }
        self.i += 1;
        self.open -= 1;
        self.out.push(')');
        Ok((Last::Atom, Measure { depth: m.depth + 1, ..m }))
    }

    /// A group's name, the cursor on its first byte.
    fn name(&mut self, open: usize) -> Result<String, RegexError> {
        let start = self.i;
        let close = self.b[start..].iter().position(|&c| c == b'>').ok_or(err(open, NAME_OPEN))?;
        let name = &self.b[start..start + close];
        if !is_name(name) {
            return Err(err(start, NAME_SHAPE));
        }
        let name = &self.s[start..start + close];
        if self.names.iter().any(|(n, _)| n == name) {
            return Err(err(open, NAME_TWICE));
        }
        self.i = start + close + 1;
        Ok(name.to_string())
    }

    /// The flags of `(?flags)` or `(?flags:`, the cursor on the first; true for a bare group.
    fn flag_group(&mut self, open: usize, flags: &mut Flags) -> Result<bool, RegexError> {
        let mut seen = [false; 3];
        let mut dash: Option<usize> = None;
        let mut after_dash = false;
        let mut any = false;
        loop {
            let at = self.i;
            let Some(c) = self.peek() else { return Err(err(open, GROUP_OPEN)) };
            match c {
                b')' | b':' => {
                    if !any && dash.is_none() {
                        return Err(err(open, FLAGS_EMPTY));
                    }
                    if let Some(d) = dash
                        && !after_dash
                    {
                        return Err(err(d, FLAGS_DASH));
                    }
                    self.i += 1;
                    return Ok(c == b')');
                }
                b'-' => {
                    if dash.is_some() {
                        return Err(err(at, FLAGS_SECOND_DASH));
                    }
                    dash = Some(at);
                }
                c if c.is_ascii_alphabetic() => {
                    let k = match c {
                        b'i' => 0,
                        b'm' => 1,
                        b's' => 2,
                        _ => return Err(err(at, UNKNOWN_FLAG)),
                    };
                    if seen[k] {
                        return Err(err(at, FLAG_TWICE));
                    }
                    seen[k] = true;
                    any = true;
                    after_dash |= dash.is_some();
                    let on = dash.is_none();
                    match k {
                        0 => flags.i = on,
                        1 => flags.m = on,
                        _ => flags.s = on,
                    }
                }
                _ => return Err(err(open, UNKNOWN_GROUP)),
            }
            self.i += 1;
        }
    }
}

/// Checks a pattern and its flags against the dialect.
pub(crate) fn check(pattern: &str, flags: &str) -> Result<Checked, RegexError> {
    let flags = check_flags(flags)?;
    let mut p = Parser { s: pattern, b: pattern.as_bytes(), i: 0, open: 0, groups: 0, names: Vec::new(), out: String::new() };
    let m = p.alternation(flags)?;
    if p.i < p.b.len() {
        return Err(err(p.i, GROUP_CLOSE));
    }
    if m.depth > MAX_DEPTH {
        return Err(err(0, NESTING));
    }
    if m.size > MAX_SIZE {
        return Err(err(0, SIZE));
    }
    Ok(Checked { spelling: p.out, groups: p.groups, names: p.names })
}

/// A piece of a replacement: text, or a capture group's number.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Piece {
    Text(String),
    Group(usize),
}

/// Reads a replacement against the groups of its pattern.
pub(crate) fn check_replacement(r: &str, groups: usize, names: &[(String, usize)]) -> Result<Vec<Piece>, RegexError> {
    let e = |at, message| RegexError { part: RegexPart::Replacement, at, message };
    let b = r.as_bytes();
    let mut out: Vec<Piece> = Vec::new();
    let mut text = String::new();
    let mut i = 0;
    let number = |d: &[u8], at: usize| -> Result<usize, RegexError> {
        if d.len() > 1 && d[0] == b'0' {
            return Err(e(at, R_ZERO));
        }
        let n = d.iter().fold(0usize, |v, &c| v.saturating_mul(10).saturating_add(usize::from(c - b'0')));
        if n > groups { Err(e(at, R_NO_NUMBER)) } else { Ok(n) }
    };
    while i < b.len() {
        let at = i;
        match b[i] {
            b'$' => {
                let group = match b.get(i + 1) {
                    Some(b'$') => {
                        text.push('$');
                        i += 2;
                        continue;
                    }
                    Some(c) if c.is_ascii_digit() => {
                        let mut j = i + 1;
                        while b.get(j).is_some_and(u8::is_ascii_digit) {
                            j += 1;
                        }
                        let n = number(&b[i + 1..j], at)?;
                        i = j;
                        n
                    }
                    Some(b'{') => {
                        let start = i + 2;
                        let close = b[start..].iter().position(|&c| c == b'}').ok_or(e(at, R_BRACES))?;
                        let inner = &b[start..start + close];
                        i = start + close + 1;
                        if !inner.is_empty() && inner.iter().all(u8::is_ascii_digit) {
                            number(inner, at)?
                        } else if is_name(inner) {
                            let name = &r[start..start + close];
                            names.iter().find(|(n, _)| n == name).map(|(_, g)| *g).ok_or(e(at, R_NO_NAME))?
                        } else {
                            return Err(e(at, R_BRACES));
                        }
                    }
                    _ => return Err(e(at, R_DOLLAR)),
                };
                if !text.is_empty() {
                    out.push(Piece::Text(std::mem::take(&mut text)));
                }
                out.push(Piece::Group(group));
            }
            b'\\' => {
                match b.get(i + 1) {
                    Some(b'\\') => text.push('\\'),
                    Some(b'n') => text.push('\n'),
                    Some(b't') => text.push('\t'),
                    Some(b'/') => text.push('/'),
                    Some(b'u') => {
                        let start = i + 3;
                        let mut j = start;
                        while b.get(j).is_some_and(u8::is_ascii_hexdigit) {
                            j += 1;
                        }
                        let n = j - start;
                        let c = (b.get(i + 2) == Some(&b'{') && b.get(j) == Some(&b'}') && (1..=6).contains(&n))
                            .then(|| char::from_u32(hex_value(&b[start..j])))
                            .flatten()
                            .ok_or(e(at, R_UNICODE))?;
                        text.push(c);
                        i = j + 1;
                        continue;
                    }
                    _ => return Err(e(at, R_ESCAPE)),
                }
                i += 2;
            }
            _ => {
                let c = r[i..].chars().next().unwrap();
                text.push(c);
                i += c.len_utf8();
            }
        }
    }
    if !text.is_empty() {
        out.push(Piece::Text(text));
    }
    Ok(out)
}
