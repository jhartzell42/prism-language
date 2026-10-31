# The canvas World

*Written by Claude, 31 October 2026, on the `claude-self-hosting` branch. Subject to
change: implementers use their discretion.*

The canvas World is the one foreign World chess runs under (`docs/decisions.md` D9). It
opens a window for each `Gui.Window` place, paints each `Gui.Scene` into its window in
software, measures text for `Gui.Metrics`, and sends the window's raw input back to the
program. A drawing World is four tags with no hit testing (Pitch ch. 6, "Speaking a
subset"). Buttons, layout, focus and deciding what a click hit are all done in Prism, by
the window library (`examples/chess/window.prism`). The World follows prism-reflex's Gloss
World (`src/Prism/Gloss.hs`).

```
cargo run --release -p prism_zero -- run --world canvas \
    examples/chess/rules.prism examples/chess/board.prism examples/chess/window.prism
make -f worlds/canvas/Makefile          # the same, chess in a window (the default target)
make -f worlds/canvas/Makefile check    # build, test and lint this crate, on any platform
```

## What it answers

- **`Gui.Window(title, size)`**
  - Its first value is `WindowId title`. This is the Test World's policy
    (`examples/README.md`, doubt WI), so a program sees the same id in a test and in a window.
  - It opens one operating-system window per place, `size` logical pixels inside.
  - A new `title` retitles the window and keeps it. When the place leaves, the window closes.
  - It answers the `!WindowInput` stream by the place's path:
    - `Press Point(x, y)` for the left mouse button, in the program's coordinates (logical
      pixels from the top left, y down).
    - `Pressed Key` for the keys `Key` names: `Char c`, `Left`, `Right`, `Backspace`, `Tab`,
      `Enter`. Space comes as `Char ' '`, and Delete is taken as `Backspace`, as in the
      Gloss World.
    - `Resized Extent(w, h)` when the window's size changes.
- **`Gui.Scene(window, draw)`**
  - The scene is painted into every window whose id is `window`, in document order, over
    a `Paper` background.
  - Every window is repainted on each patch that moves a window or a scene.
  - What each draw command does:
    - `Draw.Rect(rect, paint)` fills the rectangle.
    - `Draw.Text(at, text)` draws one line in `Ink`, with the top of the line at `at`.
    - `Draw.Image(rect, src)` draws a PNG, read from `src` relative to the working
      directory and scaled to the rectangle.
    - `Draw.Clip(rect, draws)` draws `draws` cut to the rectangle, nested clips intersecting.
    - `Draw.Group(turn, draws)` turns `draws` clockwise by `turn.angle` degrees about
      `(turn.x, turn.y)`, then scales them about that point by `turn.scale`. This is the
      same transform the Test World applies to find a clickable, so a turned card is drawn
      where a test clicks it.
  - The paints are the Gloss World's palette, with `Paper` and `Ink` in place of its
    `White` and `Black`. `Rgb r, g, b` takes channels from 0 to 1. An unknown paint draws
    light grey.
- **`Gui.Metrics(text)`**
  - Its first value is the text's `Extent` in the bundled font, which is a ready state: a
    pure computation over what the World holds.
  - The width is the sum of the characters' advances at 12 px, rounded up. The height is
    the font's line at that size, 14. The Test World's font is 9 a character and 14 high
    (doubt TM), so rows lay out the same height under both Worlds and only widths differ.
- **`Gui.Focus(window, at)`**: accepted and ignored. The window library draws its own focus.

## What it ignores

- **`Draw.Clickable(rect, path)`**. It exists for the Test World's `click element` (D9).
  Hit testing is the program's.
- **Any draw command it does not know.** It draws nothing for it.
- **Any element of another family.** The World reports the element once on standard error
  (for example, a bare `Ui.Label` with no `@Window` around it) and leaves it unanswered.
  `Prelude.*` belongs to prism_zero's `UnwrapWorld`, which wraps this World, and is passed
  over in silence.

## How it runs

