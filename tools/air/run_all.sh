#!/bin/bash
# Measure a step's tree (binaries of build_all.sh TAG): the key numbers (3 seeds), the page's world (Wide, seed 1, size 22)
# to year 100 (cover rows, ice rows), the globe's and the islands' radiation budgets at year 30.
T=$1
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad
D=$S/s3/$T
W=/home/user/primordial-simulation/.claude/worktrees/agent-ab92bfed03ac3c12c
rm -f $D/DONE_ALL
($D/radprobe globe 30 > $D/globe.txt 2>&1; $D/radprobe wide 1 22 30 > $D/wide22.txt 2>&1; $D/radprobe wide 1 32 30 > $D/wide.txt 2>&1; $D/iceprobe 22 > $D/ice.txt 2>&1) &
$D/coverprobe run 1 22 100 $D/band_s1 band > $D/log_band_s1.txt 2>&1 &
wait
(cd $W/tape && $D/ranges the_key_numbers --ignored --nocapture > $D/keys.txt 2>&1)
echo done > $D/DONE_ALL
