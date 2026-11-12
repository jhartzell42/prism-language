// Reading the transcript's spelling back: values, paths, and the patterns the test
// language matches with (Conformance ch. 1, "Checks"; Appendix A).
//
// Reading is printing's inverse, without types: `Name(f=v)` is a record, `Fam.Tag(f=v)`
// an element, `Name` or `Name a, b` a choice, a bare qualified name a fault kind, a
// number followed by `/` a path, a number with a point a `Float`, an `Error(..)` record a
// failed `~T`. A pattern is a value with three wildcards in it: `ignore` for anything,
// `..` for the remaining fields, and `Fam.*` for any element of a family.

#pragma once

#include <string>
#include <string_view>

#include "prism_rtl/pod.hpp"

namespace prism_rtl {

/// A pattern of the test language: a value with wildcards in it.
struct Pattern {
    Pod pod;
    /// Matches anything.
    static Pattern any();
    /// Whether `v` matches.
    bool matches(const Pod& v) const;
    bool operator==(const Pattern&) const = default;
};

namespace read {

/// Read a value.
Expected<Pod, std::string> value(std::string_view src);
/// Read a pattern.
Expected<Pattern, std::string> pattern(std::string_view src);
/// Read a path (`1/#0/[2]/0`).
Expected<Path, std::string> path(std::string_view src);

}  // namespace read

/// Rust's `char` classes the reader and the Test World use. ASCII exactly; above it, the
/// Unicode spaces are whitespace, every other character is alphabetic, and none is
/// uppercase.
bool char_is_whitespace(char32_t c);
bool char_is_alphabetic(char32_t c);
bool char_is_alphanumeric(char32_t c);
bool char_is_uppercase(char32_t c);

}  // namespace prism_rtl
