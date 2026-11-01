#!/usr/bin/env bash
# Compiles programs with a prismc binary and runs their test scripts under the compiled
# Test World, each transcript compared byte for byte with its `.expected` (the runtime
# compares, as `prism_zero test` does): the conformance transcripts in tests/conformance/
# (a directory of the script's name holds more modules of its program) and the examples.
#
#   tests/bootstrap/compiled-tests.sh [PRISMC]
#
# PRISMC is a compiled prismc (default target/stage1/prismc); `zero` runs prismc under
# compiler zero instead. Exits 1 listing the programs that fail.
#
# Written by Claude, 31 October 2026, on the claude-self-hosting branch. Subject to change.
set -u
cd "$(dirname "$0")/../.."
ulimit -s unlimited 2>/dev/null || ulimit -s 1048576 2>/dev/null || true
prismc=${1:-target/stage1/prismc}
compile() {
    if [ "$prismc" = zero ]; then
        target/release/prism_zero run prismc/*.prism prismc/templates/*.prism -- compile "$@"
    else
        "$prismc" -- compile "$@"
    fi
}
failed=()
run() { # name, script, files...
    local name=$1 script=$2; shift 2
    local out=target/compiled-tests/$name
    rm -rf "$out"
    if ! compile "$@" "$script" -o "$out" --name "t_${name//-/_}" > "$out.log" 2>&1; then
        failed+=("$name (compile)"); return
    fi
    if ! (cd "$out" && CARGO_TARGET_DIR=../../compiled-build cargo build -q) >> "$out.log" 2>&1; then
        failed+=("$name (cargo)"); return
    fi
    if ! target/compiled-build/debug/"t_${name//-/_}" --world test "$script" > "$out.transcript" 2>> "$out.log"; then
        failed+=("$name (transcript)"); return
    fi
    echo "ok $name"
}
mkdir -p target/compiled-tests
for script in tests/conformance/*.prismtest; do
    base=${script%.prismtest}
    files=("$base.prism")
    if [ -d "$base" ]; then files+=("$base"/*.prism); fi
    run "$(basename "$base")" "$script" "${files[@]}"
done
run counter examples/counter.prismtest examples/counter.prism
run half-counter examples/half-counter.prismtest examples/half-counter.prism
run todos examples/todos.prismtest examples/todos.prism
chess=(examples/chess/board.prism examples/chess/rules.prism examples/chess/window.prism)
run chess-rules examples/chess/rules.prismtest "${chess[@]}"
run chess-board examples/chess/board.prismtest "${chess[@]}"
if [ ${#failed[@]} -gt 0 ]; then
    printf 'failed: %s\n' "${failed[@]}"
    exit 1
fi
echo "every compiled transcript matches"
