# examples/chess/window/

The module `Window` (claude/modules-style-pass.md): The window library: the Ui family's elements, the host documents that lay them out, and the window that draws them, over the four tags a drawing World accepts (`Gui.Window`, `Gui.Scene`, `Gui.Metrics`, `Gui.Focus`; Pitch chapters 5 and 6).

- `mod.prism`: `Window` itself: geometry and paint; the `Draw` family a scene draws; the drawing World's four `Gui` tags.
- `hits.prism`: `Window.Hits`, a child module: hit testing, focus and the routing of input to the element under it.
- `hosting.prism`: `Window.Hosting`, a child module: a box answers each of its child's Ui elements with the inputs routed to it.
- `layout.prism`: `Window.Layout`, a child module: layout as plain functions over plain data (prism-reflex's layout.md): room down, size up, placed by the parent; an element's `Res` (its layout as a `Plan`, data with `^Eq`), and the one placing pass, `PlaceAt`.
- `ui.prism`: `Window.Ui`, a child module: the `Ui` family, leaves and containers.
- `widgets.prism`: `Window.Widgets`, a child module: the widget functions, each keeping a hold or hosting its child; `@Laid`, one element laid out in a `for` row of its own, one row per child, its leaves measuring their own texts; and `@Window`.
- `tests.prism`: `Window.Tests`, its test module: a column taller than the window draws only what shows; focus by press and Tab; a key typed (`tests.expected`).

Written by Claude on the claude-self-hosting branch; subject to change.
