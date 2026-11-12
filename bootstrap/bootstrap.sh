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
# A compiler emits code for the runtime of its own source: the runtime's interface (runtime/rust,
# prism_rtl, and the Worlds' crates) changes together with the emitter, so the starting
# compiler's gen1 is built against the runtime that compiler was emitted for, and every later
# generation against the runtime of the tree (the copy in target/bootstrap/src). A crate is
# built against a runtime by copying it into a root laid out as the repository is, at its
# own path, so the `../../../runtime/rust` it was emitted with finds that root's.
#
# The starting prismc is, in order: $PRISMC; the installed target/bootstrap/prismc; or,
# on a fresh clone, the seed: bootstrap/prismc-rust.tar.xz, the Rust crate prismc emitted
# for itself at its last fixed point, with the runtime crates it was emitted against,
# unpacked and built with cargo. The installed compiler's runtime is kept beside it
# (target/bootstrap/prismc-runtime); $PRISMC's is $PRISMC_RUNTIME, a root holding runtime/rust
# and worlds/*/rust, else the working tree's. Nothing compiled is
# kept in the repository except that archive. An installed compiler older than the source's
# syntax cannot compile prismc/: the bootstrap then stops with one line saying so and
# naming --from-seed, the compiler's diagnostics in target/bootstrap/gen1-compile.log.
#
#   bootstrap/bootstrap.sh            # bootstrap and install
#   bootstrap/bootstrap.sh --update   # also rewrite the seed from the new fixed point
#   bootstrap/bootstrap.sh --from-seed   # start from the seed, ignoring $PRISMC and the
#                                        # installed compiler (with --update too)
#   bootstrap/bootstrap.sh --show-start  # only say which compiler it would start from
#   bootstrap/bootstrap.sh --seed-at DIR   # only unpack and build the seed in DIR
#                                          # (bootstrap/test-seed.sh)
#
# Written by Claude on the claude-self-hosting branch. Subject to change.
set -eu
cd "$(dirname "$0")/.."
export PRISM_ROOT=$PWD  # for the emitted Makefiles; cargo gets the crate by --manifest-path
ulimit -s unlimited 2>/dev/null || ulimit -s 1048576 2>/dev/null || true
update=0
seed_at=""
from_seed=0
show_start=0
while [ $# -gt 0 ]; do
    case "$1" in
        --update) update=1; shift ;;
        --from-seed) from_seed=1; shift ;;
        --show-start) show_start=1; shift ;;
        --seed-at) seed_at=${2:?--seed-at takes a directory}; shift 2 ;;
        *) echo "unknown option $1" >&2; exit 2 ;;
    esac
