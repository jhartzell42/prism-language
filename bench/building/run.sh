#!/usr/bin/env bash
# Building Prism: from Prism source to a runnable binary, each
# pipeline all one language. The installed Rust prismc emits Rust and cargo builds it; the
# installed C++ prismc emits C++ and make builds it, the runtime library and the Worlds
# included, from clean, with every core. For each subject and each configuration: the emit
# time (and the emitted lines a second), the build time, their total, the binary's size,
# and the rebuild time: the emitted program's own sources touched and built again, the
# runtime library and the dependencies reused.
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
. bench/lib.sh
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
        chess) echo "examples/basic-chess --name chess" ;;
        table) echo "bench/gui/prism examples/basic-chess/basic_window --name bench" ;;
        *) echo "bench: unknown subject $1 (prismc chess table)" >&2; exit 2 ;;
    esac
}
lines_of() { # subject: Prism lines, every file of its trees
    case "$1" in
        prismc) find prismc prelude -name '*.prism' -exec cat {} + | wc -l | tr -d ' ' ;;
        chess) find examples/basic-chess prelude -name '*.prism' -exec cat {} + | wc -l | tr -d ' ' ;;
        table) find bench/gui/prism examples/basic-chess/basic_window prelude -name '*.prism' -exec cat {} + | wc -l | tr -d ' ' ;;
    esac
}
now() { python3 -c 'import time; print(time.time())'; }
secs() { python3 -c "import sys; print(f'{float(sys.argv[2]) - float(sys.argv[1]):.1f}')" "$1" "$2"; }
plus() { python3 -c "import sys; print(f'{float(sys.argv[1]) + float(sys.argv[2]):.1f}')" "$1" "$2"; }
mb() { python3 -c "import os,sys; print(f'{os.path.getsize(sys.argv[1]) / 1048576:.1f}')" "$1"; }
per_sec() { python3 -c "import sys; print(f'{float(sys.argv[1]) / float(sys.argv[2]):,.0f}')" "$1" "$2"; }
# count_lines DIR EXT...: the lines of every file with those extensions under DIR, at any
# depth (the emitted modules are nested as the program's are: src/PATH/mod.rs, mod.cpp)
count_lines() {
    local dir=$1; shift
    local args=() ext
    for ext in "$@"; do [ ${#args[@]} -eq 0 ] || args+=(-o); args+=(-name "*.$ext"); done
    find "$dir" -type f \( "${args[@]}" \) -exec cat {} + | wc -l | tr -d ' '
}

declare -A emit_s build_s total_s size_mb emitted_lines emit_lps rebuild_s
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
        if [ $lang = rust ]; then emitted_lines[$subject-$lang]=$(count_lines "$dir/src" rs)
        else emitted_lines[$subject-$lang]=$(count_lines "$dir/src" cpp hpp); fi
        emit_lps[$subject-$lang]=$(per_sec "${emitted_lines[$subject-$lang]}" "${emit_s[$subject-$lang]}")
    done
    for c in "${configs[@]}"; do
        case "$c" in rust-*) lang=rust ;; cpp-*) lang=cpp ;; *) echo "bench: unknown configuration $c" >&2; exit 2 ;; esac
        dir=$out/$subject/$c
        rm -rf "$dir" && cp -R "$out/$subject/emit-$lang" "$dir"
        # build LOG: the configuration's build, cargo's or make's; the clean build has
        # removed what the rebuild reuses (the cargo target, the runtime's and the Worlds'
        # libraries) before calling it
        build() {
            case "$c" in
                rust-release)
                    CARGO_TARGET_DIR="$PWD/$out/cargo-$c" cargo build --offline --release -q --manifest-path "$PWD/$dir/Cargo.toml" 2> "$1"
                    bin=$out/cargo-$c/release/$name ;;
                cpp-*)
                    make -s -C "$dir" -j"$jobs" CXX="$CXX" OPT="-${c#cpp-}" CONFIG="bench-${c#cpp-}" > "$1" 2>&1
                    bin=$dir/$name ;;
            esac
        }
        echo "bench: $subject: $c built from clean" >&2
        case "$c" in
            rust-release) rm -rf "$out/cargo-$c" ;;
            cpp-*) rm -rf "runtime/cpp/build/bench-${c#cpp-}" "worlds/canvas/cpp/build/bench-${c#cpp-}" ;;
        esac
        t0=$(now)
        build "$dir.log"
        t1=$(now)
        [ -x "$bin" ] || { echo "bench: $subject $c built no binary at $bin (see $dir.log)" >&2; exit 1; }
        cp "$bin" "$dir/$name.bin"
        build_s[$subject-$c]=$(secs "$t0" "$t1")
        total_s[$subject-$c]=$(plus "${emit_s[$subject-$lang]}" "${build_s[$subject-$c]}")
        size_mb[$subject-$c]=$(mb "$bin")
        # The rebuild: the program's own emitted sources touched (not the runtime's), so
        # cargo or make compiles the program again over the runtime library and the
        # dependencies the clean build left.
        echo "bench: $subject: $c rebuilt with the emitted sources touched" >&2
        sleep 1  # a whole second newer than the clean build's outputs, whatever the filesystem's mtime grain
        find "$dir/src" -type f \( -name '*.rs' -o -name '*.cpp' -o -name '*.hpp' \) -exec touch {} +
        t0=$(now)
        build "$dir.rebuild.log"
        t1=$(now)
        rebuild_s[$subject-$c]=$(secs "$t0" "$t1")
    done
done

{
    echo "# Building Prism: results"
    echo
    echo "*Written by bench/building/run.sh on $(date '+%Y-%m-%d %H:%M %Z'); regenerate, do not edit.*"
    echo
    echo "- **Machine:** $(machine_line); load $(load_line)"
    echo "- **prism-language:** $(git rev-parse --short HEAD)$(git diff --quiet || echo ' (with uncommitted changes)')"
    echo "- **Rust:** $(rustc --version), cargo -j$jobs; **C++:** $($CXX --version | head -1), make -j$jobs"
    echo "- **Emitting:** the installed fixed points, Rust by \`target/bootstrap/prismc\`, C++ by \`target/bootstrap/prismc-cpp\`"
    echo
    echo "From Prism source to a runnable binary, from clean, seconds. Emit is prismc's run, and"
    echo "lines/s its emitted lines (every file under the emitted src/) over that time; build is"
    echo "cargo's or make's, the runtime library and the Worlds included. Rebuild is the same build"
    echo "run again after the emitted program's own sources are touched: the program compiled"
    echo "and linked once more over the runtime library and the dependencies the clean build"
    echo "left, which is what an edit to a Prism program costs after its first build."
    echo
    for subject in "${subjects[@]}"; do
        echo "## $subject ($(lines_of "$subject") lines of Prism with the prelude; emitted: ${emitted_lines[$subject-rust]} lines of Rust, ${emitted_lines[$subject-cpp]} of C++)"
        echo
        echo "| Configuration | Emit | lines/s | Build | Total | Rebuild | Binary MB |"
        echo "|---|---|---|---|---|---|---|"
        for c in "${configs[@]}"; do
            case "$c" in rust-*) lang=rust ;; *) lang=cpp ;; esac
            echo "| $c | ${emit_s[$subject-$lang]} | ${emit_lps[$subject-$lang]} | ${build_s[$subject-$c]} | ${total_s[$subject-$c]} | ${rebuild_s[$subject-$c]} | ${size_mb[$subject-$c]} |"
        done
        echo
    done
} > bench/building/RESULTS.md
echo "bench: wrote bench/building/RESULTS.md" >&2
