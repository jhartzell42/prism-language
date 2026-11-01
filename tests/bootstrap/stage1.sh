#!/usr/bin/env bash
# Stage 1 of the bootstrap (docs/architecture.md, "The pipeline"): stage 0, a prismc
# binary of the old dynamic chain (its back end emitted `prism_rt` values over
# `prism_dyn`; its sources are in git history, 2524e50 and before), compiles prismc's
# sources to a Rust crate in target/stage1; cargo builds it into target/stage1/prismc, a
# native prismc with the typed back end. Compiler zero is not in the chain: it is orders
# of magnitude slower, and stays a fallback for a feature stage 0 refuses.
#
#   STAGE0=path/to/prismc tests/bootstrap/stage1.sh
#
# Written by Claude, 1 November 2026, on the claude-self-hosting branch. Subject to change.
set -eu
cd "$(dirname "$0")/../.."
stage0=${STAGE0:-target/keep/prismc-old-chain-1101-0619}
[ -x "$stage0" ] || { echo "no stage 0 at $stage0 (set STAGE0)" >&2; exit 2; }
ulimit -s unlimited 2>/dev/null || ulimit -s 1048576 2>/dev/null || true
# Every stage compiles one snapshot of the sources (target/bootstrap-src/), taken here,
# so that edits to prismc/ while the stages run reach all of them or none.
rm -rf target/bootstrap-src && mkdir -p target/bootstrap-src && cp -r prismc target/bootstrap-src/
sources=(target/bootstrap-src/prismc/*.prism target/bootstrap-src/prismc/templates/*.prism)
rm -rf target/stage1
start=$(date +%s)
"$stage0" compile "${sources[@]}" -o target/stage1 --name prismc
echo "stage 1: prismc compiled by stage 0 in $(( $(date +%s) - start ))s"
start=$(date +%s)
(cd target/stage1 && CARGO_TARGET_DIR=../stage1-build cargo build --release -q)
cp target/stage1-build/release/prismc target/stage1/prismc
echo "stage 1: cargo built target/stage1/prismc in $(( $(date +%s) - start ))s"
