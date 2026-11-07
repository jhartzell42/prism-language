// Regular expressions over RE2 (include/prism_rtl/regex.hpp): the dialect check of
// claude/regex-dialect.md, one left-to-right parse giving the first diagnostic or the
// pattern spelled for RE2 with the books' meaning, and the searches. The parse is
// runtime/rust/src/regex_dialect.rs's, function for function; the shared corpus
// (tests/regex/corpus.txt) holds both to every message and position.
//
// Lifetimes: RE2 sees the searched text as a `re2::StringPiece` (its own class in RE2 up to
// 2023-03, Ubuntu 24.04's; an alias of `absl::string_view` since, Homebrew's) into the
// `Text`'s buffer only for the length of one `RE2::Match` call, which the `Text` (held by the
// caller) outlives; the submatch views are turned into byte offsets before the call
// returns to Prism, and every `Text` and `Cursor` a match holds is a view counted on that
// buffer. A compiled `RE2` lives in its `RegexCompiled`, which an `Rc` owns; RE2's
// matching is const and keeps its caches inside the object, so no other state is shared.
// No statics but constant tables, so no initialisation order to get wrong.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include "prism_rtl/regex.hpp"

#include <re2/re2.h>

#include <algorithm>
#include <array>
#include <cstdint>
#include <memory>
#include <tuple>
#include <utility>

#include "prism_rtl/result.hpp"

