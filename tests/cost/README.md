# tests/cost/

*Written by Claude on the claude-self-hosting branch; subject to change.* Tests that count
work, not time: `run.sh` builds each program pair here through `./prism test` (whose
transcripts must match), then counts the instructions each test binary retires
(`/usr/bin/time -l`, macOS).

- `one-reader/`, `ten-readers/`: a costly dynamic `let` read by one place and by ten. Ten
  must cost under twice one: the `let` is computed once per write behind it and shared by
  its readers (Graph ch. 4, "Invalidation by stamped marks"; Jimmy, 5 November: "of course
  we want to cache multiply-read lets").
- `builder-one-write/`, `builder-many-writes/`: a `for` over 20,000 keys whose hold is
  written with the value it has, once and forty times. Forty must cost under twice one: a
  builder whose source is the very map it last built from (one node) does nothing, rather
  than walking both key sets (claude/incremental-host-document.md, "Without a route").
- `per-key-one-edit/`, `per-key-many-edits/`: a `for` over `@PerKey` of a 2,000-key map,
  each row doing a thousand steps of arithmetic when its value moves, the map written with
  one key changed per click, once and forty times. Forty must cost under twice one: only
  the changed key's inner fires, so a click costs one row's work and a walk of the keys,
  not every row's (Semantics Appendix B, `@PerKey`; round 9 PK). Over the plain `for k, $v
  <- $m`, every row's `$v` fires on every write (pick UD), and forty clicks cost forty
  times the rows.
