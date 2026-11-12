#!/usr/bin/env bash
# The cost tests: work counted, not time. Each pair of programs
# differs only in how often the costly thing is asked for; both are built and their
# transcripts checked by `./prism test`, then each test binary is run again for the
# instructions it retires (bench/lib.sh's `instructions_of`: macOS's `time -l`, `perf
# stat` on a Linux machine with hardware counters, else valgrind's cachegrind, which counts
# every instruction exactly and slowly; the output names the counter used). Ten readers of a costly `let` must
# cost under twice one reader's (a `let` is computed once per write behind it, not once per
# read); forty writes of a large `for`'s source with the value it has must cost under twice
# one write's (a builder whose source is the map it last built from does nothing); forty
# edits of one key each of a large map under `@PerKey` must cost under twice one edit's (only
# the changed key's row does its work).
#
#   tests/cost/run.sh
#
# Written by Claude on the claude-self-hosting branch; subject to change.
set -u
cd "$(dirname "$0")/../.." || exit 2
. bench/lib.sh
instr_setup
echo "cost: instructions counted by $(instr_tool)"
instructions() { # binary: instructions its `test` run retires
    instructions_of "$1" test
}
count() { # NAME: build and check tests/cost/NAME, then count its run
    local name=$1 bin
    ./prism test "tests/cost/$name" > /dev/null 2>&1 || { echo "cost: tests/cost/$name fails its transcript" >&2; echo 0; return; }
    bin=target/prism-build/release/${name//-/_}
    instructions "$bin"
}
failed=0
# pair LABEL FEW MANY: MANY must cost under twice FEW
pair() {
    local few many
    few=$(count "$2")
    many=$(count "$3")
    echo "cost: $1: $2 $few instructions, $3 $many"
    if [ "$few" -gt 0 ] && [ "$many" -gt 0 ] && [ "$many" -lt $(( 2 * few )) ]; then
        echo "cost: $1: ok"
    else
        echo "cost: $1: FAILED: $3 costs $many instructions, not under twice $2's $few"
        failed=1
    fi
}
pair "a let read by ten" one-reader ten-readers
pair "a builder over its last source" builder-one-write builder-many-writes
# marginal LABEL FEW MANY: each of the 39 extra edits of MANY must cost under a twentieth
# of FEW's whole run, which builds the 2,000 rows once: an edit is one row's work and a
# walk of the keys, where doubling the whole run would turn on how large the build counts
# under the machine's counter (Jimmy: the doubling was too strict)
marginal() {
    local few many
    few=$(count "$2")
    many=$(count "$3")
    echo "cost: $1: $2 $few instructions, $3 $many ($(( (many - few) / 39 )) per extra edit)"
    if [ "$few" -gt 0 ] && [ "$many" -gt 0 ] && [ $(( (many - few) / 39 )) -lt $(( few / 20 )) ]; then
        echo "cost: $1: ok"
    else
        echo "cost: $1: FAILED: an extra edit costs $(( (many - few) / 39 )) instructions, not under a twentieth of $2's $few"
        failed=1
    fi
}
marginal "a row per key changed" per-key-one-edit per-key-many-edits
marginal "no kept row woken by a removed key" per-key-drop-one per-key-drop-many
[ $failed = 0 ] && echo "cost: ok" || { echo "cost: some failed"; exit 1; }