namespace prism_rtl {

namespace {

// The messages and names, word for word as runtime/rust/src/regex_dialect.rs and
// claude/regex-dialect.md have them; the corpus checks the messages.
constexpr std::string_view UNKNOWN_FLAG = "an unknown flag: the flags are i, m and s";
constexpr std::string_view FLAG_TWICE = "a flag written twice";
constexpr std::string_view STRAY_CLOSE_BRACKET = "a ] outside a class: a literal one is \\]";
constexpr std::string_view STRAY_CLOSE_BRACE = "a } outside a count: a literal one is \\}";
constexpr std::string_view TRAILING_BACKSLASH = "a \\ at the end of the pattern";
constexpr std::string_view HEX_TWO = "a \\x escape needs two hex digits or braces";
constexpr std::string_view HEX_BRACES = "a \\x{...} escape needs one to six hex digits";
constexpr std::string_view HEX_NO_CHAR = "a \\x escape names no character: a surrogate or past U+10FFFF";
constexpr std::string_view WORD_BRACES = "\\b{...} is Rust's only: write \\b";
constexpr std::string_view ANGLE = "\\< and \\> are Rust's only: write \\b";
constexpr std::string_view BACKREFERENCE = "a backreference: refused so that every match is linear in its text";
constexpr std::string_view OCTAL = "an octal escape is RE2's only: write \\x";
constexpr std::string_view QUOTE = "\\Q...\\E is RE2's only: escape each character";
constexpr std::string_view ONE_BYTE = "\\C, one byte, is RE2's only";
constexpr std::string_view UNKNOWN_ESCAPE = "an unknown escape";
constexpr std::string_view PROPERTY_NAME = "a \\p escape needs a property name";
constexpr std::string_view PROPERTY_OPEN = "a \\p{ that is never closed";
constexpr std::string_view PROPERTY_NEGATED = "a negated property is written \\P{...}";
constexpr std::string_view PROPERTY_REFUSED = "the property is refused: RE2 and Rust disagree on C, Cn, Cs and LC";
constexpr std::string_view PROPERTY_UNKNOWN = "an unknown property: general categories by short name, scripts and Any";
constexpr std::string_view CLASS_OPEN = "a class opened here is never closed";
constexpr std::string_view CLASS_FIRST_BRACKET = "a ] first in a class: write \\]";
constexpr std::string_view CLASS_NESTED = "a [ inside a class: write \\[ (nested classes are Rust's only)";
constexpr std::string_view CLASS_ASCII_UNKNOWN = "an unknown ASCII class";
constexpr std::string_view CLASS_SET_OPERATION = "&&, ~~ and -- in a class are Rust's set operations: escape them";
constexpr std::string_view CLASS_DASH = "a - in a class that is neither a range nor first or last: write \\-";
constexpr std::string_view CLASS_ORDER = "a range whose ends are out of order";
constexpr std::string_view CLASS_RANGE_SET = "a range end that is a class";
constexpr std::string_view CLASS_NEGATED_UNDER_I = "a negated class inside a class under i: write it outside the class";
constexpr std::string_view CLASS_ESCAPE = "an escape that a class cannot hold";
constexpr std::string_view LOOKAROUND = "a lookaround: refused so that every match is linear in its text";
constexpr std::string_view UNKNOWN_GROUP = "an unknown group: the groups are (re), (?:re), (?<name>re), (?P<name>re) and the flag groups";
constexpr std::string_view NAME_SHAPE = "a group name is a lowercase value name, [a-z_][a-z0-9_]*";
constexpr std::string_view NAME_OPEN = "a group name that is never closed";
constexpr std::string_view NAME_TWICE = "a group name used twice";
constexpr std::string_view GROUP_OPEN = "a group opened here is never closed";
constexpr std::string_view GROUP_CLOSE = "a ) that closes no group";
constexpr std::string_view FLAGS_EMPTY = "an empty flag group";
constexpr std::string_view FLAGS_SECOND_DASH = "a second - in a flag group";
constexpr std::string_view FLAGS_DASH = "a - that turns off no flag";
constexpr std::string_view BRACE = "a brace that opens no count: a literal one is \\{";
constexpr std::string_view COUNT_ZERO = "a count with a leading zero";
constexpr std::string_view COUNT_BOUND = "a count above 1000";
constexpr std::string_view COUNT_ORDER = "a count whose minimum exceeds its maximum";
constexpr std::string_view NOTHING_TO_REPEAT = "a repetition with nothing to repeat";
constexpr std::string_view REPEAT_REPEAT = "a repetition of a repetition: write (?:a*)* to say it";
constexpr std::string_view REPEAT_ASSERTION = "a repetition of an assertion";
constexpr std::string_view PRODUCT = "nested counts multiplying past 1000";
constexpr std::string_view NESTING = "groups and repetitions nested more than 100 deep";
constexpr std::string_view SIZE = "the pattern expands past 10,000 atoms";
constexpr std::string_view R_ZERO = "a group number with a leading zero";
constexpr std::string_view R_BRACES = "a ${...} that is neither a group number nor a group name";
constexpr std::string_view R_DOLLAR = "a $ that is not $n, ${n}, ${name} or $$";
constexpr std::string_view R_NO_NUMBER = "the pattern has no group of that number";
constexpr std::string_view R_NO_NAME = "the pattern has no group of that name";
constexpr std::string_view R_UNICODE = "a \\u{...} escape needs one to six hex digits naming a character";
constexpr std::string_view R_ESCAPE = "an unknown escape in a replacement: the escapes are \\\\, \\n, \\t, \\u{hex} and \\/";

/// The general categories both libraries read alike (doubt PC refuses `C`, `Cn`, `Cs`, `LC`).
constexpr std::string_view CATEGORIES[34] = {
    "Cc", "Cf", "Co", "L", "Ll", "Lm", "Lo", "Lt", "Lu", "M", "Mc", "Me", "Mn", "N", "Nd", "Nl",
    "No", "P", "Pc", "Pd", "Pe", "Pf", "Pi", "Po", "Ps", "S", "Sc", "Sk", "Sm", "So", "Z", "Zl",
    "Zp", "Zs"
};

/// Unicode 15.0's scripts, the names both libraries accept (doubt UN).
constexpr std::string_view SCRIPTS[163] = {
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
    "Zanabazar_Square"
};

constexpr std::uint32_t MAX_COUNT = 1000;
constexpr std::uint32_t MAX_DEPTH = 100;
constexpr std::uint64_t MAX_SIZE = 10000;

using Ranges = std::vector<std::pair<std::uint32_t, std::uint32_t>>;

/// The first diagnostic, thrown to `check`'s caller (the Rust parse's `?`).
struct Refused {
    RegexError error;
};

[[noreturn]] void refuse(RegexPart part, std::size_t at, std::string_view message) { throw Refused{RegexError{part, at, message}}; }
[[noreturn]] void err(std::size_t at, std::string_view message) { refuse(RegexPart::pattern, at, message); }
[[noreturn]] void e(std::size_t at, std::string_view message) { refuse(RegexPart::replacement, at, message); }

bool is_digit(unsigned char c) { return c >= '0' && c <= '9'; }
bool is_hex(unsigned char c) { return is_digit(c) || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F'); }
bool is_alpha(unsigned char c) { return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z'); }
bool is_lower(unsigned char c) { return c >= 'a' && c <= 'z'; }
bool is_upper(unsigned char c) { return c >= 'A' && c <= 'Z'; }
/// ASCII punctuation, as Rust's `u8::is_ascii_punctuation`.
bool is_punct(unsigned char c) { return (c >= 0x21 && c <= 0x2F) || (c >= 0x3A && c <= 0x40) || (c >= 0x5B && c <= 0x60) || (c >= 0x7B && c <= 0x7E); }

std::uint32_t hex_value(std::string_view d) {
    std::uint32_t v = 0;
    for (unsigned char c : d) v = v * 16 + std::uint32_t(is_digit(c) ? c - '0' : (c | 0x20) - 'a' + 10);
    return v;
}

bool is_name(std::string_view n) {
    if (n.empty()) return false;
    unsigned char f = n[0];
    if (!(is_lower(f) || f == '_')) return false;
    return std::all_of(n.begin() + 1, n.end(), [](unsigned char c) { return is_lower(c) || is_digit(c) || c == '_'; });
}

/// The code point at `s[i]` (well-formed UTF-8: a `Text`, or a literal prismc wrote) and
/// its length.
std::pair<std::uint32_t, std::size_t> char_at(std::string_view s, std::size_t i) {
    unsigned char c = s[i];
    if (c < 0x80) return {c, 1};
    std::size_t n = c >= 0xF0 ? 4 : c >= 0xE0 ? 3 : 2;
    std::uint32_t v = c & (0x7F >> n);
    for (std::size_t k = 1; k < n; ++k) v = (v << 6) | (static_cast<unsigned char>(s[i + k]) & 0x3F);
    return {v, n};
}

bool on_boundary(std::string_view s, std::size_t i) { return i >= s.size() || (static_cast<unsigned char>(s[i]) & 0xC0) != 0x80; }

const Ranges* ascii_class(std::string_view name) {
    static const std::array<std::pair<std::string_view, Ranges>, 14> classes = {{
        {"alnum", {{0x30, 0x39}, {0x41, 0x5A}, {0x61, 0x7A}}},
        {"alpha", {{0x41, 0x5A}, {0x61, 0x7A}}},
        {"ascii", {{0x00, 0x7F}}},
        {"blank", {{0x09, 0x09}, {0x20, 0x20}}},
        {"cntrl", {{0x00, 0x1F}, {0x7F, 0x7F}}},
        {"digit", {{0x30, 0x39}}},
        {"graph", {{0x21, 0x7E}}},
        {"lower", {{0x61, 0x7A}}},
        {"print", {{0x20, 0x7E}}},
        {"punct", {{0x21, 0x2F}, {0x3A, 0x40}, {0x5B, 0x60}, {0x7B, 0x7E}}},
        {"space", {{0x09, 0x0D}, {0x20, 0x20}}},
        {"upper", {{0x41, 0x5A}}},
        {"word", {{0x30, 0x39}, {0x41, 0x5A}, {0x5F, 0x5F}, {0x61, 0x7A}}},
        {"xdigit", {{0x30, 0x39}, {0x41, 0x46}, {0x61, 0x66}}},
    }};
    for (const auto& [n, r] : classes)
        if (n == name) return &r;
    return nullptr;
}

/// `\d`, `\w`, `\s` by their letter.
const Ranges& perl_class(unsigned char letter) {
    switch (letter | 0x20) {
        case 'd': return *ascii_class("digit");
        case 'w': return *ascii_class("word");
        default: return *ascii_class("space");
    }
}

Ranges complement(const Ranges& ranges) {
    Ranges out;
    std::uint32_t next = 0;
    for (auto [lo, hi] : ranges) {
        if (lo > next) out.push_back({next, lo - 1});
        next = hi + 1;
    }
    if (next <= 0x10FFFF) out.push_back({next, 0x10FFFF});
    return out;
}

/// A code point as RE2 is to read it: ASCII letters and digits as themselves, everything
/// else `\x{...}`, so that no character of the pattern is syntax to it.
void push_cp(std::string& out, std::uint32_t c) {
    if (c < 0x80 && (is_alpha(static_cast<unsigned char>(c)) || is_digit(static_cast<unsigned char>(c)))) {
        out += char(c);
        return;
    }
    static const char* digits = "0123456789abcdef";
    std::string hex;
    do {
        hex.insert(hex.begin(), digits[c & 0xF]);
        c >>= 4;
    } while (c);
    out += "\\x{" + hex + "}";
}

void push_ranges(std::string& out, const Ranges& ranges) {
    for (auto [lo, hi] : ranges) {
        push_cp(out, lo);
        if (hi != lo) {
            out += '-';
            push_cp(out, hi);
        }
    }
}

struct Flags {
    bool i = false, m = false, s = false;
};

enum class Last { nothing, flag_group, assertion, repetition, atom };

struct Measure {
    std::uint32_t depth = 0;
    std::uint64_t size = 0;
    std::uint32_t product = 0;
    Measure then(Measure o) const { return {std::max(depth, o.depth), size + o.size, std::max(product, o.product)}; }
};
constexpr Measure ATOM{0, 1, 1};

/// A class's item: code points, or a property (its name and whether negated).
struct Item {
    Ranges ranges;
    bool property = false, negated = false;
    std::string name;
};

/// One character read inside a class: a single code point, or a set.
struct ClassAtom {
    bool is_char;
    std::uint32_t cp = 0;
    Item set;
};

struct Checked {
    std::string spelling;
    std::size_t groups = 0;
    std::vector<std::pair<std::string, std::size_t>> names;
};

Flags check_flags(std::string_view flags) {
    Flags f;
    for (std::size_t at = 0; at < flags.size();) {
        auto [c, len] = char_at(flags, at);
        bool* slot = c == 'i' ? &f.i : c == 'm' ? &f.m : c == 's' ? &f.s : nullptr;
        if (!slot) refuse(RegexPart::flags, at, UNKNOWN_FLAG);
        if (*slot) refuse(RegexPart::flags, at, FLAG_TWICE);
        *slot = true;
        at += len;
    }
    return f;
}

class Parser {
public:
    explicit Parser(std::string_view s) : s_(s) {}
    std::string_view s_;
    std::size_t i_ = 0;
    std::uint32_t open_ = 0;
    std::size_t groups_ = 0;
    std::vector<std::pair<std::string, std::size_t>> names_;
    std::string out_;

    std::optional<unsigned char> peek(std::size_t k = 0) const {
        if (i_ + k < s_.size()) return static_cast<unsigned char>(s_[i_ + k]);
        return std::nullopt;
    }
    std::optional<unsigned char> byte(std::size_t j) const {
        if (j < s_.size()) return static_cast<unsigned char>(s_[j]);
        return std::nullopt;
    }

    /// Branches until the end or a `)`, which is left for the caller.
    Measure alternation(Flags flags) {
        Measure m = sequence(flags);
        while (peek() == '|') {
            ++i_;
            out_ += '|';
            m = m.then(sequence(flags));
        }
        return m;
    }

    Measure sequence(Flags& flags) {
        Measure total;
        Last last = Last::nothing;
        Measure last_m;
        while (auto c = peek()) {
            std::size_t at = i_;
            if (*c == '|' || *c == ')') break;
            if (*c == '*' || *c == '+' || *c == '?' || *c == '{') {
                std::uint32_t lo;
                std::optional<std::uint32_t> hi;
                if (*c == '{') {
                    std::tie(lo, hi) = count();
                } else {
                    ++i_;
                    lo = *c == '+' ? 1 : 0;
                    if (*c == '?') hi = 1;
                }
                switch (last) {
                    case Last::nothing:
                    case Last::flag_group: err(at, NOTHING_TO_REPEAT);
                    case Last::repetition: err(at, REPEAT_REPEAT);
                    case Last::assertion: err(at, REPEAT_ASSERTION);
                    case Last::atom: break;
                }
                bool counted = *c == '{';
                std::uint32_t factor = counted ? std::max(hi.value_or(lo), std::uint32_t(1)) : 1;
                std::uint32_t product = factor * std::max(last_m.product, std::uint32_t(1));
                if (counted && product > MAX_COUNT) err(at, PRODUCT);
                if (*c == '*') out_ += '*';
                else if (*c == '+') out_ += '+';
                else if (*c == '?') out_ += '?';
                else if (!hi) out_ += "{" + std::to_string(lo) + ",}";
                else if (*hi == lo) out_ += "{" + std::to_string(lo) + "}";
                else out_ += "{" + std::to_string(lo) + "," + std::to_string(*hi) + "}";
                if (peek() == '?') {
                    ++i_;
                    out_ += '?';
                }
                Measure m{last_m.depth + 1, last_m.size * factor, product};
                // The repeated piece replaces the piece it repeats in the sequence's sums.
                total = {std::max(total.depth, m.depth), total.size - last_m.size + m.size, std::max(total.product, m.product)};
                last = Last::repetition;
                last_m = m;
                continue;
            }
            auto [kind, m] = piece(flags);
            if (kind != Last::flag_group) total = total.then(m);
            last = kind;
            last_m = m;
        }
        return total;
    }

    /// A count after `{`: its shape, zeros, bound and order (no `hi`: no bound).
    std::pair<std::uint32_t, std::optional<std::uint32_t>> count() {
        std::size_t at = i_, j = at + 1;
        auto digits = [&](std::size_t& k) {
            std::size_t start = k;
            while (byte(k) && is_digit(*byte(k))) ++k;
            return s_.substr(start, k - start);
        };
        std::string_view first = digits(j);
        if (first.empty()) err(at, BRACE);
        std::optional<std::string_view> second;
        if (byte(j) == '}') {
        } else if (byte(j) == ',') {
            ++j;
            second = digits(j);
            if (byte(j) != '}') err(at, BRACE);
        } else {
            err(at, BRACE);
        }
        i_ = j + 1;
        auto zero = [](std::string_view d) { return d.size() > 1 && d[0] == '0'; };
        if (zero(first) || (second && zero(*second))) err(at, COUNT_ZERO);
        auto value = [](std::string_view d) {
            std::uint32_t v = 0;
            for (unsigned char c : d) v = std::min(v * 10 + std::uint32_t(c - '0'), MAX_COUNT + 1);
            return v;
        };
        std::uint32_t lo = value(first);
        std::optional<std::uint32_t> hi;
        if (!second) hi = lo;
        else if (!second->empty()) hi = value(*second);
        if (lo > MAX_COUNT || (hi && *hi > MAX_COUNT)) err(at, COUNT_BOUND);
        if (hi && lo > *hi) err(at, COUNT_ORDER);
        return {lo, hi};
    }

    std::pair<Last, Measure> piece(Flags& flags) {
        std::size_t at = i_;
        unsigned char c = *peek();
        switch (c) {
            case '(': return group(flags);
            case '[': class_(flags); return {Last::atom, ATOM};
            case ']': err(at, STRAY_CLOSE_BRACKET);
            case '}': err(at, STRAY_CLOSE_BRACE);
            case '.':
                ++i_;
                out_ += flags.s ? "(?s:.)" : "[^\\x{a}]";
                return {Last::atom, ATOM};
            case '^':
                ++i_;
                out_ += flags.m ? "(?m:^)" : "\\A";
                return {Last::assertion, ATOM};
            case '$':
                ++i_;
                out_ += flags.m ? "(?m:$)" : "\\z";
                return {Last::assertion, ATOM};
            case '\\': return escape(flags);
            default: {
                auto [cp, len] = char_at(s_, i_);
                i_ += len;
                literal(cp, flags);
                return {Last::atom, ATOM};
            }
        }
    }

    void literal(std::uint32_t cp, Flags flags) {
        if (flags.i) out_ += "(?i:";
        push_cp(out_, cp);
        if (flags.i) out_ += ')';
    }

    static std::uint32_t control(unsigned char c) {
        switch (c) {
            case 't': return 0x09;
            case 'n': return 0x0A;
            case 'r': return 0x0D;
            case 'f': return 0x0C;
            case 'v': return 0x0B;
            default: return 0x07;
        }
    }
    static bool is_control(unsigned char c) { return c == 't' || c == 'n' || c == 'r' || c == 'f' || c == 'v' || c == 'a'; }

    /// `\x..` after the `\` at `at`, the cursor on the `x`.
    std::uint32_t hex(std::size_t at) {
        ++i_;
        std::uint32_t v;
        if (peek() == '{') {
            std::size_t start = i_ + 1, j = start;
            while (byte(j) && is_hex(*byte(j))) ++j;
            std::size_t n = j - start;
            if (byte(j) != '}' || n == 0 || n > 6) err(at, HEX_BRACES);
            i_ = j + 1;
            v = hex_value(s_.substr(start, n));
        } else {
            auto a = peek(), b = peek(1);
            if (!(a && b && is_hex(*a) && is_hex(*b))) err(at, HEX_TWO);
            v = hex_value(s_.substr(i_, 2));
            i_ += 2;
        }
        if ((v >= 0xD800 && v <= 0xDFFF) || v > 0x10FFFF) err(at, HEX_NO_CHAR);
        return v;
    }

    /// `\p...` or `\P...` after the `\` at `at`, the cursor on the `p`: its name, checked.
    std::string property(std::size_t at) {
        ++i_;
        std::string_view name;
        auto c = peek();
        if (c == '{') {
            std::size_t start = i_ + 1;
            std::size_t close = s_.find('}', start);
            if (close == std::string_view::npos) err(at, PROPERTY_OPEN);
            name = s_.substr(start, close - start);
            i_ = close + 1;
            if (!name.empty() && name[0] == '^') err(at, PROPERTY_NEGATED);
        } else if (c && is_alpha(*c)) {
            name = s_.substr(i_, 1);
            ++i_;
        } else {
            err(at, PROPERTY_NAME);
        }
        if (name == "C" || name == "Cn" || name == "Cs" || name == "LC") err(at, PROPERTY_REFUSED);
        if (name == "Any" || std::find(std::begin(CATEGORIES), std::end(CATEGORIES), name) != std::end(CATEGORIES) ||
            std::find(std::begin(SCRIPTS), std::end(SCRIPTS), name) != std::end(SCRIPTS))
            return std::string(name);
        err(at, PROPERTY_UNKNOWN);
    }

    /// An escape outside a class.
    std::pair<Last, Measure> escape(Flags flags) {
        std::size_t at = i_;
        ++i_;
        auto oc = peek();
        if (!oc) err(at, TRAILING_BACKSLASH);
        unsigned char c = *oc;
        std::uint32_t cp = 0;
        if (c == '<' || c == '>') err(at, ANGLE);
        if (is_punct(c)) {
            ++i_;
            cp = c;
        } else if (is_control(c)) {
            ++i_;
            cp = control(c);
        } else if (c == 'x') {
            cp = hex(at);
        } else if (c == 'd' || c == 'w' || c == 's' || c == 'D' || c == 'W' || c == 'S') {
            ++i_;
            if (flags.i) out_ += "(?i:";
            out_ += '[';
            if (is_upper(c)) out_ += '^';
            push_ranges(out_, perl_class(c));
            out_ += ']';
            if (flags.i) out_ += ')';
            return {Last::atom, ATOM};
        } else if (c == 'p' || c == 'P') {
            std::string name = property(at);
            if (flags.i) out_ += "(?i:";
            out_ += std::string("\\") + char(c) + "{" + name + "}";
            if (flags.i) out_ += ')';
            return {Last::atom, ATOM};
        } else if (c == 'A' || c == 'z' || c == 'b' || c == 'B') {
            if (c == 'b' && peek(1) == '{') err(at, WORD_BRACES);
            ++i_;
            out_ += c == 'A' ? "\\A" : c == 'z' ? "\\z" : c == 'b' ? "\\b" : "\\B";
            return {Last::assertion, ATOM};
        } else if (c >= '1' && c <= '9') {
            err(at, BACKREFERENCE);
        } else if (c == '0') {
            err(at, OCTAL);
        } else if (c == 'Q' || c == 'E') {
            err(at, QUOTE);
        } else if (c == 'C') {
            err(at, ONE_BYTE);
        } else {
            err(at, UNKNOWN_ESCAPE);
        }
        literal(cp, flags);
        return {Last::atom, ATOM};
    }

    /// One character inside a class: a code point or an escape.
    ClassAtom class_atom(Flags flags) {
        std::size_t at = i_;
        if (peek() != '\\') {
            auto [cp, len] = char_at(s_, i_);
            i_ += len;
            return {true, cp, {}};
        }
        ++i_;
        auto oc = peek();
        if (!oc) err(at, TRAILING_BACKSLASH);
        unsigned char c = *oc;
        if (c == '<' || c == '>') err(at, CLASS_ESCAPE);
        if (is_punct(c)) {
            ++i_;
            return {true, c, {}};
        }
        if (is_control(c)) {
            ++i_;
            return {true, control(c), {}};
        }
        if (c == 'x') return {true, hex(at), {}};
        if (c == 'd' || c == 'w' || c == 's' || c == 'D' || c == 'W' || c == 'S') {
            ++i_;
            const Ranges& ranges = perl_class(c);
            if (is_upper(c)) {
                if (flags.i) err(at, CLASS_NEGATED_UNDER_I);
                return {false, 0, Item{complement(ranges), false, false, {}}};
            }
            return {false, 0, Item{ranges, false, false, {}}};
        }
        if (c == 'p' || c == 'P') {
            std::string name = property(at);
            if (c == 'P' && flags.i) err(at, CLASS_NEGATED_UNDER_I);
            return {false, 0, Item{{}, true, c == 'P', name}};
        }
        err(at, CLASS_ESCAPE);
    }

    /// An ASCII class `[:name:]` at the cursor (a `[` inside a class), or the nested-class
    /// diagnostic.
    Item ascii_item(Flags flags) {
        std::size_t at = i_, j = at + 1;
        if (byte(j) == ':') {
            ++j;
            bool negated = byte(j) == '^';
            if (negated) ++j;
            std::size_t start = j;
            while (byte(j) && is_alpha(*byte(j))) ++j;
            if (j > start && byte(j) == ':' && byte(j + 1) == ']') {
                const Ranges* ranges = ascii_class(s_.substr(start, j - start));
                if (!ranges) err(at, CLASS_ASCII_UNKNOWN);
                if (negated && flags.i) err(at, CLASS_NEGATED_UNDER_I);
                i_ = j + 2;
                return Item{negated ? complement(*ranges) : *ranges, false, false, {}};
            }
        }
        err(at, CLASS_NESTED);
    }

    /// The `&&`, `~~` and `--` refusal, at an unescaped character of a class.
    void set_operation() const {
        auto c = peek();
        if (c && (*c == '&' || *c == '~' || *c == '-') && peek(1) == c) err(i_, CLASS_SET_OPERATION);
    }

    void class_(Flags flags) {
        std::size_t open = i_;
        ++i_;
        bool negated = peek() == '^';
        if (negated) ++i_;
        std::vector<Item> items;
        bool first = true;
        while (true) {
            std::size_t at = i_;
            auto oc = peek();
            if (!oc) err(open, CLASS_OPEN);
            unsigned char c = *oc;
            if (c == ']') {
                if (first) err(at, CLASS_FIRST_BRACKET);
                ++i_;
                break;
            }
            set_operation();
            first = false;
            if (c == '[') {
                items.push_back(ascii_item(flags));
                continue;
            }
            if (c == '-') {
                if (!peek(1)) err(open, CLASS_OPEN);
                if (at == open + 1 + (negated ? 1 : 0) || peek(1) == ']') {
                    ++i_;
                    items.push_back(Item{{{0x2D, 0x2D}}, false, false, {}});
                    continue;
                }
                err(at, CLASS_DASH);
            }
            ClassAtom a = class_atom(flags);
            if (!a.is_char) {
                items.push_back(std::move(a.set));
                continue;
            }
            std::uint32_t lo = a.cp;
            bool ranged = peek() == '-' && peek(1) != ']';
            if (!ranged) {
                items.push_back(Item{{{lo, lo}}, false, false, {}});
                continue;
            }
            set_operation();
            ++i_;
            std::size_t right = i_;
            if (!peek()) err(open, CLASS_OPEN);
            if (peek() == '[') err(right, CLASS_NESTED);
            set_operation();
            ClassAtom b = class_atom(flags);
            if (!b.is_char) err(right, CLASS_RANGE_SET);
            if (b.cp < lo) err(at, CLASS_ORDER);
            items.push_back(Item{{{lo, b.cp}}, false, false, {}});
        }
        if (flags.i) out_ += "(?i:";
        out_ += '[';
        if (negated) out_ += '^';
        for (const Item& item : items) {
            if (item.property) out_ += std::string("\\") + (item.negated ? 'P' : 'p') + "{" + item.name + "}";
            else push_ranges(out_, item.ranges);
        }
        out_ += ']';
        if (flags.i) out_ += ')';
    }

    /// A group at its `(`; a bare flag group changes `flags` for the rest of the enclosing one.
    std::pair<Last, Measure> group(Flags& flags) {
        std::size_t open = i_;
        ++i_;
        Flags inner = flags;
        bool capture = false;
        std::optional<std::string> name;
        if (peek() != '?') {
            capture = true;
        } else {
            ++i_;
            auto oc = peek();
            if (!oc) err(open, GROUP_OPEN);
            unsigned char c = *oc;
            if (c == ':') {
                ++i_;
            } else if (c == '=' || c == '!') {
                err(open, LOOKAROUND);
            } else if (c == '<' && (peek(1) == '=' || peek(1) == '!')) {
                err(open, LOOKAROUND);
            } else if (c == '<') {
                ++i_;
                name = name_(open);
                capture = true;
            } else if (c == 'P') {
                auto n = peek(1);
                if (!n) err(open, GROUP_OPEN);
                if (*n == '<') {
                    i_ += 2;
                    name = name_(open);
                    capture = true;
                } else if (*n == '=' || *n == '>') {
                    err(open, BACKREFERENCE);
                } else {
                    err(open, UNKNOWN_GROUP);
                }
            } else if (is_alpha(c) || c == '-' || c == ')') {
                if (flag_group(open, inner)) {
                    flags = inner;
                    return {Last::flag_group, Measure{}};
                }
            } else {
                err(open, UNKNOWN_GROUP);
            }
        }
        if (capture) {
            ++groups_;
            if (name) names_.push_back({*name, groups_});
            out_ += '(';
        } else {
            out_ += "(?:";
        }
        if (open_ == MAX_DEPTH) err(0, NESTING);
        ++open_;
        Measure m = alternation(inner);
        if (peek() != ')') err(open, GROUP_OPEN);
        ++i_;
        --open_;
        out_ += ')';
        m.depth += 1;
        return {Last::atom, m};
    }

    /// A group's name, the cursor on its first byte.
    std::string name_(std::size_t open) {
        std::size_t start = i_;
        std::size_t close = s_.find('>', start);
        if (close == std::string_view::npos) err(open, NAME_OPEN);
        std::string_view n = s_.substr(start, close - start);
        if (!is_name(n)) err(start, NAME_SHAPE);
        for (const auto& [k, g] : names_)
            if (k == n) err(open, NAME_TWICE);
        i_ = close + 1;
        return std::string(n);
    }

    /// The flags of `(?flags)` or `(?flags:`, the cursor on the first; true for a bare group.
    bool flag_group(std::size_t open, Flags& flags) {
        bool seen[3] = {false, false, false};
        std::optional<std::size_t> dash;
        bool after_dash = false, any = false;
        while (true) {
            std::size_t at = i_;
            auto oc = peek();
            if (!oc) err(open, GROUP_OPEN);
            unsigned char c = *oc;
            if (c == ')' || c == ':') {
                if (!any && !dash) err(open, FLAGS_EMPTY);
                if (dash && !after_dash) err(*dash, FLAGS_DASH);
                ++i_;
                return c == ')';
            }
            if (c == '-') {
                if (dash) err(at, FLAGS_SECOND_DASH);
                dash = at;
            } else if (is_alpha(c)) {
                int k = c == 'i' ? 0 : c == 'm' ? 1 : c == 's' ? 2 : -1;
                if (k < 0) err(at, UNKNOWN_FLAG);
                if (seen[k]) err(at, FLAG_TWICE);
                seen[k] = true;
                any = true;
                after_dash |= dash.has_value();
                bool on = !dash;
                (k == 0 ? flags.i : k == 1 ? flags.m : flags.s) = on;
            } else {
                err(open, UNKNOWN_GROUP);
            }
            ++i_;
        }
    }
};

Checked check(std::string_view pattern, std::string_view flags) {
    Flags f = check_flags(flags);
    Parser p(pattern);
    Measure m = p.alternation(f);
    if (p.i_ < pattern.size()) err(p.i_, GROUP_CLOSE);
    if (m.depth > MAX_DEPTH) err(0, NESTING);
    if (m.size > MAX_SIZE) err(0, SIZE);
    return Checked{std::move(p.out_), p.groups_, std::move(p.names_)};
}

using detail::ReplacementPiece;

std::vector<ReplacementPiece> check_replacement(std::string_view r, std::size_t groups, const std::vector<std::pair<std::string, std::size_t>>& names) {
    std::vector<ReplacementPiece> out;
    std::string text;
    auto byte = [&](std::size_t j) -> std::optional<unsigned char> {
        if (j < r.size()) return static_cast<unsigned char>(r[j]);
        return std::nullopt;
    };
    auto number = [&](std::string_view d, std::size_t at) -> std::size_t {
        if (d.size() > 1 && d[0] == '0') e(at, R_ZERO);
        std::size_t n = 0;
        for (unsigned char c : d) n = n > (SIZE_MAX - 9) / 10 ? SIZE_MAX : n * 10 + (c - '0');
        if (n > groups) e(at, R_NO_NUMBER);
        return n;
    };
    std::size_t i = 0;
    while (i < r.size()) {
        std::size_t at = i;
        unsigned char c = r[i];
        if (c == '$') {
            std::size_t group = 0;
            auto n = byte(i + 1);
            if (n == '$') {
                text += '$';
                i += 2;
                continue;
            } else if (n && is_digit(*n)) {
                std::size_t j = i + 1;
                while (byte(j) && is_digit(*byte(j))) ++j;
                group = number(r.substr(i + 1, j - i - 1), at);
                i = j;
            } else if (n == '{') {
                std::size_t start = i + 2;
                std::size_t close = r.find('}', start);
                if (close == std::string_view::npos) e(at, R_BRACES);
                std::string_view inner = r.substr(start, close - start);
                i = close + 1;
                if (!inner.empty() && std::all_of(inner.begin(), inner.end(), [](unsigned char d) { return is_digit(d); })) {
                    group = number(inner, at);
                } else if (is_name(inner)) {
                    auto it = std::find_if(names.begin(), names.end(), [&](const auto& kv) { return kv.first == inner; });
                    if (it == names.end()) e(at, R_NO_NAME);
                    group = it->second;
                } else {
                    e(at, R_BRACES);
                }
            } else {
                e(at, R_DOLLAR);
            }
            if (!text.empty()) out.push_back({std::exchange(text, {}), -1});
            out.push_back({{}, long(group)});
        } else if (c == '\\') {
            auto n = byte(i + 1);
            if (n == '\\') text += '\\';
            else if (n == 'n') text += '\n';
            else if (n == 't') text += '\t';
            else if (n == '/') text += '/';
            else if (n == 'u') {
                std::size_t start = i + 3, j = start;
                while (byte(j) && is_hex(*byte(j))) ++j;
                std::size_t k = j - start;
                if (byte(i + 2) != '{' || byte(j) != '}' || k < 1 || k > 6) e(at, R_UNICODE);
                std::uint32_t v = hex_value(r.substr(start, k));
                if ((v >= 0xD800 && v <= 0xDFFF) || v > 0x10FFFF) e(at, R_UNICODE);
                utf8_push(text, char32_t(v));
                i = j + 1;
                continue;
            } else {
                e(at, R_ESCAPE);
            }
            i += 2;
        } else {
            auto [cp, len] = char_at(r, i);
            (void)cp;
            text += r.substr(i, len);
            i += len;
        }
    }
    if (!text.empty()) out.push_back({std::move(text), -1});
    return out;
}

/// The bytes `a..b` of `t` (character boundaries) as a view: no copy.
Text view(const Text& t, std::size_t a, std::size_t b) { return Cursor::at_byte(Cursor::at_byte(t, a)->after(), b - a)->before(); }

Group group_of(const Text& t, std::size_t a, std::size_t b) {
    auto front = Cursor::at_byte(t, a), back = Cursor::at_byte(t, b);
    if (!front || !back) panic("a regex group off a character boundary");
    return Group{view(t, a, b), *front, *back};
}

/// The plain literal: `%/pattern/flags` or `%/pattern/replacement/flags`, a bare `/`
/// written `\/` and a raw newline `\n`.
std::string spelling(std::string_view pattern, const std::string* replacement, std::string_view flags) {
    auto push = [](std::string& out, std::string_view s) {
        for (std::size_t i = 0; i < s.size(); ++i) {
            char c = s[i];
            if (c == '\\') {
                out += '\\';
                if (i + 1 < s.size()) out += s[++i];
            } else if (c == '/') {
                out += "\\/";
            } else if (c == '\n') {
                out += "\\n";
            } else {
                out += c;
            }
        }
    };
    std::string out = "%/";
    push(out, pattern);
    out += '/';
    if (replacement) {
        push(out, *replacement);
        out += '/';
    }
    out += flags;
    return out;
}

}  // namespace

namespace detail {
struct RegexCompiled {
    std::string pattern, flags;
    std::unique_ptr<re2::RE2> engine;
    /// Capture groups, group 0 not counted.
    std::size_t groups = 0;
    std::vector<std::pair<Text, std::size_t>> names;
    std::vector<std::pair<std::string, std::size_t>> plain_names;
};
}  // namespace detail

std::string_view to_string(RegexPart p) {
    switch (p) {
        case RegexPart::flags: return "flags";
        case RegexPart::pattern: return "pattern";
        default: return "replacement";
    }
}

std::string RegexError::to_string() const { return std::string(prism_rtl::to_string(part)) + "@" + std::to_string(at) + " " + std::string(message); }

std::optional<Regex> Regex::compile(std::string_view pattern, std::string_view flags, RegexError* error) {
    Checked checked;
    try {
        checked = check(pattern, flags);
    } catch (const Refused& r) {
        if (error) *error = r.error;
        return std::nullopt;
    }
    RE2::Options options;
    options.set_log_errors(false);
    // The dialect's bounds (10,000 atoms, counts multiplying to 1000) fit in 1 GiB; RE2
    // takes the memory as its program and caches grow, not up front.
    options.set_max_mem(std::int64_t(1) << 30);
    auto engine = std::make_unique<re2::RE2>(checked.spelling, options);
    if (!engine->ok())
        panic("RE2 refused \"" + checked.spelling + "\", the spelling of \"" + std::string(pattern) + "\", which the dialect accepts: " + engine->error());
    if (std::size_t(engine->NumberOfCapturingGroups()) != checked.groups) panic("RE2 counts the groups of a regex differently from the dialect");
    auto c = std::make_unique<detail::RegexCompiled>();
    c->pattern = std::string(pattern);
    c->flags = std::string(flags);
    c->engine = std::move(engine);
    c->groups = checked.groups;
    for (auto& [n, g] : checked.names) c->names.push_back({Text(n), g});
    c->plain_names = std::move(checked.names);
    return Regex(Rc<const detail::RegexCompiled>(std::move(c)));
}

Regex Regex::lit(std::string_view pattern, std::string_view flags) {
    RegexError e;
    auto r = compile(pattern, flags, &e);
    if (!r) panic("a regex literal outside the dialect (prismc checks every literal): " + e.to_string());
    return *r;
}

std::optional<Regex> Regex::parse(const Text& t) { return compile(t.view(), ""); }

std::string_view Regex::pattern() const { return c_->pattern; }
std::string_view Regex::flags() const { return c_->flags; }

namespace {

/// One search of `t` from `start`, its spans in `spans` (`npos` start: no part). An empty
/// match inside a character, which RE2 can report (`\B` in `é`) and the books refuse, is
/// skipped and the search made again from the next character.
bool search(const detail::RegexCompiled& c, const Text& t, std::size_t start, bool anchored,
            std::vector<std::pair<std::size_t, std::size_t>>& spans) {
    std::string_view hay = t.view();
    std::size_t n = c.groups + 1;
    std::vector<re2::StringPiece> sub(n);
    while (true) {
        if (!c.engine->Match(re2::StringPiece(hay.data(), hay.size()), start, hay.size(), anchored ? RE2::ANCHOR_START : RE2::UNANCHORED, sub.data(), int(n)))
            return false;
        std::size_t a = std::size_t(sub[0].data() - hay.data());
        if (sub[0].empty() && !on_boundary(hay, a)) {
            if (anchored) return false;
            start = a + 1;
            while (!on_boundary(hay, start)) ++start;
            continue;
        }
        spans.assign(n, {std::string_view::npos, 0});
        for (std::size_t g = 0; g < n; ++g)
            if (sub[g].data()) spans[g] = {std::size_t(sub[g].data() - hay.data()), std::size_t(sub[g].data() - hay.data()) + sub[g].size()};
        return true;
    }
}

Match build(const detail::RegexCompiled& c, const Text& t, const std::vector<std::pair<std::size_t, std::size_t>>& spans) {
    std::vector<std::optional<Group>> groups;
    groups.reserve(spans.size());
    for (auto [a, b] : spans) groups.push_back(a == std::string_view::npos ? std::nullopt : std::optional<Group>(group_of(t, a, b)));
    Map<Text, Group> named;
    for (const auto& [name, g] : c.names)
        if (groups[g]) named.insert(name, *groups[g]);
    Group g0 = *groups[0];
    return Match{g0.text, g0.front, g0.back, Array<std::optional<Group>>(std::move(groups)), std::move(named)};
}

/// Every match by the iteration rule, its spans given to `each`.
template <class F>
void each(const detail::RegexCompiled& c, const Text& t, F&& f) {
    std::string_view hay = t.view();
    std::vector<std::pair<std::size_t, std::size_t>> spans;
    std::size_t start = 0;
    std::optional<std::size_t> last_end;
    while (start <= hay.size() && search(c, t, start, false, spans)) {
        auto [a, b] = spans[0];
        if (a == b && last_end == a) {
            // An empty match where the last one ended is not reported: go on from the next
            // character.
            if (a >= hay.size()) break;
            start = a + char_at(hay, a).second;
            continue;
        }
        f(spans);
        last_end = b;
        start = b;
    }
}

}  // namespace

std::optional<Match> Regex::match_at(const Cursor& cur) const {
    std::vector<std::pair<std::size_t, std::size_t>> spans;
    if (!search(*c_, cur.text(), cur.at(), true, spans)) return std::nullopt;
    return build(*c_, cur.text(), spans);
}

std::optional<Match> Regex::find(const Text& t) const {
    std::vector<std::pair<std::size_t, std::size_t>> spans;
    if (!search(*c_, t, 0, false, spans)) return std::nullopt;
    return build(*c_, t, spans);
}

Array<Match> Regex::matches(const Text& t) const {
    std::vector<Match> out;
    each(*c_, t, [&](const auto& spans) { out.push_back(build(*c_, t, spans)); });
    return Array<Match>(std::move(out));
}

Array<Text> Regex::split(const Text& t) const {
    std::vector<Text> out;
    std::size_t from = 0;
    each(*c_, t, [&](const auto& spans) {
        out.push_back(view(t, from, spans[0].first));
        from = spans[0].second;
    });
    out.push_back(view(t, from, t.len()));
    return Array<Text>(std::move(out));
}

void Regex::show(Printer& p) const { p.raw(spelling(pattern(), nullptr, flags())); }

std::optional<RegexReplacement> RegexReplacement::compile(std::string_view pattern, std::string_view flags, std::string_view replacement,
                                                          RegexError* error) {
    // The diagnostics come flags, pattern, replacement: the regex is checked first.
    auto regex = Regex::compile(pattern, flags, error);
    if (!regex) return std::nullopt;
    std::vector<ReplacementPiece> pieces;
    try {
        pieces = check_replacement(replacement, regex->compiled().groups, regex->compiled().plain_names);
    } catch (const Refused& r) {
        if (error) *error = r.error;
        return std::nullopt;
    }
    return RegexReplacement(*regex, make_rc<const std::string>(replacement), make_rc<const std::vector<ReplacementPiece>>(std::move(pieces)));
}

RegexReplacement RegexReplacement::lit(std::string_view pattern, std::string_view flags, std::string_view replacement) {
    RegexError e;
    auto r = compile(pattern, flags, replacement, &e);
    if (!r) panic("a regex literal outside the dialect (prismc checks every literal): " + e.to_string());
    return *r;
}

Text RegexReplacement::replace(const Text& t) const {
    std::string_view hay = t.view();
    std::optional<std::string> out;
    std::size_t from = 0;
    each(regex_.compiled(), t, [&](const auto& spans) {
        if (!out) out.emplace().reserve(hay.size());
        out->append(hay.substr(from, spans[0].first - from));
        for (const ReplacementPiece& piece : *pieces_) {
            if (piece.group < 0) out->append(piece.text);
            else if (auto [a, b] = spans[std::size_t(piece.group)]; a != std::string_view::npos) out->append(hay.substr(a, b - a));
        }
        from = spans[0].second;
    });
    // No match: the text itself, shared.
    if (!out) return t;
    out->append(hay.substr(from));
    return Text::of_utf8(std::move(*out));
}

void RegexReplacement::show(Printer& p) const { p.raw(spelling(regex_.pattern(), &*replacement_, regex_.flags())); }

void Group::show(Printer& p) const { p.record("Group", {{"text", ShowRef(text)}, {"front", ShowRef(front)}, {"back", ShowRef(back)}}); }

void Match::show(Printer& p) const {
    p.record("Match", {{"text", ShowRef(text)}, {"front", ShowRef(front)}, {"back", ShowRef(back)}, {"groups", ShowRef(groups)}, {"named", ShowRef(named)}});
}

}  // namespace prism_rtl
