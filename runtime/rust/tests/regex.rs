//! The regular-expression corpus (`tests/regex/corpus.txt`, its format in
//! `tests/regex/README.md`) against the Rust runtime, plus what a corpus line cannot say:
//! views, sharing, `?Regex`, printing, and the linear-time guard.
//!
//! Written by Claude on the claude-self-hosting branch; subject to change.

use prism_rtl::{Cursor, Group, Match, Regex, RegexReplacement, Text, show};

/// A corpus field, decoded: `⟨⟩` empty, `⟨-⟩` absent, `⟨hex⟩` a code point.
fn decode(field: &str) -> Option<String> {
    if field == "⟨-⟩" {
        return None;
    }
    let mut out = String::new();
    let mut rest = field;
    while let Some(open) = rest.find('⟨') {
        out.push_str(&rest[..open]);
        let after = &rest[open + '⟨'.len_utf8()..];
        let close = after.find('⟩').expect("an unclosed ⟨ in the corpus");
        let hex = &after[..close];
        if !hex.is_empty() {
            let c = u32::from_str_radix(hex, 16).ok().and_then(char::from_u32).expect("a bad ⟨hex⟩ in the corpus");
            out.push(c);
        }
        rest = &after[close + '⟩'.len_utf8()..];
    }
    out.push_str(rest);
    Some(out)
}

/// The byte offset of `part` inside `whole`, asserting that `part` is a view of `whole`'s
/// bytes and not a copy.
fn offset_in(whole: &Text, part: &Text) -> Result<usize, String> {
    let base = whole.as_str().as_ptr() as usize;
    let at = part.as_str().as_ptr() as usize;
    if at < base || at + part.len() > base + whole.len() {
        return Err(format!("{:?} is not a view of the searched text", part.as_str()));
    }
    Ok(at - base)
}

fn span(whole: &Text, text: &Text, front: &Cursor, back: &Cursor) -> Result<String, String> {
    let a = front.at();
    let b = back.at();
    if a > b || b > whole.len() || &whole.as_str()[a..b] != text.as_str() {
        return Err(format!("cursors {a}-{b} do not hold the text {:?}", text.as_str()));
    }
    // An empty view can sit anywhere in the buffer; a non-empty one must sit at its cursor.
    if !text.is_empty() && offset_in(whole, text)? != a {
        return Err(format!("the text of {a}-{b} is a view of other bytes"));
    }
    if front.before().len() != a || back.after().len() != whole.len() - b {
        return Err(format!("the cursors of {a}-{b} are not cursors into the searched text"));
    }
    Ok(format!("{a}-{b}"))
}

fn group(whole: &Text, g: &Group) -> Result<String, String> {
    span(whole, &g.text_p, &g.front_p, &g.back_p)
}

fn render(whole: &Text, m: &Match) -> Result<String, String> {
    let mut out = vec![span(whole, &m.text_p, &m.front_p, &m.back_p)?];
    let g0 = m.groups_p.iter().next().and_then(|g| g.as_ref()).ok_or("groups[0] is not the whole match")?;
    if group(whole, g0)? != out[0] {
        return Err("groups[0] is not the whole match".into());
    }
    for g in m.groups_p.iter().skip(1) {
        out.push(match g {
            Some(g) => group(whole, g)?,
            None => "nil".into(),
        });
    }
    let mut names: Vec<&Text> = m.named_p.keys().collect();
    names.sort();
    for name in names {
        let g = m.named_p.get(name).unwrap();
        out.push(format!("{}={}", name.as_str(), group(whole, g)?));
    }
    Ok(out.join(" "))
}

fn run(kind: &str, pattern: &str, flags: &str, arg: Option<&str>, input: Option<&str>) -> Result<String, String> {
    if kind == "refuse" {
        let made = match arg {
            None => Regex::new(pattern, flags).map(|_| ()),
            Some(r) => RegexReplacement::new(pattern, flags, r).map(|_| ()),
        };
        return match made {
            Ok(()) => Ok("accepted".into()),
            Err(e) => Ok(format!("{}@{} {}", e.part, e.at, e.message)),
        };
    }
    let input = Text::new(input.ok_or("no input")?);
    if kind == "replace" {
        let r = RegexReplacement::new(pattern, flags, arg.ok_or("no replacement")?).map_err(|e| format!("refused: {e}"))?;
        return Ok(r.replace(&input).as_str().to_string());
    }
    let re = Regex::new(pattern, flags).map_err(|e| format!("refused: {e}"))?;
    match kind {
        "find" => re.find(&input).map_or(Ok("nil".into()), |m| render(&input, &m)),
        "at" => {
            let at: usize = arg.ok_or("no offset")?.parse().map_err(|_| "a bad offset")?;
            let c = Cursor::at_byte(&input, at).ok_or("the offset is not a cursor")?;
            re.match_at(&c).map_or(Ok("nil".into()), |m| render(&input, &m))
        }
        "all" => {
            let ms = re.matches(&input);
            if ms.is_empty() {
                return Ok("none".into());
            }
            let all: Result<Vec<String>, String> = ms.iter().map(|m| render(&input, m).map(|s| format!("[{s}]"))).collect();
            Ok(all?.join(" "))
        }
        "split" => {
            let pieces = re.split(&input);
            let mut out = Vec::new();
            for p in pieces.iter() {
                let a = offset_in(&input, p)?;
                out.push(format!("{}-{}", a, a + p.len()));
            }
            Ok(out.join(" "))
        }
        _ => Err(format!("an unknown kind {kind}")),
    }
}

