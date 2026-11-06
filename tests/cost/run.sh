#!/usr/bin/env bash
# The cost tests (tests/cost/README.md): work counted, not time. Each pair of programs
# differs only in how often the costly thing is asked for; both are built and their
# transcripts checked by `./prism test`, then each test binary is run again under
# `/usr/bin/time -l` for the instructions it retires. Ten readers of a costly `let` must
# cost under twice one reader's (a `let` is computed once per write behind it, not once per
# read); forty writes of a large `for`'s source with the value it has must cost under twice
# one write's (a builder whose source is the map it last built from does nothing).
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
    if [ "$target" = cpp ]; then bin=target/prism-cpp/${name//-/_}/${name//-/_}; else bin=target/prism-build/release/${name//-/_}; fi
    instructions "$bin"
}
failed=0
# pair LABEL FEW MANY: MANY must cost under twice FEW
pair() {
    local few many
    few=$(count "$2")
    many=$(count "$3")
    echo "cost ($target): $1: $2 $few instructions, $3 $many"
    if [ "$few" -gt 0 ] && [ "$many" -gt 0 ] && [ "$many" -lt $(( 2 * few )) ]; then
        echo "cost: $1: ok"
    else
        echo "cost: $1: FAILED: $3 costs $many instructions, not under twice $2's $few"
        failed=1
    fi
}
pair "a let read by ten" one-reader ten-readers
pair "a builder over its last source" builder-one-write builder-many-writes
[ $failed = 0 ] && echo "cost: ok" || { echo "cost: some failed"; exit 1; }
