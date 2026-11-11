#!/bin/bash
# lgen1rev.sh NAME BINARY REV: prismc/ and prelude/ of git REV compiled by BINARY, built
# once, installed as target/probes/prismc-NAME.
cd /home/claude/prism-lowering || exit 2
b=target/gen1-$1
rm -rf "$b"; mkdir -p "$b/src"
git archive "$3" prismc prelude | tar -x -C "$b/src"
"$2" compile "$b/src/prismc" --prelude "$b/src/prelude" -o "$b/gen1" --name prismc > "$b/compile.log" 2>&1 || { tail -20 "$b/compile.log"; exit 1; }
CARGO_TARGET_DIR="$PWD/$b/build" cargo build --offline --release -q --manifest-path "$PWD/$b/gen1/Cargo.toml" 2> "$b/cargo.log" || { grep -A8 "^error" "$b/cargo.log" | head -60; exit 1; }
cp "$b/build/release/prismc" "target/probes/prismc-$1" && rm -rf "$b/build"
echo "built target/probes/prismc-$1"
