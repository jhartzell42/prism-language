# The window library lays out, draws and hit-tests the whole tree on every change

*Written by Claude on the claude-self-hosting branch, 5 November 2026; subject to change.
A performance bug in `examples/chess/window.prism`: the programs are right, and any
change costs as much as building the whole window.*

## The scenario

The GUI table (`bench/gui/`) on the cloud box (2 cores, Xvfb, 5 November), medians in ms:

| op | Prism C++ | Prism Rust | GTK |
|---|---|---|---|
| create1k | 1,055 | 717 | 482 |
| update10th | 1,218 | 2,567 | 53 |
| select | 1,217 | 2,591 | 6 |
| swap | 1,174 | 2,142 | 121 |
| remove | 1,073 | 2,149 | 31 |

- **Selecting one row costs as much as creating a thousand.** GTK selects in 6 ms;
  Prism takes 1.2 s in C++ and 2.6 s in Rust.
- **Every edit that touches one row or a hundred costs about one create1k.** The work is
  proportional to the window's size, not to the change.

## The mechanism

`@Window` (window.prism, about line 901) computes everything from the whole element tree
as plain functions of one behavior:

- **The layout,** `$placed = PlaceAt (LayRoot (EnvOf $sizes, $?focus), $kids, $size), ...`
  (line 910): one call over every element. A changed label in row 2 re-lays out all
  1,000 rows: `LayStack` measures every child (`natural`), sizes it again (`Final`), and
  with `Stretch` lays it out a third time (`Restretch`).
- **The draw list:** `StackAt` folds `Merged` over the rows, appending each row's
  `[Draw]` and `[Hit]` to the accumulator, so the list is rebuilt whole.
- **The scene:** `Gui.Scene(window=id, draw=$placed.draw)` hands the World the whole
  list, and the World repaints all of it, rows scrolled off the window included.
- **The measured words:** `TextsIn $kids` (line 906) rebuilds the set of every word in
  the tree. In C++ that set is also quadratic (`cpp-union-big-into-small.md`).
- **Hit testing and focus:** `?HitAt` and `?FocusRect` fold over every hit, which is
  fine per click, but the hits are rebuilt with the layout.

Profiles agree (gdb stack samples, 5 November):

- **C++ select:** 21 of 30 samples in `texts_in` / `union_` under `rows_build`, and the
  rest in `placed` and `scene`.
- **Rust select:** 26 of 30 in `Main::placed`, 25 of those in `lay_stack`'s fold, mostly
  `im_rc` `Vector::append` and `merge_rebalance` on the `Hit` and `Draw` lists (memcpy
  at the top).

Prism's dynamics already know which row changed; the window library throws that away by
laying out the tree as one function of one value.

## Is it a bug?

Yes, in the library, not the language. A window whose cost per change is the whole tree
cannot run a list of ten thousand rows, and the benchmark exists to show what Prism's
incremental model buys. Cuis's Morphic, redrawing damaged rectangles only, does
update10th in 1 ms on the same box.

## The fix: bare-minimum, bone-simple refactoring, staying in Prism (Jimmy)

Jimmy: the fix is "bare minimum bone simple" refactoring, and the layout engine *stays*
in Prism. No new semantics, with one possible exception below.

- **Per-child dynamics instead of one function of the tree.** Lay out each child where
  the tree already has a place for it (a `for` over the children by path), so a
  child's `Laid`, its `[Draw]` and its words refire only when that child changes. The
  stack then combines children's sizes, which is cheap, instead of re-laying them out.
- **One layout per child per change.** `natural`, `Final` and `Restretch` each call
  `Lay`; reuse the first result when the room did not change.
- **Draw only what the window shows.** Skip children whose rectangle falls outside the
  window before building their draw commands.
- **The word set per child,** unioned at the parent, so a changed label re-measures only
  its own words.
- **Allowed new piece:** a combinator that removes spurious refires from a dynamic
  holding a value of an `^Eq` type (refire only when the new value differs). Jimmy: "we
  might want" it; nothing else new.
- **Keep it small.** Do not build a general retained-mode layout system, a diffing World
  protocol, or damage regions in the World; those are designs, not this fix.

## Tests and measurement

- **The existing window tests and chess's transcripts** pin behavior; they must not
  change.
- **The bench is the measure:** `bench/gui/run.sh prism-cpp prism-rust`, before and
  after. Success is select, swap, remove and update10th well under create1k (tens of
  milliseconds, the order of GTK), with create1k no slower.
