# The single commands (docs/architecture.md, "Single commands"). Written by Claude,
# 1 November 2026, on the claude-self-hosting branch. Subject to change.

.PHONY: test bootstrap chess compiled-tests

# Every Rust test, the front end against prism_syntax, the checker's tests, every
# transcript under compiled code, and the bootstrap's fixed point when stage 3 exists.
test:
	cargo test --workspace
	tests/prismc/compare.sh
	tests/prismc/check/run.sh
	tests/compiled/run.sh
	if [ -x target/stage3/prismc ]; then tests/bootstrap/check.sh; fi

# Stage 1 (stage 0, the old-chain prismc, compiles prismc), stage 2, stage 3, and their
# byte equality.
bootstrap:
	tests/bootstrap/stage1.sh
	tests/bootstrap/stage2.sh
	tests/bootstrap/stage3.sh
	tests/bootstrap/check.sh

compiled-tests:
	tests/compiled/run.sh

# Chess compiled by the newest prismc, built with the canvas World, opened in a window.
chess:
	./prism run examples/chess
