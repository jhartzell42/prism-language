# prismc/check/regex/

The module `Check.Regex` (claude/modules-style-pass.md): a literal's pattern, flags and replacement read against the dialect (claude/regex-dialect.md), the first diagnostic or none, as both runtimes read them.

- `mod.prism`: `Check.Regex` itself.
- `tests.prism`: `Check.Regex.Tests`, its test module: tests/regex/corpus.txt as checks, written by `gen-tests.py` (a test module runs under the Test World, which reads no file); `tests.expected` its transcript.
- `gen-tests.py`: writes `tests.prism` and `tests.expected` from the corpus (`python3 -I prismc/check/regex/gen-tests.py`).

Written by Claude on the claude-self-hosting branch; subject to change.
