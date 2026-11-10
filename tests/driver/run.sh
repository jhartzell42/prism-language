#!/usr/bin/env bash
# The `prism` driver's World choice (`prism run --print-world`): the World prismc takes from
# the root module's `emit` row, `Gui.*` the canvas World, a row without Ui or Gui the CLI
# World, a row with `Ui.*` and no `Gui.*` refused (no World answers Ui elements alone), and
# `--world` over all of it; a program with no `@main` has no root to run. Then prismc's
# own options: an option it does not know, an unknown target, or an option missing its
# value is refused, never ignored; an absolute `-o` finds the runtime at PRISM_ROOT or is
# refused, naming `--runtime`; and `prism compile --target cpp` writes C++. Last, chess
# compiled to C++ runs under the C++ canvas World: its `main` calls the World and its
# Makefile links it. Between them, a binder an arm reads once is moved (gap OC), and a program's small
# modules share one C++ translation unit (pick SF).
#
#   tests/driver/run.sh
#
# prismc is the `prism` script's (`$PRISMC`, else the newest binary). Exits 1 listing the
# cases that fail.
#
# Written by Claude, 3 November 2026, on the claude-self-hosting branch. Subject to change.
set -u
cd "$(dirname "$0")/../.." || exit 2
export PRISM_ROOT=$PWD
failed=()
# expect NAME WANT ARGS...: `prism run --print-world ARGS` prints WANT and exits 0, or with
# WANT `refused` exits 2 saying why
expect() {
    local name=$1 want=$2; shift 2
    local out code
    out=$(./prism run --print-world "$@" 2>&1); code=$?
    if [ "$want" = refused ]; then
        if [ $code -eq 2 ] && [ -n "$out" ]; then echo "ok $name: refused ($(echo "$out" | tail -1))"; return; fi
    elif [ $code -eq 0 ] && [ "$(echo "$out" | tail -1)" = "$want" ]; then
        echo "ok $name: $want"; return
    fi
    failed+=("$name")
    echo "FAILED $name: wanted $want, got exit $code: $out"
}
expect chess canvas examples/chess
expect cli-root cli tests/driver/cli-root.prism
expect gui-tree canvas tests/driver/gui-tree
expect birds refused tests/conformance/sem-ch06-birds
expect birds-as-cli cli --world cli tests/conformance/sem-ch06-birds
expect counter-no-root refused tests/conformance/counter

prismc=${PRISMC:-$(ls -t target/bootstrap/prismc target/keep/prismc-* 2>/dev/null | grep -v old-chain | head -1)}
out=target/driver
rm -rf "$out"
# refuse NAME WORDS ARGS...: `prismc ARGS` exits non-zero, says WORDS, and writes nothing
refuse() {
    local name=$1 words=$2; shift 2
    local said code
    said=$("$prismc" "$@" 2>&1); code=$?
    if [ $code -ne 0 ] && echo "$said" | grep -q -- "$words" && [ ! -e "$out/$name" ]; then
        echo "ok $name: refused ($words)"; return
    fi
    failed+=("$name")
    echo "FAILED $name: wanted a refusal saying '$words', got exit $code: $said"
}
refuse unknown-option "unknown option --bogus" compile --bogus tests/driver/cli-root.prism -o "$out/unknown-option"
refuse unknown-target "unknown target wasm" compile --target wasm tests/driver/cli-root.prism -o "$out/unknown-target"
refuse missing-value "--target takes a value" compile tests/driver/cli-root.prism -o "$out/missing-value" --target
refuse check-unknown-option "unknown option --bogus" --check --bogus tests/driver/cli-root.prism
# An absolute -o: the Rust crate finds the runtime at PRISM_ROOT, and builds; with neither
# PRISM_ROOT nor --runtime it is refused, naming --runtime, since `..` per segment of an
# absolute OUT would climb to `/`. A relative -o with no PRISM_ROOT keeps `..` per
# segment, OUT being relative to the repository root (the bootstraps' and ./prism's call).
abs=$PWD/$out/abs-rust
abslog=target/driver-absolute.log
if "$prismc" compile tests/driver/cli-root.prism -o "$abs" > "$abslog" 2>&1 &&
    CARGO_TARGET_DIR=$PWD/target/prism-build cargo build --offline --release -q --manifest-path "$abs/Cargo.toml" >> "$abslog" 2>&1; then
    echo "ok absolute-out: the crate finds the runtime at PRISM_ROOT and builds"
