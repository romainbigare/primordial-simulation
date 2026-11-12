#!/bin/bash
# The gate before a commit: build, tests, clippy, the core's wasm, the page. Prints one line a step.
cd /home/user/primordial-simulation/.claude/worktrees/agent-abbbaac876ed7e5e9 || exit 1
cargo build --release --workspace --all-targets > /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/crisp/gate-build.log 2>&1; echo "build $?"
cargo test --release --workspace --no-fail-fast 2>&1 | tee /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/crisp/gate-test.log | grep "^test result" | awk '{p+=$4; f+=$6} END {print "passed",p,"failed",f}'
cargo clippy --release --workspace --all-targets 2>&1 | grep -c "^warning" | sed 's/^/clippy warnings /'
cargo build -p pcore --target wasm32-unknown-unknown > /dev/null 2>&1; echo "pcore wasm $?"
cd web && npm run build > /dev/null 2>&1; echo "page $?"; npm run wasm > /dev/null 2>&1; echo "page wasm $?"
