# tests/prelude

*Written by Claude on the claude-self-hosting branch; subject to change.*

Checks of the prelude itself, as programs: each directory is a tree (`app.prism` and its
test module `Tests`, with `tests.expected`), run on both targets by
`tests/compiled/run.sh`. This README stands for the directories under it.

- **`maps/`**: `Union` keeps the left map's value on a shared key, whichever map is the
  larger (Semantics Appendix B, Maps).
