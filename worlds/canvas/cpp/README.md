# The canvas World in C++

*Written by Claude on the `claude-self-hosting` branch; subject to change.*

The C++ form of `worlds/canvas/rust` (the Rust crate `prism_world_canvas_rtl`): the World a
program compiled to C++ runs under when its root emits `Gui.*`, over `runtime/cpp`'s
protocol. It answers what the Rust one answers, the same way: `Gui.Window`, `Gui.Scene`,
`Gui.Metrics` and `Gui.Focus` (worlds/canvas/rust/README.md and the headers of
worlds/canvas/rust/src hold word for word). It presents settled frames only: it paints after a
round of ticks that leaves `pending` unset, never in between.

```cpp
prism_world_canvas::run<MyProgram>()                    // inside the prelude's Prelude.Unwrap
prism_world_canvas::run_with<MyProgram>(wrap, hooks)    // another wrapper; hooks for a driver
```

- **`include/prism_world_canvas/`**: `world.hpp` (the World with no display: `Shared`,
  `CanvasReady`, `CanvasWorld`, and `UnwrapWorld`, the prelude's policy around it),
  `font.hpp`, `render.hpp`, `damage.hpp` (the frame painted again only where the scene
  changed), `canvas.hpp` (the loop, `Session`, `Hooks`, `run`), `big_stack.hpp`.
- **`src/`**: `world.cpp`, `font.cpp`, `render.cpp`, `damage.cpp`, `app.cpp` (the Xlib loop, Linux),
  `app_macos.mm` (the Cocoa loop, macOS), and `big_stack.cpp`, the thread ticks run on
  (below).
- **`tests/`**: `test_canvas.cpp` (the Rust crate's unit tests, test for test, plus the
  Rust World's widths for five texts), `smoke_program.hpp` (a hand-written program with
  one window), `xvfb_smoke.cpp` (Linux: the Rust `xvfb_smoke`, that window clicked
  through XTest), `macos_smoke.cpp` (macOS: that window's pixel read back and pressed
  through `Hooks`), `damage_smoke.cpp` (the Rust `damage_smoke`: a window stepped through
  ten scenes through `Hooks`, every pixel of each frame checked against a full repaint),
  `chess_smoke.sh` with `robot.cpp` and `robot.hpp` (chess under Xvfb, Linux).

```
make -C worlds/canvas/cpp test          # g++, clang++, then ASan+UBSan with g++ (clang on a Mac)
make -C worlds/canvas/cpp check         # once, with $(CXX)
make -C worlds/canvas/cpp chess-smoke   # chess compiled to C++ under Xvfb: e2, e4, exit 0
```

## The libraries, and why

On Linux from apt (`libx11-dev libxext-dev libcairo2-dev libfreetype-dev`, and
`libxtst-dev` for the tests' robot); on macOS from Homebrew (`cairo freetype
pkg-config`) and the system's Cocoa and QuartzCore.

- **Xlib with MIT-SHM** for the windows. It is what softbuffer does on X11 under winit:
  a window, its events, and an image the X server reads from shared memory, so a frame
  costs no copy through the socket. Cairo paints straight into the shared image. A
  display without MIT-SHM is told so on standard error and painted through `XPutImage`.
  SDL2 would add a layer that only hands back the same image.
- **cairo** for the raster: rectangles, clips and transforms with anti-aliasing, as
  tiny-skia gives the Rust World, and PNG loading for `Draw.Image`.
- **FreeType** for the font, unhinted. The measures are fontdue's to the number: each
  advance is the font's own, scaled in `float` by px over units per em; the line is the
  `hhea` ascent less descent. `Gui.Metrics` answers what the Rust World answers, so a
  layout comes out the same. Glyph bitmaps can differ from fontdue's by a pixel at an edge.
- **The font file is the Rust World's own**, `worlds/canvas/rust/fonts/DejaVuSans.ttf`, built
  into the object with the assembler's `.incbin`.

## What differs from `worlds/canvas/rust`

- **The scale factor is 1 on X11**, which here reads no DPI setting (macOS: below).
- **Keys on X11** are `XLookupString`'s: Latin-1 characters, and the named keys the Rust World
  names.
- **Hooks.** `Hooks::settled` is called each time the loop would sleep with every window
  showing the document; `Hooks::presented` after each frame is synced to the server. A
  hook queues input through `Session` as a click would. The benchmark (bench/) drives its
  operations this way; nothing calls them otherwise.
- **Painting only the damage** is the Rust World's (its README, "Painting"; damage.hpp),
  over cairo: a box is cleared through cairo in a scratch surface, its commands drawn there
  whole by `Renderer::draw_some`, and its rows copied into the frame. The bounds follow the
  context's matrix as `draw` sets it, with two more pixels round a scaled image for cairo's
  filter. **On X11 only the damage is presented**: each damaged box goes to the server
  through `XShmPutImage` (or `XPutImage`), and the whole image after a map, an expose or a
  resize. On macOS the whole frame goes (below).
- **`UnwrapWorld` lives here**, over worlds/cli/cpp's `Prelude`, since worlds/cli/cpp has
  the policy inline in its World only.

## A compiled program

A program whose root's row emits `Gui.*` is emitted by prismc with this World: its
`main.cpp` runs the root through `prism_world_canvas::run<P>()`, and its Makefile builds
this library (`make lib`, after runtime/cpp's) and links it with cairo, FreeType and the
window system (prismc/templates/cpp.prism, `CppMakefileCanvas`). `main` keeps the main
thread for the windows, which AppKit requires, and the loop runs every tick, and the
program's teardown, on a thread with a 4 GB stack (`BigStack`), the stack the emitted
`main` gives the CLI World; the loop waits on each, so the two threads never run at once.
`PROG test` and `--world cli` run as for any root.

## On macOS

- **`src/app_macos.mm` is the window side**, the Xlib loop's rounds over AppKit: the loop
  takes events with `nextEventMatchingMask:` on the main thread and sleeps there with
  `distantFuture` when nothing is owed. Everything else (the font, the renderer, the
  World) is the same file on both.
- **Painting.** Cairo paints into a buffer of the window's size in physical pixels, again
  only where the scene changed; it becomes a `CGImage` set as the view's layer contents,
  the whole frame (a layer's contents are one image), and `[CATransaction flush]` hands
  the frame to the window server. `Hooks::settled` runs after that flush. The window's
  colour space is sRGB, the image's: with the screen's, Core Animation converted every
  pixel of every frame on the CPU before the flush (about 125 M instructions a 900 by 800
  frame at scale 2).
- **The scale factor is the window's `backingScaleFactor`** (2 on a Retina screen), so
  text and shapes are drawn at the screen's resolution in logical pixels, as the Rust
  World under winit does.
- **Keys** are the event's characters, as Unicode, and the named keys by key code.
- **Tests.** `macos_smoke` and `damage_smoke` need a window server (on Linux, `xvfb-run -a` when no `DISPLAY` is set) but no
  accessibility permission: they read the frame back through `Session::pixel` and press
  through `Session::input`. The Xvfb tests and the XTest robot are Linux-only, and
  LeakSanitizer does not run on arm64 macOS, so `check` runs ASan there without leak
  checking.
