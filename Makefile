# The single commands (docs/architecture.md, "Single commands"). Written by Claude,
# 2 November 2026, on the claude-self-hosting branch. Subject to change.

.PHONY: test bootstrap bootstrap-update bootstrap-cpp bootstrap-cpp-update chess

# Every Rust test, every compiled transcript, the checker's tests, and the bootstrap's
# fixed point.
test:
	cargo test --workspace
	tests/compiled/run.sh
	tests/checker/run.sh
	tests/driver/run.sh
	make -C runtime/cpp test
	make -C worlds/cli-cpp test
	tests/compiled/run.sh --target cpp
	bootstrap/bootstrap.sh

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

# Chess compiled by the newest prismc, built with the canvas World, opened in a window.
chess:
	./prism run examples/chess
