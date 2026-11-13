# runtime/cpp/tools

Written by Claude on the claude-self-hosting branch; subject to change.

- **`gen_case_tables.py`** writes `../src/case_tables.inc`, the tables of `Upper` and
  `Lower` (`../src/text_case.cpp`; the statement is `tests/text/README.md`'s). It reads
  three files of the Unicode Character Database, all of one version (it stops if they
  differ, and the table records the version):
  - `UnicodeData.txt`, the simple mappings (fields 12 and 13);
  - `SpecialCasing.txt`, the unconditional full mappings, which replace them; its
    conditional lines are skipped (Final_Sigma is coded in `text_case.cpp`; the `tr`,
    `az` and `lt` tailorings are not applied);
  - `DerivedCoreProperties.txt`, `Cased` and `Case_Ignorable`, for Final_Sigma.

  The version is **17.0.0**, the one Rust's `std` carries in the toolchain here (Rust 1.98,
  `char::UNICODE_VERSION`), so the two targets agree on every code point.
  `runtime/rust/tests/case.rs` (`unicode_version`) and `runtime/cpp/tests/test_case.cpp`
  (`case_tables_version`) both check 17.0.0: a toolchain with newer tables fails the first,
  and the tables are then regenerated from that version and both checks moved.

To regenerate, in an empty directory of its own (the files are data from outside):

```
V=17.0.0
for f in UnicodeData.txt SpecialCasing.txt DerivedCoreProperties.txt; do
  curl -sSfLO https://www.unicode.org/Public/$V/ucd/$f
done
python3 -I runtime/cpp/tools/gen_case_tables.py DIR > runtime/cpp/src/case_tables.inc
```

The table is two stages: `stage1` (a byte per block of 128 code points, up to U+E01FF;
past it every code point maps to itself and is neither cased nor case-ignorable), the
deduplicated blocks of `stage2` (a record index per code point), the records (each
mapping a delta or a sequence index, and the context class) and the sequences of two or
three code points. 17.0.0 gives 284 records, 169 blocks and 73 sequences: 55,868 bytes.
