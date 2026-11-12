# Building Prism: results

*Written by bench/building/run.sh on 2026-11-08 11:10 EST; regenerate, do not edit.*

- **Machine:** Intel(R) Xeon(R) Processor, 2 cores, 8 GB, Ubuntu 24.04.5 LTS, Linux 6.18.44-fc-v310; load 1.77 1.95 1.98
- **prism-language:** 83183b7 (with uncommitted changes)
- **Rust:** rustc 1.97.0 (2d8144b78 2026-07-07), cargo -j2
- **Emitting:** the installed fixed points, Rust by `target/bootstrap/prismc`

From Prism source to a runnable binary, from clean, seconds. Emit is prismc's run, and
lines/s its emitted lines (every file under the emitted src/) over that time; build is
cargo's, the runtime library and the Worlds included. Rebuild is the same build
run again after the emitted program's own sources are touched: the program compiled
and linked once more over the runtime library and the dependencies the clean build
left, which is what an edit to a Prism program costs after its first build.

## prismc (33821 lines of Prism with the prelude; emitted: 116303 lines of Rust)

| Configuration | Emit | lines/s | Build | Total | Rebuild | Binary MB |
|---|---|---|---|---|---|---|
| rust-release | 36.7 | 3,169 | 199.3 | 236.0 | 146.6 | 29.9 |

## chess (3076 lines of Prism with the prelude; emitted: 18301 lines of Rust)

| Configuration | Emit | lines/s | Build | Total | Rebuild | Binary MB |
|---|---|---|---|---|---|---|
| rust-release | 1.3 | 14,078 | 128.0 | 129.3 | 23.1 | 7.9 |

## table (2133 lines of Prism with the prelude; emitted: 14399 lines of Rust)

| Configuration | Emit | lines/s | Build | Total | Rebuild | Binary MB |
|---|---|---|---|---|---|---|
| rust-release | 0.8 | 17,999 | 128.8 | 129.6 | 19.7 | 7.4 |

