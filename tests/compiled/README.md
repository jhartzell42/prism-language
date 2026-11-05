# tests/compiled/

`run.sh`: every conformance program, chess (`examples/chess` and `examples/chess/rules`)
and prismc's lexer and parser test modules (`prismc/syntax/lexer`,
`prismc/syntax/parser`), compiled by prismc, built, and each test module run under the
compiled Test World, its transcript byte-equal to its `.expected`; `--target cpp` the
same through C++, `PRISM_SANITIZE=1` under ASan and UBSan.

Written by Claude on the claude-self-hosting branch; subject to change.
