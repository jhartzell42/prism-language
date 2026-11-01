# The canvas World over `prism_rtl`

*Written by Claude, 1 November 2026, on the `claude-self-hosting` branch. Subject to
change: implementers use their discretion.*

This is `worlds/canvas` speaking compiled programs' protocol (`docs/backend.md`): a
`prism_rtl::Program` and `prism_rtl::World`, everything crossing as a `Pod`.
`worlds/canvas` stays as it is and serves the interpreter (prism_zero) until the
interpreter goes. What the World answers, what it ignores, how it runs, the font, the
dependencies and the known limits are as `worlds/canvas/README.md` says. This file says
only what differs.

```rust
prism_world_canvas_rtl::run::<MyProgram>()          // on the main thread
```

## What differs from `worlds/canvas`

- **The modules.**
  - `font.rs` is `worlds/canvas/src/font.rs` itself, through `#[path]`, with the bundled
    font beside it. It has no `prism_rt` in it.
  - `render.rs` and `app.rs` are copies with `prism_rt::Value` turned into `Pod` and the
    loop into `prism_rtl`'s. Painting itself is unchanged.
  - `world.rs` is the protocol half, rewritten. It carries its own `gui` helpers
    (`window_id`, `extent`, `press` and the rest), since `prism_rtl` has no families
    module.
- **The prelude.** `run` puts the World inside `prism_world_cli::UnwrapWorld`, the CLI
  World's `Prelude.Unwrap` policy as a wrapper, where `worlds/canvas` was wrapped by
  prism_zero. A poisoned root prints its exception and ends the run with 1. `run_with`
  takes another wrapper.
- **The loop.** It is `prism_rtl::drive` turned inside out, as before. The build is the
  first tick, `compact` runs after every tick, and a failed tick or failed build ends
  the run with `World::failed`'s code, 1.
- **Late input.** Input queued for a window whose place has left is dropped, and the
  wrapper drops any answer to a place no longer in the document.

## What was checked, and where

- **On the Linux container**
  - `cargo clippy -p prism_world_canvas_rtl --all-targets` is clean.
  - The unit tests run without a display, as in `worlds/canvas`: the ready state, the
    patch read into windows and scenes, the input queue, every draw command painted
    offscreen at scale 1 and 2, and the font.
- **Under Xvfb** (`tests/xvfb_smoke.rs`, part of `cargo test`)
  - A program written by hand in `docs/backend.md`'s shape opens one window with a red
    rectangle.
  - A second X client reads the screen until the rectangle's centre is red, then clicks
    it through XTest.
  - The program hears `Press Point(x=60.0, y=35.0)` and tears both places down, and the
    run ends with 0.
  - With no `DISPLAY` the test starts itself again under `xvfb-run -a`. With no
    `xvfb-run` either, as on a Mac, it says so and passes.
- **Not checked: chess.** Chess needs `prismc` to emit it, so no compiled chess exists
  yet. The interpreter's chess under `worlds/canvas` is what was checked by hand.
- **Not checked: macOS itself**, as in `worlds/canvas`.
