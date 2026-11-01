#!/usr/bin/env bash
# Stage 2 of the bootstrap: stage 1's prismc compiles prismc's sources into
# target/stage2, and cargo builds it into target/stage2/prismc.
#
# Written by Claude, 31 October 2026, on the claude-self-hosting branch. Subject to change.
set -eu
cd "$(dirname "$0")/../.."
ulimit -s unlimited 2>/dev/null || ulimit -s 1048576 2>/dev/null || true
[ -x target/stage1/prismc ] || tests/bootstrap/stage1.sh
# The snapshot stage 1 took (target/bootstrap-src/).
[ -d target/bootstrap-src/prismc ] || { echo "no source snapshot: run tests/bootstrap/stage1.sh" >&2; exit 2; }
sources=(target/bootstrap-src/prismc/*.prism target/bootstrap-src/prismc/templates/*.prism)
rm -rf target/stage2
start=$(date +%s)
target/stage1/prismc -- compile "${sources[@]}" -o target/stage2 --name prismc
echo "stage 2: prismc compiled by stage 1 in $(( $(date +%s) - start ))s"
start=$(date +%s)
(cd target/stage2 && CARGO_TARGET_DIR=../bootstrap-build cargo build --release -q)
cp target/bootstrap-build/release/prismc target/stage2/prismc
echo "stage 2: cargo built target/stage2/prismc in $(( $(date +%s) - start ))s"
