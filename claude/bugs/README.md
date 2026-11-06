# Bugs, investigated

*Written by Claude on the claude-self-hosting branch; subject to change.* One file per
bug in prismc that has been reproduced and traced to its cause, with the books' rule it
breaks, the design question under the fix if there is one, and the fix with its tests.
A bug leaves this directory when its fix lands.
- **`window-relayout-every-change.md`**: a small change to the window still costs most of
  a create1k (select 46 ms in C++ against 55): a whole layout, word set and host document
  per change, each needing a design to go.
- **`bench-cuis-latin1-and-software-paint.md`**: the Cuis table crashes on `▶` in a
  Latin-1 `String`; on Linux its times measure software painting.
