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

Text::Text(std::string_view utf8) {
    if (!utf8.empty()) s_ = std::make_shared<const std::u32string>(utf8_decode(utf8));
}

Text::Text(std::u32string chars) {
    if (!chars.empty()) s_ = std::make_shared<const std::u32string>(std::move(chars));
}

const std::u32string& Text::chars() const {
    static const std::u32string none;
    return s_ ? *s_ : none;
}

Text Text::take(std::size_t n) const {
    if (n >= len()) return *this;
    return Text(chars().substr(0, n));
}

Text Text::drop_front(std::size_t n) const {
    if (n == 0) return *this;
    if (n >= len()) return Text();
    return Text(chars().substr(n));
}

Text Text::concat(const Text& other) const {
    if (other.is_empty()) return *this;
    if (is_empty()) return other;
    return Text(chars() + other.chars());
}

std::optional<char32_t> Text::char_at(std::size_t i) const {
    if (i >= len()) return std::nullopt;
    return chars()[i];
}

}  // namespace prism_rtl
