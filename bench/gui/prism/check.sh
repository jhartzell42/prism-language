#!/usr/bin/env bash
# The window library's cost checks (bench/gui/README.md, "The checks"): both drivers built,
# then each operation run once with `check`, which fails a run whose scene draws texts
# outside the window, or whose update10th, select, swap or remove retires a tenth or more
# of the instructions of the Create 1,000 rows before it. Opens a window for each run, so
# the display must be awake. macOS only (the instruction count is `proc_pid_rusage`'s).
#
#   bench/gui/prism/check.sh [IMPL...]     # prism-cpp prism-rust, or those named
#
# Written by Claude on the claude-self-hosting branch; subject to change.
set -eu
cd "$(dirname "$0")/../../.."
impls=("$@")
[ ${#impls[@]} -gt 0 ] || impls=(prism-cpp prism-rust)
ops=(create1k update10th select swap remove clear)
failed=0
for impl in "${impls[@]}"; do
    case "$impl" in
        prism-cpp) make -s -C bench/gui/prism cpp; cmd=target/bench/prism/driver ;;
        prism-rust) make -s -C bench/gui/prism rust; cmd=target/bench/prism-rs/driver ;;
        *) echo "check: unknown implementation $impl (prism-cpp prism-rust)" >&2; exit 2 ;;
    esac
    for op in "${ops[@]}"; do
        if $cmd "$op" 1 0 check > /dev/null; then
            echo "check: $impl $op: ok"
        else
            echo "check: $impl $op: FAILED"
            failed=1
        fi
    done
done
[ $failed = 0 ] && echo "check: all passed" || { echo "check: some failed"; exit 1; }
