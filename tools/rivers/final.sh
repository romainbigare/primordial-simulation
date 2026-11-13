#!/bin/bash
# The CPU cost of the final page against the page of eec58761, the pan frames, then the full gate and the page's build.
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/rv
W=/home/user/primordial-simulation/.claude/worktrees/agent-ab76b80ac68603520
cd $S || exit 1
rm -f $S/final.log $S/final.done
{
  echo "== cost (phone, CPU ms a forced frame, whole process tree)"; uptime
  ROUNDS=3 FRAMES=5 bash $S/cpucost.sh pEf p23f
  uptime
  echo "== pan (phone, final page)"
  PAGE=p23/index.html timeout 1500 node pan.js 8761 phone 1 150 shot shots/pan/final "b200:200:12.6:10.75,b100:100:12.9:10.8" | tail -9
  echo "== errors (desk and phone, final page)"
  bash $S/shots.sh p23 shots/p23 desk "200" "lake:13.3:10.9" | tail -1
  bash $S/shots.sh p23 shots/p23 phone "200" "lake:13.3:10.9" | tail -1
  echo "== gate"
  bash $S/gate.sh $S/gateG.log; cat $S/gateG.log
  echo "== page"
  npm --prefix $W/web run build 2>&1 | tail -1
  npm --prefix $W/web run wasm 2>&1 | tail -1
} > $S/final.log 2>&1
echo done > $S/final.done
