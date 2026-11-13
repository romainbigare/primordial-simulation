#!/bin/bash
# The full gate once on the merged tree, then the frost test once (its scene changed after the lost helper's run).
W=/home/user/primordial-simulation/.claude/worktrees/agent-ab92bfed03ac3c12c
F=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/s3/final
cd $W
echo "== build"; cargo build --release --workspace --all-targets 2>&1 | grep -E "^(warning|error)|Finished" | head -20; echo "build exit ${PIPESTATUS[0]}"
echo "== test"; cargo test --release --workspace --no-fail-fast > $F/gate_test.log 2>&1; echo "test exit $?"
grep -E "^test result|FAILED|panicked" $F/gate_test.log | head -60
echo "== clippy"; cargo clippy --release --workspace --all-targets 2>&1 | tee $F/gate_clippy.log | grep -cE "^warning|^error"; tail -2 $F/gate_clippy.log
echo "== wasm"; cargo build -p pcore --target wasm32-unknown-unknown 2>&1 | tail -2
echo "== web"; (cd web && npm run build > $F/gate_web.log 2>&1; echo "npm build exit $?"; npm run wasm >> $F/gate_web.log 2>&1; echo "npm wasm exit $?"); tail -5 $F/gate_web.log
echo "== frost"; cargo test --release -p pcore --test cover -- --ignored --nocapture > $F/frost_test.log 2>&1; echo "frost exit $?"; grep -vE "^\s+[0-9]+:|at /rustc|at \./" $F/frost_test.log | tail -15
git status --short
echo GATE_DONE
