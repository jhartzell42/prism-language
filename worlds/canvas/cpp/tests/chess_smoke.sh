#!/usr/bin/env bash
# Chess compiled to C++ under the C++ canvas World on a real X server: the robot waits for
# the board, clicks the pawn on e2 (its square turns yellow), clicks e4, waits for the
# root's result "Black's turn" (PRISM_RESULTS) and for e2 to show empty, then closes the
# window as a window manager would; the run must end with 0. The squares are 64 pixels
# from the window's origin, which under Xvfb, with no window manager, is the screen's.
#
#   tests/chess_smoke.sh ROBOT      (make chess-smoke builds the robot and runs this)
#
# With no DISPLAY it runs itself again under `xvfb-run -a`. The program is compiled with
# the prism script's prismc (`$PRISMC`, else its newest binary), which must have the C++
# target, into target/canvas-cpp/chess.
#
# Written by Claude on the claude-self-hosting branch; subject to change.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../../.." && pwd)
robot=$(realpath "$1")
prog=$repo/target/canvas-cpp/chess

if [ -z "${DISPLAY:-}" ]; then
    command -v xvfb-run > /dev/null || { echo "chess_smoke: skipped, no display and no xvfb-run"; exit 0; }
    cd "$repo"
    ./prism compile --target cpp examples/chess -o target/canvas-cpp/chess
    [ -f "$prog/src/main.cpp" ] || { echo "chess_smoke: prismc (${PRISMC:-the newest}) emitted no C++; set PRISMC to one with --target cpp" >&2; exit 1; }
    make -s -C "$prog" CXX=g++ -j"$(getconf _NPROCESSORS_ONLN)"
    exec timeout 120 xvfb-run -a -s "-screen 0 1024x768x24" "$here/chess_smoke.sh" "$robot"
fi

cd "$repo"
log=$(mktemp)
trap 'rm -f "$log"' EXIT
PRISM_RESULTS=1 "$prog/chess" 2> "$log" &
pid=$!
fail() { echo "chess_smoke: $*" >&2; cat "$log" >&2; kill "$pid" 2>/dev/null || true; exit 1; }

"$robot" wait 96 160 b3b3b3 30 || fail "the board (b6, grey) never showed"
"$robot" click 280 400                                      # the pawn on e2
"$robot" wait 258 386 f0dc5a || fail "e2 never turned yellow"
"$robot" click 280 280                                      # e4
for _ in $(seq 100); do grep -q "Black's turn" "$log" && break; sleep 0.1; done
grep -q "Black's turn" "$log" || fail "the root never said Black's turn"
"$robot" wait 258 386 ffffff || fail "e2 is not empty after the move"
"$robot" close Chess || fail "no window named Chess"
code=0
wait "$pid" || code=$?
[ "$code" = 0 ] || fail "the run ended with $code"
echo "chess_smoke: e2-e4 played under the C++ canvas World; exit 0"
