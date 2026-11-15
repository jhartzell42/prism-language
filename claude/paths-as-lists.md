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
- A host's answer row stands under `[p]` then `Call`, so a forwarded place is `[p]/0`, built
  early or late (`HostRow::build` adds the `Call` as `first_value` does); its answers and
  deliveries come in under that `Call`, and one that does not is a fault.

## The test language (done, 1d3f334c)

- `LexPath` reads `/` as `PstCall` and `.` as a step; `PsCall` in the tree.

## The lowering (a8982201, built as q3 and q4; fixes since in 66cdec07)

- **Positions carry a prefix.** `Pos.prefix` holds the caller positions of the inlined
  applications a place stands in. `InCallPos`/`AfterCallPos` move into and out of one;
  `InlineWidgetBody`'s one-clause branch, `InlineBlock` and `LocalWidgetLine` (`InCall`)
  take them. A blueprint `let`, a widget parameter's argument and a top-level blueprint
  `let` lower the expression they name, whose own line starts its list.
- **`PlaceAt`, `PosPat` and the documents** spell the prefix (`InPrefix`, `PrefixPats`);
  `PlaceText` names a place by it.
- **`GenDef.under`**: `UnderGeneration` (a `rebuild`), `UnderArm` (a building `match` or
  `if`, an early choice), `UnderCall` (a widget's clauses static, early, dynamic, bundled
  or shared; an applied blueprint; a block's function of several clauses),
  `UnderCaller` (a line that takes a late result, the lines below a split, an element
  value applied: the caller's list, as before, at the line's own position, its path
  carrying the inlined applications' prefix). Rows under their own place are made
  from a block at position 0 (`RowsWx`), built with the place's path and `row_base` 0
  (`UnderRows`), take one position (`GenWidth`) and leave no gaps (`RowGapsUnder`);
  their answers and documents go by the place's segments (`OwnPlaceGenArm`,
  `OwnPlacePath`).
- **A hosted child's own application** is the guest's root (`Wx.childRoot`): no list of
  its own. A child that is a widget of several clauses or an applied blueprint still
  takes one; to fix if the run shows it.
- **Late numbering** never fires once no generation is gappy; it is deleted once the full
  run is green. `DeferredLine` stays for build order.

## Tests

- `paths-widget-application-starts-a-list`, `paths-generation-inside-a-called-widget`:
  derived by hand.
- The six late-value tests, re-derived under PL.
- Every other transcript moves mechanically; those come from the implementation's run,
  audited for anything beyond the spelling, in one commit by ruling PL (2a6ae70e: each
  section's lines the same with paths taken off, each section's count of distinct paths
  kept). That audit misses a wrong spelling applied the same way everywhere: q4 spelled a
  late host row's places `[p].0`, which a check of the spelling itself found.
- Test scripts' `click at` paths: each old place matched to the new one with the same
  content, the script re-run until its transcript is the old one but for paths.

## Negative space

- **No stable place identities instead of positions.** Buys transcripts that still read
  as positions in source order.
- **No runtime renumbering.** Buys paths fixed at build.
