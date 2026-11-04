#!/usr/bin/env bash
# The bootstrap and its fixed point (docs/architecture.md, "The goal").
#
# A starting prismc compiles prismc/ into the crate target/bootstrap/gen1, cargo builds it,
# and gen1's binary compiles prismc/ again into gen2. When the two crates are byte-equal,
# that is the fixed point (the compiler compiles itself to the crate it was built from)
# and gen1's binary is installed as target/bootstrap/prismc, which `./prism` uses. When
# prismc/ changes what prismc emits, gen1 is the new compiler still emitting the starting
# binary's output, so they differ: gen2 is built and compiles gen3, which must equal gen2,
# and gen2's binary is installed. Every compile reads one copy of prismc/ and prelude/
# taken at the start (target/bootstrap/src), so an edit made during the run reaches none.
#
# The starting prismc is, in order: $PRISMC; the installed target/bootstrap/prismc; or,
# on a fresh clone, the seed: bootstrap/prismc-rust.tar.xz, the Rust crate prismc emitted
# for itself at its last fixed point, unpacked and built with cargo. Nothing compiled is
# kept in the repository except that archive.
#
#   bootstrap/bootstrap.sh            # bootstrap and install
#   bootstrap/bootstrap.sh --update   # also rewrite the seed from the new fixed point
#
# Written by Claude on the claude-self-hosting branch. Subject to change.
set -eu
cd "$(dirname "$0")/.."
ulimit -s unlimited 2>/dev/null || ulimit -s 1048576 2>/dev/null || true
update=0
[ "${1:-}" = "--update" ] && update=1
b=target/bootstrap
seed=bootstrap/prismc-rust.tar.xz
mkdir -p "$b"
# build_crate DIR: cargo builds the crate in DIR, its binary at DIR/prismc
build_crate() {
    local log; log="$(pwd)/$1.log"
    (cd "$1" && CARGO_TARGET_DIR="$(cd .. && pwd)/build" cargo build --offline --release -q 2> "$log")
    cp "$b/build/release/prismc" "$1/prismc"
}
start=${PRISMC:-}
if [ -z "$start" ] && [ -x "$b/prismc" ]; then start=$b/prismc; fi
if [ -z "$start" ]; then
    [ -f "$seed" ] || { echo "no starting prismc: set PRISMC, or keep $seed" >&2; exit 2; }
    echo "seed: building prismc from $seed"
    rm -rf "$b/seed" && mkdir -p "$b/seed"
    tar -C "$b/seed" -xJf "$seed"
    build_crate "$b/seed"
    start=$b/seed/prismc
fi
rm -rf "$b/gen1" "$b/gen2" "$b/gen3" "$b/src"
mkdir -p "$b/src"
cp -r prismc prelude "$b/src/"
src=("$b/src/prismc" --prelude "$b/src/prelude")
# compile N BINARY: prismc/ compiled by BINARY into genN
compile() {
    local t=$(date +%s)
    "$2" compile "${src[@]}" -o "$b/gen$1" --name prismc
    echo "gen$1: prismc/ compiled by $2 in $(( $(date +%s) - t ))s"
}
# build N: genN built by cargo
build() {
    local t=$(date +%s)
    build_crate "$b/gen$1"
    echo "gen$1: cargo built in $(( $(date +%s) - t ))s (warnings in $b/gen$1.log)"
}
same() { diff -r -q "$b/gen$1/src" "$b/gen$2/src" && diff -q "$b/gen$1/Cargo.toml" "$b/gen$2/Cargo.toml"; }
compile 1 "$start"
build 1
compile 2 "$b/gen1/prismc"
fixed=1
if ! same 1 2; then
    echo "gen1 and gen2 differ: prismc/ emits other code than $start; one more generation"
    build 2
    compile 3 "$b/gen2/prismc"
    if ! same 2 3; then
        echo "gen2 and gen3 differ: no fixed point" >&2
        exit 1
    fi
    fixed=2
fi
echo "fixed point: gen$fixed and gen$((fixed + 1)) are byte-equal ($(cat "$b/gen$fixed"/src/*.rs | wc -l) lines of Rust)"
cp "$b/gen$fixed/prismc" "$b/prismc.new" && mv "$b/prismc.new" "$b/prismc"
echo "installed $b/prismc (gen$fixed)"
if [ $update = 1 ]; then
    tar -C "$b/gen$fixed" -cJf "$seed.new" src Cargo.toml && mv "$seed.new" "$seed"
    echo "seed: wrote $seed from gen$fixed ($(du -h "$seed" | cut -f1))"
fi
