#!/usr/bin/env bash
# Building Prism (bench/building/README.md): from Prism source to a runnable binary, each
# pipeline all one language. The installed Rust prismc emits Rust and cargo builds it; the
# installed C++ prismc emits C++ and make builds it, the runtime library and the Worlds
# included, from clean, with every core. For each subject and each configuration: the emit
# time, the build time, their total, and the binary's size.
#
#   bench/building/run.sh [SUBJECT...]        # prismc chess table, or those named
#   CONFIGS="rust-release" bench/building/run.sh
#
# Configurations (Jimmy: the shipping ones only): rust-release (cargo release, opt-level 3,
# its default parallelism) and cpp-O3 (`make -j` every core with $CXX, clang++ by default).
# The binaries stay
# in target/bench/building/SUBJECT/CONFIG/ for bench/running/run.sh. Writes
# bench/building/RESULTS.md. The emitting binaries are the installed fixed points
# (`make bootstrap bootstrap-cpp`). Run it with nothing else running.
#
# Written by Claude on the claude-self-hosting branch; subject to change.
set -eu
cd "$(dirname "$0")/../.."
export PRISM_ROOT=$PWD
ulimit -s unlimited 2>/dev/null || ulimit -s 1048576 2>/dev/null || true
jobs=$(getconf _NPROCESSORS_ONLN)
CXX=${CXX:-clang++}
subjects=("$@")
[ ${#subjects[@]} -gt 0 ] || subjects=(prismc chess table)
read -r -a configs <<< "${CONFIGS:-rust-release cpp-O3}"
rust_prismc=target/bootstrap/prismc
cpp_prismc=target/bootstrap/prismc-cpp
[ -x "$rust_prismc" ] || { echo "bench: no $rust_prismc (make bootstrap)" >&2; exit 2; }
[ -x "$cpp_prismc" ] || { echo "bench: no $cpp_prismc (make bootstrap-cpp)" >&2; exit 2; }
out=target/bench/building
mkdir -p "$out"

sources_of() { # subject: prismc's arguments naming its sources and its name
    case "$1" in
        prismc) echo "prismc --prelude prelude --name prismc" ;;
        chess) echo "examples/chess --name chess" ;;
        table) echo "bench/gui/prism/bench.prism examples/chess/window.prism --name bench" ;;
        *) echo "bench: unknown subject $1 (prismc chess table)" >&2; exit 2 ;;
    esac
}
lines_of() { # subject: Prism lines
    case "$1" in
        prismc) cat prismc/*.prism prelude/*.prism | wc -l | tr -d ' ' ;;
        chess) cat examples/chess/*.prism prelude/*.prism | wc -l | tr -d ' ' ;;
        table) cat bench/gui/prism/bench.prism examples/chess/window.prism prelude/*.prism | wc -l | tr -d ' ' ;;
    esac
}
now() { python3 -c 'import time; print(time.time())'; }
secs() { python3 -c "import sys; print(f'{float(sys.argv[2]) - float(sys.argv[1]):.1f}')" "$1" "$2"; }
plus() { python3 -c "import sys; print(f'{float(sys.argv[1]) + float(sys.argv[2]):.1f}')" "$1" "$2"; }
mb() { python3 -c "import os,sys; print(f'{os.path.getsize(sys.argv[1]) / 1048576:.1f}')" "$1"; }

declare -A emit_s build_s total_s size_mb emitted_lines
for subject in "${subjects[@]}"; do
    read -r -a src <<< "$(sources_of "$subject")"
    name=${src[${#src[@]}-1]}
    mkdir -p "$out/$subject"
    for lang in rust cpp; do
        # one emit per language; the levels build the same emitted program
        dir=$out/$subject/emit-$lang
        rm -rf "$dir"
        echo "bench: $subject: emitted to $lang by the $lang prismc" >&2
        t0=$(now)
        if [ $lang = rust ]; then "$rust_prismc" compile "${src[@]}" -o "$dir" > "$dir.log" 2>&1
        else "$cpp_prismc" compile "${src[@]}" -o "$dir" --target cpp > "$dir.log" 2>&1; fi
        t1=$(now)
        emit_s[$subject-$lang]=$(secs "$t0" "$t1")
        if [ $lang = rust ]; then emitted_lines[$subject-$lang]=$(cat "$dir"/src/*.rs | wc -l | tr -d ' ')
        else emitted_lines[$subject-$lang]=$(cat "$dir"/src/*.cpp "$dir"/src/*.hpp | wc -l | tr -d ' '); fi
    done
    for c in "${configs[@]}"; do
        case "$c" in rust-*) lang=rust ;; cpp-*) lang=cpp ;; *) echo "bench: unknown configuration $c" >&2; exit 2 ;; esac
        dir=$out/$subject/$c
        rm -rf "$dir" && cp -R "$out/$subject/emit-$lang" "$dir"
        echo "bench: $subject: $c built from clean" >&2
        t0=$(now)
        case "$c" in
            rust-release)
                rm -rf "$out/cargo-$c" && CARGO_TARGET_DIR="$PWD/$out/cargo-$c" cargo build --offline --release -q --manifest-path "$PWD/$dir/Cargo.toml" 2> "$dir.log"
                bin=$out/cargo-$c/release/$name ;;
            cpp-*)
                opt=-${c#cpp-}
                rm -rf "runtime/cpp/build/bench$opt" "worlds/canvas/cpp/build/bench$opt"
                make -s -C "$dir" -j"$jobs" CXX="$CXX" OPT="$opt" CONFIG="bench$opt" > "$dir.log" 2>&1
                bin=$dir/$name ;;
        esac
        t1=$(now)
        [ -x "$bin" ] || { echo "bench: $subject $c built no binary at $bin (see $dir.log)" >&2; exit 1; }
        cp "$bin" "$dir/$name.bin"
        build_s[$subject-$c]=$(secs "$t0" "$t1")
        total_s[$subject-$c]=$(plus "${emit_s[$subject-$lang]}" "${build_s[$subject-$c]}")
        size_mb[$subject-$c]=$(mb "$bin")
    done
done

{
    echo "# Building Prism: results"
    echo
    echo "*Written by bench/building/run.sh on $(date '+%Y-%m-%d %H:%M %Z'); regenerate, do not edit.*"
    echo
    echo "- **Machine:** $(sysctl -n machdep.cpu.brand_string), $jobs cores, $(( $(sysctl -n hw.memsize) / 1073741824 )) GB, $(sw_vers -productName) $(sw_vers -productVersion); load $(sysctl -n vm.loadavg)"
    echo "- **prism-language:** $(git rev-parse --short HEAD)$(git diff --quiet || echo ' (with uncommitted changes)')"
    echo "- **Rust:** $(rustc --version), cargo -j$jobs; **C++:** $($CXX --version | head -1), make -j$jobs"
    echo "- **Emitting:** the installed fixed points, Rust by \`target/bootstrap/prismc\`, C++ by \`target/bootstrap/prismc-cpp\`"
    echo
    echo "From Prism source to a runnable binary, from clean, seconds. Emit is prismc's run; build"
    echo "is cargo's or make's, the runtime library and the Worlds included."
    echo
    for subject in "${subjects[@]}"; do
        echo "## $subject ($(lines_of "$subject") lines of Prism with the prelude; emitted: ${emitted_lines[$subject-rust]} lines of Rust, ${emitted_lines[$subject-cpp]} of C++)"
        echo
        echo "| Configuration | Emit | Build | Total | Binary MB |"
        echo "|---|---|---|---|---|"
        for c in "${configs[@]}"; do
            case "$c" in rust-*) lang=rust ;; *) lang=cpp ;; esac
            echo "| $c | ${emit_s[$subject-$lang]} | ${build_s[$subject-$c]} | ${total_s[$subject-$c]} | ${size_mb[$subject-$c]} |"
        done
        echo
    done
} > bench/building/RESULTS.md
echo "bench: wrote bench/building/RESULTS.md" >&2
