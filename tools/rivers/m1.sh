#!/bin/bash
# Build the page, deploy it as $1 (and ${1}f held at full pixels), and shoot the mouths' views on the phone into shots/$1.
# m1.sh NAME [VIEWS]
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/rv
W=/home/user/primordial-simulation/.claude/worktrees/agent-ab76b80ac68603520
cd $W/web && npm run build 2>&1 | grep -iE "error|panic" ; [ -n "$WASM" ] && (npm run wasm 2>&1 | tail -1)
cd $S && bash deploy.sh $1 >/dev/null && bash deploy.sh ${1}f --full > /dev/null
V=${2:-"lake500:500:13.3:10.9,lake200:200:13.3:10.9,lake100:100:13.3:10.9,lakem50:50:13.344:10.936,lakem10:10:13.344:10.936,sea500:500:17.608:14.299,sea200:200:17.608:14.299,sea100:100:17.608:14.299,sea50:50:17.608:14.299,sea10:10:17.608:14.299"}
mkdir -p shots/$1
PAGE=$1/index.html timeout 3000 node bench.js 8761 ${PROFILE:-phone} 1 150 shot shots/$1/${PROFILE:-phone} "$V" | grep -o '"when":"[^"]*"\|"errors":.*'
