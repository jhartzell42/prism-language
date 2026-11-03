// The printer (print.hpp).

#include "prism_rtl/print.hpp"

#include <charconv>
#include <cmath>
#include <cstdio>

namespace prism_rtl {

/// Rust's `char::is_control`: the general category Cc.
static bool is_control(char32_t c) { return c < 0x20 || (c >= 0x7F && c <= 0x9F); }

void Printer::text(std::string_view s) { quoted(utf8_decode(s), U'"'); }

void Printer::quoted(std::u32string_view chars, char32_t quote) {
    utf8_push(out_, quote);
    for (char32_t c : chars) {
        if (c == U'\\') out_ += "\\\\";
        else if (c == U'\n') out_ += "\\n";
        else if (c == U'\t') out_ += "\\t";
        else if (c == quote) {
            out_ += '\\';
            utf8_push(out_, c);
        } else if (is_control(c)) {
            char buf[16];
            std::snprintf(buf, sizeof buf, "\\u{%x}", unsigned(c));
            out_ += buf;
        } else utf8_push(out_, c);
    }
    utf8_push(out_, quote);
}

void Printer::inner(ShowRef x, At at) {
    At outer = at_;
    at_ = at;
    x.show(*this);
    at_ = outer;
}

void Printer::list(std::string_view open, std::span<const ShowRef> items, std::string_view close) {
    out_ += open;
    for (std::size_t i = 0; i < items.size(); ++i) {
        if (i > 0) out_ += ", ";
        inner(items[i], At::Inner);
    }
    out_ += close;
}

void Printer::choice(std::string_view ctor, std::span<const ShowRef> args) {
    bool bare = args.empty() || at_ == At::Whole || (at_ == At::Field && args.size() == 1);
    if (!bare) out_ += '(';
    out_ += ctor;
    if (!args.empty()) list(" ", args, "");
    if (!bare) out_ += ')';
}

void Printer::record(std::string_view name, std::span<const std::pair<std::string_view, ShowRef>> fields) {
    out_ += name;
    if (fields.empty() && name.find('.') != std::string_view::npos) return;
    out_ += '(';
    for (std::size_t i = 0; i < fields.size(); ++i) {
        if (i > 0) out_ += ", ";
        out_ += fields[i].first;
        out_ += '=';
        inner(fields[i].second, At::Field);
    }
    out_ += ')';
}

void Printer::map(std::span<const std::pair<ShowRef, ShowRef>> entries) {
    out_ += '{';
    for (std::size_t i = 0; i < entries.size(); ++i) {
        if (i > 0) out_ += ", ";
        inner(entries[i].first, At::Inner);
        out_ += ": ";
        inner(entries[i].second, At::Inner);
    }
    out_ += '}';
}

void Printer::option(const ShowRef* x) {
    if (!x) out_ += "nil";
    else prefixed("? ", *x);
}

void Printer::prefixed(std::string_view prefix, ShowRef x) {
    out_ += prefix;
    inner(x, At::Inner);
}

std::string float_digits(double f) {
    if (std::isnan(f)) return "nan";
    if (std::isinf(f)) return f > 0 ? "inf" : "-inf";
    std::string out = std::signbit(f) ? "-" : "";
    if (f == 0) return out + "0";
    // The shortest round-trip digits and the decimal exponent, from `to_chars`'s
    // scientific form `d.ddde±XX`: the value is 0.d1d2..dn * 10^exp.
    char buf[64];
    auto r = std::to_chars(buf, buf + sizeof buf, std::fabs(f), std::chars_format::scientific);
    std::string_view sci(buf, std::size_t(r.ptr - buf));
    std::size_t e = sci.find('e');
    std::string digits;
    for (char c : sci.substr(0, e))
        if (c != '.') digits += c;
    int exp = 0;
    std::from_chars(sci.data() + e + 1 + (sci[e + 1] == '+'), sci.data() + sci.size(), exp);
    // On an exact tie between two shortest candidates `to_chars` rounds to even, and Rust
    // rounds up (its Dragon4 fallback's last digit). The exact expansion (a double has at
    // most 767 significant digits) shows a tie: the digits so far, then 5, then zeros.
    char exact[800];
    auto x = std::to_chars(exact, exact + sizeof exact, std::fabs(f), std::chars_format::scientific, 780);
    std::string_view xs(exact, std::size_t(x.ptr - exact));
    std::size_t xe = xs.find('e');
    std::string xdigits;
    for (char c : xs.substr(0, xe))
        if (c != '.') xdigits += c;
    int xexp = 0;
    std::from_chars(xs.data() + xe + 1 + (xs[xe + 1] == '+'), xs.data() + xs.size(), xexp);
    std::size_t n0 = digits.size();
    if (xexp == exp && xdigits.compare(0, n0, digits) == 0 && xdigits[n0] == '5' &&
        xdigits.find_first_not_of('0', n0 + 1) == std::string::npos) {
        std::size_t i = n0;
        while (i > 0 && digits[i - 1] == '9') digits[--i] = '0';
        if (i == 0) {
            digits.insert(digits.begin(), '1');
            exp += 1;
        } else digits[i - 1] += 1;
        while (digits.size() > 1 && digits.back() == '0') digits.pop_back();
    }
    exp += 1;
    int n = int(digits.size());
    if (exp <= 0) out += "0." + std::string(std::size_t(-exp), '0') + digits;
    else if (exp < n) out += digits.substr(0, std::size_t(exp)) + "." + digits.substr(std::size_t(exp));
    else out += digits + std::string(std::size_t(exp - n), '0');
    return out;
}

void Printer::float64(double f) {
    std::string s = float_digits(f);
    out_ += s;
    if (s.find('.') == std::string::npos && std::isfinite(f)) out_ += ".0";
}

/// An `__int128` in decimal.
static std::string i128_string(Nanos n) {
    if (n == 0) return "0";
    bool neg = n < 0;
    unsigned __int128 m = neg ? ~(unsigned __int128)n + 1 : (unsigned __int128)n;
    std::string s;
    while (m) {
        s.insert(s.begin(), char('0' + int(m % 10)));
        m /= 10;
    }
    return neg ? "-" + s : s;
}

void Printer::duration(Nanos ns) {
    static const std::pair<const char*, Nanos> units[] = {
        {"h", Nanos(3600000000000)}, {"m", Nanos(60000000000)}, {"s", Nanos(1000000000)},
        {"ms", Nanos(1000000)},      {"us", Nanos(1000)},       {"ns", Nanos(1)},
    };
    const char* name = "s";
    Nanos size = 1;
    if (ns != 0)
        for (auto [u, s] : units)
            if (ns % s == 0) {
                name = u;
                size = s;
                break;
            }
    out_ += i128_string(ns / size);
    out_ += name;
}

std::string show_ref(ShowRef x) {
    Printer p;
    x.show(p);
    return p.take();
}

std::string print_path(const Path& p) { return show(p); }

void Show<Path>::show(const Path& s, Printer& p) {
    auto segs = s.segments();
    for (std::size_t i = 0; i < segs.size(); ++i) {
        if (i > 0) p.raw("/");
        const Segment& seg = segs[i];
        switch (seg.kind()) {
        case Segment::Kind::Pos:
            p.raw(std::to_string(seg.number()));
            break;
        case Segment::Kind::Gen:
            p.raw("#" + std::to_string(seg.number()));
            break;
        case Segment::Kind::Key:
            p.raw("[");
            p.inner(seg.key_value(), Printer::At::Whole);
            p.raw("]");
            break;
        }
    }
}

static std::vector<std::pair<std::string_view, ShowRef>> fields_of(const Fields& fs) {
    std::vector<std::pair<std::string_view, ShowRef>> out;
    for (const auto& [n, v] : fs) out.emplace_back(n, ShowRef(v));
    return out;
}

static std::vector<ShowRef> items_of(const std::vector<Pod>& xs) { return std::vector<ShowRef>(xs.begin(), xs.end()); }

void Show<Pod>::show(const Pod& s, Printer& p) {
    using K = Pod::Kind;
    switch (s.kind()) {
    case K::Unit:
        return p.raw("unit");
    case K::Bool:
        return Show<bool>::show(*s.if_bool(), p);
    case K::Int:
        return Show<Int>::show(*s.if_int(), p);
    case K::Float:
        return p.float64(s.if_float()->v);
    case K::Char:
        return Show<char32_t>::show(*s.if_char(), p);
    case K::Text:
        return Show<Text>::show(*s.if_text(), p);
    case K::Duration:
        return p.duration(*s.if_duration());
    case K::Record:
        return p.record(s.if_record()->name, fields_of(s.if_record()->fields));
    case K::Element:
        return p.record(s.if_element()->name, fields_of(s.if_element()->fields));
    case K::Choice:
        return p.choice(s.if_choice()->ctor, items_of(s.if_choice()->args));
    case K::Tuple:
        return p.tuple(items_of(*s.if_tuple()));
    case K::Array:
        return p.array(items_of(*s.if_array()));
    case K::Map:
        return p.map_of(*s.if_map());
    case K::Nil:
        return p.option(nullptr);
    case K::Some: {
        ShowRef r(*s.if_some());
        return p.option(&r);
    }
    case K::Ok:
        return p.prefixed("~ ", *s.if_ok());
    case K::Failed:
        return Show<Pod>::show(*s.if_failed(), p);
    case K::Path:
        return Show<Path>::show(*s.if_path(), p);
    case K::Shown:
        return p.raw(s.if_shown()->text);
    }
}

}  // namespace prism_rtl
