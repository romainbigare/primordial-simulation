#!/bin/bash
# usage: cmp.sh DIR1 DIR2 -- fingerprints and series.csv but ms_*
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/speed
ok=1
for r in a b c d; do
  h1=$(grep -o "hash [0-9a-f]*" $1/$r.log); h2=$(grep -o "hash [0-9a-f]*" $2/$r.log)
  python3 $S/strip.py $1/$r/series.csv > /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/s1.csv
  python3 $S/strip.py $2/$r/series.csv > /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/s2.csv
  if [ "$h1" == "$h2" ] && [ -n "$h1" ] && cmp -s /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/s1.csv /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/s2.csv; then echo "$r same $h1"; else echo "$r DIFFERENT $h1 / $h2"; ok=0; fi
done
