#!/bin/bash
# The gate and the slow tests but the two learning ones (which run on their own, final3.sh).
C=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/climate
echo "---" >> $C/gate.log
$C/gate.sh
cd /home/user/primordial-simulation/.claude/worktrees/agent-a89ba647ceddee687
cargo test --release --workspace --no-fail-fast -- --ignored --skip a_body_that_learns_lives_on_what_it_can_take --skip body_learns_to_move_toward_what_it_can_take > $C/g_ignored.txt 2>&1
grep "^test result" $C/g_ignored.txt | awk '{p+=$4; f+=$6} END {print "ignored2 (but the two learning tests): passed",p,"failed",f}' >> $C/gate.log
