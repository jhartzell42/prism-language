//! Regular expressions (Semantics Appendix B, "Regular expressions"; rulings round 5):
//! `Regex`, `RegexReplacement`, the `Group` and `Match` records, and the searches behind
//! `?Regex`, `?Match`, `?Find`, `Matches`, `Replace` and `Split`, over `regex-automata`'s
//! meta engine. A pattern is checked by `regex_dialect` (claude/regex-dialect.md), never
//! by the library, and the library reads the spelling the check gives it.
//!
//! - A `Regex` is an `Rc` of its compiled form: a clone is a count, and an emitted literal
//!   is built once per thread (`thread_local!`) and cloned where it is used.
//! - Every text a match reports is a view of the searched text's buffer, and every cursor
//!   a cursor into the searched text: nothing matched is copied.
//! - The iteration rule is the books', from searches at a position, not the library's
//!   iterators.
//!
//! Written by Claude on the claude-self-hosting branch; subject to change.

use std::rc::Rc;

use regex_automata::util::captures::Captures;
use regex_automata::util::syntax;
use regex_automata::{Anchored, Input, meta};

use crate::collections::{Array, Cursor, Map, Text};
use crate::print::{Printer, Show};
pub use crate::regex_dialect::{RegexError, RegexPart};
use crate::regex_dialect::{Piece, check, check_replacement};

struct Compiled {
    pattern: Box<str>,
    flags: Box<str>,
    engine: meta::Regex,
    /// Capture groups, group 0 not counted: the pattern's, which the engine's count can
    /// fall short of (it drops a trailing group that can never take part, `(a){0}`).
    groups: usize,
    names: Vec<(Text, usize)>,
}

/// `Regex`: a pattern in the dialect with its flags, compiled once and shared.
#[derive(Clone)]
pub struct Regex(Rc<Compiled>);

/// `Group = record: text, front, back` (Appendix B).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Group {
    pub text: Text,
    pub front: Cursor,
    pub back: Cursor,
}

/// `Match = record: text, front, back, groups: [?Group], named: Text ==> Group`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Match {
    pub text: Text,
    pub front: Cursor,
    pub back: Cursor,
    pub groups: Array<Option<Group>>,
    pub named: Map<Text, Group>,
}

/// The bytes `a..b` of `t` (both character boundaries), as a view: no copy.
fn view(t: &Text, a: usize, b: usize) -> Text {
    let from = Cursor::at_byte(t, a).expect("a match starts on a character boundary").after();
    Cursor::at_byte(&from, b - a).expect("a match ends on a character boundary").before()
}

fn group_of(t: &Text, a: usize, b: usize) -> Group {
    let front = Cursor::at_byte(t, a).expect("a group starts on a character boundary");
    let back = Cursor::at_byte(t, b).expect("a group ends on a character boundary");
    Group { text: view(t, a, b), front, back }
}

impl Regex {
    /// A pattern and its flags, checked against the dialect.
    pub fn new(pattern: &str, flags: &str) -> Result<Regex, RegexError> {
        let checked = check(pattern, flags)?;
        let engine = meta::Regex::builder()
            .syntax(syntax::Config::new().nest_limit(1000))
            .configure(meta::Config::new().nfa_size_limit(None))
            .build(&checked.spelling)
            .unwrap_or_else(|e| panic!("regex-automata refused {:?}, the spelling of {pattern:?}, which the dialect accepts: {e}", checked.spelling));
        let names = checked.names.into_iter().map(|(n, g)| (Text::from(n), g)).collect();
        Ok(Regex(Rc::new(Compiled { pattern: pattern.into(), flags: flags.into(), engine, groups: checked.groups, names })))
    }

    /// A literal `%/pattern/flags`: prismc checked it before tick 0, so a refusal here is a
    /// compiler bug, a panic.
    pub fn lit(pattern: &str, flags: &str) -> Regex {
        Regex::new(pattern, flags).unwrap_or_else(|e| panic!("a regex literal outside the dialect (prismc checks every literal): {e}"))
    }

    /// `?Regex: fn Text -> ?Regex`: no flags (inline `(?i)` sets them), `nil` outside the
    /// dialect.
    pub fn parse(t: &Text) -> Option<Regex> {
        Regex::new(t.as_str(), "").ok()
    }

