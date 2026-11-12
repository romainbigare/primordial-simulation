#!/bin/sh
# Frames a second of two builds measured at the same time (same contention): pair-cost.sh PORT_A PORT_B RUNS SCALES OUT
cd "$(dirname "$0")"
A=$1; B=$2; RUNS=$3; SCALES=$4; OUT=$5
: > "$OUT"
for i in $(seq 1 "$RUNS"); do
  ORDER=${ORDER:-above} timeout 1500 node cost.js "$A" 30 14.5,5.5 "$SCALES" >> "$OUT" 2>&1 &
  ORDER=${ORDER:-above} timeout 1500 node cost.js "$B" 30 14.5,5.5 "$SCALES" >> "$OUT" 2>&1 &
  wait
done
grep "frames\|pace" "$OUT" | awk '{k=$1" "$2" "$3; s[k]+=$5; n[k]++} END {for (k in s) printf "%s m: %.2f\n", k, s[k]/n[k]}' | sort
