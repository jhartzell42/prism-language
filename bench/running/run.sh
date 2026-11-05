#!/usr/bin/env bash
# Running Prism once compiled (bench/running/README.md): how fast compiled Prism code runs,
# all-Rust against all-C++. Two workloads: prismc itself, built at each configuration by
# bench/building/run.sh, compiling prismc/ to its own language and compiling
# tests/conformance/sem-ch07-normalize (wall time, peak memory, instructions
# retired); and the GUI table under each target's canvas World (bench/gui/run.sh, which
# also runs GTK and Cuis beside it).
#
#   bench/running/run.sh                       # both configurations, the GUI table too
#   CONFIGS="rust-release" GUI=0 NORM=5 bench/running/run.sh
#
# The configurations are bench/building/run.sh's: rust-release and cpp-O3 (Jimmy: the
# shipping ones only). Writes bench/running/RESULTS.md. Run it with nothing else running,
# after bench/building/run.sh.
#
# Written by Claude on the claude-self-hosting branch; subject to change.
set -eu
cd "$(dirname "$0")/../.."
ulimit -s unlimited 2>/dev/null || ulimit -s 1048576 2>/dev/null || true
norm=${NORM:-5}
gui=${GUI:-1}
built=target/bench/building/prismc
if [ -n "${CONFIGS:-}" ]; then read -r -a configs <<< "$CONFIGS"
else configs=(); for d in rust-release cpp-O3; do [ -x "$built/$d/prismc.bin" ] && configs+=("$d"); done; fi
[ ${#configs[@]} -gt 0 ] || { echo "bench: no prismc built under $built (run bench/building/run.sh prismc)" >&2; exit 2; }
out=target/bench/running
mkdir -p "$out"
normal=tests/conformance/sem-ch07-normalize

now() { python3 -c 'import time; print(time.time())'; }
secs() { python3 -c "import sys; print(f'{float(sys.argv[2]) - float(sys.argv[1]):.1f}')" "$1" "$2"; }
# timed LOG CMD...: CMD under `time -l`, its report in LOG.time; prints the wall seconds
timed() {
    local log=$1; shift
    local t0 t1; t0=$(now)
    /usr/bin/time -l "$@" > "$log" 2> "$log.time" || { echo "bench: failed: $* (see $log.time)" >&2; tail -5 "$log.time" >&2; return 1; }
    t1=$(now)
    secs "$t0" "$t1"
}
peak_mb() { awk '/maximum resident set size/ { printf "%.0f", $1 / 1048576 }' "$1"; }
instr_g() { awk '/instructions retired/ { printf "%.2f", $1 / 1e9 }' "$1"; }
median() { sort -g | awk '{ v[NR] = $1 } END { print v[int((NR + 1) / 2)] }'; }

declare -A self_s self_mb self_gi norm_s norm_mb norm_gi
for c in "${configs[@]}"; do
    bin=$built/$c/prismc.bin
    [ -x "$bin" ] || { echo "bench: no $bin (bench/building/run.sh prismc)" >&2; exit 2; }
    case "$c" in rust-*) target=rust ;; *) target=cpp ;; esac
    dir=$out/$c
    rm -rf "$dir" && mkdir -p "$dir"
    echo "bench: $c: prismc/ compiled to $target by it" >&2
    self_s[$c]=$(timed "$dir/self.log" "$bin" compile prismc --prelude prelude -o "$dir/self" --name prismc --target "$target")
    self_mb[$c]=$(peak_mb "$dir/self.log.time")
    self_gi[$c]=$(instr_g "$dir/self.log.time")
    echo "bench: $c: normalize compiled to $target by it, $norm times" >&2
    : > "$dir/norm.txt"
    for i in $(seq "$norm"); do
        s=$(timed "$dir/norm.log" "$bin" compile "$normal" --prelude prelude -o "$dir/norm" --name norm --target "$target")
        echo "$s $(peak_mb "$dir/norm.log.time") $(instr_g "$dir/norm.log.time")" >> "$dir/norm.txt"
    done
    norm_s[$c]=$(awk '{print $1}' "$dir/norm.txt" | median)
    norm_mb[$c]=$(awk '{print $2}' "$dir/norm.txt" | median)
    norm_gi[$c]=$(awk '{print $3}' "$dir/norm.txt" | median)
done

if [ "$gui" != 0 ]; then
    echo "bench: the GUI table (bench/gui/run.sh)" >&2
    bench/gui/run.sh
fi

{
    echo "# Running Prism once compiled: results"
    echo
    echo "*Written by bench/running/run.sh on $(date '+%Y-%m-%d %H:%M %Z'); regenerate, do not edit.*"
    echo
    echo "- **Machine:** $(sysctl -n machdep.cpu.brand_string), $(getconf _NPROCESSORS_ONLN) cores, $(( $(sysctl -n hw.memsize) / 1073741824 )) GB, $(sw_vers -productName) $(sw_vers -productVersion); load $(sysctl -n vm.loadavg)"
    echo "- **prism-language:** $(git rev-parse --short HEAD)$(git diff --quiet || echo ' (with uncommitted changes)')"
    echo "- **The binaries:** bench/building/run.sh's (bench/building/RESULTS.md says how each was built)"
    echo
    echo "## prismc as the workload"
    echo
    echo "Each binary is a prismc built at that configuration, compiling to its own language."
    echo "Instructions are the processor's \"instructions retired\" (\`time -l\`), in billions; memory"
    echo "is the peak resident set, MB. normalize is \`$normal\`, the median of $norm runs."
    echo
    echo "| Configuration | prismc/: s | MB | G instr | normalize: s | MB | G instr |"
    echo "|---|---|---|---|---|---|---|"
    for c in "${configs[@]}"; do
        echo "| $c | ${self_s[$c]} | ${self_mb[$c]} | ${self_gi[$c]} | ${norm_s[$c]} | ${norm_mb[$c]} | ${norm_gi[$c]} |"
    done
    echo
    if [ "$gui" != 0 ] && [ -f bench/gui/RESULTS.md ]; then
        echo "## The GUI table as the workload"
        echo
        echo "bench/gui/RESULTS.md's timings (prism-cpp is the table compiled to C++ at -O3, prism-rust"
        echo "compiled to Rust as a release build; GTK and Cuis beside them for scale):"
        echo
        sed -n '/^| Operation |/,/^$/p' bench/gui/RESULTS.md | head -12
    fi
} > bench/running/RESULTS.md
echo "bench: wrote bench/running/RESULTS.md" >&2
