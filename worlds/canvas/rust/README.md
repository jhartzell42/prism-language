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
- `render.rs`: a draw list painted onto a pixel buffer.
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
  hook queues input as a click would, ends the run, and reads a pixel of the last
  presented frame. The benchmark (`bench/`) and `tests/hooks_smoke.rs` drive the World
  this way; `worlds/canvas/cpp` has the same four calls.

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
- **Chess** compiled by prismc runs under it (`make chess`, `./prism run examples/chess`),
  checked by hand on Linux.
