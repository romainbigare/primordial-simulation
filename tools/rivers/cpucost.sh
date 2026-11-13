#!/bin/bash
# The CPU time a forced frame costs (the browser's whole process tree, ms) on the phone profile, pages A and B by
# turns, ROUNDS times: cpucost.sh A B [VIEWS]; then the median and the spread of each page and view (cpustat.py).
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/rv
cd $S || exit 1
a=$1; b=$2
VIEWS=${3:-"fit:0:11:11,b2km:2000:15.7099:12.6688,b200m:200:15.7099:12.6688"}
for r in $(seq 1 "${ROUNDS:-3}"); do
  for v in $a $b; do
    PAGE=$v/index.html FRAMES=${FRAMES:-6} timeout 3000 node bench.js 8761 ${PROFILE:-phone} 1 150 time /dev/null "$VIEWS" 2>&1 | python3 $S/cpustat.py tag $v
  done
done | python3 $S/cpustat.py sum
