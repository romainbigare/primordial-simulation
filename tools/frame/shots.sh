#!/bin/bash
# Pictures of a build at the check views: shots.sh BUILD PROFILE SEED [VIEWS]
F=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/frame
V1=fit:0:11:11,k2:2000:11.5:9,k1:1000:13.6:5.2,coast:200:2.1:9.2,lake:200:14.3:11.2,ridge:200:9.6:10.8,forest:50:8.0:7.1,meadow:10:15.7:8.7,true:1000:13.6:5.2:true
mkdir -p $F/pics
PAGE=dist-$1/index.html timeout 3000 node $F/bench.js 8810 $2 $3 ${MONTHS:-150} shot $F/pics/$1-$2-$3 "${4:-$V1}" | tail -1
