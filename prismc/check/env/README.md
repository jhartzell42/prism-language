# check/env/

The module `Check.Env` (docs/modules-style-pass.md): The checker's environment (Type System ch.

- `mod.prism`: the main file: the header; what the environment holds, building the environment.
- `decls.prism`: a section; third pass: declarations read as types.
- `names.prism`: a section; first pass: every declared name, before any type is read, second pass: what each module reaches by a bare name, name resolution.
- `reading.prism`: a section; reading types.
- `traits.prism`: a section; coherence, trait resolution.

Written by Claude on the claude-self-hosting branch; subject to change.