#[test]
fn corpus() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/regex/corpus.txt");
    let corpus = std::fs::read_to_string(path).expect("tests/regex/corpus.txt");
    let mut failures = Vec::new();
    let mut cases = 0;
    for (n, line) in corpus.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        assert_eq!(f.len(), 6, "corpus line {}: six fields", n + 1);
        cases += 1;
        let pattern = decode(f[1]).expect("a pattern");
        let flags = decode(f[2]).expect("flags");
        let arg = decode(f[3]);
        let input = decode(f[4]);
        let expected = decode(f[5]).expect("an expected result");
        let got = run(f[0], &pattern, &flags, arg.as_deref(), input.as_deref()).unwrap_or_else(|e| format!("error: {e}"));
        if got != expected {
            failures.push(format!("line {}: {} {:?}\n  expected {:?}\n  got      {:?}", n + 1, f[0], pattern, expected, got));
        }
    }
    assert!(failures.is_empty(), "{} of {cases} corpus cases fail:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn a_runtime_pattern_is_nil_outside_the_dialect() {
    assert!(Regex::parse(&Text::new("a**")).is_none());
    assert!(Regex::parse(&Text::new("(?=a)")).is_none());
    let re = Regex::parse(&Text::new("(?i)k+")).expect("in the dialect");
    assert_eq!(re.find(&Text::new("xKk")).map(|m| m.text_p.as_str().to_string()), Some("Kk".to_string()));
}

#[test]
fn a_regex_is_shared_not_copied_and_equal_by_spelling() {
    let a = Regex::new("[a-z]+", "i").unwrap();
    let b = a.clone();
    assert!(a.same(&b));
    assert!(a == Regex::new("[a-z]+", "i").unwrap());
    assert!(a != Regex::new("[a-z]+", "").unwrap());
    assert!(a != Regex::new("[a-y]+", "i").unwrap());
    let r = RegexReplacement::new("a", "", "b").unwrap();
    assert!(r == RegexReplacement::new("a", "", "b").unwrap());
    assert!(r != RegexReplacement::new("a", "", "c").unwrap());
}

#[test]
fn regexes_and_matches_print_as_the_books_say() {
    assert_eq!(show(&Regex::new("a/b\\/c", "im").unwrap()), "%/a\\/b\\/c/im");
    assert_eq!(show(&RegexReplacement::new("_", "", "-/").unwrap()), "%/_/-\\//");
    let m = Regex::new("(b)(x)?", "").unwrap().find(&Text::new("ab")).unwrap();
    let shown = show(&m);
    assert!(shown.starts_with("Match(text=\"b\", front=Cursor(at=1), back=Cursor(at=2), groups=[["), "{shown}");
    assert!(shown.contains("Group(text=\"b\", front=Cursor(at=1), back=Cursor(at=2))"), "{shown}");
    assert!(shown.ends_with(", nil]], named={})"), "{shown}");
}

#[test]
fn a_match_from_a_view_reports_cursors_into_that_view() {
    // A text that is itself a view of a bigger buffer: offsets are the view's own.
    let whole = Text::new("xx abc");
    let view = Cursor::at_byte(&whole, 3).unwrap().after();
    let m = Regex::new("b", "").unwrap().find(&view).unwrap();
    assert_eq!((m.front_p.at(), m.back_p.at(), m.front_p.before().as_str()), (1, 2, "a"));
    // ?Match at a cursor of the view sees the view's text before it, not the buffer's.
    let c = Cursor::at_byte(&view, 0).unwrap();
    assert!(Regex::new("^a", "").unwrap().match_at(&c).is_some());
}

#[test]
#[should_panic(expected = "a regex literal outside the dialect")]
fn a_bad_literal_is_a_fatal_error() {
    let _ = Regex::lit("a**", "");
}

#[test]
fn searches_are_linear_in_the_text() {
    // Exponential for a backtracker: (x+x+)+y on x^n with no y.
    let re = Regex::new("(x+x+)+y", "").unwrap();
    let text = Text::from("x".repeat(100_000));
    let start = std::time::Instant::now();
    assert!(re.find(&text).is_none());
    assert!(re.match_at(&Cursor::front(&text)).is_none());
    let took = start.elapsed();
    assert!(took.as_secs_f64() < 10.0, "(x+x+)+y over 100,000 bytes took {took:?}");
}

#[test]
fn a_regex_reads_back_from_its_plain_form() {
    // An emitted record holding a regex derives `FromPod`: the
    // spelling `ToPod` writes reads back to a regex equal by spelling.
    use prism_rtl::{FromPod, ToPod};
    for (pattern, flags) in [("a/b", ""), (r"\d+\.\d", "i"), ("x\ny", "m"), ("", "")] {
        let r = Regex::lit(pattern, flags);
        assert_eq!(Regex::from_pod(&r.to_pod()).unwrap(), r, "{pattern:?}");
    }
    let s = RegexReplacement::lit(r"(\w+)/(\w+)", "", "$2/$1");
    assert_eq!(RegexReplacement::from_pod(&s.to_pod()).unwrap(), s);
}
