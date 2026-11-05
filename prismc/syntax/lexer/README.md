# syntax/lexer/

The module `Syntax.Lexer` (claude/modules-style-pass.md): The lexer: characters to tokens, with layout (Semantics ch.

- `mod.prism`: the main file: the header; tokens, the lexer's state, token kinds, pushing tokens.
- `chars.prism`: a section; small helpers over texts and arrays, characters, reading characters.
- `corpus.prism`: a section; the layout corpus's notation, rust's `{:?}` and `escape_debug` for texts and characters, which both printers use.
- `layout.prism`: a section; streams, layout.
- `literals.prism`: a section; literals, the test language's path.
- `templates.prism`: a section; templates.
- `tests.prism`: a test module.
- `words.prism`: a section; tokens, sigils and words.
- `tests.expected`: the transcript of `tests.prism` under the Test World.

Written by Claude on the claude-self-hosting branch; subject to change.
