#!/bin/bash
# Page pictures before (the work branch's page, port 8831) and after (the merged tree, port 8832).
F=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/s3/final
O=$F/pics; mkdir -p $O
cd /tmp
python3 -m http.server 8831 --directory $F/dist-before > /dev/null 2>&1 & A=$!
python3 -m http.server 8832 --directory $F/dist-after > /dev/null 2>&1 & B=$!
sleep 2
run() { # year day mode fitonly
  for side in before:8831 after:8832; do
    n=${side%%:*}; port=${side##*:}
    node $F/tools/day.js $port $O/$n-y$1-d$2-$3 $1 $2 14 15 $3 $4 >> $O/log.txt 2>&1 & P="$P $!"
  done
  wait $P; P=""
}
run 30 210 phone 0
run 30 40 desk 0
run 30 40 phone 0
kill $A $B
echo REDO_DONE >> $O/log.txt
