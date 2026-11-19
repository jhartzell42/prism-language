#!/usr/bin/env python3
# Writes tests/checker/neg/regex-NAME.prism and .expected: one program per diagnostic of
# claude/regex-dialect.md, each a literal outside the dialect that prismc's checker must
# refuse before tick 0 with that message at that character. Each program's literal is the
# first row of tests/regex/corpus.txt refused with the message that a literal can spell
# (one line, its delimiters read by Semantics ch. 2's rule); the position is the corpus's
# `part@at`, moved to the literal's place in the line. Run it after the corpus or the
# dialect's messages change:
#
#   python3 -I tests/checker/regex-neg.py
#
# Written by Claude on the claude-self-hosting branch; subject to change.
import os
import re
import sys

here = os.path.dirname(os.path.abspath(__file__))
root = os.path.normpath(os.path.join(here, "..", ".."))
corpus = os.path.join(root, "tests", "regex", "corpus.txt")
neg = os.path.join(here, "neg")

# Every message of claude/regex-dialect.md, word for word, and the program it names.
MESSAGES = [
    ("unknown-flag", "an unknown flag: the flags are i, m and s"),
    ("flag-twice", "a flag written twice"),
    ("stray-bracket", "a ] outside a class: a literal one is \\]"),
    ("stray-brace", "a } outside a count: a literal one is \\}"),
    ("trailing-backslash", "a \\ at the end of the pattern"),
    ("hex-two", "a \\x escape needs two hex digits or braces"),
    ("hex-braces", "a \\x{...} escape needs one to six hex digits"),
    ("hex-no-char", "a \\x escape names no character: a surrogate or past U+10FFFF"),
    ("word-braces", "\\b{...} is Rust's only: write \\b"),
    ("angle", "\\< and \\> are Rust's only: write \\b"),
    ("backreference", "a backreference: refused so that every match is linear in its text"),
    ("octal", "an octal escape is RE2's only: write \\x"),
    ("quote", "\\Q...\\E is RE2's only: escape each character"),
    ("one-byte", "\\C, one byte, is RE2's only"),
    ("unknown-escape", "an unknown escape"),
    ("property-name", "a \\p escape needs a property name"),
    ("property-open", "a \\p{ that is never closed"),
    ("property-negated", "a negated property is written \\P{...}"),
    ("property-refused", "the property is refused: RE2 and Rust disagree on C, Cn, Cs and LC"),
    ("property-unknown", "an unknown property: general categories by short name, scripts and Any"),
    ("class-open", "a class opened here is never closed"),
    ("class-first-bracket", "a ] first in a class: write \\]"),
    ("class-nested", "a [ inside a class: write \\[ (nested classes are Rust's only)"),
    ("class-ascii-unknown", "an unknown ASCII class"),
    ("class-set-operation", "&&, ~~ and -- in a class are Rust's set operations: escape them"),
    ("class-dash", "a - in a class that is neither a range nor first or last: write \\-"),
    ("class-order", "a range whose ends are out of order"),
    ("class-range-set", "a range end that is a class"),
    ("class-negated-under-i", "a negated class inside a class under i: write it outside the class"),
    ("class-escape", "an escape that a class cannot hold"),
    ("lookaround", "a lookaround: refused so that every match is linear in its text"),
    ("unknown-group", "an unknown group: the groups are (re), (?:re), (?<name>re), (?P<name>re) and the flag groups"),
    ("name-shape", "a group name is a lowercase value name, [a-z_][a-z0-9_]*"),
    ("name-open", "a group name that is never closed"),
    ("name-twice", "a group name used twice"),
    ("group-open", "a group opened here is never closed"),
    ("group-close", "a ) that closes no group"),
    ("flags-empty", "an empty flag group"),
    ("flags-second-dash", "a second - in a flag group"),
    ("flags-dash", "a - that turns off no flag"),
    ("brace", "a brace that opens no count: a literal one is \\{"),
    ("count-zero", "a count with a leading zero"),
    ("count-bound", "a count above 1000"),
    ("count-order", "a count whose minimum exceeds its maximum"),
    ("nothing-to-repeat", "a repetition with nothing to repeat"),
    ("repeat-repeat", "a repetition of a repetition: write (?:a*)* to say it"),
    ("repeat-assertion", "a repetition of an assertion"),
    ("product", "nested counts multiplying past 1000"),
    ("nesting", "groups and repetitions nested more than 100 deep"),
    ("size", "the pattern expands past 10,000 atoms"),
    ("r-zero", "a group number with a leading zero"),
    ("r-braces", "a ${...} that is neither a group number nor a group name"),
    ("r-dollar", "a $ that is not $n, ${n}, ${name} or $$"),
    ("r-no-number", "the pattern has no group of that number"),
    ("r-no-name", "the pattern has no group of that name"),
    ("r-unicode", "a \\u{...} escape needs one to six hex digits naming a character"),
    ("r-escape", "an unknown escape in a replacement: the escapes are \\\\, \\n, \\t, \\u{hex} and \\/"),
]

