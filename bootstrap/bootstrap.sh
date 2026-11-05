#!/usr/bin/env bash
# The bootstrap and its fixed point (claude/architecture.md, "The goal").
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
#   bootstrap/bootstrap.sh --seed-at DIR   # only unpack and build the seed in DIR, three
#                                          # levels under the root (bootstrap/test-seed.sh)
#
# Written by Claude on the claude-self-hosting branch. Subject to change.
set -eu
cd "$(dirname "$0")/.."
export PRISM_ROOT=$PWD  # for the emitted Makefiles; cargo gets the crate by --manifest-path
ulimit -s unlimited 2>/dev/null || ulimit -s 1048576 2>/dev/null || true
update=0
seed_at=""
case "${1:-}" in
    --update) update=1 ;;
    --seed-at) seed_at=${2:?--seed-at takes a directory three levels under the repository root} ;;
esac
b=target/bootstrap
seed=bootstrap/prismc-rust.tar.xz
mkdir -p "$b"
# pack ARCHIVE DIR FILES...: FILES of DIR as an xz tar with no macOS extended attributes,
# which a Linux tar warns about once per file: COPYFILE_DISABLE stops bsdtar's AppleDouble
# entries, and --no-xattrs, where the tar takes it, its LIBARCHIVE.xattr headers.
pack() {
    local out=$1 dir=$2; shift 2
    local flags=()
    if tar --no-xattrs -cf /dev/null -T /dev/null 2> /dev/null; then flags=(--no-xattrs); fi
    COPYFILE_DISABLE=1 tar ${flags[@]+"${flags[@]}"} -C "$dir" -cJf "$out" "$@"
}
# build_crate DIR: cargo builds the crate in DIR, its binary at DIR/prismc
build_crate() {
    local log; log="$(pwd)/$1.log"
    CARGO_TARGET_DIR="$PWD/$b/build" cargo build --offline --release -q --manifest-path "$PWD/$1/Cargo.toml" 2> "$log"
    cp "$b/build/release/prismc" "$1/prismc"
}
# build_seed DIR: the seed unpacked into DIR and built, its binary at DIR/prismc. cargo
# is given the crate by its path from the repository root, never run inside DIR: the
# crate's `../../../runtime/rust` would then resolve from DIR's physical place, which under
# a target/ that is a symlink out of the repository is not in it.
build_seed() {
    [ -f "$seed" ] || { echo "no starting prismc: set PRISMC, or keep $seed" >&2; exit 2; }
    echo "seed: building prismc from $seed in $1"
    rm -rf "$1" && mkdir -p "$1"
    tar -C "$1" -xJf "$seed"
    # Every build here is offline; a fresh machine has no crates yet, so get them first.
    cargo fetch -q --manifest-path "$PWD/$1/Cargo.toml"
    build_crate "$1"
}
if [ -n "$seed_at" ]; then build_seed "$seed_at"; echo "seed: built $seed_at/prismc"; exit 0; fi
start=${PRISMC:-}
if [ -z "$start" ] && [ -x "$b/prismc" ]; then start=$b/prismc; fi
if [ -z "$start" ]; then
    build_seed "$b/seed"
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
echo "fixed point: gen$fixed and gen$((fixed + 1)) are byte-equal ($(find "$b/gen$fixed/src" -name '*.rs' -exec cat {} + | wc -l) lines of Rust)"
cp "$b/gen$fixed/prismc" "$b/prismc.new" && mv "$b/prismc.new" "$b/prismc"
echo "installed $b/prismc (gen$fixed)"
if [ $update = 1 ]; then
    pack "$seed.new" "$b/gen$fixed" src Cargo.toml && mv "$seed.new" "$seed"
    echo "seed: wrote $seed from gen$fixed ($(du -h "$seed" | cut -f1))"
fi