else
    failed+=(absolute-out)
    echo "FAILED absolute-out: an absolute -o with PRISM_ROOT set does not build ($abslog)"
fi
said=$(env -u PRISM_ROOT "$prismc" compile tests/driver/cli-root.prism -o "$PWD/$out/absolute-out-bare" 2>&1); code=$?
if [ $code -ne 0 ] && echo "$said" | grep -q -- "an absolute -o needs --runtime PATH or PRISM_ROOT" && [ ! -e "$out/absolute-out-bare" ]; then
    echo "ok absolute-out-bare: refused, naming --runtime"
else
    failed+=(absolute-out-bare)
    echo "FAILED absolute-out-bare: wanted a refusal naming --runtime, got exit $code: $said"
fi
if env -u PRISM_ROOT "$prismc" compile tests/driver/cli-root.prism -o "$out/relative" > /dev/null 2>&1 &&
    grep -q 'path = "../../../runtime/rust"' "$out/relative/Cargo.toml"; then
    echo "ok relative-out: .. per segment of a relative OUT"
else
    failed+=(relative-out)
    echo "FAILED relative-out: a relative -o with no PRISM_ROOT does not find the runtime by .. per segment"
fi
if ./prism compile --target cpp tests/driver/cli-root.prism -o "$out/cpp" > /dev/null 2>&1 && [ -f "$out/cpp/src/main.cpp" ] && [ ! -e "$out/cpp/Cargo.toml" ]; then
    echo "ok compile-cpp: C++ written"
else
    failed+=(compile-cpp)
    echo "FAILED compile-cpp: prism compile --target cpp wrote no C++ program (or a Cargo.toml)"
fi
# Pick SF: small modules' definitions share a translation unit; headers stay per module.
if ./prism compile --target cpp tests/conformance/mod-nested-names -o "$out/units" > /dev/null 2>&1 &&
    [ -f "$out/units/src/unit_1.cpp" ] && [ ! -e "$out/units/src/unit_2.cpp" ] &&
    [ -z "$(find "$out/units/src" -name 'mod*.cpp')" ] && [ -f "$out/units/src/a/types/module.hpp" ] &&
    [ "$(grep -c '^namespace crate::' "$out/units/src/unit_1.cpp")" -ge 3 ]; then
    echo "ok cpp-units: a program's small modules in one unit, their headers per module"
else
    failed+=(cpp-units)
    echo "FAILED cpp-units: mod-nested-names' C++ is not one unit over per-module headers ($out/units/src)"
fi
# Gap OC: a binder of an owned scrutinee its arm reads once is moved, not cloned.
if ./prism compile tests/conformance/sem-ch07-normalize -o "$out/normalize" > /dev/null 2>&1 &&
    grep -Eq 'Option::Some\((b2_[0-9a-v]+)\) => ::std::option::Option::Some\(Term::Lam\(::std::rc::Rc::new\(\1\)\)\),' "$out/normalize/src/program.rs"; then
    echo "ok owned-binder-moved: a binder read once is moved"
else
    failed+=(owned-binder-moved)
    echo "FAILED owned-binder-moved: ?Reduce's b2 is not moved at its one read ($out/normalize/src/program.rs)"
fi
# Gap LF: a function whose result holds its `$` parameter takes the dynamic whole (Type
# System ch. 4, Application): lowered, a `Dynamic` in and out, never refused.
said=$("$prismc" compile tests/driver/lifted-holds-dynamic.prism -o "$out/lifted-holds-dynamic" 2>&1)
if [ $? -eq 0 ] && grep -Eq 'fn wrap_f\(a_[a-z0-9]+: &crate::prelude::Dynamic<crate::prelude::Nat>\) -> crate::prelude::Array<crate::prelude::Dynamic<crate::prelude::Nat>>' "$out/lifted-holds-dynamic/src/program.rs"; then
    echo "ok lifted-holds-dynamic: Wrap takes its dynamic whole"
