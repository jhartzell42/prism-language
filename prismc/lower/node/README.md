# lower/node/

The module `Lower.Node` (docs/modules-style-pass.md): `Lower.Node`: widget blocks lowered to nodes (Compiler ch.

- `mod.prism`: the main file: the header; widget blocks: one node per block, children in static positions merged in.
- `def.prism`: a section; a node from a widget definition.
- `dynamics.prism`: a section; dynamics and events inside a block.
- `elements.prism`: a section; elements.
- `finish.prism`: a section; the node, finished: heights, methods, the `Node` hooks, the build.
- `keyed.prism`: a section; `for`.
- `static.prism`: a section; widgets applied in a static position: transparent.
- `switch.prism`: a section; `rebuild` and `@Switch`.

Written by Claude on the claude-self-hosting branch; subject to change.
