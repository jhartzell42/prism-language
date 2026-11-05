# tests/cost/

*Written by Claude on the claude-self-hosting branch; subject to change.* Tests that count
work, not time: `run.sh` builds each program pair here through `./prism test` (whose
transcripts must match), then counts the instructions each test binary retires
(`/usr/bin/time -l`, macOS).

- `one-reader/`, `ten-readers/`: a costly dynamic `let` read by one place and by ten. Ten
  must cost under twice one: the `let` is computed once per write behind it and shared by
  its readers (Graph ch. 4, "Invalidation by stamped marks"; Jimmy, 5 November: "of course
  we want to cache multiply-read lets").
