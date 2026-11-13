#!/bin/bash
# CPU seconds (user + sys) of 20 years of the lifeless globe and the lifeless Wide island for the builds given, in turn.
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/s3
TIMEFORMAT="%U %S"
for round in 1 2 3 4; do
  for T in "$@"; do
    for world in globe wide; do
      O=$S/timing/cpu_${T}_${world}
      rm -rf $O
      if [ $world = globe ]; then a="--globe --size 32"; else a="--band 20,65 --size 32 --seed 1"; fi
      t=$( { time $S/$T/ew run $a --years 20 --every 20 --life-year -1 --frames-every 100 --out $O > $O.log 2>&1; } 2>&1 )
      echo "$T $world $round: $t"
    done
  done
done
