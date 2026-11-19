# tests/regex: the regular-expression corpus

Written by Claude on the claude-self-hosting branch; subject to change.

`corpus.txt` states what the runtime answers for regular expressions (Semantics
Appendix B, "Regular expressions"; the dialect as `claude/regex-dialect.md` fixes it),
byte offsets and diagnostics included. One test reads it:

- Rust: `runtime/rust/tests/regex.rs` (`cargo test --release -p prism_rtl --test regex`).

prismc's checker is held to its refusals and acceptances too: `Check.Regex.Tests`
(`prismc/check/regex/tests.prism`) is written from this file by `gen-tests.py` beside it
(`tests/compiled/run.sh regex`), and tests/checker/regex-neg.py writes one checker program
per message from its first refused literal. Run both after changing this file.

## The format

One case a line, six fields separated by tabs; `#` begins a comment line, and blank lines
are skipped:

```
kind	pattern	flags	arg	input	expected
```

In every field `⟨⟩` is the empty text, `⟨-⟩` an absent one, and `⟨hex⟩` the code point
`hex` (`⟨A⟩` a newline, `⟨20⟩` a space where one would be hard to see); everything else
is itself, UTF-8.

| kind | arg | what runs | expected |
| --- | --- | --- | --- |
| `find` | `⟨-⟩` | `?Find` on the input | `nil`, or a match |
| `at` | a byte offset | `?Match` at the cursor there | `nil`, or a match |
| `all` | `⟨-⟩` | `Matches` | `none`, or each match in `[...]`, space-separated |
| `split` | `⟨-⟩` | `Split` | each piece's span, space-separated |
| `replace` | the replacement | `Replace` | the resulting text |
| `refuse` | the replacement, or `⟨-⟩` for a `Regex` | the constructor | `part@at message` |

A match is its spans: `a-b` (bytes `a` to `b`) for group 0, then one per numbered group,
`a-b` or `nil`, then `name=a-b` for each named group that took part, by name. The tests
also check that every match's text, every group's and every `Split` piece is a view of the
input's bytes at those offsets (no copy), and that the cursors stand there.
