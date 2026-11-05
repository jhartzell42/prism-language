// `Upper` and `Lower` (`tests/text/case.txt`, its format and the statement it pins in
// `tests/text/README.md`) against the C++ runtime, plus what a line of the file cannot say:
// an unchanged text is answered itself, and the tables' Unicode version. The twin of
// runtime/rust/tests/case.rs.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include <fstream>
#include <stdexcept>
#include <string>
#include <vector>

#include "check.hpp"
#include "prism_rtl/text_case.hpp"

using namespace prism_rtl;

namespace {

/// A field of the file, decoded: `\u{hex}` a code point, `\\` a backslash.
std::string decode(const std::string& field) {
    std::string out;
    for (std::size_t i = 0; i < field.size();) {
        if (field[i] != '\\') {
            out.push_back(field[i++]);
            continue;
        }
        if (field.compare(i, 2, "\\\\") == 0) {
            out.push_back('\\');
            i += 2;
        } else if (field.compare(i, 3, "\\u{") == 0) {
            std::size_t close = field.find('}', i);
            if (close == std::string::npos) throw std::runtime_error("an unclosed \\u{ in case.txt");
            unsigned long c = std::stoul(field.substr(i + 3, close - i - 3), nullptr, 16);
            if (c > 0x10FFFF || (c >= 0xD800 && c <= 0xDFFF)) throw std::runtime_error("a bad \\u{hex} in case.txt");
            utf8_push(out, char32_t(c));
            i = close + 1;
        } else {
            throw std::runtime_error("an unknown escape in case.txt: " + field.substr(i, 2));
        }
    }
    return out;
}

std::vector<std::string> fields(const std::string& line) {
    std::vector<std::string> f;
    std::size_t start = 0;
    for (std::size_t tab; (tab = line.find('\t', start)) != std::string::npos; start = tab + 1)
        f.push_back(line.substr(start, tab - start));
    f.push_back(line.substr(start));
    return f;
}

}  // namespace

TEST(case_file) {
    std::ifstream f(std::string(PRISM_TESTS_DIR) + "/text/case.txt");
    CHECK(f.good());
    std::string line;
    int n = 0, cases = 0, failed = 0;
    while (std::getline(f, line)) {
        ++n;
        if (!line.empty() && line[0] == '#') continue;
        if (line.find('\t') == std::string::npos && line.find_first_not_of(" ") == std::string::npos) continue;
        auto fs = fields(line);
        if (fs.size() != 3) {
            check::fail(__FILE__, __LINE__, "case.txt line " + std::to_string(n) + ": three fields");
            continue;
        }
        Text input(decode(fs[0]));
        ++cases;
        std::string up = std::string(text_upper(input).view()), low = std::string(text_lower(input).view());
        if (up != decode(fs[1])) {
            ++failed;
            check::fail(__FILE__, __LINE__, "case.txt line " + std::to_string(n) + ": Upper \"" + fs[0] + "\" is \"" + up + "\"");
        }
        if (low != decode(fs[2])) {
            ++failed;
            check::fail(__FILE__, __LINE__, "case.txt line " + std::to_string(n) + ": Lower \"" + fs[0] + "\" is \"" + low + "\"");
        }
    }
    CHECK(cases > 0);
    if (failed) std::cout << "  " << failed << " of " << 2 * cases << " mappings fail\n";
}

TEST(case_unchanged_is_shared) {
    Text up("ALREADY UPPER, ΣΑΣ, ẞ"), low("already lower, σας, ß");
    CHECK(text_upper(up).view().data() == up.view().data());
    CHECK(text_lower(low).view().data() == low.view().data());
    Text mixed("Mixed");
    CHECK(text_upper(mixed).view() == "MIXED");
    CHECK(text_upper(mixed).view().data() != mixed.view().data());
}

TEST(case_tables_version) { CHECK_EQ(std::string(case_unicode_version()), std::string("17.0.0")); }
