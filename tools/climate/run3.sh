#!/bin/bash
# Open edges: bands 30-55, 20-65, 25-60, seeds 1 and 2, size 32 (and 22), 30 years, the stretch of the band.
cd /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/climate/runs
B=${1:-../climate_O}; TAG=${2:-O}
for band in "30 55" "20 65" "25 60"; do for size in 32 22; do for seed in 1 2; do
  set -- $band
  name=${TAG}_s${seed}_n${size}_b$1-$2
  $B $seed $size $1 $2 30 $name band 1 open > $name.log 2>&1
  echo "$name $SECONDS" >> times.txt
done; done; done
echo done
