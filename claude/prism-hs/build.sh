#!/bin/bash
# cabal cannot reach Hackage from here; ghc --make against the system package db is enough.
set -e
cd "$(dirname "$0")"
mkdir -p build
ghc --make -O0 -isrc -iapp -outputdir build -o prism-hs app/Main.hs -Wall -Wno-name-shadowing -Wno-unused-matches -Wno-unused-do-bind -Wno-incomplete-patterns -Wno-incomplete-uni-patterns -Wno-unused-imports -Wno-type-defaults "$@"
