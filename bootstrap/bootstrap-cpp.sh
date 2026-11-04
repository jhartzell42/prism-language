#!/usr/bin/env bash
# The C++ bootstrap and its fixed point (bootstrap/README.md, "The C++ target").
#
# A starting prismc compiles prismc/ to C++ (`--target cpp`) into target/bootstrap/cpp-gen1,
# make builds it with $CXX (g++ by default), and gen1's binary compiles prismc/ to C++ again
# into cpp-gen2. When the two programs are byte-equal, that is the C++ fixed point: the
# compiler compiled to C++ compiles itself to the C++ it was built from. When prismc/
# changes what prismc emits, gen1 still emits the starting binary's output: gen2 is built
# and compiles gen3, which must equal gen2. The C++ binary at the fixed point then compiles
# prismc/ to Rust, which must be byte-equal to what the Rust-built prismc
# (target/bootstrap/prismc, or $RUST_PRISMC) emits: one model, two printers, the same answer from either
# compiler. The binary is installed as target/bootstrap/prismc-cpp. Every compile reads one
# copy of prismc/ and prelude/ taken at the start (target/bootstrap/cpp-src).
#
# The starting prismc is, in order: $PRISMC; target/bootstrap/prismc (the Rust bootstrap's);
# or, on a fresh clone, the seed bootstrap/prismc-cpp.tar.xz, the C++ prismc emitted for
# itself at its last fixed point, unpacked and built with make.
#
#   bootstrap/bootstrap-cpp.sh            # bootstrap, check against Rust, install
#   bootstrap/bootstrap-cpp.sh --update   # also rewrite the seed from the new fixed point
#
# Written by Claude, 4 November 2026, on the claude-self-hosting branch. Subject to change.
set -eu
cd "$(dirname "$0")/.."
ulimit -s unlimited 2>/dev/null || ulimit -s 1048576 2>/dev/null || true
update=0
[ "${1:-}" = "--update" ] && update=1
b=target/bootstrap
seed=bootstrap/prismc-cpp.tar.xz
CXX=${CXX:-g++}
OPT=${OPT:--O1}
mkdir -p "$b"
# build_cpp DIR: make builds the program in DIR, its binary at DIR/prismc (one job: the
# largest file needs most of the memory a small machine has)
build_cpp() {
    make -s -C "$1" -j"${JOBS:-1}" CXX="$CXX" OPT="$OPT" > "$1.log" 2>&1 || { tail -30 "$1.log" >&2; exit 1; }
}
start=${PRISMC:-}
if [ -z "$start" ] && [ -x "$b/prismc" ]; then start=$b/prismc; fi
if [ -z "$start" ]; then
    [ -f "$seed" ] || { echo "no starting prismc: set PRISMC, or keep $seed" >&2; exit 2; }
    echo "seed: building prismc from $seed"
    rm -rf "$b/cpp-seed" && mkdir -p "$b/cpp-seed"
    tar -C "$b/cpp-seed" -xJf "$seed"
    build_cpp "$b/cpp-seed"
    start=$b/cpp-seed/prismc
fi
rm -rf "$b/cpp-gen1" "$b/cpp-gen2" "$b/cpp-gen3" "$b/cpp-src" "$b/cpp-rust"
mkdir -p "$b/cpp-src"
cp -r prismc prelude "$b/cpp-src/"
src=("$b/cpp-src/prismc" --prelude "$b/cpp-src/prelude")
# compile N BINARY: prismc/ compiled to C++ by BINARY into cpp-genN
compile() {
    local t=$(date +%s)
    "$2" compile "${src[@]}" -o "$b/cpp-gen$1" --name prismc --target cpp
    echo "cpp-gen$1: prismc/ compiled to C++ by $2 in $(( $(date +%s) - t ))s"
}
# build N: cpp-genN built by make
build() {
    local t=$(date +%s)
    build_cpp "$b/cpp-gen$1"
    echo "cpp-gen$1: built with $CXX $OPT in $(( $(date +%s) - t ))s"
}
same() { diff -r -q "$b/cpp-gen$1/src" "$b/cpp-gen$2/src" && diff -q "$b/cpp-gen$1/Makefile" "$b/cpp-gen$2/Makefile"; }
compile 1 "$start"
build 1
compile 2 "$b/cpp-gen1/prismc"
fixed=1
if ! same 1 2; then
    echo "cpp-gen1 and cpp-gen2 differ: prismc/ emits other C++ than $start; one more generation"
    build 2
    compile 3 "$b/cpp-gen2/prismc"
    if ! same 2 3; then
        echo "cpp-gen2 and cpp-gen3 differ: no fixed point" >&2
        exit 1
    fi
    fixed=2
fi
echo "C++ fixed point: cpp-gen$fixed and cpp-gen$((fixed + 1)) are byte-equal ($(cat "$b/cpp-gen$fixed"/src/*.cpp "$b/cpp-gen$fixed"/src/*.hpp | wc -l) lines of C++)"
# The C++ compiler's Rust against the Rust compiler's ($RUST_PRISMC, else the Rust
# bootstrap's, which must be built from the same prismc/: run `make bootstrap` first).
rust=${RUST_PRISMC:-$b/prismc}
if [ -x "$rust" ]; then
    rm -rf "$b/cpp-rust" "$b/rust-rust"
    "$b/cpp-gen$fixed/prismc" compile "${src[@]}" -o "$b/cpp-rust" --name prismc
    "$rust" compile "${src[@]}" -o "$b/rust-rust" --name prismc
    if diff -r -q "$b/cpp-rust" "$b/rust-rust"; then
        echo "the C++ prismc compiles prismc/ to the Rust the Rust prismc does, byte for byte"
    else
        echo "the C++ prismc's Rust differs from $rust's" >&2
        exit 1
    fi
else
    echo "no Rust prismc (make bootstrap): the comparison with its Rust is skipped"
fi
cp "$b/cpp-gen$fixed/prismc" "$b/prismc-cpp.new" && mv "$b/prismc-cpp.new" "$b/prismc-cpp"
echo "installed $b/prismc-cpp (cpp-gen$fixed)"
if [ $update = 1 ]; then
    tar -C "$b/cpp-gen$fixed" -cJf "$seed.new" src Makefile && mv "$seed.new" "$seed"
    echo "seed: wrote $seed from cpp-gen$fixed ($(du -h "$seed" | cut -f1))"
fi
