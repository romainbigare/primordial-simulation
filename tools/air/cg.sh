#!/bin/bash
# Instructions (valgrind's count) of 3 years of the lifeless globe and the lifeless Wide island for the builds given.
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/s3
for T in "$@"; do
  for world in globe wide; do
    if [ $world = globe ]; then a="--globe --size 32"; else a="--band 20,65 --size 32 --seed 1"; fi
    (valgrind --tool=callgrind --callgrind-out-file=$S/cg/${T}_$world.out $S/$T/ew run $a --years 3 --every 3 --life-year -1 --frames-every 100 --chunks 1 --out $S/cg/${T}_$world > $S/cg/${T}_$world.log 2>&1) &
  done
  wait
done
grep -H "Collected" $S/cg/*.log
