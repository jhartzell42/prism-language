# examples/chess/window/

*Written by Claude on the claude-self-hosting branch; subject to change.*

The module `Window` (claude/modules-style-pass.md): The window library: the Ui family's elements, the host documents that lay them out, and the window that draws them, over the four tags a drawing World accepts (`Gui.Window`, `Gui.Scene`, `Gui.Metrics`, `Gui.Focus`; Pitch chapters 5 and 6).

- `mod.prism`: `Window` itself: geometry and paint; the `Draw` family a scene draws; the drawing World's four `Gui` tags.
- `hits.prism`: `Window.Hits`, a child module: hit testing, focus and the routing of input to the element under it.
- `hosting.prism`: `Window.Hosting`, a child module: a box answers each of its child's Ui elements with the inputs routed to it.
- `layout.prism`: `Window.Layout`, a child module: layout as plain functions over plain data (prism-reflex's layout.md): room down, size up, placed by the parent; an element's `Res` (its layout as a `Plan`, data with `^Eq`), and the one placing pass, `PlaceAt`.
- `ui.prism`: `Window.Ui`, a child module: the `Ui` family, leaves and containers.
- `widgets.prism`: `Window.Widgets`, a child module: the widget functions, each keeping a hold or hosting its child; `@Laid`, one element laid out in a `for` row of its own, one row per child, its leaves measuring their own texts; and `@Window`.
- `tests.prism`: `Window.Tests`, its test module: a column taller than the window draws only what shows; focus by press and Tab; a key typed; a slider and a text box kept in step; a dialog hidden, shown and keeping its text, its background taking presses; a grid's columns each as wide as their widest child (`tests.expected`).

## What it offers

- **Leaves:** `Ui.Label`, `Ui.Button`, `Ui.Image` (a picture on a live paint, clickable),
  `Ui.TextInput` (focused by a press or Tab, typed into), and `Ui.Slider` (a track from
  `low` to `high` in whole steps: a press on it answers the step nearest the press).
- **Containers:** `@Column`, `@Row`, `@VBox`, `@HBox`; `@Grid` (each column as wide as its
  widest child, each row as tall as its tallest: a board, or a form of captions and
  boxes); `@Layers` (children on top of one another); `@Framed` and its modifiers
  (`@Pad`, `@Fill`, `@Sized`, `@Aspect`, `@Anchor`, `@Turned`, `@Backed`), and
  `@FramedBy`, a frame whose spec is live, with `@Bordered`, a band of paint whose width
  follows a dynamic; `@ShowIf`, a column drawn only while a flag holds, whose children
  keep their state while hidden.
- **Widgets that keep a hold:** `@TextInput name, initial`; `@TextField name, initial,
  !set`, a text input that each firing of `!set` replaces (a box kept in step with a
  slider); `@Overlay $open, at, @child`, a dialog inside `@Layers`: a panel on paper with
  a dark edge at `at` (fractions of the room), drawn only while `$open`.
- **Presses:** the topmost hit under the pointer takes a press. A frame with a background
  is opaque: its background takes the presses its children do not, and answers nothing,
  so a click on a dialog's margin does not reach the board beneath it. A key goes to the
  focused text input only while that input shows, so typing after a dialog closes edits
  nothing hidden.

## Doubts

- **DG** (Claude's) The slider answers presses and nothing else: the canvas Worlds send
  `Press`, `Pressed` and `Resized`, with no pointer motion or release, so a knob cannot be
  dragged. A press anywhere on the track jumps to that step. Recommendation: add
  `Moved Point` and `Released Point` to `WindowInput` in both canvas Worlds when a widget
  needs dragging, and let the slider follow the pointer between a press on it and the
  release; until then press-to-jump plus a text box covers the setting.
- **OP** (Claude's) A frame's background takes presses (`Opaque` hits), where before a
  press passed through it to whatever lay beneath. Chess and the GUI benchmark have no
  background under a clickable that should hear it; a card table might want a backed
  frame that lets presses through, which would be a field of `FrameSpec`.
- **HS** (Claude's) A hidden `Ui.Shown` still lays its children out and measures their
  texts in their rows (they are places, with holds), and draws and hits nothing; it takes
  no room. The alternative, a `for` over a map holding the dialog or nothing, would tear
  the dialog's places down on close and lose what was typed in them.
- **FS** (Claude's) `Ui.Frame`'s spec is live (`$FrameSpec`): a border follows a setting
  without a `rebuild`, which would make the board's squares new places and their paths
  new.

