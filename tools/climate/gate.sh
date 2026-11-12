#!/bin/bash
# The gate before a commit; writes one line per step to gate.log in the scratch dir.
L=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/climate/gate.log
cd /home/user/primordial-simulation/.claude/worktrees/agent-a89ba647ceddee687
echo "gate start $(date +%T) $(git rev-parse --short HEAD)" >> $L
cargo build --release --workspace --all-targets > /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/climate/g_build.txt 2>&1; echo "build $? $(grep -c '^warning' /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/climate/g_build.txt) warnings $(date +%T)" >> $L
cargo test --release --workspace --no-fail-fast > /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/climate/g_test.txt 2>&1
echo "test $(grep '^test result' /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/climate/g_test.txt | awk '{p+=$4; f+=$6} END {print "passed",p,"failed",f}') $(date +%T)" >> $L
cargo clippy --release --workspace --all-targets > /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/climate/g_clippy.txt 2>&1; echo "clippy $? $(grep -c '^warning' /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/climate/g_clippy.txt) warnings $(date +%T)" >> $L
cargo build -p pcore --target wasm32-unknown-unknown > /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/climate/g_wasm.txt 2>&1; echo "wasm $? $(date +%T)" >> $L
(cd web && npm run build && npm run wasm) > /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/climate/g_web.txt 2>&1; echo "web $? $(date +%T)" >> $L
echo "gate end" >> $L
