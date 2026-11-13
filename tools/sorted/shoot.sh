#!/bin/bash
# shoot.sh YEAR MODE NAME:PORT ... : near/mid/fit shots at year YEAR (day ~120) for each served page, sequentially.
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/sorted
y=$1; mode=$2; shift 2
cd $S
for d in "$@"; do
  n=${d%%:*}; port=${d##*:}
  DAY=120 node at2.js $port $S/shots/$n-y$y-$mode $y 14 15 8 6 $mode > $S/shots/$n-y$y-$mode.txt 2>&1
  echo "$n y$y $mode: $(grep -h 'paused at' $S/shots/$n-y$y-$mode.txt) $(grep -h 'errors' $S/shots/$n-y$y-$mode.txt)"
done
