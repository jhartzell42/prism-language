#!/usr/bin/env bash
# The GUI benchmark: js-framework-benchmark's operations on the same
# table in Prism compiled to Rust, and in C with GTK 3 (the Cuis Smalltalk
# table is run only when named: Jimmy took it out of the scripted runs). Builds each, runs every operation RUNS times after WARMUP untimed runs, one
# process per operation and implementation, and writes bench/gui/RESULTS.md: the machine, the
# versions, the median time of each operation with its spread, and each process's peak
# memory. The raw numbers go to target/bench/results/.
#
#   bench/gui/run.sh [IMPL...]          # prism-rust gtk, or those named (cuis too)
#   RUNS=10 WARMUP=2 bench/gui/run.sh   # the defaults
#
# Run it with nothing else running: no builds, no other apps busy. The load average at the
# start and the end is recorded. On Linux with no DISPLAY the whole script runs itself
# again under `xvfb-run -a`, one virtual X server for every run (and xvfb-run joins its
# command's stderr to stdout); on macOS the display must be awake.
#
# Written by Claude on the claude-self-hosting branch; subject to change.
set -eu
self=$(cd "$(dirname "$0")" && pwd)/$(basename "$0")  # absolute, for the run under xvfb-run below
cd "$(dirname "$0")/../.."
. bench/lib.sh
# installed_at FILE: its modification time, to the minute (BSD stat on macOS, GNU stat else;
# `stat -f` is a file-system query under GNU stat, so the platform chooses)
installed_at() {
    case "$bench_os" in
        Darwin) stat -f %Sm -t '%F %R' "$1" ;;
        *) stat -c %y "$1" | cut -c1-16 ;;
    esac
}
if [ "$bench_os" != Darwin ] && [ -z "${DISPLAY:-}" ]; then
    command -v xvfb-run > /dev/null || bench_die "no DISPLAY and no xvfb-run to make one (apt install xvfb)"
    echo "bench: no DISPLAY; running under xvfb-run -a" >&2
    PRISM_BENCH_XVFB=1 exec xvfb-run -a "$self" "$@"
fi
timecmd=$(time_cmd)
runs=${RUNS:-10}
warmup=${WARMUP:-2}
ops=(create1k replace1k update10th select swap remove create10k append1k clear)
impls=("$@")
[ ${#impls[@]} -gt 0 ] || impls=(prism-rust gtk)
raw=target/bench/results
mkdir -p "$raw"

# The builds, before any timing.
for impl in "${impls[@]}"; do
    case "$impl" in
        prism-rust) make -s -C bench/gui/prism rust ;;
        gtk) make -s -C bench/gui/gtk ;;
        cuis) bench/gui/cuis/setup.sh ;;
        *) echo "bench: unknown implementation $impl (prism-rust gtk cuis)" >&2; exit 2 ;;
    esac
done

command_of() { # impl
    case "$1" in
        prism-rust) echo target/bench/prism-rs/driver ;;
        gtk) echo target/bench/gtk/table ;;
        cuis) echo bench/gui/cuis/run.sh ;;
    esac
}

# stats FILE: "median min max n" of the third column of `OP RUN MS` lines
stats() {
    awk '$1 != "peak_rss_bytes" && NF == 3 { print $3 }' "$1" | sort -g | awk '
        { v[NR] = $1 }
        END {
            if (NR == 0) { print "- - - 0"; exit }
            m = (NR % 2) ? v[(NR + 1) / 2] : (v[NR / 2] + v[NR / 2 + 1]) / 2
            printf "%.1f %.1f %.1f %d\n", m, v[1], v[NR], NR
        }'
}

load_start=$(load_line)
for impl in "${impls[@]}"; do
    cmd=$(command_of "$impl")
    for op in "${ops[@]}"; do
        out=$raw/$impl-$op.txt
        err=$raw/$impl-$op.err
        echo "bench: $impl $op ($runs runs after $warmup)" >&2
        if ! $timecmd $cmd "$op" "$runs" "$warmup" > "$out" 2> "$err"; then
            echo "bench: $impl $op failed (see $err)" >&2
            echo "failed" >> "$out"
        fi
    done
