#!/usr/bin/env bash
# The bootstrap and its fixed point (docs/architecture.md, "The goal"): the kept prismc
# compiles prismc/ into the crate target/bootstrap/gen1, cargo builds it, and gen1's
# binary compiles prismc/ again into target/bootstrap/gen2. When the two crates are
# byte-equal, that is the fixed point (the compiler compiles itself to the crate it was
# built from) and gen1's binary is installed as target/bootstrap/prismc, which the
# `prism` script then picks as the newest. When prismc/ changes what prismc emits (a
# template, a lowering), gen1 is the new compiler still emitting the kept binary's
# output, so they differ: gen2 is built and compiles gen3, which must equal gen2, and
# gen2's binary is installed. Every compile reads one copy of prismc/ and prelude/ taken
# at the start (target/bootstrap/src), so an edit made while the run goes on reaches none.
#
#   tests/bootstrap/bootstrap.sh     # the kept binary: $PRISMC, else the newest target/keep/prismc-*
#
# Written by Claude, 2 November 2026, on the claude-self-hosting branch. Subject to change.
set -eu
cd "$(dirname "$0")/../.."
ulimit -s unlimited 2>/dev/null || ulimit -s 1048576 2>/dev/null || true
kept=${PRISMC:-$(ls -t target/keep/prismc-* 2>/dev/null | grep -v old-chain | head -1)}
[ -x "$kept" ] || { echo "no kept prismc (target/keep/prismc-*, or set PRISMC)" >&2; exit 2; }
b=target/bootstrap
rm -rf "$b/gen1" "$b/gen2" "$b/src"
mkdir -p "$b/src"
cp -r prismc prelude "$b/src/"
src=("$b/src/prismc" --prelude "$b/src/prelude")
# compile N BINARY: prismc/ compiled by BINARY into genN
compile() {
    local start=$(date +%s)
    "$2" compile "${src[@]}" -o "$b/gen$1" --name prismc
    echo "gen$1: prismc/ compiled by $2 in $(( $(date +%s) - start ))s"
}
# build N: genN built by cargo, its binary at genN/prismc
build() {
    local start=$(date +%s)
    (cd "$b/gen$1" && CARGO_TARGET_DIR=../build cargo build --offline --release -q 2> ../build-gen$1.log)
    cp "$b/build/release/prismc" "$b/gen$1/prismc"
    echo "gen$1: cargo built in $(( $(date +%s) - start ))s (warnings in $b/build-gen$1.log)"
}
same() { diff -r -q "$b/gen$1/src" "$b/gen$2/src" && diff -q "$b/gen$1/Cargo.toml" "$b/gen$2/Cargo.toml"; }
rm -rf "$b/gen3"
compile 1 "$kept"
build 1
compile 2 "$b/gen1/prismc"
fixed=1
if ! same 1 2; then
    echo "gen1 and gen2 differ: prismc/ emits other code than $kept; one more generation"
    build 2
    compile 3 "$b/gen2/prismc"
    if ! same 2 3; then
        echo "gen2 and gen3 differ: no fixed point" >&2
        exit 1
    fi
    fixed=2
fi
echo "fixed point: gen$fixed and gen$((fixed + 1)) are byte-equal ($(cat "$b/gen$fixed"/src/*.rs | wc -l) lines of Rust)"
cp "$b/gen$fixed/prismc" "$b/prismc"
echo "installed $b/prismc (gen$fixed)"
