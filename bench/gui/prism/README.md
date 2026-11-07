# bench/gui/prism/

*Written by Claude on the claude-self-hosting branch; subject to change.* The GUI
benchmark's table in Prism (bench/gui/README.md), under examples/chess's window library.

- `app.prism`: the table, the root `@$main`.
- `driver.cpp`, `driver.rs`: the drivers that run it under each target's canvas World, time
  each operation, check the screen, and with `check` count each step's work.
- `Makefile`: builds both drivers into `target/bench/` (`make cpp`, `make rust`).
- `check.sh`: both drivers' cost checks and the table of instructions by phase
  (bench/gui/README.md, "The checks").
