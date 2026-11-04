#!/usr/bin/env bash
# Fetches what the Cuis table runs on into target/bench/deps: the Cuis-Smalltalk-Dev
# repository at its newest commit (a shallow clone: the image, its sources, and the
# CuisVM.app it ships, an OpenSmalltalk Cog VM with the VectorEnginePlugin Cuis 7 draws
# with). Records the commit it got in target/bench/deps/cuis.commit.
#
# Written by Claude on the claude-self-hosting branch; subject to change.
set -eu
deps=$(cd "$(dirname "$0")/../.." && pwd)/target/bench/deps
mkdir -p "$deps"
if [ ! -d "$deps/cuis" ]; then
    git clone -q --depth 1 https://github.com/Cuis-Smalltalk/Cuis-Smalltalk-Dev.git "$deps/cuis"
fi
xattr -dr com.apple.quarantine "$deps/cuis/CuisVM.app" 2>/dev/null || true
git -C "$deps/cuis" rev-parse HEAD > "$deps/cuis.commit"
echo "cuis: $(cat "$deps/cuis.commit") in $deps/cuis"
