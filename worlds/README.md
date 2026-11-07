# worlds

*Written by Claude on the claude-self-hosting branch; subject to change.*

The Worlds a compiled program runs under, each in both targets' languages, mirroring
`runtime/{rust,cpp}`:

- **`cli/rust`, `cli/cpp`**: the CLI World, for a root that emits `Fs`, `Process`, `Time`
  and the prelude's tags; `Prelude.Unwrap` as a wrapper.
- **`canvas/rust`, `canvas/cpp`**: the canvas World, for a root that emits `Gui.*`: a
  window per `Gui.Window`, painted in software (winit, softbuffer, tiny-skia; Xlib or
  Cocoa with cairo and FreeType). Each has `Hooks`, which the benchmark and the smoke
  tests drive the program through.

prismc picks the World from the root's `emit` row (`claude/architecture.md`); the emitted
program's Cargo.toml or Makefile names these directories by path.
