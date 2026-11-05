#!/usr/bin/env bash
# The cost tests (tests/cost/README.md): work counted, not time. Each pair of programs
# differs only in how many places read one costly dynamic `let`; both are built and their
# transcripts checked by `./prism test`, then each test binary is run again under
# `/usr/bin/time -l` for the instructions it retires. Ten readers must cost under twice one
# reader's instructions: a `let` is computed once per write behind it, not once per read.
# macOS only (`time -l` reports instructions there).
#
#   tests/cost/run.sh [--target cpp]       # Rust by default (its test build), or C++ at -O3
#
# Written by Claude on the claude-self-hosting branch; subject to change.
set -u
cd "$(dirname "$0")/../.." || exit 2
target=rust
[ "${1:-}" = --target ] && target=${2:-rust}
instructions() { # binary: instructions its `test` run retires
    /usr/bin/time -l "$1" test 2>&1 >/dev/null | awk '/instructions retired/ { print $1 }'
}
count() { # NAME: build and check tests/cost/NAME, then count its run
    local name=$1 bin
    ./prism test --target "$target" "tests/cost/$name" > /dev/null 2>&1 || { echo "cost: tests/cost/$name fails its transcript" >&2; echo 0; return; }
    if [ "$target" = cpp ]; then bin=target/prism-cpp/${name//-/_}/${name//-/_}; else bin=target/prism-build/debug/${name//-/_}; fi
    instructions "$bin"
}
one=$(count one-reader)
ten=$(count ten-readers)
echo "cost ($target): one reader $one instructions, ten readers $ten"
if [ "$one" -gt 0 ] && [ "$ten" -gt 0 ] && [ "$ten" -lt $(( 2 * one )) ]; then
    echo "cost: ok"
else
    echo "cost: FAILED: ten readers of a let cost $ten instructions, not under twice one reader's $one"
    exit 1
fi
