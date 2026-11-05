#!/usr/bin/env bash
# The checker's tests (prismc/README.md, Checker), through a native prismc's `--check`:
# every corpus program checks clean, or gives exactly the diagnostics its
# `pos/NAME.expected` records (the programs the README lists as wrong by the books), and so
# does every program of its own under `pos/`; every
# negative program under `neg/` gives exactly its `.expected` diagnostic. A directory under
# `pos/` or `neg/` is one program, a tree (docs/modules-style-pass.md), its `.expected`
# beside it; every program is checked with `--tests`, its test modules read. A `# args: ...`
# first line passes options (`--world Ui`). Then the typed output (prismc/typed.prism):
# `--typed` over chess and the conformance counter annotates every expression with a type,
# none of them `unknown`; a `pos/NAME.colors` holds every `(app` and `(fn` line of
# `pos/NAME.prism`'s typed output, the colors it resolves. Exits 1 listing the mismatches,
# 0 when everything matches.
# `--update` rewrites the `.expected` files from what the checker says now, for review.
#
# prismc is `$PRISMC`, or the newest of target/bootstrap/prismc and target/keep/prismc-*,
# as the `prism` script picks it.
#
# Written by Claude, 2 November 2026, on the claude-self-hosting branch. Subject to change.
set -u
cd "$(dirname "$0")/../.." || exit 2
here=tests/checker
update=0
only=()
for a in "$@"; do
    case "$a" in
        --update) update=1 ;;
        *) only+=("$a") ;;
    esac
done

prismc=${PRISMC:-$(ls -t target/bootstrap/prismc target/keep/prismc-* 2>/dev/null | grep -v old-chain | head -1)}
[ -x "$prismc" ] || { echo "no prismc binary (target/keep/prismc-*, or make bootstrap)" >&2; exit 2; }
echo "prismc: $prismc"
ulimit -s unlimited 2>/dev/null || ulimit -s 1048576 2>/dev/null || true

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# check NAME FILES...: the checker's report for a program, in $tmp/NAME.out
check() {
    local name=$1; shift
    rm -f "$tmp/$name.out"
    "$prismc" --check --tests "$@" --out "$tmp/$name.out" > "$tmp/$name.err" 2>&1
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

# The positive set: every program of the corpus, each a tree (docs/modules-style-pass.md).
declare -A programs
for d in tests/conformance/*/; do
    [ -f "$d/app.prism" ] && programs[$(basename "$d")]="${d%/}"
done
programs[chess]=examples/chess
programs[prelude]=prelude
programs[prismc]=prismc
for f in "$here"/pos/*.prism; do [ -e "$f" ] && programs[$(basename "$f" .prism)]="$f"; done
# a directory under pos/ is one program, a tree (docs/modules-style-pass.md)
for d in "$here"/pos/*/; do [ -d "$d" ] && programs[$(basename "$d")]="${d%/}"; done

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

# The negative trees: a directory under neg/ is one program, its `.expected` beside it.
for d in "$here"/neg/*/; do
    [ -d "$d" ] || continue
    name=$(basename "$d")
    wanted "$name" || continue
    check "neg-$name" "${d%/}"
    compare "neg-$name" "$here/neg/$name.expected"
    echo "neg $name: $(head -1 "$tmp/neg-$name.out")"
done

# The typed output: complete (no expression without a known type) for these programs.
declare -A typed
typed[typed-chess]=examples/chess
typed[typed-counter]=tests/conformance/conf-ch01-counter
for name in $(printf '%s\n' "${!typed[@]}" | sort); do
    wanted "$name" || continue
    rm -f "$tmp/$name.typed"
    # shellcheck disable=SC2086
    "$prismc" --typed ${typed[$name]} --out "$tmp/$name.typed" > "$tmp/$name.err" 2>&1
    if [ ! -s "$tmp/$name.typed" ]; then
        failed+=("$name"); echo "typed $name: (nothing written)"
    else
        unknown=$(grep -c unknown "$tmp/$name.typed")
        exprs=$(grep -c "^  (expr " "$tmp/$name.typed")
        [ "$unknown" = 0 ] || failed+=("$name")
        echo "typed $name: $exprs expressions, $unknown unknown"
    fi
done

# The colors: every application's and function's, as the typed output resolves them.
for f in "$here"/pos/*.colors; do
    [ -e "$f" ] || continue
    name=$(basename "$f" .colors)
    wanted "$name" || continue
    "$prismc" --typed "$here/pos/$name.prism" --out "$tmp/$name.typed" > "$tmp/$name.err" 2>&1
    grep -E '^  \((app|fn) ' "$tmp/$name.typed" > "$tmp/$name.colors" 2>/dev/null
    if [ "$update" = 1 ]; then cp "$tmp/$name.colors" "$f"
    else diff -q "$f" "$tmp/$name.colors" > /dev/null || failed+=("colors-$name")
    fi
    echo "colors $name: $(wc -l < "$tmp/$name.colors") lines"
done

if [ ${#failed[@]} -gt 0 ]; then
    echo "mismatches: ${failed[*]}"
    exit 1
fi
echo "all match"
