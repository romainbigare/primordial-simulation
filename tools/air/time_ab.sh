#!/bin/bash
# Flow's wall time (the tape's ms_flow, years 6 to 20) of the lifeless globe and the lifeless Wide island for two builds, in turn.
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/s3
for round in 1 2 3; do
  for T in "$@"; do
    for world in globe wide; do
      O=$S/timing/${T}_${world}_$round
      rm -rf $O
      if [ $world = globe ]; then a="--globe --size 32"; else a="--band 20,65 --size 32 --seed 1"; fi
      $S/$T/ew run $a --years 20 --every 20 --life-year -1 --frames-every 100 --out $O > $O.log 2>&1
      python3 - $O/series.csv "$T $world round $round" <<'PY'
import csv, sys
rows = list(csv.DictReader(open(sys.argv[1])))
ms = [float(r['ms_flow']) for r in rows if float(r['year']) > 5]
allms = [sum(float(r[k]) for k in r if k.startswith('ms_')) for r in rows if float(r['year']) > 5]
print(f"{sys.argv[2]}: ms_flow a year, mean {sum(ms)/len(ms):.1f}, least {min(ms):.1f}; all ms a year, mean {sum(allms)/len(allms):.1f}")
PY
    done
  done
done
