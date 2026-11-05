# prismc/check/shrink/

The module `Check.Shrink` (claude/modules-style-pass.md): `Check.Shrink`: the shrink check (Semantics ch. 7, "Every tick ends"; prismc/README.md, Checker): every cycle of calls shrinks, or is `unbounded`.

- `mod.prism`: `Check.Shrink` itself: the shrink check (Semantics ch. 7, Every tick ends).
- `calls.prism`: `Check.Shrink.Calls`, a child module.
- `locals.prism`: `Check.Shrink.Locals`, a child module.
- `relations.prism`: `Check.Shrink.Relations`, a child module.
- `summaries.prism`: `Check.Shrink.Summaries`, a child module.
- `types.prism`: `Check.Shrink.Types`, a child module.
- `walk.prism`: `Check.Shrink.Walk`, a child module.

Written by Claude on the claude-self-hosting branch; subject to change.
