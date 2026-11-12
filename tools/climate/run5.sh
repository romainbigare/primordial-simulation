#!/bin/bash
# On the merged ground: Wide, Mild (in the planet's air) and None (alone), seeds 1 and 2, 32 x 32, 30 years.
cd /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/climate/runs
B=../climate_F
for seed in 1 2; do
  $B $seed 32 20 65 30 F_s${seed}_n32_b20-65 band 1 open > F_s${seed}_n32_b20-65.log 2>&1
  $B $seed 32 30 55 30 F_s${seed}_n32_b30-55 band 1 open > F_s${seed}_n32_b30-55.log 2>&1
  $B $seed 32 40 42.3 30 F_s${seed}_n32_b40-42.3 band 1 > F_s${seed}_n32_b40-42.3.log 2>&1
done
echo done
