#!/bin/bash
# lparts.sh NAME BINARY PART: one part of the suites on BINARY, logs in target/probes/NAME/.
# PART: rest (check, unused, checker, driver, Rust), cpp1, cpp2 (conformance halves),
# cpp3 (chess, prelude maps, lexer, parser, regex).
cd /home/claude/prism-lowering
L=target/probes/$1; mkdir -p $L; export PRISMC=$2
names=($(ls tests/conformance)); n=${#names[@]}; h=$((n/2))
case $3 in
rest)
  ./prism check prismc/ > $L/check.log 2>&1; echo "check exit $?" >> $L/check.log
  ./prism check --unused prismc/ > $L/unused.log 2>&1; echo "unused exit $?" >> $L/unused.log
  tests/checker/run.sh > $L/checker.log 2>&1; echo "checker exit $?" >> $L/checker.log
  tests/driver/run.sh > $L/driver.log 2>&1; echo "driver exit $?" >> $L/driver.log
  tests/compiled/run.sh > $L/rust.log 2>&1; echo "rust exit $?" >> $L/rust.log
  for f in check unused checker driver rust; do tail -1 $L/$f.log; done; grep -h "^failed" -A30 $L/rust.log | head -20 ;;
cpp1) CXX=clang++ tests/compiled/run.sh --target cpp "${names[@]:0:$h}" > $L/cpp1.log 2>&1; echo "cpp1 exit $?" >> $L/cpp1.log; tail -12 $L/cpp1.log ;;
cpp2) CXX=clang++ tests/compiled/run.sh --target cpp "${names[@]:$h}" > $L/cpp2.log 2>&1; echo "cpp2 exit $?" >> $L/cpp2.log; tail -12 $L/cpp2.log ;;
cpp3) CXX=clang++ tests/compiled/run.sh --target cpp chess-rules prelude-maps chess lexer parser regex > $L/cpp3.log 2>&1; echo "cpp3 exit $?" >> $L/cpp3.log; tail -12 $L/cpp3.log ;;
esac
