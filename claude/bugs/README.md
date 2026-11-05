# Bugs, investigated

*Written by Claude on the claude-self-hosting branch; subject to change.* One file per
bug in prismc that has been reproduced and traced to its cause, with the books' rule it
breaks, the design question under the fix if there is one, and the fix with its tests.
A bug leaves this directory when its fix lands.
- **`absolute-out-runtime-path.md`**: an absolute `-o` gives the Rust crate runtime paths
  that climb to `/`, silently.
- **`window-relayout-every-change.md`**: the window library lays out, draws and hit-tests
  the whole tree on every change; select costs as much as create1k. Bare-minimum fix, in
  Prism (Jimmy).
- **`bench-cuis-latin1-and-software-paint.md`**: the Cuis table crashes on `▶` in a
  Latin-1 `String`; on Linux its times measure software painting.
