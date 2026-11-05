// `Upper` and `Lower` (text_case.hpp).
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include "prism_rtl/text_case.hpp"

namespace prism_rtl {

Text text_upper(const Text& t) {
    std::string s = t.to_string();
    for (auto& c : s)
        if (c >= 'a' && c <= 'z') c = char(c - 'a' + 'A');
    return Text::of_utf8(std::move(s));
}

Text text_lower(const Text& t) {
    std::string s = t.to_string();
    for (auto& c : s)
        if (c >= 'A' && c <= 'Z') c = char(c - 'A' + 'a');
    return Text::of_utf8(std::move(s));
}

const char* case_unicode_version() { return "none"; }

}  // namespace prism_rtl
