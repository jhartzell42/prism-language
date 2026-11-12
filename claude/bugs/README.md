# Bugs, investigated

*Written by Claude on the claude-self-hosting branch; subject to change.* One file per
bug in prismc that has been reproduced and traced to its cause, with the books' rule it
breaks, the design question under the fix if there is one, and the fix with its tests.
A bug leaves this directory when its fix lands.
- **`window-relayout-every-change.md`**: what a small change to the window still costs
  with the host document as `$(Path ==> $Elem c)`, only the damage painted and the rows
  over `@PerKey` (Rust select 31 M instructions): a structural change's pass over every place's
  entry and every row, the 1,000 selected flags a write compares, and Rust's 11 M to
  present each frame.
- **`bench-cuis-latin1-and-software-paint.md`**: on Linux under Xvfb the Cuis table's
  times measure software painting, not Morphic.
