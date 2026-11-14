# `$Merge` and `$GuestsOf` over generic combinators

Jimmy, 14 November: "design on top of generic non document dynamic combinators"; path
manipulators may stay primitive. Written by Claude on the claude-self-hosting branch;
subject to change. Applied after the step 3 run (the run reads the library as it stands).

## The generic pieces, in the prelude

- **`$Joined: fn [$a] -> $[a]`**: a list of dynamics as one dynamic of their values, which
  changes when any of them does. A plain builtin: the list is fixed, so nothing is kept
  between ticks.
- **`@FlattenMap: fn prompt $(k ==> $v) -> @$(k ==> v)`**: a map of dynamics as one dynamic
  map of their values, which changes when the map or any entry does. A widget, as
  `@Flatten` (`$$a`) is: it keeps its subscriptions to a changing set of entries between
  ticks. It is to `@PerKey` what `@Flatten` is to `@Switch`'s dynamic.
- **Paths (primitive, generic)**: `Prefix: fn Path, Nat -> Path` (the first n steps),
  `Under: fn Nat, Path -> Path` (one position deeper, `0/..`), `KeyAt: forall k. fn Path,
  Nat -> k where k: ^Pod` (the nth key read back; a path without one is a fault of what
  built it).

## The library over them

- `type Guests d = $(Path ==> Guest d)`: a map of the guests' values, no longer of their
  dynamics. A `for at, $guest <- $guests:` row still reads its own guest as a dynamic
  (ruling FV), so `@Outgoing` reads as before.
- `SeatPathOf p = Prefix p, 2`; `SeatIdAt<<d, r>> p = KeyAt<<SeatId d r>> p, 0` (with
  `SeatId d _: ^Pod` asked by `^Document`, which every hosted seat already needs: a key on
  a path is plain data).
- `$Merge trees = MergedUnder ($Joined trees)`, `MergedUnder` a plain function over the
  list of maps: each map's paths `Under` its position.
- `@GuestsOf role, $tree` (a widget now: it binds `@FlattenMap`):
  `$elements <- @FlattenMap $tree`, then `@ GuestsIn<<d, r>> role, $elements`, where
  `GuestsIn` keeps each element of the `Seat` family by a family pattern and packs it,
  `any ((role, (SeatIdAt<<d, r>> path), element))`. `@HostRole` binds it on a line.

Nothing in the compiler or the runtime then names a document, a family or a library
function: the runtime's `documents` module and the lowering's `IsLibraryBuiltin` go.
