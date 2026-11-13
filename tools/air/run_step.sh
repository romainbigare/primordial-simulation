#!/bin/bash
F=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/s3/final
for n in main S2 S5 S3a tip; do
  for s in 1 2 3; do for d in 15 1; do $F/bin/stepprobe_$n $s 6 $d > $F/step/${n}_s${s}_d$d.txt 2>&1 & done; done
  wait
done
echo STEP_RUNS_DONE > $F/step/DONE
