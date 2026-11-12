#!/bin/bash
# usage: climate.sh BIN TAG -- seeds 1 and 2, size 32, plants, 30 years; the climate numbers (climate.py)
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/speed
mkdir -p $S/clim/$2
$1 run --seed 1 --size 32 --years 30 --roster --plants --every 30 --out $S/clim/$2/s1 > $S/clim/$2/s1.log 2>&1 &
$1 run --seed 2 --size 32 --years 30 --roster --plants --every 30 --out $S/clim/$2/s2 > $S/clim/$2/s2.log 2>&1 &
wait
python3 $S/climate.py $S/clim/$2
