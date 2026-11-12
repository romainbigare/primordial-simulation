#!/bin/bash
# Full-frame cost of builds in turn, ROUNDS times (the order reversed every other round), to OUT (json lines).
# times.sh OUT PROFILE SEED "VIEWS" build...
F=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/frame
OUT=$1; PROF=$2; SEED=$3; VIEWS=$4; shift 4
B=("$@")
for r in $(seq 1 ${ROUNDS:-2}); do
  if (( r % 2 == 0 )); then L=($(printf '%s\n' "${B[@]}" | tac)); else L=("${B[@]}"); fi
  for b in "${L[@]}"; do
    echo -n "{\"build\":\"$b\",\"round\":$r,\"r\":" >> $OUT
    PAGE=dist-$b/${PAGEFILE:-index.html} FRAMES=${FRAMES:-3} timeout 1800 node $F/bench.js 8810 $PROF $SEED ${MONTHS:-150} time x "$VIEWS" 2>/dev/null | tail -1 | tr -d '\n' >> $OUT
    echo "}" >> $OUT
  done
done
