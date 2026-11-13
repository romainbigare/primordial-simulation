#!/bin/bash
# Round five, item 1: phone pictures of page $1 at a lake with two rivers in and at a sea mouth, five bars; with PAN, a
# pan frame too.
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/rv
cd $S || exit 1
P=$1
bash $S/shots.sh $P shots/$P phone "500 200 100" "lake:13.3:10.9" | tail -1
bash $S/shots.sh $P shots/$P phone "50 10" "lakem:13.344:10.936" | tail -1
bash $S/shots.sh $P shots/$P phone "500 200 100 50 10" "sea:17.608:14.299" | tail -1
if [ -n "$PAN" ]; then PAGE=$P/index.html timeout 1500 node pan.js 8761 phone 1 150 shot shots/pan/$P "b100:100:12.9:10.8" | tail -1; fi
echo done-$P
