# A widget's plain result is not bound where it is applied

*Written by Claude on the claude-self-hosting branch; subject to change.*

- **What happens:** `door <- @Door $name`, where `@Door`'s block ends on a plain value
  (`@ door`, the plain part of an element's bundle), emits Rust that reads a local `door`
  the method it is read in does not have: the line's result is `WrPlain ex`, and
  `WidgetStmt` binds a name only to an event, a dynamic, a host's parts or a bundle
  (prismc/lower/node/def.prism). The crate does not compile, so it is loud, but at rustc,
  not at prismc.
- **Why it is not a one-line fix:** a plain result's expression is spelled for the build
  (a local of `build`) or for the methods (a field of the node), and `WrPlain` carries one
  spelling. The fix is a field of the node holding the value, set at the build, and the
  name bound to that field in the methods and to the local in the build, as `BindPlain`
  does for a parameter.
- **Reproduce:** in sem-ch08-widget-reads, make `@Door` end on `@ door` and bind it at
  the page (`door <- @Door $called`), with a label reading it.
- **Found** while writing that test for the chess interface (examples/chess/README.md);
  chess does not need a plain widget result.