    pub fn pattern(&self) -> &str {
        &self.0.pattern
    }
    pub fn flags(&self) -> &str {
        &self.0.flags
    }
    /// The same compiled regex, not merely an equal one.
    pub fn same(&self, other: &Regex) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    /// One search of `hay` from `start`: the spans of every group, or `None`.
    fn search(&self, caps: &mut Captures, hay: &str, start: usize, anchored: bool) -> bool {
        let input = Input::new(hay).range(start..).anchored(if anchored { Anchored::Yes } else { Anchored::No });
        self.0.engine.search_captures(&input, caps);
        caps.is_match()
    }

    /// The match the last search found, over `t` (whose bytes `hay` is).
    fn build(&self, caps: &Captures, t: &Text) -> Match {
        let whole = caps.get_group(0).expect("a match has group 0");
        let mut groups: Vec<Option<Group>> = Vec::with_capacity(self.0.groups + 1);
        for g in 0..=self.0.groups {
            groups.push(caps.get_group(g).map(|s| group_of(t, s.start, s.end)));
        }
        let mut named = Map::new();
        for (name, g) in &self.0.names {
            if let Some(group) = &groups[*g] {
                named.insert(name.clone(), group.clone());
            }
        }
        let g0 = groups[0].clone().expect("group 0 takes part");
        debug_assert_eq!((g0.front.at(), g0.back.at()), (whole.start, whole.end));
        Match { text: g0.text, front: g0.front, back: g0.back, groups: groups.into_iter().collect(), named }
    }

    /// `?Match: fn Regex, Cursor -> ?Match`: the match starting at the cursor, which sees
    /// the whole text before it (`^`, `\b`).
    pub fn match_at(&self, c: &Cursor) -> Option<Match> {
        let t = c.text();
        let mut caps = self.0.engine.create_captures();
        self.search(&mut caps, t.as_str(), c.at(), true).then(|| self.build(&caps, t))
    }

    /// `?Find: fn Regex, Text -> ?Match`: the leftmost-first match.
    pub fn find(&self, t: &Text) -> Option<Match> {
        let mut caps = self.0.engine.create_captures();
        self.search(&mut caps, t.as_str(), 0, false).then(|| self.build(&caps, t))
    }

    /// Every match by the iteration rule, each given to `each` with the searched text.
    fn each(&self, t: &Text, mut each: impl FnMut(&Captures)) {
        let hay = t.as_str();
        let mut caps = self.0.engine.create_captures();
        let mut start = 0;
        let mut last_end: Option<usize> = None;
        while start <= hay.len() && self.search(&mut caps, hay, start, false) {
            let m = caps.get_match().expect("a match");
            if m.is_empty() && Some(m.start()) == last_end {
                // An empty match where the last one ended is not reported: go on from the
                // next character.
                match hay[m.start()..].chars().next() {
                    Some(c) => start = m.start() + c.len_utf8(),
                    None => break,
                }
                continue;
            }
            each(&caps);
            last_end = Some(m.end());
            start = m.end();
        }
    }

    /// `Matches: fn Regex, Text -> [Match]`.
    pub fn matches(&self, t: &Text) -> Array<Match> {
        let mut out = Vec::new();
        self.each(t, |caps| out.push(self.build(caps, t)));
        out.into_iter().collect()
    }

    /// `Split: fn Regex, Text -> [Text]`: the `k + 1` views between the `k` matches.
    pub fn split(&self, t: &Text) -> Array<Text> {
        let mut out = Vec::new();
        let mut from = 0;
        self.each(t, |caps| {
            let m = caps.get_match().expect("a match");
            out.push(view(t, from, m.start()));
            from = m.end();
        });
        out.push(view(t, from, t.len()));
        out.into_iter().collect()
    }
}

impl PartialEq for Regex {
    fn eq(&self, other: &Regex) -> bool {
        self.pattern() == other.pattern() && self.flags() == other.flags()
    }
}

impl Eq for Regex {}

impl std::fmt::Debug for Regex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&spelling(self.pattern(), None, self.flags()))
    }
}

