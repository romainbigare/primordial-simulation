#!/bin/bash
# S3a: the globe (year 30) as committed, with the lost helper's moist_min 0.7, and with a rain_time of 2 hours; the Wide island.
D=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/s3/S3a
(time $D/radprobe globe 30) > $D/globe.txt 2>&1 &
MOIST=0.7 $D/radprobe globe 30 > $D/globe_r07.txt 2>&1 &
RAINT=7200 $D/radprobe globe 30 > $D/globe_tau2h.txt 2>&1 &
$D/radprobe wide 1 32 30 > $D/wide.txt 2>&1 &
wait
echo done > $D/DONE
