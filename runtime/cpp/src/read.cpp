// The reader (read.hpp).

#include "prism_rtl/read.hpp"

#include <algorithm>
#include <charconv>
#include <limits>
#include <optional>
#include <vector>

#include "prism_rtl/print.hpp"

namespace prism_rtl {

bool char_is_whitespace(char32_t c) {
    if (c < 0x80) return c == ' ' || (c >= 0x09 && c <= 0x0D);
    return c == 0x85 || c == 0xA0 || c == 0x1680 || (c >= 0x2000 && c <= 0x200A) || c == 0x2028 || c == 0x2029 ||
           c == 0x202F || c == 0x205F || c == 0x3000;
}
bool char_is_alphabetic(char32_t c) {
    if (c < 0x80) return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
    return !char_is_whitespace(c) && !(c >= 0x80 && c <= 0x9F);
}
bool char_is_alphanumeric(char32_t c) { return char_is_alphabetic(c) || (c >= '0' && c <= '9'); }
bool char_is_uppercase(char32_t c) { return c >= 'A' && c <= 'Z'; }

// ---- Patterns ------------------------------------------------------------------------------

/// The wildcards, as the reader writes them into a pattern's value.
static const char* const ANY = "ignore";
static const char* const REST = "..";

Pattern Pattern::any() { return Pattern{Pod::name(ANY)}; }

static bool matches(const Pod& p, const Pod& whole);

static bool all(const std::vector<Pod>& ps, const std::vector<Pod>& vs) {
    if (ps.size() != vs.size()) return false;
    for (std::size_t i = 0; i < ps.size(); ++i)
        if (!matches(ps[i], vs[i])) return false;
    return true;
}

static bool fields(const Fields& ps, const Fields& fs) {
    bool rest = std::any_of(ps.begin(), ps.end(), [](const auto& f) { return f.first == REST; });
    if (!rest && ps.size() != fs.size()) return false;
    for (const auto& [n, p] : ps) {
        if (n == REST) continue;
        bool found = std::any_of(fs.begin(), fs.end(), [&](const auto& f) { return f.first == n && matches(p, f.second); });
        if (!found) return false;
    }
    return true;
}

static bool matches(const Pod& p, const Pod& whole) {
    const Pod& v = whole.structure();
    using K = Pod::Kind;
    if (auto c = p.if_choice(); c && c->ctor == ANY && c->args.empty()) return true;
    if (p.is(K::Path) && v.is(K::Path)) return print_path(*p.if_path()) == print_path(*v.if_path());
    if (p.is(K::Element) && v.is(K::Element)) {
        const std::string& t = p.if_element()->name;
        if (t.size() >= 2 && t.ends_with(".*")) {
            const std::string& u = v.if_element()->name;
            auto dot = u.rfind('.');
            return dot != std::string::npos && u.substr(0, dot) == t.substr(0, t.size() - 2);
        }
    }
    if (p.is(K::Record) && v.is(K::Record))
        return p.if_record()->name == v.if_record()->name && fields(p.if_record()->fields, v.if_record()->fields);
    if (p.is(K::Element) && v.is(K::Element))
        return p.if_element()->name == v.if_element()->name && fields(p.if_element()->fields, v.if_element()->fields);
    // A bare qualified name is a fault kind, or an element with no fields.
    if (p.is(K::Choice) && v.is(K::Element))
        return p.if_choice()->args.empty() && v.if_element()->fields.empty() && p.if_choice()->ctor == v.if_element()->name;
    if (p.is(K::Choice) && v.is(K::Choice))
        return p.if_choice()->ctor == v.if_choice()->ctor && all(p.if_choice()->args, v.if_choice()->args);
    if (p.is(K::Tuple) && v.is(K::Tuple)) return all(*p.if_tuple(), *v.if_tuple());
    if (p.is(K::Array) && v.is(K::Array)) return all(*p.if_array(), *v.if_array());
    if (p.is(K::Map) && v.is(K::Map)) {
        const auto &ps = *p.if_map(), &es = *v.if_map();
        if (ps.size() != es.size()) return false;
        for (std::size_t i = 0; i < ps.size(); ++i)
            if (!matches(ps[i].first, es[i].first) || !matches(ps[i].second, es[i].second)) return false;
        return true;
    }
    if (p.is(K::Some) && v.is(K::Some)) return matches(*p.if_some(), *v.if_some());
    if (p.is(K::Ok) && v.is(K::Ok)) return matches(*p.if_ok(), *v.if_ok());
    if (p.is(K::Failed) && v.is(K::Failed)) return matches(*p.if_failed(), *v.if_failed());
    return p == v;
}

bool Pattern::matches(const Pod& v) const { return prism_rtl::matches(pod, v); }

// ---- The lexer -----------------------------------------------------------------------------

namespace {

struct Tok {
    enum Kind { Str, Char, Num, Float, Dur, Name, P } kind;
    std::string s;  // Str, Name, P
    char32_t c = 0;
    BigInt n;
    double f = 0;
    Nanos d = 0;

