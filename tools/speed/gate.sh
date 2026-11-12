#!/bin/bash
# The gate: build, tests, clippy, wasm build of the core, the page's build. Run from the worktree.
cd /home/user/primordial-simulation/.claude/worktrees/agent-a698b5989190b88f4
cargo build --release --workspace --all-targets 2>&1 | tail -1
cargo test --release --workspace --no-fail-fast 2>&1 | tee /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/speed/test.log | grep "^test result" | awk '{p+=$4; f+=$6} END {print "passed",p,"failed",f}'
echo "clippy warnings: $(cargo clippy --release --workspace --all-targets 2>&1 | grep -c '^warning')"
cargo build -p pcore --target wasm32-unknown-unknown 2>&1 | tail -1
cd web && (npm run build && npm run wasm) 2>&1 | grep -E "error|KB|Finished"
