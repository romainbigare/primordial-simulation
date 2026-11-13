#!/bin/bash
# Pictures of three reaches of seed 1's rivers at month 150 at five bars, from page folder PAGE into OUT, for PROFILE.
# shots.sh PAGE OUT PROFILE [BARS] [PLACES]
cd /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/rv || exit 1
page=$1; out=$2; prof=$3; bars=${4:-"2000 1000 200 50 10"}; places=${5:-"plain:15.7099:12.6688 hill:9.4:12.5 mount:8.3911:5.4021"}; mkdir -p $out
V="fit:0:11:11"
for r in $places; do
  n=${r%%:*}; xy=${r#*:}
  for b in $bars; do V="$V,$n$b:$b:$xy"; done
done
PAGE=$page/index.html timeout 3000 node bench.js 8761 $prof 1 ${MONTHS:-150} shot $out/$prof "$V" | grep -o '"when":"[^"]*"\|"errors":.*'
