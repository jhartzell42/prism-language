#!/usr/bin/env bash
# Diffs prismc's front end against prism_syntax's: for every `.prism` and `.prismtest` file
# under examples/, tests/conformance/, prelude/ and prismc/ (or the files named on the
# command line), the S-expression dump of `cargo run -p prism_syntax --example dump -- FILE`
# and of `prism_zero run prismc/*.prism prismc/templates/*.prism -- FILE OUT` must be byte-equal, diagnostics
# included. `--tokens` compares the two lexers instead (`dump --tokens`), and `--fragment`
# parses each file as a run of statements (`dump --fragment`). Exits 1 and lists
# the files that differ, 0 when every file diffs clean.
#
# Written by Claude, 31 October 2026, on the claude-self-hosting branch. Subject to change.
set -u
cd "$(dirname "$0")/../.." || exit 2

mode=()
files=()
for a in "$@"; do
    case "$a" in
        --tokens) mode=(--tokens) ;;
        --fragment) mode=(--fragment) ;;
        *) files+=("$a") ;;
    esac
done
if [ ${#files[@]} -eq 0 ]; then
    while IFS= read -r f; do files+=("$f"); done < <(find examples tests/conformance prelude prismc \( -name '*.prism' -o -name '*.prismtest' \) | sort)
fi

cargo build --release -q -p prism_zero -p prism_syntax --example dump || exit 2
zero=target/release/prism_zero
dump=target/release/examples/dump

# prism_zero's evaluator recurses on the Rust stack, about 15 KB a Prism call; the parser's
# descent through the chess window library needs more than the default 8 MB
# (prismc/README.md, "Workarounds").
ulimit -s unlimited 2>/dev/null || ulimit -s 1048576 2>/dev/null || true

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

clean=0
differ=()
for f in "${files[@]}"; do
    "$dump" "${mode[@]}" "$f" > "$tmp/rust" 2>&1
    rm -f "$tmp/prismc"
    "$zero" run prismc/*.prism prismc/templates/*.prism -- "${mode[@]}" "$f" "$tmp/prismc" > "$tmp/log" 2>&1
    if [ -f "$tmp/prismc" ] && cmp -s "$tmp/rust" "$tmp/prismc"; then
        clean=$((clean + 1))
    else
        differ+=("$f")
        echo "differs: $f"
        if [ -f "$tmp/prismc" ]; then
            diff "$tmp/rust" "$tmp/prismc" | head -n 8 | sed 's/^/    /'
        else
            head -n 4 "$tmp/log" | sed 's/^/    /'
        fi
    fi
done

echo "$clean of ${#files[@]} files diff clean"
if [ ${#differ[@]} -gt 0 ]; then
    echo "files that differ:"
    printf '    %s\n' "${differ[@]}"
    exit 1
fi
exit 0
