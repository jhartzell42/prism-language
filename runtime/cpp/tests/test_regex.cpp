// The regular-expression corpus (`tests/regex/corpus.txt`, its format in
// `tests/regex/README.md`) against the C++ runtime, plus what a corpus line cannot say:
// views, sharing, `?Regex`, printing, and the linear-time guard. The twin of
// runtime/rust/tests/regex.rs.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include <chrono>
#include <fstream>
#include <sstream>
#include <string>
#include <vector>

#include "check.hpp"
#include "prism_rtl/print.hpp"
#include "prism_rtl/regex.hpp"
#include "prism_rtl/result.hpp"

using namespace prism_rtl;

namespace {

struct Fail {
    std::string why;
};

/// A corpus field, decoded: `⟨⟩` empty, `⟨-⟩` absent, `⟨hex⟩` a code point.
std::optional<std::string> decode(const std::string& field) {
    static const std::string open = "⟨", close = "⟩";
    if (field == "⟨-⟩") return std::nullopt;
    std::string out;
    std::size_t i = 0;
    while (true) {
        std::size_t o = field.find(open, i);
        if (o == std::string::npos) break;
        out += field.substr(i, o - i);
        std::size_t c = field.find(close, o);
        if (c == std::string::npos) throw Fail{"an unclosed ⟨ in the corpus"};
        std::string hex = field.substr(o + open.size(), c - o - open.size());
        if (!hex.empty()) utf8_push(out, char32_t(std::stoul(hex, nullptr, 16)));
        i = c + close.size();
    }
    out += field.substr(i);
    return out;
}

/// The byte offset of `part` in `whole`, which it must be a view of, not a copy.
std::size_t offset_in(const Text& whole, const Text& part) {
    auto base = reinterpret_cast<std::uintptr_t>(whole.view().data());
    auto at = reinterpret_cast<std::uintptr_t>(part.view().data());
    if (at < base || at + part.len() > base + whole.len()) throw Fail{"\"" + part.to_string() + "\" is not a view of the searched text"};
    return std::size_t(at - base);
}

std::string span(const Text& whole, const Text& text, const Cursor& front, const Cursor& back) {
    std::size_t a = front.at(), b = back.at();
    std::string ab = std::to_string(a) + "-" + std::to_string(b);
    if (a > b || b > whole.len() || whole.view().substr(a, b - a) != text.view()) throw Fail{"cursors " + ab + " do not hold the text"};
    if (!text.is_empty() && offset_in(whole, text) != a) throw Fail{"the text of " + ab + " is a view of other bytes"};
    if (front.before().len() != a || back.after().len() != whole.len() - b) throw Fail{"the cursors of " + ab + " are not cursors into the searched text"};
    return ab;
}

std::string group(const Text& whole, const Group& g) { return span(whole, g.text, g.front, g.back); }

std::string render(const Text& whole, const Match& m) {
    std::string out = span(whole, m.text, m.front, m.back);
    if (m.groups.empty() || !m.groups[0] || group(whole, *m.groups[0]) != out) throw Fail{"groups[0] is not the whole match"};
    for (std::size_t i = 1; i < m.groups.size(); ++i) out += " " + (m.groups[i] ? group(whole, *m.groups[i]) : std::string("nil"));
    for (const auto& [name, g] : m.named) out += " " + name.to_string() + "=" + group(whole, g);
    return out;
}

std::string run(const std::string& kind, const std::string& pattern, const std::string& flags, const std::optional<std::string>& arg,
                const std::optional<std::string>& in) {
    RegexError e{};
    if (kind == "refuse") {
        bool made = arg ? RegexReplacement::compile(pattern, flags, *arg, &e).has_value() : Regex::compile(pattern, flags, &e).has_value();
        if (made) return "accepted";
        return std::string(to_string(e.part)) + "@" + std::to_string(e.at) + " " + std::string(e.message);
    }
    if (!in) throw Fail{"no input"};
    Text input(*in);
    if (kind == "replace") {
        auto r = RegexReplacement::compile(pattern, flags, arg.value_or(""), &e);
        if (!r) throw Fail{"refused: " + std::string(e.message)};
        return r->replace(input).to_string();
    }
    auto re = Regex::compile(pattern, flags, &e);
    if (!re) throw Fail{"refused: " + std::string(e.message)};
    if (kind == "find") {
        auto m = re->find(input);
        return m ? render(input, *m) : "nil";
    }
    if (kind == "at") {
        auto c = Cursor::at_byte(input, std::stoul(arg.value()));
        if (!c) throw Fail{"the offset is not a cursor"};
        auto m = re->match_at(*c);
        return m ? render(input, *m) : "nil";
    }
    if (kind == "all") {
        auto ms = re->matches(input);
        if (ms.empty()) return "none";
        std::string out;
        for (const auto& m : ms) out += (out.empty() ? "[" : " [") + render(input, m) + "]";
        return out;
    }
    if (kind == "split") {
        std::string out;
        for (const auto& p : re->split(input)) {
            std::size_t a = offset_in(input, p);
            out += (out.empty() ? "" : " ") + std::to_string(a) + "-" + std::to_string(a + p.len());
        }
        return out;
    }
    throw Fail{"an unknown kind " + kind};
}

std::vector<std::string> fields(const std::string& line) {
    std::vector<std::string> out;
    std::size_t i = 0;
    while (true) {
        std::size_t t = line.find('\t', i);
        out.push_back(line.substr(i, t == std::string::npos ? std::string::npos : t - i));
        if (t == std::string::npos) return out;
        i = t + 1;
    }
}

}  // namespace