    Tok(Kind k, std::string s) : kind(k), s(std::move(s)) {}
    bool is_p(std::string_view p) const { return kind == P && s == p; }
    std::string debug() const {
        switch (kind) {
        case Str: return "Str(" + show(Text(s)) + ")";
        case Char: return "Char(" + show(c) + ")";
        case Num: return "Num(" + n.to_string() + ")";
        case Float: return "Float(" + float_digits(f) + ")";
        case Dur: return "Dur";
        case Name: return "Name(\"" + s + "\")";
        case P: return "P(\"" + s + "\")";
        }
        return "?";
    }
};

using Err = std::string;

const std::pair<const char*, Nanos> UNITS[] = {
    {"h", Nanos(3600000000000)}, {"m", Nanos(60000000000)}, {"s", Nanos(1000000000)},
    {"ms", Nanos(1000000)},      {"us", Nanos(1000)},       {"ns", Nanos(1)},
};

bool is_digit(char32_t c) { return c >= '0' && c <= '9'; }

Expected<std::vector<Tok>, Err> lex(std::string_view src) {
    std::u32string cs = utf8_decode(src);
    std::size_t i = 0;
    std::vector<Tok> out;
    auto at = [&](std::size_t j) -> std::optional<char32_t> {
        if (j < cs.size()) return cs[j];
        return std::nullopt;
    };
    auto word = [&](std::size_t j) { return j < cs.size() && (char_is_alphanumeric(cs[j]) || cs[j] == '_'); };
    auto narrow = [&](std::size_t a, std::size_t b) { return utf8_encode(std::u32string_view(cs).substr(a, b - a)); };
    while (i < cs.size()) {
        char32_t c = cs[i];
        if (char_is_whitespace(c)) {
            ++i;
        } else if (c == '"' || c == '\'') {
            char32_t quote = c;
            std::u32string s;
            ++i;
            for (;;) {
                if (i >= cs.size()) return unexpected(Err("unterminated quote"));
                char32_t d = cs[i++];
                if (d == quote) break;
                if (d != '\\') {
                    s.push_back(d);
                    continue;
                }
                if (i >= cs.size()) return unexpected(Err("dangling escape"));
                char32_t e = cs[i++];
                if (e == 'n') s.push_back('\n');
                else if (e == 't') s.push_back('\t');
                else if (e == 'u') {
                    std::size_t end = i;
                    while (end < cs.size() && cs[end] != '}') ++end;
                    if (end >= cs.size()) return unexpected(Err("unterminated \\u{"));
                    std::string hex = narrow(i + 1, end);
                    std::uint32_t code = 0;
                    auto r = std::from_chars(hex.data(), hex.data() + hex.size(), code, 16);
                    if (hex.empty() || r.ec != std::errc() || r.ptr != hex.data() + hex.size())
                        return unexpected(Err("invalid digit found in string"));
                    if (code > 0x10FFFF || (code >= 0xD800 && code <= 0xDFFF)) return unexpected(Err("bad code point"));
                    s.push_back(char32_t(code));
                    i = end + 1;
                } else s.push_back(e);
            }
            if (quote == '"') out.push_back(Tok{Tok::Str, utf8_encode(s)});
            else if (s.size() == 1) {
                Tok t{Tok::Char, ""};
                t.c = s[0];
                out.push_back(t);
            } else return unexpected(Err("a character literal holds one character: '" + utf8_encode(s) + "'"));
        } else if (c == '-' && at(i + 1) == U'i' && at(i + 2) == U'n' && at(i + 3) == U'f' && !word(i + 4)) {
            Tok t{Tok::Float, ""};
            t.f = -std::numeric_limits<double>::infinity();
            out.push_back(t);
            i += 4;
        } else if (is_digit(c) || (c == '-' && at(i + 1) && is_digit(*at(i + 1)))) {
            std::size_t start = i++;
            while (at(i) && is_digit(*at(i))) ++i;
            if (at(i) == U'.' && at(i + 1) && is_digit(*at(i + 1))) {
                ++i;
                while (at(i) && is_digit(*at(i))) ++i;
                std::string t = narrow(start, i);
                Tok tok{Tok::Float, ""};
                auto r = std::from_chars(t.data(), t.data() + t.size(), tok.f);
                if (r.ec == std::errc::result_out_of_range) {
                    // Rust's parse rounds an overflow to infinity and an underflow to zero.
                    bool big = t.find_first_of("123456789") < t.find('.');
                    tok.f = !big ? 0.0 : std::numeric_limits<double>::infinity();
                    if (t[0] == '-') tok.f = -tok.f;
                } else if (r.ec != std::errc()) return unexpected(Err("bad number " + t));
                out.push_back(tok);
                continue;
            }
            std::string digits = narrow(start, i);
            auto n = BigInt::parse(digits);
            if (!n) return unexpected(Err("bad number " + digits));
            std::size_t unit_start = i;
            while (at(i) && *at(i) < 0x80 && char_is_alphabetic(*at(i))) ++i;
            std::string unit = narrow(unit_start, i);
            if (unit.empty()) {
                Tok t{Tok::Num, ""};
                t.n = *n;
                out.push_back(t);
            } else {
                const Nanos* scale = nullptr;
                for (const auto& [u, s] : UNITS)
                    if (unit == u) scale = &s;
                if (!scale) return unexpected(Err("unknown duration unit " + unit));
                auto small = n->to_i64();
                if (!small) return unexpected(Err("duration out of range"));
                Tok t{Tok::Dur, ""};
                t.d = Nanos(*small) * *scale;
                out.push_back(t);
            }
        } else if (char_is_alphabetic(c) || c == '_') {
            std::size_t start = i;
            while (i < cs.size()) {
                if (word(i)) {
                    ++i;
                } else if (cs[i] == '.' && at(i + 1) && (char_is_alphabetic(*at(i + 1)) || *at(i + 1) == '*')) {
                    ++i;
                    if (cs[i] == '*') {
                        ++i;
                        break;
                    }
                } else break;
            }
            out.push_back(Tok{Tok::Name, narrow(start, i)});
        } else {
            std::string two = narrow(i, std::min(i + 2, cs.size()));
            bool done = false;
            for (const char* p : {"((", "))", "[[", "]]", ".."})
                if (two == p) {
                    out.push_back(Tok{Tok::P, p});
                    i += 2;
                    done = true;
                    break;
                }
            if (done) continue;
            const char* found = nullptr;
            for (const char* p : {"(", ")", "[", "]", "{", "}", ",", ":", "=", "?", "~", "/", "#"})
                if (char32_t(p[0]) == c) found = p;
            if (!found) return unexpected(Err("unexpected character " + show(c)));
            // A `(` that touches a name opens fields: `Card(rank=Four)`, where
            // `Node (Leaf 1)` applies a constructor.
            bool touches = std::string_view(found) == "(" && i > 0 && word(i - 1);
            out.push_back(Tok{Tok::P, touches ? "call(" : found});
            ++i;
        }
    }
    return out;
}

// ---- The parser ----------------------------------------------------------------------------

struct Parser {
    std::vector<Tok> toks;
    std::size_t at = 0;
    /// Half of a `))` has been consumed as a `)`.
    bool split_close = false;

