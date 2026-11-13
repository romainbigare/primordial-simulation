#!/bin/bash
# Phone pictures of page P, then its frame cost against p0f (and p14f's, the rivers' work before the merge), then the
# animation's frames. measure.sh P
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/rv
cd $S || exit 1
P=$1
bash $S/shots.sh $P shots/$P phone "2000 200 50 10" "plain:15.7099:12.6688 mount:8.3911:5.4021 hill:9.4:12.5" > $S/measure-$P.shots 2>&1
ROUNDS=3 bash $S/cost.sh p0f ${P}f > $S/measure-$P.cost 2>&1
ROUNDS=2 bash $S/cost.sh p0f p14f > $S/measure-$P.cost14 2>&1
ROUNDS=2 bash $S/cost.sh p0f ${P}f "plain50:50:15.7099:12.6688,plain10:10:15.7099:12.6688,mount10:10:8.3911:5.4021,ridge10:10:12.4:6.95" > $S/measure-$P.near 2>&1
timeout 900 node anim.js ${P}f phone "plain10:10:15.7099:12.6688,dry10:10:12.4:6.95" > $S/measure-$P.anim 2>&1
echo done > $S/measure-$P.done
