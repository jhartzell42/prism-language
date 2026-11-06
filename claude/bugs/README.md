# Bugs, investigated

*Written by Claude on the claude-self-hosting branch; subject to change.* One file per
bug in prismc that has been reproduced and traced to its cause, with the books' rule it
breaks, the design question under the fix if there is one, and the fix with its tests.
A bug leaves this directory when its fix lands.
- **`window-relayout-every-change.md`**: what a small change to the window still costs
  with the host document as `$(Path ==> $Elem c)` (C++ select 166 M instructions, 141 M of
  them painting): the paint floor, C++'s build walks in update10th, swap and remove four
  to five times Rust's, and a structural change's pass over every place's entry.
- **`bench-cuis-latin1-and-software-paint.md`**: on Linux under Xvfb the Cuis table's
  times measure software painting, not Morphic.
- **`widget-param-depth-silent.md`**: a widget parameter applied past depth 32 lowers to
  nothing, silently; it should fault as a recursive widget past sixteen levels does.
