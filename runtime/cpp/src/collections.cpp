// UTF-8 and `Text` (collections.hpp).

#include "prism_rtl/collections.hpp"

namespace prism_rtl {

std::u32string utf8_decode(std::string_view s) {
    std::u32string out;
    out.reserve(s.size());
    for (std::size_t i = 0; i < s.size();) {
        unsigned char c = static_cast<unsigned char>(s[i]);
        int extra = c < 0x80 ? 0 : (c >> 5) == 0x6 ? 1 : (c >> 4) == 0xE ? 2 : (c >> 3) == 0x1E ? 3 : -1;
        if (extra < 0 || i + std::size_t(extra) >= s.size()) {
            out.push_back(U'�');
            ++i;
            continue;
        }
        char32_t cp = extra == 0 ? c : extra == 1 ? c & 0x1F : extra == 2 ? c & 0x0F : c & 0x07;
        bool ok = true;
        for (int k = 1; k <= extra; ++k) {
            unsigned char d = static_cast<unsigned char>(s[i + std::size_t(k)]);
            if ((d >> 6) != 0x2) {
                ok = false;
                break;
            }
            cp = (cp << 6) | (d & 0x3F);
        }
        if (!ok) {
            out.push_back(U'�');
            ++i;
            continue;
        }
        out.push_back(cp);
        i += std::size_t(extra) + 1;
    }
    return out;
}

void utf8_push(std::string& out, char32_t c) {
    if (c < 0x80) {
        out.push_back(char(c));
    } else if (c < 0x800) {
        out.push_back(char(0xC0 | (c >> 6)));
        out.push_back(char(0x80 | (c & 0x3F)));
    } else if (c < 0x10000) {
        out.push_back(char(0xE0 | (c >> 12)));
        out.push_back(char(0x80 | ((c >> 6) & 0x3F)));
        out.push_back(char(0x80 | (c & 0x3F)));
    } else {
        out.push_back(char(0xF0 | (c >> 18)));
        out.push_back(char(0x80 | ((c >> 12) & 0x3F)));
        out.push_back(char(0x80 | ((c >> 6) & 0x3F)));
        out.push_back(char(0x80 | (c & 0x3F)));
    }
}

std::string utf8_encode(std::u32string_view s) {
    std::string out;
    out.reserve(s.size());
    for (char32_t c : s) utf8_push(out, c);
    return out;
}

std::string utf8_valid(std::string_view s) {
    std::size_t i = 0;
    while (i < s.size() && static_cast<unsigned char>(s[i]) < 0x80) ++i;
    if (i == s.size()) return std::string(s);
    // Past the ASCII prefix: decode and encode, which keeps well-formed UTF-8 as it is and
    // reads a malformed byte as U+FFFD.
    std::string out(s.substr(0, i));
    out += utf8_encode(utf8_decode(s.substr(i)));
    return out;
}

Text::Text(std::string_view utf8) {
    if (utf8.empty()) return;
    s_ = std::make_shared<const std::string>(utf8_valid(utf8));
    end_ = s_->size();
}

Text::Text(std::string&& utf8) {
    if (utf8.empty()) return;
    std::string valid = utf8_valid(utf8);
    s_ = std::make_shared<const std::string>(valid == utf8 ? std::move(utf8) : std::move(valid));
    end_ = s_->size();
}

Text Text::concat(const Text& other) const {
    if (other.is_empty()) return *this;
    if (is_empty()) return other;
    std::string s;
    s.reserve(len() + other.len());
    s += view();
    s += other.view();
    Text t;
    t.end_ = s.size();
    t.s_ = std::make_shared<const std::string>(std::move(s));
    return t;
}

namespace {

/// Whether byte `n` of `s` starts a character (or is its end).
bool boundary(std::string_view s, std::size_t n) {
    return n == s.size() || (n < s.size() && (static_cast<unsigned char>(s[n]) & 0xC0) != 0x80);
}

/// The length of the UTF-8 sequence a lead byte starts.
std::size_t seq_len(unsigned char c) { return c < 0x80 ? 1 : c < 0xE0 ? 2 : c < 0xF0 ? 3 : 4; }

/// The character of well-formed UTF-8 starting at byte `i`, `n` bytes long.
char32_t decode_at(std::string_view s, std::size_t i, std::size_t n) {
    unsigned char c = static_cast<unsigned char>(s[i]);
    char32_t cp = n == 1 ? c : n == 2 ? c & 0x1F : n == 3 ? c & 0x0F : c & 0x07;
    for (std::size_t k = 1; k < n; ++k) cp = (cp << 6) | (static_cast<unsigned char>(s[i + k]) & 0x3F);
    return cp;
}

}  // namespace

std::optional<Cursor> Cursor::at_byte(const Text& t, std::size_t n) {
    if (!boundary(t.view(), n)) return std::nullopt;
    return Cursor(t, n);
}

std::optional<std::pair<char32_t, Cursor>> Cursor::step() const {
    std::string_view s = text_.view();
    if (at_ >= s.size()) return std::nullopt;
    std::size_t n = seq_len(static_cast<unsigned char>(s[at_]));
    return std::pair<char32_t, Cursor>(decode_at(s, at_, n), Cursor(text_, at_ + n));
}

std::optional<std::pair<char32_t, Cursor>> Cursor::step_back() const {
    std::string_view s = text_.view();
    if (at_ == 0) return std::nullopt;
    std::size_t i = at_ - 1;
    while (i > 0 && (static_cast<unsigned char>(s[i]) & 0xC0) == 0x80) --i;
    return std::pair<char32_t, Cursor>(decode_at(s, i, at_ - i), Cursor(text_, i));
}

}  // namespace prism_rtl
