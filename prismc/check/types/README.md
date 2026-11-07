# prismc/check/types/

The module `Check.Types` (claude/modules-style-pass.md): The checker's types (Type System ch. 5, "The three layers of a type"): the plain layer (constructors, records, tuples, maps, arrays, functions, variables, associated types), the functor stack (sigils over a plain layer, outermost first; `@` names a row), and the color of a function type.

- `mod.prism`: `Check.Types` itself: Colors, rows, types; Small helpers over lists of texts and numbers; Fresh variables and the substitution; Stacks; Printing.
- `colors.prism`: `Check.Types.Colors`, a child module: Colors.
- `recording.prism`: `Check.Types.Recording`, a child module: The typed output (typed.prism), recorded only when it is asked for.
- `schemes.prism`: `Check.Types.Schemes`, a child module: Schemes: instantiation and generalization.
- `unify.prism`: `Check.Types.Unify`, a child module: Unification.

Written by Claude on the claude-self-hosting branch; subject to change.
