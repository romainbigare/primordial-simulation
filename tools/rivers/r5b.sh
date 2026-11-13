#!/bin/bash
# Round five: deploy the built page as p26, shoot it, pair it with p23 (870fbb00), commit, run the gate.
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/rv
W=/home/user/primordial-simulation/.claude/worktrees/agent-ab76b80ac68603520
cd $S || exit 1
rm -f $S/r5b.done
bash $S/deploy.sh p26 && bash $S/deploy.sh p26f --full
PAN=1 bash $S/r5.sh p26 > $S/r5-p26.log 2>&1
PAGE=p23/index.html timeout 1500 node pan.js 8761 phone 1 150 shot shots/pan/p23 "b100:100:12.9:10.8" | tail -1 >> $S/r5-p26.log
for v in lake500 lake200 lake100 lakem50 lakem10 sea500 sea200 sea100 sea50 sea10; do python3 pair.py pairs/r5-phone-$v.png shots/p23/phone-$v.png shots/p26/phone-$v.png; done
python3 pair.py pairs/r5-pan100.png shots/pan/p23-b100-pan4.png shots/pan/p26-b100-pan4.png
python3 sheet.py shots/p26 sheet-m phone-lake200 phone-lakem50 phone-sea500 phone-sea50
cd $W || exit 1
git add -A
git commit -q -F $S/msg_r5.txt
git log --oneline | head -1 >> $S/r5-p26.log
bash $S/gate.sh $S/gateH.log
cat $S/gateH.log >> $S/r5-p26.log
echo done > $S/r5b.done