done
# The working directory: target/bootstrap, or PRISM_BOOTSTRAP (bootstrap/test-start.sh).
b=${PRISM_BOOTSTRAP:-target/bootstrap}
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
# copy_runtime FROM TO: the Rust target's runtime crates of the root FROM (runtime/rust,
# worlds/*/rust), with no build output, copied into the root TO
copy_runtime() {
    local d
    for d in $(cd "$1" && ls -d runtime/rust worlds/*/rust 2> /dev/null); do
        mkdir -p "$2/$d"
        tar -C "$1/$d" --exclude=./target -cf - . | tar -C "$2/$d" -xf -
    done
}
# fetch=1 on a fresh clone: every build is offline, so each runtime's crates are fetched
# before its first build (the seed's, then the tree's).
fetch=0
# build_crate DIR ROOT: cargo builds the crate in DIR against the runtime under ROOT (`.`, the
# working tree; else a copy of the crate at ROOT/DIR is built), its binary at DIR/prismc
build_crate() {
    local at=$1 log; log="$(pwd)/$1.log"
    if [ "$2" != . ]; then
        at=$2/$1
        rm -rf "$at" && mkdir -p "$at"
        cp -r "$1/src" "$1/Cargo.toml" "$at/"
    fi
    if [ $fetch = 1 ]; then cargo fetch -q --manifest-path "$PWD/$at/Cargo.toml"; fi
    CARGO_TARGET_DIR="$PWD/$b/build" cargo build --offline --release -q --manifest-path "$PWD/$at/Cargo.toml" 2> "$log"
    cp "$b/build/release/prismc" "$1/prismc"
}
# build_seed DIR: the seed unpacked into DIR, a root holding the runtime crates it carries,
# its crate moved to DIR/target/bootstrap/seed (the place of the generation it was packed
# from, so its `../../../runtime/rust` is DIR's) and built, its binary at DIR/prismc. cargo
# is given the crate by its path from the repository root, never run inside it.
build_seed() {
    [ -f "$seed" ] || { echo "no starting prismc: set PRISMC, or keep $seed" >&2; exit 2; }
    echo "seed: building prismc from $seed in $1"
    rm -rf "$1" && mkdir -p "$1/target/bootstrap/seed"
    tar -C "$1" -xJf "$seed"
    [ -f "$1/runtime/rust/Cargo.toml" ] || { echo "$seed carries no runtime/rust: it cannot be built against the runtime it was emitted for" >&2; exit 2; }
    mv "$1/src" "$1/Cargo.toml" "$1/target/bootstrap/seed/"
    # Every build here is offline; a fresh machine has no crates yet, so get them first.
    fetch=1
    build_crate "$1/target/bootstrap/seed" .
    cp "$1/target/bootstrap/seed/prismc" "$1/prismc"
}
if [ -n "$seed_at" ]; then build_seed "$seed_at"; echo "seed: built $seed_at/prismc"; exit 0; fi
# The starting compiler: $PRISMC, else the installed $b/prismc, else the seed; with
# --from-seed, the seed whatever is set or installed.
start=${PRISMC:-}
from=prismc
if [ -z "$start" ] && [ -x "$b/prismc" ]; then start=$b/prismc; from=installed; fi
if [ -z "$start" ] || [ $from_seed = 1 ]; then start=""; from=seed; fi
if [ $show_start = 1 ]; then
    case $from in
        prismc) echo "start: \$PRISMC ($start)" ;;
        installed) echo "start: the installed $start" ;;
        seed) echo "start: the seed $seed" ;;
    esac
    exit 0
fi
# The root holding the runtime the starting compiler emits for.
case $from in
    prismc) start_runtime=${PRISMC_RUNTIME:-.} ;;
    installed) start_runtime=$b/prismc-runtime; [ -d "$start_runtime" ] || start_runtime=. ;;
    seed) start_runtime=$b/seed ;;
esac
# A root named by $PRISMC_RUNTIME (absolute or not) is copied into $b/start-runtime, as the
# tree is into $b/src: the crate is built in that copy, never written into the caller's root.
start_named=$start_runtime
if [ $from = prismc ] && [ "$start_runtime" != . ]; then
    rm -rf "$b/start-runtime" && copy_runtime "$start_runtime" "$b/start-runtime"
    start_runtime=$b/start-runtime
fi
if [ $from = seed ]; then
    build_seed "$b/seed"
    start=$b/seed/prismc
fi
rm -rf "$b/gen1" "$b/gen2" "$b/gen3" "$b/src"
mkdir -p "$b/src"
cp -r prismc prelude "$b/src/"
copy_runtime . "$b/src"
src=("$b/src/prismc" --prelude "$b/src/prelude")
# compile N BINARY: prismc/ compiled by BINARY into genN
compile() {
    local t=$(date +%s) log="$b/gen$1-compile.log"
    if ! "$2" compile "${src[@]}" -o "$b/gen$1" --name prismc > "$log" 2>&1; then
        tail -20 "$log" >&2
        if [ "$1" = 1 ] && [ $from = installed ]; then
            echo "the installed $start cannot compile this tree (it is older than the source's syntax); re-run with --from-seed" >&2
        fi
        exit 1
    fi
    echo "gen$1: prismc/ compiled by $2 in $(( $(date +%s) - t ))s"
}
# build N ROOT [NAME]: genN built by cargo against the runtime under ROOT (a copy of NAME's)
build() {
    local t=$(date +%s) named=${3:-$2}
    build_crate "$b/gen$1" "$2"
    echo "gen$1: cargo built against the runtime of $( [ "$named" = . ] && echo the working tree || echo "$named") in $(( $(date +%s) - t ))s (warnings in $b/gen$1.log)"
}
same() { diff -r -q "$b/gen$1/src" "$b/gen$2/src" && diff -q "$b/gen$1/Cargo.toml" "$b/gen$2/Cargo.toml"; }
compile 1 "$start"
build 1 "$start_runtime" "$start_named"
compile 2 "$b/gen1/prismc"
fixed=1
if ! same 1 2; then
    echo "gen1 and gen2 differ: prismc/ emits other code than $start; one more generation"
    build 2 "$b/src"
    compile 3 "$b/gen2/prismc"
    if ! same 2 3; then
        echo "gen2 and gen3 differ: no fixed point" >&2
        exit 1
    fi
    fixed=2
fi
echo "fixed point: gen$fixed and gen$((fixed + 1)) are byte-equal ($(find "$b/gen$fixed/src" -name '*.rs' -exec cat {} + | wc -l) lines of Rust)"
cp "$b/gen$fixed/prismc" "$b/prismc.new" && mv "$b/prismc.new" "$b/prismc"
rm -rf "$b/prismc-runtime" && copy_runtime "$b/src" "$b/prismc-runtime"
echo "installed $b/prismc (gen$fixed), its runtime in $b/prismc-runtime"
if [ $update = 1 ]; then
    grep -q '"../../../runtime/rust"' "$b/gen$fixed/Cargo.toml" || { echo "gen$fixed was not emitted three levels under the root (PRISM_BOOTSTRAP): no seed written" >&2; exit 1; }
    rm -rf "$b/pack" && mkdir -p "$b/pack"
    cp -r "$b/gen$fixed/src" "$b/gen$fixed/Cargo.toml" "$b/pack/"
    copy_runtime "$b/src" "$b/pack"
    pack "$seed.new" "$b/pack" src Cargo.toml runtime worlds && mv "$seed.new" "$seed"
    echo "seed: wrote $seed from gen$fixed and its runtime ($(du -h "$seed" | cut -f1))"
fi