- The loop is `prism_rt::world::drive` turned inside out, because winit calls the program
  and not the other way round.
  - When winit's event loop starts, it builds the program (tick 0) and opens the windows
    the program asked for.
  - Each window input is queued, and each tick takes one input. Two presses never share a
    tick, matching the Gloss World's "one turn per input". A run of resizes of one window
    collapses into the last.
  - The World also ticks with no input while the program is `pending`.
  - It collects garbage after every tick.
  - It ticks for at most 16 ms per wake-up, so a long `@Run` does not freeze the windows.
    It polls while ticks are owed and sleeps otherwise.
- After the ticks, it makes the operating system's windows match the document's
  `Gui.Window` places and repaints them if anything moved.
- The run ends in one of these ways:
  - With the code of a failed tick or a poisoned root (1, by the CLI World's policy).
  - With 0 when the user closes a window.
  - With 0 when the document holds no window and nothing can cause another tick. A
    program that never opened a window is told so.
- The World has no patience bound, since an interactive World waits for its user.
- `prism_zero run --world canvas` passes the root's row check with `Gui`, `Prelude` and
  `Ui`. `Ui` is there because compiler zero's row check does not subtract what a host
  document answers, and the chess board's `emit` line names `Ui.*` for the widgets its
  `@Window` hosts (see `CANVAS_FAMILIES` in `prism_zero/src/driver.rs`).
- The canvas World takes no program arguments. It answers no `Process.Args`.

## The font

DejaVu Sans 2.37 (`fonts/DejaVuSans.ttf`) is compiled into the binary. Its license is
`fonts/DejaVuSans-LICENSE.txt`, the upstream DejaVu `LICENSE`: the Bitstream Vera license
for the fonts, public domain for DejaVu's changes, and the Arev license for the glyphs
taken from Arev. All of these allow redistribution with the notice kept. Text is
rasterized at the window's pixel size, so it stays sharp on a Retina screen.

## Dependencies

Each one is chosen to stay small:

- `winit` 0.30 for windows and input. On Linux only the `x11` backend is compiled.
  On macOS no backend feature is needed.
- `softbuffer` 0.4 to put pixels on screen. It needs no GPU and no shader.
- `tiny-skia` 0.11 for painting, with its `png-format` feature, which decodes
  `Draw.Image` files through the `png` crate.
- `fontdue` 0.9 for the font.

## What was checked, and where

- **On the Linux container**
  - The crate builds, and `cargo clippy -p prism_world_canvas --all-targets` is clean.
  - Its unit tests run without a display:
    - the ready state's answers;
    - the patch read into windows and scenes;
    - the input queue;
    - every draw command painted offscreen, at scale 1 and 2;
    - the font's line height.
- **Under X11 on a virtual display (`Xvfb`)**
  - Chess opens, draws its board, pieces and status line, and plays e2–e4 from two
    synthetic clicks: e2 turns yellow, then the pawn moves and the status reads "Black's
    turn".
  - Closing the window exits 0.
  - A bare `Ui` program is reported and exits 0.
  - With no display, the World says so and exits 1.
  - A click repaints in about 0.07 s in a release build and 0.5 s in a debug build (both
    figures include the polling of the measurement), which is why `make chess` builds
    release.
- **Not checked: macOS itself.** The container has no Mac, so these remain unchecked:
  - the Cocoa backend;
  - Retina scaling against a real scale factor (scale 2 is tested offscreen only);
  - the window manager's close button;
  - real key events (no key was synthesized on X11 either).

## Known limits

- **Text.** One font at one size, one line. There is no kerning, shaping, bidirectional
  text or line breaking. A newline is a character like any other.
- **Window ids.** Two windows with the same title get the same id, by the Test World's
  policy, and each draws both windows' scenes.
- **Input.** Only the left mouse button and the keys `Key` names are reported. There is no
  pointer motion, release, scroll or modifier.
- **Paints.** The palette is fixed. Images are PNG only, cached for the run, and never
  reread when the file changes.
- **Repainting.** Every window is repainted in full on every change. There is no damage
  tracking, which is fine for chess.
- **The `make chess` target.** There is no root Makefile on this branch yet. A root
  `make chess` would be `$(MAKE) -f worlds/canvas/Makefile chess`.
