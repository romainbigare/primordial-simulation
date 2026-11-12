#!/bin/bash
# usage: bench2.sh BIN_A TAG_A BIN_B TAG_B -- sizes 22 32 64, 30 years, 3 repeats, A and B in turn (noise falls on both)
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/speed
mkdir -p $S/bench/$2 $S/bench/$4
for size in 22 32 64; do for k in 1 2 3; do
  $1 run --seed 1 --size $size --years 30 --roster --plants --every 1 --out $S/bench/$2/$size-$k > $S/bench/$2/$size-$k.log 2>&1
  $3 run --seed 1 --size $size --years 30 --roster --plants --every 1 --out $S/bench/$4/$size-$k > $S/bench/$4/$size-$k.log 2>&1
done; done
cd $S && echo "$2" && python3 table.py bench/$2 && echo "$4" && python3 table.py bench/$4
