#!/bin/bash
# usage: bench.sh BIN TAG -- sizes 22 32 64, 30 years, 3 repeats, one at a time
B=$1; T=$2; S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/speed
for size in 22 32 64; do for k in 1 2 3; do
  $B run --seed 1 --size $size --years 30 --roster --plants --every 1 --out $S/bench/$T/$size-$k > $S/bench/$T/$size-$k.log 2>&1
done; done
