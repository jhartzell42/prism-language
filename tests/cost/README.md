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
