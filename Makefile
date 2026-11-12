# The single commands (claude/architecture.md, "Single commands"). Written by Claude,
# 2 November 2026, on the claude-self-hosting branch. Subject to change.

.PHONY: test bootstrap bootstrap-update chess basic-chess

# Every Rust test, every compiled transcript, the checker's tests, the bootstrap's fixed
# point, and no definition in prismc/ nothing uses.
test:
	cargo test --workspace
	tests/compiled/run.sh
	tests/checker/run.sh
	tests/driver/run.sh
	bootstrap/bootstrap.sh
	@! ./prism check --unused prismc/ | grep -A2 'warning\[unused\]' || { echo "prismc has definitions nothing uses"; false; }

# prismc compiles itself to a fixed point and is installed as target/bootstrap/prismc,
# starting from the seed archive on a fresh clone.
bootstrap:
	bootstrap/bootstrap.sh

# The same, and the seed archive rewritten from the new fixed point.
bootstrap-update:
	bootstrap/bootstrap.sh --update

# Chess the document way, compiled by the newest prismc, built with the canvas World: the
# hot seat, two boards in one window (claude/document-chess.md).
chess:
	./prism run examples/chess

# The basic chess, one board with its settings dialog, in a window.
basic-chess:
	./prism run examples/basic-chess
