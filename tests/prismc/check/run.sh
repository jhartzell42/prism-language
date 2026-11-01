#!/usr/bin/env bash
# The checker's tests (prismc/README.md, Checker): every corpus program checks clean, or
# gives exactly the diagnostics its `pos/NAME.expected` records (the programs the README
# lists as wrong by the books); every negative program under `neg/` gives exactly its
# `.expected` diagnostic. A `# args: ...` first line passes options (`--world Ui`).
# Then the typed output (prismc/typed.prism): `--typed` over chess and the conformance
# counter annotates every expression with a type, none of them `unknown`.
# Exits 1 listing the mismatches, 0 when everything matches. `--update` rewrites the
# `.expected` files from what the checker says now, for review.
#
# Written by Claude, 31 October 2026, on the claude-self-hosting branch. Subject to change.
set -u
cd "$(dirname "$0")/../../.." || exit 2
here=tests/prismc/check
update=0
only=()
for a in "$@"; do
    case "$a" in
        --update) update=1 ;;
        *) only+=("$a") ;;
    esac
done

cargo build --release -q -p prism_zero || exit 2
zero=target/release/prism_zero
# compiler zero recurses on the Rust stack (prismc/README.md, Workarounds SK)
ulimit -s unlimited 2>/dev/null || ulimit -s 1048576 2>/dev/null || true
prismc=(prismc/*.prism prismc/templates/*.prism)

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# check NAME FILES...: the checker's report for a program, in $tmp/NAME.out
check() {
    local name=$1; shift
    rm -f "$tmp/$name.out"
    "$zero" run "${prismc[@]}" -- --check "$@" --out "$tmp/$name.out" > "$tmp/$name.err" 2>&1
    if [ ! -f "$tmp/$name.out" ]; then
        { echo "(the checker wrote nothing)"; tail -3 "$tmp/$name.err"; } > "$tmp/$name.out"
    fi
}

failed=()
# compare NAME EXPECTED-FILE: the report against the expected text ("no errors" when the
# file does not exist)
compare() {
    local name=$1 expected=$2
    if [ "$update" = 1 ]; then
        if [ "$(cat "$tmp/$name.out")" = "no errors" ] && [ ! -f "$expected" ] && [ "${expected#*/neg/}" = "$expected" ]; then :; else cp "$tmp/$name.out" "$expected"; fi
        return
    fi
    if [ -f "$expected" ]; then
        diff -q "$expected" "$tmp/$name.out" > /dev/null || failed+=("$name")
    else
        [ "$(cat "$tmp/$name.out")" = "no errors" ] || failed+=("$name")
    fi
}

wanted() { [ ${#only[@]} -eq 0 ] && return 0; for o in "${only[@]}"; do [ "$o" = "$1" ] && return 0; done; return 1; }

# The positive set: every program of the corpus, by its files.
declare -A programs
for f in tests/conformance/*.prism; do programs[$(basename "$f" .prism)]="$f"; done
programs[sem-ch06-birds]="tests/conformance/sem-ch06-birds.prism tests/conformance/sem-ch06-birds/birds.prism"
programs[counter]=tests/conformance/counter.prism
programs[half-counter]=tests/conformance/half-counter.prism
programs[todos]=tests/conformance/todos.prism
programs[chess]="examples/chess/board.prism examples/chess/rules.prism examples/chess/window.prism"
programs[prelude]="$(ls prelude/*.prism | tr '\n' ' ')"
programs[prismc]="$(ls prismc/*.prism prismc/templates/*.prism | tr '\n' ' ')"

for name in $(printf '%s\n' "${!programs[@]}" | sort); do
    wanted "$name" || continue
    # shellcheck disable=SC2086
    check "$name" ${programs[$name]}
    compare "$name" "$here/pos/$name.expected"
    echo "pos $name: $(tail -1 "$tmp/$name.out")"
done

# The negative set.
for f in "$here"/neg/*.prism; do
    name=$(basename "$f" .prism)
    wanted "$name" || continue
    args=()
    first=$(head -1 "$f")
    case "$first" in
        "# args: "*) read -r -a args <<< "${first#\# args: }" ;;
    esac
    check "neg-$name" "${args[@]}" "$f"
    compare "neg-$name" "$here/neg/$name.expected"
    echo "neg $name: $(head -1 "$tmp/neg-$name.out")"
done

# The typed output: complete (no expression without a known type) for these programs.
declare -A typed
typed[typed-chess]="examples/chess/board.prism examples/chess/rules.prism examples/chess/window.prism"
typed[typed-counter]=tests/conformance/conf-ch01-counter.prism
for name in $(printf '%s\n' "${!typed[@]}" | sort); do
    wanted "$name" || continue
    rm -f "$tmp/$name.typed"
    # shellcheck disable=SC2086
    "$zero" run "${prismc[@]}" -- --typed ${typed[$name]} --out "$tmp/$name.typed" > "$tmp/$name.err" 2>&1
    if [ ! -s "$tmp/$name.typed" ]; then
        failed+=("$name"); echo "typed $name: (nothing written)"
    else
        unknown=$(grep -c unknown "$tmp/$name.typed")
        exprs=$(grep -c "^  (expr " "$tmp/$name.typed")
        [ "$unknown" = 0 ] || failed+=("$name")
        echo "typed $name: $exprs expressions, $unknown unknown"
    fi
done

if [ ${#failed[@]} -gt 0 ]; then
    echo "mismatches: ${failed[*]}"
    exit 1
fi
echo "all match"
