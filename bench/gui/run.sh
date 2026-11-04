#!/usr/bin/env bash
# The GUI benchmark (bench/gui/README.md): js-framework-benchmark's operations on the same
# table in Prism compiled to C++ and to Rust, in C with GTK 3, and in Cuis Smalltalk on
# the Cog VM. Builds each, runs every operation RUNS times after WARMUP untimed runs, one
# process per operation and implementation, and writes bench/gui/RESULTS.md: the machine, the
# versions, the median time of each operation with its spread, and each process's peak
# memory. The raw numbers go to target/bench/results/.
#
#   bench/gui/run.sh [IMPL...]          # prism-cpp prism-rust gtk cuis, or those named
#   RUNS=10 WARMUP=2 bench/gui/run.sh   # the defaults
#
# Run it with nothing else running: no builds, no other apps busy. The load average at the
# start and the end is recorded.
#
# Written by Claude on the claude-self-hosting branch; subject to change.
set -eu
cd "$(dirname "$0")/../.."
runs=${RUNS:-10}
warmup=${WARMUP:-2}
ops=(create1k replace1k update10th select swap remove create10k append1k clear)
impls=("$@")
[ ${#impls[@]} -gt 0 ] || impls=(prism-cpp prism-rust gtk cuis)
raw=target/bench/results
mkdir -p "$raw"

# The builds, before any timing.
for impl in "${impls[@]}"; do
    case "$impl" in
        prism-cpp) make -s -C bench/gui/prism cpp ;;
        prism-rust) make -s -C bench/gui/prism rust ;;
        gtk) make -s -C bench/gui/gtk ;;
        cuis) bench/gui/cuis/setup.sh ;;
        *) echo "bench: unknown implementation $impl (prism-cpp prism-rust gtk cuis)" >&2; exit 2 ;;
    esac
done

command_of() { # impl
    case "$1" in
        prism-cpp) echo target/bench/prism/driver ;;
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

load_start=$(sysctl -n vm.loadavg 2>/dev/null || cat /proc/loadavg)
for impl in "${impls[@]}"; do
    cmd=$(command_of "$impl")
    for op in "${ops[@]}"; do
        out=$raw/$impl-$op.txt
        err=$raw/$impl-$op.err
        echo "bench: $impl $op ($runs runs after $warmup)" >&2
        if ! /usr/bin/time -l $cmd "$op" "$runs" "$warmup" > "$out" 2> "$err"; then
            echo "bench: $impl $op failed (see $err)" >&2
            echo "failed" >> "$out"
        fi
    done
done
load_end=$(sysctl -n vm.loadavg 2>/dev/null || cat /proc/loadavg)

# ---- bench/RESULTS.md ----------------------------------------------------------------

cell() { # impl op: "median (min–max)"
    local s; s=$(stats "$raw/$1-$2.txt")
    set -- $s
    if [ "$4" = 0 ]; then echo "failed"; else echo "$1 ($2–$3)"; fi
}
rss_mb() { # impl op: the process's peak resident memory, MB (`time -l`, bytes on macOS)
    awk '/maximum resident set size/ { printf "%.0f", $1 / 1048576 }' "$raw/$1-$2.err"
}

{
    echo "# GUI benchmark results"
    echo
    echo "*Written by bench/run.sh on $(date '+%Y-%m-%d %H:%M %Z'); regenerate, do not edit.*"
    echo
    echo "## The machine and the versions"
    echo
    echo "- **Machine:** $(sysctl -n machdep.cpu.brand_string 2>/dev/null), $(sysctl -n hw.ncpu) cores ($(sysctl -n hw.perflevel0.physicalcpu 2>/dev/null) performance, $(sysctl -n hw.perflevel1.physicalcpu 2>/dev/null) efficiency), $(( $(sysctl -n hw.memsize) / 1073741824 )) GB, $(sw_vers -productName) $(sw_vers -productVersion)"
    echo "- **Load average:** $load_start at the start, $load_end at the end"
    echo "- **prism-language:** $(git rev-parse --short HEAD)$(git diff --quiet || echo ' (with uncommitted changes)'); prismc installed $(stat -f %Sm -t '%F %R' target/bootstrap/prismc 2>/dev/null || stat -c %y target/bootstrap/prismc | cut -c1-16)"
    echo "- **C++:** $(clang++ --version | head -1), -O2; **Rust:** $(rustc --version), cargo release (opt-level 3)"
    echo "- **GTK:** $(pkg-config --modversion gtk+-3.0), cc -O2"
    echo "- **Cuis:** $( [ -f target/bench/deps/cuis.commit ] && echo "Cuis-Smalltalk-Dev $(cut -c1-10 target/bench/deps/cuis.commit), $(basename "$(ls target/bench/deps/cuis/CuisImage/Cuis*.image | head -1)")"); VM $(strings target/bench/deps/cuis/CuisVM.app/Contents/MacOS/Squeak 2>/dev/null | grep -m1 -o 'VMMaker[^ ]*' | tr -d ']' || true)"
    echo "- **Runs:** $runs timed after $warmup untimed, one process per operation"
    echo
    echo "## Time, ms: median (fastest–slowest)"
    echo
    echo "From the input (a press queued, a button clicked, an action performed) to the frame"
    echo "that shows its result handed to the window system (bench/README.md, \"What is timed\")."
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
    echo "Each process's maximum resident set size (\`/usr/bin/time -l\`), over its whole run."
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
