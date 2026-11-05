# tests/text: what both runtimes answer for texts

Written by Claude on the claude-self-hosting branch; subject to change.

`case.txt` states what `Upper` and `Lower` (Semantics Appendix B, Text) answer, character
for character. Two tests read it, so the two runtimes are held to one source and never
compared with each other:

- Rust: `runtime/rust/tests/case.rs` (`cargo test --release -p prism_rtl --test case`),
  which runs the Rust prelude's bodies (`str::to_uppercase`, `str::to_lowercase`);
- C++: `runtime/cpp/tests/test_case.cpp` (`make -C runtime/cpp test`, test `case_file`),
  which runs `prism_rtl::text_upper` and `text_lower`, what the C++ prelude calls.

The program `tests/conformance/sem-appB-upper-lower/` checks a selection through both
compiled targets.

## The statement

`Upper` and `Lower` are Unicode's default case conversion (the Unicode Standard, section
3.13, toUppercase and toLowercase), with the Unicode Character Database 17.0.0:

- every character becomes its **full** mapping: `SpecialCasing.txt`'s unconditional
  mapping where it has one (`ß` → `SS`, `ﬁ` → `FI`, `İ` → `i̇`, `ΐ` → `Ϊ́` as three code
  points), else `UnicodeData.txt`'s simple mapping, else itself;
- the one conditional mapping that depends on no language is applied: `Lower` makes `Σ`
  (U+03A3) `ς` (U+03C2) where it is **Final_Sigma**, that is, before it (skipping
  `Case_Ignorable` characters) is a `Cased` character and after it (skipping them again)
  is none, and `σ` otherwise; `Upper` makes both `σ` and `ς` `Σ`;
- **no locale**: the conditions `tr`, `az` and `lt` of `SpecialCasing.txt` are never
  applied (Turkish `i` → `İ` and `ı` are not made; `I` is `i`), and neither is
  `After_I`, `More_Above` or `Not_Before_Dot`;
- no normalization: a combining sequence is mapped character by character
  (`e` U+0301 → `E` U+0301), and the result is not composed;
- linear in the text's length: one pass, the result built once.

This is what Rust's `str::to_uppercase` and `str::to_lowercase` do (whose tables are
Unicode 17.0.0's in Rust 1.98, `char::UNICODE_VERSION`). A text already in the target
case comes back equal; whether it is the same buffer is not promised (C++ answers the
text itself, Rust a copy), and no program can tell.

## The format

One case a line, three fields separated by tabs; `#` begins a comment line, and a line
without a tab that is empty or spaces is skipped:

```
input	upper	lower
```

A field is UTF-8 as written, except that `\u{hex}` is the code point `hex` and `\\` is a
backslash. An empty field is the empty text.