FIX = "a replacement that starts with letters and holds anything but letters is written in the hashed form, `%#/x/#hello world/#`"
TERMINATORS = ",)]}:;"
LETTERS = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"


def field(text):
    if text == "⟨-⟩":
        return None
    if text == "⟨⟩":
        return ""
    return re.sub(r"⟨([0-9A-Fa-f]+)⟩", lambda m: chr(int(m.group(1), 16)), text)


def run_of_letters(t):
    n = 0
    while n < len(t) and t[n] in LETTERS:
        n += 1
    return t[:n]


def plain_body(t):
    """Whether `t` can stand between two slashes of the plain form: no raw newline, every
    `\\` paired, no unpaired `/`."""
    i = 0
    while i < len(t):
        c = t[i]
        if c == "\n":
            return False
        if c == "\\":
            if i + 1 >= len(t) or t[i + 1] == "\n":
                return False
            i += 2
            continue
        if c == "/":
            return False
        i += 1
    return True


def plain(pattern, rep, flags):
    """The plain literal and the offsets of its pattern, replacement and flags, or None."""
    if not plain_body(pattern) or any(c not in LETTERS for c in flags):
        return None
    lit = "%/" + pattern + "/"
    at_p = 2
    at_r = len(lit)
    if rep is not None:
        run = run_of_letters(rep)
        if run != rep:
            c = rep[len(run)]
            if run != "" or c in TERMINATORS or c.isspace():
                return None
            if not plain_body(rep):
                return None
        lit += rep + "/"
    at_f = len(lit)
    return lit + flags, at_p, at_r, at_f


def hashed(pattern, rep, flags):
    if any(c not in LETTERS for c in flags) or "\n" in pattern or (rep is not None and "\n" in rep):
        return None
    n = 1
    while "/" + "#" * n in pattern or (rep is not None and "/" + "#" * n in rep):
        n += 1
    close = "/" + "#" * n
    lit = "%" + "#" * n + "/" + pattern + close
    at_p = 1 + n + 1
    at_r = len(lit)
    if rep is not None:
        run = run_of_letters(rep)
        if run != rep:
            c = rep[len(run)]
            if c in TERMINATORS or (run != "" and not c.isspace()):
                return None
        lit += rep + close
    at_f = len(lit)
    return lit + flags, at_p, at_r, at_f


rows = {}  # message -> first expressible case
for raw in open(corpus, encoding="utf-8"):
    raw = raw.rstrip("\n")
    if not raw.startswith("refuse\t"):
        continue
    _kind, pattern, flags, arg, _input, expected = raw.split("\t")
    pattern, flags, rep = field(pattern), field(flags), field(arg)
    m = re.match(r"(flags|pattern|replacement)@(\d+) (.*)$", expected)
    part, at, message = m.group(1), int(m.group(2)), m.group(3)
    if message in rows:
        continue
    form = plain(pattern, rep, flags) or hashed(pattern, rep, flags)
    if form is None:
        continue
    lit, at_p, at_r, at_f = form
    if any(ord(c) < 0x20 or ord(c) == 0x7F for c in lit):
        continue
    text = {"pattern": pattern, "replacement": rep, "flags": flags}[part]
    base = {"pattern": at_p, "replacement": at_r, "flags": at_f}[part]
    bytes_before = text.encode("utf-8")[:at].decode("utf-8")
    if at >= len(text.encode("utf-8")):
        continue
    rows[message] = (lit, base + len(bytes_before))

os.makedirs(neg, exist_ok=True)
missing = []
for slug, message in MESSAGES:
    if message not in rows:
        missing.append(message)
        continue
    lit, col = rows[message]
    name = f"regex-{slug}"
    line = f"let r = {lit}"
    with open(os.path.join(neg, f"{name}.prism"), "w", encoding="utf-8") as out:
        out.write(f"# `{name}`: a literal outside the dialect (claude/regex-dialect.md), refused before tick 0.\n")
        out.write(line + "\n")
    with open(os.path.join(neg, f"{name}.expected"), "w", encoding="utf-8") as out:
        out.write(f"tests/checker/neg/{name}.prism:2: error[regex]: {message}\n")
        out.write(f"    {line}\n")
        out.write("    " + " " * (len("let r = ") + col) + "^\n")
        if message == MESSAGES[0][1]:
            out.write(f"  fix: {FIX}\n")
        out.write("1 error\n")
if missing:
    sys.exit("no literal spells these refusals:\n" + "\n".join(missing))
print(f"{len(MESSAGES)} programs")
