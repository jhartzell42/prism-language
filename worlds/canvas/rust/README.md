# The canvas World over `prism_rtl`

*Written by Claude, 1 November 2026, on the `claude-self-hosting` branch. Subject to
change: implementers use their discretion.*

The canvas World in Rust: the World a program prismc compiled to Rust runs under when its
root emits `Gui.*`, speaking compiled programs' protocol (`claude/backend.md`), a
`prism_rtl::Program` and `prism_rtl::World` with everything crossing as a `Pod`. It
answers four tags, `Gui.Window`, `Gui.Scene`, `Gui.Metrics` and `Gui.Focus`, and paints
in software: winit for the windows and their events, softbuffer for the pixels on the
screen, tiny-skia for the raster, fontdue for the bundled DejaVu Sans (`fonts/`).
`worlds/canvas/cpp` is the same World in C++.

```rust
prism_world_canvas_rtl::run::<MyProgram>()          // on the main thread
prism_world_canvas_rtl::run_hooked::<MyProgram>(wrap, hooks)   // with Hooks, for a driver
```

## The modules

- `world.rs`: the protocol half, the World with no display in it, with its own `gui`
  helpers (`window_id`, `extent`, `press` and the rest), since `prism_rtl` has no
  families module.
- `render.rs`: a draw list painted onto a pixel buffer, and the pixels one draw command
  can touch.
- `damage.rs`: a window's frame painted again only where its scene changed (below).
- `font.rs`: the bundled font and its measures.
- `app.rs`: the loop over winit's event loop.

## How it runs

- **The prelude.** `run` puts the World inside `prism_world_cli::UnwrapWorld`, the CLI
  World's `Prelude.Unwrap` policy as a wrapper. A poisoned root prints its exception and
  ends the run with 1. `run_with` takes another wrapper.
- **The loop.** It is `prism_rtl::drive` turned inside out. The build is the first tick,
  `compact` runs after every tick, and a failed tick or failed build ends the run with
  `World::failed`'s code, 1.
- **Late input.** Input queued for a window whose place has left is dropped, and the
  wrapper drops any answer to a place no longer in the document.
- **Hooks.** `Hooks::settled` is called each time the loop would sleep with no input
  queued, no tick owed, and every window's last redraw presented. Through `Session` a
  hook queues input as a click would, ends the run, reads a pixel of the last presented
  frame, and reads that frame's size and scale. The benchmark (`bench/`),
  `tests/hooks_smoke.rs` and `tests/damage_smoke.rs` drive the World this way;
  `worlds/canvas/cpp` has the same five calls.

## Painting

- **Only the damage is painted again.** Each window keeps its frame and the top-level draw
  commands it last painted (every scene's draw list for the window, in document order),
  each with its bounds in physical pixels. A new settled scene's commands are compared
  with those: the ones both lists begin and end with stay, and the rest are compared
  position by position; a position whose command differs damages the old command's
  bounds and the new one's. The damaged boxes are merged (over 32 become the one box
  around them all); each is cleared in a scratch frame, every command meeting it painted
  there whole, exactly as a full repaint paints it, and the box copied into the frame.
  `damage.rs`'s header says why that equals a full repaint, and its tests check it pixel
  for pixel.
- **The whole frame is painted** on the first frame, after a resize or a change of scale,
  and when a damaged command cannot be bounded (a coordinate or a transform that is not
  finite). That is the defined behaviour, not a recovery.
- **What moves much of the frame damages much of it:** removing a row, or anything that
  moves every row below it, damages the rows from there down, about as much as a full
  repaint costs.
- **The whole frame is presented.** softbuffer's macOS buffer starts empty every time, and
  on X11 a redraw for an expose cannot be told from one for a new scene, so the frame
  (kept as 0x00RRGGBB and converted only where it was painted) is copied whole into the
  buffer and presented: about 11 M instructions a 900 by 800 frame at scale 2 on macOS.
- **The window's colour space is sRGB** on macOS (`srgb_window`), so Core Animation hands
  the frame to the window server as it is; with the screen's colour space it converted
  every pixel of every frame on the CPU, in this process.

## What was checked, and where

- **On the Linux container**
  - `cargo clippy -p prism_world_canvas_rtl --all-targets` is clean.
  - The unit tests run without a display: the ready state, the
    patch read into windows and scenes, the input queue, every draw command painted
    offscreen at scale 1 and 2, and the font.
- **Under Xvfb** (`tests/xvfb_smoke.rs`, part of `cargo test`)
  - A program written by hand in `claude/backend.md`'s shape opens one window with a red
    rectangle.
  - A second X client reads the screen until the rectangle's centre is red, then clicks
    it through XTest.
  - The program hears `Press Point(x=60.0, y=35.0)` and tears both places down, and the
    run ends with 0.
  - With no `DISPLAY` the test starts itself again under `xvfb-run -a`. With no
    `xvfb-run` either, as on a Mac, it says so and passes.
- **Through Hooks** (`tests/hooks_smoke.rs`, part of `cargo test`): the same window, its
  pixel read back from the presented frame and pressed through `Session`, with no robot.
  It runs on macOS (checked), and on Linux when there is a display.
- **Damage** (`damage.rs`'s tests): after every step of 400 random changes to a table
  (labels, swaps, removes, inserts, rectangles, clips, turned groups and images over it,
  a second scene), at scales 1, 2 and 1.5, the frame equals a full repaint of the last
  scene byte for byte; a select and a caret each damage under a fiftieth of the frame; a
  first frame, a new size and an unbounded command repaint the whole.
  `tests/damage_smoke.rs` steps a real window through ten scenes and checks every
  presented pixel against a full repaint (macOS, checked; Linux with a display).
- **Chess** compiled by prismc runs under it (`make basic-chess`, `./prism run examples/basic-chess`),
  checked by hand on Linux.
