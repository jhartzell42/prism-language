# Paths as lists of lists

*Written by Claude on the claude-self-hosting branch, 15 November 2026. Subject to change;
implementers use discretion.*

Ruling PL (claude/rulings-since-the-bible.md): a path is a list of lists of positions, one
list per widget application. This page says what that buys, how the runtime and the test
language spell it, and how the lowering gets there. It is the plan the work follows, kept
current as pieces land.

## Goals

- **Nothing a widget or an arm builds renumbers another place.** `@Wide n` builds one
  place or two, and the line under it is position 1 either way.
- **No row is ever numbered late.** Every place's path is known when it is built, so the
  lowering's late numbering (a row's `base`, `inner_gap`, `number_places`, `lateGens`,
  `lateRowGens`, gappy generations, `DeferredLine`'s numbering refusals) has nothing to do.
- **Applications stay transparent to the program.** A path is opaque but for equality and
  order (Language ch. 6), so no program sees the change; transcripts and test scripts do.

## The spelling

- `/` starts a widget application's own list; `.` steps inside one.
- `0/1`: the caller's position 0 is a widget application, and this is its position 1.
- `1.0`: position 1 is a building arm (or a `for` row, or a generation), and this is the
  arm's position 0.
- `2.#0.[k]/0`: position 2, generation 0, key `k`, then a widget applied there.
- A host's key holds the guest's own path in the same spelling, `1.[0.0]/0`: the host at
  1, the guest's place `0.0`, and the supplier's answer, a widget applied under the key.

## What takes a position

- An element statement, `rebuild`, `for`, `@started`, `@Host`: as before.
- A widget application (a widget function, a local widget, a widget block, a blueprint
  value applied): one position in the caller's list, and a new list for its places.
- A building `match` or `if`: one position, whatever its arm builds; the arm's places
  extend the list (`1.0`).
- The primitives with no places (`@Switch`, `@Flatten`, `@Distinct`, `@PerKey`, `@Defer`,
  `@Later`, `@Run`, `@Machine`): none, as before.

## The runtime (done, ba983bb6)

- `Segment::Call` marks a widget's list starting; `Path::call()` adds it.
- The printer writes `/` for `Call` and `.` between other segments.
- The reader takes `1/0.#0.0` back: a number with a fraction lexes as one token, whose
  text is read as positions.
- A host's answer row stands under `[p]` then `Call`, so a forwarded place is `[p]/0`.

## The test language (done, 1d3f334c)

- `LexPath` reads `/` as `PstCall` and `.` as a step; `PsCall` in the tree.

## The lowering (to do)

- **Positions carry a prefix.** `Pos` gets `prefix: [Nat]`, the caller positions of the
  inlined applications it stands inside. Inlining a widget (`InlineWidget`'s one-clause
  branch, `InlineBlock`, a local widget) at caller position `p` lowers the body at
  `Pos(scope, prefix ++ [p], 0)` and resumes the caller at `p + 1`.
- **`PlaceAt`** spells `path.at(p0).call().at(p1).call()…at(at)`; `PosPat` matches the
  same segments, `Pos(p0), Call, …, Pos(at)`; `PlaceText` names a place by its whole
  prefix so field names stay unique.
- **The nine transparent generations** (building `if`/`match` arms, clause widgets static
  and dynamic, shared rows of a widget that calls itself, applied blueprints) become
  generations under their own place: their rows are built with the generation's place
  path (plus `Call` for a widget's rows) and number from 0. `transparent=False`,
  `gappy=False`, width 1. `TransparentGenArm` goes; such a generation's answers match
  `Pos(at), rest` (an arm's) or `Pos(at), Call, rest` (a widget's), like `TypedGenArm`
  without the generation segment.
- **Wide generations** (a widget that calls itself, `wideGeneration` positions) become a
  generation under one place with `Call`, each level's places in its own list.
- **Late numbering** then never fires: once every generation is non-gappy, `PlaceNumber`
  is a constant and `PlaceUnknownAtBuild` false. Delete it once the full run is green.
- **`DeferredLine`** stays for build order (a line that takes a late value is built after
  the row that gives it) but needs no numbering.

## Tests

- `paths-widget-application-starts-a-list`, `paths-generation-inside-a-called-widget`:
  derived by hand.
- The six late-value tests, re-derived under PL.
- Every other transcript moves mechanically; those come from the implementation's run,
  audited for anything beyond the spelling, in one commit by ruling PL.

## Negative space

- **No stable place identities instead of positions.** Buys transcripts that still read
  as positions in source order.
- **No runtime renumbering.** Buys paths fixed at build.
