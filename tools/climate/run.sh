#!/bin/bash
# usage: run.sh BIN TAG YEARS [STRETCH] [DRIFT]   (STRETCH: "band" to derive it, a number, or empty)
cd /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/climate/runs
BIN=$1; TAG=$2; YEARS=$3; ST=$4; DR=$5
for band in "40 42.3" "30 55" "20 65"; do for size in 22 32; do for seed in 1 2; do
  set -- $band
  name=${TAG}_s${seed}_n${size}_b$1-$2
  $BIN $seed $size $1 $2 $YEARS $name $ST $DR > $name.log 2>&1; echo "$name $SECONDS" >> times.txt
done; done; done
echo done