TEST(regex_corpus) {
    std::ifstream f(std::string(PRISM_TESTS_DIR) + "/regex/corpus.txt");
    CHECK(f.good());
    std::string line;
    int n = 0, cases = 0, failed = 0;
    while (std::getline(f, line)) {
        ++n;
        if (line.empty() || line[0] == '#') continue;
        auto fs = fields(line);
        CHECK_EQ(fs.size(), std::size_t(6));
        if (fs.size() != 6) continue;
        ++cases;
        std::string expected = decode(fs[5]).value(), got;
        try {
            got = run(fs[0], decode(fs[1]).value(), decode(fs[2]).value(), decode(fs[3]), decode(fs[4]));
        } catch (const Fail& e) {
            got = "error: " + e.why;
        } catch (const Panic& e) {
            got = std::string("panic: ") + e.what();
        }
        if (got != expected) {
            ++failed;
            std::cout << "  line " << n << ": " << fs[0] << " " << fs[1] << "\n    expected " << expected << "\n    got      " << got << "\n";
        }
    }
    if (failed) std::cout << "  " << failed << " of " << cases << " corpus cases fail\n";
    CHECK_EQ(failed, 0);
}

TEST(regex_runtime_pattern_is_nil_outside_the_dialect) {
    CHECK(!Regex::parse(Text("a**")));
    CHECK(!Regex::parse(Text("(?=a)")));
    auto re = Regex::parse(Text("(?i)k+"));
    CHECK(re.has_value());
    auto m = re->find(Text("xKk"));
    CHECK(m.has_value() && m->text == Text("Kk"));
}

TEST(regex_shared_not_copied_and_equal_by_spelling) {
    Regex a = *Regex::compile("[a-z]+", "i");
    Regex b = a;
    CHECK(a.same(b));
    CHECK(a == *Regex::compile("[a-z]+", "i"));
    CHECK(!(a == *Regex::compile("[a-z]+", "")));
    CHECK(!(a == *Regex::compile("[a-y]+", "i")));
    auto r = *RegexReplacement::compile("a", "", "b");
    CHECK(r == *RegexReplacement::compile("a", "", "b"));
    CHECK(!(r == *RegexReplacement::compile("a", "", "c")));
}

TEST(regex_prints_as_the_books_say) {
    CHECK_EQ(show(*Regex::compile("a/b\\/c", "im")), std::string("%/a\\/b\\/c/im"));
    CHECK_EQ(show(*RegexReplacement::compile("_", "", "-/")), std::string("%/_/-\\//"));
    auto m = Regex::compile("(b)(x)?", "")->find(Text("ab"));
    std::string shown = show(*m);
    CHECK(shown.starts_with("Match(text=\"b\", front=Cursor(at=1), back=Cursor(at=2), groups=[["));
    CHECK(shown.find("Group(text=\"b\", front=Cursor(at=1), back=Cursor(at=2))") != std::string::npos);
    CHECK(shown.ends_with(", nil]], named={})"));
}

TEST(regex_match_from_a_view_reports_cursors_into_that_view) {
    Text whole("xx abc");
    Text view = Cursor::at_byte(whole, 3)->after();
    auto m = Regex::compile("b", "")->find(view);
    CHECK(m && m->front.at() == 1 && m->back.at() == 2 && m->front.before() == Text("a"));
    CHECK(Regex::compile("^a", "")->match_at(*Cursor::at_byte(view, 0)).has_value());
}

TEST(regex_bad_literal_is_a_fatal_error) {
    bool panicked = false;
    try {
        (void)Regex::lit("a**", "");
    } catch (const Panic& e) {
        panicked = std::string(e.what()).find("a regex literal outside the dialect") != std::string::npos;
    }
    CHECK(panicked);
}

TEST(regex_searches_are_linear_in_the_text) {
    auto re = *Regex::compile("(x+x+)+y", "");
    Text text(std::string(100000, 'x'));
    auto start = std::chrono::steady_clock::now();
    CHECK(!re.find(text));
    CHECK(!re.match_at(Cursor::front(text)));
    double took = std::chrono::duration<double>(std::chrono::steady_clock::now() - start).count();
    CHECK(took < 10.0);
}
