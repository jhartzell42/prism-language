#!/usr/bin/env bash
# The window library's cost checks: both drivers built,
# then each operation run once with `check`, which fails a run whose scene draws texts
# outside the window or whose screen does not show the table the operation should leave.
# Opens a window for each run, so the display must be awake; on Linux with no DISPLAY the
# script runs itself again under `xvfb-run -a`. Then a table of each timed step's work by
# phase (prism_rtl's probe; the drivers' `phases` lines), millions, and each step's total
# and paint against the same target's create1k: reported, never a failure. The unit is the
# drivers' (`unit=` on each line) and heads the table: instructions on macOS
# (`proc_pid_rusage`, `thread_selfcounts`) and on a Linux machine whose kernel gives the
# drivers a perf_event instructions counter; on a Linux machine with no hardware counters
# the drivers count each thread's CPU nanoseconds instead (`CLOCK_THREAD_CPUTIME_ID`),
# and the table says ns, never instructions.
#
#   bench/gui/prism/check.sh [IMPL...]     # prism-rust, or those named
#
# Written by Claude on the claude-self-hosting branch; subject to change.
set -eu
self=$(cd "$(dirname "$0")" && pwd)/$(basename "$0")  # absolute, for the run under xvfb-run below
cd "$(dirname "$0")/../../.."
. bench/lib.sh
if [ "$bench_os" != Darwin ] && [ -z "${DISPLAY:-}" ]; then
    command -v xvfb-run > /dev/null || bench_die "no DISPLAY and no xvfb-run to make one (apt install xvfb)"
    echo "check: no DISPLAY; running under xvfb-run -a" >&2
    exec xvfb-run -a "$self" "$@"
fi
impls=("$@")
[ ${#impls[@]} -gt 0 ] || impls=(prism-rust)
ops=(create1k update10th select swap remove clear)
failed=0
logs=$(mktemp -d "${TMPDIR:-/tmp}/prism-check.XXXXXX")
trap 'rm -rf "$logs"' EXIT
for impl in "${impls[@]}"; do
    case "$impl" in
        prism-rust) make -s -C bench/gui/prism rust; cmd=target/bench/prism-rs/driver ;;
        *) echo "check: unknown implementation $impl (prism-rust)" >&2; exit 2 ;;
    esac
    for op in "${ops[@]}"; do
        if $cmd "$op" 1 0 check > "$logs/$impl-$op"; then
            echo "check: $impl $op: ok"
        else
            echo "check: $impl $op: FAILED"
            failed=1
        fi
    done
    grep -h '^phases [a-z0-9]* timed ' $(printf "$logs/$impl-%s " "${ops[@]}") 2>/dev/null | sed "s/^phases /$impl /" >> "$logs/table"
done
# The phase table: each timed step's work by phase in the drivers' unit (instructions, or
# CPU ns where the machine has no counters), millions; `other` is the process's threads'
# work outside every mark; the last two columns are the step's total and its paint
# against the same target's create1k.
if [ -s "$logs/table" ]; then
    awk '
        { impl = $1; op = $2; n++; row[n] = impl " " op
          for (i = 4; i <= NF; i++) { split($i, kv, "="); v[n, kv[1]] = kv[2] }
          u = v[n, "unit"]; if (u == "") u = "unknown"
          if (unit == "") unit = u; else if (unit != u) unit = "MIXED (" unit ", " u ")"
          if (op == "create1k") base[impl] = v[n, "total"] }
        END {
          what = unit == "instructions" ? "instructions retired" : unit == "ns" ? "CPU nanoseconds (no instruction counters on this machine)" : unit
          printf "each timed step by phase, millions of %s:\n", what
          printf "%-10s %-10s %5s %9s %9s %9s %9s %9s %9s %9s %9s %9s %9s %8s %8s\n", "impl", "op", "ticks", "propagate", "build", "patch", "diff", "perform", "compact", "paint", "loop", "other", "total", "/create", "paint/c"
          split("propagate build patch diff perform compact paint loop other total", cols, " ")
          for (r = 1; r <= n; r++) {
            split(row[r], io, " "); b = base[io[1]]
            printf "%-10s %-10s %5d", io[1], io[2], v[r, "ticks"]
            for (c = 1; c <= 10; c++) printf " %9.1f", v[r, cols[c]] / 1e6
            if (b > 0) printf " %8.3f %8.3f\n", v[r, "total"] / b, v[r, "paint"] / b; else printf " %8s %8s\n", "-", "-"
          }
        }' "$logs/table"
fi
[ $failed = 0 ] && echo "check: all passed" || { echo "check: some failed"; exit 1; }
