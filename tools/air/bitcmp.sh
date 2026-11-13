#!/bin/bash
# Two builds give one world? The globe without life (10 years) and the Wide island with plants (8 years): equal hash and
# equal series.csv but the ms_ columns. bitcmp.sh TAG_A TAG_B
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/s3
for T in $1 $2; do
  ($S/$T/ew run --globe --size 32 --years 10 --every 10 --life-year -1 --frames-every 100 --out $S/bit/${T}_globe > $S/bit/${T}_globe.log 2>&1) &
  ($S/$T/ew run --seed 1 --size 32 --band 20,65 --years 8 --every 8 --roster --plants --frames-every 100 --out $S/bit/${T}_wide > $S/bit/${T}_wide.log 2>&1) &
done
wait
for w in globe wide; do
  for T in $1 $2; do grep -o "hash [0-9a-f]*" $S/bit/${T}_$w.log | tr '\n' ' '; done
  python3 - $S/bit/$1_$w/series.csv $S/bit/$2_$w/series.csv <<'PY'
import csv, sys
a, b = (list(csv.DictReader(open(f))) for f in sys.argv[1:3])
same = len(a) == len(b) and all(all(x[k] == y[k] for k in x if not k.startswith('ms_')) for x, y in zip(a, b))
print("series.csv equal but ms_:", same, len(a), "rows")
PY
done
