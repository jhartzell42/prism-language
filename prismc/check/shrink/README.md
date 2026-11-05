# prismc/check/shrink/

The module `Check.Shrink` (claude/modules-style-pass.md): `Check.Shrink`: the shrink check (Semantics ch. 7, "Every tick ends"; prismc/README.md, Checker): every cycle of calls shrinks, or is `unbounded`.

- `mod.prism`: `Check.Shrink` itself: The shrink check (Semantics ch. 7, Every tick ends).
- `calls.prism`: `Check.Shrink.Calls`, a child module: pick CT, the call test for closures.
- `locals.prism`: `Check.Shrink.Locals`, a child module: pick LF.
- `relations.prism`: `Check.Shrink.Relations`, a child module: relations are scoped.
- `summaries.prism`: `Check.Shrink.Summaries`, a child module: summaries.
- `types.prism`: `Check.Shrink.Types`, a child module: pick NT.
- `walk.prism`: `Check.Shrink.Walk`, a child module: the walk.

Written by Claude on the claude-self-hosting branch; subject to change.