else
    failed+=(lifted-holds-dynamic)
    echo "FAILED lifted-holds-dynamic: wanted Wrap lowered over a Dynamic, got: $said"
fi
# A type variable no constraint reaches is settled by the checker (the unit type, in the
# typed output), never left for lowering to guess; only the variable `xs` quantified stays
# one, given at its use.
"$prismc" --typed tests/driver/free-variable-defaulted.prism --out "$out/free-variable-defaulted.typed" > /dev/null 2>&1
if [ -f "$out/free-variable-defaulted.typed" ] && [ "$(grep -c '(var ' "$out/free-variable-defaulted.typed")" = 2 ] &&
    grep -q "^  (binder \"xs\" .*(array (var t[0-9]*)))" "$out/free-variable-defaulted.typed" &&
    grep -q "^  (expr .*:10:35-10:39 (array unit))" "$out/free-variable-defaulted.typed" &&
    grep -q "('[0-9]* unit)" "$out/free-variable-defaulted.typed"; then
    echo "ok free-variable-defaulted: the checker settles it"
else
    failed+=(free-variable-defaulted)
    echo "FAILED free-variable-defaulted: wanted no undecided variable in the typed output ($out/free-variable-defaulted.typed)"
fi
# A bare `Error` as a tuple's part nothing decides is a `~unit`'s failure (ruling NE): the
# typed output shows the tuple as `((~unit, Int))`.
"$prismc" --typed tests/driver/fault-variable-defaulted.prism --out "$out/fault-variable-defaulted.typed" > /dev/null 2>&1
if [ -f "$out/fault-variable-defaulted.typed" ] && grep -q "(tuple (~ unit) Int)" "$out/fault-variable-defaulted.typed" &&
    ! grep -q "(tuple Prelude.Error Int)" "$out/fault-variable-defaulted.typed"; then
    echo "ok fault-variable-defaulted: the part is a ~unit"
else
    failed+=(fault-variable-defaulted)
    echo "FAILED fault-variable-defaulted: wanted the part as (~ unit) in the typed output ($out/fault-variable-defaulted.typed)"
fi
# A static clause widget's plain result is refused by lowering, never dropped.
said=$("$prismc" compile tests/driver/clause-plain-result.prism -o "$out/clause-plain-result" 2>&1)
if [ $? -ne 0 ] && echo "$said" | grep -q 'not-lowered.*a widget of several clauses whose clause ends on a plain result'; then
    echo "ok clause-plain-result: refused"
else
    failed+=(clause-plain-result)
    echo "FAILED clause-plain-result: wanted a not-lowered refusal, got: $said"
fi
if ./prism compile --target cpp examples/chess -o "$out/chess-cpp" > /dev/null 2>&1 &&
    grep -q 'prism_world_canvas::run<' "$out/chess-cpp/src/main.cpp" &&
    make -s -C "$out/chess-cpp" -j"$(getconf _NPROCESSORS_ONLN)" > "$out/chess-cpp.log" 2>&1 &&
    [ -x "$out/chess-cpp/chess" ]; then
    echo "ok chess-cpp-canvas: built under the canvas World"
else
    failed+=(chess-cpp-canvas)
    echo "FAILED chess-cpp-canvas: chess compiled to C++ does not call or link the canvas World ($out/chess-cpp.log)"
fi
# a hosted row's `document` overrides the base `Row`'s (both const): a call through the base
# would otherwise get the base's empty document, silently
if [ -f "$out/chess-cpp.log" ] && ! grep -q 'hides overloaded virtual' "$out/chess-cpp.log"; then
    echo "ok cpp-row-document: a row's document overrides Row::document"
else
    failed+=(cpp-row-document)
    echo "FAILED cpp-row-document: an emitted row's document hides Row::document ($out/chess-cpp.log)"
fi

if [ ${#failed[@]} -gt 0 ]; then
    echo "failed: ${failed[*]}"
    exit 1
fi
echo "every World as asked"
