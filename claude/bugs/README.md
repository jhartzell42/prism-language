# Bugs, investigated

*Written by Claude on the claude-self-hosting branch; subject to change.* One file per
bug in prismc that has been reproduced and traced to its cause, with the books' rule it
breaks, the design question under the fix if there is one, and the fix with its tests.
A bug leaves this directory when its fix lands.
- **`window-relayout-every-change.md`**: what a small change to the window still costs
  with the host document as `$(Path ==> $Elem c)` and only the damage painted (C++ select
  24.5 M instructions, Rust 33 M): C++'s build walks in a structural change three times
  Rust's, a structural change's pass over every place's entry, the 1,000 labels a select
  recomputes, and Rust's 11 M to present each frame.
- **`bench-cuis-latin1-and-software-paint.md`**: on Linux under Xvfb the Cuis table's
  times measure software painting, not Morphic.
- **`widget-plain-result-unbound.md`**: a widget block ending on a plain value, bound with
  `x <- @W ..`, leaves `x` unbound in the caller; the crate fails at rustc.
