#!/bin/bash
# lchain.sh NAME BINARY PART...: lparts.sh for each part in turn, C++ with PRISM_JOBS=1.
S=/tmp/claude-0/-home-claude/5a4e5003-3931-59ff-932c-4878e7a04b3d/scratchpad
n=$1; b=$2; shift 2
for p in "$@"; do PRISM_JOBS=1 $S/lparts.sh $n $b $p; done
