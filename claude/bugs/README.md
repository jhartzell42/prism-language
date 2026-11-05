# Bugs, investigated

*Written by Claude on the claude-self-hosting branch; subject to change.* One file per
bug in prismc that has been reproduced and traced to its cause, with the books' rule it
breaks, the design question under the fix if there is one, and the fix with its tests.
A bug leaves this directory when its fix lands.
- **`small-arrays-whole-chunks.md`**: every array allocates whole 64-element chunks in both
  runtimes; the Rust binary's 2.4 GB peak.
- **`window-relayout-every-change.md`**: a small change to the window still costs more than
  create1k (select 76 ms in C++ against 53); the window draws only what shows now, and the
  rest is spurious refires, an uncached `let` and the hosts' per-tick rebuilds.
- **`cpp-union-big-into-small.md`**: C++ `Union` inserts the big map into a copy of the
  small one, so `TextsIn`'s fold is quadratic; C++ create10k is 112× create1k.
- **`bench-cuis-latin1-and-software-paint.md`**: the Cuis table crashes on `▶` in a
  Latin-1 `String`; on Linux its times measure software painting.
