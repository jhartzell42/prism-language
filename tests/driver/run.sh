#!/usr/bin/env bash
# The `prism` driver's World choice (`prism run --print-world`): the World prismc takes from
# the root module's `emit` row, `Gui.*` the canvas World, a row without Ui or Gui the CLI
# World, a row with `Ui.*` and no `Gui.*` refused (no World answers Ui elements alone), and
# `--world` over all of it; a program with no `@main` has no root to run.
#
#   tests/driver/run.sh
#
# prismc is the `prism` script's (`$PRISMC`, else the newest binary). Exits 1 listing the
# cases that fail.
#
# Written by Claude, 3 November 2026, on the claude-self-hosting branch. Subject to change.
set -u
cd "$(dirname "$0")/../.." || exit 2
failed=()
# expect NAME WANT ARGS...: `prism run --print-world ARGS` prints WANT and exits 0, or with
# WANT `refused` exits 2 saying why
expect() {
    local name=$1 want=$2; shift 2
    local out code
    out=$(./prism run --print-world "$@" 2>&1); code=$?
    if [ "$want" = refused ]; then
        if [ $code -eq 2 ] && [ -n "$out" ]; then echo "ok $name: refused ($(echo "$out" | tail -1))"; return; fi
    elif [ $code -eq 0 ] && [ "$(echo "$out" | tail -1)" = "$want" ]; then
        echo "ok $name: $want"; return
    fi
    failed+=("$name")
    echo "FAILED $name: wanted $want, got exit $code: $out"
}
expect chess canvas examples/chess
expect cli-root cli tests/driver/cli-root.prism
expect birds refused tests/conformance/sem-ch06-birds.prism tests/conformance/sem-ch06-birds
expect birds-as-cli cli --world cli tests/conformance/sem-ch06-birds.prism tests/conformance/sem-ch06-birds
expect counter-no-root refused tests/conformance/counter.prism
if [ ${#failed[@]} -gt 0 ]; then
    echo "failed: ${failed[*]}"
    exit 1
fi
echo "every World as asked"
