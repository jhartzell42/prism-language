#!/usr/bin/env bash
# Stage 1 of the bootstrap (docs/architecture.md, "The pipeline"): compiler zero runs
# prismc, which compiles its own sources to a Rust crate in target/stage1; cargo builds it
# into target/stage1/prismc.
#
# Written by Claude, 31 October 2026, on the claude-self-hosting branch. Subject to change.
set -eu
cd "$(dirname "$0")/../.."
# compiler zero's evaluator recurses on the Rust stack (prismc/README.md, SK).
ulimit -s unlimited 2>/dev/null || ulimit -s 1048576 2>/dev/null || true
cargo build --release -q -p prism_zero
# Every stage compiles one snapshot of the sources (target/bootstrap-src/), taken here,
# so that edits to prismc/ while the stages run do not reach some stages and not others.
rm -rf target/bootstrap-src && mkdir -p target/bootstrap-src && cp -r prismc target/bootstrap-src/
sources=(target/bootstrap-src/prismc/*.prism target/bootstrap-src/prismc/templates/*.prism)
rm -rf target/stage1
start=$(date +%s)
target/release/prism_zero run "${sources[@]}" -- compile "${sources[@]}" -o target/stage1 --name prismc
echo "stage 1: prismc compiled under compiler zero in $(( $(date +%s) - start ))s"
start=$(date +%s)
(cd target/stage1 && CARGO_TARGET_DIR=../bootstrap-build cargo build --release -q)
cp target/bootstrap-build/release/prismc target/stage1/prismc
echo "stage 1: cargo built target/stage1/prismc in $(( $(date +%s) - start ))s"