    Expected<Unit, Err> done(std::string_view src) const {
        if (at < toks.size() && !split_close)
            return unexpected(Err("unexpected " + toks[at].debug() + " in `" + std::string(src) + "`"));
        return Unit{};
    }
    const Tok* peek() const {
        static const Tok close{Tok::P, ")"};
        if (split_close) return &close;
        return at < toks.size() ? &toks[at] : nullptr;
    }
    std::string peek_debug() const { return peek() ? "Some(" + peek()->debug() + ")" : "None"; }
    std::optional<Tok> next() {
        std::optional<Tok> t;
        if (auto p = peek()) t = *p;
        split_close = false;
        ++at;
        return t;
    }
    bool is(std::string_view p) const { return peek() && peek()->is_p(p); }
    bool eat(std::string_view p) {
        bool yes = is(p);
        if (yes) next();
        return yes;
    }
    Expected<Unit, Err> expect(std::string_view p) {
        if (eat(p)) return Unit{};
        return unexpected(Err("expected " + std::string(p) + ", found " + peek_debug()));
    }
    /// A `)`, splitting a `))` if that is what comes.
    Expected<Unit, Err> close_paren() {
        if (is("))") && !split_close) {
            split_close = true;
            return Unit{};
        }
        return expect(")");
    }
    bool starts_atom() const {
        const Tok* t = peek();
        if (!t) return false;
        if (t->kind == Tok::P) return t->s == "(" || t->s == "((" || t->s == "[[" || t->s == "{";
        return true;
    }
    /// An uppercase constructor that is not a record, an element or a Bool.
    std::optional<std::string> bare_ctor() const {
        const Tok* t = peek();
        if (!t || t->kind != Tok::Name || split_close) return std::nullopt;
        const std::string& n = t->s;
        bool upper = !n.empty() && char_is_uppercase(utf8_decode(n)[0]);
        bool call = at + 1 < toks.size() && toks[at + 1].is_p("call(");
        if (upper && n.find('.') == std::string::npos && !call && n != "True" && n != "False") return n;
        return std::nullopt;
    }
    /// A value; `app` lets a choice take several arguments with no parentheses, which the
    /// printer writes only as a whole value.
    Expected<Pod, Err> value(bool app) {
        if (eat("?")) {
            auto v = value(false);
            if (!v) return v;
            return Pod::some(*v);
        }
        if (eat("~")) {
            auto v = value(false);
            if (!v) return v;
            return Pod::ok(*v);
        }
        if (auto ctor = app ? bare_ctor() : std::nullopt) {
            next();
            std::vector<Pod> payload;
            if (starts_atom()) {
                do {
                    auto v = value(false);
                    if (!v) return v;
                    payload.push_back(*v);
                } while (eat(","));
            }
            return Pod::choice(*ctor, std::move(payload));
        }
        return atom();
    }
    /// A field's value: a constructor applied to one argument needs no parentheses.
    Expected<Pod, Err> field_value() {
        if (auto ctor = bare_ctor()) {
            next();
            std::vector<Pod> payload;
            if (starts_atom()) {
                auto v = value(false);
                if (!v) return v;
                payload.push_back(*v);
            }
            return Pod::choice(*ctor, std::move(payload));
        }
        return value(false);
    }
    Expected<std::vector<Pod>, Err> items(std::string_view close) {
        std::vector<Pod> out;
        if (!is(close)) {
            do {
                auto v = value(false);
                if (!v) return unexpected(v.error());
                out.push_back(*v);
            } while (eat(","));
        }
        if (auto e = expect(close); !e) return unexpected(e.error());
        return out;
    }
    Expected<Pod, Err> atom() {
        auto t = next();
        if (!t) return unexpected(Err("expected a value, found None"));
        switch (t->kind) {
        case Tok::Str: return Pod::text(Text(t->s));
        case Tok::Char: return Pod::character(t->c);
        case Tok::Dur: return Pod::duration(t->d);
        case Tok::Float: return Pod::floating(t->f);
        case Tok::Num:
            if (is("/")) {
                --at;
                auto p = path();
                if (!p) return unexpected(p.error());
                return Pod::path(*p);
            }
            return Pod::integer(Int(t->n));
        case Tok::Name: return named(t->s);
        case Tok::P:
            if (t->s == "(") {
                auto v = value(true);
                if (!v) return v;
                if (auto e = close_paren(); !e) return unexpected(e.error());
                return v;
            }
            if (t->s == "((") {
                auto xs = items("))");
                if (!xs) return unexpected(xs.error());
                return Pod::tuple(*xs);
            }
            if (t->s == "[[") {
                auto xs = items("]]");
                if (!xs) return unexpected(xs.error());
                return Pod::array(*xs);
            }
            if (t->s == "{") {
                Pod::Entries es;
                while (!is("}")) {
                    auto k = value(false);
                    if (!k) return k;
                    if (auto e = expect(":"); !e) return unexpected(e.error());
                    auto v = value(false);
                    if (!v) return v;
                    es.emplace_back(*k, *v);
                    if (!eat(",")) break;
                }
                if (auto e = expect("}"); !e) return unexpected(e.error());
                std::stable_sort(es.begin(), es.end(), [](const auto& a, const auto& b) { return a.first < b.first; });
                return Pod::map(std::move(es));
            }
            break;
        }
        return unexpected(Err("expected a value, found Some(" + t->debug() + ")"));
    }
    Expected<Pod, Err> named(const std::string& n) {
        if (n == "nil") return Pod::nil();
        if (n == "unit") return Pod::unit();
        if (n == "True" || n == "False") return Pod::boolean(n == "True");
        if (n == "nan") return Pod::floating(std::numeric_limits<double>::quiet_NaN());
        if (n == "inf") return Pod::floating(std::numeric_limits<double>::infinity());
        if (n.ends_with(".*")) return Pod::element(n, {});
        if (!eat("call(")) return Pod::name(n);
        Fields fs;
        while (!is(")") && !is("))")) {
            if (eat(REST)) {
                fs.emplace_back(REST, Pod::unit());
                break;
            }
            auto f = next();
            if (!f || f->kind != Tok::Name) return unexpected(Err("expected a field name in " + n + "(..)"));
            if (auto e = expect("="); !e) return unexpected(e.error());
            auto v = field_value();
            if (!v) return v;
            fs.emplace_back(f->s, *v);
            if (!eat(",")) break;
        }
        if (auto e = close_paren(); !e) return unexpected(e.error());
        if (n == "Error") return Pod::failed(Pod::record(n, std::move(fs)));
        if (n.find('.') != std::string::npos) return Pod::element(n, std::move(fs));
        return Pod::record(n, std::move(fs));
    }
    Expected<Path, Err> path() {
        std::vector<Segment> segs;
        auto n32 = [](const BigInt& n) -> std::optional<std::uint32_t> {
            auto v = n.to_i64();
            if (!v || *v < 0 || *v > 0xFFFFFFFFll) return std::nullopt;
            return std::uint32_t(*v);
        };
        for (;;) {
            auto t = next();
            if (t && t->kind == Tok::Num) {
                auto n = n32(t->n);
                if (!n) return unexpected(Err("a path segment out of range"));
                segs.push_back(Segment::pos(*n));
            } else if (t && t->is_p("#")) {
                auto g = next();
                if (!g || g->kind != Tok::Num)
                    return unexpected(Err("expected a generation after #, found " + (g ? "Some(" + g->debug() + ")" : std::string("None"))));
                auto n = n32(g->n);
                if (!n) return unexpected(Err("a path segment out of range"));
                segs.push_back(Segment::gen(*n));
            } else if (t && t->is_p("[")) {
                // A key that is a path prints as its notation, so read one back.
                bool is_path = peek() && peek()->kind == Tok::Num && at + 1 < toks.size() && toks[at + 1].is_p("/");
                Pod key;
                if (is_path) {
                    auto p = path();
                    if (!p) return p;
                    key = Pod::path(*p);
                } else {
                    auto v = value(true);
                    if (!v) return unexpected(v.error());
                    key = *v;
                }
                if (auto e = expect("]"); !e) return unexpected(e.error());
                segs.push_back(Segment::key(key));
            } else {
                return unexpected(Err("expected a path segment, found " + (t ? "Some(" + t->debug() + ")" : std::string("None"))));
            }
            if (!eat("/")) return Path(std::move(segs));
        }
    }
};

}  // namespace

namespace read {

Expected<Pod, std::string> value(std::string_view src) {
    auto toks = lex(src);
    if (!toks) return unexpected(toks.error());
    Parser p{std::move(toks).value()};
    auto v = p.value(true);
    if (!v) return v;
    if (auto d = p.done(src); !d) return unexpected(d.error());
    return v;
}

Expected<Pattern, std::string> pattern(std::string_view src) {
    auto v = value(src);
    if (!v) return unexpected(v.error());
    return Pattern{std::move(v).value()};
}

Expected<Path, std::string> path(std::string_view src) {
    auto toks = lex(src);
    if (!toks) return unexpected(toks.error());
    Parser p{std::move(toks).value()};
    auto path = p.path();
    if (!path) return path;
    if (auto d = p.done(src); !d) return unexpected(d.error());
    return path;
}

}  // namespace read
}  // namespace prism_rtl