/// `RegexReplacement`: a pattern, its flags and a replacement read by chapter 2's rule.
#[derive(Clone)]
pub struct RegexReplacement {
    regex: Regex,
    replacement: Rc<str>,
    pieces: Rc<[Piece]>,
}

impl RegexReplacement {
    pub fn new(pattern: &str, flags: &str, replacement: &str) -> Result<RegexReplacement, RegexError> {
        // The diagnostics come flags, pattern, replacement: the regex is checked first.
        let regex = Regex::new(pattern, flags)?;
        let names: Vec<(String, usize)> = regex.0.names.iter().map(|(n, g)| (n.as_str().to_string(), *g)).collect();
        let pieces = check_replacement(replacement, regex.0.groups, &names)?;
        Ok(RegexReplacement { regex, replacement: replacement.into(), pieces: pieces.into() })
    }

    /// A literal `%/pattern/replacement/flags`, checked by prismc: a refusal is a panic.
    pub fn lit(pattern: &str, flags: &str, replacement: &str) -> RegexReplacement {
        RegexReplacement::new(pattern, flags, replacement)
            .unwrap_or_else(|e| panic!("a regex literal outside the dialect (prismc checks every literal): {e}"))
    }

    pub fn regex(&self) -> &Regex {
        &self.regex
    }
    pub fn replacement(&self) -> &str {
        &self.replacement
    }

    /// `Replace: fn RegexReplacement, Text -> Text`: every match `Matches` finds, replaced.
    pub fn replace(&self, t: &Text) -> Text {
        let hay = t.as_str();
        let mut out: Option<String> = None;
        let mut from = 0;
        self.regex.each(t, |caps| {
            let m = caps.get_match().expect("a match");
            let s = out.get_or_insert_with(|| String::with_capacity(hay.len()));
            s.push_str(&hay[from..m.start()]);
            for piece in self.pieces.iter() {
                match piece {
                    Piece::Text(x) => s.push_str(x),
                    Piece::Group(g) => {
                        if let Some(span) = caps.get_group(*g) {
                            s.push_str(&hay[span.start..span.end]);
                        }
                    }
                }
            }
            from = m.end();
        });
        match out {
            // No match: the text itself, shared.
            None => t.clone(),
            Some(mut s) => {
                s.push_str(&hay[from..]);
                Text::from(s)
            }
        }
    }
}

impl PartialEq for RegexReplacement {
    fn eq(&self, other: &RegexReplacement) -> bool {
        self.regex == other.regex && self.replacement == other.replacement
    }
}

impl Eq for RegexReplacement {}

impl std::fmt::Debug for RegexReplacement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&spelling(self.regex.pattern(), Some(&self.replacement), self.regex.flags()))
    }
}

/// The plain literal (Type System ch. 2, RX-EQ): `%/pattern/flags` or
/// `%/pattern/replacement/flags`, a bare `/` written `\/` and a raw newline `\n`.
fn spelling(pattern: &str, replacement: Option<&str>, flags: &str) -> String {
    fn push(out: &mut String, s: &str) {
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            match c {
                '\\' => {
                    out.push('\\');
                    if let Some(d) = chars.next() {
                        out.push(d);
                    }
                }
                '/' => out.push_str("\\/"),
                '\n' => out.push_str("\\n"),
                c => out.push(c),
            }
        }
    }
    let mut out = String::from("%/");
    push(&mut out, pattern);
    out.push('/');
    if let Some(r) = replacement {
        push(&mut out, r);
        out.push('/');
    }
    out.push_str(flags);
    out
}

impl Show for Regex {
    fn show(&self, p: &mut Printer) {
        p.raw(&spelling(self.pattern(), None, self.flags()));
    }
}

impl Show for RegexReplacement {
    fn show(&self, p: &mut Printer) {
        p.raw(&spelling(self.regex.pattern(), Some(&self.replacement), self.regex.flags()));
    }
}

impl Show for Group {
    fn show(&self, p: &mut Printer) {
        p.record("Group", &[("text", &self.text), ("front", &self.front), ("back", &self.back)]);
    }
}

impl Show for Match {
    fn show(&self, p: &mut Printer) {
        p.record("Match", &[("text", &self.text), ("front", &self.front), ("back", &self.back), ("groups", &self.groups), ("named", &self.named)]);
    }
}
