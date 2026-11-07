# Building Prism: results

*Written by bench/building/run.sh on 2026-11-06 18:30 EST; regenerate, do not edit.*

- **Machine:** Intel(R) Xeon(R) Processor, 2 cores, 8 GB, Ubuntu 24.04.5 LTS, Linux 6.18.44-fc-v260; load 1.78 1.94 1.97
- **prism-language:** 7d68a86
- **Rust:** rustc 1.97.0 (2d8144b78 2026-07-07), cargo -j2; **C++:** Ubuntu clang version 18.1.3 (1ubuntu1), make -j2
- **Emitting:** the installed fixed points, Rust by `target/bootstrap/prismc`, C++ by `target/bootstrap/prismc-cpp`

From Prism source to a runnable binary, from clean, seconds. Emit is prismc's run, and
lines/s its emitted lines (every file under the emitted src/) over that time; build is
cargo's or make's, the runtime library and the Worlds included. Rebuild is the same build
run again after the emitted program's own sources are touched: the program compiled
and linked once more over the runtime library and the dependencies the clean build
left, which is what an edit to a Prism program costs after its first build.

## prismc (33506 lines of Prism with the prelude; emitted: 115103 lines of Rust, 188493 of C++)

| Configuration | Emit | lines/s | Build | Total | Rebuild | Binary MB |
|---|---|---|---|---|---|---|
| rust-release | 55.0 | 2,093 | 140.9 | 195.9 | 121.2 | 29.5 |
| cpp-O3 | 79.9 | 2,359 | 789.8 | 869.7 | 765.2 | 50.8 |

## chess (3076 lines of Prism with the prelude; emitted: 18293 lines of Rust, 23698 of C++)

| Configuration | Emit | lines/s | Build | Total | Rebuild | Binary MB |
|---|---|---|---|---|---|---|
| rust-release | 1.5 | 12,195 | 72.9 | 74.4 | 19.4 | 7.4 |
| cpp-O3 | 2.2 | 10,772 | 72.9 | 75.1 | 47.4 | 6.3 |

## table (2133 lines of Prism with the prelude; emitted: 14391 lines of Rust, 18711 of C++)

| Configuration | Emit | lines/s | Build | Total | Rebuild | Binary MB |
|---|---|---|---|---|---|---|
| rust-release | 1.0 | 14,391 | 69.8 | 70.8 | 16.4 | 6.9 |
| cpp-O3 | 1.6 | 11,694 | 63.4 | 65.0 | 42.6 | 5.3 |

