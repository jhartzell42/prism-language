#!/usr/bin/env bash
# Stage 3 of the bootstrap: stage 2's prismc compiles prismc's sources (the snapshot stage 1
# took, as a directory, read through Fs.List) into target/stage3, and cargo builds it into
# target/stage3/prismc.
#
# Written by Claude, 1 November 2026, on the claude-self-hosting branch. Subject to change.
set -eu
cd "$(dirname "$0")/../.."
ulimit -s unlimited 2>/dev/null || ulimit -s 1048576 2>/dev/null || true
[ -x target/stage2/prismc ] || tests/bootstrap/stage2.sh
[ -d target/bootstrap-src/prismc ] || { echo "no source snapshot: run tests/bootstrap/stage1.sh" >&2; exit 2; }
rm -rf target/stage3
start=$(date +%s)
target/stage2/prismc compile target/bootstrap-src/prismc -o target/stage3 --name prismc
echo "stage 3: prismc compiled by stage 2 in $(( $(date +%s) - start ))s"
start=$(date +%s)
(cd target/stage3 && CARGO_TARGET_DIR=../bootstrap-build cargo build --release -q)
cp target/bootstrap-build/release/prismc target/stage3/prismc
echo "stage 3: cargo built target/stage3/prismc in $(( $(date +%s) - start ))s"
