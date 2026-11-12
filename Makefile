# The single commands (claude/architecture.md, "Single commands"). Written by Claude,
# 2 November 2026, on the claude-self-hosting branch. Subject to change.

.PHONY: test test-sanitize bootstrap bootstrap-update bootstrap-cpp bootstrap-cpp-update chess basic-chess

# Every Rust test, every compiled transcript, the checker's tests, the C++ runtime's and
# Worlds' tests, the bootstrap's fixed point, and no definition in prismc/ nothing uses.
test:
	cargo test --workspace
	tests/compiled/run.sh
	tests/checker/run.sh
	tests/driver/run.sh
	make -C runtime/cpp test
	make -C worlds/cli/cpp test
	make -C worlds/canvas/cpp test
	tests/compiled/run.sh --target cpp
	bootstrap/bootstrap.sh
	@! ./prism check --unused prismc/ | grep -A2 'warning\[unused\]' || { echo "prismc has definitions nothing uses"; false; }

# The C++ compiled suite with every program and the runtime under ASan and UBSan: finds
# undefined behavior that -O3 alone would hide (Jimmy). Slow; run it before a PR.
test-sanitize:
	PRISM_SANITIZE=1 tests/compiled/run.sh --target cpp

# prismc compiles itself to a fixed point and is installed as target/bootstrap/prismc,
# starting from the seed archive on a fresh clone (bootstrap/README.md).
bootstrap:
	bootstrap/bootstrap.sh

# The same, and the seed archive rewritten from the new fixed point.
bootstrap-update:
	bootstrap/bootstrap.sh --update

# prismc compiled to C++ compiles itself to a fixed point, and to the Rust the Rust prismc
# emits; installs `target/bootstrap/prismc-cpp` (bootstrap/README.md, "The C++ target").
bootstrap-cpp:
	bootstrap/bootstrap-cpp.sh

bootstrap-cpp-update:
	bootstrap/bootstrap-cpp.sh --update

# Chess the document way, compiled by the newest prismc, built with the canvas World: the
# hot seat, two boards in one window (claude/document-chess.md).
chess:
	./prism run examples/chess

# The basic chess, one board with its settings dialog, in a window.
basic-chess:
	./prism run examples/basic-chess
