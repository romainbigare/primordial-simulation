#!/bin/bash
# The frame cost of page P against p0f on the phone profile (far views, then near), then the animation's frames.
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/rv
cd $S || exit 1
P=$1
rm -f $S/m2-$P.*
uptime > $S/m2-$P.cost
ROUNDS=3 bash $S/cost.sh p0f ${P}f >> $S/m2-$P.cost 2>&1
uptime >> $S/m2-$P.cost
ROUNDS=1 bash $S/cost.sh p0f ${P}f "plain50:50:15.7099:12.6688,plain10:10:15.7099:12.6688,mount10:10:8.3911:5.4021,ridge10:10:12.4:6.95" > $S/m2-$P.near 2>&1
timeout 900 node anim.js ${P}f phone "plain10:10:15.7099:12.6688,dry10:10:12.4:6.95" > $S/m2-$P.anim 2>&1
echo done > $S/m2-$P.done
