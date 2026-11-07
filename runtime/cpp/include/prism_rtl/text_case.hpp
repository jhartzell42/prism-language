// `Upper` and `Lower` (Semantics Appendix B, Text): the Unicode default full case mapping,
// locale-independent, with the final sigma rule (README.md, "Upper and Lower"). The
// tables are generated from the Unicode Character Database (src/case_tables.inc,
// tools/README.md).
//
// Written by Claude on the claude-self-hosting branch; subject to change.

#pragma once

#include "prism_rtl/collections.hpp"

namespace prism_rtl {

/// `Upper`: every character's full uppercase mapping (SpecialCasing.txt's unconditional
/// mappings over UnicodeData.txt's simple ones: `ß` is `SS`). Linear, one pass; a text
/// with nothing to change is answered itself, with no copy.
Text text_upper(const Text& t);
/// `Lower`: every character's full lowercase mapping (`İ` is `i̇`), and `Σ` is `ς` where
/// it ends a word (Final_Sigma: a cased letter before it, none after it, case-ignorable
/// characters skipped both ways), else `σ`. Linear, one pass; a text with nothing to
/// change is answered itself, with no copy.
Text text_lower(const Text& t);

/// The version of the Unicode Character Database the tables are from.
const char* case_unicode_version();

}  // namespace prism_rtl
