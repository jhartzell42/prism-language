# The canvas World in C++

*Written by Claude on the `claude-self-hosting` branch; subject to change.*

The C++ form of `worlds/canvas` (the Rust crate `prism_world_canvas_rtl`): the World a
program compiled to C++ runs under when its root emits `Gui.*`, over `runtime/cpp`'s
protocol. It answers what the Rust one answers, the same way: `Gui.Window`, `Gui.Scene`,
`Gui.Metrics` and `Gui.Focus` (worlds/canvas/README.md and the headers of
worlds/canvas/src hold word for word). It presents settled frames only: it paints after a
round of ticks that leaves `pending` unset, never in between.

```cpp
prism_world_canvas::run<MyProgram>()                    // inside the prelude's Prelude.Unwrap
prism_world_canvas::run_with<MyProgram>(wrap, hooks)    // another wrapper; hooks for a driver
```

- **`include/prism_world_canvas/`**: `world.hpp` (the World with no display: `Shared`,
  `CanvasReady`, `CanvasWorld`, and `UnwrapWorld`, the prelude's policy around it),
  `font.hpp`, `render.hpp`, `canvas.hpp` (the loop, `Session`, `Hooks`, `run`).
- **`src/`**: `world.cpp`, `font.cpp`, `render.cpp`, `app.cpp` (the Xlib loop, Linux),
  `app_macos.mm` (the Cocoa loop, macOS), and `program_main.cpp`, the `main` for a
  compiled program (below).
- **`tests/`**: `test_canvas.cpp` (the Rust crate's unit tests, test for test, plus the
  Rust World's widths for five texts), `smoke_program.hpp` (a hand-written program with
  one window), `xvfb_smoke.cpp` (Linux: the Rust `xvfb_smoke`, that window clicked
  through XTest), `macos_smoke.cpp` (macOS: that window's pixel read back and pressed
  through `Hooks`), `chess_smoke.sh` with `robot.cpp` and `robot.hpp` (chess under Xvfb,
  Linux).

```
make -C worlds/canvas-cpp test          # g++, clang++, then ASan+UBSan with g++ (clang on a Mac)
make -C worlds/canvas-cpp check         # once, with $(CXX)
make -C worlds/canvas-cpp chess-smoke   # chess compiled to C++ under Xvfb: e2, e4, exit 0
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
- **The font file is the Rust World's own**, `worlds/canvas/fonts/DejaVuSans.ttf`, built
  into the object with the assembler's `.incbin`.

## What differs from `worlds/canvas`

- **The scale factor is 1 on X11**, which here reads no DPI setting (macOS: below).
- **Keys on X11** are `XLookupString`'s: Latin-1 characters, and the named keys the Rust World
  names.
- **Hooks.** `Hooks::settled` is called each time the loop would sleep with every window
  showing the document; `Hooks::presented` after each frame is synced to the server. A
  hook queues input through `Session` as a click would. The benchmark (bench/) drives its
  operations this way; nothing calls them otherwise.
- **`UnwrapWorld` lives here**, over worlds/cli-cpp's `Prelude`, since worlds/cli-cpp has
  the policy inline in its World only.

## A compiled program

prismc's C++ program has its own `main.cpp`, whose canvas branch says "the C++ target has
no canvas World" (prismc/templates/cpp.prism, `CppRunRoot`). Until the template runs this
World, `make program PROG=DIR ROOT=MainProgram` builds `DIR/canvas`: the program's
sources, its `main.cpp` compiled with `main` renamed, and `src/program_main.cpp`, which
runs `crate::ROOT` under this World. `DIR/canvas test` and `--world cli` go on to the
emitted `main`.

## On macOS

- **`src/app_macos.mm` is the window side**, the Xlib loop's rounds over AppKit: the loop
  takes events with `nextEventMatchingMask:` on the main thread and sleeps there with
  `distantFuture` when nothing is owed. Everything else (the font, the renderer, the
  World) is the same file on both.
- **Painting.** Cairo paints into a buffer of the window's size in physical pixels; it
  becomes a `CGImage` set as the view's layer contents, and `[CATransaction flush]` hands
  the frame to the window server. `Hooks::settled` runs after that flush.
- **The scale factor is the window's `backingScaleFactor`** (2 on a Retina screen), so
  text and shapes are drawn at the screen's resolution in logical pixels, as the Rust
  World under winit does.
- **Keys** are the event's characters, as Unicode, and the named keys by key code.
- **Tests.** `macos_smoke` needs a logged-in session (a window server) but no
  accessibility permission: it reads the frame back through `Session::pixel` and presses
  through `Session::input`. The Xvfb tests and the XTest robot are Linux-only, and
  LeakSanitizer does not run on arm64 macOS, so `check` runs ASan there without leak
  checking.