done
load_end=$(load_line)

# ---- bench/gui/RESULTS.md ------------------------------------------------------------

cell() { # impl op: "median (min–max)"
    local s; s=$(stats "$raw/$1-$2.txt")
    set -- $s
    if [ "$4" = 0 ]; then echo "failed"; else echo "$1 ($2–$3)"; fi
}
rss_mb() { # impl op: the process's peak resident memory, MB (`time -l`'s bytes on macOS, GNU `time -v`'s kbytes)
    peak_mb_of "$raw/$1-$2.err"
}

{
    echo "# GUI benchmark results"
    echo
    echo "*Written by bench/run.sh on $(date '+%Y-%m-%d %H:%M %Z'); regenerate, do not edit.*"
    echo
    echo "## The machine and the versions"
    echo
    case "$bench_os" in
        Darwin) echo "- **Machine:** $(sysctl -n machdep.cpu.brand_string 2>/dev/null), $(sysctl -n hw.ncpu) cores ($(sysctl -n hw.perflevel0.physicalcpu 2>/dev/null) performance, $(sysctl -n hw.perflevel1.physicalcpu 2>/dev/null) efficiency), $(( $(sysctl -n hw.memsize) / 1073741824 )) GB, $(sw_vers -productName) $(sw_vers -productVersion)" ;;
        *) echo "- **Machine:** $(machine_line); display $DISPLAY$( [ -n "${PRISM_BENCH_XVFB:-}" ] && echo ", Xvfb under xvfb-run (no real display: the frame is handed to a server that shows nothing)")" ;;
    esac
    echo "- **Load average:** $load_start at the start, $load_end at the end"
    echo "- **prism-language:** $(git rev-parse --short HEAD)$(git diff --quiet || echo ' (with uncommitted changes)'); prismc installed $(installed_at target/bootstrap/prismc)"
    echo "- **Rust:** $(rustc --version), cargo release (opt-level 3)"
    echo "- **GTK:** $(pkg-config --modversion gtk+-3.0), cc -O2"
    case " ${impls[*]} " in *" cuis "*)
        echo "- **Cuis:** $( [ -f target/bench/deps/cuis.commit ] && echo "Cuis-Smalltalk-Dev $(cut -c1-10 target/bench/deps/cuis.commit), $(basename "$(ls target/bench/deps/cuis/CuisImage/Cuis*.image | head -1)")"); VM $(strings target/bench/deps/cuis/CuisVM.app/Contents/MacOS/Squeak 2>/dev/null | grep -m1 -o 'VMMaker[^ ]*' | tr -d ']' || true)" ;;
    esac
    echo "- **Runs:** $runs timed after $warmup untimed, one process per operation"
    echo
    echo "## Time, ms: median (fastest–slowest)"
    echo
    echo "From the input (a press queued, a button clicked, an action performed) to the frame"
    echo "that shows its result handed to the window system."
    echo
    printf "| Operation |"; for i in "${impls[@]}"; do printf " %s |" "$i"; done; echo
    printf "|---|"; for i in "${impls[@]}"; do printf "%s" "---|"; done; echo
    for op in "${ops[@]}"; do
        printf "| %s |" "$op"
        for i in "${impls[@]}"; do printf " %s |" "$(cell "$i" "$op")"; done
        echo
    done
    echo
    echo "## Peak memory, MB"
    echo
    echo "Each process's maximum resident set size (\`$timecmd\`), over its whole run."
    echo
    printf "| Operation |"; for i in "${impls[@]}"; do printf " %s |" "$i"; done; echo
    printf "|---|"; for i in "${impls[@]}"; do printf "%s" "---|"; done; echo
    for op in "${ops[@]}"; do
        printf "| %s |" "$op"
        for i in "${impls[@]}"; do printf " %s |" "$(rss_mb "$i" "$op")"; done
        echo
    done
} > bench/gui/RESULTS.md
echo "bench: wrote bench/gui/RESULTS.md" >&2
