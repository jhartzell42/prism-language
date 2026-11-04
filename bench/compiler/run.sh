#!/usr/bin/env bash
# The compiler benchmark (bench/compiler/README.md): Rust against C++, each pipeline all
# one language. For each build of prismc (Rust as a cargo debug and a release build; C++
# at -O0, -O1, -O2 and -O3, make -j with every core):
#
# - the compile loop: prismc/ from source to a binary, from a clean build directory: the
#   same-language prismc emits it (the Rust-built prismc emits Rust, the C++-built emits
#   C++), then cargo or make builds it, the runtime library included;
# - the compiled code's speed: that binary compiling prismc/ to its own language, and
#   compiling tests/conformance/sem-ch07-normalize.prism (median of NORM runs), each with
#   its wall time, peak memory and, on macOS, the instructions retired (`time -l`).
#
# Writes bench/compiler/RESULTS.md and the raw numbers to target/bench/compiler/.
#
#   bench/compiler/run.sh [CONFIG...]   # rust-debug rust-release cpp-O0 cpp-O1 cpp-O2 cpp-O3
#   NORM=5 bench/compiler/run.sh        # the default
#
# The emitting binaries are the installed fixed points, target/bootstrap/prismc (Rust)
# and target/bootstrap/prismc-cpp (C++): run `make bootstrap bootstrap-cpp` first. Run it
# with nothing else running.
#
# Written by Claude on the claude-self-hosting branch; subject to change.
set -eu
cd "$(dirname "$0")/../.."
ulimit -s unlimited 2>/dev/null || ulimit -s 1048576 2>/dev/null || true
norm=${NORM:-5}
jobs=$(getconf _NPROCESSORS_ONLN)
configs=("$@")
[ ${#configs[@]} -gt 0 ] || configs=(rust-debug rust-release cpp-O0 cpp-O1 cpp-O2 cpp-O3)
rust_prismc=target/bootstrap/prismc
cpp_prismc=target/bootstrap/prismc-cpp
[ -x "$rust_prismc" ] || { echo "bench: no $rust_prismc (make bootstrap)" >&2; exit 2; }
[ -x "$cpp_prismc" ] || { echo "bench: no $cpp_prismc (make bootstrap-cpp)" >&2; exit 2; }
out=target/bench/compiler
mkdir -p "$out"
src=(prismc --prelude prelude)
normal=tests/conformance/sem-ch07-normalize.prism
CXX=${CXX:-clang++}

secs() { python3 -c "import sys; print(f'{float(sys.argv[2]) - float(sys.argv[1]):.1f}')" "$1" "$2"; }
now() { python3 -c 'import time; print(time.time())'; }

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

declare -A loop_emit loop_build self_s self_mb self_gi norm_s norm_gi
for c in "${configs[@]}"; do
    dir=$out/$c
    rm -rf "$dir" && mkdir -p "$dir"
    case "$c" in
        rust-debug|rust-release)
            echo "bench: $c: emit" >&2
            loop_emit[$c]=$(timed "$dir/emit.log" "$rust_prismc" compile "${src[@]}" -o "$dir/prismc" --name prismc)
            profile=$([ "$c" = rust-release ] && echo --release || echo "")
            echo "bench: $c: cargo build $profile (clean)" >&2
            loop_build[$c]=$(timed "$dir/build.log" sh -c "cd '$dir/prismc' && CARGO_TARGET_DIR='$PWD/$dir/target' cargo build --offline -q $profile")
            bin=$dir/target/$([ "$c" = rust-release ] && echo release || echo debug)/prismc
            target=rust ;;
        cpp-*)
            opt=-${c#cpp-}
            echo "bench: $c: emit" >&2
            loop_emit[$c]=$(timed "$dir/emit.log" "$cpp_prismc" compile "${src[@]}" -o "$dir/prismc" --name prismc --target cpp)
            # a clean build: the program and runtime/cpp's library at this level, in a
            # configuration of their own
            rm -rf "runtime/cpp/build/bench$opt"
            echo "bench: $c: make -j$jobs (clean)" >&2
            loop_build[$c]=$(timed "$dir/build.log" make -s -C "$dir/prismc" -j"$jobs" CXX="$CXX" OPT="$opt" CONFIG="bench$opt")
            bin=$dir/prismc/prismc
            target=cpp ;;
        *) echo "bench: unknown configuration $c" >&2; exit 2 ;;
    esac
    echo "bench: $c: prismc/ compiled by it to $target" >&2
    self_s[$c]=$(timed "$dir/self.log" "$bin" compile "${src[@]}" -o "$dir/self" --name prismc --target "$target")
    self_mb[$c]=$(peak_mb "$dir/self.log.time")
    self_gi[$c]=$(instr_g "$dir/self.log.time")
    echo "bench: $c: $normal compiled by it, $norm times" >&2
    : > "$dir/norm.txt"
    for i in $(seq "$norm"); do
        s=$(timed "$dir/norm.log" "$bin" compile "$normal" --prelude prelude -o "$dir/norm" --name norm --target "$target")
        echo "$s $(instr_g "$dir/norm.log.time")" >> "$dir/norm.txt"
    done
    norm_s[$c]=$(sort -g "$dir/norm.txt" | awk '{ v[NR] = $1 } END { print v[int((NR + 1) / 2)] }')
    norm_gi[$c]=$(sort -g "$dir/norm.txt" | awk '{ v[NR] = $2 } END { print v[int((NR + 1) / 2)] }')
done

{
    echo "# Compiler benchmark results"
    echo
    echo "*Written by bench/compiler/run.sh on $(date '+%Y-%m-%d %H:%M %Z'); regenerate, do not edit.*"
    echo
    echo "- **Machine:** $(sysctl -n machdep.cpu.brand_string), $jobs cores, $(( $(sysctl -n hw.memsize) / 1073741824 )) GB, $(sw_vers -productName) $(sw_vers -productVersion); load $(sysctl -n vm.loadavg)"
    echo "- **prism-language:** $(git rev-parse --short HEAD)$(git diff --quiet || echo ' (with uncommitted changes)')"
    echo "- **C++:** $($CXX --version | head -1), make -j$jobs; **Rust:** $(rustc --version), cargo's default parallelism (-j$jobs; release: 16 codegen units, debug: 256)"
    echo
    echo "## The compile loop: prismc/ to a binary, seconds"
    echo
    echo "| Build | Emit | Build (clean) | Total |"
    echo "|---|---|---|---|"
    for c in "${configs[@]}"; do
        echo "| $c | ${loop_emit[$c]} | ${loop_build[$c]} | $(python3 -c "print(f'{${loop_emit[$c]} + ${loop_build[$c]}:.1f}')") |"
    done
    echo
    echo "Emit is the installed fixed point of the same language (the Rust-built prismc, a"
    echo "release build, for Rust; the C++-built one for C++); its time does not depend on the"
    echo "row's level."
    echo
    echo "## The compiled code: each binary compiling, to its own language"
    echo
    echo "| Build | prismc/: s | peak MB | instructions (G) | normalize: s | instructions (G) |"
    echo "|---|---|---|---|---|---|"
    for c in "${configs[@]}"; do
        echo "| $c | ${self_s[$c]} | ${self_mb[$c]} | ${self_gi[$c]} | ${norm_s[$c]} | ${norm_gi[$c]} |"
    done
    echo
    echo "normalize is \`$normal\`, the median of $norm runs."
} > bench/compiler/RESULTS.md
echo "bench: wrote bench/compiler/RESULTS.md" >&2
