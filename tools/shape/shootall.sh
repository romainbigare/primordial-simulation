#!/bin/bash
# shootall.sh TAG PORT: the whole islands and the close views of one build of the page.
cd /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/shape
T=$1; P=$2; Y=5
mkdir -p $T
node shoot.js $P 1 22 $Y $T/s1-22 earth 9,10,2000 9,10,500 15,13,2000 15,13,500 17,6,2000 17,6,500
node shoot.js $P 2 22 $Y $T/s2-22 earth 13,13,2000 13,13,500 7,5,2000 7,5,500 3,8,2000 3,8,500
for s in 3 4 5 6; do node shoot.js $P $s 22 $Y $T/s$s-22 earth; done
for s in 1 2 3; do node shoot.js $P $s 32 $Y $T/s$s-32 earth; done
for s in 1 2; do node shoot.js $P $s 22 $Y $T/true-s$s-22 true; done
echo DONE
