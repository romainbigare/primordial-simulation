#!/bin/bash
# Work per pixel by view: count-land and count-cover pictures of builds on both profiles.
# counts.sh PORT BUILDNAME SEED MONTHS
F=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/frame
PORT=$1; B=$2; SEED=${3:-1}; MONTHS=${4:-150}
VIEWS=${VIEWS:-fit:0:11:11,k2a:2000:12.6:8.5,k2b:2000:4:12,k1a:1000:13.6:10.8,k1b:1000:6:8,m200a:200:15.2:12.1,m200b:200:12.6:6.8,m50a:50:6.1:8.2,m50b:50:14.5:5.4,m10a:10:6.1:8.2,m10b:10:14.5:5.4}
mkdir -p $F/counts2
for prof in ${PROFILES:-phone desk}; do
  for which in land cover; do
    PAGE=dist-$B/count-$which.html timeout 1800 node $F/bench.js $PORT $prof $SEED $MONTHS shot $F/counts2/$B-$prof-$SEED-$which "$VIEWS" | tail -1
  done
done
