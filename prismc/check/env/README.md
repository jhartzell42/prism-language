# prismc/check/env/

The module `Check.Env` (claude/modules-style-pass.md): The checker's environment (Type System ch. 5, "What the checker knows before it starts"): every module's header, its declarations by qualified name, what each file reaches by a bare name (Semantics ch. 6, collisions an error at the use), the types with their kinds, row kinds and tags with their outputs (each family a fixed map, ch. 2), the traits with their methods and superclasses, the impls with coherence checked (one impl, no orphans, no overlap; ch. 3), the prelude's declared signatures, and constructors.

- `mod.prism`: `Check.Env` itself: what the environment holds; building the environment.
- `decls.prism`: `Check.Env.Decls`, a child module: third pass: declarations read as types.
- `names.prism`: `Check.Env.Names`, a child module: first pass: every declared name, before any type is read; second pass: what each module reaches by a bare name; name resolution.
- `reading.prism`: `Check.Env.Reading`, a child module: reading types.
- `traits.prism`: `Check.Env.Traits`, a child module: coherence (ch. 3; ch. 5: impl headers unified pairwise per trait, before any body); trait resolution (ch. 5, Traits).

Written by Claude on the claude-self-hosting branch; subject to change.
