#!/usr/bin/env bash
# The bootstrap's fixed point: the sources stage 2 and stage 3 emit are byte-equal
# (docs/architecture.md, "The goal"). Runs stage 1, 2 and 3 when their binaries are
# missing.
#
# Written by Claude, 31 October 2026, on the claude-self-hosting branch. Subject to change.
set -eu
cd "$(dirname "$0")/../.."
[ -x target/stage3/prismc ] || tests/bootstrap/stage3.sh
if diff -r -q -x target -x prismc target/stage2 target/stage3; then
    echo "stage 2 and stage 3 emitted byte-equal sources ($(cat target/stage3/src/*.rs | wc -l) lines of Rust)"
else
    echo "stage 2 and stage 3 differ" >&2
    exit 1
fi
