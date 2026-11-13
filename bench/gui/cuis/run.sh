#!/usr/bin/env bash
# The Cuis table (table.st) for one operation, on the Cog VM with the Cuis image:
# the script with OP, RUNS and WARMUP written in, run in a copy
# of the image so the clone stays clean. Prints `OP RUN MS` per timed run, as the other
# drivers do.
#
#   bench/gui/cuis/run.sh OP RUNS WARMUP
#
# The image and the VM are under target/bench/deps (bench/gui/cuis/setup.sh fetches them).
#
# Written by Claude on the claude-self-hosting branch; subject to change.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
deps=$here/../../../target/bench/deps
vm=$deps/cuis/CuisVM.app/Contents/MacOS/Squeak
image=$(ls "$deps"/cuis/CuisImage/Cuis*.image | head -1)
[ -x "$vm" ] && [ -f "$image" ] || { echo "cuis: no VM or image under $deps (run bench/cuis/setup.sh)" >&2; exit 2; }
work=$deps/work
mkdir -p "$work"
cp "$image" "${image%.image}.changes" "$work/"
ln -sf "$(ls "$deps"/cuis/CuisImage/*.sources | head -1)" "$work/"
sed "s/@OP@/$1/; s/@RUNS@/$2/; s/@WARMUP@/$3/" "$here/table.st" > "$work/run.st"
cd "$work"
exec "$vm" "$(basename "$image")" -s run.st
