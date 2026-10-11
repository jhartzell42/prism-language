#!/bin/bash
# Builds with cabal (the evaluator needs `reflex` from Hackage) and puts the binary where
# test.sh expects it.
set -e
cd "$(dirname "$0")"
export PATH="$HOME/.ghcup/bin:$PATH"
cabal build -v0 exe:prism-hs "$@"
cp "$(cabal list-bin exe:prism-hs)" ./prism-hs
