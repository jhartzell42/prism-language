# GUI benchmark results

*Written by bench/run.sh on 2026-11-06 19:03 EST; regenerate, do not edit.*

## The machine and the versions

- **Machine:** Intel(R) Xeon(R) Processor, 2 cores, 8 GB, Ubuntu 24.04.5 LTS, Linux 6.18.44-fc-v260; display :99, Xvfb under xvfb-run (no real display: the frame is handed to a server that shows nothing)
- **Load average:** 0.98 0.95 1.17 at the start, 1.05 1.01 1.08 at the end
- **prism-language:** d156742 (with uncommitted changes); prismc installed 2026-11-06 17:32
- **C++:** Ubuntu clang version 18.1.3 (1ubuntu1), -O3; **Rust:** rustc 1.97.0 (2d8144b78 2026-07-07), cargo release (opt-level 3)
- **GTK:** 3.24.41, cc -O2
- **Runs:** 10 timed after 2 untimed, one process per operation

## Time, ms: median (fastest–slowest)

From the input (a press queued, a button clicked, an action performed) to the frame
that shows its result handed to the window system (bench/README.md, "What is timed").

| Operation | prism-cpp | prism-rust | gtk |
|---|---|---|---|
| create1k | 76.5 (72.0–81.6) | 58.6 (52.5–60.6) | 328.5 (313.2–429.1) |
| replace1k | 128.0 (117.7–142.5) | 94.8 (82.5–133.5) | 383.5 (376.9–441.4) |
| update10th | 10.2 (8.9–11.1) | 7.6 (6.4–8.9) | 37.1 (36.6–37.4) |
| select | 7.1 (5.9–8.0) | 5.3 (4.1–6.4) | 4.1 (4.0–4.4) |
| swap | 6.8 (6.0–7.4) | 5.2 (4.2–6.0) | 83.1 (82.8–83.4) |
| remove | 15.6 (14.4–16.9) | 10.2 (8.9–11.1) | 16.9 (16.8–20.4) |
| create10k | 1200.5 (1139.1–1511.0) | 1065.0 (829.9–1196.7) | 26976.3 (26384.4–28112.1) |
| append1k | 78.2 (74.1–84.8) | 58.9 (52.7–63.5) | 807.0 (791.6–823.5) |
| clear | 48.1 (44.6–56.9) | 36.6 (33.0–39.8) | 65.4 (64.1–65.7) |

## Peak memory, MB

Each process's maximum resident set size (`/usr/bin/time -v`), over its whole run.

| Operation | prism-cpp | prism-rust | gtk |
|---|---|---|---|
| create1k | 69 | 94 | 100 |
| replace1k | 107 | 137 | 100 |
| update10th | 69 | 96 | 100 |
| select | 69 | 95 | 100 |
| swap | 69 | 96 | 103 |
| remove | 69 | 95 | 100 |
| create10k | 554 | 598 | 311 |
| append1k | 117 | 147 | 123 |
| clear | 69 | 94 | 100 |
