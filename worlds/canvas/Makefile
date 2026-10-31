# The canvas World's targets. Run from anywhere; they work in the repository root.
#
#   make -f worlds/canvas/Makefile          chess in a window (the default target)
#   make -f worlds/canvas/Makefile check    build, test and lint the World on any platform
#
# `chess` is built for the machine it runs on: on a Mac, the native macOS build. It
# builds release, since compiler zero takes about half a second per click in a debug
# build and a tenth of that in release.

ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/../..)
CHESS := examples/chess/rules.prism examples/chess/board.prism examples/chess/window.prism

.PHONY: chess check

chess:
	cd $(ROOT) && cargo run --release -p prism_zero -- run --world canvas $(CHESS)

check:
	cd $(ROOT) && cargo build -p prism_world_canvas && cargo test -p prism_world_canvas && cargo clippy -p prism_world_canvas
