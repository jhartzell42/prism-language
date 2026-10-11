#!/bin/bash
# Runs every example file against the prelude, one file at a time: the prototype has a single
# top-level namespace, so files that reuse a name (Clock, Show, @main) cannot be loaded together.
cd "$(dirname "$0")"
./build.sh >/dev/null || exit 1
for f in $(ls examples/*.prism | grep -v prelude); do
  echo "== $f: $(./prism-hs examples/prelude.prism "$f" 2>&1 | tail -1)"
done
