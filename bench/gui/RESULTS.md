# GUI benchmark results

*Written by bench/run.sh on 2026-11-08 11:43 EST; regenerate, do not edit.*

## The machine and the versions

- **Machine:** Intel(R) Xeon(R) Processor, 2 cores, 8 GB, Ubuntu 24.04.5 LTS, Linux 6.18.44-fc-v310; display :99, Xvfb under xvfb-run (no real display: the frame is handed to a server that shows nothing)
- **Load average:** 1.00 1.03 1.30 at the start, 1.00 1.00 1.10 at the end
- **prism-language:** 83183b7 (with uncommitted changes); prismc installed 2026-11-08 09:43
- **Rust:** rustc 1.97.0 (2d8144b78 2026-07-07), cargo release (opt-level 3)
- **GTK:** 3.24.41, cc -O2
- **Runs:** 10 timed after 2 untimed, one process per operation

## Time, ms: median (fastest–slowest)

From the input (a press queued, a button clicked, an action performed) to the frame
that shows its result handed to the window system.

| Operation | prism-rust | gtk |
|---|---|---|
| create1k | 60.0 (46.7–93.3) | 395.5 (353.2–435.0) |
| replace1k | 75.1 (66.1–99.1) | 496.7 (439.2–636.6) |
| update10th | 10.9 (8.6–15.3) | 49.8 (45.6–79.0) |
| select | 6.7 (4.8–11.2) | 6.0 (5.2–6.5) |
| swap | 8.0 (5.1–12.6) | 108.5 (94.9–145.6) |
| remove | 11.7 (8.7–18.4) | 32.4 (26.5–40.8) |
| create10k | 642.5 (537.7–786.3) | 39145.6 (36079.1–42977.0) |
| append1k | 50.9 (47.2–57.3) | 974.6 (836.7–1222.9) |
| clear | 29.9 (21.0–44.0) | 100.8 (87.0–122.4) |

## Peak memory, MB

Each process's maximum resident set size (`/usr/bin/time -v`), over its whole run.

| Operation | prism-rust | gtk |
|---|---|---|
| create1k | 100 | 100 |
| replace1k | 138 | 100 |
| update10th | 102 | 100 |
| select | 102 | 100 |
| swap | 102 | 104 |
| remove | 100 | 100 |
| create10k | 622 | 311 |
| append1k | 148 | 124 |
| clear | 103 | 100 |
