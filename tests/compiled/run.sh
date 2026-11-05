#!/usr/bin/env bash
# Every conformance program, and chess, through `./prism test`: compiled by prismc, built,
# each test module run under the compiled Test World, its transcript byte-equal to the
# `.expected` of its name. Every program is a tree (claude/modules-style-pass.md): all of
# them, every time: each tests/conformance/NAME/ holding an app.prism; examples/chess/rules
# alone (its test module) and examples/chess (the board's and the rules'); the prelude's own
# checks (tests/prelude/maps); and prismc's lexer, parser and dialect checks (the test
# modules of prismc/syntax/lexer, prismc/syntax/parser and prismc/check/regex).
#
#   tests/compiled/run.sh [NAMES...]     # every program, or only those named (chess too)
#   tests/compiled/run.sh --target cpp [NAMES...]   # the same through the C++ target
#
# prismc is the `prism` script's (`$PRISMC`, else the newest binary). Logs go to
# target/compiled/NAME.log (target/compiled-cpp/ for C++). Exits 1 listing the programs
# that fail, and at which step. Under C++ chess is its test modules only (the Test World).
#
# Written by Claude, 2 November 2026, on the claude-self-hosting branch. Subject to change.
set -u
cd "$(dirname "$0")/../.." || exit 2
target=rust
if [ "${1:-}" = "--target" ]; then target=$2; shift 2; fi
only=("$@")
wanted() { [ ${#only[@]} -eq 0 ] && return 0; for o in "${only[@]}"; do [ "$o" = "$1" ] && return 0; done; return 1; }
logs=target/compiled
[ "$target" = cpp ] && logs=target/compiled-cpp
mkdir -p "$logs"
failed=()
run() { # name, sources...
    local name=$1; shift
    wanted "$name" || return 0
    local log=$logs/$name.log start=$(date +%s)
    if ./prism test --target "$target" "$@" > "$log" 2>&1; then
        echo "ok $name ($(( $(date +%s) - start ))s)"
        return
    fi
    local step
    if grep -q '^FAILED ' "$log"; then
        step="transcript: $(grep '^FAILED ' "$log" | sed 's/^FAILED \([^:]*\):.*/\1/' | tr '\n' ' ')"
    elif grep -q '^ok ' "$log"; then step=transcript
    elif grep -q 'the program gave up\|Killed\|stack overflow' "$log"; then step=compile
    elif [ -f "target/prism/${name//-/_}/Cargo.toml" ] && [ "$target" = rust ]; then step=cargo
    elif [ -f "target/prism-cpp/${name//-/_}/Makefile" ] && [ "$target" = cpp ]; then step=c++
    else step=compile
    fi
    failed+=("$name ($step)")
    echo "FAILED $name: $step($log)"
}
# every program, a tree (claude/modules-style-pass.md): a directory with an app.prism, each
# test module's transcript its NAME.expected
for dir in tests/conformance/*/; do
    [ -f "$dir/app.prism" ] && run "$(basename "$dir")" "${dir%/}"
done
run chess-rules examples/chess/rules
# the prelude's own checks (tests/prelude/)
run prelude-maps tests/prelude/maps
run chess examples/chess
# prismc's own front end, checked through its modules' test modules: a module directory
# of prismc's tree, with the modules it reaches by `use`
run lexer prismc/syntax/lexer
run parser prismc/syntax/parser
run regex prismc/check/regex
if [ ${#failed[@]} -gt 0 ]; then
    echo "failed (${#failed[@]}):"
    printf '  %s\n' "${failed[@]}"
    exit 1
fi
echo "every compiled transcript matches"
