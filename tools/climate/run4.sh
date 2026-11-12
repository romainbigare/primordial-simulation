#!/bin/bash
# The strip coarser or shorter, against 5 deg rows and 20 years; and the band 35 to 70.
cd /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/climate/runs
B=../climate_S
for v in "144 20" "144 10" "72 20" "72 10"; do
  set -- $v
  STRIP_ROWS=$1 STRIP_YEARS=$2 $B 1 32 20 65 30 S_r$1_y$2 band 1 open > S_r$1_y$2.log 2>&1
done
$B 1 32 35 70 30 W_s1_n32_b35-70 band 1 open > W_s1_n32_b35-70.log 2>&1
$B 2 32 35 70 30 W_s2_n32_b35-70 band 1 open > W_s2_n32_b35-70.log 2>&1
echo done
