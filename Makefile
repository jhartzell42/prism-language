# The single commands (docs/architecture.md, "Single commands"). Written by Claude,
# 31 October 2026, on the claude-self-hosting branch. Subject to change.

CHESS := examples/chess/board.prism examples/chess/rules.prism examples/chess/window.prism

.PHONY: test bootstrap chess compiled-tests

# Every Rust test, the front end against prism_syntax, the checker's tests when present,
# the bootstrap's fixed point, and every transcript under compiled code.
test:
	cargo test --workspace
	tests/prismc/compare.sh
	if [ -x tests/prismc/check/run.sh ]; then tests/prismc/check/run.sh; fi
	tests/bootstrap/check.sh
	tests/bootstrap/compiled-tests.sh target/stage3/prismc

# Stage 1 (compiler zero runs prismc), stage 2, stage 3, and their byte equality.
bootstrap:
	tests/bootstrap/stage1.sh
	tests/bootstrap/stage2.sh
	tests/bootstrap/stage3.sh
	tests/bootstrap/check.sh

compiled-tests:
	tests/bootstrap/compiled-tests.sh

# Chess compiled by the newest prismc, built with the canvas World, opened in a window.
chess:
	@p=$$(ls -t target/stage3/prismc target/stage2/prismc target/stage1/prismc 2>/dev/null | head -1); \
	if [ -z "$$p" ]; then tests/bootstrap/stage1.sh; p=target/stage1/prismc; fi; \
	$$p -- compile $(CHESS) -o target/chess --name chess && \
	(cd target/chess && CARGO_TARGET_DIR=../chess-build cargo build --release --features canvas -q) && \
	target/chess-build/release/chess --world canvas
