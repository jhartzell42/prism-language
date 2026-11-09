# A block's build reads a name bound on a line below

*Written by Claude on the claude-widget-plain branch, 8 November 2026; subject to change.*

- **What happens:** a widget block whose build reads a name bound on a line below emits
  code that does not compile, at rustc (or clang++), not at prismc. Three forms, each
  reproduced (the program below holds the first and the third; the second was a probe):
  - an element above a hold, its first value asked with the hold's initial value:
    `((door, !knocks)) <- Pad.Door(name=$called)` above `$called <- "front" then ..`
    asks with `self.called.get(cx)` in `build`, which has no `self`;
  - a plain `let` above the plain `let` it reads (`let greeting = "hello {word}"` above
    `let word = "x"`): the build's `let greeting` reads a local `word` not declared yet;
  - a plain `let` above a widget's plain result it reads (`word <- @Word "x"`, `@Word`
    ending on `@ w`): the same.
- **The book's rule it breaks:** Semantics ch. 4, "Binding groups and cycles": a widget
  block is one binding group, any line may use any name the block binds, and the order of
  the lines decides positions and nothing else. A cycle among plain values, or a hold whose
  initial value names itself, is an error, so the values the build reads always have an
  order.
- **Cause, as far as traced:** the build's `let`s are emitted in page order. `WidgetStmt`
  threads the build environment through the lines in order, and a hold's initial value is
  bound under `{name}#init` (the local `{f}_init`) only when `HoldLine` lowers its line;
  an element's constant fields at the build are spelled through `InitEnv` of the
  environment at the element's line (prismc/lower/node/elements.prism), so a hold below has
  no `#init` yet and its `BnHold` binding is spelled for the methods. A plain `let`
  (`WidgetLetField`) and a widget's plain result (`BindPlainResult`,
  prismc/lower/node/def.prism) push their `let` when their line is lowered, so a reader
  above reads the local before it is declared.
- **The fix, roughly:** order the build's `let`s by what they read, not by page order (the
  book's cycle rule makes the order exist), or lower holds' initial values and plain
  values in the prebinding pass, before any line. Either way prismc should refuse what it
  cannot order with a diagnostic, never emit a crate that fails to compile.
- **Reproduce:** claude/bugs/hold-below-element-read-at-build/ (app.prism, tests.prism,
  tests.expected derived from the book): `./prism test claude/bugs/hold-below-element-read-at-build`
  fails at rustc; with the hold and `word`'s line moved to the top of `@page` (neither
  takes a position) the same transcript passes on Rust and C++. It moves into
  tests/conformance/ unchanged when the bug is fixed.
- **Found** while writing sem-ch08-widget-plain-result, whose `$called` is bound above
  `@Door` for this reason.
