#!/usr/bin/env bash
# The conformance programs the typed back end compiles end to end, each compiled by
# prismc, built, and its `.prismtest` run under the compiled Test World, the transcript
# byte-equal to its `.expected` (the binary's `test`, which compares as prism_rtl's
# TestWorld::verify). Today: the counter, the half counter, Normalize and the walk. The
# first three's `.expected` files are the ones prism_rtl's hand-written tests
# (prism_runtime_lib/tests/counter.rs, normalize.rs) verify too, so those tests run
# first: compiled and hand-written code meet the same bytes.
#
#   tests/compiled/run.sh [NAMES...]          # the passing set, or any programs named
#   PRISMC=zero tests/compiled/run.sh         # compiler zero running prismc's sources
#
# PRISMC is a prismc binary; by default the newest of target/stage{3,2,1}/prismc
# (tests/bootstrap/), else compiler zero. Exits 1 listing the programs that fail, and at
# which step.
#
# Written by Claude, 1 November 2026, on the claude-self-hosting branch. Subject to change.
set -u
cd "$(dirname "$0")/../.." || exit 2
ulimit -s unlimited 2>/dev/null || ulimit -s 1048576 2>/dev/null || true
prismc=${PRISMC:-}
if [ -z "$prismc" ]; then
    prismc=zero
    for stage in 3 2 1; do
        if [ -x target/stage$stage/prismc ]; then prismc=target/stage$stage/prismc; break; fi
    done
fi
[ "$prismc" = zero ] && { cargo build --release -q -p prism_zero || exit 2; }
# PRISMC_SRC: where compiler zero reads prismc's sources (default prismc/).
src=${PRISMC_SRC:-prismc}
compile() {
    if [ "$prismc" = zero ]; then
        target/release/prism_zero run "$src"/*.prism "$src"/templates/*.prism -- compile "$@"
    else
        "$prismc" compile "$@"
    fi
}
# The programs that pass; NAMES on the command line may name any other.
passing=(counter half-counter sem-ch07-normalize sem-ch07-walk be-let-fn-pair be-unbounded-value be-row-local be-template-attrs be-unbounded-lambda be-error-try be-nat-literal be-inference be-unbounded-prelude be-local-names be-tuple-show be-function-values)
only=("$@")
[ ${#only[@]} -eq 0 ] && only=("${passing[@]}")
wanted() { [ ${#only[@]} -eq 0 ] && return 0; for o in "${only[@]}"; do [ "$o" = "$1" ] && return 0; done; return 1; }
mkdir -p target/compiled
failed=()
echo "prismc: $prismc"
if [ $# -eq 0 ]; then
    cargo test -q -p prism_rtl --test counter --test normalize > target/compiled/hand-written.log 2>&1 \
        && echo "ok hand-written counter and Normalize (prism_rtl)" || { failed+=("hand-written"); echo "FAILED hand-written prism_rtl tests (target/compiled/hand-written.log)"; }
fi
run() { # name, script, files...
    local name=$1 script=$2; shift 2
    wanted "$name" || return 0
    local out=target/compiled/$name bin=t_${name//-/_}
    local start=$(date +%s)
    rm -rf "$out"
    if ! compile "$@" "$script" -o "$out" --name "$bin" > "$out.log" 2>&1 || [ ! -f "$out/Cargo.toml" ]; then
        failed+=("$name (compile)"); echo "FAILED $name: compile ($out.log)"; return
    fi
    local mid=$(date +%s)
    if ! (cd "$out" && CARGO_TARGET_DIR=../../compiled-build cargo build --offline -q) >> "$out.log" 2>&1; then
        failed+=("$name (cargo)"); echo "FAILED $name: cargo ($out.log)"; return
    fi
    if ! target/compiled-build/debug/"$bin" test "$script" >> "$out.log" 2>&1; then
        failed+=("$name (transcript)"); echo "FAILED $name: transcript ($out.log)"; return
    fi
    echo "ok $name (prismc $((mid - start))s, cargo $(( $(date +%s) - mid ))s)"
}
for script in tests/conformance/*.prismtest; do
    base=${script%.prismtest}
    files=("$base.prism")
    if [ -d "$base" ]; then files+=("$base"/*.prism); fi
    run "$(basename "$base")" "$script" "${files[@]}"
done
for script in examples/*.prismtest; do
    run "$(basename "${script%.prismtest}")" "$script" "${script%.prismtest}.prism"
done
chess=(examples/chess/board.prism examples/chess/rules.prism examples/chess/window.prism)
run chess-rules examples/chess/rules.prismtest "${chess[@]}"
run chess-board examples/chess/board.prismtest "${chess[@]}"
if [ ${#failed[@]} -gt 0 ]; then
    printf 'failed: %s\n' "${failed[@]}"
    exit 1
fi
echo "every compiled transcript matches"
