# The single commands (docs/architecture.md, "Single commands"). Written by Claude,
# 2 November 2026, on the claude-self-hosting branch. Subject to change.

.PHONY: test bootstrap chess

# Every Rust test, every compiled transcript, the checker's tests, and the bootstrap's
# fixed point.
test:
	cargo test --workspace
	tests/compiled/run.sh
	tests/prismc/check/run.sh
	tests/bootstrap/bootstrap.sh

# The kept prismc compiles prismc into gen1, gen1 compiles it into gen2, byte-equal;
# installs target/bootstrap/prismc.
bootstrap:
	tests/bootstrap/bootstrap.sh

# Chess compiled by the newest prismc, built with the canvas World, opened in a window.
chess:
	./prism run examples/chess
