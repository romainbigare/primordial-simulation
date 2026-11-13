#!/bin/bash
# Two interleaved rounds of the far views' frame cost: p0f, p17f, p18f.
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/rv
cd $S || exit 1
rm -f $S/m3.*
for r in 1 2; do
  uptime >> $S/m3.cost
  ROUNDS=1 bash $S/cost.sh p0f p17f >> $S/m3.cost 2>&1
  ROUNDS=1 bash $S/cost.sh p18f p0f >> $S/m3.cost 2>&1
done
uptime >> $S/m3.cost
echo done > $S/m3.done
