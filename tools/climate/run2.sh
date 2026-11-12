#!/bin/bash
# Extra runs: other bands with the stretch, the drift stretched too, and one long run.
cd /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/climate/runs
B=../climate_B
for band in "30 50" "25 50" "30 60" "20 50" "35 50"; do
  set -- $band
  $B 1 32 $1 $2 30 E_s1_n32_b$1-$2 band > E_s1_n32_b$1-$2.log 2>&1
done
# The drift stretched too: rain_drift_m / stretch (30-55, size 32: stretch 43.4).
$B 1 32 30 55 30 D_s1_n32_b30-55 band 0.02302 > D_s1_n32_b30-55.log 2>&1
$B 1 32 30 55 100 L_s1_n32_b30-55 band > L_s1_n32_b30-55.log 2>&1
echo done
