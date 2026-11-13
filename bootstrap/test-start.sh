#!/usr/bin/env bash
# Which compiler each bootstrap starts from, and what it says when the installed one is too
# old for the tree: under a scratch working directory (PRISM_BOOTSTRAP), an "installed"
# prismc that fails as a compiler older than the source's syntax does, and a $PRISMC.
#   - `--show-start` names $PRISMC first, else the installed compiler;
#   - `--from-seed --show-start` names the seed whatever is installed or set;
#   - a bootstrap from the failing installed compiler stops with the one line naming the
#     cause and `--from-seed`, keeps the compiler's diagnostics in its log, and builds no
#     seed.
#
#   bootstrap/test-start.sh
#
# Written by Claude on the claude-self-hosting branch; subject to change.
set -u
cd "$(dirname "$0")/.."
work=target/start-check
rm -rf "$work" && mkdir -p "$work"
trap 'rm -rf "$work"' EXIT
# The "installed" compilers: each fails at once, as an old binary does on new syntax.
fake='#!/bin/sh
echo "prism: the program gave up: Prismc.Refused: prismc/app.prism:9:1: expected a declaration, found \`mod\`" >&2
exit 1'
printf '%s\n' "$fake" > "$work/prismc"; chmod +x "$work/prismc"
printf '%s\n' "#!/bin/sh" "exit 1" > "$work/other"; chmod +x "$work/other"
failed=()
expect() { # name, wanted output, actual output
    if [ "$2" = "$3" ]; then echo "ok $1"; else failed+=("$1"); printf 'FAILED %s:\n  wanted: %s\n  got:    %s\n' "$1" "$2" "$3"; fi
}
for s in bootstrap cpp; do
    script=bootstrap/bootstrap.sh; seed=bootstrap/prismc-rust.tar.xz
    [ $s = cpp ] && { script=bootstrap/bootstrap-cpp.sh; seed=bootstrap/prismc-cpp.tar.xz; }
    # First, with a $PRISMC that fails at once: a script without --show-start stops here
    # quickly instead of running a whole bootstrap.
    shown=$(PRISMC=$work/other PRISM_BOOTSTRAP=$work $script --show-start 2>&1)
    expect "$s PRISMC" "start: \$PRISMC ($work/other)" "$shown"
    case "$shown" in start:*) ;; *) echo "FAILED: $script has no --show-start"; exit 1 ;; esac
    expect "$s installed" "start: the installed $work/prismc" "$(PRISM_BOOTSTRAP=$work $script --show-start 2>&1)"
    expect "$s from-seed" "start: the seed $seed" "$(PRISMC=$work/other PRISM_BOOTSTRAP=$work $script --from-seed --show-start 2>&1)"
    out=$(PRISM_BOOTSTRAP=$work $script 2>&1); code=$?
    expect "$s old installed: status" "1" "$code"
    expect "$s old installed: the line" "the installed $work/prismc cannot compile this tree (it is older than the source's syntax); re-run with --from-seed" "$(printf '%s\n' "$out" | tail -1)"
    log=$work/gen1-compile.log; [ $s = cpp ] && log=$work/cpp-gen1-compile.log
    expect "$s old installed: diagnostics kept" "1" "$(grep -c 'expected a declaration' "$log" 2>/dev/null || echo 0)"
    expect "$s old installed: no seed built" "" "$(printf '%s\n' "$out" | grep 'seed: building')"
done
# A $PRISMC whose runtime root $PRISMC_RUNTIME is an absolute path: gen1 is built against
# that root's runtime without writing into it. The "compiler" emits a crate with no
# dependency whose binary fails at once, so the bootstrap stops at gen2's compile.
rt=$(pwd)/$work/start-root
mkdir -p "$rt/runtime/rust/src"
printf '%s\n' '[package]' 'name = "prism_runtime"' 'version = "0.0.0"' > "$rt/runtime/rust/Cargo.toml"
emit='#!/bin/sh
while [ $# -gt 0 ]; do [ "$1" = -o ] && out=$2; shift; done
mkdir -p "$out/src"
printf "%s\n" "[package]" "name = \"prismc\"" "version = \"0.0.0\"" "edition = \"2021\"" "" "[workspace]" > "$out/Cargo.toml"
echo "fn main() { std::process::exit(1) }" > "$out/src/main.rs"'
printf '%s\n' "$emit" > "$work/emit"; chmod +x "$work/emit"
out=$(PRISMC=$(pwd)/$work/emit PRISMC_RUNTIME=$rt PRISM_BOOTSTRAP=$work bootstrap/bootstrap.sh 2>&1)
expect "bootstrap absolute PRISMC_RUNTIME: gen1 built" "gen1: cargo built against the runtime of $rt" "$(printf '%s\n' "$out" | grep '^gen1: cargo built' | sed 's/ in [0-9]*s .*//')"
expect "bootstrap absolute PRISMC_RUNTIME: the root untouched" "" "$(ls "$rt" | grep -v '^runtime$')"
[ ${#failed[@]} -eq 0 ] || { echo "failed: ${failed[*]}"; exit 1; }
echo "each bootstrap starts where it says, and refuses an old installed compiler by name"
