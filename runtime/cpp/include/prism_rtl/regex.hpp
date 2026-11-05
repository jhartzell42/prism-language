// Regular expressions (Semantics Appendix B, "Regular expressions"; rulings round 5):
// `Regex`, `RegexReplacement`, the `Group` and `Match` records, and the searches behind
// `?Regex`, `?Match`, `?Find`, `Matches`, `Replace` and `Split`, over RE2. The twin of
// runtime/rust/src/regex.rs.
//
// - A pattern is checked against the dialect by this runtime's own parse
//   (claude/regex-dialect.md, the same procedure as the Rust runtime's), never by RE2;
//   RE2 reads the spelling the parse gives it.
// - This header does not include RE2's: a program that uses no regex needs none of its
//   headers, and links `-lre2` only when `src/regex.cpp`'s object is pulled in.
// - A `Regex` is an `Rc` of its compiled form (non-atomic, claude/decisions.md D4): a
//   copy is a count. An emitted literal is a function-local static, built on first use.
// - Every text a match reports is a view of the searched text's buffer, and every cursor
//   a cursor into the searched text: nothing matched is copied. A `Match` keeps that
//   buffer alive through its views, as any view does.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#pragma once

#include <cstddef>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

#include "prism_rtl/collections.hpp"
#include "prism_rtl/print.hpp"
#include "prism_rtl/rc.hpp"

namespace prism_rtl {

/// Which text a diagnostic is about.
enum class RegexPart { flags, pattern, replacement };
std::string_view to_string(RegexPart p);

/// A diagnostic: the part, the byte offset in it, and one of the dialect's fixed messages
/// (static storage).
struct RegexError {
    RegexPart part = RegexPart::pattern;
    std::size_t at = 0;
    std::string_view message;
    /// `pattern@3 message`, as the corpus writes it.
    std::string to_string() const;
};

/// `Group = record: text, front, back` (Appendix B).
struct Group {
    Text text;
    Cursor front, back;
    friend bool operator==(const Group& a, const Group& b) { return a.text == b.text && a.front == b.front && a.back == b.back; }
    void show(Printer& p) const;
};

/// `Match = record: text, front, back, groups: [?Group], named: Text ==> Group`.
struct Match {
    Text text;
    Cursor front, back;
    Array<std::optional<Group>> groups;
    Map<Text, Group> named;
    friend bool operator==(const Match& a, const Match& b) {
        return a.text == b.text && a.front == b.front && a.back == b.back && a.groups == b.groups && a.named == b.named;
    }
    void show(Printer& p) const;
};

namespace detail {
struct RegexCompiled;
struct ReplacementPiece {
    std::string text;
    /// A group's number, or -1 for text.
    long group = -1;
};
}  // namespace detail

/// `Regex`: a pattern in the dialect with its flags, compiled once and shared.
class Regex {
public:
    /// A pattern and its flags, checked against the dialect; none, and `*error` set, when
    /// they are outside it.
    static std::optional<Regex> compile(std::string_view pattern, std::string_view flags, RegexError* error = nullptr);
    /// A literal `%/pattern/flags`: prismc checked it before tick 0, so a refusal here is
    /// a compiler bug, a `Panic`.
    static Regex lit(std::string_view pattern, std::string_view flags);
    /// `?Regex: fn Text -> ?Regex`: no flags (inline `(?i)` sets them), none outside the
    /// dialect.
    static std::optional<Regex> parse(const Text& t);

    /// `?Match: fn Regex, Cursor -> ?Match`: the match starting at the cursor, which sees
    /// the whole text before it (`^`, `\b`).
    std::optional<Match> match_at(const Cursor& c) const;
    /// `?Find: fn Regex, Text -> ?Match`: the leftmost-first match.
    std::optional<Match> find(const Text& t) const;
    /// `Matches: fn Regex, Text -> [Match]`, by the books' iteration rule.
    Array<Match> matches(const Text& t) const;
    /// `Split: fn Regex, Text -> [Text]`: the `k + 1` views between the `k` matches.
    Array<Text> split(const Text& t) const;

    std::string_view pattern() const;
    std::string_view flags() const;
    /// The same compiled regex, not merely an equal one.
    bool same(const Regex& o) const { return c_.get() == o.c_.get(); }
    /// Equal by spelling: pattern and flags written alike (Type System ch. 2, RX-EQ).
    friend bool operator==(const Regex& a, const Regex& b) { return a.pattern() == b.pattern() && a.flags() == b.flags(); }
    /// `%/pattern/flags`.
    void show(Printer& p) const;

    const detail::RegexCompiled& compiled() const { return *c_; }

private:
    explicit Regex(Rc<const detail::RegexCompiled> c) : c_(std::move(c)) {}
    Rc<const detail::RegexCompiled> c_;
};

/// `RegexReplacement`: a pattern, its flags and a replacement read by chapter 2's rule.
class RegexReplacement {
public:
    static std::optional<RegexReplacement> compile(std::string_view pattern, std::string_view flags, std::string_view replacement,
                                                   RegexError* error = nullptr);
    /// A literal `%/pattern/replacement/flags`, checked by prismc: a refusal is a `Panic`.
    static RegexReplacement lit(std::string_view pattern, std::string_view flags, std::string_view replacement);

    /// `Replace: fn RegexReplacement, Text -> Text`: every match `Matches` finds, replaced.
    Text replace(const Text& t) const;

    const Regex& regex() const { return regex_; }
    std::string_view replacement() const { return *replacement_; }
    friend bool operator==(const RegexReplacement& a, const RegexReplacement& b) {
        return a.regex_ == b.regex_ && *a.replacement_ == *b.replacement_;
    }
    /// `%/pattern/replacement/flags`.
    void show(Printer& p) const;

private:
    RegexReplacement(Regex r, Rc<const std::string> text, Rc<const std::vector<detail::ReplacementPiece>> pieces)
        : regex_(std::move(r)), replacement_(std::move(text)), pieces_(std::move(pieces)) {}
    Regex regex_;
    Rc<const std::string> replacement_;
    Rc<const std::vector<detail::ReplacementPiece>> pieces_;
};

}  // namespace prism_rtl
