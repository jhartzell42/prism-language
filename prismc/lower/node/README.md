# prismc/lower/node/

The module `Lower.Node` (claude/modules-style-pass.md): `Lower.Node`: widget blocks lowered to nodes (Compiler ch. 4; claude/backend.md): one node per block, its places, dynamics, holds, `for` and `rebuild`, and its build.

- `mod.prism`: `Lower.Node` itself: widget blocks: one node per block, children in static positions merged in.
- `def.prism`: `Lower.Node.Def`, a child module: A node from a widget definition.
- `dynamics.prism`: `Lower.Node.Dynamics`, a child module: Dynamics and events inside a block.
- `elements.prism`: `Lower.Node.Elements`, a child module: elements.
- `finish.prism`: `Lower.Node.Finish`, a child module: The node, finished: heights, methods, the `Node` hooks, the build.
- `keyed.prism`: `Lower.Node.Keyed`, a child module: `for`.
- `static.prism`: `Lower.Node.Static`, a child module: widgets applied in a static position: transparent.
- `switch.prism`: `Lower.Node.Switch`, a child module: `rebuild` and `@Switch` (claude/backend.md, "Dynamic structure"; Compiler ch. 3).

Written by Claude on the claude-self-hosting branch; subject to change.
