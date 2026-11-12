#!/usr/bin/env bash
# Both seeds built where a fresh checkout builds them, under a target/ that is a symlink out
# of the repository (as every worktree here keeps it, out of iCloud): each seed unpacked
# three levels under the root, in a directory whose physical place is elsewhere, built by
# the bootstrap scripts' own `--seed-at`, and its binary run on a loose file.
#
#   bootstrap/test-seed.sh
#
# Written by Claude on the claude-self-hosting branch; subject to change.
set -eu
cd "$(dirname "$0")/.."
export PRISM_ROOT=$PWD
mkdir -p target
away=$(mktemp -d)
link=target/seed-check
rm -rf "$link"
ln -s "$away" "$link"
trap 'rm -rf "$link" "$away"' EXIT
failed=()
check() { # name, binary
    if [ -x "$2" ] && "$2" --check tests/driver/cli-root.prism --out "$away/$1.out" > /dev/null 2>&1 && [ "$(tail -1 "$away/$1.out")" = "no errors" ]; then
        echo "ok $1: the seed builds through a symlinked target and checks a file"
    else
        failed+=("$1"); echo "FAILED $1"
    fi
}
# The Rust seed carries the runtime its crate was emitted against, so a change to the
# working tree's runtime/rust never breaks it.
if tar -tJf bootstrap/prismc-rust.tar.xz | grep -qx 'runtime/rust/Cargo.toml'; then
    echo "ok rust-runtime: the seed carries its own runtime/rust"
else
    failed+=(rust-runtime); echo "FAILED rust-runtime: the seed has no runtime/rust of its own"
fi
# So does the C++ seed its runtime/cpp.
if tar -tJf bootstrap/prismc-cpp.tar.xz | grep -qx 'runtime/cpp/Makefile'; then
    echo "ok cpp-runtime: the C++ seed carries its own runtime/cpp"
else
    failed+=(cpp-runtime); echo "FAILED cpp-runtime: the C++ seed has no runtime/cpp of its own"
fi
bootstrap/bootstrap.sh --seed-at "$link/rust" > "$away/rust.log" 2>&1 || tail -20 "$away/rust.log"
check rust "$link/rust/prismc"
bootstrap/bootstrap-cpp.sh --seed-at "$link/cpp" > "$away/cpp.log" 2>&1 || tail -20 "$away/cpp.log"
check cpp "$link/cpp/prismc"
[ ${#failed[@]} -eq 0 ] || { echo "failed: ${failed[*]}"; exit 1; }
echo "both seeds build through a symlinked target"
