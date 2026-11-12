#!/bin/bash
# The gate, the slow tests, then the runs for the report on the merged ground.
C=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/climate
$C/gate.sh
cd /home/user/primordial-simulation/.claude/worktrees/agent-a89ba647ceddee687
cargo test --release --workspace --no-fail-fast -- --ignored > $C/g_ignored.txt 2>&1
grep "^test result" $C/g_ignored.txt | awk '{p+=$4; f+=$6} END {print "ignored: passed",p,"failed",f}' >> $C/gate.log
cp target/release/ew $C/ew_f
cd $C
for b in "20,65 wide" "30,55 mild"; do set -- $b
  for s in 1 2; do ./ew_f run --band $1 --seed $s --size 32 --years 30 --roster --plants --out bit/f_$2_s$s > bit/f_$2_s$s.log 2>&1; done
done
./ew_f run --seed 1 --size 32 --years 30 --roster --plants --out bit/f_none_s1 > bit/f_none_s1.log 2>&1
echo "final done" >> $C/gate.log
