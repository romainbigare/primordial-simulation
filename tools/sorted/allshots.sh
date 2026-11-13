#!/bin/bash
# All before | after shots: years 3, 30, 100; desk and phone; main (8821) and the sorted start (8822).
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/sorted
for y in 3 30 100; do
  for mode in desk phone; do
    $S/shoot.sh $y $mode main:8821 after:8822
    for v in fit mid near; do
      python3 $S/pair.py $S/shots/main-y$y-$mode-$v.png $S/shots/after-y$y-$mode-$v.png $S/shots/pair-y$y-$mode-$v.png
    done
  done
done
echo done > $S/shots/ALLDONE
