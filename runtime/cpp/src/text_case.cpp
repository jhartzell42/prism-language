// `Upper` and `Lower` (text_case.hpp): one pass over the text's UTF-8, each character
// looked up in two-stage tables generated from the Unicode Character Database
// (case_tables.inc, written by tools/gen_case_tables.py; tools/README.md).
//
// A character's record holds its uppercase and lowercase mappings, each a delta from the
// code point or, for a mapping of two or three code points, an index into `seqs`, and its
// context class for Final_Sigma. The bytes before the first character that changes are
// copied once, when it is met; a text in which none changes is answered itself.
//
// Undefined behaviour, reasoned out: a `Text` is well-formed UTF-8 (collections.hpp), so a
// lead byte's length never runs past the end (checked anyway: a broken text is a `Panic`,
// never a read past the buffer); bytes are read as `unsigned char`; a code point is at
// most U+10FFFF, below `table_end`'s check the stage-1 index is in bounds (the
// static_asserts below), and every index the tables hold is checked at compile time to be
// in its array; a delta added to a code point is an `int32_t` sum of two values below
// 2^21, and the generator refuses a mapping that is not a scalar value.
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#include "prism_rtl/text_case.hpp"

#include <cstdint>
#include <string>
#include <string_view>

#include "prism_rtl/result.hpp"

namespace prism_rtl {

namespace {

namespace tables {

struct Record {
    std::int32_t upper_delta;
    std::int32_t lower_delta;
    std::uint16_t upper_seq;  // 0: none; else an index into `seqs`
    std::uint16_t lower_seq;
    std::uint8_t context;  // 0 neither, 1 Case_Ignorable, 2 Cased and not Case_Ignorable
};

/// A mapping of two or three code points, 0 after the last.
struct Seq {
    char32_t c[3];
};

#include "case_tables.inc"

constexpr std::size_t n_blocks = sizeof(stage2) / sizeof(stage2[0]);
constexpr std::size_t n_records = sizeof(records) / sizeof(records[0]);
constexpr std::size_t n_seqs = sizeof(seqs) / sizeof(seqs[0]);

constexpr bool indices_in_bounds() {
    for (auto b : stage1)
        if (b >= n_blocks) return false;
    for (const auto& blk : stage2)
        for (auto r : blk)
            if (r >= n_records) return false;
    for (const auto& r : records)
        if (r.upper_seq >= n_seqs || r.lower_seq >= n_seqs || r.context > 2) return false;
    return true;
}

static_assert(sizeof(stage1) / sizeof(stage1[0]) * block_size == table_end);
static_assert(table_end <= 0x110000);
static_assert(indices_in_bounds());
static_assert(records[0].upper_delta == 0 && records[0].lower_delta == 0 && records[0].upper_seq == 0 &&
              records[0].lower_seq == 0 && records[0].context == 0);

}  // namespace tables

const tables::Record& record_of(char32_t c) {
    if (c >= tables::table_end) return tables::records[0];
    return tables::records[tables::stage2[tables::stage1[c / tables::block_size]][c % tables::block_size]];
}

/// The character starting at byte `i` of well-formed UTF-8, and its length in bytes.
char32_t decode_at(const char* p, std::size_t n, std::size_t i, std::size_t& len) {
    unsigned char b = static_cast<unsigned char>(p[i]);
    if (b < 0x80) {
        len = 1;
        return b;
    }
    len = b < 0xE0 ? 2 : b < 0xF0 ? 3 : 4;
    if (n - i < len) panic("Upper or Lower over a text that is not well-formed UTF-8");
    char32_t c = b & (len == 2 ? 0x1F : len == 3 ? 0x0F : 0x07);
    for (std::size_t k = 1; k < len; ++k) c = (c << 6) | (static_cast<unsigned char>(p[i + k]) & 0x3F);
    return c;
}

constexpr char32_t capital_sigma = 0x3A3, small_sigma = 0x3C3, final_sigma = 0x3C2;

/// Whether the first character from byte `i` on that is not Case_Ignorable is Cased.
bool cased_follows(const char* p, std::size_t n, std::size_t i) {
    while (i < n) {
        std::size_t len;
        auto ctx = record_of(decode_at(p, n, i, len)).context;
        if (ctx != 1) return ctx == 2;
        i += len;
    }
    return false;
}

template <bool Upper>
Text convert(const Text& t) {
    const char* p = t.view().data();
    std::size_t n = t.len();
    std::string out;
    bool started = false;
    // Lower only: whether the last character before `i` that is not Case_Ignorable is Cased.
    bool cased_before = false;
    for (std::size_t i = 0; i < n;) {
        std::size_t len;
        char32_t c = decode_at(p, n, i, len);
        const tables::Record& r = record_of(c);
        std::int32_t delta = Upper ? r.upper_delta : r.lower_delta;
        std::uint16_t seq = Upper ? r.upper_seq : r.lower_seq;
        if (delta != 0 || seq != 0) {
            if (!started) {
                out.reserve(n + n / 4 + 8);
                out.append(p, i);
                started = true;
            }
            if (!Upper && c == capital_sigma) {
                // Final_Sigma (SpecialCasing.txt; the Unicode Standard, 3.13): the cursor
                // after it is skipped forward over Case_Ignorable characters only, so the
                // work is linear in all: each character is passed at most twice.
                utf8_push(out, cased_before && !cased_follows(p, n, i + len) ? final_sigma : small_sigma);
            } else if (seq != 0) {
                for (char32_t m : tables::seqs[seq].c)
                    if (m != 0) utf8_push(out, m);
            } else {
                utf8_push(out, char32_t(std::int32_t(c) + delta));
            }
        } else if (started) {
            out.append(p + i, len);
        }
        if (!Upper && r.context != 1) cased_before = r.context == 2;
        i += len;
    }
    return started ? Text::of_utf8(std::move(out)) : t;
}

}  // namespace

Text text_upper(const Text& t) { return convert<true>(t); }

Text text_lower(const Text& t) { return convert<false>(t); }

const char* case_unicode_version() { return tables::unicode_version; }

}  // namespace prism_rtl
